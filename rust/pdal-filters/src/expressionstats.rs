//! `filters.expressionstats` metadata calculation.

use pdal_core::expr::ConditionalExpression;
use pdal_core::metadata::{MetadataNode, MetadataValue};
use pdal_core::point::{DimId, PointView};
use pdal_core::stage::StageError;
use std::collections::BTreeMap;

pub struct ExpressionStatsFilter {
    dim: DimId,
    expressions: Vec<ConditionalExpression>,
}

impl ExpressionStatsFilter {
    pub fn new(dim_name: &str, sources: &[String]) -> Result<Self, StageError> {
        let mut expressions = Vec::with_capacity(sources.len());
        for source in sources {
            let expression = ConditionalExpression::parse(source).map_err(|err| {
                StageError(format!("The expression '{source}' is invalid: {err}"))
            })?;
            expressions.push(expression);
        }
        Ok(Self {
            dim: DimId::from_name(dim_name),
            expressions,
        })
    }

    pub fn metadata(
        &mut self,
        view: &PointView,
        dim_name: &str,
    ) -> Result<MetadataNode, StageError> {
        let layout = view.layout().as_ref();
        for expression in &mut self.expressions {
            expression.prepare(layout).map_err(StageError)?;
        }

        let mut stats: Vec<(String, BTreeMap<u64, u64>)> = self
            .expressions
            .iter()
            .map(|expression| (expression.print(), BTreeMap::new()))
            .collect();

        for idx in 0..view.len() {
            let value = view.get_f64(idx, &self.dim);
            for (expr_idx, expression) in self.expressions.iter().enumerate() {
                if expression.eval(view, idx) {
                    *stats[expr_idx].1.entry(value.to_bits()).or_default() += 1;
                }
            }
        }

        let mut metadata = MetadataNode::new("metadata");
        metadata.add_value("dimension", MetadataValue::String(dim_name.to_string()));
        for (position, (expression, bins)) in stats.into_iter().enumerate() {
            let mut statistic = MetadataNode::new("statistic");
            statistic.add_value("expression", MetadataValue::String(expression));
            statistic.add_value("position", MetadataValue::U64(position as u64));

            for (value_bits, count) in bins {
                let mut bin = MetadataNode::new("bins");
                bin.add_value("count", MetadataValue::U64(count));
                bin.add_value("value", MetadataValue::F64(f64::from_bits(value_bits)));
                statistic.add_child(bin);
            }
            metadata.add_child(statistic);
        }
        Ok(metadata)
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
        let metadata = filter.metadata(&view, "X").unwrap();

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
