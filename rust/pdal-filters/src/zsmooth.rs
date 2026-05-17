use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct ZsmoothFilter {
    radius: f64,
    position: f64,
    dim: DimId,
}

impl ZsmoothFilter {
    pub fn new(radius: f64, position: f64, dim_name: String) -> Self {
        Self {
            radius,
            position,
            dim: DimId::from_name(&dim_name),
        }
    }
}

impl Filter for ZsmoothFilter {
    fn name(&self) -> &str {
        "filters.zsmooth"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let mut values = index
                .radius_2d_excluding(idx, self.radius)
                .into_iter()
                .map(|neighbor| view.get_f64(neighbor, &DimId::Z))
                .collect::<Vec<_>>();
            values.sort_by(f64::total_cmp);

            let value = if values.is_empty() {
                view.get_f64(idx, &DimId::Z)
            } else if values.len() == 1 || self.position == 0.0 {
                values[0]
            } else if self.position == 1.0 {
                values[values.len() - 1]
            } else {
                let pos = self.position * (values.len() - 1) as f64;
                let low = pos.floor() as usize;
                let high = low + 1;
                let highfrac = pos - low as f64;
                let lowfrac = 1.0 - highfrac;
                values[low] * lowfrac + values[high] * highfrac
            };

            output.set_f64(idx, &self.dim, value);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ZsmoothFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn view(points: &[(f64, f64, f64)], output_dim: &str) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::from_name(output_dim), DimType::F64);
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
    fn interpolates_neighbor_z_percentile() {
        let output_dim = "Zsmoothed";
        let view = view(
            &[
                (0.0, 0.0, 5.0),
                (0.1, 0.0, 10.0),
                (0.1, 0.0, 20.0),
                (0.1, 0.0, 30.0),
                (0.1, 0.0, 40.0),
            ],
            output_dim,
        );
        let mut filter = ZsmoothFilter::new(1.0, 0.5, output_dim.to_string());
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::from_name(output_dim)), 25.0);
    }

    #[test]
    fn falls_back_to_own_z_without_neighbors() {
        let output_dim = "Zsmoothed";
        let view = view(&[(0.0, 0.0, 12.0), (10.0, 0.0, 99.0)], output_dim);
        let mut filter = ZsmoothFilter::new(1.0, 0.5, output_dim.to_string());
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::from_name(output_dim)), 12.0);
    }
}
