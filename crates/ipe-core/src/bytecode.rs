use serde::{Deserialize, Serialize};

/// Bytecode instruction set
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Instruction {
    /// Load a field from the evaluation context
    LoadField { offset: u16 },
    
    /// Load a constant from the constant pool
    LoadConst { idx: u16 },
    
    /// Compare two values on the stack
    Compare { op: CompOp },
    
    /// Unconditional jump
    Jump { offset: i16 },
    
    /// Jump if top of stack is false
    JumpIfFalse { offset: i16 },
    
    /// Call a built-in function
    Call { func: u8, argc: u8 },
    
    /// Return from policy evaluation
    Return { value: bool },
    
    /// Logical AND of two boolean values
    And,
    
    /// Logical OR of two boolean values
    Or,
    
    /// Logical NOT of a boolean value
    Not,
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompOp {
    Eq,   // ==
    Neq,  // !=
    Lt,   // <
    Lte,  // <=
    Gt,   // >
    Gte,  // >=
}

/// Runtime values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
}

/// Compiled policy header
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyHeader {
    pub magic: [u8; 4],      // "IPE\0"
    pub version: u32,
    pub policy_id: u64,
    pub code_size: u32,
    pub const_size: u32,
}

/// Compiled policy bytecode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPolicy {
    pub header: PolicyHeader,
    pub code: Vec<Instruction>,
    pub constants: Vec<Value>,
}

impl CompiledPolicy {
    /// Create a new compiled policy
    pub fn new(policy_id: u64) -> Self {
        Self {
            header: PolicyHeader {
                magic: *b"IPE\0",
                version: 1,
                policy_id,
                code_size: 0,
                const_size: 0,
            },
            code: Vec::new(),
            constants: Vec::new(),
        }
    }
    
    /// Add an instruction to the bytecode
    pub fn emit(&mut self, instr: Instruction) {
        self.code.push(instr);
        self.header.code_size += 1;
    }
    
    /// Add a constant to the constant pool
    pub fn add_constant(&mut self, value: Value) -> u16 {
        let idx = self.constants.len() as u16;
        self.constants.push(value);
        self.header.const_size += 1;
        idx
    }
    
    /// Serialize to bytes (for storage)
    pub fn to_bytes(&self) -> bincode::Result<Vec<u8>> {
        bincode::serialize(self)
    }
    
    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> bincode::Result<Self> {
        bincode::deserialize(bytes)
    }
    
    /// Get the size in bytes
    pub fn size_bytes(&self) -> usize {
        std::mem::size_of::<PolicyHeader>()
            + self.code.len() * std::mem::size_of::<Instruction>()
            + self.constants.iter().map(|v| match v {
                Value::Int(_) => 8,
                Value::Bool(_) => 1,
                Value::String(s) => s.len(),
            }).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_policy_creation() {
        let mut policy = CompiledPolicy::new(1);
        
        // Simple policy: load field, load const, compare, return
        policy.emit(Instruction::LoadField { offset: 0 });
        let const_idx = policy.add_constant(Value::Int(42));
        policy.emit(Instruction::LoadConst { idx: const_idx });
        policy.emit(Instruction::Compare { op: CompOp::Eq });
        policy.emit(Instruction::Return { value: true });
        
        assert_eq!(policy.code.len(), 4);
        assert_eq!(policy.constants.len(), 1);
    }
    
    #[test]
    fn test_serialization() {
        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::Return { value: true });
        
        let bytes = policy.to_bytes().unwrap();
        let deserialized = CompiledPolicy::from_bytes(&bytes).unwrap();
        
        assert_eq!(policy.header.policy_id, deserialized.header.policy_id);
        assert_eq!(policy.code, deserialized.code);
    }
}
