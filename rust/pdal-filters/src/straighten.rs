//! `filters.straighten`: warp points between world coordinates and a
//! "straightened" frame that runs along a track polyline.
//!
//! A `LINESTRING ZM` polyline defines the track; the M value of each vertex is
//! the roll angle in radians. Straightening maps a point to (arc-length,
//! cross-track, height); the reverse mode maps it back.

use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

const FOUR_EPSILON: f64 = 4.0 * f64::EPSILON;

fn double_near(a: f64, b: f64, epsilon: f64) -> bool {
    if a.is_nan() || b.is_nan() {
        return a.is_nan() && b.is_nan();
    }
    let diff = a - b;
    diff > -epsilon && diff <= epsilon
}

/// Azimuth of the segment (x1,y1)->(x2,y2): `atan2(dx, dy)`.
fn azimuth(x1: f64, y1: f64, x2: f64, y2: f64) -> f64 {
    (x2 - x1).atan2(y2 - y1)
}

/// Interpolate between two angles by `ratio`, staying on the unit circle.
fn angular_ratio(v1: f64, v2: f64, ratio: f64) -> f64 {
    let sin = v2.sin() * ratio + v1.sin() * (1.0 - ratio);
    let cos = v2.cos() * ratio + v1.cos() * (1.0 - ratio);
    sin.atan2(cos)
}

/// Squared distance from a point to a segment, returning the closest point on
/// the segment through `out`.
fn sqr_dist_to_line(
    pt_x: f64,
    pt_y: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    out: &mut (f64, f64),
) -> f64 {
    let mut min_x = x1;
    let mut min_y = y1;
    let dx = x2 - x1;
    let dy = y2 - y1;

    if !double_near(dx, 0.0, FOUR_EPSILON) || !double_near(dy, 0.0, FOUR_EPSILON) {
        let t = ((pt_x - x1) * dx + (pt_y - y1) * dy) / (dx * dx + dy * dy);
        if t > 1.0 {
            min_x = x2;
            min_y = y2;
        } else if t > 0.0 {
            min_x += dx * t;
            min_y += dy * t;
        }
    }

    let ddx = pt_x - min_x;
    let ddy = pt_y - min_y;
    let dist = ddx * ddx + ddy * ddy;

    // Snap to the segment if rounding put the point fractionally off it.
    if double_near(dist, 0.0, FOUR_EPSILON) {
        *out = (pt_x, pt_y);
        return 0.0;
    }
    *out = (min_x, min_y);
    dist
}

/// 3x3 linear part plus a translation, an affine transform.
struct Affine {
    linear: [[f64; 3]; 3],
    trans: [f64; 3],
}

impl Affine {
    /// Build a transform from a translation and XYZ-convention Euler angles,
    /// mirroring `straighten::Utils::getTransformation`.
    fn from_euler(x: f64, y: f64, z: f64, roll: f64, pitch: f64, yaw: f64) -> Self {
        let (a, b) = (yaw.cos(), yaw.sin());
        let (c, d) = (pitch.cos(), pitch.sin());
        let (e, f) = (roll.cos(), roll.sin());
        let (de, df) = (d * e, d * f);
        Affine {
            linear: [
                [a * c, a * df - b * e, b * f + a * de],
                [b * c, a * e + b * df, b * de - a * f],
                [-d, c * f, c * e],
            ],
            trans: [x, y, z],
        }
    }

    /// Apply the transform to a point: `linear * p + trans`.
    fn apply(&self, p: [f64; 3]) -> [f64; 3] {
        let l = &self.linear;
        [
            l[0][0] * p[0] + l[0][1] * p[1] + l[0][2] * p[2] + self.trans[0],
            l[1][0] * p[0] + l[1][1] * p[1] + l[1][2] * p[2] + self.trans[1],
            l[2][0] * p[0] + l[2][1] * p[1] + l[2][2] * p[2] + self.trans[2],
        ]
    }

    /// Apply the inverse transform. The linear part is always a rotation
    /// (built from Euler angles), so its inverse is its transpose.
    fn apply_inverse(&self, p: [f64; 3]) -> [f64; 3] {
        let l = &self.linear;
        let d = [
            p[0] - self.trans[0],
            p[1] - self.trans[1],
            p[2] - self.trans[2],
        ];
        [
            l[0][0] * d[0] + l[1][0] * d[1] + l[2][0] * d[2],
            l[0][1] * d[0] + l[1][1] * d[1] + l[2][1] * d[2],
            l[0][2] * d[0] + l[1][2] * d[1] + l[2][2] * d[2],
        ]
    }
}

