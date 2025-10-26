use crate::bytecode::CompiledPolicy;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Parse error: {0}")]
    ParseError(String),
}

pub struct PolicyCompiler {}

impl PolicyCompiler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for PolicyCompiler {
    fn default() -> Self {
        Self::new()
    }
}
