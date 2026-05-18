use crate::math;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct PlaneFitFilter {
    knn: usize,
}

impl PlaneFitFilter {
    pub fn new(knn: usize) -> Self {
        Self { knn }
    }
}

impl Filter for PlaneFitFilter {
    fn name(&self) -> &str {
        "filters.planefit"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let neighbors = index
                .knn(idx, self.knn + 1)
                .into_iter()
                .skip(1)
                .map(|(id, _dist)| id)
                .collect::<Vec<_>>();
            if neighbors.is_empty() {
                continue;
            }

            let covariance = math::covariance(view, &neighbors);
            if math::is_zero_matrix(covariance) {
                continue;
            }

            let centroid = math::centroid(view, &neighbors);
            let (_values, vectors) = math::symmetric_eigen_decomposition(covariance);
            let normal = [vectors[0][0], vectors[1][0], vectors[2][0]];
            let distance = abs_distance(view, idx, centroid, normal);
            let neighbor_distance_sum = neighbors
                .iter()
                .map(|neighbor| abs_distance(view, *neighbor, centroid, normal))
                .sum::<f64>();
            let mean_neighbor_distance = neighbor_distance_sum / self.knn as f64;
            output.set_f64(
                idx,
                &DimId::PlaneFit,
                distance / (distance + mean_neighbor_distance),
            );
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for PlaneFitFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        false
    }
}

fn abs_distance(view: &PointView, id: PointId, centroid: [f64; 3], normal: [f64; 3]) -> f64 {
    let point = [
        view.get_f64(id, &DimId::X) - centroid[0],
        view.get_f64(id, &DimId::Y) - centroid[1],
        view.get_f64(id, &DimId::Z) - centroid[2],
    ];
    (normal[0] * point[0] + normal[1] * point[1] + normal[2] * point[2]).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn elevated_point_has_full_plane_fit_score() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::PlaneFit, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (x, y, z) in [
            (0.0, 0.0, 1.0),
            (-1.0, 0.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, -1.0, 0.0),
            (0.0, 1.0, 0.0),
        ] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
            view.set_f64(idx, &DimId::Z, z);
        }

        let mut filter = PlaneFitFilter::new(4);
        let out = filter.run(&view).unwrap().remove(0);
        assert!((out.get_f64(0, &DimId::PlaneFit) - 1.0).abs() < 1e-6);
    }
}