/// Per-vertex polyline data: position, roll, cumulative arc length, azimuth.
struct Vertex {
    x: f64,
    y: f64,
    z: f64,
    roll: f64,
    w: f64,
    azimuth: f64,
}

/// Segment parameters sampled along the polyline.
struct Sample {
    x: f64,
    y: f64,
    z: f64,
    m: f64,
    azimuth: f64,
    offset: f64,
}

pub struct Polyline {
    verts: Vec<Vertex>,
}

impl Polyline {
    /// Parse a `LINESTRING ZM (x y z m, ...)` WKT string. Returns `None` for
    /// anything that is not a valid ZM line string with at least two vertices.
    pub fn parse(wkt: &str) -> Option<Polyline> {
        let s = wkt.trim();
        let upper = s.to_uppercase();
        let rest = upper.strip_prefix("LINESTRING")?.trim_start();
        if !rest.starts_with("ZM") {
            return None;
        }
        let open = s.find('(')?;
        let close = s.rfind(')')?;
        if close <= open {
            return None;
        }

        let mut raw: Vec<[f64; 4]> = Vec::new();
        for token in s[open + 1..close].split(',') {
            let parts: Vec<&str> = token.split_whitespace().collect();
            if parts.len() != 4 {
                return None;
            }
            let mut v = [0.0f64; 4];
            for (i, p) in parts.iter().enumerate() {
                v[i] = p.parse().ok()?;
            }
            raw.push(v);
        }
        if raw.len() < 2 {
            return None;
        }

        let n = raw.len();
        let mut verts: Vec<Vertex> = Vec::with_capacity(n);
        let mut cum = 0.0;
        for (i, v) in raw.iter().enumerate() {
            if i != 0 {
                let dx = v[0] - raw[i - 1][0];
                let dy = v[1] - raw[i - 1][1];
                cum += (dx * dx + dy * dy).sqrt();
            }
            // Azimuth points to the next vertex, except the last which reuses
            // the azimuth of the final segment.
            let az = if i + 1 != n {
                azimuth(v[0], v[1], raw[i + 1][0], raw[i + 1][1])
            } else {
                azimuth(raw[i - 1][0], raw[i - 1][1], v[0], v[1])
            };
            verts.push(Vertex {
                x: v[0],
                y: v[1],
                z: v[2],
                roll: v[3],
                w: cum,
                azimuth: az,
            });
        }
        Some(Polyline { verts })
    }

    /// Find the closest polyline segment to a point, sampling its parameters.
    ///
    /// Every segment is tested (the C++ filter narrows candidates with a
    /// KD-tree); for a single-segment track the result is identical, and a
    /// full scan is the globally closest segment in every case.
    fn closest_segment(&self, px: f64, py: f64) -> Sample {
        let mut best = f64::MAX;
        let mut sample = Sample {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            m: 0.0,
            azimuth: 0.0,
            offset: 0.0,
        };
        for k in 0..self.verts.len() - 1 {
            let a = &self.verts[k];
            let b = &self.verts[k + 1];
            let mut proj = (0.0, 0.0);
            let dist = sqr_dist_to_line(px, py, a.x, a.y, b.x, b.y, &mut proj);
            if dist < best {
                best = dist;
                let dx = proj.0 - a.x;
                let dy = proj.1 - a.y;
                let tx = b.x - a.x;
                let ty = b.y - a.y;
                let ratio = ((dx * dx + dy * dy) / (tx * tx + ty * ty)).sqrt();
                sample = Sample {
                    x: proj.0,
                    y: proj.1,
                    z: b.z * ratio + a.z * (1.0 - ratio),
                    m: angular_ratio(a.roll, b.roll, ratio),
                    azimuth: angular_ratio(a.azimuth, b.azimuth, ratio),
                    offset: a.w + (dx * dx + dy * dy).sqrt(),
                };
            }
        }
        sample
    }

