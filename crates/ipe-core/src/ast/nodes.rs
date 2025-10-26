//! AST node definitions

use std::fmt;

/// A complete policy definition
#[derive(Debug, Clone, PartialEq)]
pub struct Policy {
    /// Policy name (identifier)
    pub name: String,
    /// Natural language intent string
    pub intent: String,
    /// Trigger conditions (when to evaluate this policy)
    pub triggers: Vec<Condition>,
    /// Requirements (what must be true for Allow)
    pub requirements: Requirements,
    /// Optional metadata
    pub metadata: Option<Metadata>,
    /// Source location
    pub location: SourceLocation,
}

impl Policy {
    /// Create a new policy
    pub fn new(
        name: String,
        intent: String,
        triggers: Vec<Condition>,
        requirements: Requirements,
    ) -> Self {
        Self {
            name,
            intent,
            triggers,
            requirements,
            metadata: None,
            location: SourceLocation::default(),
        }
    }

    /// Add metadata to the policy
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set source location
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = location;
        self
    }
}

/// Policy requirements (requires or denies)
#[derive(Debug, Clone, PartialEq)]
pub enum Requirements {
    /// Allow if conditions are met
    Requires { conditions: Vec<Condition>, where_clause: Option<Vec<Condition>> },
    /// Deny with optional reason
    Denies { reason: Option<String> },
}

impl Requirements {
    /// Create a requires clause
    pub fn requires(conditions: Vec<Condition>) -> Self {
        Self::Requires { conditions, where_clause: None }
    }

    /// Create a requires clause with where
    pub fn requires_where(conditions: Vec<Condition>, where_clause: Vec<Condition>) -> Self {
        Self::Requires {
            conditions,
            where_clause: Some(where_clause),
        }
    }

    /// Create a denies clause
    pub fn denies(reason: Option<String>) -> Self {
        Self::Denies { reason }
    }
}

/// A condition in triggers or requirements
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// The expression
    pub expr: Expression,
    /// Source location
    pub location: SourceLocation,
}

impl Condition {
    /// Create a new condition
    pub fn new(expr: Expression) -> Self {
        Self {
            expr,
            location: SourceLocation::default(),
        }
    }

    /// Create with location
    pub fn with_location(mut self, location: SourceLocation) -> Self {
        self.location = location;
        self
    }
}

/// An expression in the AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// Literal value
    Literal(Value),

    /// Path access (e.g., resource.type)
    Path(Path),

    /// Binary operation (e.g., x == y)
    Binary { left: Box<Expression>, op: BinaryOp, right: Box<Expression> },

    /// Logical operation (and, or, not)
    Logical { op: LogicalOp, operands: Vec<Expression> },

    /// Membership test (x in [a, b, c])
    In { expr: Box<Expression>, list: Vec<Value> },

    /// Aggregate function (count, any, all, etc.)
    Aggregate { path: Path, func: AggregateFunc, condition: Box<Condition> },

    /// Function call
    Call { name: String, args: Vec<Expression> },
}

impl Expression {
    /// Create a literal expression
    pub fn literal(value: Value) -> Self {
        Self::Literal(value)
    }

    /// Create a path expression
    pub fn path(segments: Vec<String>) -> Self {
        Self::Path(Path { segments })
    }

    /// Create a binary expression
    pub fn binary(left: Expression, op: BinaryOp, right: Expression) -> Self {
        Self::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
        }
    }

    /// Create a logical AND
    pub fn and(operands: Vec<Expression>) -> Self {
        Self::Logical { op: LogicalOp::And, operands }
    }

    /// Create a logical OR
    pub fn or(operands: Vec<Expression>) -> Self {
        Self::Logical { op: LogicalOp::Or, operands }
    }

    /// Create a NOT expression
    pub fn not(operand: Expression) -> Self {
        Self::Logical {
            op: LogicalOp::Not,
            operands: vec![operand],
        }
    }

    /// Create an IN expression
    pub fn in_list(expr: Expression, list: Vec<Value>) -> Self {
        Self::In { expr: Box::new(expr), list }
    }
}

/// A path (dot-separated identifiers)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path {
    pub segments: Vec<String>,
}

