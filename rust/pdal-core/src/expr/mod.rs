//! The PDAL filter expression language.
//!
//! Port of `filters/private/expr/`. This mini-language backs the `expression`,
//! `assign` and related filters: comparison/logical predicates and arithmetic
//! over point dimensions.
//!
//! Pipeline: [`lexer::Lexer`] turns source text into [`token::Token`]s; the
//! parser (follow-up) builds an AST; the evaluator runs it against a point.

pub mod ast;
pub mod expression;
pub mod lexer;
pub mod parser;
pub mod token;

pub use expression::{
    AssignStatement, ConditionalExpression, IdentExpression, MathExpression,
};
pub use parser::{parse_conditional, parse_math};
