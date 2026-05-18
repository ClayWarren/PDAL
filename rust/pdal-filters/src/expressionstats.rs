//! `filters.expressionstats` -- accumulate count statistics for a dimension
//! based on conditional expressions.

use pdal_core::expr::ConditionalExpression;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::point::{DimId, PointLayout, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};
use std::collections::BTreeMap;

/// The `filters.expressionstats` stage.
pub struct ExpressionStatsFilter {
    dim: DimId,
    dim_name: String,
    expressions: Vec<ConditionalExpression>,
    /// Accumulated statistics: expression source -> (value_bits -> count).
    stats: Vec<(String, BTreeMap<u64, u64>)>,
    prepared: bool,
}

impl ExpressionStatsFilter {
    pub fn new(dim_name: &str, sources: &[String]) -> Result<Self, StageError> {
        if sources.is_empty() {
            return Err(StageError("No expressions provided".to_string()));
        }
        let mut expressions = Vec::with_capacity(sources.len());
        for source in sources {
            let expression = ConditionalExpression::parse(source).map_err(|err| {
                StageError(format!("The expression '{source}' is invalid: {err}"))
            })?;
            expressions.push(expression);
        }
        let stats = expressions
            .iter()
            .map(|e| (e.print(), BTreeMap::new()))
            .collect();
        Ok(Self {
            dim: DimId::from_name(dim_name),
            dim_name: dim_name.to_string(),
            expressions,
            stats,
            prepared: false,
        })
    }

    fn ensure_prepared(&mut self, layout: &PointLayout) -> Result<(), StageError> {
        if !self.prepared {
            if layout.dim(&self.dim).is_none() {
                return Err(StageError(format!(
                    "Unknown dimension: {}",
                    self.dim.name()
                )));
            }
            for expression in &mut self.expressions {
                expression.prepare(layout).map_err(StageError)?;
            }
            self.prepared = true;
        }
        Ok(())
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
        self.ensure_prepared(input.layout().as_ref())?;

        // Process every point in the view.
        for idx in 0..input.len() {
            let value = input.get_f64(idx, &self.dim);
            for (expr_idx, expression) in self.expressions.iter().enumerate() {
                if expression.eval(input, idx) {
                    *self.stats[expr_idx].1.entry(value.to_bits()).or_default() += 1;
                }
            }
        }

        // Stats filters typically pass through the input view.
        Ok(vec![input.clone()])
    }

    fn metadata(&self) -> MetadataNode {
        let mut metadata = MetadataNode::new("metadata");
        metadata.add_value("dimension", MetadataValue::String(self.dim_name.clone()));
        for (position, (expression, bins)) in self.stats.iter().enumerate() {
            let mut statistic = MetadataNode::new("statistic");
            statistic.add_value("expression", MetadataValue::String(expression.clone()));
            statistic.add_value("position", MetadataValue::U64(position as u64));

            for (value_bits, count) in bins {
                let mut bin = MetadataNode::new("bins");
                bin.add_value("count", MetadataValue::U64(*count));
                bin.add_value("value", MetadataValue::F64(f64::from_bits(*value_bits)));
                statistic.add_child(bin);
            }
            metadata.add_child(statistic);
        }
        metadata
    }
}

impl Streamable for ExpressionStatsFilter {
    fn reset(&mut self) {
        for s in &mut self.stats {
            s.1.clear();
        }
    }

    fn process_one(&mut self, view: &mut PointView, idx: pdal_core::point::PointId) -> bool {
        if self.ensure_prepared(view.layout().as_ref()).is_err() {
            return true;
        }

        let value = view.get_f64(idx, &self.dim);
        for (expr_idx, expression) in self.expressions.iter().enumerate() {
            if expression.eval(view, idx) {
                *self.stats[expr_idx].1.entry(value.to_bits()).or_default() += 1;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdal_core::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    #[test]
    fn computes_bins_by_expression() {
        let mut layout = PointLayout::new();
        layout.register(DimId::X, DimType::F64);
        let mut view = PointView::new(Rc::new(layout));
        for value in [1.0, 1.0, 2.0, 3.0, 3.0, 4.0] {
            let idx = view.add_point();
            view.set_f64(idx, &DimId::X, value);
        }

        let mut filter =
            ExpressionStatsFilter::new("X", &["X < 3".to_string(), "X >= 3".to_string()]).unwrap();
        filter.run(&view).unwrap();
        let metadata = filter.metadata();

        assert_eq!(
            metadata
                .find_child("dimension")
                .and_then(MetadataNode::value),
            Some(&MetadataValue::String("X".into()))
        );
        let stats: Vec<&MetadataNode> = metadata
            .children()
            .iter()
            .filter(|node| node.name() == "statistic")
            .collect();
        assert_eq!(stats.len(), 2);
        assert_eq!(
            stats[0]
                .children()
                .iter()
                .filter(|node| node.name() == "bins")
                .count(),
            2
        );
        assert_eq!(
            stats[1]
                .children()
                .iter()
                .filter(|node| node.name() == "bins")
                .count(),
            2
        );
    }
}
