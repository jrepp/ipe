pub mod ast;
pub mod compiler;
pub mod engine;
pub mod index;
pub mod bytecode;
pub mod rar;
pub mod interpreter;
pub mod tiering;
pub mod parser;
pub mod store;

#[cfg(feature = "jit")]
pub mod jit;

// Test utilities (available in tests and when used as a dependency with dev profile)
#[cfg(any(test, feature = "testing"))]
pub mod testing;

pub use ast::{Policy, Condition, Requirements};
pub use compiler::{PolicyCompiler, CompileError};
pub use engine::{PolicyEngine, Decision, DecisionKind};
pub use rar::{EvaluationContext, Resource, Action, Request, Principal};

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
}

pub type Result<T> = std::result::Result<T, Error>;
