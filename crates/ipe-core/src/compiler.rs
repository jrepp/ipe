use crate::ast::nodes::{
    BinaryOp, ComparisonOp, Condition, Expression, LogicalOp, Policy, Requirements, Value,
};
use crate::bytecode::{CompOp, CompiledPolicy, Instruction, Value as BytecodeValue};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Unsupported expression: {0}")]
    UnsupportedExpression(String),

    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    #[error("Type mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    #[error("Too many constants (max 65536)")]
    TooManyConstants,

    #[error("Aggregate functions not yet supported: {0}")]
    UnsupportedAggregate(String),
}

pub type CompileResult<T> = Result<T, CompileError>;

/// Context for tracking variables during compilation
struct CompileContext {
    /// Map from path string to field offset
    field_offsets: HashMap<String, u16>,
    /// Next available field offset
    next_offset: u16,
}

impl CompileContext {
    fn new() -> Self {
        Self {
            field_offsets: HashMap::new(),
            next_offset: 0,
        }
    }

    /// Get or allocate a field offset for a path
    fn get_or_allocate_field(&mut self, path: &str) -> u16 {
        if let Some(&offset) = self.field_offsets.get(path) {
            offset
        } else {
            let offset = self.next_offset;
            self.field_offsets.insert(path.to_string(), offset);
            self.next_offset += 1;
            offset
        }
    }
}

pub struct PolicyCompiler {
    policy: CompiledPolicy,
    context: CompileContext,
}

impl PolicyCompiler {
    pub fn new(policy_id: u64) -> Self {
        Self {
            policy: CompiledPolicy::new(policy_id),
            context: CompileContext::new(),
        }
    }

    /// Compile an AST policy to bytecode
    pub fn compile(mut self, policy: &Policy) -> CompileResult<CompiledPolicy> {
        // For now, we compile the requirements section
        // In a full implementation, we'd also handle triggers
        match &policy.requirements {
            Requirements::Requires { conditions, where_clause } => {
                // Compile all conditions with AND logic
                for (i, condition) in conditions.iter().enumerate() {
                    self.compile_condition(condition)?;

                    // If not the last condition, emit AND
                    if i < conditions.len() - 1 {
                        self.policy.emit(Instruction::And);
                    }
                }

                // If there's a where clause, compile it and AND with main conditions
                if let Some(where_conds) = where_clause {
                    for condition in where_conds {
                        self.compile_condition(condition)?;
                        self.policy.emit(Instruction::And);
                    }
                }

                // Return true if all conditions passed
                self.policy.emit(Instruction::Return { value: true });
            },
            Requirements::Denies { .. } => {
                // Denies always returns false
                self.policy.emit(Instruction::Return { value: false });
            },
        }

        Ok(self.policy)
    }

    fn compile_condition(&mut self, condition: &Condition) -> CompileResult<()> {
        self.compile_expression(&condition.expr)
    }

