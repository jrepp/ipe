# Abstract Syntax Tree (AST) Documentation

The IPE AST represents the parsed structure of policy definitions. The AST provides a typed, validated representation that can be compiled to bytecode or analyzed for correctness.

## AST Node Structure

```mermaid
classDiagram
    class Policy {
        +String name
        +String intent
        +Vec~Condition~ triggers
        +Requirements requirements
        +Vec~Metadata~ metadata
    }

    class Requirements {
        <<enumeration>>
        Requires(Vec~Condition~, Option~Vec~Condition~~)
        Denies(Option~String~)
    }

    class Condition {
        +Expression expr
    }

    class Expression {
        <<enumeration>>
        Literal(Value)
        Path(Vec~String~)
        Binary(Box~Expression~, BinaryOp, Box~Expression~)
        Logical(LogicalOp, Vec~Expression~)
        In(Box~Expression~, Vec~Value~)
        Aggregate(AggregateFunc, Box~Expression~)
        Call(String, Vec~Expression~)
    }

    class Value {
        <<enumeration>>
        String(String)
        Int(i64)
        Float(f64)
        Bool(bool)
        Array(Vec~Value~)
    }

    class BinaryOp {
        <<enumeration>>
        Comparison(ComparisonOp)
        Arithmetic(ArithmeticOp)
    }

    class ComparisonOp {
        <<enumeration>>
        Eq
        Neq
        Lt
        LtEq
        Gt
        GtEq
    }

    class LogicalOp {
        <<enumeration>>
        And
        Or
        Not
    }

    class AggregateFunc {
        <<enumeration>>
        Count
        Sum
        Avg
        Min
        Max
    }

    Policy --> Requirements
    Policy --> Condition : triggers
    Requirements --> Condition
    Condition --> Expression
    Expression --> Value
    Expression --> BinaryOp
    Expression --> LogicalOp
    Expression --> AggregateFunc
    BinaryOp --> ComparisonOp
```

## Policy Structure

A policy consists of:
1. **Name**: Unique identifier for the policy
2. **Intent**: Natural language description of policy purpose
3. **Triggers**: Conditions that determine when the policy applies
4. **Requirements**: What must be satisfied (Requires) or denied (Denies)
5. **Metadata**: Optional key-value pairs for additional context

### Example Policy

```rust
policy RequireApproval:
  "Production deployments need 2+ approvals from senior engineers"

  triggers when
    resource.type == "Deployment"
    and environment in ["production", "staging"]

  requires
    approvals.count >= 2
    where approver.role == "senior-engineer"
```

### AST Representation

```mermaid
graph TD
    P[Policy: RequireApproval]
    P --> I[Intent: String]
    P --> T[Triggers]
    P --> R[Requirements]

    T --> C1[Condition: AND]
    C1 --> B1[Binary: ==]
    C1 --> B2[Binary: IN]

    B1 --> P1[Path: resource.type]
    B1 --> V1[Value: Deployment]

    B2 --> P2[Path: environment]
    B2 --> A1[Array: prod, staging]

    R --> RC[Requires]
    RC --> C2[Condition: >=]
    C2 --> P3[Path: approvals.count]
    C2 --> V2[Value: 2]

    RC --> W[Where Clause]
    W --> C3[Condition: ==]
    C3 --> P4[Path: approver.role]
    C3 --> V3[Value: senior-engineer]
```

## Expression Types

### 1. Literal
Direct values embedded in the policy.

```rust
Expression::Literal(Value::Int(42))
Expression::Literal(Value::String("test"))
Expression::Literal(Value::Bool(true))
```

### 2. Path
Field references in RAR (Resource/Action/Request) context.

```rust
Expression::Path(vec!["resource", "type"])
Expression::Path(vec!["request", "principal", "id"])
```

### 3. Binary
Comparison and arithmetic operations.

```rust
Expression::Binary(
    Box::new(Expression::Path(vec!["age"])),
    BinaryOp::Comparison(ComparisonOp::Gt),
    Box::new(Expression::Literal(Value::Int(18)))
)
```

