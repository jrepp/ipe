# Bytecode Documentation

IPE compiles policies to bytecode, a sequence of instructions for the interpreter.
The instructions use a stack, which stores values in last-in, first-out order.
Optional just-in-time (JIT) compilation converts bytecode to native code at runtime.

## Instruction Set

```mermaid
classDiagram
    class Instruction {
        <<enumeration>>
        LoadField(u16 offset)
        LoadConst(u16 idx)
        Compare(CompOp op)
        And
        Or
        Not
        Jump(i16 offset)
        JumpIfFalse(i16 offset)
        Call(u16 func, u8 argc)
        Return(bool value)
    }

    class CompOp {
        <<enumeration>>
        Eq
        Neq
        Lt
        Lte
        Gt
        Gte
    }

    class Value {
        <<enumeration>>
        Int(i64)
        Bool(bool)
        String(String)
    }

    class CompiledPolicy {
        +u32 id
        +Vec~Instruction~ code
        +Vec~Value~ constants
    }

    Instruction --> CompOp
    CompiledPolicy --> Instruction
    CompiledPolicy --> Value
```

## Execution Model

```mermaid
sequenceDiagram
    participant P as Policy
    participant I as Interpreter
    participant S as Stack
    participant C as Context (RAR)

    P->>I: evaluate(context)
    I->>S: Create stack
    loop For each instruction
        alt LoadField
            I->>C: Get field value
            C-->>I: Value
            I->>S: Push value
        else LoadConst
            I->>P: Get constant
            P-->>I: Value
            I->>S: Push value
        else Compare
            S-->>I: Pop 2 values
            I->>I: Compare values
            I->>S: Push boolean result
        else And/Or/Not
            S-->>I: Pop operand(s)
            I->>I: Logical operation
            I->>S: Push boolean result
        else Return
            S-->>I: Get result
            I-->>P: Decision
        end
    end
```

## Instruction Details

### 1. LoadField { offset: u16 }

LoadField puts a field from the Resource/Action/Request (RAR) context on the stack.

Behavior:

- The interpreter reads the field mapping at the specified offset.
- It finds the field in the RAR structure (resource, action, request).
- It puts the field value on the stack.

Example:

```rust
LoadField { offset: 0 }  // Load resource.type
LoadField { offset: 1 }  // Load resource.owner
```

Stack Effect: `[] → [Value]`

---

### 2. LoadConst { idx: u16 }

LoadConst puts a constant on the stack. The constant pool stores values that the policy references.

Behavior:

- The interpreter reads the constant at the specified index.
- It clones the value onto the stack.

Example:

```rust
LoadConst { idx: 0 }  // Load constant "production"
LoadConst { idx: 1 }  // Load constant 42
```

Stack Effect: `[] → [Value]`

---

### 3. Compare { op: CompOp }

Compare applies the specified comparison operator to the top two stack values.

Operators:

- `Eq`: Equal (==)
- `Neq`: Not equal (!=)
- `Lt`: Less than (<)
- `Lte`: Less than or equal (<=)
- `Gt`: Greater than (>)
- `Gte`: Greater than or equal (>=)

Behavior:

- The interpreter removes two values from the stack (b, then a).
- It compares a and b with op.
- It puts the boolean result on the stack.

Example:

```rust
// Evaluate: resource.count >= 5
LoadField { offset: 0 }
LoadConst { idx: 0 }      // 5
Compare { op: CompOp::Gte }
```

Stack Effect: `[Value, Value] → [Bool]`

---

### 4. And

And applies logical AND to the top two boolean values. A boolean value is true or false.

Behavior:

- The interpreter removes two booleans from the stack.
- It computes a AND b.
- It puts the boolean result on the stack.

Example:

```rust
// Evaluate: is_admin and is_active
LoadField { offset: 0 }   // is_admin
LoadField { offset: 1 }   // is_active
And
```

Stack Effect: `[Bool, Bool] → [Bool]`

---

### 5. Or

Or applies logical OR to the top two boolean values.

Behavior:

- The interpreter removes two booleans from the stack.
- It computes a OR b.
- It puts the boolean result on the stack.

Example:

