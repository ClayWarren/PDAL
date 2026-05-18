use pdal_core::point::PointView;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct HeadFilter {
    count: u64,
    invert: bool,
    index: u64,
}

impl HeadFilter {
    pub fn new(count: u64, invert: bool) -> Self {
        HeadFilter {
            count,
            invert,
            index: 0,
        }
    }
}

impl Filter for HeadFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.head"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();
        self.index = 0;

        for idx in 0..input.len() {
            let mut keep = false;
            if self.index < self.count {
                keep = true;
            }
            self.index += 1;

            if self.invert {
                keep = !keep;
            }

            if keep {
                out.append_point(input, idx);
            }
        }

        Ok(vec![out])
    }
}

impl Streamable for HeadFilter {
    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        let mut keep = false;
        if self.index < self.count {
            keep = true;
        }
        self.index += 1;

        if self.invert {
            keep = !keep;
        }
        keep
    }

    fn reset(&mut self) {
        self.index = 0;
    }
}
