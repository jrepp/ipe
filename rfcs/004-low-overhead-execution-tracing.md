# RFC-004: Low-Overhead Execution Tracing

**Status:** 📝 Draft
**Created:** 2025-11-04
**Author:** IPE Contributors

## Summary

A low-overhead tracing system that captures full execution traces (request context, data context, and branch decisions) with minimal performance impact (<5% overhead). Uses pre-allocated flat buffers during hot-path execution and deferred expansion with symbol information and expensive runtime data.

## Motivation

### Current State
- No instrumentation on interpreter hot paths
- JIT compilation only logs success/failure
- No visibility into branch decisions during policy evaluation
- No ability to debug why a policy allowed/denied a request
- No performance profiling per-policy or per-predicate

### Desired State
- Full execution traces capturing:
  - Request context (principal, timestamp, source IP)
  - Data context (resource attributes, action, metadata)
  - Branch decisions (which predicates evaluated true/false)
  - Performance metrics (evaluation time per predicate)
- <5% performance overhead on hot paths (~2.5μs for 50μs interpreter, ~0.5μs for 10μs JIT)
- Zero-allocation fast path for JIT-compiled predicates
- Post-processing capability to enrich traces with symbols and metadata

### Use Cases

**1. Authorization Debugging**
```
Why was this request denied?
→ Trace shows: predicate #7 (resource.owner == principal.id) evaluated false
→ Context: resource.owner = "alice", principal.id = "bob"
```

**2. Performance Analysis**
```
Which predicates are slowest?
→ Trace shows: predicate #3 (database lookup) took 45μs of 50μs total
→ Optimization target identified
```

**3. Audit & Compliance**
```
Show all authorization decisions for user "alice" on 2025-11-03
→ Replay traces with full context and branch history
→ Generate compliance report
```

**4. Policy Optimization**
```
Which predicates are evaluated most frequently?
→ Trace aggregation shows: predicate #1 evaluated 10M times, #5 only 100 times
→ Reorder predicates to fail fast
```

## Design Goals

1. **Fast Path First:** Minimize overhead on JIT hot paths (<1% impact)
2. **Flat Buffer Design:** Pre-allocated, contiguous memory for cache efficiency
3. **Deferred Enrichment:** Expensive operations (symbol lookup, string formatting) happen post-evaluation
4. **Conditional Compilation:** Zero-cost when tracing disabled at compile time
5. **Thread-Safe:** Lock-free writes for multi-threaded evaluation

## Detailed Design

### Option 1: Ring Buffer with Fixed-Size Events (Recommended)