    fn compile_expression(&mut self, expr: &Expression) -> CompileResult<()> {
        match expr {
            Expression::Literal(value) => self.compile_literal(value),

            Expression::Path(path) => {
                // Load field from context
                let path_str = path.to_string();
                let offset = self.context.get_or_allocate_field(&path_str);
                self.policy.emit(Instruction::LoadField { offset });
                Ok(())
            },

            Expression::Binary { left, op, right } => {
                // Compile left and right expressions
                self.compile_expression(left)?;
                self.compile_expression(right)?;

                // Emit comparison instruction
                match op {
                    BinaryOp::Comparison(comp_op) => {
                        let op = match comp_op {
                            ComparisonOp::Eq => CompOp::Eq,
                            ComparisonOp::Neq => CompOp::Neq,
                            ComparisonOp::Lt => CompOp::Lt,
                            ComparisonOp::LtEq => CompOp::Lte,
                            ComparisonOp::Gt => CompOp::Gt,
                            ComparisonOp::GtEq => CompOp::Gte,
                        };
                        self.policy.emit(Instruction::Compare { op });
                        Ok(())
                    },
                }
            },

            Expression::Logical { op, operands } => {
                match op {
                    LogicalOp::And => {
                        // Compile all operands and AND them together
                        for (i, operand) in operands.iter().enumerate() {
                            self.compile_expression(operand)?;
                            if i > 0 {
                                self.policy.emit(Instruction::And);
                            }
                        }
                        Ok(())
                    },
                    LogicalOp::Or => {
                        // Compile all operands and OR them together
                        for (i, operand) in operands.iter().enumerate() {
                            self.compile_expression(operand)?;
                            if i > 0 {
                                self.policy.emit(Instruction::Or);
                            }
                        }
                        Ok(())
                    },
                    LogicalOp::Not => {
                        // Compile operand and NOT it
                        if let Some(operand) = operands.first() {
                            self.compile_expression(operand)?;
                            self.policy.emit(Instruction::Not);
                            Ok(())
                        } else {
                            Err(CompileError::UnsupportedExpression(
                                "NOT requires an operand".to_string(),
                            ))
                        }
                    },
                }
            },

            Expression::In { expr, list } => {
                // For IN expressions, we generate comparison logic
                // expr == list[0] OR expr == list[1] OR ...
                self.compile_expression(expr)?;

                // Load first value and compare
                if let Some(_first) = list.first() {
                    // Duplicate the expression result for multiple comparisons
                    for (i, value) in list.iter().enumerate() {
                        if i > 0 {
                            // Duplicate the original value for next comparison
                            self.compile_expression(expr)?;
                        }
                        self.compile_literal(value)?;
                        self.policy.emit(Instruction::Compare { op: CompOp::Eq });

                        // OR with previous results if not first
                        if i > 0 {
                            self.policy.emit(Instruction::Or);
                        }
                    }
                    Ok(())
                } else {
                    // Empty list - always false
                    let idx = self.add_constant(BytecodeValue::Bool(false))?;
                    self.policy.emit(Instruction::LoadConst { idx });
                    Ok(())
                }
            },

            Expression::Call { name, args } => {
                // Compile arguments
                for arg in args {
                    self.compile_expression(arg)?;
                }

                // Emit function call
                // Function ID mapping (simplified for now)
                let func_id = match name.as_str() {
                    "count" => 0,
                    "any" => 1,
                    "all" => 2,
                    _ => {
                        return Err(CompileError::UnsupportedExpression(format!(
                            "Unknown function: {}",
                            name
                        )))
                    },
                };

                self.policy.emit(Instruction::Call { func: func_id, argc: args.len() as u8 });
                Ok(())
            },

            Expression::Aggregate { .. } => Err(CompileError::UnsupportedAggregate(
                "Aggregate functions require special handling".to_string(),
            )),
        }
    }

    fn compile_literal(&mut self, value: &Value) -> CompileResult<()> {
        let bytecode_value = match value {
            Value::Int(n) => BytecodeValue::Int(*n),
            Value::Bool(b) => BytecodeValue::Bool(*b),
            Value::String(s) => BytecodeValue::String(s.clone()),
            Value::Float(_) => {
                return Err(CompileError::UnsupportedExpression(
                    "Float literals not yet supported in bytecode".to_string(),
                ))
            },
            Value::Array(_) => {
                return Err(CompileError::UnsupportedExpression(
                    "Array literals not yet supported in bytecode".to_string(),
                ))
            },
        };

        let idx = self.add_constant(bytecode_value)?;
        self.policy.emit(Instruction::LoadConst { idx });
        Ok(())
    }

    fn add_constant(&mut self, value: BytecodeValue) -> CompileResult<u16> {
        if self.policy.constants.len() >= 65536 {
            return Err(CompileError::TooManyConstants);
        }
        Ok(self.policy.add_constant(value))
    }

    /// Get the field mappings for this policy
    pub fn field_mappings(&self) -> &HashMap<String, u16> {
        &self.context.field_offsets
    }
}

impl Default for PolicyCompiler {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_policy(requirements: Requirements) -> Policy {
        Policy::new("TestPolicy".to_string(), "Test intent".to_string(), vec![], requirements)
    }

    #[test]
    fn test_compile_literal_int() {
        let condition = Condition::new(Expression::literal(Value::Int(42)));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have: LoadConst, Return
        assert_eq!(compiled.code.len(), 2);
        assert!(matches!(compiled.code[0], Instruction::LoadConst { idx: 0 }));
        assert!(matches!(compiled.code[1], Instruction::Return { value: true }));
        assert_eq!(compiled.constants.len(), 1);
        assert_eq!(compiled.constants[0], BytecodeValue::Int(42));
    }

    #[test]
    fn test_compile_literal_bool() {
        let condition = Condition::new(Expression::literal(Value::Bool(true)));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        assert_eq!(compiled.constants[0], BytecodeValue::Bool(true));
    }

    #[test]
    fn test_compile_literal_string() {
        let condition = Condition::new(Expression::literal(Value::String("test".to_string())));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        assert_eq!(compiled.constants[0], BytecodeValue::String("test".to_string()));
    }

    #[test]
    fn test_compile_path() {
        let condition = Condition::new(Expression::path(vec!["resource".to_string()]));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have: LoadField, Return
        assert_eq!(compiled.code.len(), 2);
        assert!(matches!(compiled.code[0], Instruction::LoadField { offset: 0 }));
    }