```rust
// Evaluate: is_admin or is_owner
LoadField { offset: 0 }   // is_admin
LoadField { offset: 1 }   // is_owner
Or
```

Stack Effect: `[Bool, Bool] → [Bool]`

---

### 6. Not

Not applies logical NOT to the top boolean value.

Behavior:

- The interpreter removes a boolean from the stack.
- It computes NOT a.
- It puts the boolean result on the stack.

Example:

```rust
// Evaluate: not is_suspended
LoadField { offset: 0 }   // is_suspended
Not
```

Stack Effect: `[Bool] → [Bool]`

---

### 7. Jump { offset: i16 }

Jump changes the program counter by a relative offset. The program counter records the position of the next instruction.

Behavior:

- The interpreter adds the offset to the program counter.
- It continues at the new location.

Example:

```rust
Jump { offset: 5 }  // Skip next 5 instructions
```

Stack Effect: No change

---

### 8. JumpIfFalse { offset: i16 }

If the top stack value is false, JumpIfFalse changes the program counter by a relative offset.

Behavior:

- The interpreter removes a boolean from the stack.
- If the value is false, the interpreter adds the offset to the program counter.
- Otherwise, it continues to the next instruction.

Example:

```rust
// Short-circuit AND: if (!a) skip b
LoadField { offset: 0 }    // a
JumpIfFalse { offset: 3 }  // Skip to after b evaluation
LoadField { offset: 1 }    // b
And
```

Stack Effect: `[Bool] → []`

---

### 9. Call { func: u16, argc: u8 }

Call runs a built-in function.

Behavior:

- The interpreter removes argc arguments from the stack.
- It calls the function with ID func.
- It puts the function result on the stack.

Example:

```rust
// Evaluate: max(a, b)
LoadField { offset: 0 }   // a
LoadField { offset: 1 }   // b
Call { func: 0, argc: 2 } // max function
```

Stack Effect: `[Value × argc] → [Value]`

---

### 10. Return { value: bool }

Return ends policy evaluation with a boolean decision.

Behavior:

- The interpreter clears the stack.
- It returns the boolean decision.
- It stops execution.

Example:

```rust
Return { value: true }   // Allow
Return { value: false }  // Deny
```

Stack Effect: `[*] → [] (terminates)`

## Compilation Examples

### Simple Comparison

Policy:

```
requires resource.count >= 5
```

Bytecode:

```rust
LoadField { offset: 0 }   // resource.count
LoadConst { idx: 0 }      // 5
Compare { op: Gte }
Return { value: true }
```

---

### Logical AND

Policy:

```
requires
  resource.type == "Document"
  and resource.confidential == true
```

Bytecode:

```rust
// resource.type == "Document"
LoadField { offset: 0 }
LoadConst { idx: 0 }      // "Document"
Compare { op: Eq }

// resource.confidential == true
LoadField { offset: 1 }
LoadConst { idx: 1 }      // true
Compare { op: Eq }

// Combine with AND
And
Return { value: true }
```

---

### IN Expression

Policy:

```
requires environment in ["prod", "staging"]
```

Bytecode (expanded to OR chain):

```rust
// environment == "prod"
LoadField { offset: 0 }
LoadConst { idx: 0 }      // "prod"
Compare { op: Eq }

// environment == "staging"
LoadField { offset: 0 }
LoadConst { idx: 1 }      // "staging"
Compare { op: Eq }

// Combine with OR
Or
Return { value: true }
```

---

### Short-Circuit AND

Policy:

```
requires is_admin and has_permission
```

Optimized Bytecode (with short-circuit):

```rust
LoadField { offset: 0 }    // is_admin
JumpIfFalse { offset: 3 }  // Skip if false
LoadField { offset: 1 }    // has_permission
And
Return { value: true }
```

## Constant Pool

The constant pool stores literal values referenced by LoadConst instructions.

```mermaid
graph LR
    C[Constant Pool]
    C --> C0["idx=0: Int(42)"]
    C --> C1["idx=1: String(prod)"]
    C --> C2["idx=2: Bool(true)"]
    C --> C3["idx=3: String(admin)"]
```

### Constant Deduplication

The compiler stores each constant once to reduce bytecode size:

