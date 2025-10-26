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

impl Value {
    /// Check if the value is truthy (for boolean operations)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::String(s) => !s.is_empty(),
        }
    }

    /// Compare two values using the given comparison operator
    pub fn compare(&self, other: &Value, op: CompOp) -> Result<bool, String> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Self::compare_ordered(*a, *b, op)),
            (Value::String(a), Value::String(b)) => Ok(Self::compare_ordered(a.as_str(), b.as_str(), op)),
            (Value::Bool(a), Value::Bool(b)) => Ok(Self::compare_bools(*a, *b, op)),
            _ => Err(format!("Cannot compare {:?} with {:?}", self, other)),
        }
    }

    /// Generic comparison for types that implement PartialOrd and PartialEq
    fn compare_ordered<T: PartialOrd + PartialEq>(a: T, b: T, op: CompOp) -> bool {
        match op {
            CompOp::Eq => a == b,
            CompOp::Neq => a != b,
            CompOp::Lt => a < b,
            CompOp::Lte => a <= b,
            CompOp::Gt => a > b,
            CompOp::Gte => a >= b,
        }
    }

    /// Boolean comparison (ordering operations not supported)
    fn compare_bools(a: bool, b: bool, op: CompOp) -> bool {
        match op {
            CompOp::Eq => a == b,
            CompOp::Neq => a != b,
            _ => false, // < > <= >= not supported for booleans
        }
    }
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

    // Value tests
    #[test]
    fn test_value_is_truthy_bool() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
    }

    #[test]
    fn test_value_is_truthy_int() {
        assert!(Value::Int(1).is_truthy());
        assert!(Value::Int(-1).is_truthy());
        assert!(Value::Int(100).is_truthy());
        assert!(!Value::Int(0).is_truthy());
    }

    #[test]
    fn test_value_is_truthy_string() {
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(Value::String("x".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
    }

    #[test]
    fn test_value_compare_int_eq() {
        let a = Value::Int(42);
        let b = Value::Int(42);
        let c = Value::Int(10);

        assert_eq!(a.compare(&b, CompOp::Eq).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Eq).unwrap(), false);
    }

    #[test]
    fn test_value_compare_int_neq() {
        let a = Value::Int(42);
        let b = Value::Int(10);
        let c = Value::Int(42);

        assert_eq!(a.compare(&b, CompOp::Neq).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Neq).unwrap(), false);
    }

    #[test]
    fn test_value_compare_int_lt() {
        let a = Value::Int(10);
        let b = Value::Int(42);
        let c = Value::Int(5);

        assert_eq!(a.compare(&b, CompOp::Lt).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Lt).unwrap(), false);
        assert_eq!(a.compare(&a, CompOp::Lt).unwrap(), false);
    }

    #[test]
    fn test_value_compare_int_lte() {
        let a = Value::Int(10);
        let b = Value::Int(42);
        let c = Value::Int(10);

        assert_eq!(a.compare(&b, CompOp::Lte).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Lte).unwrap(), true);
        assert_eq!(b.compare(&a, CompOp::Lte).unwrap(), false);
    }

    #[test]
    fn test_value_compare_int_gt() {
        let a = Value::Int(42);
        let b = Value::Int(10);
        let c = Value::Int(100);

        assert_eq!(a.compare(&b, CompOp::Gt).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Gt).unwrap(), false);
        assert_eq!(a.compare(&a, CompOp::Gt).unwrap(), false);
    }

    #[test]
    fn test_value_compare_int_gte() {
        let a = Value::Int(42);
        let b = Value::Int(10);
        let c = Value::Int(42);

        assert_eq!(a.compare(&b, CompOp::Gte).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Gte).unwrap(), true);
        assert_eq!(b.compare(&a, CompOp::Gte).unwrap(), false);
    }

    #[test]
    fn test_value_compare_string_eq() {
        let a = Value::String("hello".to_string());
        let b = Value::String("hello".to_string());
        let c = Value::String("world".to_string());

        assert_eq!(a.compare(&b, CompOp::Eq).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Eq).unwrap(), false);
    }

    #[test]
    fn test_value_compare_string_lt() {
        let a = Value::String("apple".to_string());
        let b = Value::String("banana".to_string());

        assert_eq!(a.compare(&b, CompOp::Lt).unwrap(), true);
        assert_eq!(b.compare(&a, CompOp::Lt).unwrap(), false);
    }

    #[test]
    fn test_value_compare_bool_eq() {
        let a = Value::Bool(true);
        let b = Value::Bool(true);
        let c = Value::Bool(false);

        assert_eq!(a.compare(&b, CompOp::Eq).unwrap(), true);
        assert_eq!(a.compare(&c, CompOp::Eq).unwrap(), false);
    }

    #[test]
    fn test_value_compare_bool_neq() {
        let a = Value::Bool(true);
        let b = Value::Bool(false);

        assert_eq!(a.compare(&b, CompOp::Neq).unwrap(), true);
        assert_eq!(a.compare(&a, CompOp::Neq).unwrap(), false);
    }

    #[test]
    fn test_value_compare_bool_ordering_not_supported() {
        let a = Value::Bool(true);
        let b = Value::Bool(false);

        // Boolean ordering operations should return false
        assert_eq!(a.compare(&b, CompOp::Lt).unwrap(), false);
        assert_eq!(a.compare(&b, CompOp::Lte).unwrap(), false);
        assert_eq!(a.compare(&b, CompOp::Gt).unwrap(), false);
        assert_eq!(a.compare(&b, CompOp::Gte).unwrap(), false);
    }

    #[test]
    fn test_value_compare_type_mismatch() {
        let a = Value::Int(42);
        let b = Value::String("42".to_string());

        assert!(a.compare(&b, CompOp::Eq).is_err());

        let c = Value::Bool(true);
        assert!(a.compare(&c, CompOp::Eq).is_err());
    }

    // CompiledPolicy tests
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
