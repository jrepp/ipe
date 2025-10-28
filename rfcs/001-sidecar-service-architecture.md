# RFC-001: Sidecar Service Architecture

**Status:** 📝 Draft
**Created:** 2025-10-27
**Author:** IPE Contributors

## Summary

A minimal Rust service (<50MB) that runs as a sidecar, providing sub-millisecond policy evaluation via separate data and control planes. Optimized for local communication and zero-downtime updates.

## Motivation

Modern workloads need policy evaluation that is:
- **Fast:** <1ms p99 latency via local sockets
- **Lightweight:** <50MB memory for embedded deployment
- **Co-located:** Minimal network overhead, no external calls
- **Hot-reloadable:** Update policies without restart
- **Simple:** Standard protocols, no code generation

## Architecture

### Service Model

```
┌─────────────────────────────────────────┐
│         IPE Sidecar Service             │
├─────────────────────────────────────────┤
│                                         │
│  ┌─────────────┐    ┌────────────────┐ │
│  │ Control     │    │  Data Plane    │ │
│  │ Plane       │    │  (Evaluation)  │ │
│  │             │    │                │ │
│  │ - Policy    │───▶│ - Query API    │ │
│  │   Updates   │    │ - Hot Read     │ │
│  │ - Data Sync │    │ - Zero-copy    │ │
│  │ - Metrics   │    │                │ │
│  └─────────────┘    └────────────────┘ │
│         │                    ▲          │
│         │                    │          │
└─────────┼────────────────────┼──────────┘
          │                    │
          │                    │
   Control Socket         Data Socket(s)
   (Unix/TCP)            (Unix/TCP/Multi)
          │                    │
          ▼                    ▼
   ┌──────────┐         ┌──────────┐
   │ Control  │         │ Workload │
   │ Service  │         │ Services │
   └──────────┘         └──────────┘
```

### Planes

#### Data Plane (Read-Only, High-Throughput)
- Lock-free reads via arc-swap
- Multiple listeners (unix + TCP)
- Zero coordination between requests
- Primary: `/var/run/ipe/eval.sock`

#### Control Plane (Write, Admin)
- GitOps-based policy synchronization (see RFC-004)
- Atomic updates with versioning
- Authentication required
- Audit logging enabled
- Primary: `/var/run/ipe/control.sock`

### Configuration

```toml
[service]
# Service identity
name = "ipe-sidecar"
instance_id = "pod-abc-123"

# Memory constraints
max_heap_mb = 50
policy_cache_mb = 10
data_cache_mb = 5

[data_plane]
# Listener configurations
[[data_plane.listeners]]
type = "unix"
path = "/var/run/ipe/eval.sock"
mode = 0o666

[[data_plane.listeners]]
type = "tcp"
bind = "127.0.0.1:9001"
max_connections = 1000

# Performance tuning
worker_threads = 2  # Limit threads for small footprint
max_concurrent_evals = 500

[control_plane]
# Control listener
type = "unix"
path = "/var/run/ipe/control.sock"
mode = 0o660

# Security
require_auth = true
allowed_clients = ["/usr/local/bin/ipe-ctl"]

# Update coordination
atomic_swap = true  # Use arc-swap for zero-downtime updates
validation_required = true

[storage]
# Policy store backend (memory for dev, rocksdb for prod)
policy_backend = "memory"  # or "rocksdb"
policy_path = "/var/lib/ipe/policies"

# Data store backend (memory for dev, rocksdb for prod)
data_backend = "memory"  # or "rocksdb"
data_path = "/var/lib/ipe/data"

# Persistence
persist_on_update = true
snapshot_interval = 300  # seconds

[observability]
metrics_enabled = true
metrics_path = "/var/run/ipe/metrics.sock"
trace_sampling_rate = 0.01  # 1% of requests
```

## Memory Budget