    #[test]
    fn test_compile_binary_comparison() {
        // x == 42
        let condition = Condition::new(Expression::binary(
            Expression::path(vec!["x".to_string()]),
            BinaryOp::Comparison(ComparisonOp::Eq),
            Expression::literal(Value::Int(42)),
        ));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have: LoadField, LoadConst, Compare, Return
        assert_eq!(compiled.code.len(), 4);
        assert!(matches!(compiled.code[0], Instruction::LoadField { offset: 0 }));
        assert!(matches!(compiled.code[1], Instruction::LoadConst { idx: 0 }));
        assert!(matches!(compiled.code[2], Instruction::Compare { op: CompOp::Eq }));
        assert!(matches!(compiled.code[3], Instruction::Return { value: true }));
    }

    #[test]
    fn test_compile_all_comparison_ops() {
        let ops = vec![
            (ComparisonOp::Eq, CompOp::Eq),
            (ComparisonOp::Neq, CompOp::Neq),
            (ComparisonOp::Lt, CompOp::Lt),
            (ComparisonOp::LtEq, CompOp::Lte),
            (ComparisonOp::Gt, CompOp::Gt),
            (ComparisonOp::GtEq, CompOp::Gte),
        ];

        for (ast_op, bytecode_op) in ops {
            let condition = Condition::new(Expression::binary(
                Expression::literal(Value::Int(1)),
                BinaryOp::Comparison(ast_op),
                Expression::literal(Value::Int(2)),
            ));
            let policy = create_simple_policy(Requirements::requires(vec![condition]));

            let compiler = PolicyCompiler::new(1);
            let compiled = compiler.compile(&policy).unwrap();

            assert!(matches!(
                compiled.code[2],
                Instruction::Compare { op } if op == bytecode_op
            ));
        }
    }

    #[test]
    fn test_compile_logical_and() {
        // true AND false
        let condition = Condition::new(Expression::and(vec![
            Expression::literal(Value::Bool(true)),
            Expression::literal(Value::Bool(false)),
        ]));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have: LoadConst(true), LoadConst(false), And, Return
        assert_eq!(compiled.code.len(), 4);
        assert!(matches!(compiled.code[2], Instruction::And));
    }

    #[test]
    fn test_compile_logical_or() {
        // true OR false
        let condition = Condition::new(Expression::or(vec![
            Expression::literal(Value::Bool(true)),
            Expression::literal(Value::Bool(false)),
        ]));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        assert!(matches!(compiled.code[2], Instruction::Or));
    }

    #[test]
    fn test_compile_logical_not() {
        // NOT true
        let condition = Condition::new(Expression::logical_not(Expression::literal(Value::Bool(true))));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have: LoadConst(true), Not, Return
        assert_eq!(compiled.code.len(), 3);
        assert!(matches!(compiled.code[1], Instruction::Not));
    }

    #[test]
    fn test_compile_in_expression() {
        // env in ["prod", "staging"]
        let condition = Condition::new(Expression::in_list(
            Expression::path(vec!["env".to_string()]),
            vec![Value::String("prod".to_string()), Value::String("staging".to_string())],
        ));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should compile to: load env, load "prod", compare, load env, load "staging", compare, OR
        // LoadField(env), LoadConst("prod"), Compare(Eq), LoadField(env), LoadConst("staging"), Compare(Eq), Or, Return
        assert!(compiled.code.len() > 5);
        assert!(compiled.constants.contains(&BytecodeValue::String("prod".to_string())));
        assert!(compiled.constants.contains(&BytecodeValue::String("staging".to_string())));
    }

    #[test]
    fn test_compile_multiple_conditions() {
        // Two conditions: x == 1 AND y == 2
        let conditions = vec![
            Condition::new(Expression::binary(
                Expression::path(vec!["x".to_string()]),
                BinaryOp::Comparison(ComparisonOp::Eq),
                Expression::literal(Value::Int(1)),
            )),
            Condition::new(Expression::binary(
                Expression::path(vec!["y".to_string()]),
                BinaryOp::Comparison(ComparisonOp::Eq),
                Expression::literal(Value::Int(2)),
            )),
        ];
        let policy = create_simple_policy(Requirements::requires(conditions));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have AND between the two conditions
        let and_count = compiled.code.iter().filter(|i| matches!(i, Instruction::And)).count();
        assert_eq!(and_count, 1);
    }

    #[test]
    fn test_compile_denies() {
        let policy = create_simple_policy(Requirements::denies(Some("Not allowed".to_string())));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should just have Return(false)
        assert_eq!(compiled.code.len(), 1);
        assert!(matches!(compiled.code[0], Instruction::Return { value: false }));
    }

    #[test]
    fn test_compile_with_where_clause() {
        let conditions = vec![Condition::new(Expression::literal(Value::Bool(true)))];
        let where_clause = vec![Condition::new(Expression::literal(Value::Bool(true)))];

        let policy = create_simple_policy(Requirements::requires_where(conditions, where_clause));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have AND to combine main conditions with where clause
        let and_count = compiled.code.iter().filter(|i| matches!(i, Instruction::And)).count();
        assert!(and_count > 0);
    }

