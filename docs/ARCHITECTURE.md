# Architecture Overview

IPE evaluates policies through several execution tiers. An execution tier is a method that runs a policy. Concurrent readers can access policies without locks.

## System Architecture

```mermaid
graph TB
    subgraph "Input Layer"
        PS[Policy Source<br/>.ipe files]
    end

    subgraph "Parsing Layer"
        L[Lexer]
        P[Parser]
        PS --> L
        L --> P
    end

    subgraph "AST Layer"
        AST[Abstract Syntax Tree]
        TC[Type Checker]
        P --> AST
        AST --> TC
    end

    subgraph "Compilation Layer"
        C[Compiler]
        BC[Bytecode]
        FM[Field Mapping]
        TC --> C
        C --> BC
        C --> FM
    end

    subgraph "Storage Layer"
        DS[PolicyDataStore<br/>Lock-Free]
        SS[PolicySnapshot<br/>Immutable]
        DS -.Arc Clone.-> SS
        BC --> DS
        FM --> DS
    end

    subgraph "Execution Layer"
        I[Interpreter<br/>Baseline]
        BJ[Baseline JIT<br/>100+ evals]
        OJ[Optimized JIT<br/>10k+ evals]

        SS --> I
        SS --> BJ
        SS --> OJ
    end

    subgraph "Evaluation Layer"
        E[PolicyEngine]
        R[RAR Context<br/>Resource/Action/Request]

        I --> E
        BJ --> E
        OJ --> E
        R --> E
    end

    subgraph "Output Layer"
        D[Decision<br/>Allow/Deny]
        E --> D
    end

    style PS fill:#e1f5ff
    style BC fill:#fff4e1
    style DS fill:#f0f0f0
    style D fill:#e8f5e9
```

## Component Details

### 1. Lexer & Parser

The lexer splits policy text into tokens, the units of source text.
The parser converts these tokens into an abstract syntax tree (AST).
An AST represents the structure of a policy.

```mermaid
sequenceDiagram
    participant S as Source
    participant L as Lexer
    participant P as Parser
    participant AST as AST

    S->>L: "policy Foo: ..."
    L->>L: Tokenize
    L->>P: Token stream
    P->>P: Parse grammar
    P->>AST: Build nodes
    AST-->>P: Policy AST
```

Files:

- [`crates/ipe-core/src/parser/lexer.rs`](../crates/ipe-core/src/parser/lexer.rs) - Tokenization
- [`crates/ipe-core/src/parser/parse.rs`](../crates/ipe-core/src/parser/parse.rs) - Recursive descent parser
- [`crates/ipe-core/src/parser/token.rs`](../crates/ipe-core/src/parser/token.rs) - Token definitions

Output: Abstract Syntax Tree (AST)

---

### 2. AST & Type System

The AST represents policies with types. The type system makes sure that policy expressions use compatible types.

```mermaid
classDiagram
    class Policy {
        +name: String
        +intent: String
        +triggers: Vec~Condition~
        +requirements: Requirements
    }

    class Type {
        <<enumeration>>
        String
        Int
        Float
        Bool
        Resource
    }

    class TypeChecker {
        +check(AST) Result
    }

    Policy --> Type
    TypeChecker --> Policy
```

Features:

- Type inference from literals
- Type compatibility checking
- Int/Float coercion
- Path resolution

Files:

- [`crates/ipe-core/src/ast/nodes.rs`](../crates/ipe-core/src/ast/nodes.rs) - AST node definitions
- [`crates/ipe-core/src/ast/types.rs`](../crates/ipe-core/src/ast/types.rs) - Type system
- [`crates/ipe-core/src/ast/visitor.rs`](../crates/ipe-core/src/ast/visitor.rs) - Visitor pattern

Documentation: [AST Documentation](AST.md)

---

### 3. Bytecode Compiler

The compiler converts the AST to bytecode, a sequence of instructions for the interpreter.
The instructions use a stack, which stores values in last-in, first-out order.

```mermaid
graph LR
    A[AST] --> C[Compiler]
    C --> B[Bytecode]
    C --> FM[Field Mapping]
    C --> CP[Constant Pool]

    B -.contains.-> I[Instructions]
    CP -.contains.-> V[Values]
```

Optimizations:

- Constant deduplication
- IN expression expansion to OR chains
- Field offset allocation

Files:

- [`crates/ipe-core/src/compiler.rs`](../crates/ipe-core/src/compiler.rs) - AST to bytecode compiler
- [`crates/ipe-core/src/bytecode.rs`](../crates/ipe-core/src/bytecode.rs) - Bytecode definitions