    /// Sample the polyline at arc-length position `pk`.
    fn interpolate(&self, pk: f64) -> Sample {
        let n = self.verts.len();
        let mut cur = self.verts.iter().position(|v| pk < v.w).unwrap_or(n - 1);
        if cur == 0 {
            cur = 1;
        }
        let prev = &self.verts[cur - 1];
        let current = &self.verts[cur];

        let tx = current.x - prev.x;
        let ty = current.y - prev.y;
        let segment_length = (tx * tx + ty * ty).sqrt();
        let offset = prev.w;
        let ratio = (pk - offset) / segment_length;

        if ratio > 1.0 {
            Sample {
                x: current.x,
                y: current.y,
                z: current.z,
                m: current.roll,
                azimuth: current.azimuth,
                offset,
            }
        } else if ratio > 0.0 {
            Sample {
                x: ratio * current.x + (1.0 - ratio) * prev.x,
                y: ratio * current.y + (1.0 - ratio) * prev.y,
                z: ratio * current.z + (1.0 - ratio) * prev.z,
                m: angular_ratio(prev.roll, current.roll, ratio),
                azimuth: angular_ratio(prev.azimuth, current.azimuth, ratio),
                offset,
            }
        } else {
            Sample {
                x: prev.x,
                y: prev.y,
                z: prev.z,
                m: prev.roll,
                azimuth: prev.azimuth,
                offset,
            }
        }
    }
}

pub struct StraightenFilter {
    polyline: Polyline,
    reverse: bool,
    offset: f64,
}

impl StraightenFilter {
    /// Build a filter, returning `None` when the polyline WKT is invalid.
    pub fn new(polyline_wkt: &str, reverse: bool, offset: f64) -> Option<Self> {
        Polyline::parse(polyline_wkt).map(|polyline| Self {
            polyline,
            reverse,
            offset,
        })
    }

    fn transform(&self, x: f64, y: f64, z: f64) -> [f64; 3] {
        use std::f64::consts::FRAC_PI_2;
        if self.reverse {
            let s = self.polyline.interpolate(x);
            let t = Affine::from_euler(s.x, s.y, s.z, s.m, 0.0, FRAC_PI_2 - s.azimuth);
            t.apply([0.0, y, z])
        } else {
            let s = self.polyline.closest_segment(x, y);
            let t = Affine::from_euler(s.x, s.y, s.z, s.m, 0.0, FRAC_PI_2 - s.azimuth);
            let straight = t.apply_inverse([x, y, z]);
            [
                straight[0] + s.offset + self.offset,
                straight[1],
                straight[2],
            ]
        }
    }
}

impl Filter for StraightenFilter {
    fn name(&self) -> &str {
        "filters.straighten"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = view.make_new();
        for idx in 0..view.len() {
            out.append_point(view, idx);
            let p = self.transform(
                view.get_f64(idx, &DimId::X),
                view.get_f64(idx, &DimId::Y),
                view.get_f64(idx, &DimId::Z),
            );
            out.set_f64(idx, &DimId::X, p[0]);
            out.set_f64(idx, &DimId::Y, p[1]);
            out.set_f64(idx, &DimId::Z, p[2]);
        }
        Ok(vec![out])
    }
}

