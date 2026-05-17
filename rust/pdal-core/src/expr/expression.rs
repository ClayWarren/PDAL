//! Typed expression wrappers.
//!
//! Port of `filters/private/expr/{ConditionalExpression, MathExpression,
//! IdentExpression, AssignStatement}`. Each wraps a parsed [`Expression`] with
//! a type-checked `prepare` and a typed `eval`.

use super::ast::Expression;
use super::parser::{parse_assign_parts, parse_conditional, parse_math};
use crate::point::{DimId, PointId, PointLayout, PointView};

/// A boolean-valued expression (PDAL's `ConditionalExpression`).
#[derive(Default)]
pub struct ConditionalExpression {
    expr: Expression,
}

impl ConditionalExpression {
    /// Parse `text` as a conditional expression.
    pub fn parse(text: &str) -> Result<Self, String> {
        Ok(ConditionalExpression {
            expr: parse_conditional(text)?,
        })
    }

    fn wrap(expr: Expression) -> Self {
        ConditionalExpression { expr }
    }

    /// Whether the expression holds a parsed tree.
    pub fn valid(&self) -> bool {
        self.expr.valid()
    }

    /// Render the expression to source-like text.
    pub fn print(&self) -> String {
        self.expr.print()
    }

    /// Resolve identifiers, and reject non-boolean or always-constant
    /// expressions.
    pub fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.expr.prepare(layout)?;
        if let Some(top) = self.expr.top_node() {
            if top.is_value() {
                return Err("Expression evaluates to a value, not a boolean.".into());
            }
            if let Some(value) = top.const_logical() {
                return Err(if value {
                    "Expression is always true."
                } else {
                    "Expression is always false."
                }
                .into());
            }
        }
        Ok(())
    }

    /// Evaluate the predicate for point `idx`. An empty expression is `true`.
    pub fn eval(&self, view: &PointView, idx: PointId) -> bool {
        match self.expr.top_node() {
            Some(node) => node.eval(view, idx).bval,
            None => true,
        }
    }
}

/// A numeric-valued expression (PDAL's `MathExpression`).
#[derive(Default)]
pub struct MathExpression {
    expr: Expression,
}

impl MathExpression {
    /// Parse `text` as a numeric expression.
    pub fn parse(text: &str) -> Result<Self, String> {
        Ok(MathExpression {
            expr: parse_math(text)?,
        })
    }

    fn wrap(expr: Expression) -> Self {
        MathExpression { expr }
    }

    /// Whether the expression holds a parsed tree.
    pub fn valid(&self) -> bool {
        self.expr.valid()
    }

    /// Render the expression to source-like text.
    pub fn print(&self) -> String {
        self.expr.print()
    }

    /// Resolve identifiers, and reject non-numeric expressions.
    pub fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.expr.prepare(layout)?;
        if let Some(top) = self.expr.top_node() {
            if !top.is_value() {
                return Err("Expression doesn't evaluate to a value.".into());
            }
        }
        Ok(())
    }

    /// Evaluate the expression for point `idx`. An empty expression is `0.0`.
    pub fn eval(&self, view: &PointView, idx: PointId) -> f64 {
        match self.expr.top_node() {
            Some(node) => node.eval(view, idx).dval,
            None => 0.0,
        }
    }
}

/// A bare dimension reference (PDAL's `IdentExpression`).
#[derive(Default)]
pub struct IdentExpression {
    expr: Expression,
}

impl IdentExpression {
    fn wrap(expr: Expression) -> Self {
        IdentExpression { expr }
    }

    /// Whether the expression holds a parsed tree.
    pub fn valid(&self) -> bool {
        self.expr.valid()
    }

    /// Render the expression to source-like text.
    pub fn print(&self) -> String {
        self.expr.print()
    }

    /// Resolve the identifier against `layout`. An empty identifier fails.
    pub fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        if self.expr.top_node().is_none() {
            return Err(String::new());
        }
        self.expr.prepare(layout)
    }

    /// The referenced dimension name, or empty if there is no identifier.
    pub fn name(&self) -> String {
        self.expr
            .top_node()
            .and_then(|n| n.as_var())
            .map(|v| v.name().to_string())
            .unwrap_or_default()
    }

    /// The referenced dimension, available after a successful `prepare`.
    pub fn dim(&self) -> Option<DimId> {
        self.expr
            .top_node()
            .and_then(|n| n.as_var())
            .map(|v| v.dim().clone())
    }
}

