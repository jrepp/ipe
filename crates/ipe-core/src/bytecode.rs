use serde::{Deserialize, Serialize};

/// Bytecode instruction set
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, bincode_next::Encode, bincode_next::Decode,
)]
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
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    bincode_next::Encode,
    bincode_next::Decode,
)]
pub enum CompOp {
    Eq,  // ==
    Neq, // !=
    Lt,  // <
    Lte, // <=
    Gt,  // >
    Gte, // >=
}

/// Runtime values
/// Optimized for cache coherency with small string optimization
#[derive(
    Debug, Clone, PartialEq, Serialize, Deserialize, bincode_next::Encode, bincode_next::Decode,
)]
pub enum Value {
    Int(i64),
    Bool(bool),
    String(String),
}

impl Value {
    /// Create a Value from an integer (inline hint for optimizer)
    #[inline]
    pub const fn from_int(i: i64) -> Self {
        Value::Int(i)
    }

    /// Create a Value from a boolean (inline hint for optimizer)
    #[inline]
    pub const fn from_bool(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl Value {
    /// Check if the value is truthy (for boolean operations)
    /// Marked inline for hot path optimization
    #[inline]
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::String(s) => !s.is_empty(),
        }
    }

    /// Compare two values using the given comparison operator
    /// Marked inline for hot path optimization
    #[inline]
    pub fn compare(&self, other: &Value, op: CompOp) -> Result<bool, String> {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => Ok(Self::compare_int(*a, *b, op)),
            (Value::String(a), Value::String(b)) => {
                Ok(Self::compare_ordered(a.as_str(), b.as_str(), op))
            },
            (Value::Bool(a), Value::Bool(b)) => Ok(Self::compare_bools(*a, *b, op)),
            _ => Err(format!("Cannot compare {:?} with {:?}", self, other)),
        }
    }

    /// Fast integer comparison without generic overhead
    /// Most common hot path for comparisons
    #[inline]
    fn compare_int(a: i64, b: i64, op: CompOp) -> bool {
        match op {
            CompOp::Eq => a == b,
            CompOp::Neq => a != b,
            CompOp::Lt => a < b,
            CompOp::Lte => a <= b,
            CompOp::Gt => a > b,
            CompOp::Gte => a >= b,
        }
    }

    /// Generic comparison for types that implement PartialOrd and PartialEq
    #[inline]
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
    #[inline]
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
#[derive(Debug, Clone, Serialize, Deserialize, bincode_next::Encode, bincode_next::Decode)]
pub struct PolicyHeader {
    pub magic: [u8; 4], // "IPE\0"
    pub version: u32,
    pub policy_id: u64,
    pub code_size: u32,
    pub const_size: u32,
}

