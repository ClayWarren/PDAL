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

    fn run_one(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
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
    fn process_one(&mut self, _view: &mut PointView, _idx: pdal_core::point::PointId) -> bool {
        false
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout};
    use std::rc::Rc;

    fn view(returns: &[(f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::ReturnNumber, DimType::U8);
        layout.register(DimId::NumberOfReturns, DimType::U8);
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for (idx, (rn, nr)) in returns.iter().enumerate() {
            let id = view.add_point();
            view.set_f64(id, &DimId::ReturnNumber, *rn);
            view.set_f64(id, &DimId::NumberOfReturns, *nr);
            view.set_f64(id, &DimId::X, idx as f64);
        }
        view
    }

    #[test]
    fn splits_requested_return_groups_in_stable_order() {
        let input = view(&[(1.0, 3.0), (2.0, 3.0), (3.0, 3.0), (1.0, 1.0)]);
        let mut filter = ReturnsFilter::new(vec![
            "first".to_string(),
            "intermediate".to_string(),
            "last".to_string(),
            "only".to_string(),
        ]);

        let outputs = filter.run_one(&input).unwrap();

        assert_eq!(outputs.len(), 4);
        assert_eq!(outputs[0].get_f64(0, &DimId::X), 0.0);
        assert_eq!(outputs[1].get_f64(0, &DimId::X), 1.0);
        assert_eq!(outputs[2].get_f64(0, &DimId::X), 2.0);
        assert_eq!(outputs[3].get_f64(0, &DimId::X), 3.0);
    }

    #[test]
    fn rejects_unknown_return_groups_and_is_not_streamable() {
        let mut filter = ReturnsFilter::new(vec!["bogus".to_string()]);
        assert!(filter.run_one(&view(&[(1.0, 1.0)])).is_err());

        let mut input = view(&[(1.0, 1.0)]);
        assert!(!filter.process_one(&mut input, 0));
        filter.reset();
    }
}