Documentation: [Bytecode Documentation](BYTECODE.md)

---

### 4. PolicyDataStore (removed)

Removed in #36 - the v2 runtime uses per-scope snapshots from `runtime_facts.rs` instead. (Historical section retained below for context.)

PolicyDataStore stored policies for concurrent readers.

```mermaid
graph TB
    subgraph "Writer Thread"
        W[Writer] -->|Update Request| UV[Validation Worker]
    end

    subgraph "PolicyDataStore"
        UV -->|Validate & Compile| NS[New Snapshot]
        NS -->|Atomic Swap| CS[Current Snapshot<br/>Arc~PolicySnapshot~]
    end

    subgraph "Reader Threads"
        R1[Reader 1] -.Arc::clone.-> CS
        R2[Reader 2] -.Arc::clone.-> CS
        R3[Reader N] -.Arc::clone.-> CS
    end

    style CS fill:#f0f0f0
    style NS fill:#fff4e1
```

Features:

- Lock-free reads: Arc::clone() reads require no locks.
- Atomic updates: The store replaces each immutable snapshot in one operation.
- Background validation: A worker pool makes sure that policies are valid.
- Continuous access: Existing readers retain their snapshot during updates.

Performance:

- Reads: O(1) with Arc clone
- Updates: Asynchronous, non-blocking
- Memory: Copy-on-write (the store drops unused old snapshots)

Files:

- [`crates/ipe-core/src/store.rs`](../crates/ipe-core/src/store.rs) - Lock-free store implementation
- [`crates/ipe-core/src/index.rs`](../crates/ipe-core/src/index.rs) - Resource type indexing

---

### 5. Execution Tiers

Policy usage determines the execution tier.

```mermaid
stateDiagram-v2
    [*] --> Interpreter
    Interpreter --> BaselineJIT: 100+ evaluations
    BaselineJIT --> OptimizedJIT: 10k+ evals<br/>+ avg latency > 20μs
    OptimizedJIT --> [*]

    note right of Interpreter
        Default tier
        ~50μs per eval
    end note

    note right of BaselineJIT
        Fast compilation
        ~20μs per eval
    end note

    note right of OptimizedJIT
        Full optimization
        ~10μs per eval
    end note
```

Promotion Logic:

```rust
match tier {
    Interpreter => {
        // Promote after 100 evaluations
        count >= 100
    },
    BaselineJIT => {
        // Promote after 10k evals AND high latency
        count >= 10_000 && avg_latency > 20_000
    },
    OptimizedJIT => {
        // Already at top tier
        false
    },
}
```

Files:

- [`crates/ipe-core/src/tiering.rs`](../crates/ipe-core/src/tiering.rs) - Adaptive tiering
- [`crates/ipe-core/src/jit.rs`](../crates/ipe-core/src/jit.rs) - JIT compiler (Cranelift)

---

### 6. Interpreter

The interpreter runs bytecode policies.

```mermaid
sequenceDiagram
    participant E as Engine
    participant I as Interpreter
    participant S as Stack
    participant R as RAR Context

    E->>I: evaluate(policy, ctx)
    I->>S: Create stack
    loop For each instruction
        alt LoadField
            I->>R: Get field value
            R-->>I: Value
            I->>S: Push
        else LoadConst
            I->>I: Get constant
            I->>S: Push
        else Compare/And/Or
            S-->>I: Pop operands
            I->>I: Compute
            I->>S: Push result
        else Return
            S-->>I: Get result
            I-->>E: Decision
        end
    end
```

Stack Operations:

- Push adds a value to the stack.
- Pop removes and returns the top value.
- Peek returns the top value without removal.
- Clear resets the stack for the next evaluation.

Files:

- [`crates/ipe-core/src/interpreter.rs`](../crates/ipe-core/src/interpreter.rs) - Bytecode interpreter
- [`crates/ipe-core/src/rar.rs`](../crates/ipe-core/src/rar.rs) - Resource/Action/Request context

---

### 7. PolicyEngine (Public API)

PolicyEngine provides the public interface for policy evaluation.

```mermaid
graph TB
    PE[PolicyEngine]
    PDB[PolicyDB]
    CTX[EvaluationContext]
    D[Decision]

    PE --> PDB
    PE --> CTX
    PE --> D

    PDB -.indexed by.-> RT[ResourceType]
    D -.contains.-> MP[Matched Policies]
    D -.contains.-> R[Reason]
```