impl Streamable for StraightenFilter {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        let p = self.transform(
            view.get_f64(idx, &DimId::X),
            view.get_f64(idx, &DimId::Y),
            view.get_f64(idx, &DimId::Z),
        );
        view.set_f64(idx, &DimId::X, p[0]);
        view.set_f64(idx, &DimId::Y, p[1]);
        view.set_f64(idx, &DimId::Z, p[2]);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(points: &[[f64; 3]]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for p in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, p[0]);
            view.set_f64(idx, &DimId::Y, p[1]);
            view.set_f64(idx, &DimId::Z, p[2]);
        }
        view
    }

    #[test]
    fn rejects_invalid_polyline() {
        assert!(Polyline::parse("not a polyline").is_none());
        assert!(StraightenFilter::new("not a polyline", false, 0.0).is_none());
    }

    #[test]
    fn straighten_then_unstraighten_round_trips() {
        let poly = "LINESTRING ZM (0 0 0 0, 0 100 0 0)";
        let orig = [[2.0, 25.0, 1.0], [-1.0, 50.0, 0.5], [3.0, 75.0, 2.0]];

        let mut straighten = StraightenFilter::new(poly, false, 0.0).unwrap();
        let straightened = straighten.run_one(&view(&orig)).unwrap().pop().unwrap();
        assert!(straightened.get_f64(0, &DimId::X) > 10.0);

        let pts: Vec<[f64; 3]> = (0..straightened.len())
            .map(|i| {
                [
                    straightened.get_f64(i, &DimId::X),
                    straightened.get_f64(i, &DimId::Y),
                    straightened.get_f64(i, &DimId::Z),
                ]
            })
            .collect();

        let mut reverse = StraightenFilter::new(poly, true, 0.0).unwrap();
        let back = reverse.run_one(&view(&pts)).unwrap().pop().unwrap();
        for (i, o) in orig.iter().enumerate() {
            let i = i as u64;
            assert!((back.get_f64(i, &DimId::X) - o[0]).abs() < 1e-6);
            assert!((back.get_f64(i, &DimId::Y) - o[1]).abs() < 1e-6);
            assert!((back.get_f64(i, &DimId::Z) - o[2]).abs() < 1e-6);
        }
    }

    #[test]
    fn polyline_parse_rejects_wrong_token_count() {
        assert!(Polyline::parse("LINESTRING ZM (1 2 3)").is_none());
        assert!(Polyline::parse("LINESTRING ZM (1 2 3 4 5)").is_none());
    }

    #[test]
    fn polyline_parse_rejects_non_numeric() {
        assert!(Polyline::parse("LINESTRING ZM (a b c d, e f g h)").is_none());
    }

    #[test]
    fn polyline_parse_rejects_single_vertex() {
        assert!(Polyline::parse("LINESTRING ZM (0 0 0 0)").is_none());
    }

    #[test]
    fn polyline_parse_rejects_missing_zm_prefix() {
        assert!(Polyline::parse("LINESTRING (0 0, 1 1)").is_none());
    }

    #[test]
    fn polyline_parse_rejects_missing_parens() {
        assert!(Polyline::parse("LINESTRING ZM 0 0 0 0, 1 1 0 0").is_none());
    }

    #[test]
    fn double_near_handles_nan() {
        assert!(double_near(f64::NAN, f64::NAN, 1e-6));
        assert!(!double_near(f64::NAN, 1.0, 1e-6));
        assert!(!double_near(1.0, f64::NAN, 1e-6));
    }

    #[test]
    fn angular_ratio_handles_basic_interpolation() {
        let r = angular_ratio(0.0, std::f64::consts::FRAC_PI_2, 0.5);
        assert!(r > 0.0 && r < std::f64::consts::FRAC_PI_2);
    }

    #[test]
    fn azimuth_returns_atan2_dx_dy() {
        let az = azimuth(0.0, 0.0, 1.0, 0.0);
        assert!((az - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn sqr_dist_to_line_handles_degenerate_segment() {
        let mut out = (0.0, 0.0);
        let d = sqr_dist_to_line(1.0, 1.0, 0.0, 0.0, 0.0, 0.0, &mut out);
        assert!(d > 0.0);
    }

    #[test]
    fn sqr_dist_to_line_returns_zero_for_point_on_line() {
        let mut out = (0.0, 0.0);
        let d = sqr_dist_to_line(0.5, 0.0, 0.0, 0.0, 1.0, 0.0, &mut out);
        assert!(d < 1e-10);
    }

    #[test]
    fn straighten_streamable_process_one_works() {
        let mut f =
            StraightenFilter::new("LINESTRING ZM (0 0 0 0, 0 100 0 0)", false, 0.0).unwrap();
        let mut v = view(&[[2.0, 25.0, 1.0]]);
        assert!(f.process_one(&mut v, 0));
        assert!(v.get_f64(0, &DimId::X) > 10.0);
    }

    #[test]
    fn straighten_filter_name_is_filters_straighten() {
        let f = StraightenFilter::new("LINESTRING ZM (0 0 0 0, 0 100 0 0)", false, 0.0).unwrap();
        assert_eq!(f.name(), "filters.straighten");
    }

    #[test]
    fn straighten_interpolate_handles_ratio_lt_zero_branch() {
        // Use a polyline where the point's projected ratio goes below 0.
        // Reverse mode + a small x value queries interpolate with pk < first vertex.w.
        let mut f = StraightenFilter::new("LINESTRING ZM (0 0 0 0, 100 0 0 0)", true, 0.0).unwrap();
        let mut v = view(&[[-5.0, 0.0, 0.0]]);
        let _ = f.process_one(&mut v, 0);
    }

    #[test]
    fn straighten_interpolate_handles_ratio_gt_one_branch() {
        // A pk past the last vertex triggers the ratio > 1 branch.
        let mut f = StraightenFilter::new("LINESTRING ZM (0 0 0 0, 100 0 0 0)", true, 0.0).unwrap();
        let mut v = view(&[[200.0, 0.0, 0.0]]);
        let _ = f.process_one(&mut v, 0);
    }
}
