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

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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

    fn streamable(&self) -> bool {
        true
    }

    fn stream_chunk(&mut self, chunk: &mut PointView) -> Result<(), StageError> {
        let n = chunk.len();
        let mut write = 0u64;
        for read in 0..n {
            if self.process_one(chunk, read) {
                if write != read {
                    chunk.copy_point_within(read, write);
                }
                write += 1;
            }
        }
        chunk.truncate(write);
        Ok(())
    }
}

impl Streamable for HeadFilter {
    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
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

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout};
    use std::rc::Rc;

    fn view(count: u64) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for value in 0..count {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, value as f64);
        }
        view
    }

    #[test]
    fn stream_chunk_matches_run_one() {
        let input = view(6);
        let mut standard = HeadFilter::new(3, false);
        let expected = standard.run_one(&input).unwrap().remove(0);

        let mut chunk = input;
        let mut streamed = HeadFilter::new(3, false);
        streamed.stream_chunk(&mut chunk).unwrap();

        assert_eq!(chunk.len(), expected.len());
        for idx in 0..chunk.len() {
            assert_eq!(
                chunk.get_f64(idx, &DimId::X),
                expected.get_f64(idx, &DimId::X)
            );
        }
    }
}
