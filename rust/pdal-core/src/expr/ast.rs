//! Expression AST and evaluator.
//!
//! Port of `filters/private/expr/Expression.{hpp,cpp}`. The parser builds a
//! tree of [`Node`]s; [`Node::prepare`] resolves dimension identifiers against
//! a layout, and [`Node::eval`] evaluates the tree for one point.

use crate::point::{DimId, PointId, PointLayout, PointView};

/// The kind of an AST node.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeType {
    And,
    Or,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Negative,
    Value,
    Identifier,
    Function,
    None,
}

/// The result of evaluating a node.
///
/// Mirrors PDAL's `Result`: it carries both a numeric and a boolean slot, and
/// the parent node reads whichever slot its child's kind produces (value nodes
/// fill `dval`, logical nodes fill `bval`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvalValue {
    /// Numeric result (meaningful for value nodes).
    pub dval: f64,
    /// Boolean result (meaningful for logical nodes).
    pub bval: bool,
}

impl EvalValue {
    /// A numeric result.
    pub fn num(d: f64) -> Self {
        EvalValue {
            dval: d,
            bval: false,
        }
    }

    /// A boolean result.
    pub fn boolean(b: bool) -> Self {
        EvalValue {
            dval: 0.0,
            bval: b,
        }
    }
}

/// An expression AST node.
pub trait Node {
    /// The node kind.
    fn node_type(&self) -> NodeType;

    /// Whether the node yields a boolean (rather than numeric) result.
    fn is_bool(&self) -> bool;

    /// Whether the node yields a numeric result.
    fn is_value(&self) -> bool {
        !self.is_bool()
    }

    /// Render the node back to source-like text (diagnostic).
    fn print(&self) -> String;

    /// Resolve identifiers against `layout`; errors on an unknown dimension.
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String>;

    /// Evaluate the node for point `idx` of `view`.
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue;

    /// If this node is a numeric constant, its value -- used by the parser
    /// for constant folding. Non-constant nodes return `None`.
    fn const_value(&self) -> Option<f64> {
        None
    }
}

/// A boxed AST node -- the Rust analog of PDAL's `NodePtr`.
pub type NodePtr = Box<dyn Node>;

// ---------------------------------------------------------------------------
// Logical negation: !sub
// ---------------------------------------------------------------------------

/// Logical negation node (`!`).
pub struct NotNode {
    sub: NodePtr,
}

impl NotNode {
    /// Negate `sub`.
    pub fn new(sub: NodePtr) -> Self {
        NotNode { sub }
    }
}

impl Node for NotNode {
    fn node_type(&self) -> NodeType {
        NodeType::Not
    }
    fn is_bool(&self) -> bool {
        true
    }
    fn print(&self) -> String {
        format!("!({})", self.sub.print())
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.sub.prepare(layout)
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        EvalValue::boolean(!self.sub.eval(view, idx).bval)
    }
}

// ---------------------------------------------------------------------------
// Unary arithmetic negation: -sub
// ---------------------------------------------------------------------------

/// Unary numeric negation node (`-`).
pub struct UnMathNode {
    sub: NodePtr,
}

impl UnMathNode {
    /// Negate `sub`.
    pub fn new(sub: NodePtr) -> Self {
        UnMathNode { sub }
    }
}

impl Node for UnMathNode {
    fn node_type(&self) -> NodeType {
        NodeType::Negative
    }
    fn is_bool(&self) -> bool {
        false
    }
    fn print(&self) -> String {
        format!("-({})", self.sub.print())
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.sub.prepare(layout)
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        EvalValue::num(-self.sub.eval(view, idx).dval)
    }
}

// ---------------------------------------------------------------------------
// Binary arithmetic: left (+|-|*|/) right
// ---------------------------------------------------------------------------

/// Binary arithmetic node (`+`, `-`, `*`, `/`).
pub struct BinMathNode {
    ty: NodeType,
    left: NodePtr,
    right: NodePtr,
}

impl BinMathNode {
    /// A binary arithmetic node of kind `ty`.
    pub fn new(ty: NodeType, left: NodePtr, right: NodePtr) -> Self {
        BinMathNode { ty, left, right }
    }
}

