# End-to-End Example: Policy Lifecycle with JIT

This document demonstrates the complete lifecycle of a policy in IPE, from creation to JIT-optimized execution.

## Step 1: Write Policy (Human)

A DevOps engineer writes a policy in natural language:

```rust
policy RequireApproval:
  "Production deployments of high-risk services need 2+ approvals from senior engineers
   in different departments to ensure proper oversight and reduce single-point-of-failure risk"
  
  triggers when
    resource.type == "Deployment"
    and environment == "production"
    and resource.risk_level in ["high", "critical"]
  
  requires
    approvals.count >= 2
    where approver.role == "senior-engineer"
    and approver.department != requester.department
  
  metadata
    severity: critical
    owner: security-team
    tags: [compliance, deployment, approval]
    version: 1.2.0
```

## Step 2: Compilation Pipeline

### 2a. Parsing (nom)

```rust
// Parser output (AST)
PolicyAst {
    name: "RequireApproval",
    intent: "Production deployments of high-risk...",
    triggers: vec![
        Condition::Comparison {
            path: Path::from("resource.type"),
            op: CompOp::Eq,
            value: Value::String("Deployment")
        },
        Condition::Comparison {
            path: Path::from("environment"),
            op: CompOp::Eq,
            value: Value::String("production")
        },
        Condition::Membership {
            path: Path::from("resource.risk_level"),
            values: vec!["high", "critical"]
        }
    ],
    requirements: Requirements::Aggregate {
        condition: Condition::Comparison {
            path: Path::from("approvals.count"),
            op: CompOp::Gte,
            value: Value::Int(2)
        },
        filter: Some(vec![
            // ... approver conditions
        ])
    }
}
```

### 2b. Optimization

```rust
// Query planner reorders for selectivity
QueryPlan {
    // Most selective first: resource type (indexed, 99% reduction)
    indexed_filters: vec![
        IndexedFilter::Eq("resource.type", "Deployment")
    ],
    
    // Next: environment check (90% reduction of remaining)
    ordered_conditions: vec![
        Condition::Comparison { path: "environment", ... },
        Condition::Membership { path: "resource.risk_level", ... },
    ],
    
    // Aggregate last (most expensive)
    aggregate: Some(AggregateCondition { ... })
}
```

### 2c. Bytecode Generation

```rust
// Compact bytecode (~180 bytes)
CompiledPolicy {
    header: PolicyHeader {
        magic: [73, 80, 69, 0],  // "IPE\0"
        version: 1,
        policy_id: 42,
        code_size: 15,
        const_size: 4,
    },
    code: vec![
        // Load resource.type
        LoadField { offset: 0 },           // 0: Load resource type
        LoadConst { idx: 0 },              // 1: Load "Deployment"
        Compare { op: CompOp::Eq },        // 2: Compare
        JumpIfFalse { offset: 12 },        // 3: Short-circuit if false
        
        // Load environment
        LoadField { offset: 8 },           // 4: Load environment
        LoadConst { idx: 1 },              // 5: Load "production"
        Compare { op: CompOp::Eq },        // 6: Compare
        JumpIfFalse { offset: 8 },         // 7: Short-circuit if false
        
        // Check risk level
        LoadField { offset: 16 },          // 8: Load risk_level
        LoadConst { idx: 2 },              // 9: Load ["high", "critical"]
        Call { func: 0, argc: 2 },         // 10: in_array()
        JumpIfFalse { offset: 4 },         // 11: Short-circuit if false
        
        // Aggregate check (approvals)
        Call { func: 1, argc: 3 },         // 12: count_where()
        LoadConst { idx: 3 },              // 13: Load 2
        Compare { op: CompOp::Gte },       // 14: >= check
        
        Return { value: true },            // 15: Return result
    ],
    constants: vec![
        Value::String("Deployment"),
        Value::String("production"),
        Value::Array(vec!["high", "critical"]),
        Value::Int(2),
    ]
}
```

## Step 3: Initial Deployment (Interpreter Mode)

