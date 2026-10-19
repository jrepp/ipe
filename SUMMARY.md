# Idempotent Predicate Engine: Executive Summary

> **Superseded.** This vision was written for the legacy engine (bytecode,
> interpreter, adaptive JIT tiering), which was removed after the version-2
> runtime landed. Kept for historical context. The shipping product is the
> v2 runtime: see [README.md](README.md) and
> [docs/RUNTIME_DATA.md](docs/RUNTIME_DATA.md).

## Vision

A next-generation predicate engine that combines human readability, AI integration, and extreme performance through runtime bytecode-to-native compilation.

## Problem Statement

Current predicate systems force a choice between:
- **Readable** (YAML, natural language) → Slow, unpredictable
- **Fast** (compiled, optimized) → Complex, developer-hostile

IPE eliminates this trade-off.

## Solution

### Three-Layer Architecture

```
┌─────────────────────────────────────────────┐
│  1. Natural Language Intent                 │  ← Human/AI Interface
│     "Deployments need 2 approvals"          │     Reviews, diffs, docs
└─────────────────┬───────────────────────────┘
                  │ Compile
┌─────────────────▼───────────────────────────┐
│  2. Semantic AST + Bytecode                 │  ← Portable, queryable
│     Type-checked, optimized IR              │     Cross-platform
└─────────────────┬───────────────────────────┘
                  │ Runtime JIT
┌─────────────────▼───────────────────────────┐
│  3. Native Machine Code                     │  ← Maximum performance
│     x86_64, ARM64, WASM                     │     <10μs evaluation
└─────────────────────────────────────────────┘
```

### Key Innovation: Adaptive JIT Compilation

Unlike traditional interpreters, IPE uses **runtime profiling** to automatically optimize hot predicates:

1. **Cold predicates** (rarely evaluated): Interpreted at ~50μs
2. **Warm predicates** (>100 evals): Baseline JIT at ~10μs (5x faster)
3. **Hot predicates** (>10k evals): Optimized JIT at ~5μs (10x faster)

This happens **transparently** with zero configuration.

## Technical Highlights

### Performance

| Metric | Value | Notes |
|--------|-------|-------|
| Predicate evaluation (JIT) | <10μs | 10x faster than OPA |
| Memory per predicate | 200 bytes | 50x smaller than Rego |
| Binary size | <2MB | Embedded-friendly |
| JIT compilation | <1ms | Faster than V8's TurboFan |
| Throughput | 100k ops/sec | Single-threaded |

### Embeddability

```
Rust Core (ipe-core)
    │
    ├─→ Native Lib (C FFI)
    │       ├─→ Python (PyO3)
    │       ├─→ Node.js (napi-rs)
    │       └─→ Go (cgo)
    │
    └─→ WebAssembly
            ├─→ Browser
            └─→ Server (wasmtime)
```

### Operational Excellence

- **Atomic updates:** Zero-downtime predicate reloads via Arc-swap
- **gRPC control plane:** Versioned deployments, rollbacks
- **Observability:** Metrics, traces, explain mode
- **Testing:** Inline predicate tests, sample data validation

## Differentiators

### vs. OPA/Rego

| Feature | OPA/Rego | IPE |
|---------|----------|-----|
| Language | Datalog-like | SQL/Go-like |
| Eval mode | Interpreter | Interpreter + JIT |
| Latency | ~500μs | <10μs (JIT) |
| Memory | ~10KB/predicate | 200 bytes/predicate |
| AI integration | Afterthought | Native |

### vs. Cedar

| Feature | Cedar | IPE |
|---------|-------|-----|
| Scope | AWS-centric | General purpose |
| Performance | Compiled | Compiled + JIT |
| Visual tools | None | Built-in web UI |
| Natural language | No | Yes (intent strings) |

### vs. Custom DSLs

| Feature | Custom DSL | IPE |
|---------|------------|-----|
| Tooling | DIY | Batteries-included |
| AI support | Manual | Automatic |
| Performance | Varies | Guaranteed <100μs |
| Bindings | Language-specific | Universal (FFI/WASM) |

## Use Cases

### DevOps

```rust
predicate DeploymentFreeze:
  "Block production deployments during code freeze"

  triggers when
    resource.type == "Deployment"
    and environment == "production"
    and current_time in freeze_windows

  denies with reason "Code freeze in effect"
```