impl Path {
    /// Create a new path
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
    }

    /// Create a simple path with one segment
    pub fn simple(segment: String) -> Self {
        Self { segments: vec![segment] }
    }

    /// Get the first segment
    pub fn root(&self) -> Option<&str> {
        self.segments.first().map(|s| s.as_str())
    }

    /// Check if this is a simple path (one segment)
    pub fn is_simple(&self) -> bool {
        self.segments.len() == 1
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.segments.join("."))
    }
}

/// A value in the AST
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
}

impl Value {
    /// Check if value is truthy
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Float(f) => *f != 0.0,
        }
    }

    /// Get the type name
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "String",
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::Array(_) => "Array",
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Comparison
    Comparison(ComparisonOp),
}

/// Comparison operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonOp {
    Eq,   // ==
    Neq,  // !=
    Lt,   // <
    Gt,   // >
    LtEq, // <=
    GtEq, // >=
}

impl fmt::Display for ComparisonOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComparisonOp::Eq => write!(f, "=="),
            ComparisonOp::Neq => write!(f, "!="),
            ComparisonOp::Lt => write!(f, "<"),
            ComparisonOp::Gt => write!(f, ">"),
            ComparisonOp::LtEq => write!(f, "<="),
            ComparisonOp::GtEq => write!(f, ">="),
        }
    }
}

/// Logical operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicalOp {
    And,
    Or,
    Not,
}

impl fmt::Display for LogicalOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogicalOp::And => write!(f, "and"),
            LogicalOp::Or => write!(f, "or"),
            LogicalOp::Not => write!(f, "not"),
        }
    }
}

/// Aggregate functions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AggregateFunc {
    Count,
    Any,
    All,
    Sum,
    Max,
    Min,
}

impl fmt::Display for AggregateFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateFunc::Count => write!(f, "count"),
            AggregateFunc::Any => write!(f, "any"),
            AggregateFunc::All => write!(f, "all"),
            AggregateFunc::Sum => write!(f, "sum"),
            AggregateFunc::Max => write!(f, "max"),
            AggregateFunc::Min => write!(f, "min"),
        }
    }
}

/// Policy metadata
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    pub fields: Vec<(String, Value)>,
}

impl Metadata {
    /// Create empty metadata
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Add a field
    pub fn add_field(mut self, key: String, value: Value) -> Self {
        self.fields.push((key, value));
        self
    }

    /// Get a field value
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

impl Default for Metadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Source location for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub length: usize,
}