Decision Resolution:

1. No policies: The engine denies the request.
2. All policies allow: The engine allows the request and lists the matched policies.
3. Any policy denies: The engine denies the request. A deny overrides an allow.
4. Evaluation errors: The engine returns the error with the policy name.

Files:

- [`crates/ipe-core/src/engine.rs`](../crates/ipe-core/src/engine.rs) - Public API
- [`crates/ipe-core/src/index.rs`](../crates/ipe-core/src/index.rs) - Policy database

---

## Data Flow

### Policy Compilation

```mermaid
flowchart LR
    PS[".ipe source"] -->|Lexer| T[Tokens]
    T -->|Parser| AST[AST]
    AST -->|TypeCheck| VAST[Validated AST]
    VAST -->|Compiler| BC[Bytecode]
    style PS fill:#e1f5ff
    style BC fill:#fff4e1
```

### Policy Evaluation

```mermaid
flowchart LR
    CTX[EvaluationContext<br/>RAR] -->|Engine| E[PolicyEngine]
    E -->|Query| DS[PolicyDataStore]
    DS -->|Snapshot| SS[PolicySnapshot]
    SS -->|Indexed Lookup| P[Relevant Policies]
    P -->|Execute| I[Interpreter/JIT]
    I -->|Result| D[Decision]

    style CTX fill:#e1f5ff
    style D fill:#e8f5e9
```

## Performance Characteristics

| Component | Operation | Time Complexity | Notes |
|-----------|-----------|----------------|-------|
| Lexer | Tokenize | O(n) | n = source length |
| Parser | Parse | O(n) | n = token count |
| TypeChecker | Check | O(nodes) | nodes = AST size |
| Compiler | Compile | O(nodes) | nodes = AST size |
| DataStore | Read | O(1) | Arc clone |
| DataStore | Update | O(m) | m = policy count |
| Interpreter | Evaluate | O(k) | k = instruction count |
| Engine | Query | O(p) | p = matching policies |

Typical Latencies:

- Parsing: 100-500μs per policy
- Compilation: 200-800μs per policy
- Evaluation (Interpreter): 10-50μs per policy
- Evaluation (JIT): 5-20μs per policy

## Concurrency Model

### Lock-Free Reads

```mermaid
sequenceDiagram
    participant R1 as Reader Thread 1
    participant R2 as Reader Thread 2
    participant DS as DataStore
    participant S1 as Snapshot v1
    participant S2 as Snapshot v2
    participant W as Writer Thread

    R1->>DS: snapshot()
    DS->>S1: Arc::clone()
    DS-->>R1: Arc<Snapshot v1>

    W->>DS: update(new_policy)
    DS->>DS: Validate & compile
    DS->>S2: Create new snapshot
    DS->>DS: Atomic swap

    R2->>DS: snapshot()
    DS->>S2: Arc::clone()
    DS-->>R2: Arc<Snapshot v2>

    R1->>R1: Evaluate (still using v1)
    R2->>R2: Evaluate (using v2)

    note over R1,R2: Both readers run concurrently<br/>without blocking
```

Read behavior:

- Readers and writers do not share locks.
- Readers do not compete with other readers for locks.
- Writers do not block readers.
- Readers see a consistent snapshot.

### Memory Management

Reference Counting:

```rust
Arc<PolicySnapshot> // Atomic reference counted

// When last Arc drops:
//  1. Snapshot memory freed
//  2. Policy bytecode freed
//  3. Constant pools freed
```

Typical Memory Usage:

- 1 Policy: ~500 bytes
- 100 Policies: ~50 KB
- 10,000 Policies: ~5 MB
- Snapshot overhead: ~100 bytes

## Testing & Quality

Test Coverage: 93.67% (248 tests)

Test Pyramid:

```mermaid
graph TB
    U[Unit Tests<br/>171 tests]
    I[Integration Tests<br/>52 tests]
    E2E[End-to-End Tests<br/>25 tests]

    E2E --> I
    I --> U

    style U fill:#e8f5e9
    style I fill:#fff4e1
    style E2E fill:#e1f5ff
```

Key Test Files:

- [`crates/ipe-core/src/testing.rs`](../crates/ipe-core/src/testing.rs) - Test helpers
- Each module has tests in `mod tests`

## Related Documentation

- [AST Documentation](AST.md) - Abstract syntax tree details
- [Bytecode Documentation](BYTECODE.md) - Instruction set and execution
- [README](../README.md) - Project overview and quick start
- [RFC](../RFC.md) - Technical specification