### SecOps

```rust
predicate RequireMFA:
  "High-privilege actions need MFA verification"

  triggers when
    action.privilege_level >= "admin"
    and not request.mfa_verified

  denies with reason "MFA required for admin actions"
```

### Compliance

```rust
predicate AuditHighRisk:
  "Log all high-risk resource modifications"

  triggers when
    action.operation in ["Update", "Delete"]
    and resource.risk_level == "high"

  requires
    audit_log.enabled
    and notification.sent_to("security-team")
```

## AI Integration

### Natural Language → Predicate

```
User: "We need to require two approvals for production deployments"

AI generates:
predicate ProductionApprovals:
  "Production deployments require two approvals"

  triggers when
    resource.type == "Deployment"
    and environment == "production"

  requires
    approvals.count >= 2
```

### Semantic Queries

```sql
-- Find all predicates affecting deployments
search predicates where affects resource:Deployment

-- Explain why a request was denied
explain decision for deployment:api-v2
  with context { environment: "prod" }

-- Detect conflicts
find conflicts between predicate:RequireApproval
  and predicate:FastTrackHotfix
```

### Explainability

```
Decision: DENIED

Matched policies:
  ✓ RequireApproval (severity: critical)
    - Condition met: environment == "production"
    - Condition met: resource.type == "Deployment"
    - Condition failed: approvals.count >= 2
      → Got 1, need 2
      → Missing approval from: senior-engineers

Action required:
  Obtain approval from: @bob, @charlie, @diana
```

## Implementation Status

The statuses below are historical (legacy-engine era). The legacy bytecode
instruction set, JIT integration, adaptive tiering, and stack interpreter were
removed with that engine. The v2 runtime (runtime facts, policy documents,
language-2 evaluation, HTTP service) is implemented; see the README for what
ships today.

### Historical (legacy engine, removed)
- ✅ RFC and architecture design
- ✅ Bytecode instruction set
- ✅ JIT compiler integration (Cranelift)
- ✅ Adaptive tiering logic
- ✅ RAR (Resource-Action-Request) model
- ✅ Workspace structure
- 🚧 Language parser (nom)
- 🚧 Bytecode compiler
- 🚧 Predicate indexing
- 🚧 Basic interpreter

### Current (v2 runtime)
- ✅ Scoped HTTP runtime service (`ipe-server`)
- ✅ Runtime facts with mandatory TTL; push and pull ingestion
- ✅ Language-2 evaluation with budgets and evidence deadlines
- ✅ Policy documents with immutable revisions and provenance
- 📋 Production hardening, control plane, language bindings

## Next Steps

1. **Approve RFC** - Review and finalize architecture
2. **Phase 1 kickoff** - Begin core engine implementation
3. **Set up infrastructure** - CI/CD, benchmarks, docs
4. **Build MVP** - Working interpreter + basic JIT
5. **Alpha release** - Internal testing and feedback

## Success Metrics

### Technical
- <10μs p99 latency for predicate evaluation
- <2MB binary size
- 100k ops/sec throughput
- Zero CVEs in first year

### Adoption
- 10+ organizations in production
- 50+ GitHub stars in first quarter
- Active community contributions
- Integration with major cloud platforms

## Investment Required

### Engineering
- 2 FTE for 6 months (core engine)
- 1 FTE for 3 months (web application)
- 1 FTE ongoing (maintenance, community)

### Infrastructure
- CI/CD (GitHub Actions): $100/month
- Benchmarking cluster: $500/month
- Documentation hosting: $50/month

### Total
- ~$300k for first year (labor + infrastructure)
- ~$150k/year ongoing

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| JIT complexity | Fallback to interpreter, incremental rollout |
| Performance targets | Extensive benchmarking, early prototyping |
| Security vulnerabilities | Fuzzing, audits, memory safety (Rust) |
| Adoption barriers | Excellent docs, tutorials, migration tools |

## Conclusion

Idempotent Predicate Engine represents a paradigm shift in predicate management:

- **Human-friendly** syntax with natural language intent
- **AI-native** architecture for generation and queries
- **Extreme performance** via adaptive JIT compilation
- **Production-ready** operational features

This isn't just another predicate engine—it's the foundation for the next generation of security and compliance automation.

---

**Ready to build?** See [RFC.md](RFC.md) for technical details.

**Questions?** Contact the IPE team.