```rust
// Control plane deploys policy atomically
let policy = TieredPolicy::new(compiled_bytecode, "RequireApproval");

// First 100 evaluations use interpreter
for request in deployment_requests.take(100) {
    let ctx = build_context(request);
    
    // Interpreter execution (~50μs)
    let start = Instant::now();
    let decision = policy.evaluate(&ctx)?;
    let latency = start.elapsed();
    
    println!("Eval {}: {}μs (Interpreter)", i, latency.as_micros());
    // Output: Eval 0: 47μs (Interpreter)
    // Output: Eval 50: 49μs (Interpreter)
    // Output: Eval 99: 48μs (Interpreter)
}
```

## Step 4: JIT Compilation (Automatic)

After 100 evaluations, the tiering system triggers JIT compilation:

```rust
// Background thread triggered automatically
async fn jit_compile_hot_policy(policy: Arc<TieredPolicy>) {
    let mut compiler = JitCompiler::new()?;
    
    // Translate bytecode to Cranelift IR
    let ir = compiler.translate_to_ir(&policy.bytecode)?;
    
    // Cranelift IR (simplified)
    fn0(ctx: i64) -> i8 {
    block0(v0: i64):
        // Load resource.type from ctx
        v1 = load.i64 v0+0
        v2 = iconst.i64 0x7f8a... // "Deployment" ptr
        v3 = call strcmp(v1, v2)
        v4 = icmp eq v3, 0
        brz v4, block_deny        // Short-circuit
        
        // Load environment
        v5 = load.i64 v0+8
        v6 = iconst.i64 0x7f8b... // "production" ptr
        v7 = call strcmp(v5, v6)
        v8 = icmp eq v7, 0
        brz v8, block_deny        // Short-circuit
        
        // ... more conditions
        
        jump block_allow
    
    block_allow:
        v20 = iconst.i8 1
        return v20
        
    block_deny:
        v21 = iconst.i8 0
        return v21
    }
    
    // Compile to native code (~800μs)
    let native_code = compiler.compile(ir)?;
    
    // Store in policy (lock-free)
    policy.jit_code.store(Some(native_code));
    policy.tier.store(ExecutionTier::BaselineJIT);
    
    tracing::info!(
        "JIT compiled policy '{}': ~500μs compile, 5x speedup expected",
        policy.name
    );
}
```

## Step 5: Native Execution (Hot Path)

```rust
// Next 1000 evaluations use JIT-compiled code
for request in deployment_requests.take(1000) {
    let ctx = build_context(request);
    
    // JIT execution (~10μs)
    let start = Instant::now();
    let decision = policy.evaluate(&ctx)?;  // Automatically uses JIT
    let latency = start.elapsed();
    
    println!("Eval {}: {}μs (JIT)", i, latency.as_micros());
    // Output: Eval 100: 9μs (JIT)
    // Output: Eval 500: 8μs (JIT)
    // Output: Eval 999: 7μs (JIT)
}
```

### Native Code Generated (x86_64 assembly)

```asm
; Function: eval_require_approval
; Signature: fn(*const Context) -> bool

eval_require_approval:
    push    rbp
    mov     rbp, rsp
    
    ; Load resource.type (ctx + 0)
    mov     rax, [rdi + 0]
    lea     rsi, [rip + .Lstr_deployment]
    call    strcmp
    test    eax, eax
    jnz     .Ldeny                    ; Fast path: early exit
    
    ; Load environment (ctx + 8)
    mov     rax, [rdi + 8]
    lea     rsi, [rip + .Lstr_production]
    call    strcmp
    test    eax, eax
    jnz     .Ldeny                    ; Fast path: early exit
    
    ; Load risk_level (ctx + 16)
    mov     rax, [rdi + 16]
    mov     rcx, 2                    ; Array size
    lea     rdx, [rip + .Larr_risk_levels]
    call    in_array
    test    al, al
    jz      .Ldeny                    ; Fast path: early exit
    
    ; Count approvals (ctx + 24)
    mov     rax, [rdi + 24]
    lea     rsi, [rip + .Lfn_filter_approvers]
    call    count_where
    cmp     rax, 2
    jl      .Ldeny
    
.Lallow:
    mov     al, 1                     ; Return true
    pop     rbp
    ret
    
.Ldeny:
    xor     al, al                    ; Return false
    pop     rbp
    ret

.Lstr_deployment:
    .ascii "Deployment\0"

.Lstr_production:
    .ascii "production\0"

.Larr_risk_levels:
    .quad .Lstr_high
    .quad .Lstr_critical
```

