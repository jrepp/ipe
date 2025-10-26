//! Type system for IPE policies

use super::nodes::{Expression, Value, Condition};
use std::collections::HashMap;

/// Type information
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    String,
    Int,
    Float,
    Bool,
    Array(Box<Type>),
    Resource(String), // Named resource type
    Any,              // Unknown/dynamic type
}

impl Type {
    /// Check if this type is compatible with another
    pub fn is_compatible_with(&self, other: &Type) -> bool {
        match (self, other) {
            (Type::Any, _) | (_, Type::Any) => true,
            (Type::String, Type::String) => true,
            (Type::Int, Type::Int) => true,
            (Type::Float, Type::Float) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => true, // Allow int/float coercion
            (Type::Array(t1), Type::Array(t2)) => t1.is_compatible_with(t2),
            (Type::Resource(r1), Type::Resource(r2)) => r1 == r2,
            _ => false,
        }
    }

    /// Get type from value
    pub fn from_value(value: &Value) -> Self {
        match value {
            Value::String(_) => Type::String,
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::Bool(_) => Type::Bool,
            Value::Array(arr) => {
                if arr.is_empty() {
                    Type::Array(Box::new(Type::Any))
                } else {
                    Type::Array(Box::new(Type::from_value(&arr[0])))
                }
            }
        }
    }
}

/// Type environment for type checking
#[derive(Debug, Clone)]
pub struct TypeEnv {
    variables: HashMap<String, Type>,
}

impl TypeEnv {
    /// Create a new type environment
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Add a variable binding
    pub fn bind(&mut self, name: String, typ: Type) {
        self.variables.insert(name, typ);
    }