### 4. Logical
Boolean operations (AND, OR, NOT).

```rust
Expression::Logical(
    LogicalOp::And,
    vec![expr1, expr2, expr3]
)
```

### 5. In
Membership testing.

```rust
Expression::In(
    Box::new(Expression::Path(vec!["env"])),
    vec![Value::String("prod"), Value::String("staging")]
)
```

### 6. Aggregate
Collection operations.

```rust
Expression::Aggregate(
    AggregateFunc::Count,
    Box::new(Expression::Path(vec!["items"]))
)
```

### 7. Call
Function invocations.

```rust
Expression::Call(
    "max".to_string(),
    vec![expr1, expr2]
)
```

## Type System

```mermaid
classDiagram
    class Type {
        <<enumeration>>
        String
        Int
        Float
        Bool
        Array(Box~Type~)
        Resource(String)
    }

    class TypeEnv {
        +HashMap~String,Type~ bindings
        +bind(String, Type)
        +lookup(String) Type?
    }

    Type --> Type : Array element
    TypeEnv --> Type : bindings
```

### Type Checking

The type checker ensures:
- Binary operations have compatible operands
- Logical operations work on boolean expressions
- Path expressions resolve to valid fields
- Function calls have correct argument types

### Type Compatibility Rules

| Operation | Left Type | Right Type | Result Type |
|-----------|-----------|------------|-------------|
| `==, !=` | T | T | Bool |
| `<, <=, >, >=` | Int/Float | Int/Float | Bool |
| `and, or` | Bool | Bool | Bool |
| `not` | Bool | - | Bool |
| `in` | T | Array(T) | Bool |

### Int/Float Coercion

The type system allows Int to coerce to Float for comparison operations:
- `5 < 3.14` → Valid (Int coerces to Float)
- `3.14 > 5` → Valid (Int coerces to Float)

## Visitor Pattern

The AST implements the visitor pattern for traversal and analysis.

```mermaid
classDiagram
    class Visitor~T~ {
        <<trait>>
        +visit_policy(Policy) T
        +visit_expression(Expression) T
        +visit_condition(Condition) T
    }

    class Walker {
        +walk_policy(Visitor, Policy)
        +walk_expression(Visitor, Expression)
        +walk_condition(Visitor, Condition)
    }

    class CountingVisitor {
        +count: usize
    }

    class PathCollector {
        +paths: Vec~Vec~String~~
    }

    Visitor <|.. CountingVisitor
    Visitor <|.. PathCollector
    Walker ..> Visitor : uses
```

### Example Visitors

#### CountingVisitor
Counts nodes in the AST:
```rust
let mut visitor = CountingVisitor { count: 0 };
walk_policy(&mut visitor, &policy);
println!("Total nodes: {}", visitor.count);
```

#### PathCollector
Collects all field paths referenced:
```rust
let mut visitor = PathCollector::new();
walk_policy(&mut visitor, &policy);
println!("Paths: {:?}", visitor.paths);
```

## Construction Helpers

The AST provides builder methods for ergonomic construction:

```rust
// Expression builders
Expression::literal(Value::Int(42))
Expression::path(vec!["resource", "type"])
Expression::binary(left, op, right)
Expression::and(vec![expr1, expr2])
Expression::or(vec![expr1, expr2])
Expression::not(operand)
Expression::in_list(expr, values)

// Policy builders
Policy::new(name, intent, triggers, requirements)
Requirements::requires(conditions)
Requirements::requires_where(conditions, where_clause)
Requirements::denies(reason)
```

## Source Location Tracking

Each AST node includes source location information for error reporting:

```rust
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub file: Option<String>,
}
```

This enables precise error messages:
```
Error at line 12, column 5: Type mismatch in comparison
  resource.count == "five"
                    ^^^^^^ expected Int, found String
```

## Related Documentation

- [Bytecode Documentation](BYTECODE.md) - How AST compiles to bytecode
- [Parser Documentation](../crates/ipe-core/src/parser/) - How text becomes AST
- [Type System](../crates/ipe-core/src/ast/types.rs) - Type checking implementation
