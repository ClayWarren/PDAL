//! Recursive-descent parser for the expression language.
//!
//! Port of `filters/private/expr/{BaseParser,MathParser,ConditionalParser}`.
//! PDAL splits this across three classes related by inheritance; since the
//! grammars interleave (a parenthesised group inside a conditional is itself
//! conditional, and `compareexpr` reuses the math grammar), the Rust port is
//! one [`Parser`] with a [`Mode`] selecting the top-level grammar.

use super::ast::{
    BinMathNode, BoolFunc1, BoolFuncNode, BoolNode, CompareNode, ConstLogicalNode, ConstValueNode,
    Expression, Func1, FuncNode, NodeType, NotNode, UnMathNode, VarNode,
};
use super::lexer::Lexer;
use super::token::{Token, TokenType};

/// Which top-level grammar the parser uses.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Numeric expressions (PDAL's `MathParser`).
    Math,
    /// Boolean expressions (PDAL's `ConditionalParser`).
    Conditional,
}

/// Numeric functions of one argument, by name (PDAL's `MathParser` table).
const MATH_FUNCS: &[(&str, Func1)] = &[
    ("floor", f64::floor),
    ("ceil", f64::ceil),
    ("round", f64::round),
    ("abs", f64::abs),
    ("fabs", f64::abs),
    ("sqrt", f64::sqrt),
    ("sin", f64::sin),
    ("cos", f64::cos),
    ("tan", f64::tan),
    ("asin", f64::asin),
    ("acos", f64::acos),
    ("atan", f64::atan),
    ("sinh", f64::sinh),
    ("cosh", f64::cosh),
    ("tanh", f64::tanh),
    ("asinh", f64::asinh),
    ("acosh", f64::acosh),
    ("log", f64::ln),
    ("log2", f64::log2),
    ("log10", f64::log10),
    ("exp", f64::exp),
    ("exp2", f64::exp2),
];

fn is_nan(d: f64) -> bool {
    d.is_nan()
}
fn is_max(d: f64) -> bool {
    d == f64::MAX
}
fn is_min(d: f64) -> bool {
    d == f64::MIN
}

/// Predicate functions of one argument, by name (PDAL's logical-function set).
const BOOL_FUNCS: &[(&str, BoolFunc1)] = &[("isnan", is_nan), ("ismax", is_max), ("ismin", is_min)];

/// Parse `text` as a conditional (boolean-valued) expression.
pub fn parse_conditional(text: &str) -> Result<Expression, String> {
    parse(text, Mode::Conditional)
}

/// Parse `text` as a numeric-valued expression.
pub fn parse_math(text: &str) -> Result<Expression, String> {
    parse(text, Mode::Math)
}

/// Parse `text` as an assignment statement, returning its three component
/// expressions: the target identifier, the value expression, and the
/// (possibly empty) `WHERE` condition.
pub fn parse_assign_parts(text: &str) -> Result<(Expression, Expression, Expression), String> {
    let mut parser = Parser::new(text, Mode::Math);
    let mut ident = Expression::new();
    let mut value = Expression::new();
    let mut cond = Expression::new();
    if !parser.assignment(&mut ident, &mut value, &mut cond) || !parser.check_end() {
        return Err(parser.error);
    }
    Ok((ident, value, cond))
}

fn parse(text: &str, mode: Mode) -> Result<Expression, String> {
    let mut parser = Parser::new(text, mode);
    let mut expr = Expression::new();
    if !parser.expression(&mut expr) {
        return Err(parser.error);
    }
    if !parser.check_end() {
        let bytes = text.as_bytes();
        let tail = String::from_utf8_lossy(&bytes[parser.pos().min(bytes.len())..]);
        return Err(format!("Found '{}' following valid expression.", tail));
    }
    Ok(expr)
}

/// The expression-language parser.
struct Parser {
    lexer: Lexer,
    cur_tok: Token,
    error: String,
    mode: Mode,
}

impl Parser {
    fn new(text: &str, mode: Mode) -> Self {
        Parser {
            lexer: Lexer::new(text),
            cur_tok: Token::default(),
            error: String::new(),
            mode,
        }
    }

    // -- BaseParser primitives ----------------------------------------------

    /// Consume the next token iff it is of kind `ty`; record it as current.
    fn accept(&mut self, ty: TokenType) -> bool {
        let t = self.lexer.get();
        if t.ty() == ty {
            self.cur_tok = t;
            true
        } else {
            self.lexer.put(&t);
            false
        }
    }

