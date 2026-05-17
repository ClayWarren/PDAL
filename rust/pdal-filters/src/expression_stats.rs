//! `filters.expressionstats` -- accumulate count statistics for a dimension
//! based on conditional expressions.

use pdal_core::expr::ConditionalExpression;
use pdal_core::point::{DimId, PointId, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::HashMap;

/// The `filters.expressionstats` stage.
pub struct ExpressionStatsFilter {
    dim_name: String,
    expressions: Vec<ConditionalExpression>,
    // Map: Expression source -> (Value -> Count)
    stats: HashMap<String, HashMap<String, u64>>,
}

impl ExpressionStatsFilter {
    pub fn new(dim_name: &str, sources: &[String]) -> Result<Self, StageError> {
        let mut expressions = Vec::with_capacity(sources.len());
        for src in sources {
            let expr = ConditionalExpression::parse(src)
                .map_err(|e| StageError(format!("The expression '{src}' is invalid: {e}")))?;
            expressions.push(expr);
        }
        Ok(ExpressionStatsFilter {
            dim_name: dim_name.to_string(),
            expressions,
            stats: HashMap::new(),
        })
    }

    /// Retrieve the accumulated statistics.
    pub fn stats(&self) -> &HashMap<String, HashMap<String, u64>> {
        &self.stats
    }
}

impl Filter for ExpressionStatsFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.expressionstats"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        let layout = input.layout().as_ref();
        for expr in self.expressions.iter_mut() {
            expr.prepare(layout).map_err(StageError)?;
        }

        for idx in 0..input.len() {
            self.process_point(input, idx);
        }

        // Like most "stats" filters, this passes the input view through.
        Ok(vec![input.clone()])
    }
}

impl ExpressionStatsFilter {
    fn process_point(&mut self, view: &PointView, idx: PointId) {
        let dim = DimId::from_name(&self.dim_name);
        let val = view.get_f64(idx, &dim);
        let val_str = val.to_string();

        for expr in &self.expressions {
            if expr.eval(view, idx) {
                let entry = self.stats.entry(expr.print()).or_default();
                *entry.entry(val_str.clone()).or_insert(0) += 1;
            }
        }
    }
}

impl Streamable for ExpressionStatsFilter {
    fn process_one(&mut self, view: &mut PointView, idx: PointId) -> bool {
        self.process_point(view, idx);
        true
    }

    fn reset(&mut self) {
        self.stats.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimId, DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn classified(values: &[f64]) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        for &c in values {
            let p = view.add_point();
            view.set_f64(p, &DimId::Classification, c);
        }
        view
    }

    #[test]
    fn stats_count_matches() {
        let input = classified(&[1.0, 2.0, 2.0, 7.0]);
        let mut filter =
            ExpressionStatsFilter::new("Classification", &["Classification == 2".to_string()])
                .unwrap();
        filter.run(&input).unwrap();

        let stats = filter.stats();
        let counts = stats.get("(Classification==2)").unwrap();
        assert_eq!(*counts.get("2").unwrap(), 2);
    }
}
