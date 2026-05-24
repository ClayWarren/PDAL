use delaunator::{triangulate, Point};
use pdal_core::point::{DimId, DimType, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct HagDelaunayFilter {
    count: usize,
    allow_extrapolation: bool,
    class_label: u8,
}

impl HagDelaunayFilter {
    pub fn new(count: usize, allow_extrapolation: bool, class_label: u8) -> Self {
        Self {
            count,
            allow_extrapolation,
            class_label,
        }
    }
}

impl Filter for HagDelaunayFilter {
    fn name(&self) -> &str {
        "filters.hag_delaunay"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let ground = (0..view.len())
            .filter(|idx| view.get_f64(*idx, &DimId::Classification) as u8 == self.class_label)
            .collect::<Vec<_>>();
        if ground.is_empty() {
            return Ok(vec![output]);
        }

        let bounds = Bounds2d::new(view, &ground);
        for idx in 0..view.len() {
            if view.get_f64(idx, &DimId::Classification) as u8 == self.class_label {
                output.set_f64(idx, &DimId::HeightAboveGround, 0.0);
                continue;
            }

            let x0 = view.get_f64(idx, &DimId::X);
            let y0 = view.get_f64(idx, &DimId::Y);
            let z0 = view.get_f64(idx, &DimId::Z);
            let neighbors = knn_ground_2d(view, &ground, x0, y0, self.count);
            let nearest = neighbors[0];
            let x = view.get_f64(nearest, &DimId::X);
            let y = view.get_f64(nearest, &DimId::Y);
            let z = view.get_f64(nearest, &DimId::Z);

            let ground_z = if (x0 == x && y0 == y) || neighbors.len() == 1 {
                z
            } else if !self.allow_extrapolation && !bounds.contains(x0, y0) {
                z0
            } else {
                interpolate_ground(view, &neighbors, x0, y0).unwrap_or(z0)
            };
            output.set_f64(idx, &DimId::HeightAboveGround, z0 - ground_z);
        }

        Ok(vec![output])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::HeightAboveGround, DimType::F64)]
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for HagDelaunayFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

struct Bounds2d {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds2d {
    fn new(view: &PointView, ids: &[PointId]) -> Self {
        let mut bounds = Self {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for id in ids {
            let x = view.get_f64(*id, &DimId::X);
            let y = view.get_f64(*id, &DimId::Y);
            bounds.min_x = bounds.min_x.min(x);
            bounds.min_y = bounds.min_y.min(y);
            bounds.max_x = bounds.max_x.max(x);
            bounds.max_y = bounds.max_y.max(y);
        }
        bounds
    }

    fn contains(&self, x: f64, y: f64) -> bool {
        self.min_x <= x && x <= self.max_x && self.min_y <= y && y <= self.max_y
    }
}

fn knn_ground_2d(
    view: &PointView,
    ground: &[PointId],
    x: f64,
    y: f64,
    count: usize,
) -> Vec<PointId> {
    let mut neighbors = ground
        .iter()
        .map(|idx| {
            let dx = view.get_f64(*idx, &DimId::X) - x;
            let dy = view.get_f64(*idx, &DimId::Y) - y;
            (*idx, dx * dx + dy * dy)
        })
        .collect::<Vec<_>>();
    neighbors.sort_by(|a, b| a.1.total_cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    neighbors.truncate(count.min(neighbors.len()));
    neighbors.into_iter().map(|(idx, _)| idx).collect()
}

fn interpolate_ground(view: &PointView, neighbors: &[PointId], x0: f64, y0: f64) -> Option<f64> {
    let points = neighbors
        .iter()
        .map(|id| Point {
            x: view.get_f64(*id, &DimId::X),
            y: view.get_f64(*id, &DimId::Y),
        })
        .collect::<Vec<_>>();
    let triangulation = triangulate(&points);

    for tri in triangulation.triangles.chunks_exact(3) {
        let a = neighbors[tri[0]];
        let b = neighbors[tri[1]];
        let c = neighbors[tri[2]];
        let z =
            barycentric_interpolation(point3(view, a), point3(view, b), point3(view, c), x0, y0);
        if z.is_finite() {
            return Some(z);
        }
    }
    Some(view.get_f64(neighbors[0], &DimId::Z))
}

fn point3(view: &PointView, id: PointId) -> (f64, f64, f64) {
    (
        view.get_f64(id, &DimId::X),
        view.get_f64(id, &DimId::Y),
        view.get_f64(id, &DimId::Z),
    )
}

fn barycentric_interpolation(
    a: (f64, f64, f64),
    b: (f64, f64, f64),
    c: (f64, f64, f64),
    x: f64,
    y: f64,
) -> f64 {
    let denom = (b.1 - c.1) * (a.0 - c.0) + (c.0 - b.0) * (a.1 - c.1);
    if denom == 0.0 {
        return f64::INFINITY;
    }

    let lambda1 = ((b.1 - c.1) * (x - c.0) + (c.0 - b.0) * (y - c.1)) / denom;
    let lambda2 = ((c.1 - a.1) * (x - c.0) + (a.0 - c.0) * (y - c.1)) / denom;
    let lambda3 = 1.0 - lambda1 - lambda2;

    if lambda1 < 0.0 || lambda2 < 0.0 || lambda3 < 0.0 {
        return f64::INFINITY;
    }
    lambda1 * a.2 + lambda2 * b.2 + lambda3 * c.2
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{PointLayout, PointView};
    use std::rc::Rc;

    fn view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::Classification, DimType::F64);
        layout.register(DimId::HeightAboveGround, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z, class) in [
            (0.0, 0.0, 10.0, 2.0),
            (4.0, 0.0, 10.0, 2.0),
            (0.0, 4.0, 18.0, 2.0),
            (1.0, 1.0, 20.0, 1.0),
            (8.0, 8.0, 30.0, 1.0),
        ] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
            view.set_f64(idx, &DimId::Classification, class);
        }
        view
    }

    #[test]
    fn interpolates_ground_surface() {
        let view = view();
        let mut filter = HagDelaunayFilter::new(3, true, 2);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::HeightAboveGround), 0.0);
        assert_eq!(out.get_f64(3, &DimId::HeightAboveGround), 8.0);
    }

    #[test]
    fn outside_without_extrapolation_gets_zero_height() {
        let view = view();
        let mut filter = HagDelaunayFilter::new(3, false, 2);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);
        assert_eq!(out.get_f64(4, &DimId::HeightAboveGround), 0.0);
    }
}
