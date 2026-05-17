//! MongoDB-style query expression parser and evaluator.
//!
//! Port of `filters/private/mongoexpression/`.

use serde_json::Value;
use crate::point::{DimId, PointId, PointView};
use std::fmt;

/// A logical operator for combining filters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
    Not,
    Nor,
}

impl LogicalOp {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "$and" => Some(LogicalOp::And),
            "$or" => Some(LogicalOp::Or),
            "$not" => Some(LogicalOp::Not),
            "$nor" => Some(LogicalOp::Nor),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            LogicalOp::And => "$and",
            LogicalOp::Or => "$or",
            LogicalOp::Not => "$not",
            LogicalOp::Nor => "$nor",
        }
    }
}

/// A comparison operator for comparing a dimension to a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOp {
    Eq,
    Gt,
    Gte,
    Lt,
    Lte,
    Ne,
    In,
    Nin,
}

impl ComparisonOp {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "$eq" => Some(ComparisonOp::Eq),
            "$gt" => Some(ComparisonOp::Gt),
            "$gte" => Some(ComparisonOp::Gte),
            "$lt" => Some(ComparisonOp::Lt),
            "$lte" => Some(ComparisonOp::Lte),
            "$ne" => Some(ComparisonOp::Ne),
            "$in" => Some(ComparisonOp::In),
            "$nin" => Some(ComparisonOp::Nin),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ComparisonOp::Eq => "$eq",
            ComparisonOp::Gt => "$gt",
            ComparisonOp::Gte => "$gte",
            ComparisonOp::Lt => "$lt",
            ComparisonOp::Lte => "$lte",
            ComparisonOp::Ne => "$ne",
            ComparisonOp::In => "$in",
            ComparisonOp::Nin => "$nin",
        }
    }
}

/// The RHS of a comparison: either a constant value or another dimension.
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    Value(f64),
    Dim(DimId),
}

impl Operand {
    pub fn from_json(v: &Value) -> Result<Self, String> {
        if v.is_number() {
            Ok(Operand::Value(v.as_f64().unwrap_or(0.0)))
        } else if v.is_string() {
            Ok(Operand::Dim(DimId::from_name(v.as_str().unwrap())))
        } else {
            Err(format!("Invalid comparison operand: {}", v))
        }
    }

    pub fn get(&self, view: &PointView, idx: PointId) -> f64 {
        match self {
            Operand::Value(v) => *v,
            Operand::Dim(dim) => view.get_f64(idx, dim),
        }
    }
}

/// An AST node for the Mongo expression.
#[derive(Debug, Clone, PartialEq)]
pub enum MongoNode {
    Logical {
        op: LogicalOp,
        children: Vec<MongoNode>,
    },
    Comparison {
        dim: DimId,
        op: ComparisonOp,
        operands: Vec<Operand>,
    },
}

impl MongoNode {
    pub fn eval(&self, view: &PointView, idx: PointId) -> bool {
        match self {
            MongoNode::Logical { op, children } => match op {
                LogicalOp::And => children.iter().all(|c| c.eval(view, idx)),
                LogicalOp::Or => children.iter().any(|c| c.eval(view, idx)),
                LogicalOp::Not => !children[0].eval(view, idx),
                LogicalOp::Nor => !children.iter().any(|c| c.eval(view, idx)),
            },
            MongoNode::Comparison { dim, op, operands } => {
                let val = view.get_f64(idx, dim);
                match op {
                    ComparisonOp::Eq => val == operands[0].get(view, idx),
                    ComparisonOp::Gt => val > operands[0].get(view, idx),
                    ComparisonOp::Gte => val >= operands[0].get(view, idx),
                    ComparisonOp::Lt => val < operands[0].get(view, idx),
                    ComparisonOp::Lte => val <= operands[0].get(view, idx),
                    ComparisonOp::Ne => val != operands[0].get(view, idx),
                    ComparisonOp::In => operands.iter().any(|o| val == o.get(view, idx)),
                    ComparisonOp::Nin => !operands.iter().any(|o| val == o.get(view, idx)),
                }
            }
        }
    }
}

pub struct MongoExpression {
    root: MongoNode,
}

impl MongoExpression {
    pub fn parse(json_str: &str) -> Result<Self, String> {
        let v: Value = serde_json::from_str(json_str).map_err(|e| e.to_string())?;
        let mut children = Vec::new();
        Self::build(&mut children, &v)?;
        
        let root = if children.len() == 1 {
            children.remove(0)
        } else {
            MongoNode::Logical {
                op: LogicalOp::And,
                children,
            }
        };
        
        Ok(Self { root })
    }

    fn build(nodes: &mut Vec<MongoNode>, v: &Value) -> Result<(), String> {
        if v.is_array() {
            for item in v.as_array().unwrap() {
                Self::build(nodes, item)?;
            }
            return Ok(());
        }

        if !v.is_object() {
            return Err(format!("Unexpected expression: {}", v));
        }

        let obj = v.as_object().unwrap();
        
        for (key, val) in obj {
            if let Some(op) = LogicalOp::parse(key) {
                if op != LogicalOp::Not && !val.is_array() {
                    return Err("Logical operator expressions must be arrays".to_string());
                }
                
                let mut inner_nodes = Vec::new();
                if op == LogicalOp::Not {
                   Self::build(&mut inner_nodes, val)?;
                } else {
                    for item in val.as_array().unwrap() {
                        Self::build(&mut inner_nodes, item)?;
                    }
                }
                
                if op == LogicalOp::Not && inner_nodes.len() != 1 {
                     return Err("Logical NOT must have exactly one child".to_string());
                }

                nodes.push(MongoNode::Logical {
                    op,
                    children: inner_nodes,
                });
            } else {
                // key is a dimension name
                let dim = DimId::from_name(key);
                
                if val.is_object() {
                    let comp_obj = val.as_object().unwrap();
                    for (comp_key, comp_val) in comp_obj {
                        if let Some(op) = ComparisonOp::parse(comp_key) {
                            let mut operands = Vec::new();
                            if op == ComparisonOp::In || op == ComparisonOp::Nin {
                                if !comp_val.is_array() {
                                    return Err(format!("{} must be an array", comp_key));
                                }
                                for item in comp_val.as_array().unwrap() {
                                    operands.push(Operand::from_json(item)?);
                                }
                            } else {
                                operands.push(Operand::from_json(comp_val)?);
                            }
                            nodes.push(MongoNode::Comparison { dim: dim.clone(), op, operands });
                        } else {
                             return Err(format!("Invalid comparison operator: {}", comp_key));
                        }
                    }
                } else {
                    // Implicit equality
                    nodes.push(MongoNode::Comparison {
                        dim,
                        op: ComparisonOp::Eq,
                        operands: vec![Operand::from_json(val)?],
                    });
                }
            }
        }

        Ok(())
    }

    pub fn eval(&self, view: &PointView, idx: PointId) -> bool {
        self.root.eval(view, idx)
    }
}

impl fmt::Display for MongoExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.root) // Simple for now
    }
}