impl Node for BinMathNode {
    fn node_type(&self) -> NodeType {
        self.ty
    }
    fn is_bool(&self) -> bool {
        false
    }
    fn print(&self) -> String {
        let op = match self.ty {
            NodeType::Add => "+",
            NodeType::Subtract => "-",
            NodeType::Multiply => "*",
            NodeType::Divide => "/",
            _ => "",
        };
        format!("({} {} {})", self.left.print(), op, self.right.print())
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.left.prepare(layout)?;
        self.right.prepare(layout)
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        let l = self.left.eval(view, idx).dval;
        let r = self.right.eval(view, idx).dval;
        EvalValue::num(match self.ty {
            NodeType::Add => l + r,
            NodeType::Subtract => l - r,
            NodeType::Multiply => l * r,
            // PDAL returns NaN on divide-by-zero rather than +/-inf.
            NodeType::Divide => {
                if r == 0.0 {
                    f64::NAN
                } else {
                    l / r
                }
            }
            _ => 0.0,
        })
    }
}

// ---------------------------------------------------------------------------
// Boolean combination: left (&&|||) right
// ---------------------------------------------------------------------------

/// Boolean combination node (`&&`, `||`).
pub struct BoolNode {
    ty: NodeType,
    left: NodePtr,
    right: NodePtr,
}

impl BoolNode {
    /// A boolean combination node of kind `ty`.
    pub fn new(ty: NodeType, left: NodePtr, right: NodePtr) -> Self {
        BoolNode { ty, left, right }
    }
}

impl Node for BoolNode {
    fn node_type(&self) -> NodeType {
        self.ty
    }
    fn is_bool(&self) -> bool {
        true
    }
    fn print(&self) -> String {
        let op = match self.ty {
            NodeType::And => "&&",
            NodeType::Or => "||",
            _ => "",
        };
        format!("({} {} {})", self.left.print(), op, self.right.print())
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.left.prepare(layout)?;
        self.right.prepare(layout)
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        // PDAL evaluates both sides (no short-circuit); eval has no
        // side effects, so this is behaviourally identical.
        let l = self.left.eval(view, idx).bval;
        let r = self.right.eval(view, idx).bval;
        EvalValue::boolean(match self.ty {
            NodeType::And => l && r,
            NodeType::Or => l || r,
            _ => false,
        })
    }
}

// ---------------------------------------------------------------------------
// Comparison: left (==|!=|<|<=|>|>=) right
// ---------------------------------------------------------------------------

/// Numeric comparison node (`==`, `!=`, `<`, `<=`, `>`, `>=`).
pub struct CompareNode {
    ty: NodeType,
    left: NodePtr,
    right: NodePtr,
}

impl CompareNode {
    /// A comparison node of kind `ty`.
    pub fn new(ty: NodeType, left: NodePtr, right: NodePtr) -> Self {
        CompareNode { ty, left, right }
    }
}

impl Node for CompareNode {
    fn node_type(&self) -> NodeType {
        self.ty
    }
    fn is_bool(&self) -> bool {
        true
    }
    fn print(&self) -> String {
        let op = match self.ty {
            NodeType::Equal => "==",
            NodeType::NotEqual => "!=",
            NodeType::Greater => ">",
            NodeType::GreaterEqual => ">=",
            NodeType::Less => "<",
            NodeType::LessEqual => "<=",
            _ => "",
        };
        format!("({}{}{})", self.left.print(), op, self.right.print())
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.left.prepare(layout)?;
        self.right.prepare(layout)
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        let l = self.left.eval(view, idx).dval;
        let r = self.right.eval(view, idx).dval;
        EvalValue::boolean(match self.ty {
            NodeType::Equal => l == r,
            NodeType::NotEqual => l != r,
            NodeType::Less => l < r,
            NodeType::LessEqual => l <= r,
            NodeType::Greater => l > r,
            NodeType::GreaterEqual => l >= r,
            _ => false,
        })
    }
}

// ---------------------------------------------------------------------------
// Function calls: name(sub)
// ---------------------------------------------------------------------------

/// A single-argument numeric function: `f64 -> f64`.
pub type Func1 = fn(f64) -> f64;
/// A single-argument predicate function: `f64 -> bool`.
pub type BoolFunc1 = fn(f64) -> bool;

/// Call of a numeric function of one numeric argument.
pub struct FuncNode {
    name: String,
    func: Func1,
    sub: NodePtr,
}

impl FuncNode {
    /// A call of `func` (named `name`) on `sub`.
    pub fn new(name: impl Into<String>, func: Func1, sub: NodePtr) -> Self {
        FuncNode {
            name: name.into(),
            func,
            sub,
        }
    }
}

impl Node for FuncNode {
    fn node_type(&self) -> NodeType {
        NodeType::Function
    }
    fn is_bool(&self) -> bool {
        false
    }
    fn print(&self) -> String {
        format!("{}({})", self.name, self.sub.print())
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.sub.prepare(layout)
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        EvalValue::num((self.func)(self.sub.eval(view, idx).dval))
    }
}

