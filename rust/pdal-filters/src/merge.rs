use pdal_core::point::PointView;
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
            for i in 0..in_view.len() {
                acc.append_point(in_view, i);
            }
        }
    }
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
