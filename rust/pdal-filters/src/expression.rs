//! `filters.expression` -- pass points that satisfy a conditional expression.
//!
//! Port of `filters/ExpressionFilter.cpp`. The filter holds one or more
//! conditional expressions and produces one output view per expression: a
//! point lands in view `i` when expression `i` evaluates true for it.
//! In streaming mode (a single expression) it decides each point in turn.

use pdal_core::expr::ConditionalExpression;
use pdal_core::point::{PointId, PointLayout, PointView};
use pdal_core::stage::{Filter, StageError, Streamable};

/// The `filters.expression` stage.
pub struct ExpressionFilter {
    expressions: Vec<ConditionalExpression>,
    prepared: bool,
}

impl ExpressionFilter {
    /// Build the filter from expression source strings. A syntactically
    /// invalid expression is rejected here, mirroring PDAL, where option
    /// parsing rejects it.
    pub fn new(sources: &[String]) -> Result<Self, StageError> {
        let mut expressions = Vec::with_capacity(sources.len());
        for src in sources {
            let expr = ConditionalExpression::parse(src)
                .map_err(|e| StageError(format!("The expression '{src}' is invalid: {e}")))?;
            expressions.push(expr);
        }
        Ok(ExpressionFilter {
            expressions,
            prepared: false,
        })
    }

    /// Resolve every expression's identifiers against `layout`, once.
    fn ensure_prepared(&mut self, layout: &PointLayout) -> Result<(), StageError> {
        if !self.prepared {
            for expr in self.expressions.iter_mut() {
                expr.prepare(layout).map_err(StageError)?;
            }
            self.prepared = true;
        }
        Ok(())
    }
}

impl Filter for ExpressionFilter {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn name(&self) -> &str {
        "filters.expression"
    }

    fn run(&mut self, input: &PointView) -> Result<Vec<PointView>, StageError> {
        // Empty input yields no views (PDAL returns an empty set).
        if input.is_empty() {
            return Ok(Vec::new());
        }
        self.ensure_prepared(input.layout().as_ref())?;

        // One output view per expression.
        let mut views: Vec<PointView> = self.expressions.iter().map(|_| input.make_new()).collect();

        for idx in 0..input.len() {
            for (i, expr) in self.expressions.iter().enumerate() {
                if expr.eval(input, idx) {
                    views[i].append_point(input, idx);
                }
            }
        }

        Ok(views)
    }
}

impl Streamable for ExpressionFilter {
    /// Decide one streaming point. PDAL streams `filters.expression` only with
    /// a single expression; the C++ wrapper rejects other cases before
    /// delegating here, so anything else is defensively dropped.
    fn process_one(&mut self, view: &PointView, idx: PointId) -> bool {
        if self.ensure_prepared(view.layout().as_ref()).is_err() {
            return false;
        }
        match self.expressions.as_slice() {
            [expr] => expr.eval(view, idx),
            _ => false,
        }
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
    fn single_expression_keeps_matching_points() {
        let input = classified(&[1.0, 2.0, 2.0, 7.0]);
        let mut filter = ExpressionFilter::new(&["Classification == 2".to_string()]).unwrap();
        let out = filter.run(&input).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].len(), 2);
    }

    #[test]
    fn each_expression_gets_its_own_view() {
        let input = classified(&[1.0, 2.0, 2.0, 7.0]);
        let mut filter = ExpressionFilter::new(&[
            "Classification == 2".to_string(),
            "Classification >= 7".to_string(),
        ])
        .unwrap();
        let out = filter.run(&input).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].len(), 2); // two points classified 2
        assert_eq!(out[1].len(), 1); // one point classified 7
    }

    #[test]
    fn empty_input_yields_no_views() {
        let input = classified(&[]);
        let mut filter = ExpressionFilter::new(&["Classification == 2".to_string()]).unwrap();
        assert!(filter.run(&input).unwrap().is_empty());
    }

    #[test]
    fn syntactically_invalid_expression_is_rejected() {
        assert!(ExpressionFilter::new(&["Classification ==".to_string()]).is_err());
    }

    #[test]
    fn unknown_dimension_fails_at_run() {
        let input = classified(&[1.0]);
        let mut filter = ExpressionFilter::new(&["NoSuchDim == 1".to_string()]).unwrap();
        assert!(filter.run(&input).is_err());
    }

    #[test]
    fn streaming_evaluates_each_point() {
        let input = classified(&[1.0, 2.0, 7.0, 2.0]);
        let mut filter = ExpressionFilter::new(&["Classification == 2".to_string()]).unwrap();
        let kept: Vec<PointId> = (0..input.len())
            .filter(|&i| filter.process_one(&input, i))
            .collect();
        assert_eq!(kept, vec![1, 3]);
    }
}