impl Default for SourceLocation {
    fn default() -> Self {
        Self { line: 0, column: 0, length: 0 }
    }
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(line: usize, column: usize, length: usize) -> Self {
        Self { line, column, length }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_creation() {
        let policy = Policy::new(
            "TestPolicy".to_string(),
            "Test intent".to_string(),
            vec![],
            Requirements::requires(vec![]),
        );

        assert_eq!(policy.name, "TestPolicy");
        assert_eq!(policy.intent, "Test intent");
        assert!(policy.triggers.is_empty());
        assert!(policy.metadata.is_none());
    }

    #[test]
    fn test_policy_with_metadata() {
        let metadata =
            Metadata::new().add_field("severity".to_string(), Value::String("high".to_string()));

        let policy = Policy::new(
            "Test".to_string(),
            "Intent".to_string(),
            vec![],
            Requirements::requires(vec![]),
        )
        .with_metadata(metadata);

        assert!(policy.metadata.is_some());
        assert_eq!(
            policy.metadata.unwrap().get("severity"),
            Some(&Value::String("high".to_string()))
        );
    }

    #[test]
    fn test_requirements_variants() {
        let req1 = Requirements::requires(vec![]);
        assert!(matches!(req1, Requirements::Requires { .. }));

        let req2 = Requirements::denies(Some("Not allowed".to_string()));
        assert!(matches!(req2, Requirements::Denies { .. }));
    }

    #[test]
    fn test_condition_creation() {
        let expr = Expression::literal(Value::Bool(true));
        let cond = Condition::new(expr.clone());

        assert_eq!(cond.expr, expr);
    }

    #[test]
    fn test_expression_literal() {
        let expr = Expression::literal(Value::Int(42));
        assert!(matches!(expr, Expression::Literal(Value::Int(42))));
    }

    #[test]
    fn test_expression_path() {
        let expr = Expression::path(vec!["resource".to_string(), "type".to_string()]);
        match expr {
            Expression::Path(path) => {
                assert_eq!(path.segments.len(), 2);
                assert_eq!(path.segments[0], "resource");
                assert_eq!(path.segments[1], "type");
            },
            _ => panic!("Expected path expression"),
        }
    }

    #[test]
    fn test_expression_binary() {
        let left = Expression::literal(Value::Int(1));
        let right = Expression::literal(Value::Int(2));
        let expr =
            Expression::binary(left.clone(), BinaryOp::Comparison(ComparisonOp::Lt), right.clone());

        match expr {
            Expression::Binary { left: l, op, right: r } => {
                assert_eq!(*l, left);
                assert_eq!(*r, right);
                assert_eq!(op, BinaryOp::Comparison(ComparisonOp::Lt));
            },
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_expression_logical_and() {
        let expr1 = Expression::literal(Value::Bool(true));
        let expr2 = Expression::literal(Value::Bool(false));
        let and_expr = Expression::and(vec![expr1.clone(), expr2.clone()]);

        match and_expr {
            Expression::Logical { op, operands } => {
                assert_eq!(op, LogicalOp::And);
                assert_eq!(operands.len(), 2);
            },
            _ => panic!("Expected logical expression"),
        }
    }

    #[test]
    fn test_expression_logical_or() {
        let expr1 = Expression::literal(Value::Bool(true));
        let expr2 = Expression::literal(Value::Bool(false));
        let or_expr = Expression::or(vec![expr1, expr2]);

        match or_expr {
            Expression::Logical { op, .. } => assert_eq!(op, LogicalOp::Or),
            _ => panic!("Expected logical OR"),
        }
    }

    #[test]
    fn test_expression_logical_not() {
        let expr = Expression::literal(Value::Bool(true));
        let not_expr = Expression::not(expr);

        match not_expr {
            Expression::Logical { op, operands } => {
                assert_eq!(op, LogicalOp::Not);
                assert_eq!(operands.len(), 1);
            },
            _ => panic!("Expected logical NOT"),
        }
    }

    #[test]
    fn test_expression_in() {
        let expr = Expression::path(vec!["env".to_string()]);
        let values = vec![Value::String("prod".to_string()), Value::String("staging".to_string())];
        let in_expr = Expression::in_list(expr.clone(), values.clone());

        match in_expr {
            Expression::In { expr: e, list } => {
                assert_eq!(*e, expr);
                assert_eq!(list, values);
            },
            _ => panic!("Expected IN expression"),
        }
    }

    #[test]
    fn test_path_creation() {
        let path = Path::new(vec!["resource".to_string(), "type".to_string()]);
        assert_eq!(path.segments.len(), 2);
        assert_eq!(path.root(), Some("resource"));
        assert!(!path.is_simple());
    }

    #[test]
    fn test_path_simple() {
        let path = Path::simple("foo".to_string());
        assert_eq!(path.segments.len(), 1);
        assert!(path.is_simple());
    }

    #[test]
    fn test_path_display() {
        let path = Path::new(vec!["resource".to_string(), "type".to_string()]);
        assert_eq!(path.to_string(), "resource.type");
    }

    #[test]
    fn test_value_is_truthy() {
        assert!(Value::Bool(true).is_truthy());
        assert!(!Value::Bool(false).is_truthy());
        assert!(Value::Int(1).is_truthy());
        assert!(!Value::Int(0).is_truthy());
        assert!(Value::String("hello".to_string()).is_truthy());
        assert!(!Value::String("".to_string()).is_truthy());
        assert!(Value::Array(vec![Value::Int(1)]).is_truthy());
        assert!(!Value::Array(vec![]).is_truthy());
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(Value::String("test".to_string()).type_name(), "String");
        assert_eq!(Value::Int(42).type_name(), "Int");
        assert_eq!(Value::Float(3.15).type_name(), "Float");
        assert_eq!(Value::Bool(true).type_name(), "Bool");
        assert_eq!(Value::Array(vec![]).type_name(), "Array");
    }

    #[test]
    fn test_comparison_op_display() {
        assert_eq!(ComparisonOp::Eq.to_string(), "==");
        assert_eq!(ComparisonOp::Neq.to_string(), "!=");
        assert_eq!(ComparisonOp::Lt.to_string(), "<");
        assert_eq!(ComparisonOp::Gt.to_string(), ">");
        assert_eq!(ComparisonOp::LtEq.to_string(), "<=");
        assert_eq!(ComparisonOp::GtEq.to_string(), ">=");
    }

    #[test]
    fn test_logical_op_display() {
        assert_eq!(LogicalOp::And.to_string(), "and");
        assert_eq!(LogicalOp::Or.to_string(), "or");
        assert_eq!(LogicalOp::Not.to_string(), "not");
    }

    #[test]
    fn test_aggregate_func_display() {
        assert_eq!(AggregateFunc::Count.to_string(), "count");
        assert_eq!(AggregateFunc::Any.to_string(), "any");
        assert_eq!(AggregateFunc::All.to_string(), "all");
        assert_eq!(AggregateFunc::Sum.to_string(), "sum");
        assert_eq!(AggregateFunc::Max.to_string(), "max");
        assert_eq!(AggregateFunc::Min.to_string(), "min");
    }

    #[test]
    fn test_metadata_operations() {
        let metadata = Metadata::new()
            .add_field("severity".to_string(), Value::String("high".to_string()))
            .add_field("owner".to_string(), Value::String("security-team".to_string()));

        assert_eq!(metadata.get("severity"), Some(&Value::String("high".to_string())));
        assert_eq!(metadata.get("owner"), Some(&Value::String("security-team".to_string())));
        assert_eq!(metadata.get("nonexistent"), None);
    }

    #[test]
    fn test_source_location() {
        let loc = SourceLocation::new(10, 5, 20);
        assert_eq!(loc.line, 10);
        assert_eq!(loc.column, 5);
        assert_eq!(loc.length, 20);
    }

    #[test]
    fn test_complex_policy_construction() {
        // Build a policy: resource.type == "Deployment" and environment in ["prod", "staging"]
        let trigger1 = Condition::new(Expression::binary(
            Expression::path(vec!["resource".to_string(), "type".to_string()]),
            BinaryOp::Comparison(ComparisonOp::Eq),
            Expression::literal(Value::String("Deployment".to_string())),
        ));

        let trigger2 = Condition::new(Expression::in_list(
            Expression::path(vec!["environment".to_string()]),
            vec![Value::String("prod".to_string()), Value::String("staging".to_string())],
        ));

        let requirement = Condition::new(Expression::binary(
            Expression::path(vec!["approvals".to_string(), "count".to_string()]),
            BinaryOp::Comparison(ComparisonOp::GtEq),
            Expression::literal(Value::Int(2)),
        ));

        let policy = Policy::new(
            "RequireApproval".to_string(),
            "Production deployments need 2+ approvals".to_string(),
            vec![trigger1, trigger2],
            Requirements::requires(vec![requirement]),
        );

        assert_eq!(policy.name, "RequireApproval");
        assert_eq!(policy.triggers.len(), 2);
        match &policy.requirements {
            Requirements::Requires { conditions, .. } => assert_eq!(conditions.len(), 1),
            _ => panic!("Expected requires"),
        }
    }

    #[test]
    fn test_denies_with_reason() {
        let requirements = Requirements::denies(Some("Access denied".to_string()));
        match requirements {
            Requirements::Denies { reason } => {
                assert_eq!(reason, Some("Access denied".to_string()));
            },
            _ => panic!("Expected denies"),
        }
    }

    #[test]
    fn test_requires_with_where() {
        let conditions = vec![Condition::new(Expression::literal(Value::Bool(true)))];
        let where_clause = vec![Condition::new(Expression::literal(Value::Bool(true)))];

        let requirements = Requirements::requires_where(conditions.clone(), where_clause.clone());

        match requirements {
            Requirements::Requires { conditions: c, where_clause: Some(w) } => {
                assert_eq!(c.len(), 1);
                assert_eq!(w.len(), 1);
            },
            _ => panic!("Expected requires with where"),
        }
    }
}