| Component | Budget | Strategy |
|-----------|--------|----------|
| Service binary | 5-10MB | Static linking |
| Policy store | 10-20MB | Bytecode + index |
| Data store | 5-10MB | Dynamic data |
| Eval context | 5MB | Object pooling |
| OS buffers | 10MB | Sockets, files |
| **Target** | **<50MB** | **Enforced limit** |

### Optimization Strategies
- Arena allocation for compilation
- Zero-copy bytecode interpretation
- Lazy policy loading
- LRU caching with bounded size

## Deployment Model

### Sidecar Pattern

```yaml
# Kubernetes example
apiVersion: v1
kind: Pod
metadata:
  name: app-with-policy
spec:
  containers:
  - name: app
    image: myapp:latest
    volumeMounts:
    - name: ipe-socket
      mountPath: /var/run/ipe

  - name: ipe-sidecar
    image: ipe:latest
    resources:
      limits:
        memory: 64Mi  # Enforced limit
        cpu: 100m
    volumeMounts:
    - name: ipe-socket
      mountPath: /var/run/ipe
    - name: ipe-config
      mountPath: /etc/ipe

  volumes:
  - name: ipe-socket
    emptyDir: {}
  - name: ipe-config
    configMap:
      name: ipe-config
```

### Process Model

```
Main Thread (Tokio Runtime)
├─ Data Plane Listener(s)
│  ├─ Worker 1: Process eval requests
│  └─ Worker 2: Process eval requests
│
├─ Control Plane Listener
│  └─ Single threaded: Process control commands
│
├─ Background Tasks
│  ├─ Policy sync (periodic)
│  ├─ Metrics collection
│  └─ Health checks
```

## Update Mechanism

**Policy updates** (via control plane):
1. Validate → Compile → Atomic swap → Async persist
2. Zero downtime using arc-swap
3. Version tracking via content hashes

**Data updates** (via control plane):
1. Validate schema → Update → Async persist
2. Used for dynamic data (approvals, etc.)

```rust
// Atomic policy swap - zero downtime
policy_store.swap(path, new_bytecode, new_hash, parent_hash);
```

## Security

- **Socket permissions:** Unix socket ACLs
- **Authentication:** Control plane only
- **Validation:** Pre-apply policy checks
- **Audit log:** All control operations
- **Resource limits:** Memory and connections enforced

## Implementation Phases

| Phase | Timeline | Deliverables |
|-------|----------|--------------|
| 1. Minimal Service | Weeks 1-2 | Tokio service, unix socket, in-memory store, basic eval |
| 2. Control Plane | Weeks 3-4 | Control API, atomic swap, versioning |
| 3. Multi-Listener | Week 5 | Multiple sockets, TCP support, pooling |
| 4. Storage | Week 6 | RocksDB/SQLite backends, persistence |
| 5. Production | Weeks 7-8 | Profiling, metrics, health checks |

## Alternatives Considered

| Approach | Rejected Because |
|----------|------------------|
| gRPC | Heavier deps, more complex than SSE |
| REST | No server-push, less efficient for streaming |
| Embedded lib | No process isolation, harder hot-reload |

## Success Metrics

- **Memory footprint**: <50MB baseline, <100MB under load
- **Latency**: <500μs p50, <1ms p99 for local evaluation
- **Throughput**: >10k evals/sec per core
- **Update latency**: <10ms for policy swap
- **Zero data loss**: During policy updates
- **Zero downtime**: During policy updates

## References

- [MCP Protocol](https://modelcontextprotocol.io/)
- [SSE Specification](https://html.spec.whatwg.org/multipage/server-sent-events.html)
- [arc-swap](https://docs.rs/arc-swap/) for lock-free reads
- [tokio](https://tokio.rs/) for async runtime
- [RFC-002: SSE/JSON Protocol](002-sse-json-protocol.md)
- [RFC-003: Policy Tree Storage](003-policy-tree-storage.md)
- [RFC-004: Control Plane Architecture](004-control-plane.md)
