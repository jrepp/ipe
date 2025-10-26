//! Abstract Syntax Tree (AST) for IPE policies
//!
//! The AST represents the parsed structure of IPE policies before compilation.

pub mod nodes;
pub mod types;
pub mod visitor;

pub use nodes::{
    Policy, Condition, Expression, Requirements, Metadata, Path, Value, BinaryOp, LogicalOp,
    ComparisonOp, AggregateFunc,
};
pub use types::{Type, TypeChecker};
pub use visitor::{Visitor, walk_policy};
