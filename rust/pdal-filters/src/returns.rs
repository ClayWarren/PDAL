use pdal_core::point::{DimId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

const RETURN_FIRST: u32 = 1;
const RETURN_INTERMEDIATE: u32 = 2;
const RETURN_LAST: u32 = 4;
const RETURN_ONLY: u32 = 8;

pub struct ReturnsFilter {
    pub groups: Vec<String>,
}

impl ReturnsFilter {
    pub fn new(groups: Vec<String>) -> Self {
        ReturnsFilter { groups }
    }
}

impl Filter for ReturnsFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.returns"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output_types = 0u32;
        for g in &self.groups {
            let trimmed = g.trim();
            match trimmed {
                "first" => output_types |= RETURN_FIRST,
                "intermediate" => output_types |= RETURN_INTERMEDIATE,
                "last" => output_types |= RETURN_LAST,
                "only" => output_types |= RETURN_ONLY,
                _ => return Err(StageError(format!("Invalid output type: '{}'.", trimmed))),
            }
        }

        let mut first_view = input.make_new();
        let mut intermediate_view = input.make_new();
        let mut last_view = input.make_new();
        let mut only_view = input.make_new();

        let return_number_dim = DimId::from_name("ReturnNumber");
        let number_of_returns_dim = DimId::from_name("NumberOfReturns");

        for idx in 0..input.len() {
            let rn = input.get_f64(idx, &return_number_dim) as u8;
            let nr = input.get_f64(idx, &number_of_returns_dim) as u8;

            if (output_types & RETURN_FIRST) != 0 && rn == 1 && nr > 1 {
                first_view.append_point(input, idx);
            }
            if (output_types & RETURN_INTERMEDIATE) != 0 && rn > 1 && rn < nr && nr > 2 {
                intermediate_view.append_point(input, idx);
            }
            if (output_types & RETURN_LAST) != 0 && rn == nr && nr > 1 {
                last_view.append_point(input, idx);
            }
            if (output_types & RETURN_ONLY) != 0 && nr == 1 {
                only_view.append_point(input, idx);
            }
        }

        let mut results = Vec::new();
        if (output_types & RETURN_FIRST) != 0 && !first_view.is_empty() {
            results.push(first_view);
        }
        if (output_types & RETURN_INTERMEDIATE) != 0 && !intermediate_view.is_empty() {
            results.push(intermediate_view);
        }
        if (output_types & RETURN_LAST) != 0 && !last_view.is_empty() {
            results.push(last_view);
        }
        if (output_types & RETURN_ONLY) != 0 && !only_view.is_empty() {
            results.push(only_view);
        }

        Ok(results)
    }
}

impl Streamable for ReturnsFilter {
    fn process_one(&mut self) -> bool {
        false
    }

    fn reset(&mut self) {}
}