    /// Look up a variable type
    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.variables.get(name)
    }

    /// Create standard environment with built-in variables
    pub fn standard() -> Self {
        let mut env = Self::new();
        env.bind("resource".to_string(), Type::Resource("Resource".to_string()));
        env.bind("action".to_string(), Type::Resource("Action".to_string()));
        env.bind("request".to_string(), Type::Resource("Request".to_string()));
        env
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

/// Type checker for expressions
pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    /// Create a new type checker
    pub fn new(env: TypeEnv) -> Self {
        Self {
            env,
            errors: Vec::new(),
        }
    }

    /// Check the type of an expression
    pub fn check_expression(&mut self, expr: &Expression) -> Type {
        match expr {
            Expression::Literal(value) => Type::from_value(value),

            Expression::Path(path) => {
                // Look up the root in environment
                if let Some(root) = path.root() {
                    self.env.lookup(root).cloned().unwrap_or(Type::Any)
                } else {
                    Type::Any
                }
            }

            Expression::Binary { left, op: _, right } => {
                let left_type = self.check_expression(left);
                let right_type = self.check_expression(right);

                // Check compatibility
                if !left_type.is_compatible_with(&right_type) {
                    self.errors.push(TypeError::IncompatibleTypes {
                        left: left_type.clone(),
                        right: right_type.clone(),
                    });
                }

                // Binary comparisons return bool
                Type::Bool
            }

            Expression::Logical { op: _, operands } => {
                // Check all operands are boolean
                for operand in operands {
                    let typ = self.check_expression(operand);
                    if !matches!(typ, Type::Bool | Type::Any) {
                        self.errors.push(TypeError::ExpectedBool { got: typ });
                    }
                }
                Type::Bool
            }

            Expression::In { expr, list: _ } => {
                // Check expr type matches list element type
                let _expr_type = self.check_expression(expr);
                // TODO: Check list element types
                Type::Bool
            }

            Expression::Aggregate { .. } => {
                // Aggregate functions return their specific type
                Type::Int // Most aggregates return numbers
            }

            Expression::Call { name, args } => {
                // Check built-in functions
                self.check_function_call(name, args)
            }
        }
    }

    /// Check a condition
    pub fn check_condition(&mut self, cond: &Condition) -> Type {
        self.check_expression(&cond.expr)
    }

    fn check_function_call(&mut self, _name: &str, args: &[Expression]) -> Type {
        // Type check arguments
        for arg in args {
            self.check_expression(arg);
        }
        // Return Any for now - would need function signature database
        Type::Any
    }

    /// Get collected errors
    pub fn errors(&self) -> &[TypeError] {
        &self.errors
    }

    /// Check if there are any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Type checking errors
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    IncompatibleTypes { left: Type, right: Type },
    ExpectedBool { got: Type },
    UndefinedVariable { name: String },
    InvalidFieldAccess { base: Type, field: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::nodes::{BinaryOp, ComparisonOp};

    #[test]
    fn test_type_compatibility() {
        assert!(Type::String.is_compatible_with(&Type::String));
        assert!(Type::Int.is_compatible_with(&Type::Int));
        assert!(Type::Int.is_compatible_with(&Type::Float));
        assert!(Type::Float.is_compatible_with(&Type::Int));
        assert!(!Type::String.is_compatible_with(&Type::Int));
        assert!(Type::Any.is_compatible_with(&Type::String));
        assert!(Type::String.is_compatible_with(&Type::Any));
    }

    #[test]
    fn test_type_from_value() {
        assert_eq!(Type::from_value(&Value::String("test".to_string())), Type::String);
        assert_eq!(Type::from_value(&Value::Int(42)), Type::Int);
        assert_eq!(Type::from_value(&Value::Float(3.14)), Type::Float);
        assert_eq!(Type::from_value(&Value::Bool(true)), Type::Bool);
    }

    #[test]
    fn test_array_type() {
        let arr = Value::Array(vec![Value::Int(1), Value::Int(2)]);
        let typ = Type::from_value(&arr);
        assert!(matches!(typ, Type::Array(_)));
    }

    #[test]
    fn test_empty_array_type() {
        let arr = Value::Array(vec![]);
        let typ = Type::from_value(&arr);
        assert_eq!(typ, Type::Array(Box::new(Type::Any)));
    }

    #[test]
    fn test_type_env_creation() {
        let env = TypeEnv::new();
        assert!(env.lookup("foo").is_none());
    }

    #[test]
    fn test_type_env_binding() {
        let mut env = TypeEnv::new();
        env.bind("x".to_string(), Type::Int);
        assert_eq!(env.lookup("x"), Some(&Type::Int));
    }

    #[test]
    fn test_standard_env() {
        let env = TypeEnv::standard();
        assert!(env.lookup("resource").is_some());
        assert!(env.lookup("action").is_some());
        assert!(env.lookup("request").is_some());
    }

    #[test]
    fn test_check_literal() {
        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::literal(Value::Int(42));
        let typ = checker.check_expression(&expr);

        assert_eq!(typ, Type::Int);
        assert!(!checker.has_errors());
    }

    #[test]
    fn test_check_path() {
        let env = TypeEnv::standard();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::path(vec!["resource".to_string(), "type".to_string()]);
        let typ = checker.check_expression(&expr);

        assert!(matches!(typ, Type::Resource(_)));
        assert!(!checker.has_errors());
    }

    #[test]
    fn test_check_binary_compatible() {
        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::binary(
            Expression::literal(Value::Int(1)),
            BinaryOp::Comparison(ComparisonOp::Lt),
            Expression::literal(Value::Int(2)),
        );

        let typ = checker.check_expression(&expr);
        assert_eq!(typ, Type::Bool);
        assert!(!checker.has_errors());
    }

    #[test]
    fn test_check_binary_incompatible() {
        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::binary(
            Expression::literal(Value::String("hello".to_string())),
            BinaryOp::Comparison(ComparisonOp::Lt),
            Expression::literal(Value::Int(2)),
        );

        let typ = checker.check_expression(&expr);
        assert_eq!(typ, Type::Bool);
        assert!(checker.has_errors());
        assert_eq!(checker.errors().len(), 1);
    }

    #[test]
    fn test_check_logical_and() {
        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::and(vec![
            Expression::literal(Value::Bool(true)),
            Expression::literal(Value::Bool(false)),
        ]);

        let typ = checker.check_expression(&expr);
        assert_eq!(typ, Type::Bool);
        assert!(!checker.has_errors());
    }

    #[test]
    fn test_check_logical_with_non_bool() {
        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::and(vec![
            Expression::literal(Value::Bool(true)),
            Expression::literal(Value::Int(42)), // Wrong type
        ]);

        let typ = checker.check_expression(&expr);
        assert_eq!(typ, Type::Bool);
        assert!(checker.has_errors());
    }

    #[test]
    fn test_check_in_expression() {
        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::in_list(
            Expression::literal(Value::String("prod".to_string())),
            vec![
                Value::String("prod".to_string()),
                Value::String("staging".to_string()),
            ],
        );

        let typ = checker.check_expression(&expr);
        assert_eq!(typ, Type::Bool);
    }

    #[test]
    fn test_check_condition() {
        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let cond = Condition::new(Expression::literal(Value::Bool(true)));
        let typ = checker.check_condition(&cond);

        assert_eq!(typ, Type::Bool);
        assert!(!checker.has_errors());
    }

    #[test]
    fn test_int_float_coercion() {
        assert!(Type::Int.is_compatible_with(&Type::Float));
        assert!(Type::Float.is_compatible_with(&Type::Int));

        let env = TypeEnv::new();
        let mut checker = TypeChecker::new(env);

        let expr = Expression::binary(
            Expression::literal(Value::Int(1)),
            BinaryOp::Comparison(ComparisonOp::Lt),
            Expression::literal(Value::Float(2.0)),
        );

        checker.check_expression(&expr);
        assert!(!checker.has_errors());
    }

    #[test]
    fn test_resource_type_equality() {
        let t1 = Type::Resource("Deployment".to_string());
        let t2 = Type::Resource("Deployment".to_string());
        let t3 = Type::Resource("Service".to_string());

        assert!(t1.is_compatible_with(&t2));
        assert!(!t1.is_compatible_with(&t3));
    }
}
