//! `filters.mongoexpression` -- filter points using a MongoDB-style query.

use pdal_core::expr::MongoExpression;
use pdal_core::point::{PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

pub struct MongoExpressionFilter {
    expression: MongoExpression,
}

impl MongoExpressionFilter {
    pub fn new(json_str: &str) -> Result<Self, StageError> {
        let expression = MongoExpression::parse(json_str)
            .map_err(|e| StageError(format!("Invalid Mongo expression: {}", e)))?;
        Ok(MongoExpressionFilter { expression })
    }
}

impl Filter for MongoExpressionFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.mongoexpression"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let mut output = input.make_new();
        for idx in 0..input.len() {
            if self.expression.eval(input, idx) {
                output.append_point(input, idx);
            }
        }
        Ok(vec![output])
    }
}

impl Streamable for MongoExpressionFilter {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        self.expression.eval(view, idx)
    }

    fn reset(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn test_view(values: &[(f64, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        layout.register(DimId::Y, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for &(x, y) in values {
            let p = view.add_point();
            view.set_f64(p, &DimId::X, x);
            view.set_f64(p, &DimId::Y, y);
        }
        view
    }

    #[test]
    fn simple_equality() {
        let input = test_view(&[(1.0, 10.0), (2.0, 20.0)]);
        let mut filter = MongoExpressionFilter::new(r#"{"X": 1.0}"#).unwrap();
        let output = filter.run(&input).unwrap();
        assert_eq!(output[0].len(), 1);
        assert_eq!(output[0].get_f64(0, &DimId::Y), 10.0);
    }

    #[test]
    fn compound_and() {
        let input = test_view(&[(1.0, 10.0), (1.0, 20.0), (2.0, 10.0)]);
        let mut filter = MongoExpressionFilter::new(r#"{"X": 1.0, "Y": 10.0}"#).unwrap();
        let output = filter.run(&input).unwrap();
        assert_eq!(output[0].len(), 1);
    }

    #[test]
    fn gt_operator() {
        let input = test_view(&[(1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]);
        let mut filter = MongoExpressionFilter::new(r#"{"X": {"$gt": 1.5}}"#).unwrap();
        let output = filter.run(&input).unwrap();
        assert_eq!(output[0].len(), 2);
    }

    #[test]
    fn in_operator() {
        let input = test_view(&[(1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]);
        let mut filter = MongoExpressionFilter::new(r#"{"X": {"$in": [1.0, 3.0]}}"#).unwrap();
        let output = filter.run(&input).unwrap();
        assert_eq!(output[0].len(), 2);
    }
}
