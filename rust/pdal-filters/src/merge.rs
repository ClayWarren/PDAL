use pdal_core::point::{DimId, DimType, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::cell::RefCell;

pub struct MergeFilter {
    pub accumulated: RefCell<Option<PointView>>,
}

impl MergeFilter {
    pub fn new() -> Self {
        MergeFilter {
            accumulated: RefCell::new(None),
        }
    }
}

impl Default for MergeFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl MergeFilter {
    pub fn merge_view(&self, in_view: &PointView) {
        let mut acc_ref = self.accumulated.borrow_mut();
        if acc_ref.is_none() {
            *acc_ref = Some(in_view.make_new());
        }
        if let Some(acc) = &mut *acc_ref {
            let union = union_dimensions(acc, in_view);
            if !has_dimensions(acc, &union) {
                *acc = acc.with_dimensions(&union);
            }
            let source = if has_dimensions(in_view, &union) {
                in_view.clone()
            } else {
                in_view.with_dimensions(&union)
            };
            for i in 0..source.len() {
                acc.append_point(&source, i);
            }
        }
    }
}

fn union_dimensions(left: &PointView, right: &PointView) -> Vec<(DimId, DimType)> {
    let mut dims = view_dimensions(left);
    for (dim, ty) in view_dimensions(right) {
        if !dims.iter().any(|(existing, _)| existing == &dim) {
            dims.push((dim, ty));
        }
    }
    dims
}

fn view_dimensions(view: &PointView) -> Vec<(DimId, DimType)> {
    let mut dims = Vec::new();
    for idx in 0..view.layout().dim_count() {
        if let Some((dim, ty)) = view.layout().dim_at(idx) {
            dims.push((dim.clone(), ty));
        }
    }
    dims
}

fn has_dimensions(view: &PointView, dims: &[(DimId, DimType)]) -> bool {
    dims.iter().all(|(dim, _)| view.layout().dim(dim).is_some())
}

impl Filter for MergeFilter {
    fn name(&self) -> &str {
        "filters.merge"
    }

    fn run(&mut self, inputs: &[PointView]) -> Result<Vec<PointView>, StageError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }
        for view in inputs {
            self.merge_view(view);
        }
        let acc_ref = self.accumulated.borrow();
        if let Some(acc) = &*acc_ref {
            let mut out = acc.make_new();
            for i in 0..acc.len() {
                out.append_point(acc, i);
            }
            Ok(vec![out])
        } else {
            Ok(Vec::new())
        }
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.run(std::slice::from_ref(input))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for MergeFilter {
    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        true
    }

    fn reset(&mut self) {
        *self.accumulated.borrow_mut() = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout};
    use std::rc::Rc;

    fn view(dims: &[(DimId, DimType)], values: &[(&DimId, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        for (dim, ty) in dims {
            layout.register(dim.clone(), *ty);
        }
        let mut view = PointView::new(Rc::new(layout));
        let id = view.add_point();
        for (dim, value) in values {
            view.set_f64(id, dim, *value);
        }
        view
    }

    #[test]
    fn merge_unions_input_layouts() {
        let left = view(
            &[
                (DimId::X, DimType::F64),
                (DimId::Classification, DimType::U8),
            ],
            &[(&DimId::X, 1.0), (&DimId::Classification, 7.0)],
        );
        let right = view(&[(DimId::X, DimType::F64)], &[(&DimId::X, 2.0)]);
        let mut filter = MergeFilter::new();

        let out = filter.run(&[left, right]).unwrap().remove(0);

        assert_eq!(out.len(), 2);
        assert!(out.layout().dim(&DimId::Classification).is_some());
        assert_eq!(out.get_f64(0, &DimId::Classification), 7.0);
        assert_eq!(out.get_f64(1, &DimId::Classification), 0.0);
    }
}
