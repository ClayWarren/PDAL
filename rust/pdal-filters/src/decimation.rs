use pdal_core::options::Options;
use pdal_core::point::PointView;
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct DecimationFilter {
    step: f64,
    offset: u64,
    limit: u64,
    index: u64,
    kept: u64,
}

impl DecimationFilter {
    pub fn new(options: &Options) -> Self {
        Self {
            step: options.get_f64("step", 1.0),
            offset: options.get_u64("offset", 0),
            limit: options.get_u64("limit", u64::MAX),
            index: 0,
            kept: 0,
        }
    }
}

impl Filter for DecimationFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.decimation"
    }

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut out = input.make_new();

        if self.step < 1.0 {
            return Err(StageError("Option step must be >= 1.0".to_string()));
        }

        let last_idx = std::cmp::min(self.limit, input.len());
        if last_idx > self.offset {
            let count = ((last_idx - self.offset) as f64 / self.step).round() as u64;
            for idx in 0..count {
                let src_idx = self.offset + (idx as f64 * self.step).round() as u64;
                if src_idx < input.len() {
                    out.append_point(input, src_idx);
                }
            }
        }

        Ok(vec![out])
    }

    fn streamable(&self) -> bool {
        true
    }

    fn stream_chunk(&mut self, chunk: &mut PointView) -> Result<(), StageError> {
        if self.step < 1.0 {
            return Err(StageError("Option step must be >= 1.0".to_string()));
        }

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

impl Streamable for DecimationFilter {
    fn reset(&mut self) {
        self.index = 0;
        self.kept = 0;
    }

    fn process_one(
        &mut self,
        _view: &mut pdal_core::point::PointView,
        _idx: pdal_core::point::PointId,
    ) -> bool {
        let expected = self.offset + (self.kept as f64 * self.step).round() as u64;
        let keep = self.index >= self.offset && self.index < self.limit && self.index == expected;
        if keep {
            self.kept += 1;
        }
        self.index += 1;
        keep
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

    fn options(entries: &[(&str, &str)]) -> Options {
        let mut options = Options::new();
        for (key, value) in entries {
            options.add(key, *value);
        }
        options
    }

    #[test]
    fn stream_chunk_matches_run_one() {
        let opts = options(&[("step", "2"), ("offset", "1"), ("limit", "8")]);
        let input = view(10);
        let mut standard = DecimationFilter::new(&opts);
        let expected = standard.run_one(&input).unwrap().remove(0);

        let mut chunk = input;
        let mut streamed = DecimationFilter::new(&opts);
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