/// Errors from bytecode storage encoding and decoding.
#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    #[error("{0}")]
    Encode(#[from] bincode_next::error::EncodeError),
    #[error("{0}")]
    Decode(#[from] bincode_next::error::DecodeError),
}

/// Compiled policy bytecode
#[derive(Debug, Clone, Serialize, Deserialize, bincode_next::Encode, bincode_next::Decode)]
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
    pub fn to_bytes(&self) -> Result<Vec<u8>, SerializationError> {
        // Version 1 used bincode 1.3's fixed-width, little-endian encoding.
        Ok(bincode_next::encode_to_vec(self, bincode_next::config::legacy())?)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SerializationError> {
        // Keep the original decoder's behavior: trailing bytes are permitted.
        let (policy, _) = bincode_next::decode_from_slice(bytes, bincode_next::config::legacy())?;
        Ok(policy)
    }

    /// Get the size in bytes
    pub fn size_bytes(&self) -> usize {
        std::mem::size_of::<PolicyHeader>()
            + self.code.len() * std::mem::size_of::<Instruction>()
            + self
                .constants
                .iter()
                .map(|v| match v {
                    Value::Int(_) => 8,
                    Value::Bool(_) => 1,
                    Value::String(s) => s.len(),
                })
                .sum::<usize>()
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

        assert!(a.compare(&b, CompOp::Eq).unwrap());
        assert!(!a.compare(&c, CompOp::Eq).unwrap());
    }

    #[test]
    fn test_value_compare_int_neq() {
        let a = Value::Int(42);
        let b = Value::Int(10);
        let c = Value::Int(42);

        assert!(a.compare(&b, CompOp::Neq).unwrap());
        assert!(!a.compare(&c, CompOp::Neq).unwrap());
    }

    #[test]
    fn test_value_compare_int_lt() {
        let a = Value::Int(10);
        let b = Value::Int(42);
        let c = Value::Int(5);

        assert!(a.compare(&b, CompOp::Lt).unwrap());
        assert!(!a.compare(&c, CompOp::Lt).unwrap());
        assert!(!c.compare(&c, CompOp::Lt).unwrap());
    }

    #[test]
    fn test_value_compare_int_lte() {
        let a = Value::Int(10);
        let b = Value::Int(42);
        let c = Value::Int(10);

        assert!(a.compare(&b, CompOp::Lte).unwrap());
        assert!(a.compare(&c, CompOp::Lte).unwrap());
        assert!(!b.compare(&a, CompOp::Lte).unwrap());
    }

    #[test]
    fn test_value_compare_int_gt() {
        let a = Value::Int(42);
        let b = Value::Int(10);
        let c = Value::Int(100);

        assert!(a.compare(&b, CompOp::Gt).unwrap());
        assert!(!a.compare(&c, CompOp::Gt).unwrap());
        assert!(!a.compare(&b.clone(), CompOp::Lt).unwrap());
    }

    #[test]
    fn test_value_compare_int_gte() {
        let a = Value::Int(42);
        let b = Value::Int(10);
        let c = Value::Int(42);

        assert!(a.compare(&b, CompOp::Gte).unwrap());
        assert!(a.compare(&c, CompOp::Gte).unwrap());
        assert!(!b.compare(&a, CompOp::Gte).unwrap());
    }

    #[test]
    fn test_value_compare_string_eq() {
        let a = Value::String("hello".to_string());
        let b = Value::String("hello".to_string());
        let c = Value::String("world".to_string());

        assert!(a.compare(&b, CompOp::Eq).unwrap());
        assert!(!a.compare(&c, CompOp::Eq).unwrap());
    }

    #[test]
    fn test_value_compare_string_lt() {
        let a = Value::String("apple".to_string());
        let b = Value::String("banana".to_string());

        assert!(a.compare(&b, CompOp::Lt).unwrap());
        assert!(!b.compare(&a, CompOp::Lt).unwrap());
    }

    #[test]
    fn test_value_compare_bool_eq() {
        let a = Value::Bool(true);
        let b = Value::Bool(true);
        let c = Value::Bool(false);

        assert!(a.compare(&b, CompOp::Eq).unwrap());
        assert!(!a.compare(&c, CompOp::Eq).unwrap());
    }

    #[test]
    fn test_value_compare_bool_neq() {
        let a = Value::Bool(true);
        let b = Value::Bool(false);

        assert!(a.compare(&b, CompOp::Neq).unwrap());
        assert!(!b.compare(&b, CompOp::Neq).unwrap());
    }

    #[test]
    fn test_value_compare_bool_ordering_not_supported() {
        let a = Value::Bool(true);
        let b = Value::Bool(false);

        // Boolean ordering operations should return false
        assert!(!a.compare(&b, CompOp::Lt).unwrap());
        assert!(!a.compare(&b, CompOp::Lte).unwrap());
        assert!(!a.compare(&b, CompOp::Gt).unwrap());
        assert!(!a.compare(&b, CompOp::Gte).unwrap());
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

    #[test]
    fn test_bincode_v1_storage_compatibility() {
        let mut policy = CompiledPolicy::new(0x0102030405060708);
        for value in [Value::Int(-42), Value::Bool(true), Value::String("policy λ".into())] {
            policy.add_constant(value);
        }
        for instr in [
            Instruction::LoadField { offset: 258 },
            Instruction::LoadConst { idx: 1 },
            Instruction::Compare { op: CompOp::Eq },
            Instruction::Compare { op: CompOp::Neq },
            Instruction::Compare { op: CompOp::Lt },
            Instruction::Compare { op: CompOp::Lte },
            Instruction::Compare { op: CompOp::Gt },
            Instruction::Compare { op: CompOp::Gte },
            Instruction::Jump { offset: -2 },
            Instruction::JumpIfFalse { offset: 3 },
            Instruction::Call { func: 4, argc: 2 },
            Instruction::Return { value: true },
            Instruction::And,
            Instruction::Or,
            Instruction::Not,
        ] {
            policy.emit(instr);
        }

        // Generated by bincode 1.3.3 before the codec migration. The fixture
        // covers every instruction, comparison operator, and constant type.
        let legacy = include_bytes!("../tests/fixtures/policy-v1.bin");
        assert_eq!(policy.to_bytes().unwrap(), legacy);
        let decoded = CompiledPolicy::from_bytes(legacy).unwrap();
        assert_eq!(decoded.header.magic, policy.header.magic);
        assert_eq!(decoded.header.version, policy.header.version);
        assert_eq!(decoded.header.policy_id, policy.header.policy_id);
        assert_eq!(decoded.header.code_size, policy.header.code_size);
        assert_eq!(decoded.header.const_size, policy.header.const_size);
        assert_eq!(decoded.code, policy.code);
        assert_eq!(decoded.constants, policy.constants);

        let mut with_trailing = legacy.to_vec();
        with_trailing.extend_from_slice(b"trailing");
        assert_eq!(CompiledPolicy::from_bytes(&with_trailing).unwrap().code, policy.code);
    }

    #[test]
    fn test_invalid_serialized_policy_returns_error() {
        let legacy = include_bytes!("../tests/fixtures/policy-v1.bin");
        for end in 0..legacy.len() {
            assert!(CompiledPolicy::from_bytes(&legacy[..end]).is_err(), "length {end}");
        }
        let mut invalid = legacy.to_vec();
        // The 24-byte header and 8-byte vector length precede the first enum tag.
        invalid[32..36].copy_from_slice(&u32::MAX.to_le_bytes());
        let error = CompiledPolicy::from_bytes(&invalid).unwrap_err();
        let error: crate::Error = error.into();
        assert!(matches!(error, crate::Error::SerializationError(_)));
    }
}
