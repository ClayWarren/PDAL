use crate::math;
use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct ApproximateCoplanarFilter {
    knn: usize,
    threshold1: f64,
    threshold2: f64,
}

impl ApproximateCoplanarFilter {
    pub fn new(knn: usize, threshold1: f64, threshold2: f64) -> Self {
        Self {
            knn,
            threshold1,
            threshold2,
        }
    }
}

impl Filter for ApproximateCoplanarFilter {
    fn name(&self) -> &str {
        "filters.approximatecoplanar"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let ids = index
                .knn(idx, self.knn)
                .into_iter()
                .map(|(id, _dist)| id)
                .collect::<Vec<_>>();
            let covariance = math::covariance(view, &ids);
            if math::is_zero_matrix(covariance) {
                continue;
            }

            let ev = math::symmetric_eigenvalues(covariance);
            let coplanar = (ev[1] > self.threshold1 * ev[0]) && (self.threshold2 * ev[1] > ev[2]);
            output.set_f64(idx, &DimId::Coplanar, u8::from(coplanar) as f64);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ApproximateCoplanarFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
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
        layout.register(DimId::Coplanar, DimType::F64);
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
    fn labels_plane_points() {
        let mut points = Vec::new();
        for x in 0..5 {
            for y in 0..5 {
                points.push((x as f64, y as f64, 0.01 * x as f64 + 0.02 * y as f64));
            }
        }
        let view = view(&points);
        let mut filter = ApproximateCoplanarFilter::new(8, 25.0, 6.0);
        let out = filter.run(&view).unwrap().remove(0);
        let coplanar = (0..out.len())
            .map(|idx| out.get_f64(idx, &DimId::Coplanar) as u64)
            .sum::<u64>();
        assert!(coplanar >= 20);
    }

    #[test]
    fn rejects_volume_points() {
        let mut points = Vec::new();
        for x in 0..3 {
            for y in 0..3 {
                for z in 0..3 {
                    points.push((x as f64, y as f64, z as f64));
                }
            }
        }
        let view = view(&points);
        let mut filter = ApproximateCoplanarFilter::new(8, 25.0, 6.0);
        let out = filter.run(&view).unwrap().remove(0);
        for idx in 0..out.len() {
            assert_eq!(out.get_f64(idx, &DimId::Coplanar) as u8, 0);
        }
    }
}