/// Call of a predicate function of one numeric argument.
pub struct BoolFuncNode {
    name: String,
    func: BoolFunc1,
    sub: NodePtr,
}

impl BoolFuncNode {
    /// A call of predicate `func` (named `name`) on `sub`.
    pub fn new(name: impl Into<String>, func: BoolFunc1, sub: NodePtr) -> Self {
        BoolFuncNode {
            name: name.into(),
            func,
            sub,
        }
    }
}

impl Node for BoolFuncNode {
    fn node_type(&self) -> NodeType {
        NodeType::Function
    }
    fn is_bool(&self) -> bool {
        true
    }
    fn print(&self) -> String {
        format!("{}({})", self.name, self.sub.print())
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        self.sub.prepare(layout)
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        EvalValue::boolean((self.func)(self.sub.eval(view, idx).dval))
    }
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// A numeric literal.
pub struct ConstValueNode {
    val: f64,
}

impl ConstValueNode {
    /// A numeric constant.
    pub fn new(val: f64) -> Self {
        ConstValueNode { val }
    }
    /// The constant's value.
    pub fn value(&self) -> f64 {
        self.val
    }
}

impl Node for ConstValueNode {
    fn node_type(&self) -> NodeType {
        NodeType::Value
    }
    fn is_bool(&self) -> bool {
        false
    }
    fn print(&self) -> String {
        // PDAL uses std::to_string (fixed 6 decimals); Rust's default
        // formatting is shorter but represents the same value.
        format!("{}", self.val)
    }
    fn prepare(&mut self, _layout: &PointLayout) -> Result<(), String> {
        Ok(())
    }
    fn eval(&self, _view: &PointView, _idx: PointId) -> EvalValue {
        EvalValue::num(self.val)
    }
    fn const_value(&self) -> Option<f64> {
        Some(self.val)
    }
}

/// A boolean literal.
pub struct ConstLogicalNode {
    val: bool,
}

impl ConstLogicalNode {
    /// A boolean constant.
    pub fn new(val: bool) -> Self {
        ConstLogicalNode { val }
    }
    /// The constant's value.
    pub fn value(&self) -> bool {
        self.val
    }
}

impl Node for ConstLogicalNode {
    fn node_type(&self) -> NodeType {
        NodeType::Value
    }
    fn is_bool(&self) -> bool {
        true
    }
    fn print(&self) -> String {
        if self.val { "true" } else { "false" }.to_string()
    }
    fn prepare(&mut self, _layout: &PointLayout) -> Result<(), String> {
        Ok(())
    }
    fn eval(&self, _view: &PointView, _idx: PointId) -> EvalValue {
        EvalValue::boolean(self.val)
    }
}

// ---------------------------------------------------------------------------
// Dimension reference
// ---------------------------------------------------------------------------

/// A reference to a point dimension by name.
pub struct VarNode {
    name: String,
    id: DimId,
}

impl VarNode {
    /// A reference to the dimension named `name`.
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let id = DimId::from_name(&name);
        VarNode { name, id }
    }

    /// The referenced dimension name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The resolved dimension id (valid after a successful `prepare`).
    pub fn dim(&self) -> &DimId {
        &self.id
    }
}

impl Node for VarNode {
    fn node_type(&self) -> NodeType {
        NodeType::Identifier
    }
    fn is_bool(&self) -> bool {
        false
    }
    fn print(&self) -> String {
        self.name.clone()
    }
    fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        if layout.dim(&self.id).is_none() {
            return Err(format!(
                "Unknown dimension '{}' in assignment.",
                self.name
            ));
        }
        Ok(())
    }
    fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        EvalValue::num(view.get_f64(idx, &self.id))
    }
}

// ---------------------------------------------------------------------------
// Expression: a parsed tree plus the parser's working node stack
// ---------------------------------------------------------------------------

/// A parsed expression.
///
/// During parsing this doubles as the parser's working stack of nodes; once
/// parsing completes the stack holds a single root node.
#[derive(Default)]
pub struct Expression {
    error: String,
    nodes: Vec<NodePtr>,
}

impl Expression {
    /// An empty expression.
    pub fn new() -> Self {
        Expression::default()
    }

