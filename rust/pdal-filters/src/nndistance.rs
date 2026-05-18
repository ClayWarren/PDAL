use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NNDistanceMode {
    Kth,
    Average,
}

pub struct NNDistanceFilter {
    pub k: usize,
    pub mode: NNDistanceMode,
}

impl NNDistanceFilter {
    pub fn new(k: usize, mode: NNDistanceMode) -> Self {
        Self { k, mode }
    }
}

impl Filter for NNDistanceFilter {
    fn name(&self) -> &str {
        "filters.nndistance"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let neighbors = index.knn(idx, self.k + 1);
            if neighbors.len() <= 1 {
                output.set_f64(idx, &DimId::NNDistance, 0.0);
                continue;
            }

            let value = match self.mode {
                NNDistanceMode::Kth => neighbors
                    .get(self.k)
                    .or_else(|| neighbors.last())
                    .map(|(_, sqr)| sqr.sqrt())
                    .unwrap_or(0.0),
                NNDistanceMode::Average => {
                    let mut count = 0usize;
                    let mut sum = 0.0;
                    for (_, sqr) in neighbors.iter().skip(1).take(self.k) {
                        count += 1;
                        sum += sqr.sqrt();
                    }
                    if count == 0 {
                        0.0
                    } else {
                        sum / count as f64
                    }
                }
            };
            output.set_f64(idx, &DimId::NNDistance, value);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for NNDistanceFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn line_view() -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::Z, DimType::F64);
        layout.register(DimId::NNDistance, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for x in [0.0, 1.0, 3.0] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
        }
        view
    }

    #[test]
    fn kth_uses_requested_neighbor_distance() {
        let view = line_view();
        let mut filter = NNDistanceFilter::new(2, NNDistanceMode::Kth);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::NNDistance), 3.0);
        assert_eq!(out.get_f64(1, &DimId::NNDistance), 2.0);
    }

    #[test]
    fn average_skips_query_point() {
        let view = line_view();
        let mut filter = NNDistanceFilter::new(2, NNDistanceMode::Average);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::NNDistance), 2.0);
        assert_eq!(out.get_f64(1, &DimId::NNDistance), 1.5);
    }
}