**Architecture:**
```
┌─────────────────────────────────────────────────────────────────┐
│                    Hot Path (During Evaluation)                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Thread-Local Ring Buffer                                       │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ [Event][Event][Event][Event][Event]...[Event]         │    │
│  │  ^head                              ^tail              │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  Event Structure (32 bytes, cache-line aligned):                │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ timestamp: u64        (8 bytes)                        │    │
│  │ event_type: u8        (1 byte)  - Branch/Enter/Exit    │    │
│  │ predicate_id: u16     (2 bytes) - Which predicate      │    │
│  │ result: u8            (1 byte)  - true/false/error     │    │
│  │ duration_ns: u32      (4 bytes) - Nano precision       │    │
│  │ context_hash: u64     (8 bytes) - Hash of context      │    │
│  │ metadata: u64         (8 bytes) - Free-form data       │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  Fast Path Operations:                                          │
│  • Push event:  1-2 CPU cycles (no allocation, no locks)       │
│  • Ring wrap:   Automatic overwrite of oldest events           │
│  • Cost:        ~0.5-1μs per event                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                Cold Path (Post-Evaluation Enrichment)           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Trace Enrichment (happens after eval() returns):               │
│  ┌────────────────────────────────────────────────────────┐    │
│  │ 1. Copy events from ring buffer to owned trace         │    │
│  │ 2. Resolve predicate_id → symbol names                 │    │
│  │ 3. Expand context_hash → full context values           │    │
│  │ 4. Format human-readable trace                         │    │
│  └────────────────────────────────────────────────────────┘    │
│                                                                  │
│  Enriched Trace:                                                │
│  {                                                               │
│    "policy_id": "allow-deploys-with-approval",                  │
│    "evaluation_time_us": 47,                                    │
│    "result": "allow",                                           │
│    "events": [                                                  │
│      {                                                          │
│        "type": "branch",                                        │
│        "predicate": "resource.type == 'deployment'",            │
│        "result": true,                                          │
│        "duration_ns": 120,                                      │
│        "context": {"resource.type": "deployment"}               │
│      },                                                         │
│      {                                                          │
│        "type": "branch",                                        │
│        "predicate": "has_approval(principal.id, resource.id)",  │
│        "result": true,                                          │
│        "duration_ns": 42000,  // Database lookup!               │
│        "context": {                                             │
│          "principal.id": "bot-123",                             │
│          "resource.id": "deploy-456"                            │
│        }                                                        │
│      }                                                          │
│    ]                                                            │
│  }                                                               │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Implementation:**

```rust
/// Thread-local trace buffer (zero allocation on hot path)
const RING_SIZE: usize = 1024; // 32KB per thread
const EVENT_SIZE: usize = 32;  // Cache-line aligned

thread_local! {
    static TRACE_RING: RefCell<RingBuffer> = RefCell::new(RingBuffer::new(RING_SIZE));
}

#[repr(C, align(32))]
#[derive(Copy, Clone)]
struct TraceEvent {
    timestamp: u64,      // TSC or CLOCK_MONOTONIC
    event_type: u8,      // Branch=1, Enter=2, Exit=3
    predicate_id: u16,   // Index into policy bytecode
    result: u8,          // 0=false, 1=true, 2=error
    duration_ns: u32,    // Per-predicate timing
    context_hash: u64,   // FxHash of relevant context fields
    metadata: u64,       // Free-form: PC, stack depth, etc.
}

/// Ring buffer with wrap-around
struct RingBuffer {
    events: Box<[TraceEvent; RING_SIZE]>,
    head: AtomicUsize,  // Next write position
    tail: AtomicUsize,  // Oldest event position
}

impl RingBuffer {
    #[inline(always)]
    fn push(&mut self, event: TraceEvent) {
        let head = self.head.load(Ordering::Relaxed);
        unsafe {
            // SAFETY: head is always < RING_SIZE
            *self.events.get_unchecked_mut(head) = event;
        }
        self.head.store((head + 1) % RING_SIZE, Ordering::Relaxed);
    }
}

/// Hot path instrumentation (interpreter)
#[inline(always)]
fn trace_branch(predicate_id: u16, result: bool, duration_ns: u32, ctx_hash: u64) {
    #[cfg(feature = "trace")]
    {
        TRACE_RING.with(|ring| {
            ring.borrow_mut().push(TraceEvent {
                timestamp: rdtsc(),  // Or monotonic clock
                event_type: 1,       // Branch
                predicate_id,
                result: result as u8,
                duration_ns,
                context_hash: ctx_hash,
                metadata: 0,
            });
        });
    }
}

/// Interpreter integration
impl Interpreter {
    fn evaluate(&mut self, policy: &Policy, ctx: &EvaluationContext) -> Result<bool> {
        let start = rdtsc();

        // Main interpreter loop
        loop {
            let instr = unsafe { policy.code.get_unchecked(pc) };
            match instr {
                Instruction::JumpIfFalse(offset) => {
                    let cond = self.stack_pop()?;
                    let result = cond.is_truthy();

                    // TRACE: Branch decision
                    trace_branch(
                        pc as u16,
                        result,
                        (rdtsc() - start) as u32,
                        hash_context(ctx, pc),
                    );

                    if !result {
                        pc += *offset as usize;
                    }
                }
                // ... other instructions
            }
        }
    }
}