## Step 6: Live Update (Zero Downtime)

DevOps team updates the policy to require 3 approvals:

```rust
policy RequireApproval:
  "Production deployments of high-risk services need 3+ approvals..."  // Changed
  
  // ... triggers unchanged ...
  
  requires
    approvals.count >= 3  // Changed from 2 to 3
```

Control plane handles atomic swap:

```rust
// Compile new version
let new_bytecode = compiler.compile(updated_policy_source)?;
let new_policy = TieredPolicy::new(new_bytecode, "RequireApproval");

// JIT compile immediately (critical policy)
jit_compiler.compile_sync(&new_policy)?;

// Atomic swap (lock-free)
policy_db.update("RequireApproval", new_policy)?;
// Old policy still serves in-flight requests
// New requests immediately use new policy

// Old policy cleaned up after grace period
tokio::time::sleep(Duration::from_secs(60)).await;
```

## Step 7: Observability

### Metrics

```rust
// Prometheus metrics
policy_evaluations_total{policy="RequireApproval",tier="jit"} 1234567
policy_eval_duration_microseconds{policy="RequireApproval",quantile="0.5"} 8.2
policy_eval_duration_microseconds{policy="RequireApproval",quantile="0.99"} 15.7
policy_jit_compilations_total{policy="RequireApproval"} 2
policy_cache_hits_total{policy="RequireApproval"} 1234500
```

### Tracing

```json
{
  "trace_id": "7a8c9d2e-...",
  "span_id": "1b3f4e5d-...",
  "operation": "policy_evaluation",
  "policy": "RequireApproval",
  "tier": "optimized_jit",
  "duration_us": 7.3,
  "decision": "deny",
  "matched_conditions": [
    "resource.type == Deployment",
    "environment == production",
    "risk_level in [high, critical]"
  ],
  "failed_conditions": [
    "approvals.count >= 3 (got 2)"
  ]
}
```

### Explain Mode

```bash
$ ipe explain --policy RequireApproval --context request.json

Policy: RequireApproval
Tier: Optimized JIT
Decision: DENY

Evaluation trace:
  ✓ resource.type == "Deployment" (8ns, cached)
  ✓ environment == "production" (12ns, cached)
  ✓ risk_level in ["high", "critical"] (15ns)
  ✗ approvals.count >= 3 (3.2μs, aggregate)
    → Got 2 approvals, need 3
    → Approvers: alice@eng (senior), bob@eng (senior)
    → Missing: 1 more from different department

Required action:
  Obtain approval from senior engineers in:
    - Product department: carol@product, dave@product
    - Operations department: eve@ops, frank@ops

Total evaluation time: 7.4μs (5.1x faster than interpreter)
```

## Performance Summary

| Phase | Evaluations | Mode | Avg Latency | Throughput |
|-------|-------------|------|-------------|------------|
| Cold start | 0-100 | Interpreter | 48μs | 20k/sec |
| Warming up | 100-1000 | JIT compiling | 48μs | 20k/sec |
| Hot path | 1000+ | Baseline JIT | 9μs | 110k/sec |
| Optimized | 10000+ | Optimized JIT | 7μs | 140k/sec |

**Total improvement:** 6.8x faster, 7x higher throughput

## Memory Usage

```
Policy storage:
  Source code:    482 bytes (.ipe file)
  Bytecode:       180 bytes (compiled)
  JIT code:       2.1 KB (baseline) | 3.8 KB (optimized)
  Total:          4.0 KB per policy
  
For 10,000 policies:
  Cold storage:   ~1.8 MB (bytecode only)
  Hot tier (64):  ~250 KB (JIT compiled)
  Total RSS:      ~4 MB (with overhead)
```

## Conclusion

This end-to-end flow demonstrates:

1. **Human-friendly** policy authoring with natural language
2. **Automatic optimization** via adaptive JIT compilation
3. **Zero-downtime updates** with atomic policy swapping
4. **Production-grade** observability and debugging
5. **Extreme performance** (<10μs) with minimal memory (<4KB/policy)

The entire system requires **zero configuration**—developers write policies, and the engine handles the rest.
