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

## Trace Export Formats

To enable integration with existing observability infrastructure, IPE will support multiple trace export formats. The core ring buffer remains format-agnostic; export happens during the enrichment phase.

### JSON (Default)

**Format:** Human-readable, widely supported
**Use case:** CLI tools, debugging, log aggregation

```json
{
  "trace_version": "1.0",
  "policy_id": "allow-deploys-with-approval",
  "timestamp": "2025-11-05T14:23:45.123Z",
  "evaluation_time_us": 47,
  "result": "allow",
  "principal": {
    "id": "bot-123",
    "roles": ["deployer"]
  },
  "resource": {
    "type": "deployment",
    "id": "deploy-456"
  },
  "events": [
    {
      "type": "branch",
      "predicate": "resource.type == 'deployment'",
      "result": true,
      "duration_ns": 120,
      "context": {
        "resource.type": "deployment"
      }
    },
    {
      "type": "branch",
      "predicate": "has_approval(principal.id, resource.id)",
      "result": true,
      "duration_ns": 42000,
      "context": {
        "principal.id": "bot-123",
        "resource.id": "deploy-456"
      }
    }
  ]
}
```

### OpenTelemetry (Recommended for Production)

**Format:** [OTLP](https://opentelemetry.io/docs/specs/otlp/) (gRPC/HTTP)
**Use case:** Integration with Jaeger, Grafana Tempo, Datadog, New Relic

OpenTelemetry provides:
- Standardized span model with attributes
- Distributed tracing via trace context propagation
- Rich ecosystem (collectors, exporters, visualizers)
- Sampling strategies (head, tail, probabilistic)

**Implementation:**

```rust
use opentelemetry::{trace::Tracer, KeyValue};
use opentelemetry_otlp::WithExportConfig;

/// Export enriched trace as OpenTelemetry span
impl EnrichedTrace {
    pub fn export_otel(&self, tracer: &dyn Tracer) -> Result<()> {
        let mut span = tracer.start("policy.evaluation");

        // Add span attributes
        span.set_attribute(KeyValue::new("policy.id", self.policy_id.clone()));
        span.set_attribute(KeyValue::new("policy.result", self.result.to_string()));
        span.set_attribute(KeyValue::new("policy.duration_us", self.evaluation_time_us as i64));

        // Add events as span events (not child spans, for lower overhead)
        for event in &self.events {
            span.add_event(
                format!("predicate.{}", event.event_type),
                vec![
                    KeyValue::new("predicate.expr", event.predicate_expr.clone()),
                    KeyValue::new("predicate.result", event.result),
                    KeyValue::new("predicate.duration_ns", event.duration_ns as i64),
                ]
            );
        }

        span.end();
        Ok(())
    }
}

/// Configure OTLP exporter
pub fn setup_otel_exporter(endpoint: &str) -> Result<TracerProvider> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint)
        )
        .with_trace_config(
            trace::config()
                .with_resource(Resource::new(vec![
                    KeyValue::new("service.name", "ipe-policy-engine"),
                    KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ]))
        )
        .install_batch(opentelemetry::runtime::Tokio)
}
```

**Span Hierarchy:**

```
policy.evaluation (root span)
├─ event: predicate.branch (expr="resource.type == 'deployment'", result=true, duration_ns=120)
├─ event: predicate.branch (expr="has_approval(...)", result=true, duration_ns=42000)
└─ event: policy.result (result=allow, duration_us=47)
```

**Distributed Tracing Example:**

```rust
// Propagate trace context from incoming request
impl PolicyEngine {
    pub fn evaluate_with_trace_context(
        &self,
        policy: &Policy,
        ctx: &EvaluationContext,
        trace_parent: Option<&str>,  // W3C Trace Context header
    ) -> Result<bool> {
        // Extract parent span context
        let parent_cx = trace_parent
            .and_then(|tp| TraceContextPropagator::extract(tp))
            .unwrap_or_else(|| Context::current());

        // Create child span for policy evaluation
        let span = tracer.start_with_context("policy.evaluation", &parent_cx);
        let _guard = span.enter();

        // Evaluate policy (tracing happens automatically)
        self.evaluate(policy, ctx)
    }
}
```

**Benefits:**
- Out-of-the-box integration with monitoring platforms
- Distributed tracing across services (API gateway → IPE → database)
- Standardized sampling and tail-based sampling
- Low overhead (~50-100μs for export, batched in background)

### BSON (Binary JSON)

**Format:** Binary-encoded JSON
**Use case:** High-throughput log shipping, MongoDB integration

BSON provides:
- ~30% smaller than JSON
- Faster serialization (~2-3x)
- Native support in MongoDB

```rust
use bson::{doc, Bson};

impl EnrichedTrace {
    pub fn to_bson(&self) -> Result<bson::Document> {
        doc! {
            "trace_version": "1.0",
            "policy_id": &self.policy_id,
            "timestamp": bson::DateTime::now(),
            "evaluation_time_us": self.evaluation_time_us as i64,
            "result": self.result,
            "events": self.events.iter().map(|e| doc! {
                "type": e.event_type.to_string(),
                "predicate": &e.predicate_expr,
                "result": e.result,
                "duration_ns": e.duration_ns as i64,
                "context": bson::to_bson(&e.context_values)?
            }).collect::<Result<Vec<_>, _>>()?
        }
    }
}
```

**Performance:**
- JSON: ~500 bytes, ~15μs serialization
- BSON: ~350 bytes, ~5μs serialization

### MessagePack

**Format:** Binary serialization
**Use case:** Low-latency RPC, minimal overhead

```rust
use rmp_serde::encode;

impl EnrichedTrace {
    pub fn to_msgpack(&self) -> Result<Vec<u8>> {
        rmp_serde::to_vec(self)
    }
}
```

**Performance:**
- MessagePack: ~320 bytes, ~3μs serialization
- Fastest serialization format
- Good for high-throughput streaming

### Protobuf (for W3C Trace Context)

**Format:** Google Protocol Buffers
**Use case:** gRPC integration, cross-language compatibility

```protobuf
syntax = "proto3";

message TraceEvent {
  string policy_id = 1;
  uint64 evaluation_time_us = 2;
  bool result = 3;
  repeated PredicateEvent events = 4;
}

message PredicateEvent {
  string predicate_expr = 1;
  bool result = 2;
  uint32 duration_ns = 3;
  map<string, string> context = 4;
}
```

### Export Configuration

```rust
/// Trace export configuration
pub struct ExportConfig {
    /// Export format
    pub format: ExportFormat,
    /// Export destination
    pub destination: ExportDestination,
    /// Batch size (number of traces before flush)
    pub batch_size: usize,
    /// Flush interval (seconds)
    pub flush_interval_secs: u64,
    /// Compression (gzip, zstd, none)
    pub compression: Compression,
}

pub enum ExportFormat {
    Json,
    Bson,
    MessagePack,
    OpenTelemetry,
    Protobuf,
}

pub enum ExportDestination {
    File { path: PathBuf },
    Http { endpoint: String, headers: HashMap<String, String> },
    Grpc { endpoint: String },
    Kafka { brokers: Vec<String>, topic: String },
    RocksDB { path: PathBuf },
    Stdout,
    Stderr,
}
```

---

## Sampling Strategies

To control tracing overhead and storage costs, IPE implements adaptive sampling. Sampling decisions happen **before** trace enrichment to minimize wasted CPU cycles.

### Design Goals

1. **Low overhead by default:** Sample 1-5% of traces in production
2. **Intelligent sampling:** Always capture errors, slow evaluations
3. **Adaptive:** Increase sampling rate when system is idle, decrease under load
4. **Per-policy control:** Critical policies can have higher sampling rates

### Sampling Strategies

#### 1. Probabilistic Sampling (Default)

**Strategy:** Sample X% of all evaluations randomly

```rust
pub struct ProbabilisticSampler {
    /// Sampling rate (0.0 - 1.0)
    rate: f64,
    /// RNG for sampling decisions
    rng: SmallRng,
}

impl Sampler for ProbabilisticSampler {
    fn should_sample(&mut self, _ctx: &EvaluationContext) -> bool {
        self.rng.gen::<f64>() < self.rate
    }
}
```

**Configuration:**
- Default: 1% sampling (captures 1/100 evaluations)
- Low-traffic: 10% sampling
- High-traffic: 0.1% sampling

**Pros:** Simple, predictable overhead
**Cons:** May miss rare errors

#### 2. Rate-Limited Sampling

**Strategy:** Sample at most N traces per second

```rust
pub struct RateLimitedSampler {
    /// Maximum traces per second
    max_rate: u64,
    /// Current window start time
    window_start: Instant,
    /// Traces sampled in current window
    count: AtomicU64,
}

impl Sampler for RateLimitedSampler {
    fn should_sample(&mut self, _ctx: &EvaluationContext) -> bool {
        let now = Instant::now();
        if now.duration_since(self.window_start) > Duration::from_secs(1) {
            self.window_start = now;
            self.count.store(0, Ordering::Relaxed);
        }

        let current = self.count.fetch_add(1, Ordering::Relaxed);
        current < self.max_rate
    }
}
```

**Configuration:**
- Default: 100 traces/sec
- Burst protection: prevents tracing storms

**Pros:** Predictable storage costs, protects against trace storms
**Cons:** May miss tail latency issues

#### 3. Tail-Based Sampling (Intelligent)

**Strategy:** Always sample errors, slow evaluations, and rare events

```rust
pub struct TailBasedSampler {
    /// Base sampling rate
    base_rate: f64,
    /// Slow evaluation threshold (microseconds)
    slow_threshold_us: u64,
    /// Always sample errors
    sample_errors: bool,
    /// Always sample first N evaluations per policy
    sample_first_n: usize,
    /// Per-policy evaluation counters
    eval_counts: HashMap<String, AtomicUsize>,
}

impl Sampler for TailBasedSampler {
    fn should_sample(&mut self, ctx: &EvaluationContext) -> bool {
        // Always sample first N evaluations (cold start debugging)
        let policy_id = &ctx.policy_id;
        let count = self.eval_counts
            .entry(policy_id.clone())
            .or_insert_with(|| AtomicUsize::new(0))
            .fetch_add(1, Ordering::Relaxed);

        if count < self.sample_first_n {
            return true;
        }

        // Probabilistic sampling
        thread_rng().gen::<f64>() < self.base_rate
    }

    fn should_export(&self, trace: &RawTrace) -> bool {
        // Always export errors
        if self.sample_errors && trace.result.is_err() {
            return true;
        }

        // Always export slow evaluations
        if trace.duration_us > self.slow_threshold_us {
            return true;
        }

        // Otherwise, follow sampling decision
        trace.sampled
    }
}
```

**Configuration:**
- `base_rate`: 1% (probabilistic baseline)
- `slow_threshold_us`: 1000μs (sample if evaluation > 1ms)
- `sample_errors`: true (always capture denials, errors)
- `sample_first_n`: 10 (always sample first 10 evals per policy)

**Pros:** Captures important events automatically
**Cons:** More complex, requires post-evaluation decision

#### 4. Adaptive Sampling

**Strategy:** Adjust sampling rate based on system load

```rust
pub struct AdaptiveSampler {
    /// Target tracing overhead (% of CPU)
    target_overhead_pct: f64,
    /// Current sampling rate
    current_rate: AtomicU64,  // Fixed-point: rate * 10000
    /// Moving average of tracing cost
    avg_trace_cost_ns: AtomicU64,
    /// Moving average of evaluation time
    avg_eval_time_ns: AtomicU64,
}

impl AdaptiveSampler {
    fn adjust_rate(&self) {
        let trace_cost = self.avg_trace_cost_ns.load(Ordering::Relaxed);
        let eval_time = self.avg_eval_time_ns.load(Ordering::Relaxed);

        if eval_time == 0 {
            return;
        }

        // Current overhead = (trace_cost / eval_time) * sampling_rate
        let current_rate = self.current_rate.load(Ordering::Relaxed) as f64 / 10000.0;
        let current_overhead = (trace_cost as f64 / eval_time as f64) * current_rate;

        // Adjust rate to meet target overhead
        let target_rate = (self.target_overhead_pct / 100.0) / (trace_cost as f64 / eval_time as f64);
        let new_rate = (target_rate.clamp(0.001, 1.0) * 10000.0) as u64;

        self.current_rate.store(new_rate, Ordering::Relaxed);
    }
}
```

**Configuration:**
- `target_overhead_pct`: 5% (spend at most 5% of CPU on tracing)
- Adjusts rate every 10 seconds based on observed overhead

**Pros:** Self-tuning, maintains performance budget
**Cons:** Complex, may oscillate under load

#### 5. Per-Policy Sampling

**Strategy:** Configure sampling rates per policy

```rust
pub struct PerPolicySampler {
    /// Default sampling rate
    default_rate: f64,
    /// Per-policy overrides
    policy_rates: HashMap<String, f64>,
}

impl Sampler for PerPolicySampler {
    fn should_sample(&mut self, ctx: &EvaluationContext) -> bool {
        let rate = self.policy_rates
            .get(&ctx.policy_id)
            .copied()
            .unwrap_or(self.default_rate);

        thread_rng().gen::<f64>() < rate
    }
}
```

**Configuration:**
```yaml
sampling:
  default: 0.01  # 1%
  policies:
    critical-admin-policy: 1.0  # 100% (always trace)
    high-volume-api: 0.001      # 0.1% (minimal overhead)
```

**Pros:** Fine-grained control
**Cons:** Requires policy knowledge

### Sampling Configuration

```rust
/// Sampling configuration
pub struct SamplingConfig {
    /// Primary sampling strategy
    pub strategy: SamplingStrategy,
    /// Fallback to tail-based sampling for important events
    pub always_sample_errors: bool,
    pub always_sample_slow: bool,
    pub slow_threshold_us: u64,
}

pub enum SamplingStrategy {
    Always,
    Never,
    Probabilistic { rate: f64 },
    RateLimited { max_per_sec: u64 },
    TailBased { base_rate: f64, slow_threshold_us: u64 },
    Adaptive { target_overhead_pct: f64 },
    PerPolicy { default_rate: f64, overrides: HashMap<String, f64> },
}

impl Default for SamplingConfig {
    fn default() -> Self {
        SamplingConfig {
            strategy: SamplingStrategy::TailBased {
                base_rate: 0.01,  // 1% baseline
                slow_threshold_us: 1000,  // >1ms = slow
            },
            always_sample_errors: true,
            always_sample_slow: true,
            slow_threshold_us: 1000,
        }
    }
}
```

### Sampling Decision Flow

```
┌─────────────────────────────────────────┐
│  Policy Evaluation Starts               │
└──────────────┬──────────────────────────┘
               │
               ▼
┌─────────────────────────────────────────┐
│  Check Sampling Strategy                │
│  - Probabilistic?                       │
│  - Rate-limited?                        │
│  - Per-policy override?                 │
└──────────────┬──────────────────────────┘
               │
               ▼
       ┌───────┴────────┐
       │  Sample?       │
       └───┬────────┬───┘
    YES    │        │    NO
           ▼        ▼
    ┌──────────┐  ┌──────────────────┐
    │  Trace   │  │  Skip Tracing    │
    │  Enabled │  │  (zero overhead) │
    └─────┬────┘  └──────────────────┘
          │
          ▼
    ┌──────────────────────────┐
    │  Evaluation Completes    │
    └──────────┬───────────────┘
               │
               ▼
    ┌──────────────────────────┐
    │  Post-Evaluation Check   │
    │  - Error?                │
    │  - Slow?                 │
    └──────────┬───────────────┘
               │
               ▼
       ┌───────┴────────┐
       │  Keep Trace?   │
       └───┬────────┬───┘
    YES    │        │    NO
           ▼        ▼
    ┌──────────┐  ┌──────────┐
    │  Export  │  │  Discard │
    └──────────┘  └──────────┘
```

### Performance Impact of Sampling

| Sampling Rate | Overhead (Interpreter) | Overhead (JIT) | Storage (traces/day @ 10K rps) |
|---------------|------------------------|----------------|--------------------------------|
| 0% (disabled) | 0%                     | 0%             | 0                              |
| 0.1%          | <0.5%                  | <0.1%          | ~86K traces (~8GB)             |
| 1%            | <1%                    | <0.2%          | ~864K traces (~80GB)           |
| 10%           | ~3%                    | ~1%            | ~8.6M traces (~800GB)          |
| 100%          | ~5%                    | ~2%            | ~86M traces (~8TB)             |

**Recommendation:** Start with 1% tail-based sampling (captures errors + slow evals + 1% baseline)

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

### Phase 5: Sampling & Configuration (Week 4)
- Implement probabilistic and tail-based sampling
- Add sampling configuration (YAML/TOML)
- Benchmark overhead at different sampling rates
- Implement adaptive sampling
- **Success metric:** <1% overhead at 1% sampling rate

### Phase 6: Export Formats (Week 5)
- Add JSON export (default)
- Add BSON export for MongoDB integration
- Add MessagePack for high-throughput scenarios
- Build CLI tool for trace analysis and visualization
- **Success metric:** Export to multiple formats with <100μs overhead

### Phase 7: OpenTelemetry Integration (Week 6)
- Implement OTLP exporter (gRPC/HTTP)
- Add W3C Trace Context propagation
- Configure batching and compression
- Test integration with Jaeger/Grafana Tempo
- **Success metric:** End-to-end distributed tracing

### Phase 8: Advanced Features (Future)
- Real-time trace streaming (SSE/WebSocket)
- Trace aggregation and analytics
- ML-based anomaly detection (slow predicates, unusual patterns)
- Policy optimization recommendations based on trace data
- Kafka/Pulsar export for high-throughput scenarios

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
- [ ] Exports traces in multiple formats (JSON, BSON, MessagePack, OTLP)

### Sampling & Overhead
- [ ] Probabilistic sampling with configurable rate
- [ ] Tail-based sampling (errors + slow evaluations)
- [ ] Adaptive sampling based on system load
- [ ] Per-policy sampling rate overrides
- [ ] <1% overhead at 1% sampling rate
- [ ] Zero overhead when sampling disabled

### Integration
- [ ] OpenTelemetry OTLP export (gRPC/HTTP)
- [ ] W3C Trace Context propagation for distributed tracing
- [ ] Integration with Jaeger, Grafana Tempo, Datadog
- [ ] Export to file, HTTP, Kafka, RocksDB
- [ ] Batching and compression support

### Usability
- [ ] CLI tool can answer "why was this request denied?"
- [ ] Aggregation tool identifies top 10 slowest predicates
- [ ] Trace visualization (flame graph, timeline)
- [ ] Human-readable trace formatting

### Production Readiness
- [ ] Configurable sampling strategies (probabilistic, tail-based, adaptive)
- [ ] Ring buffer size configurable per deployment
- [ ] Graceful degradation if tracing overhead exceeds budget
- [ ] Zero-cost when `feature = "trace"` is disabled
- [ ] PII redaction for sensitive fields
- [ ] Trace encryption at rest

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

6. **Export format priority:** Which format should be the default?
   - **Recommendation:** JSON for development, OpenTelemetry for production

7. **Sampling defaults:** What should the default sampling rate be?
   - **Recommendation:** 1% tail-based sampling (errors + slow + 1% baseline)

---

## Recommended Configuration

### Development Environment

```yaml
tracing:
  enabled: true
  sampling:
    strategy: always  # Capture all traces for debugging
  export:
    format: json
    destination: stdout
    pretty_print: true
  ring_buffer_size: 1024
```

### Staging Environment

```yaml
tracing:
  enabled: true
  sampling:
    strategy: tail_based
    base_rate: 0.10  # 10% baseline
    slow_threshold_us: 1000
    always_sample_errors: true
    always_sample_slow: true
  export:
    format: opentelemetry
    destination:
      otlp_endpoint: "http://jaeger:4317"
    batch_size: 100
    flush_interval_secs: 5
  ring_buffer_size: 2048
```

### Production Environment

```yaml
tracing:
  enabled: true
  sampling:
    strategy: tail_based
    base_rate: 0.01  # 1% baseline
    slow_threshold_us: 1000
    always_sample_errors: true
    always_sample_slow: true
  export:
    format: opentelemetry
    destination:
      otlp_endpoint: "https://otel-collector.prod.internal:4317"
    batch_size: 1000
    flush_interval_secs: 10
    compression: zstd
  ring_buffer_size: 4096
  per_policy_sampling:
    critical-admin-access: 1.0  # Always trace admin access
    high-volume-api-check: 0.001  # Minimal sampling for high-volume
```

### High-Performance Production (Minimal Overhead)

```yaml
tracing:
  enabled: true
  sampling:
    strategy: adaptive
    target_overhead_pct: 2.0  # Max 2% CPU for tracing
  export:
    format: messagepack
    destination:
      kafka:
        brokers: ["kafka-1:9092", "kafka-2:9092", "kafka-3:9092"]
        topic: "ipe-traces"
    batch_size: 5000
    flush_interval_secs: 30
    compression: zstd
  ring_buffer_size: 8192
```

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

### Tracing Systems
- **Linux `perf`:** Low-overhead profiling via ring buffers
- **DTrace:** Hierarchical probes with minimal overhead
- **eBPF tracing:** Kernel-level observability with minimal overhead
- **High-performance tracing:** ["Dapper: Large-Scale Distributed Systems Tracing"](https://research.google/pubs/pub36356/) (Google)

### OpenTelemetry
- **OpenTelemetry Specification:** [https://opentelemetry.io/docs/specs/](https://opentelemetry.io/docs/specs/)
- **OTLP Protocol:** [https://opentelemetry.io/docs/specs/otlp/](https://opentelemetry.io/docs/specs/otlp/)
- **W3C Trace Context:** [https://www.w3.org/TR/trace-context/](https://www.w3.org/TR/trace-context/)
- **OpenTelemetry Rust SDK:** [https://docs.rs/opentelemetry/](https://docs.rs/opentelemetry/)

### Sampling Strategies
- **Tail-based sampling:** ["Canopy: End-to-End Performance Tracing At Scale"](https://research.fb.com/publications/canopy-end-to-end-performance-tracing-at-scale/) (Meta)
- **Adaptive sampling:** ["Adaptive Sampling for Performance Debugging"](https://dl.acm.org/doi/10.1145/3302424.3303976)
- **Probabilistic sampling:** OpenTelemetry specification

### Export Formats
- **BSON Specification:** [https://bsonspec.org/](https://bsonspec.org/)
- **MessagePack:** [https://msgpack.org/](https://msgpack.org/)
- **Protocol Buffers:** [https://protobuf.dev/](https://protobuf.dev/)

### Implementation
- **Cranelift instrumentation:** [cranelift-codegen/profiler](https://docs.rs/cranelift-codegen/latest/cranelift_codegen/profiler/)
- **Rust ring buffers:** [https://docs.rs/ringbuf/](https://docs.rs/ringbuf/)
- **Lock-free data structures:** [https://docs.rs/crossbeam/](https://docs.rs/crossbeam/)

---

**Next Steps:**
1. Review this RFC with the team
2. Benchmark synthetic workloads (empty traces vs full traces)
3. Prototype ring buffer implementation (Phase 1)
4. Validate <5% overhead on real policies