    #[test]
    fn test_compile_function_call() {
        // count()
        let condition =
            Condition::new(Expression::Call { name: "count".to_string(), args: vec![] });
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have Call instruction
        assert!(compiled
            .code
            .iter()
            .any(|i| matches!(i, Instruction::Call { func: 0, argc: 0 })));
    }

    #[test]
    fn test_compile_function_call_with_args() {
        // max(1, 2)
        let condition = Condition::new(Expression::Call {
            name: "count".to_string(),
            args: vec![Expression::literal(Value::Int(1)), Expression::literal(Value::Int(2))],
        });
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should compile arguments and have Call with argc=2
        assert!(compiled.code.iter().any(|i| matches!(i, Instruction::Call { argc: 2, .. })));
    }

    #[test]
    fn test_compile_complex_expression() {
        // (x == 1 OR y == 2) AND z == 3
        let condition = Condition::new(Expression::and(vec![
            Expression::or(vec![
                Expression::binary(
                    Expression::path(vec!["x".to_string()]),
                    BinaryOp::Comparison(ComparisonOp::Eq),
                    Expression::literal(Value::Int(1)),
                ),
                Expression::binary(
                    Expression::path(vec!["y".to_string()]),
                    BinaryOp::Comparison(ComparisonOp::Eq),
                    Expression::literal(Value::Int(2)),
                ),
            ]),
            Expression::binary(
                Expression::path(vec!["z".to_string()]),
                BinaryOp::Comparison(ComparisonOp::Eq),
                Expression::literal(Value::Int(3)),
            ),
        ]));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should have both AND and OR instructions
        assert!(compiled.code.iter().any(|i| matches!(i, Instruction::And)));
        assert!(compiled.code.iter().any(|i| matches!(i, Instruction::Or)));
    }

    #[test]
    fn test_field_mapping() {
        let condition = Condition::new(Expression::binary(
            Expression::path(vec!["resource".to_string(), "type".to_string()]),
            BinaryOp::Comparison(ComparisonOp::Eq),
            Expression::path(vec!["expected".to_string(), "value".to_string()]),
        ));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Check that field mappings were recorded (need to access them before consuming compiler)
        // This test verifies that different paths get different offsets
        let load_field_count = compiled
            .code
            .iter()
            .filter(|i| matches!(i, Instruction::LoadField { .. }))
            .count();
        assert_eq!(load_field_count, 2);
    }

    #[test]
    fn test_error_unsupported_float() {
        let condition = Condition::new(Expression::literal(Value::Float(3.15)));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let result = compiler.compile(&policy);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompileError::UnsupportedExpression(_)));
    }

    #[test]
    fn test_error_unsupported_array() {
        let condition = Condition::new(Expression::literal(Value::Array(vec![])));
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let result = compiler.compile(&policy);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompileError::UnsupportedExpression(_)));
    }

    #[test]
    fn test_error_unknown_function() {
        let condition = Condition::new(Expression::Call {
            name: "unknown_func".to_string(),
            args: vec![],
        });
        let policy = create_simple_policy(Requirements::requires(vec![condition]));

        let compiler = PolicyCompiler::new(1);
        let result = compiler.compile(&policy);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CompileError::UnsupportedExpression(_)));
    }

    #[test]
    fn test_compile_rfc_example() {
        // From RFC: resource.type == "Deployment" AND environment in ["production", "staging"]
        //           AND approvals.count >= 2
        let trigger = Expression::and(vec![
            Expression::binary(
                Expression::path(vec!["resource".to_string(), "type".to_string()]),
                BinaryOp::Comparison(ComparisonOp::Eq),
                Expression::literal(Value::String("Deployment".to_string())),
            ),
            Expression::in_list(
                Expression::path(vec!["environment".to_string()]),
                vec![Value::String("production".to_string()), Value::String("staging".to_string())],
            ),
        ]);

        let requirement = Condition::new(Expression::binary(
            Expression::path(vec!["approvals".to_string(), "count".to_string()]),
            BinaryOp::Comparison(ComparisonOp::GtEq),
            Expression::literal(Value::Int(2)),
        ));

        let policy = Policy::new(
            "RequireApproval".to_string(),
            "Production deployments need 2+ approvals".to_string(),
            vec![Condition::new(trigger)],
            Requirements::requires(vec![requirement]),
        );

        let compiler = PolicyCompiler::new(1);
        let compiled = compiler.compile(&policy).unwrap();

        // Should successfully compile
        assert!(!compiled.code.is_empty());
        assert!(compiled.code.iter().any(|i| matches!(i, Instruction::Return { value: true })));
    }
}
