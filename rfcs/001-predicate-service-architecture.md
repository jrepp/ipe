# RFC-001: Predicate Service Architecture

**Status:** 📝 Draft
**Created:** 2025-10-27
**Author:** IPE Contributors

## Summary

A minimal Rust service (<50MB) that runs as a predicate service, providing sub-millisecond policy evaluation via three distinct planes: Control (policy management), Data (privileged data writes), and Evaluation (application queries). Each plane has separate privilege boundaries and authorization models.

## Motivation

Modern workloads need policy evaluation that is:
- **Fast:** <1ms p99 latency via local sockets
- **Lightweight:** <50MB memory for embedded deployment
- **Co-located:** Minimal network overhead, no external calls
- **Hot-reloadable:** Update policies without restart
- **Simple:** Standard protocols, no code generation

## Architecture

### Three-Plane Architecture

```
                    ┌────────────────────────────────────────┐
                    │     IPE Predicate Service              │
                    ├────────────────────────────────────────┤
                    │                                        │
   CONTROL PLANE    │  ┌──────────────────────────────────┐ │  Privilege: HIGHEST
   (Admin)          │  │  Control Plane                   │ │  Auth: Admin tokens
   ─────────────────┼─▶│  - Policy management             │ │  Who: Security admins
                    │  │  - GitOps sync                   │ │
                    │  │  - Policy distribution           │ │
                    │  └──────────────────────────────────┘ │
                    │                │                       │
                    │                ▼                       │
   DATA PLANE       │  ┌──────────────────────────────────┐ │  Privilege: PRIVILEGED
   (Services)       │  │  Data Plane                      │ │  Auth: Service tokens
   ─────────────────┼─▶│  - Dynamic data writes           │ │  Who: Services
                    │  │  - Approvals, relationships      │ │
                    │  │  - Message plane distribution    │ │
                    │  └──────────────────────────────────┘ │
                    │                │                       │
                    │                ▼                       │
   EVALUATION PLANE │  ┌──────────────────────────────────┐ │  Privilege: APPLICATION
   (Applications)   │  │  Evaluation Plane                │ │  Auth: App tokens
   ◀────────────────┼──│  - Predicate evaluation          │ │  Who: Application code
                    │  │  - Feature flag queries          │ │
                    │  │  - Authorization checks          │ │
                    │  │  - Read-only access              │ │
                    │  └──────────────────────────────────┘ │
                    │                                        │
                    └────────────────────────────────────────┘

     /control.sock            /data.sock              /eval.sock
          │                       │                       │
          ▼                       ▼                       ▼
    ┌──────────┐           ┌──────────┐           ┌──────────┐
    │  Admin   │           │ Services │           │   Apps   │
    │  Tools   │           │ (writes) │           │ (queries)│
    └──────────┘           └──────────┘           └──────────┘
```

### Plane Definitions

#### 1. Control Plane (HIGHEST Privilege)
**Purpose:** Policy authoring, management, and distribution

**Operations:**
- Sync policies from Git (RFC-004)
- Update policy trees
- Manage `_features` policies
- Registry management (list instances, stats)

**Authorization:**
- Admin-level credentials (mTLS, admin tokens)
- Restricted to policy authors and security admins
- Full audit logging

**Socket:** `/var/run/ipe/control.sock`

#### 2. Data Plane (PRIVILEGED Writes)
**Purpose:** Dynamic data that policies evaluate against

**Operations:**
- Write approvals, permissions, relationships
- Update rate limit counters
- Manage dynamic policy context data
- Data replication via message plane

**Authorization:**
- Service-level tokens
- Specific services authorized for specific namespaces
- Write-only (predicates read from local cache)

**Socket:** `/var/run/ipe/data.sock`

#### 3. Evaluation Plane (APPLICATION Level)
**Purpose:** Application-facing predicate evaluation

**Operations:**
- Evaluate predicates
- Query feature flags
- Authorization checks
- Read-only policy queries

**Authorization:**
- Application-level tokens
- Read-only access
- Rate limited per client

**Socket:** `/var/run/ipe/eval.sock`

### Configuration

```toml
[service]
# Service identity
name = "ipe-predicate"
instance_id = "pod-abc-123"

# Memory constraints
max_heap_mb = 50
policy_cache_mb = 10
data_cache_mb = 5

[evaluation_plane]
# Application-facing evaluation requests (LOWEST privilege)
[[evaluation_plane.listeners]]
type = "unix"
path = "/var/run/ipe/eval.sock"
mode = 0o666  # Readable by applications

[[evaluation_plane.listeners]]
type = "tcp"
bind = "127.0.0.1:9001"
max_connections = 1000

# Performance tuning
worker_threads = 2
max_concurrent_evals = 500

# Authorization
require_auth = true
auth_type = "app_token"

[data_plane]
# Service writes of dynamic data (PRIVILEGED)
type = "unix"
path = "/var/run/ipe/data.sock"
mode = 0o660  # Restricted to services

# Authorization
require_auth = true
auth_type = "service_token"
namespace_auth = [
  {namespace = "approvals.*", services = ["deployment-service"]},
  {namespace = "relationships.*", services = ["identity-service"]},
]

# Replication
message_plane = "pubsub"
pubsub_url = "nats://localhost:4222"

[control_plane]
# Admin policy management (HIGHEST privilege)
type = "unix"
path = "/var/run/ipe/control.sock"
mode = 0o660  # Restricted to admins

# Security
require_auth = true
auth_type = "admin_token"
allowed_clients = ["/usr/local/bin/ipe-ctl"]

# Update coordination
atomic_swap = true
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

### Co-Location Pattern

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

  - name: ipe-predicate
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
- [RFC-005: Data Plane Architecture](005-data-plane.md)