/// JIT integration (even lower overhead)
impl JitCompiler {
    fn compile(&self, policy: &Policy) -> Result<JitFunction> {
        // In JIT code generation, emit:
        // - Load trace buffer pointer
        // - Store TraceEvent directly (no function call)
        // - Increment head pointer

        // Example Cranelift IR:
        // v1 = load.i64 thread_local_trace_buffer
        // v2 = load.i64 v1[head_offset]
        // store.i64 timestamp, v2[0]
        // store.i8 event_type, v2[8]
        // store.i16 predicate_id, v2[9]
        // ...
        // v3 = iadd v2, 32
        // store.i64 v3, v1[head_offset]

        // Cost: ~5-10 instructions = <1ns overhead
    }
}
```

**Trace Enrichment (Cold Path):**

```rust
/// Enriched trace with full context
pub struct EnrichedTrace {
    pub policy_id: String,
    pub evaluation_time_us: u64,
    pub result: bool,
    pub events: Vec<EnrichedEvent>,
}

pub struct EnrichedEvent {
    pub timestamp_us: u64,
    pub event_type: EventType,
    pub predicate_expr: String,  // Human-readable predicate
    pub result: bool,
    pub duration_ns: u32,
    pub context_values: HashMap<String, Value>,  // Relevant context
}

impl Trace {
    /// Enrich raw events with symbols and context
    pub fn enrich(&self, policy: &Policy, ctx: &EvaluationContext) -> EnrichedTrace {
        let mut events = Vec::with_capacity(self.raw_events.len());

        for event in &self.raw_events {
            // Resolve predicate ID to source expression
            let predicate_expr = policy.bytecode_map
                .get(event.predicate_id)
                .map(|bc| bc.source_expr.clone())
                .unwrap_or_else(|| format!("predicate_{}", event.predicate_id));

            // Expand context hash to full values
            let context_values = extract_relevant_context(ctx, event.context_hash);

            events.push(EnrichedEvent {
                timestamp_us: event.timestamp / 1000,
                event_type: event.event_type.into(),
                predicate_expr,
                result: event.result == 1,
                duration_ns: event.duration_ns,
                context_values,
            });
        }

        EnrichedTrace {
            policy_id: policy.id.clone(),
            evaluation_time_us: self.total_duration_ns / 1000,
            result: self.final_result,
            events,
        }
    }
}
```

**Performance Characteristics:**
- **Hot path overhead:** ~0.5-1μs per event (5-10 CPU cycles)
- **Memory:** 32KB per thread for ring buffer
- **Enrichment cost:** ~10-50μs (done after evaluation, off critical path)
- **Total overhead:** <5% for typical policies (10-20 predicates)

---

### Option 2: Hierarchical Span Tree

**Architecture:**
```
Span Tree (Nested Structure):
┌─────────────────────────────────────────────────┐
│ Root Span: policy.evaluate()                    │
│ ├─ Span: predicate[0] "resource.type == ..."   │
│ │  └─ Event: comparison                         │
│ ├─ Span: predicate[1] "has_approval(...)"      │
│ │  ├─ Span: database.lookup()                   │
│ │  │  └─ Event: rocksdb.get()                   │
│ │  └─ Event: result = true                      │
│ └─ Span: predicate[2] "principal.role ..."     │
│     └─ Event: comparison                         │
└─────────────────────────────────────────────────┘

Storage: Arena-allocated tree
Memory: Variable (32-256 bytes per span)
Overhead: ~2-5μs per span (allocation + bookkeeping)
```

**Implementation:**

```rust
use tracing::{span, event, Level};

