use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct SeparateScanLineFilter {
    pub group_by: u64,
}

impl SeparateScanLineFilter {
    pub fn new(group_by: u64) -> Self {
        SeparateScanLineFilter { group_by }
    }
}

impl Filter for SeparateScanLineFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.separatescanline"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let edge_dim = DimId::from_name("EdgeOfFlightLine");
        let mut results = Vec::new();
        let mut v = input.make_new();

        let mut line_num = 1;
        for i in 0..input.len() {
            v.append_point(input, i);
            if input.get_f64(i, &edge_dim) as u8 != 0 {
                line_num += 1;
                if line_num > self.group_by {
                    results.push(v);
                    v = input.make_new();
                    line_num = 1;
                }
            }
        }

        if !v.is_empty() {
            results.push(v);
        }

        Ok(results)
    }
}

impl Streamable for SeparateScanLineFilter {
    fn process_one(&mut self, _view: &mut PointView, _idx: PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}
