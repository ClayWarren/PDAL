use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct LofFilter {
    minpts: usize,
}

impl LofFilter {
    pub fn new(minpts: usize) -> Self {
        Self { minpts }
    }
}

impl Filter for LofFilter {
    fn name(&self) -> &str {
        "filters.lof"
    }

    fn run_one(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        let count = self.minpts + 1;
        let adjacency = (0..view.len())
            .map(|idx| {
                index
                    .knn(idx, count)
                    .into_iter()
                    .map(|(id, sqr_dist)| (id, sqr_dist.sqrt()))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        for (idx, neighbors) in adjacency.iter().enumerate() {
            if let Some((_, distance)) = neighbors.last() {
                output.set_f64(idx as u64, &DimId::NNDistance, *distance);
            }
        }

        for idx in 0..view.len() {
            let mut mean = 0.0;
            let mut n = 0.0;
            for (neighbor, distance) in &adjacency[idx as usize] {
                let k_distance = output.get_f64(*neighbor, &DimId::NNDistance);
                let reach_distance = k_distance.max(*distance);
                n += 1.0;
                mean += (reach_distance - mean) / n;
            }
            output.set_f64(idx, &DimId::LocalReachabilityDistance, 1.0 / mean);
        }

        for idx in 0..view.len() {
            let lrd = output.get_f64(idx, &DimId::LocalReachabilityDistance);
            let mut mean = 0.0;
            let mut n = 0.0;
            for (neighbor, _) in &adjacency[idx as usize] {
                let ratio = output.get_f64(*neighbor, &DimId::LocalReachabilityDistance) / lrd;
                n += 1.0;
                mean += (ratio - mean) / n;
            }
            output.set_f64(idx, &DimId::LocalOutlierFactor, mean);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for LofFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn grid_with_outlier() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::NNDistance, DimType::F64);
        layout.register(DimId::LocalReachabilityDistance, DimType::F64);
        layout.register(DimId::LocalOutlierFactor, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));

        for i in 0..5 {
            for j in 0..5 {
                let idx = view.add_point();
                view.set_f64(idx, &DimId::X, i as f64 * 2.0);
                view.set_f64(idx, &DimId::Y, j as f64 * 2.0);
                view.set_f64(idx, &DimId::Z, 0.0);
            }
        }
        let idx = view.add_point();
        view.set_f64(idx, &DimId::X, 1000.0);
        view.set_f64(idx, &DimId::Y, 1000.0);
        view.set_f64(idx, &DimId::Z, 1000.0);

        view
    }

    #[test]
    fn flags_outlier() {
        let view = grid_with_outlier();
        let mut filter = LofFilter::new(10);
        let out = filter.run(std::slice::from_ref(&view)).unwrap().remove(0);

        let nn_inlier = out.get_f64(12, &DimId::NNDistance);
        let nn_outlier = out.get_f64(25, &DimId::NNDistance);
        let lof_inlier = out.get_f64(12, &DimId::LocalOutlierFactor);
        let lof_outlier = out.get_f64(25, &DimId::LocalOutlierFactor);

        assert!((nn_inlier - 4.0).abs() < 1e-6);
        assert!(out.get_f64(12, &DimId::LocalReachabilityDistance) > 0.0);
        assert!(nn_outlier > nn_inlier);
        assert!(nn_outlier > 100.0);
        assert!(lof_outlier > lof_inlier);
        assert!(lof_outlier > 2.0);
        assert!(lof_inlier < 2.0);
    }

    #[test]
    fn minpts_controls_k_distance() {
        let view = grid_with_outlier();
        let mut near = LofFilter::new(4);
        let near = near.run(std::slice::from_ref(&view)).unwrap().remove(0);
        let mut far = LofFilter::new(10);
        let far = far.run(std::slice::from_ref(&view)).unwrap().remove(0);

        assert!(near.get_f64(12, &DimId::NNDistance) < far.get_f64(12, &DimId::NNDistance));
    }
}
