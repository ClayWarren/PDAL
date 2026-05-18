use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

#[allow(clippy::approx_constant)]
const PDAL_PI: f64 = 3.14159;

pub struct RadialDensityFilter {
    pub radius: f64,
}

impl RadialDensityFilter {
    pub fn new(radius: f64) -> Self {
        Self { radius }
    }
}

impl Filter for RadialDensityFilter {
    fn name(&self) -> &str {
        "filters.radialdensity"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        let factor = 1.0 / ((4.0 / 3.0) * PDAL_PI * self.radius.powi(3));
        for idx in 0..view.len() {
            let count = index.radius(idx, self.radius).len();
            output.set_f64(idx, &DimId::RadialDensity, count as f64 * factor);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for RadialDensityFilter {
    fn process_one(&mut self, _view: &pdal_core::point::PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::RadialDensity, DimType::F64);
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
    fn density_matches_existing_cpp_test_shape() {
        let view = view(&[
            (0.0, 0.0, 0.0),
            (0.1, 0.0, 0.0),
            (0.0, 0.1, 0.0),
            (0.0, 0.0, 0.1),
            (0.1, 0.1, 0.0),
            (100.0, 100.0, 100.0),
        ]);
        let mut filter = RadialDensityFilter::new(1.0);
        let out = filter.run(&view).unwrap().remove(0);
        let factor = 1.0 / ((4.0 / 3.0) * PDAL_PI);
        for idx in 0..5 {
            assert!((out.get_f64(idx, &DimId::RadialDensity) - 5.0 * factor).abs() < 1e-9);
        }
        assert!((out.get_f64(5, &DimId::RadialDensity) - factor).abs() < 1e-9);
    }
}
