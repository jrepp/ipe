//! Visitor pattern for traversing AST

use super::nodes::{Policy, Condition, Expression, Requirements, Path, Value};

/// Visitor trait for AST traversal
pub trait Visitor: Sized {
    /// Visit a policy
    fn visit_policy(&mut self, policy: &Policy) {
        walk_policy(self, policy);
    }

    /// Visit requirements
    fn visit_requirements(&mut self, requirements: &Requirements) {
        walk_requirements(self, requirements);
    }

    /// Visit a condition
    fn visit_condition(&mut self, condition: &Condition) {
        walk_condition(self, condition);
    }

    /// Visit an expression
    fn visit_expression(&mut self, expr: &Expression) {
        walk_expression(self, expr);
    }

    /// Visit a path
    fn visit_path(&mut self, _path: &Path) {
        // Leaf node, no children
    }

    /// Visit a value
    fn visit_value(&mut self, _value: &Value) {
        // Leaf node, no children
    }
}

/// Walk a policy node
pub fn walk_policy<V: Visitor>(visitor: &mut V, policy: &Policy) {
    // Visit triggers
    for trigger in &policy.triggers {
        visitor.visit_condition(trigger);
    }

    // Visit requirements
    visitor.visit_requirements(&policy.requirements);
}

/// Walk requirements
pub fn walk_requirements<V: Visitor>(visitor: &mut V, requirements: &Requirements) {
    match requirements {
        Requirements::Requires {
            conditions,
            where_clause,
        } => {
            for cond in conditions {
                visitor.visit_condition(cond);
            }
            if let Some(where_conds) = where_clause {
                for cond in where_conds {
                    visitor.visit_condition(cond);
                }
            }
        }
        Requirements::Denies { .. } => {
            // No sub-nodes to visit
        }
    }
}

/// Walk a condition
pub fn walk_condition<V: Visitor>(visitor: &mut V, condition: &Condition) {
    visitor.visit_expression(&condition.expr);
}

