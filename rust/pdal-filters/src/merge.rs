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

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        self.merge_view(input);
        let acc_ref = self.accumulated.borrow();
        if let Some(acc) = &*acc_ref {
            let mut out = acc.make_new();
            for i in 0..acc.len() {
                out.append_point(acc, i);
            }
            Ok(vec![out])
        } else {
            Ok(vec![input.make_new()])
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Streamable for MergeFilter {
    fn process_one(&mut self) -> bool {
        true
    }

    fn reset(&mut self) {
        *self.accumulated.borrow_mut() = None;
    }
}
