use pdal_core::point::{DimId, DimType, PointLayout, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct LabelDuplicatesFilter {
    pub dim_names: Vec<String>,
}

impl LabelDuplicatesFilter {
    pub fn new(dim_names: Vec<String>) -> Self {
        LabelDuplicatesFilter { dim_names }
    }
}

impl Filter for LabelDuplicatesFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.label_duplicates"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        if input.is_empty() {
            return Ok(vec![out]);
        }

        out.append_point(input, 0);
        let dup_dim = DimId::from_name("Duplicate");
        out.set_f64(0, &dup_dim, 0.0);

        let dims: Vec<_> = self
            .dim_names
            .iter()
            .map(|name| DimId::from_name(name))
            .collect();

        for idx in 1..input.len() {
            out.append_point(input, idx);
            let mut is_dup = true;
            for dim in &dims {
                let current = input.get_f64(idx, dim);
                let previous = input.get_f64(idx - 1, dim);
                if current != previous {
                    is_dup = false;
                    break;
                }
            }
            let out_idx = out.len() - 1;
            out.set_f64(out_idx, &dup_dim, if is_dup { 1.0 } else { 0.0 });
        }

        Ok(vec![out])
    }

    fn output_dimensions(&self) -> Vec<(DimId, DimType)> {
        vec![(DimId::from_name("Duplicate"), DimType::F64)]
    }
}

impl Streamable for LabelDuplicatesFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::DimType;
    use std::rc::Rc;

    fn make_view(values: &[(f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        layout.register(DimId::from_name("Duplicate"), DimType::F64);
        let layout = Rc::new(layout);
        let mut view = PointView::new(layout);
        for &(x, y) in values {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, x);
            view.set_f64(idx, &DimId::Y, y);
        }
        view
    }

    #[test]
    fn label_duplicates_empty_input() {
        let layout = Rc::new(PointLayout::new());
        let view = PointView::new(layout);
        let mut filter = LabelDuplicatesFilter::new(vec!["X".to_string()]);
        let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
        assert_eq!(outputs[0].len(), 0);
    }

    #[test]
    fn label_duplicates_single_point() {
        let view = make_view(&[(1.0, 2.0)]);
        let mut filter = LabelDuplicatesFilter::new(vec!["X".to_string()]);
        let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
        let dup = DimId::from_name("Duplicate");
        assert_eq!(outputs[0].get_f64(0, &dup), 0.0);
    }

    #[test]
    fn label_duplicates_detects_duplicates() {
        let view = make_view(&[(1.0, 2.0), (1.0, 3.0), (2.0, 3.0)]);
        let mut filter = LabelDuplicatesFilter::new(vec!["X".to_string()]);
        let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
        let dup = DimId::from_name("Duplicate");
        assert_eq!(outputs[0].get_f64(0, &dup), 0.0);
        assert_eq!(outputs[0].get_f64(1, &dup), 1.0); // X=1 same as previous
        assert_eq!(outputs[0].get_f64(2, &dup), 0.0); // X=2 differs
    }

    #[test]
    fn label_duplicates_multi_dim_match() {
        let view = make_view(&[(1.0, 2.0), (1.0, 2.0), (1.0, 3.0)]);
        let mut filter = LabelDuplicatesFilter::new(vec!["X".to_string(), "Y".to_string()]);
        let outputs = filter.run(std::slice::from_ref(&view)).unwrap();
        let dup = DimId::from_name("Duplicate");
        assert_eq!(outputs[0].get_f64(0, &dup), 0.0);
        assert_eq!(outputs[0].get_f64(1, &dup), 1.0); // both X and Y match
        assert_eq!(outputs[0].get_f64(2, &dup), 0.0); // Y differs
    }

    #[test]
    fn label_duplicates_names() {
        let filter = LabelDuplicatesFilter::new(vec!["X".to_string()]);
        assert_eq!(filter.name(), "filters.label_duplicates");
        assert!(filter.as_any().downcast_ref::<LabelDuplicatesFilter>().is_some());
    }

    #[test]
    fn label_duplicates_process_one_returns_false() {
        let mut filter = LabelDuplicatesFilter::new(vec!["X".to_string()]);
        let mut scratch = PointView::new(Rc::new(PointLayout::new()));
        assert!(!filter.process_one(&mut scratch, 0));
    }
}
