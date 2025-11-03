pub mod ast;
pub mod bytecode;
pub mod compiler;
pub mod engine;
pub mod index;
pub mod interpreter;
pub mod parser;
pub mod rar;
pub mod store;
pub mod tiering;

#[cfg(feature = "jit")]
pub mod jit;

#[cfg(feature = "approvals")]
pub mod approval;

#[cfg(feature = "approvals")]
pub mod relationship;

// Test utilities (available in tests and when used as a dependency with dev profile)
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use ast::{Condition, Policy, Requirements};
pub use compiler::{CompileError, PolicyCompiler};
pub use engine::{Decision, DecisionKind, PolicyEngine};
pub use rar::{Action, EvaluationContext, Principal, Request, Resource};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Policy not found: {0}")]
    PolicyNotFound(String),

    #[error("Evaluation error: {0}")]
    EvaluationError(String),

    #[error("Compilation error: {0}")]
    CompilationError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[cfg(feature = "jit")]
    #[error("JIT compilation error: {0}")]
    JitError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[cfg(feature = "approvals")]
    #[error("Approval error: {0}")]
    ApprovalError(#[from] crate::approval::ApprovalError),

    #[cfg(feature = "approvals")]
    #[error("Relationship error: {0}")]
    RelationshipError(#[from] crate::relationship::RelationshipError),

    #[error("No approval store configured")]
    NoApprovalStore,

    #[error("No relationship store configured")]
    NoRelationshipStore,
}

pub type Result<T> = std::result::Result<T, Error>;
