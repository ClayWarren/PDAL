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

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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
}

impl Streamable for DecimationFilter {
    fn reset(&mut self) {
        self.index = 0;
        self.kept = 0;
    }

    fn process_one(
        &mut self,
        _view: &pdal_core::point::PointView,
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
