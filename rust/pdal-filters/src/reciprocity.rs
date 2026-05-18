use pdal_core::point::{DimId, PointView};
use pdal_core::spatial::SpatialIndex3d;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct ReciprocityFilter {
    knn: usize,
}

impl ReciprocityFilter {
    pub fn new(knn: usize) -> Self {
        Self { knn }
    }
}

impl Filter for ReciprocityFilter {
    fn name(&self) -> &str {
        "filters.reciprocity"
    }

    fn run(&mut self, view: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = view.make_new();
        for idx in 0..view.len() {
            output.append_point(view, idx);
        }

        let index = SpatialIndex3d::new(view);
        for idx in 0..view.len() {
            let reciprocity = reciprocity_for(&index, idx, self.knn);
            output.set_f64(idx, &DimId::Reciprocity, reciprocity);
        }

        Ok(vec![output])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for ReciprocityFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }
}

fn reciprocity_for(index: &SpatialIndex3d, idx: pdal_core::point::PointId, knn: usize) -> f64 {
    if knn == 0 {
        return 0.0;
    }

    let neighbors = index.knn(idx, knn + 1);
    let mut unidirectional = 0;
    for (neighbor, _dist) in neighbors {
        if neighbor == idx {
            continue;
        }

        let reciprocal_neighbors = index.knn(neighbor, knn + 1);
        if !reciprocal_neighbors
            .iter()
            .any(|(candidate, _dist)| *candidate == idx)
        {
            unidirectional += 1;
        }
    }

    100.0 * unidirectional as f64 / knn as f64
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
        layout.register(DimId::Reciprocity, DimType::F64);
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
    fn matches_existing_basic_case() {
        let view = view(&[
            (0.0, 0.0, 0.0),
            (10.0, 10.0, 10.0),
            (11.0, 11.0, 11.0),
            (12.0, 12.0, 12.0),
            (18.0, 18.0, 18.0),
        ]);
        let mut filter = ReciprocityFilter::new(3);
        let out = filter.run(&view).unwrap().remove(0);
        assert_eq!(out.get_f64(0, &DimId::Reciprocity), 100.0);
        assert_eq!(out.get_f64(1, &DimId::Reciprocity), 0.0);
    }
}