impl Interpreter {
    fn evaluate(&mut self, policy: &Policy, ctx: &EvaluationContext) -> Result<bool> {
        let _span = span!(Level::DEBUG, "policy.evaluate",
            policy_id = %policy.id,
            principal = %ctx.request.principal.id,
        ).entered();

        loop {
            match instr {
                Instruction::JumpIfFalse(offset) => {
                    let _pred_span = span!(Level::TRACE, "predicate",
                        id = pc,
                        expr = ?policy.bytecode_map.get(pc),
                    ).entered();

                    let start = Instant::now();
                    let result = self.stack_pop()?.is_truthy();
                    let duration = start.elapsed();

                    event!(Level::TRACE, "branch",
                        result = result,
                        duration_ns = duration.as_nanos(),
                    );

                    if !result {
                        pc += *offset as usize;
                    }
                }
            }
        }
    }
}
```

**Pros:**
- Rich ecosystem (integration with OpenTelemetry, Jaeger)
- Structured logging with arbitrary metadata
- Built-in sampling and filtering

**Cons:**
- Higher overhead (~2-5μs per span vs <1μs per event)
- Heap allocations on span creation
- Complex dependency (tracing + subscriber + exporter)

---

### Option 3: Hybrid Approach

**Strategy:** Use flat buffers for hot paths, spans for cold paths

```rust
// Hot path: JIT-compiled predicates
#[inline(always)]
fn fast_trace(event: TraceEvent) {
    TRACE_RING.with(|ring| ring.borrow_mut().push(event));
}

// Cold path: Database lookups, external calls
fn slow_trace<F>(op: &str, f: F) -> Result<T>
where F: FnOnce() -> Result<T>
{
    let _span = span!(Level::DEBUG, "external_op", op = op).entered();
    let start = Instant::now();
    let result = f();
    let duration = start.elapsed();
    event!(Level::DEBUG, "completed", duration_ns = duration.as_nanos());
    result
}

// Usage:
match instr {
    // Fast path
    Instruction::JumpIfFalse(offset) => {
        let result = self.stack_pop()?.is_truthy();
        fast_trace(TraceEvent { ... });
    }

    // Slow path
    Instruction::HasApproval { .. } => {
        let result = slow_trace("has_approval", || {
            self.approval_store.lookup(...)
        })?;
    }
}
```

**Pros:**
- Best of both worlds: <1μs for hot paths, rich traces for slow paths
- Adaptive: traces are detailed where it matters (slow operations)

**Cons:**
- Two tracing systems to maintain
- Complexity in aggregating traces

---

## Implementation Phases

### Phase 1: Core Ring Buffer (Week 1)
- Implement `TraceEvent` and `RingBuffer`
- Add thread-local storage
- Benchmark overhead on synthetic workloads
- **Success metric:** <1μs per event, <5% total overhead

### Phase 2: Interpreter Integration (Week 1-2)
- Add `trace_branch()` calls to interpreter loop
- Instrument `JumpIfFalse`, `JumpIfTrue`, `LoadField`
- Add `#[cfg(feature = "trace")]` guards
- **Success metric:** Full traces for interpreter evaluations

### Phase 3: Trace Enrichment (Week 2)
- Implement `EnrichedTrace` and symbol resolution
- Add context expansion (hash → values)
- Build human-readable trace formatter
- **Success metric:** Debuggable traces with source expressions

### Phase 4: JIT Integration (Week 3)
- Emit tracing code in Cranelift IR
- Optimize for minimal instruction count
- Verify W^X compatibility
- **Success metric:** <0.5μs overhead for JIT paths

### Phase 5: Export & Tooling (Week 4)
- Add JSON export for traces
- Build CLI tool for trace analysis
- Implement aggregation (hot predicates, slow paths)
- Add sampling configuration
- **Success metric:** Production-ready observability

### Phase 6: Advanced Features (Future)
- OpenTelemetry integration
- Distributed tracing (span context propagation)
- Real-time trace streaming (SSE)
- ML-based anomaly detection

---

## Alternatives Considered

### A. Full `tracing` crate integration
**Rejected:** Too much overhead for JIT hot paths (2-5μs per span vs <1μs for flat buffer)

