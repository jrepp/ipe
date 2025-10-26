use crate::bytecode::{Value, Instruction, CompiledPolicy, CompOp};
use crate::rar::EvaluationContext;

/// Maximum stack size to prevent stack overflow
const MAX_STACK_SIZE: usize = 1024;

/// Evaluation stack for the interpreter
pub struct Stack {
    values: Vec<Value>,
    max_size: usize,
}

impl Stack {
    /// Create a new stack with default max size
    pub fn new() -> Self {
        Self::with_capacity(MAX_STACK_SIZE)
    }

    /// Create a new stack with specified max size
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            values: Vec::with_capacity(max_size.min(128)), // Start small, can grow
            max_size,
        }
    }

    /// Push a value onto the stack
    pub fn push(&mut self, value: Value) -> Result<(), String> {
        if self.values.len() >= self.max_size {
            return Err(format!("Stack overflow: exceeded max size of {}", self.max_size));
        }
        self.values.push(value);
        Ok(())
    }

    /// Pop a value from the stack
    pub fn pop(&mut self) -> Result<Value, String> {
        self.values.pop().ok_or_else(|| "Stack underflow".to_string())
    }

    /// Peek at the top value without removing it
    pub fn peek(&self) -> Result<&Value, String> {
        self.values.last().ok_or_else(|| "Stack is empty".to_string())
    }

    /// Get the current stack size
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if the stack is empty
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Clear the stack
    pub fn clear(&mut self) {
        self.values.clear();
    }
}

impl Default for Stack {
    fn default() -> Self {
        Self::new()
    }
}

use crate::rar::AttributeValue;
use std::collections::HashMap;

/// Field mapping from offset to path
pub type FieldMapping = HashMap<u16, Vec<String>>;

/// Bytecode interpreter (fallback when JIT not available)
pub struct Interpreter {
    stack: Stack,
    field_map: FieldMapping,
}

impl Interpreter {
    /// Create a new interpreter with the given field mapping
    pub fn new(field_map: FieldMapping) -> Self {
        Self {
            stack: Stack::new(),
            field_map,
        }
    }

    /// Evaluate a compiled policy against an evaluation context
    pub fn evaluate(
        &mut self,
        policy: &CompiledPolicy,
        ctx: &EvaluationContext,
    ) -> Result<bool, String> {
        self.stack.clear();
        let mut pc = 0; // Program counter

        while pc < policy.code.len() {
            let instr = &policy.code[pc];

            match instr {
                Instruction::LoadField { offset } => {
                    let value = self.load_field(*offset, ctx)?;
                    self.stack.push(value)?;
                }

                Instruction::LoadConst { idx } => {
                    let value = policy
                        .constants
                        .get(*idx as usize)
                        .ok_or_else(|| format!("Invalid constant index: {}", idx))?
                        .clone();
                    self.stack.push(value)?;
                }

                Instruction::Compare { op } => {
                    let b = self.stack.pop()?;
                    let a = self.stack.pop()?;
                    let result = a.compare(&b, *op)?;
                    self.stack.push(Value::Bool(result))?;
                }

                Instruction::And => {
                    let b = self.stack.pop()?;
                    let a = self.stack.pop()?;
                    let result = a.is_truthy() && b.is_truthy();
                    self.stack.push(Value::Bool(result))?;
                }

                Instruction::Or => {
                    let b = self.stack.pop()?;
                    let a = self.stack.pop()?;
                    let result = a.is_truthy() || b.is_truthy();
                    self.stack.push(Value::Bool(result))?;
                }

                Instruction::Not => {
                    let a = self.stack.pop()?;
                    let result = !a.is_truthy();
                    self.stack.push(Value::Bool(result))?;
                }

                Instruction::Return { value } => {
                    return Ok(*value);
                }

                Instruction::Jump { offset } => {
                    pc = (pc as i32 + *offset as i32) as usize;
                    continue;
                }

                Instruction::JumpIfFalse { offset } => {
                    let cond = self.stack.pop()?;
                    if !cond.is_truthy() {
                        pc = (pc as i32 + *offset as i32) as usize;
                        continue;
                    }
                }

                Instruction::Call { func, argc } => {
                    return Err(format!("Function calls not yet supported: func={}, argc={}", func, argc));
                }
            }

            pc += 1;
        }

        // If we reach here without a Return instruction, default to deny
        Ok(false)
    }