/// Walk an expression
pub fn walk_expression<V: Visitor>(visitor: &mut V, expr: &Expression) {
    match expr {
        Expression::Literal(value) => {
            visitor.visit_value(value);
        }

        Expression::Path(path) => {
            visitor.visit_path(path);
        }

        Expression::Binary { left, right, .. } => {
            visitor.visit_expression(left);
            visitor.visit_expression(right);
        }

        Expression::Logical { operands, .. } => {
            for operand in operands {
                visitor.visit_expression(operand);
            }
        }

        Expression::In { expr, list } => {
            visitor.visit_expression(expr);
            for value in list {
                visitor.visit_value(value);
            }
        }

        Expression::Aggregate { condition, .. } => {
            visitor.visit_condition(condition);
        }

        Expression::Call { args, .. } => {
            for arg in args {
                visitor.visit_expression(arg);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::nodes::{BinaryOp, ComparisonOp};

    /// Test visitor that counts nodes
    struct CountingVisitor {
        policies: usize,
        conditions: usize,
        expressions: usize,
        paths: usize,
        values: usize,
    }

    impl CountingVisitor {
        fn new() -> Self {
            Self {
                policies: 0,
                conditions: 0,
                expressions: 0,
                paths: 0,
                values: 0,
            }
        }
    }

    impl Visitor for CountingVisitor {
        fn visit_policy(&mut self, policy: &Policy) {
            self.policies += 1;
            walk_policy(self, policy);
        }

        fn visit_condition(&mut self, condition: &Condition) {
            self.conditions += 1;
            walk_condition(self, condition);
        }

        fn visit_expression(&mut self, expr: &Expression) {
            self.expressions += 1;
            walk_expression(self, expr);
        }

        fn visit_path(&mut self, _path: &Path) {
            self.paths += 1;
        }

        fn visit_value(&mut self, _value: &Value) {
            self.values += 1;
        }
    }

    #[test]
    fn test_count_simple_policy() {
        let policy = Policy::new(
            "Test".to_string(),
            "Intent".to_string(),
            vec![],
            Requirements::requires(vec![]),
        );

        let mut visitor = CountingVisitor::new();
        visitor.visit_policy(&policy);

        assert_eq!(visitor.policies, 1);
    }

    #[test]
    fn test_count_policy_with_triggers() {
        let trigger = Condition::new(Expression::literal(Value::Bool(true)));

        let policy = Policy::new(
            "Test".to_string(),
            "Intent".to_string(),
            vec![trigger],
            Requirements::requires(vec![]),
        );

        let mut visitor = CountingVisitor::new();
        visitor.visit_policy(&policy);

        assert_eq!(visitor.policies, 1);
        assert_eq!(visitor.conditions, 1);
        assert_eq!(visitor.expressions, 1);
        assert_eq!(visitor.values, 1);
    }

    #[test]
    fn test_count_binary_expression() {
        let expr = Expression::binary(
            Expression::literal(Value::Int(1)),
            BinaryOp::Comparison(ComparisonOp::Lt),
            Expression::literal(Value::Int(2)),
        );

        let mut visitor = CountingVisitor::new();
        visitor.visit_expression(&expr);

        assert_eq!(visitor.expressions, 3); // Binary + 2 literals
        assert_eq!(visitor.values, 2);
    }

    #[test]
    fn test_count_path_expression() {
        let expr = Expression::path(vec!["resource".to_string(), "type".to_string()]);

        let mut visitor = CountingVisitor::new();
        visitor.visit_expression(&expr);

        assert_eq!(visitor.expressions, 1);
        assert_eq!(visitor.paths, 1);
    }

    #[test]
    fn test_count_logical_expression() {
        let expr = Expression::and(vec![
            Expression::literal(Value::Bool(true)),
            Expression::literal(Value::Bool(false)),
        ]);

        let mut visitor = CountingVisitor::new();
        visitor.visit_expression(&expr);

        assert_eq!(visitor.expressions, 3); // AND + 2 literals
        assert_eq!(visitor.values, 2);
    }

    #[test]
    fn test_count_in_expression() {
        let expr = Expression::in_list(
            Expression::path(vec!["env".to_string()]),
            vec![Value::String("prod".to_string())],
        );

        let mut visitor = CountingVisitor::new();
        visitor.visit_expression(&expr);

        assert_eq!(visitor.expressions, 2); // IN + path
        assert_eq!(visitor.paths, 1);
        assert_eq!(visitor.values, 1);
    }

    #[test]
    fn test_count_complex_policy() {
        // Create a complex policy with multiple triggers and requirements
        let trigger1 = Condition::new(Expression::binary(
            Expression::path(vec!["resource".to_string(), "type".to_string()]),
            BinaryOp::Comparison(ComparisonOp::Eq),
            Expression::literal(Value::String("Deployment".to_string())),
        ));

        let trigger2 = Condition::new(Expression::in_list(
            Expression::path(vec!["environment".to_string()]),
            vec![
                Value::String("prod".to_string()),
                Value::String("staging".to_string()),
            ],
        ));

        let requirement = Condition::new(Expression::binary(
            Expression::path(vec!["approvals".to_string(), "count".to_string()]),
            BinaryOp::Comparison(ComparisonOp::GtEq),
            Expression::literal(Value::Int(2)),
        ));

        let policy = Policy::new(
            "ComplexPolicy".to_string(),
            "Complex policy for testing".to_string(),
            vec![trigger1, trigger2],
            Requirements::requires(vec![requirement]),
        );

        let mut visitor = CountingVisitor::new();
        visitor.visit_policy(&policy);

        assert_eq!(visitor.policies, 1);
        assert_eq!(visitor.conditions, 3); // 2 triggers + 1 requirement
        assert!(visitor.expressions > 5); // Multiple nested expressions
        assert_eq!(visitor.paths, 3); // resource.type, environment, approvals.count
        assert!(visitor.values > 3); // Multiple values
    }

    #[test]
    fn test_denies_requirements() {
        let policy = Policy::new(
            "DenyPolicy".to_string(),
            "Deny access".to_string(),
            vec![],
            Requirements::denies(Some("Not authorized".to_string())),
        );

        let mut visitor = CountingVisitor::new();
        visitor.visit_policy(&policy);

        assert_eq!(visitor.policies, 1);
        assert_eq!(visitor.conditions, 0); // Denies has no conditions
    }

    #[test]
    fn test_requires_with_where() {
        let conditions = vec![Condition::new(Expression::literal(Value::Bool(true)))];
        let where_clause = vec![Condition::new(Expression::literal(Value::Bool(false)))];

        let policy = Policy::new(
            "WherePolicy".to_string(),
            "Policy with where".to_string(),
            vec![],
            Requirements::requires_where(conditions, where_clause),
        );

        let mut visitor = CountingVisitor::new();
        visitor.visit_policy(&policy);

        assert_eq!(visitor.conditions, 2); // 1 requires + 1 where
        assert_eq!(visitor.expressions, 2);
        assert_eq!(visitor.values, 2);
    }

    /// Test visitor that collects all paths
    struct PathCollector {
        paths: Vec<String>,
    }

    impl PathCollector {
        fn new() -> Self {
            Self { paths: Vec::new() }
        }
    }

    impl Visitor for PathCollector {
        fn visit_path(&mut self, path: &Path) {
            self.paths.push(path.to_string());
        }
    }

    #[test]
    fn test_path_collection() {
        let expr = Expression::binary(
            Expression::path(vec!["resource".to_string(), "type".to_string()]),
            BinaryOp::Comparison(ComparisonOp::Eq),
            Expression::path(vec!["expected".to_string(), "value".to_string()]),
        );

        let mut collector = PathCollector::new();
        collector.visit_expression(&expr);

        assert_eq!(collector.paths.len(), 2);
        assert!(collector.paths.contains(&"resource.type".to_string()));
        assert!(collector.paths.contains(&"expected.value".to_string()));
    }
}