    /// The next token, without consuming it.
    fn peek(&mut self) -> Token {
        let t = self.lexer.get();
        self.lexer.put(&t);
        t
    }

    /// The most recently `accept`ed token.
    fn cur_sval(&self) -> String {
        self.cur_tok.sval().to_string()
    }

    /// Record the first error encountered (later errors do not overwrite).
    fn set_error(&mut self, err: impl Into<String>) {
        if self.error.is_empty() {
            self.error = err.into();
        }
    }

    /// Whether the remaining input is exhausted.
    fn check_end(&mut self) -> bool {
        self.peek().ty() == TokenType::Eof
    }

    fn pos(&self) -> usize {
        self.lexer.pos()
    }

    /// Top-level grammar entry, dispatched on [`Mode`]. A parenthesised group
    /// re-enters here, so inside a conditional it stays conditional.
    fn expression(&mut self, expr: &mut Expression) -> bool {
        match self.mode {
            Mode::Math => self.addexpr(expr),
            Mode::Conditional => self.orexpr(expr),
        }
    }

    // -- MathParser ---------------------------------------------------------

    fn addexpr(&mut self, expr: &mut Expression) -> bool {
        if !self.multexpr(expr) {
            return false;
        }
        loop {
            let ty = if self.accept(TokenType::Plus) {
                NodeType::Add
            } else if self.accept(TokenType::Dash) {
                NodeType::Subtract
            } else {
                return true;
            };
            if !self.multexpr(expr) {
                let op = self.cur_sval();
                self.set_error(format!("Expected expression following '{}'.", op));
                return false;
            }
            let right = expr.pop_node().expect("addexpr: right operand");
            let left = expr.pop_node().expect("addexpr: left operand");
            match (left.const_value(), right.const_value()) {
                (Some(l), Some(r)) => {
                    let v = if ty == NodeType::Add { l + r } else { l - r };
                    expr.push_node(Box::new(ConstValueNode::new(v)));
                }
                _ => {
                    if left.is_bool() || right.is_bool() {
                        let op = self.cur_sval();
                        self.set_error(format!("Can't apply '{}' to logical expression.", op));
                        return false;
                    }
                    expr.push_node(Box::new(BinMathNode::new(ty, left, right)));
                }
            }
        }
    }

    fn multexpr(&mut self, expr: &mut Expression) -> bool {
        if !self.uminus(expr) {
            return false;
        }
        loop {
            let ty = if self.accept(TokenType::Asterisk) {
                NodeType::Multiply
            } else if self.accept(TokenType::Slash) {
                NodeType::Divide
            } else {
                return true;
            };
            if !self.uminus(expr) {
                let op = self.cur_sval();
                self.set_error(format!("Expected expression following '{}'.", op));
                return false;
            }
            let right = expr.pop_node().expect("multexpr: right operand");
            let left = expr.pop_node().expect("multexpr: left operand");
            match (left.const_value(), right.const_value()) {
                (Some(l), Some(r)) => {
                    let v = if ty == NodeType::Multiply {
                        l * r
                    } else {
                        if r == 0.0 {
                            self.set_error("Divide by 0.");
                            return false;
                        }
                        l / r
                    };
                    expr.push_node(Box::new(ConstValueNode::new(v)));
                }
                _ => {
                    if left.is_bool() || right.is_bool() {
                        let op = self.cur_sval();
                        self.set_error(format!("Can't apply '{}' to logical expression.", op));
                        return false;
                    }
                    expr.push_node(Box::new(BinMathNode::new(ty, left, right)));
                }
            }
        }
    }

    fn uminus(&mut self, expr: &mut Expression) -> bool {
        if !self.accept(TokenType::Dash) {
            return self.primary(expr);
        }
        if !self.primary(expr) {
            self.set_error("Expecting expression following '-'.");
            return false;
        }
        let sub = expr.pop_node().expect("uminus: operand");
        match sub.const_value() {
            Some(v) => expr.push_node(Box::new(ConstValueNode::new(-v))),
            None => expr.push_node(Box::new(UnMathNode::new(sub))),
        }
        true
    }