/// A `dim = value [WHERE condition]` assignment statement (PDAL's
/// `AssignStatement`), used by `filters.assign`.
#[derive(Default)]
pub struct AssignStatement {
    ident: IdentExpression,
    value: MathExpression,
    conditional: ConditionalExpression,
}

impl AssignStatement {
    /// Parse `text` as an assignment statement.
    pub fn parse(text: &str) -> Result<Self, String> {
        let (ident, value, cond) = parse_assign_parts(text)?;
        Ok(AssignStatement {
            ident: IdentExpression::wrap(ident),
            value: MathExpression::wrap(value),
            conditional: ConditionalExpression::wrap(cond),
        })
    }

    /// The assignment target dimension.
    pub fn ident(&self) -> &IdentExpression {
        &self.ident
    }

    /// The value assigned to the target.
    pub fn value(&self) -> &MathExpression {
        &self.value
    }

    /// The `WHERE` condition; empty (always-true) when there is no clause.
    pub fn condition(&self) -> &ConditionalExpression {
        &self.conditional
    }

    /// Whether the statement parsed to a valid assignment.
    pub fn valid(&self) -> bool {
        self.ident.valid()
    }

    /// Resolve all three component expressions against `layout`.
    pub fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.ident.prepare(layout)?;
        self.value.prepare(layout)?;
        self.conditional.prepare(layout)
    }

    /// A multi-line debug rendering.
    pub fn print(&self) -> String {
        format!(
            "Ident = {}\nValue = {}\nCondition = {}\n",
            self.ident.print(),
            self.value.print(),
            self.conditional.print()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    fn point(dims: &[(DimId, f64)]) -> PointView {
        let mut layout = PointLayout::new();
        for (id, _) in dims {
            layout.register(id.clone(), DimType::F64);
        }
        let mut view = PointView::new(Rc::new(layout));
        let p = view.add_point();
        for (id, v) in dims {
            view.set_f64(p, id, *v);
        }
        view
    }

    #[test]
    fn conditional_expression_evaluates() {
        let view = point(&[(DimId::Classification, 2.0)]);
        let mut cond = ConditionalExpression::parse("Classification == 2").unwrap();
        cond.prepare(view.layout().as_ref()).unwrap();
        assert!(cond.eval(&view, 0));
    }

    #[test]
    fn conditional_rejects_value_and_constant_expressions() {
        let view = point(&[(DimId::X, 1.0)]);

        // A bare numeric expression is not a boolean.
        let mut value = ConditionalExpression::parse("X").unwrap();
        assert_eq!(
            value.prepare(view.layout().as_ref()).unwrap_err(),
            "Expression evaluates to a value, not a boolean."
        );

        // A wholly-constant predicate folds and is rejected.
        let mut always = ConditionalExpression::parse("1 < 2").unwrap();
        assert_eq!(
            always.prepare(view.layout().as_ref()).unwrap_err(),
            "Expression is always true."
        );
    }

    #[test]
    fn math_expression_evaluates() {
        let view = point(&[(DimId::X, 3.0)]);
        let mut math = MathExpression::parse("X * 2 + 1").unwrap();
        math.prepare(view.layout().as_ref()).unwrap();
        assert_eq!(math.eval(&view, 0), 7.0);
    }

    #[test]
    fn assignment_without_where_clause() {
        let view = point(&[(DimId::Classification, 0.0), (DimId::Z, 5.0)]);
        let mut stmt = AssignStatement::parse("Classification = 2").unwrap();
        assert!(stmt.valid());
        stmt.prepare(view.layout().as_ref()).unwrap();

        assert_eq!(stmt.ident().dim(), Some(DimId::Classification));
        assert_eq!(stmt.value().eval(&view, 0), 2.0);
        // No WHERE clause -> the condition always matches.
        assert!(stmt.condition().eval(&view, 0));
    }

    #[test]
    fn assignment_with_where_clause() {
        let view = point(&[
            (DimId::Classification, 2.0),
            (DimId::X, 4.0),
            (DimId::Z, 0.0),
        ]);
        let mut stmt =
            AssignStatement::parse("Z = X * 2 WHERE Classification == 2").unwrap();
        stmt.prepare(view.layout().as_ref()).unwrap();

        assert_eq!(stmt.ident().dim(), Some(DimId::Z));
        assert_eq!(stmt.value().eval(&view, 0), 8.0);
        assert!(stmt.condition().eval(&view, 0));
    }

    #[test]
    fn malformed_assignments_fail() {
        assert!(AssignStatement::parse("= 5").is_err());
        assert!(AssignStatement::parse("Classification 2").is_err());
        assert!(AssignStatement::parse("Classification = 2 garbage").is_err());
    }
}
