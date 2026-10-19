pub mod rar;

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

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

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