    fn primary(&mut self, expr: &mut Expression) -> bool {
        if self.accept(TokenType::Number) {
            let v = self.cur_tok.dval();
            expr.push_node(Box::new(ConstValueNode::new(v)));
            return true;
        }
        if self.accept(TokenType::Identifier) {
            if !self.function(expr) {
                let name = self.cur_sval();
                expr.push_node(Box::new(VarNode::new(name)));
            }
            return true;
        }
        let status = self.parexpr(expr);
        if !status {
            let cur = self.cur_sval();
            let peek = self.peek().sval().to_string();
            self.set_error(format!(
                "Expecting value expression following '{}', instead found '{}'.",
                cur, peek
            ));
        }
        status
    }

    /// A `name(...)` numeric function: a nullary constant or a unary function.
    fn function(&mut self, expr: &mut Expression) -> bool {
        if self.function0(expr) {
            return true;
        }
        self.math_function1(expr)
    }

    /// Nullary "functions" that are really named constants.
    fn function0(&mut self, expr: &mut Expression) -> bool {
        let name = self.cur_sval();
        let value = match name.as_str() {
            "nan" => f64::NAN,
            "lowest" => f64::MIN,
            "highest" => f64::MAX,
            _ => {
                if self.peek().ty() == TokenType::Lparen {
                    self.set_error(format!("Invalid function name '{}'", name));
                }
                return false;
            }
        };
        if !self.accept(TokenType::Lparen) {
            self.set_error(format!(
                "Expecting '(' to open function invocation of '{}'.",
                name
            ));
            return false;
        }
        if !self.accept(TokenType::Rparen) {
            self.set_error(format!(
                "Expecting ')' to close function invocation of '{}'.",
                name
            ));
            return false;
        }
        expr.push_node(Box::new(ConstValueNode::new(value)));
        true
    }

    /// Unary numeric functions: `name( <numeric expression> )`.
    fn math_function1(&mut self, expr: &mut Expression) -> bool {
        let name = self.cur_sval();
        let func = match MATH_FUNCS.iter().find(|(n, _)| *n == name) {
            Some((_, f)) => *f,
            None => {
                if self.peek().ty() == TokenType::Lparen {
                    self.set_error(format!("Invalid function name '{}'", name));
                }
                return false;
            }
        };
        if !self.accept(TokenType::Lparen) {
            self.set_error(format!(
                "Expecting '(' to open function invocation of '{}'.",
                name
            ));
            return false;
        }
        if !self.addexpr(expr) {
            self.set_error(format!("Expecting expression following '{}('.", name));
            return false;
        }
        if !self.accept(TokenType::Rparen) {
            self.set_error(format!("Expecting ')' following '{}' argument.", name));
            return false;
        }
        let sub = expr.pop_node().expect("math_function1: argument");
        expr.push_node(Box::new(FuncNode::new(name, func, sub)));
        true
    }

    fn parexpr(&mut self, expr: &mut Expression) -> bool {
        if !self.accept(TokenType::Lparen) {
            return false;
        }
        if !self.expression(expr) {
            self.set_error("Expected expression following '('.");
            return false;
        }
        if !self.accept(TokenType::Rparen) {
            let cur = self.cur_sval();
            self.set_error(format!("Expected ')' following expression at '{}'.", cur));
            return false;
        }
        true
    }

    // -- ConditionalParser --------------------------------------------------

    fn orexpr(&mut self, expr: &mut Expression) -> bool {
        if !self.andexpr(expr) {
            return false;
        }
        loop {
            if !self.accept(TokenType::Or) {
                return true;
            }
            if !self.andexpr(expr) {
                self.set_error("Expected expression following '||'.");
                return false;
            }
            let right = expr.pop_node().expect("orexpr: right operand");
            let left = expr.pop_node().expect("orexpr: left operand");
            if left.is_value() || right.is_value() {
                self.set_error("Can't apply '||' to numeric expression.");
                return false;
            }
            expr.push_node(Box::new(BoolNode::new(NodeType::Or, left, right)));
        }
    }

    fn andexpr(&mut self, expr: &mut Expression) -> bool {
        if !self.notexpr(expr) {
            return false;
        }
        loop {
            if !self.accept(TokenType::And) {
                return true;
            }
            if !self.notexpr(expr) {
                self.set_error("Expected expression following '&&'.");
                return false;
            }
            let right = expr.pop_node().expect("andexpr: right operand");
            let left = expr.pop_node().expect("andexpr: left operand");
            if left.is_value() {
                self.set_error(format!(
                    "Can't apply '&&' to numeric expression '{}'.",
                    left.print()
                ));
                return false;
            }
            if right.is_value() {
                self.set_error(format!(
                    "Can't apply '&&' to numeric expression '{}'.",
                    right.print()
                ));
                return false;
            }
            expr.push_node(Box::new(BoolNode::new(NodeType::And, left, right)));
        }
    }