```rust
// Before deduplication
LoadConst { idx: 0 }  // "admin"
LoadConst { idx: 1 }  // "admin"  (duplicate)

// After deduplication
LoadConst { idx: 0 }  // "admin"
LoadConst { idx: 0 }  // "admin"  (reused)
```

## Field Mapping

Field offsets map to RAR (Resource/Action/Request) paths:

```mermaid
graph TD
    FM[Field Mapping]
    FM --> F0["offset=0 → resource.type"]
    FM --> F1["offset=1 → resource.owner"]
    FM --> F2["offset=2 → request.principal.id"]
    FM --> F3["offset=3 → resource.confidential"]
```

Example Mapping:

```rust
HashMap {
    0 => vec!["resource", "type"],
    1 => vec!["resource", "owner"],
    2 => vec!["request", "principal", "id"],
    3 => vec!["resource", "confidential"],
}
```

## Interpreter Architecture

```mermaid
graph TB
    subgraph Interpreter
        PC[Program Counter]
        S[Stack]
        FM[Field Mapping]
    end

    subgraph Policy
        C[Code]
        CP[Constant Pool]
    end

    subgraph Context
        R[Resource]
        A[Action]
        Q[Request]
    end

    PC -->|fetch| C
    C -->|execute| S
    S -->|LoadConst| CP
    S -->|LoadField| FM
    FM -->|lookup| R
    FM -->|lookup| A
    FM -->|lookup| Q
```

### Stack

- Type: Vec<Value>
- Max Size: 1024 elements
- Operations: push, pop, peek, clear
- Bounds Checking: Yes (prevents overflow/underflow)

### Program Counter

- Type: usize
- Range: 0 to code.len()
- Bounds Checking: Yes (prevents out-of-bounds access)

## Performance Characteristics

| Operation | Time Complexity | Notes |
|-----------|----------------|-------|
| LoadField | O(path_depth) | Navigate RAR structure |
| LoadConst | O(1) | Index into constant pool |
| Compare | O(1) | Type-specific comparison |
| And/Or/Not | O(1) | Boolean operations |
| Jump | O(1) | Direct offset |
| Call | O(f) | Function-dependent |
| Return | O(1) | Immediate termination |

### Memory Usage

Typical Policy:

- Code: ~50-200 bytes
- Constants: ~20-100 bytes
- Field Mapping: ~50-150 bytes
- Total: ~120-450 bytes per policy

Stack:

- Size: 8 bytes × stack_depth
- Max: 8 KB (1024 elements)

## Optimization Opportunities

### 1. Constant Folding

```rust
// Before
LoadConst { idx: 0 }  // 5
LoadConst { idx: 1 }  // 3
Compare { op: Lt }    // 5 < 3 = false

// After
LoadConst { idx: 2 }  // false (precomputed)
```

### 2. Dead Code Elimination

```rust
// Before
LoadConst { idx: 0 }  // false
JumpIfFalse { offset: 5 }
...  // Dead code (never executed)
Return { value: true }

// After
Return { value: false }  // Direct return
```

### 3. Peephole Optimization

```rust
// Before
LoadConst { idx: 0 }
Not
Not

// After
LoadConst { idx: 0 }  // Double negation eliminated
```

## JIT Compilation

Cranelift can compile bytecode to native code:

```mermaid
graph LR
    B[Bytecode] -->|Profile| H{Hot?}
    H -->|Yes| J[JIT Compiler]
    H -->|No| I[Interpreter]
    J --> N[Native Code]
    N --> E[Execute]
    I --> E
```

Thresholds:

- Baseline JIT: 100+ evaluations
- Optimized JIT: 10,000+ evaluations, average latency > 20μs

Performance:

- Interpreter: ~50μs per policy
- Baseline JIT: ~20μs per policy (2-3x faster)
- Optimized JIT: ~10μs per policy (5-10x faster)

## Related Documentation

- [AST Documentation](AST.md) - How policies parse to AST
- [Interpreter](../crates/ipe-core/src/interpreter.rs) - Bytecode interpreter implementation
- [JIT Compiler](../crates/ipe-core/src/jit.rs) - Native code generation
- [Compiler](../crates/ipe-core/src/compiler.rs) - AST to bytecode compilation
