//! `filters.crop` -- filter points inside or outside a bounding box or polygon.

use pdal_core::geometry::Geometry;
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Clone, Copy)]
pub struct CropCenter {
    x: f64,
    y: f64,
    z: Option<f64>,
}

impl CropCenter {
    pub fn new_2d(x: f64, y: f64) -> Self {
        Self { x, y, z: None }
    }

    pub fn new_3d(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z: Some(z) }
    }
}

pub struct CropFilter {
    outside: bool,
    bounds: Vec<(f64, f64, f64, f64, f64, f64)>, // minx, miny, minz, maxx, maxy, maxz
    polygons: Vec<Geometry>,
    centers: Vec<CropCenter>,
    distance: f64,
}

impl CropFilter {
    pub fn new(
        outside: bool,
        bounds: Vec<(f64, f64, f64, f64, f64, f64)>,
        polygons_wkt: Vec<String>,
        centers: Vec<CropCenter>,
        distance: f64,
    ) -> Result<Self, StageError> {
        let mut polygons = Vec::new();
        for wkt in polygons_wkt {
            polygons.push(Geometry::from_wkt(&wkt).map_err(StageError)?);
        }
        Ok(CropFilter {
            outside,
            bounds,
            polygons,
            centers,
            distance,
        })
    }

    fn check_point(&self, x: f64, y: f64, z: f64) -> bool {
        let inside = self.bounds.iter().any(|b| Self::inside_bound(*b, x, y, z))
            || self.polygons.iter().any(|p| p.contains(x, y))
            || self
                .centers
                .iter()
                .any(|center| self.inside_center(*center, x, y, z));

        self.outside != inside
    }

    fn inside_bound(b: (f64, f64, f64, f64, f64, f64), x: f64, y: f64, z: f64) -> bool {
        x >= b.0 && x <= b.3 && y >= b.1 && y <= b.4 && z >= b.2 && z <= b.5
    }

    fn inside_center(&self, center: CropCenter, x: f64, y: f64, z: f64) -> bool {
        let dx = (x - center.x).abs();
        let dy = (y - center.y).abs();
        if dx > self.distance || dy > self.distance {
            return false;
        }

        let distance2 = self.distance * self.distance;
        if let Some(center_z) = center.z {
            let dz = (z - center_z).abs();
            if dz > self.distance {
                return false;
            }
            dx * dx + dy * dy + dz * dz < distance2
        } else {
            dx * dx + dy * dy < distance2
        }
    }

    fn crop_output<F>(&self, input: &PointView, keep: F) -> PointView
    where
        F: Fn(f64, f64, f64) -> bool,
    {
        let mut output = input.make_new();
        for idx in 0..input.len() {
            let x = input.get_f64(idx, &DimId::X);
            let y = input.get_f64(idx, &DimId::Y);
            let z = input.get_f64(idx, &DimId::Z);
            if keep(x, y, z) {
                output.append_point(input, idx);
            }
        }
        output
    }
}

impl Filter for CropFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.crop"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut outputs = Vec::new();

        for polygon in &self.polygons {
            outputs.push(self.crop_output(input, |x, y, _| self.outside != polygon.contains(x, y)));
        }
        for bound in &self.bounds {
            outputs.push(self.crop_output(input, |x, y, z| {
                self.outside != Self::inside_bound(*bound, x, y, z)
            }));
        }
        for center in &self.centers {
            outputs.push(self.crop_output(input, |x, y, z| {
                self.outside != self.inside_center(*center, x, y, z)
            }));
        }

        Ok(outputs)
    }
}

impl Streamable for CropFilter {
    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        let x = view.get_f64(idx, &DimId::X);
        let y = view.get_f64(idx, &DimId::Y);
        let z = view.get_f64(idx, &DimId::Z);

        self.check_point(x, y, z)
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in points {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, *x);
            view.set_f64(idx, &DimId::Y, *y);
            view.set_f64(idx, &DimId::Z, *z);
        }
        view
    }

    #[test]
    fn run_returns_one_view_per_crop_region() {
        let input = view(&[(2.0, 2.0, 0.0), (6.0, 2.0, 0.0), (10.0, 2.0, 0.0)]);
        let mut filter = CropFilter::new(
            false,
            vec![
                (1.0, 1.0, f64::NEG_INFINITY, 3.0, 3.0, f64::INFINITY),
                (5.0, 1.0, f64::NEG_INFINITY, 7.0, 3.0, f64::INFINITY),
            ],
            vec!["POLYGON ((9 1, 11 1, 11 3, 9 3, 9 1))".to_string()],
            vec![],
            0.0,
        )
        .unwrap();

        let outputs = filter.run_one(&input).unwrap();

        assert_eq!(outputs.len(), 3);
        assert_eq!(outputs[0].get_f64(0, &DimId::X), 10.0);
        assert_eq!(outputs[1].get_f64(0, &DimId::X), 2.0);
        assert_eq!(outputs[2].get_f64(0, &DimId::X), 6.0);
    }

    #[test]
    fn center_crop_uses_strict_radius() {
        let input = view(&[(5.0, 5.0, 0.0), (7.5, 5.0, 0.0), (7.4, 5.0, 0.0)]);
        let mut filter = CropFilter::new(
            false,
            vec![],
            vec![],
            vec![CropCenter::new_2d(5.0, 5.0)],
            2.5,
        )
        .unwrap();

        let outputs = filter.run_one(&input).unwrap();

        assert_eq!(outputs.len(), 1);
        assert_eq!(outputs[0].len(), 2);
        assert_eq!(outputs[0].get_f64(1, &DimId::X), 7.4);
    }
}