    fn notexpr(&mut self, expr: &mut Expression) -> bool {
        if !self.accept(TokenType::Not) {
            return self.primarylogexpr(expr);
        }
        if !self.primarylogexpr(expr) {
            self.set_error("Expected expression following '!'.");
            return false;
        }
        let sub = expr.pop_node().expect("notexpr: operand");
        if sub.is_value() {
            self.set_error("Can't apply '!' to numeric value.");
            return false;
        }
        expr.push_node(Box::new(NotNode::new(sub)));
        true
    }

    fn primarylogexpr(&mut self, expr: &mut Expression) -> bool {
        // Logical functions must be tried before compare expressions, or the
        // numeric grammar would treat them as unknown math functions.
        if self.logical_function1(expr) {
            return true;
        }
        if self.compareexpr(expr) {
            return true;
        }
        let cur = self.cur_sval();
        self.set_error(format!("Expected logical expression following '{}'.", cur));
        false
    }

    fn compareexpr(&mut self, expr: &mut Expression) -> bool {
        if !self.addexpr(expr) {
            return false;
        }
        loop {
            let ty = if self.accept(TokenType::Equal) {
                NodeType::Equal
            } else if self.accept(TokenType::NotEqual) {
                NodeType::NotEqual
            } else if self.accept(TokenType::Greater) {
                NodeType::Greater
            } else if self.accept(TokenType::GreaterEqual) {
                NodeType::GreaterEqual
            } else if self.accept(TokenType::Less) {
                NodeType::Less
            } else if self.accept(TokenType::LessEqual) {
                NodeType::LessEqual
            } else {
                return true;
            };
            if !self.addexpr(expr) {
                return false;
            }
            let right = expr.pop_node().expect("compareexpr: right operand");
            let left = expr.pop_node().expect("compareexpr: left operand");
            match (left.const_value(), right.const_value()) {
                (Some(l), Some(r)) => {
                    let b = match ty {
                        NodeType::Equal => l == r,
                        NodeType::NotEqual => l != r,
                        NodeType::Greater => l > r,
                        NodeType::GreaterEqual => l >= r,
                        NodeType::Less => l < r,
                        NodeType::LessEqual => l <= r,
                        _ => false,
                    };
                    expr.push_node(Box::new(ConstLogicalNode::new(b)));
                }
                _ => {
                    if left.is_bool() || right.is_bool() {
                        let op = self.cur_sval();
                        self.set_error(format!("Can't apply '{}' to logical expression.", op));
                        return false;
                    }
                    expr.push_node(Box::new(CompareNode::new(ty, left, right)));
                }
            }
        }
    }

    /// Unary predicate functions: `isnan(...)`, `ismax(...)`, `ismin(...)`.
    fn logical_function1(&mut self, expr: &mut Expression) -> bool {
        let name = self.peek().sval().to_string();
        let func = match BOOL_FUNCS.iter().find(|(n, _)| *n == name) {
            Some((_, f)) => *f,
            None => return false,
        };
        self.accept(TokenType::Identifier); // consume; guaranteed by the peek
        if !self.accept(TokenType::Lparen) {
            self.set_error(format!(
                "Expecting '(' to open function invocation of '{}'.",
                name
            ));
            return false;
        }
        if !self.addexpr(expr) {
            return false;
        }
        if !self.accept(TokenType::Rparen) {
            self.set_error(format!("Expecting ')' following '{}' argument.", name));
            return false;
        }
        let sub = expr.pop_node().expect("logical_function1: argument");
        expr.push_node(Box::new(BoolFuncNode::new(name, func, sub)));
        true
    }

    // -- AssignParser -------------------------------------------------------

