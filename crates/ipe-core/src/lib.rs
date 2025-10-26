pub mod ast;
pub mod compiler;
pub mod engine;
pub mod index;
pub mod bytecode;
pub mod rar;
pub mod interpreter;
pub mod tiering;

#[cfg(feature = "jit")]
pub mod jit;

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
    
    #[cfg(feature = "jit")]
    #[error("JIT compilation error: {0}")]
    JitError(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
