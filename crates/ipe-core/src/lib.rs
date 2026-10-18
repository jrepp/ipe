#[cfg(feature = "legacy")]
pub mod ast;
#[cfg(feature = "legacy")]
pub mod bytecode;
#[cfg(feature = "legacy")]
pub mod compiler;
#[cfg(feature = "legacy")]
pub mod engine;
#[cfg(feature = "legacy")]
pub mod index;
#[cfg(feature = "legacy")]
pub mod interpreter;
#[cfg(feature = "legacy")]
pub mod parser;
pub mod rar;

#[cfg(feature = "jit")]
pub mod jit;

#[cfg(feature = "approvals")]
pub mod approval;

#[cfg(feature = "approvals")]
pub mod relationship;

#[cfg(feature = "approvals")]
pub mod runtime_facts;

#[cfg(feature = "approvals")]
pub mod runtime_policy;

#[cfg(feature = "approvals")]
pub mod policy_document;

#[cfg(feature = "approvals")]
pub mod policy_scenarios;

// Test utilities (available in tests and when used as a dependency with dev profile)
#[cfg(any(all(test, feature = "legacy"), feature = "testing"))]
pub mod testing;

#[cfg(feature = "legacy")]
pub use ast::{Condition, Policy, Requirements};
#[cfg(feature = "legacy")]
pub use compiler::{CompileError, PolicyCompiler};
#[cfg(feature = "legacy")]
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
    #[cfg(feature = "legacy")]
    SerializationError(#[from] bytecode::SerializationError),

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
