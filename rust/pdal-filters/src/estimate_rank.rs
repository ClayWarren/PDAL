use crate::math;
use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct EstimateRankFilter {
    knn: usize,
    threshold: f64,
}

impl EstimateRankFilter {
    pub fn new(knn: usize, threshold: f64) -> Self {
        Self { knn, threshold }
    }
}

impl Filter for EstimateRankFilter {
    fn name(&self) -> &str {
        "filters.estimaterank"
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
            output.set_f64(
                idx,
                &DimId::Rank,
                math::rank(view, &ids, self.threshold) as f64,
            );
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for EstimateRankFilter {
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
        layout.register(DimId::Rank, DimType::F64);
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
    fn planar_points_have_rank_two() {
        let mut points = Vec::new();
        for x in 0..5 {
            for y in 0..5 {
                points.push((x as f64, y as f64, 0.0));
            }
        }
        let view = view(&points);
        let mut filter = EstimateRankFilter::new(8, 0.01);
        let out = filter.run(&view).unwrap().remove(0);
        for idx in 0..out.len() {
            assert_eq!(out.get_f64(idx, &DimId::Rank) as u8, 2);
        }
    }

    #[test]
    fn linear_points_have_rank_one() {
        let points = (0..12)
            .map(|idx| (idx as f64, 0.0, 0.0))
            .collect::<Vec<_>>();
        let view = view(&points);
        let mut filter = EstimateRankFilter::new(8, 0.01);
        let out = filter.run(&view).unwrap().remove(0);
        for idx in 0..out.len() {
            assert_eq!(out.get_f64(idx, &DimId::Rank) as u8, 1);
        }
    }
}