    /// Load a field value from the evaluation context
    fn load_field(&self, offset: u16, ctx: &EvaluationContext) -> Result<Value, String> {
        let path = self
            .field_map
            .get(&offset)
            .ok_or_else(|| format!("Unknown field offset: {}", offset))?;

        // Navigate the path through the context
        if path.is_empty() {
            return Err("Empty field path".to_string());
        }

        // First component determines which part of RAR to access
        match path[0].as_str() {
            "resource" => self.access_resource(&path[1..], &ctx.resource),
            "action" => self.access_action(&path[1..], &ctx.action),
            "request" => self.access_request(&path[1..], &ctx.request),
            _ => Err(format!("Unknown RAR component: {}", path[0])),
        }
    }

    fn access_resource(&self, path: &[String], resource: &crate::rar::Resource) -> Result<Value, String> {
        if path.is_empty() {
            return Err("Resource path cannot be empty".to_string());
        }

        match path[0].as_str() {
            "type" => Ok(Value::Int(resource.type_id.0 as i64)),
            attr_name => {
                let attr = resource
                    .attributes
                    .get(attr_name)
                    .ok_or_else(|| format!("Attribute not found: {}", attr_name))?;
                self.attr_to_value(attr)
            }
        }
    }

    fn access_action(&self, path: &[String], _action: &crate::rar::Action) -> Result<Value, String> {
        if path.is_empty() {
            return Err("Action path cannot be empty".to_string());
        }

        // For now, just return error for unsupported paths
        Err(format!("Action field not supported: {}", path[0]))
    }

    fn access_request(&self, path: &[String], request: &crate::rar::Request) -> Result<Value, String> {
        if path.is_empty() {
            return Err("Request path cannot be empty".to_string());
        }

        match path[0].as_str() {
            "principal" => {
                if path.len() < 2 {
                    return Err("Principal path too short".to_string());
                }
                self.access_principal(&path[1..], &request.principal)
            }
            attr_name => {
                let attr = request
                    .metadata
                    .get(attr_name)
                    .ok_or_else(|| format!("Request metadata not found: {}", attr_name))?;
                self.attr_to_value(attr)
            }
        }
    }

    fn access_principal(&self, path: &[String], principal: &crate::rar::Principal) -> Result<Value, String> {
        if path.is_empty() {
            return Err("Principal path cannot be empty".to_string());
        }

        match path[0].as_str() {
            "id" => Ok(Value::String(principal.id.clone())),
            attr_name => {
                let attr = principal
                    .attributes
                    .get(attr_name)
                    .ok_or_else(|| format!("Principal attribute not found: {}", attr_name))?;
                self.attr_to_value(attr)
            }
        }
    }

