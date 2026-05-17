use pdal_core::point::PointView;
use pdal_core::stage::{Filter, StageError};
use std::cmp;

pub struct TailFilter {
    count: u64,
    invert: bool,
}

impl TailFilter {
    pub fn new(count: u64, invert: bool) -> Self {
        TailFilter { count, invert }
    }
}

impl Filter for TailFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.tail"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        let len = input.len();

        let start;
        let end;

        if self.invert {
            start = 0;
            end = len.saturating_sub(cmp::min(self.count, len));
        } else {
            start = len.saturating_sub(cmp::min(self.count, len));
            end = len;
        }

        for idx in start..end {
            out.append_point(input, idx);
        }

        Ok(vec![out])
    }
}

impl pdal_core::stage::Streamable for TailFilter {
    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        // Tail filter cannot stream
        false
    }

    fn reset(&mut self) {}
}