### B. DTrace/eBPF for kernel-level tracing
**Rejected:** Requires root privileges, Linux-only, complex tooling

### C. Sampling-based profiling
**Rejected:** Misses infrequent but important events (edge case bugs)

### D. Printf debugging
**Rejected:** Not structured, no post-processing, huge overhead

### E. Custom binary protocol
**Considered:** Very low overhead but reinvents the wheel. Flat buffer is simpler and sufficient.

---

## Success Metrics

### Performance
- [ ] <5% overhead on interpreter evaluation (50μs → <52.5μs)
- [ ] <1% overhead on JIT evaluation (10μs → <10.1μs)
- [ ] <1μs per trace event write
- [ ] <50μs trace enrichment time

### Functionality
- [ ] Captures all branch decisions in policy evaluation
- [ ] Resolves predicate IDs to source expressions
- [ ] Expands context hashes to full values
- [ ] Exports traces in JSON format

### Usability
- [ ] CLI tool can answer "why was this request denied?"
- [ ] Aggregation tool identifies top 10 slowest predicates
- [ ] Trace visualization (flame graph, timeline)

### Production Readiness
- [ ] Configurable sampling rate (1%, 10%, 100%)
- [ ] Ring buffer size configurable per deployment
- [ ] Graceful degradation if tracing overhead exceeds budget
- [ ] Zero-cost when `feature = "trace"` is disabled

---

## Open Questions

1. **Timestamp source:** TSC (fastest, ~5ns) vs `CLOCK_MONOTONIC` (~20ns)?
   - **Recommendation:** TSC for hot path, monotonic for enrichment

2. **Context hashing:** FxHash (fast) vs SipHash (secure)?
   - **Recommendation:** FxHash (not a security boundary)

3. **Storage backend:** In-memory only or persist to disk?
   - **Recommendation:** Start in-memory, add RocksDB export in Phase 5

4. **Trace retention:** How long to keep traces?
   - **Recommendation:** Configurable (default: last 1000 evaluations per thread)

5. **Privacy:** Should traces include PII (principal IDs, resource data)?
   - **Recommendation:** Make PII inclusion configurable, default to hashes only

---

## Integration with Existing Systems

### Tiering System (`tiering.rs`)
- Add `trace_promotion()` event when tier changes
- Track promotion reasons (eval count, latency threshold)

### Store (`store.rs`)
- Add `trace_policy_load()` on snapshot updates
- Track cache hit/miss rates

### JIT Compiler (`jit.rs`)
- Emit tracing prologue/epilogue in generated code
- Track compilation time and success rates

### Approval Store (`rar.rs`)
- Add `trace_approval_lookup()` for database queries
- Measure RocksDB latency distribution

---

## Security Considerations

1. **Trace data sensitivity:** Traces may contain PII (user IDs, resource names)
   - **Mitigation:** Hash sensitive fields, add redaction API

2. **DoS via trace overhead:** Attacker floods with evals to slow down via tracing
   - **Mitigation:** Sampling + adaptive tracing (disable if overhead > 5%)

3. **Trace data leakage:** Unauthorized access to trace files
   - **Mitigation:** Encrypt traces at rest, require auth for export

---

## References

- **Linux `perf`:** Low-overhead profiling via ring buffers
- **DTrace:** Hierarchical probes with minimal overhead
- **OpenTelemetry:** Span-based tracing standard
- **Cranelift instrumentation:** [cranelift-codegen/profiler](https://docs.rs/cranelift-codegen/latest/cranelift_codegen/profiler/)
- **High-performance tracing:** ["Fast Tracing for Online Services"](https://research.google/pubs/pub36356/) (Google Dapper paper)

---

**Next Steps:**
1. Review this RFC with the team
2. Benchmark synthetic workloads (empty traces vs full traces)
3. Prototype ring buffer implementation (Phase 1)
4. Validate <5% overhead on real policies