    fn attr_to_value(&self, attr: &AttributeValue) -> Result<Value, String> {
        match attr {
            AttributeValue::String(s) => Ok(Value::String(s.clone())),
            AttributeValue::Int(i) => Ok(Value::Int(*i)),
            AttributeValue::Bool(b) => Ok(Value::Bool(*b)),
            AttributeValue::Array(_) => Err("Array attributes not yet supported".to_string()),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Stack tests
    #[test]
    fn test_stack_new() {
        let stack = Stack::new();
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_push_pop() {
        let mut stack = Stack::new();

        stack.push(Value::Int(42)).unwrap();
        assert_eq!(stack.len(), 1);
        assert!(!stack.is_empty());

        let val = stack.pop().unwrap();
        assert_eq!(val, Value::Int(42));
        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_push_multiple() {
        let mut stack = Stack::new();

        stack.push(Value::Int(1)).unwrap();
        stack.push(Value::Int(2)).unwrap();
        stack.push(Value::Int(3)).unwrap();

        assert_eq!(stack.len(), 3);

        assert_eq!(stack.pop().unwrap(), Value::Int(3));
        assert_eq!(stack.pop().unwrap(), Value::Int(2));
        assert_eq!(stack.pop().unwrap(), Value::Int(1));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_peek() {
        let mut stack = Stack::new();

        stack.push(Value::Int(42)).unwrap();
        assert_eq!(*stack.peek().unwrap(), Value::Int(42));
        assert_eq!(stack.len(), 1); // Peek doesn't remove

        stack.push(Value::Int(100)).unwrap();
        assert_eq!(*stack.peek().unwrap(), Value::Int(100));
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_stack_underflow() {
        let mut stack = Stack::new();

        // Pop from empty stack should fail
        assert!(stack.pop().is_err());

        // Peek from empty stack should fail
        assert!(stack.peek().is_err());
    }

    #[test]
    fn test_stack_overflow() {
        let mut stack = Stack::with_capacity(3);

        stack.push(Value::Int(1)).unwrap();
        stack.push(Value::Int(2)).unwrap();
        stack.push(Value::Int(3)).unwrap();

        // Fourth push should fail
        let result = stack.push(Value::Int(4));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Stack overflow"));
    }

    #[test]
    fn test_stack_clear() {
        let mut stack = Stack::new();

        stack.push(Value::Int(1)).unwrap();
        stack.push(Value::Int(2)).unwrap();
        stack.push(Value::Int(3)).unwrap();

        assert_eq!(stack.len(), 3);

        stack.clear();

        assert_eq!(stack.len(), 0);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_stack_mixed_types() {
        let mut stack = Stack::new();

        stack.push(Value::Int(42)).unwrap();
        stack.push(Value::Bool(true)).unwrap();
        stack.push(Value::String("hello".to_string())).unwrap();

        assert_eq!(stack.len(), 3);

        assert_eq!(stack.pop().unwrap(), Value::String("hello".to_string()));
        assert_eq!(stack.pop().unwrap(), Value::Bool(true));
        assert_eq!(stack.pop().unwrap(), Value::Int(42));
    }

    #[test]
    fn test_stack_with_capacity() {
        let stack = Stack::with_capacity(100);
        assert_eq!(stack.max_size, 100);
        assert!(stack.is_empty());
    }

    // Interpreter tests
    use crate::rar::{EvaluationContext, AttributeValue, ResourceTypeId};

    #[test]
    fn test_interpreter_load_const() {
        let mut policy = CompiledPolicy::new(1);
        let idx = policy.add_constant(Value::Int(42));
        policy.emit(Instruction::LoadConst { idx });
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        let result = interp.evaluate(&policy, &ctx).unwrap();
        assert!(result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[test]
    fn test_interpreter_compare_eq() {
        let mut policy = CompiledPolicy::new(1);
        let idx1 = policy.add_constant(Value::Int(42));
        let idx2 = policy.add_constant(Value::Int(42));

        policy.emit(Instruction::LoadConst { idx: idx1 });
        policy.emit(Instruction::LoadConst { idx: idx2 });
        policy.emit(Instruction::Compare { op: CompOp::Eq });
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        let result = interp.evaluate(&policy, &ctx).unwrap();
        assert!(result);
        // Stack should have the comparison result
        assert_eq!(*interp.stack.peek().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_interpreter_compare_lt() {
        let mut policy = CompiledPolicy::new(1);
        let idx1 = policy.add_constant(Value::Int(10));
        let idx2 = policy.add_constant(Value::Int(42));

        policy.emit(Instruction::LoadConst { idx: idx1 });
        policy.emit(Instruction::LoadConst { idx: idx2 });
        policy.emit(Instruction::Compare { op: CompOp::Lt });
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(*interp.stack.peek().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_interpreter_and() {
        let mut policy = CompiledPolicy::new(1);
        let idx1 = policy.add_constant(Value::Bool(true));
        let idx2 = policy.add_constant(Value::Bool(true));

        policy.emit(Instruction::LoadConst { idx: idx1 });
        policy.emit(Instruction::LoadConst { idx: idx2 });
        policy.emit(Instruction::And);
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(*interp.stack.peek().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_interpreter_and_false() {
        let mut policy = CompiledPolicy::new(1);
        let idx1 = policy.add_constant(Value::Bool(true));
        let idx2 = policy.add_constant(Value::Bool(false));

        policy.emit(Instruction::LoadConst { idx: idx1 });
        policy.emit(Instruction::LoadConst { idx: idx2 });
        policy.emit(Instruction::And);
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(*interp.stack.peek().unwrap(), Value::Bool(false));
    }

    #[test]
    fn test_interpreter_or() {
        let mut policy = CompiledPolicy::new(1);
        let idx1 = policy.add_constant(Value::Bool(false));
        let idx2 = policy.add_constant(Value::Bool(true));

        policy.emit(Instruction::LoadConst { idx: idx1 });
        policy.emit(Instruction::LoadConst { idx: idx2 });
        policy.emit(Instruction::Or);
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(*interp.stack.peek().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_interpreter_not() {
        let mut policy = CompiledPolicy::new(1);
        let idx = policy.add_constant(Value::Bool(false));

        policy.emit(Instruction::LoadConst { idx });
        policy.emit(Instruction::Not);
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(*interp.stack.peek().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_interpreter_load_field() {
        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::LoadField { offset: 0 });
        policy.emit(Instruction::Return { value: true });

        let mut field_map = FieldMapping::new();
        field_map.insert(0, vec!["resource".to_string(), "name".to_string()]);

        let mut interp = Interpreter::new(field_map);

        let mut ctx = EvaluationContext::default();
        ctx.resource.attributes.insert(
            "name".to_string(),
            AttributeValue::String("test-resource".to_string()),
        );

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(
            *interp.stack.peek().unwrap(),
            Value::String("test-resource".to_string())
        );
    }

    #[test]
    fn test_interpreter_load_field_principal_id() {
        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::LoadField { offset: 0 });
        policy.emit(Instruction::Return { value: true });

        let mut field_map = FieldMapping::new();
        field_map.insert(0, vec!["request".to_string(), "principal".to_string(), "id".to_string()]);

        let mut interp = Interpreter::new(field_map);

        let mut ctx = EvaluationContext::default();
        ctx.request.principal.id = "user-123".to_string();

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(
            *interp.stack.peek().unwrap(),
            Value::String("user-123".to_string())
        );
    }

    #[test]
    fn test_interpreter_complex_policy() {
        // Policy: resource.priority == 5 AND resource.enabled == true
        let mut policy = CompiledPolicy::new(1);

        // Load resource.priority
        policy.emit(Instruction::LoadField { offset: 0 });
        // Load constant 5
        let idx_five = policy.add_constant(Value::Int(5));
        policy.emit(Instruction::LoadConst { idx: idx_five });
        // Compare ==
        policy.emit(Instruction::Compare { op: CompOp::Eq });

        // Load resource.enabled
        policy.emit(Instruction::LoadField { offset: 1 });
        // Load constant true
        let idx_true = policy.add_constant(Value::Bool(true));
        policy.emit(Instruction::LoadConst { idx: idx_true });
        // Compare ==
        policy.emit(Instruction::Compare { op: CompOp::Eq });

        // AND the two comparisons
        policy.emit(Instruction::And);
        policy.emit(Instruction::Return { value: true });

        let mut field_map = FieldMapping::new();
        field_map.insert(0, vec!["resource".to_string(), "priority".to_string()]);
        field_map.insert(1, vec!["resource".to_string(), "enabled".to_string()]);

        let mut interp = Interpreter::new(field_map);

        let mut ctx = EvaluationContext::default();
        ctx.resource.attributes.insert("priority".to_string(), AttributeValue::Int(5));
        ctx.resource.attributes.insert("enabled".to_string(), AttributeValue::Bool(true));

        interp.evaluate(&policy, &ctx).unwrap();
        assert_eq!(*interp.stack.peek().unwrap(), Value::Bool(true));
    }

    #[test]
    fn test_interpreter_invalid_field_offset() {
        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::LoadField { offset: 999 });
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        let result = interp.evaluate(&policy, &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown field offset"));
    }

    #[test]
    fn test_interpreter_invalid_constant_index() {
        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::LoadConst { idx: 999 });
        policy.emit(Instruction::Return { value: true });

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        let result = interp.evaluate(&policy, &ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid constant index"));
    }

    #[test]
    fn test_interpreter_no_return_defaults_false() {
        let policy = CompiledPolicy::new(1);
        // No instructions, should default to false

        let mut interp = Interpreter::default();
        let ctx = EvaluationContext::default();

        let result = interp.evaluate(&policy, &ctx).unwrap();
        assert!(!result); // Should default to deny
    }
}