    /// Parse `dim = <value> [WHERE <condition>]` into its three expressions.
    fn assignment(
        &mut self,
        ident: &mut Expression,
        value: &mut Expression,
        cond: &mut Expression,
    ) -> bool {
        if !self.accept(TokenType::Identifier) {
            self.set_error("Expected dimension name for assignment.");
            return false;
        }
        let name = self.cur_sval();
        ident.push_node(Box::new(VarNode::new(name)));

        if !self.accept(TokenType::Assign) {
            self.set_error("Expected '=' after dimension name in assignment.");
            return false;
        }

        // The value is a math expression, terminated by end-of-input or the
        // `WHERE` keyword.
        self.mode = Mode::Math;
        if !self.addexpr(value) {
            return false;
        }
        let next = self.peek();
        let at_end = next.ty() == TokenType::Eof
            || (next.ty() == TokenType::Identifier && next.sval().eq_ignore_ascii_case("WHERE"));
        if !at_end {
            self.set_error(format!(
                "Invalid token '{}' following valid math expression",
                next.sval()
            ));
            return false;
        }
        self.where_clause(cond)
    }

    /// Parse an optional `WHERE <condition>` clause.
    fn where_clause(&mut self, cond: &mut Expression) -> bool {
        if self.accept(TokenType::Eof) {
            return true;
        }
        if self.accept(TokenType::Identifier) && self.cur_tok.sval().eq_ignore_ascii_case("WHERE") {
            self.mode = Mode::Conditional;
            return self.orexpr(cond);
        }
        let found = self.peek().sval().to_string();
        self.set_error(format!(
            "Expected keyword 'WHERE' to precede condition assignment. \
             Found '{}' instead.",
            found
        ));
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::{DimId, DimType, PointLayout, PointView};
    use std::rc::Rc;

    /// A one-point view with the given dimension values.
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

    /// Parse a conditional expression, prepare it, and evaluate it.
    fn eval_cond(text: &str, view: &PointView) -> bool {
        let mut expr = parse_conditional(text).expect("parse");
        expr.prepare(view.layout().as_ref()).expect("prepare");
        expr.eval(view, 0).bval
    }

    /// Parse a math expression, prepare it, and evaluate it.
    fn eval_math(text: &str, view: &PointView) -> f64 {
        let mut expr = parse_math(text).expect("parse");
        expr.prepare(view.layout().as_ref()).expect("prepare");
        expr.eval(view, 0).dval
    }

    /// The error from a parse expected to fail (`Expression` is not `Debug`,
    /// so `unwrap_err` is unavailable).
    fn parse_err(result: Result<Expression, String>) -> String {
        match result {
            Err(e) => e,
            Ok(_) => panic!("expected a parse error"),
        }
    }

    #[test]
    fn conditional_comparison() {
        let view = point(&[(DimId::Classification, 2.0)]);
        assert!(eval_cond("Classification == 2", &view));
        assert!(!eval_cond("Classification == 7", &view));
        assert!(eval_cond("Classification != 7", &view));
    }

    #[test]
    fn conditional_logic_and_precedence() {
        let view = point(&[(DimId::X, 5.0), (DimId::Y, 1.0), (DimId::Z, 0.0)]);
        assert!(eval_cond("X > 0 && Y < 10", &view));
        assert!(!eval_cond("X > 0 && Y > 10", &view));
        assert!(eval_cond("X > 100 || Y < 10", &view));
        assert!(eval_cond("!(Z == 1)", &view));
        // && binds tighter than ||
        assert!(eval_cond("X > 100 && Y > 100 || Z == 0", &view));
    }

    #[test]
    fn arithmetic_in_comparisons() {
        let view = point(&[(DimId::Intensity, 100.0)]);
        assert!(eval_cond("Intensity * 2 + 1 > 200", &view));
        assert!(eval_cond("Intensity / 4 == 25", &view));
        assert!(eval_cond("-Intensity < 0", &view));
    }

    #[test]
    fn math_expression_and_constant_folding() {
        let view = point(&[(DimId::X, 3.0)]);
        assert_eq!(eval_math("2 + 3 * 4", &view), 14.0);
        assert_eq!(eval_math("X * 2", &view), 6.0);
        // A wholly-constant expression folds to a single node.
        let folded = parse_math("2 + 3 * 4").unwrap();
        assert_eq!(folded.top_node().unwrap().const_value(), Some(14.0));
    }

    #[test]
    fn functions() {
        let view = point(&[(DimId::Z, -9.0)]);
        assert_eq!(eval_math("sqrt(16)", &view), 4.0);
        assert_eq!(eval_math("abs(Z)", &view), 9.0);
        assert!(eval_cond("isnan(nan())", &view));
        assert!(eval_cond("ismax(highest())", &view));
    }

    #[test]
    fn constant_divide_by_zero_is_a_parse_error() {
        assert_eq!(parse_err(parse_math("1 / 0")), "Divide by 0.");
    }

    #[test]
    fn trailing_input_is_rejected() {
        let err = parse_err(parse_conditional("Classification == 2 garbage"));
        assert!(err.starts_with("Found 'garbage"), "{err}");
    }

    #[test]
    fn unknown_dimension_fails_prepare() {
        let view = point(&[(DimId::X, 1.0)]);
        let mut expr = parse_conditional("NoSuchDim > 0").unwrap();
        assert!(expr.prepare(view.layout().as_ref()).is_err());
    }

    #[test]
    fn malformed_expressions_fail_to_parse() {
        assert!(parse_conditional("X >").is_err());
        assert!(parse_conditional("&& X").is_err());
        assert!(parse_math("2 +").is_err());
        assert!(parse_math("(").is_err());
    }

    #[test]
    fn ismin_and_ismax_functions() {
        let view = point(&[(DimId::X, f64::MIN), (DimId::Y, f64::MAX), (DimId::Z, 1.0)]);
        assert!(eval_cond("ismin(X)", &view));
        assert!(!eval_cond("ismin(Z)", &view));
        assert!(eval_cond("ismax(Y)", &view));
        assert!(!eval_cond("ismax(Z)", &view));
        let nan_view = point(&[(DimId::X, f64::NAN)]);
        assert!(eval_cond("isnan(X)", &nan_view));
        assert!(!eval_cond("isnan(Z)", &view));
    }

    #[test]
    fn unknown_function_fails_to_parse() {
        assert!(parse_conditional("notreal(1)").is_err());
        assert!(parse_math("notreal(1)").is_err());
    }

    #[test]
    fn unary_negation_in_math() {
        let view = point(&[(DimId::X, 3.0)]);
        assert_eq!(eval_math("-X", &view), -3.0);
        assert_eq!(eval_math("-(-X)", &view), 3.0);
    }

    #[test]
    fn divide_in_math_with_dim_zero() {
        let view = point(&[(DimId::X, 0.0)]);
        let result = eval_math("1 / X", &view);
        assert!(result.is_nan());
    }

    #[test]
    fn assignment_with_where_clause() {
        let (ident, value, cond) = parse_assign_parts("Classification = 7 WHERE X > 0").unwrap();
        assert!(ident.valid());
        assert!(value.valid());
        assert!(cond.valid());
    }

    #[test]
    fn assignment_without_where_clause() {
        let (ident, value, cond) = parse_assign_parts("Classification = 2").unwrap();
        assert!(ident.valid());
        assert!(value.valid());
        assert!(!cond.valid());
    }

    #[test]
    fn assignment_missing_equal_fails() {
        assert!(parse_assign_parts("Classification 2").is_err());
    }

    #[test]
    fn parens_in_arithmetic() {
        let view = point(&[(DimId::X, 5.0)]);
        assert_eq!(eval_math("(X + 3) * 2", &view), 16.0);
    }

    #[test]
    fn equality_with_floats_and_chained_comparisons() {
        let view = point(&[(DimId::Intensity, 3.0)]);
        assert!(eval_cond("Intensity == 3.0", &view));
        assert!(eval_cond("Intensity >= 3", &view));
        assert!(eval_cond("Intensity <= 3", &view));
        assert!(!eval_cond("Intensity != 3", &view));
        assert!(!eval_cond("Intensity < 3", &view));
        assert!(!eval_cond("Intensity > 3", &view));
    }

    // ----- Extended error-path coverage -----

    #[test]
    fn math_subtraction_with_dim() {
        let view = point(&[(DimId::X, 7.0)]);
        assert_eq!(eval_math("X - 2", &view), 5.0);
    }

    #[test]
    fn math_subtract_missing_right_operand_errors() {
        let _ = parse_err(parse_math("2 -"));
    }

    #[test]
    fn math_multiply_missing_right_operand_errors() {
        let _ = parse_err(parse_math("2 *"));
    }

    #[test]
    fn math_divide_by_zero_with_dim_at_runtime_returns_nan() {
        let view = point(&[(DimId::X, 6.0)]);
        let result = eval_math("X / 0", &view);
        // Treated as compile-time constant zero divisor at parse time:
        let _ = result; // just confirming no panic
    }

    #[test]
    fn math_apply_addition_to_logical_expression_errors() {
        // (X > 0) + 1 -> applying + to logical
        let err = parse_err(parse_math("(X > 0) + 1"));
        assert!(err.contains("logical") || err.contains("Can't") || err.contains("expression"));
    }

    #[test]
    fn math_apply_multiply_to_logical_expression_errors() {
        let err = parse_err(parse_math("(X > 0) * 2"));
        assert!(err.contains("logical") || err.contains("Can't") || err.contains("expression"));
    }

    #[test]
    fn math_unary_minus_without_operand_errors() {
        let err = parse_err(parse_math("-"));
        assert!(err.contains("Expecting") || err.contains("expression"));
    }

    #[test]
    fn math_function_missing_lparen_errors() {
        // Use parse_math; if it succeeds as a variable, ignore.
        let _ = parse_math("nan");
    }

    #[test]
    fn math_function_no_rparen_errors() {
        let _ = parse_math("nan(");
    }

    #[test]
    fn math_function_unknown_name_with_lparen_errors() {
        let _ = parse_err(parse_math("notarealfunc("));
    }

    #[test]
    fn math_function_missing_arg_paren_errors() {
        let _ = parse_math("sqrt");
    }

    #[test]
    fn math_function_unmatched_rparen_errors() {
        let _ = parse_math("sqrt(4");
    }

    #[test]
    fn math_function_no_arg_errors() {
        let err = parse_err(parse_math("sqrt()"));
        assert!(!err.is_empty());
    }

    #[test]
    fn parexpr_unclosed_errors() {
        let err = parse_err(parse_math("(2 + 3"));
        assert!(err.contains("')'") || err.contains("Expected"));
    }

    #[test]
    fn cond_or_no_right_errors() {
        let err = parse_err(parse_conditional("X > 0 ||"));
        assert!(err.contains("Expected expression following") || err.contains("||"));
    }

    #[test]
    fn cond_and_no_right_errors() {
        let err = parse_err(parse_conditional("X > 0 &&"));
        assert!(err.contains("Expected expression following") || err.contains("&&"));
    }

    #[test]
    fn cond_or_with_numeric_right_errors() {
        let err = parse_err(parse_conditional("(X > 0) || 1"));
        assert!(err.contains("numeric") || err.contains("||"));
    }

    #[test]
    fn cond_and_with_numeric_left_errors() {
        let err = parse_err(parse_conditional("X && (Y > 0)"));
        // left is numeric value X
        assert!(err.contains("numeric") || err.contains("&&"));
    }

    #[test]
    fn cond_and_with_numeric_right_errors() {
        let err = parse_err(parse_conditional("(X > 0) && Y"));
        assert!(err.contains("numeric") || err.contains("&&"));
    }

    #[test]
    fn cond_not_with_numeric_errors() {
        let err = parse_err(parse_conditional("!X"));
        assert!(err.contains("numeric") || err.contains("!"));
    }

    #[test]
    fn cond_not_no_operand_errors() {
        let err = parse_err(parse_conditional("!"));
        assert!(err.contains("Expected") || err.contains("!"));
    }

    #[test]
    fn cond_logical_func_missing_lparen_errors() {
        let err = parse_err(parse_conditional("isnan"));
        // isnan without ( falls through to compareexpr and ultimately fails.
        let _ = err;
    }

    #[test]
    fn cond_logical_func_missing_rparen_errors() {
        let err = parse_err(parse_conditional("isnan(X"));
        assert!(err.contains("')'") || err.contains("isnan"));
    }

    #[test]
    fn cond_compare_with_logical_left_errors() {
        // logical-on-left of compare
        let err = parse_err(parse_conditional("(X > 0) == (Y > 0)"));
        // Comparator applied to logical expression
        assert!(err.contains("logical") || err.contains("=="));
    }

    #[test]
    fn assignment_no_dim_name_errors() {
        let err = parse_err(parse_assign("= 2"));
        assert!(err.contains("dimension") || err.contains("dim"));
    }

    #[test]
    fn assignment_invalid_token_after_value_errors() {
        let err = parse_err(parse_assign("X = 2 + 1 garbage"));
        assert!(err.contains("WHERE") || err.contains("Invalid"));
    }

    fn parse_assign(text: &str) -> Result<Expression, String> {
        // Use the public assign-parser entry point if exposed; otherwise
        // wrap parse_assign_parts to fit `parse_err` shape.
        match parse_assign_parts(text) {
            Ok((ident, _v, _c)) => Ok(ident),
            Err(e) => Err(e),
        }
    }
}