    /// Discard all nodes and any error.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.error.clear();
    }

    /// Whether the expression holds at least one node.
    pub fn valid(&self) -> bool {
        !self.nodes.is_empty()
    }

    /// The current error message, if any.
    pub fn error(&self) -> &str {
        &self.error
    }

    /// Record an error message.
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = error.into();
    }

    /// Push a node onto the working stack.
    pub fn push_node(&mut self, node: NodePtr) {
        self.nodes.push(node);
    }

    /// Pop the top node off the working stack.
    pub fn pop_node(&mut self) -> Option<NodePtr> {
        self.nodes.pop()
    }

    /// The top (root) node, if any.
    pub fn top_node(&self) -> Option<&dyn Node> {
        self.nodes.last().map(|n| n.as_ref())
    }

    /// Render the expression to source-like text.
    pub fn print(&self) -> String {
        self.nodes.last().map(|n| n.print()).unwrap_or_default()
    }

    /// Resolve identifiers in the root node against `layout`.
    pub fn prepare(&mut self, layout: &PointLayout) -> Result<(), String> {
        match self.nodes.last_mut() {
            Some(node) => node.prepare(layout),
            None => Ok(()),
        }
    }

    /// Evaluate the root node for point `idx` of `view`.
    pub fn eval(&self, view: &PointView, idx: PointId) -> EvalValue {
        match self.nodes.last() {
            Some(node) => node.eval(view, idx),
            None => EvalValue::num(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::point::{DimType, PointLayout, PointView};
    use std::rc::Rc;

    /// A one-point view with `Classification` set to `cls`.
    fn point_with_classification(cls: f64) -> PointView {
        let mut layout = PointLayout::new();
        layout.register(DimId::Classification, DimType::U8);
        let mut view = PointView::new(Rc::new(layout));
        let p = view.add_point();
        view.set_f64(p, &DimId::Classification, cls);
        view
    }

    fn num(d: f64) -> NodePtr {
        Box::new(ConstValueNode::new(d))
    }

    #[test]
    fn arithmetic_and_divide_by_zero() {
        let view = point_with_classification(0.0);
        let add = BinMathNode::new(NodeType::Add, num(2.0), num(3.0));
        assert_eq!(add.eval(&view, 0).dval, 5.0);

        let div0 = BinMathNode::new(NodeType::Divide, num(1.0), num(0.0));
        assert!(div0.eval(&view, 0).dval.is_nan());
    }

    #[test]
    fn comparisons_and_logic() {
        let view = point_with_classification(0.0);
        let gt = CompareNode::new(NodeType::Greater, num(3.0), num(2.0));
        assert!(gt.eval(&view, 0).bval);

        let and = BoolNode::new(
            NodeType::And,
            Box::new(ConstLogicalNode::new(true)),
            Box::new(ConstLogicalNode::new(false)),
        );
        assert!(!and.eval(&view, 0).bval);

        let not = NotNode::new(Box::new(ConstLogicalNode::new(false)));
        assert!(not.eval(&view, 0).bval);
    }

    #[test]
    fn var_node_resolves_and_evaluates() {
        let view = point_with_classification(7.0);

        // Classification == 2  -> false for a point classified 7.
        let mut node = CompareNode::new(
            NodeType::Equal,
            Box::new(VarNode::new("Classification")),
            num(2.0),
        );
        node.prepare(view.layout().as_ref()).unwrap();
        assert!(!node.eval(&view, 0).bval);

        // Classification == 7  -> true.
        let mut node = CompareNode::new(
            NodeType::Equal,
            Box::new(VarNode::new("Classification")),
            num(7.0),
        );
        node.prepare(view.layout().as_ref()).unwrap();
        assert!(node.eval(&view, 0).bval);
    }

    #[test]
    fn prepare_rejects_unknown_dimension() {
        let view = point_with_classification(0.0);
        let mut node = VarNode::new("NoSuchDimension");
        assert!(node.prepare(view.layout().as_ref()).is_err());
    }

    #[test]
    fn function_node_applies_its_function() {
        let view = point_with_classification(0.0);
        let f = FuncNode::new("sqrt", f64::sqrt as Func1, num(16.0));
        assert_eq!(f.eval(&view, 0).dval, 4.0);
    }

    #[test]
    fn expression_stack_and_eval() {
        let view = point_with_classification(2.0);
        let mut expr = Expression::new();
        assert!(!expr.valid());

        let mut node = CompareNode::new(
            NodeType::Equal,
            Box::new(VarNode::new("Classification")),
            num(2.0),
        );
        node.prepare(view.layout().as_ref()).unwrap();
        expr.push_node(Box::new(node));

        assert!(expr.valid());
        assert!(expr.eval(&view, 0).bval);
        expr.clear();
        assert!(!expr.valid());
    }
}
