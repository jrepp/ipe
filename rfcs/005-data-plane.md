# RFC-005: Data Plane Architecture

**Status:** 📝 Draft
**Created:** 2025-10-27
**Author:** IPE Contributors
**Depends On:** RFC-001, RFC-003

## Summary

The data plane provides high-availability, partition-tolerant storage for dynamic data that predicates evaluate against. This is the **middle privilege layer** - services write privileged data (approvals, relationships, permissions) that affects predicate execution. Separate from the Control Plane (highest privilege, policy management) and Evaluation Plane (lowest privilege, application queries).

## Motivation

The IPE architecture has **three privilege layers**:

1. **Control Plane** (HIGHEST) - Admins manage policies via GitOps
2. **Data Plane** (PRIVILEGED) - Services write data that policies evaluate
3. **Evaluation Plane** (APPLICATION) - Apps query predicates for decisions

The Data Plane needs:
- **High Availability:** Services write data directly, must tolerate failures
- **Partition Tolerance:** Continue operating during network partitions
- **High Write Throughput:** Optimized for frequent updates (permissions, relationships)
- **Low Read Latency:** Predicate evaluations need fast local access
- **Service-Level Authorization:** Services write privileged data with service tokens
- **Eventual Consistency:** AP not CP - availability over strong consistency
- **Logical Separation:** Different from evaluation queries and control operations

## Key Features

✅ Separate data plane from control plane (different authorization)
✅ RocksDB storage engine (same as policies, different instance)
✅ Message plane for data distribution
✅ High write throughput with eventual consistency
✅ Direct service writes (no control plane bottleneck)
✅ Predicate services cache data locally
✅ Optimistic locking for conflict resolution

## Three-Plane Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                      IPE System                                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  CONTROL PLANE (CP - Strong Consistency)          [HIGHEST]    │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  • Admins write policies via GitOps                       │ │
│  │  • Strong consistency required                            │ │
│  │  • Admin tokens (mTLS)                                    │ │
│  │  • /control.sock                                          │ │
│  └───────────────────────────────────────────────────────────┘ │
│                          │                                      │
│                          ▼ (policies)                           │
│  DATA PLANE (AP - Eventual Consistency)          [PRIVILEGED]  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  • Services write approvals, relationships, permissions   │ │
│  │  • Eventual consistency, high availability                │ │
│  │  • Service tokens (namespace-scoped)                      │ │
│  │  • /data.sock (write) → message plane → replicate        │ │
│  └───────────────────────────────────────────────────────────┘ │
│                          │                                      │
│                          ▼ (data)                               │
│  EVALUATION PLANE (Read-Only)                   [APPLICATION]  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  • Applications query predicates for decisions            │ │
│  │  • Reads from local cache (policies + data)              │ │
│  │  • Application tokens                                     │ │
│  │  • /eval.sock (read-only)                                │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## CAP Theorem Trade-offs

| Plane | CAP | Justification |
|-------|-----|---------------|
| **Control Plane** | CP | Strong consistency needed for policy updates |
| **Data Plane** | AP | Availability and partition tolerance for high write loads |
| **Evaluation Plane** | AP | Read-only from local cache, eventual consistency acceptable |

**Why AP for Data Plane:**
- Services need to write permissions/relationships even during network issues
- Predicate evaluations can tolerate slightly stale data
- High write volume requires eventual consistency
- Authorization failures are worse than stale data
- Logical separation from evaluation queries (different sockets, auth)

## Architecture

### Data Plane Components

```
┌─────────────────────────────────────────────────────────┐
│                    Message Plane                        │
│              (Data Distribution Layer)                  │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  ┌──────────────┐      ┌─────────────────┐            │
│  │  Data Store  │      │  Replication    │            │
│  │  (RocksDB)   │◀────▶│  Manager        │            │
│  │              │      │                 │            │
│  │ - KV pairs   │      │ - Pub/Sub       │            │
│  │ - Versioning │      │ - Gossip        │            │
│  │ - Tombstones │      │ - Merkle trees  │            │
│  └──────────────┘      └─────────────────┘            │
│         ▲                       │                       │
│         │                       │                       │
│  ┌──────┴───────────────────────▼────────────────────┐ │
│  │         Data Plane API (RFC-002)                  │ │
│  │                                                    │ │
│  │  - put (write data)                               │ │
│  │  - get (read data)                                │ │
│  │  - delete (tombstone)                             │ │
│  │  - subscribe (watch for changes)                  │ │
│  └────────────────────────────────────────────────────┘ │
│                    │                                    │
└────────────────────┼────────────────────────────────────┘
                     │
                     ▼
              Data Socket
         /var/run/ipe/data.sock
```

### Separation of Concerns

```
Control Plane          Data Plane              Predicate Service
(Policies)             (Dynamic Data)          (Evaluation)
     │                      │                         │
     │ sync policies        │                         │
     ├─────────────────────────────────────────────▶ │
     │                      │                         │
     │                      │ write permissions       │
     │                      ◀─────────────────────────┤
     │                      │                         │
     │                      │ subscribe to changes    │
     │                      ├────────────────────────▶│
     │                      │                         │
     │                      │ replicate data          │
     │                      ◀────────────────────────▶│
     │                      │ (message plane)         │
     │                      │                         │
     │                      │                         │
     │                      │    evaluate predicate   │
     │                      │    (read local data)    │
     │                      │         ◀───────────────┤
```

## Data Storage Model

### Data Structure

```rust
struct DataEntry {
    key: String,             // Namespaced key (e.g., "approvals.deploy-123")
    value: Vec<u8>,          // Arbitrary data (JSON, MessagePack, etc.)
    version: u64,            // Vector clock or timestamp
    created_at: u64,         // Unix timestamp
    updated_at: u64,         // Unix timestamp
    ttl: Option<u64>,        // Optional expiration
    tombstone: bool,         // Soft delete marker
}
```

### Storage Backend

**RocksDB Configuration:**
```
# Separate RocksDB instance from policies
data_store/
├── live/          # Active key-value data
├── versions/      # Version history (optional)
└── metadata/      # Store metadata (version vectors, etc.)
```

**Optimizations:**
- Write-optimized LSM tree configuration
- Batch writes for high throughput
- Bloom filters for fast key lookups
- Compaction tuned for write-heavy workload
- Optional TTL-based expiration

## Message Plane

The message plane handles data distribution between predicate services.

### Distribution Strategies

**1. Pub/Sub (Recommended)**
```
Service A writes → Data Plane → Pub/Sub → Predicate Services subscribe
```
- Central message broker (Redis, NATS, Kafka)
- Topic-based filtering (e.g., `data.approvals.*`)
- At-least-once delivery
- Predicate services subscribe to relevant topics

**2. Gossip Protocol (Alternative)**
```
Service A writes → Local Data Plane → Gossip to peers
```
- Epidemic-style propagation
- Eventually consistent
- No central broker needed
- Good for high partition tolerance

**3. Hybrid (Production)**
```
Service writes → Local Data Plane → Pub/Sub (fast path)
                                  → Gossip (slow path, fallback)
```
- Pub/Sub for normal operation
- Gossip as fallback during partitions
- Best of both worlds

### Replication Protocol

```json
// Data update message
{
  "type": "data.update",
  "key": "approvals.deploy-123",
  "value": {"approvers": ["alice", "bob"]},
  "version": 42,
  "timestamp": 1698765432,
  "source_instance": "ipe-abc-123"
}

// Data delete message
{
  "type": "data.delete",
  "key": "approvals.deploy-123",
  "version": 43,
  "timestamp": 1698765500,
  "tombstone": true
}
```

## Data Plane API

### Write Operations

```json
// Put data (idempotent)
event: put
data: {
  "method": "put",
  "params": {
    "key": "approvals.deploy-123",
    "value": {"approvers": ["alice", "bob"], "required": 2},
    "ttl": 3600  // Optional: expire in 1 hour
  }
}

// Response
data: {
  "result": {
    "key": "approvals.deploy-123",
    "version": 42,
    "updated_at": 1698765432
  }
}

// Batch put (high throughput)
event: batch-put
data: {
  "method": "batch-put",
  "params": {
    "entries": [
      {"key": "approvals.deploy-123", "value": {...}},
      {"key": "approvals.deploy-456", "value": {...}}
    ]
  }
}
```

### Read Operations

```json
// Get data
event: get
data: {
  "method": "get",
  "params": {"key": "approvals.deploy-123"}
}

// Response
data: {
  "result": {
    "key": "approvals.deploy-123",
    "value": {"approvers": ["alice", "bob"], "required": 2},
    "version": 42,
    "updated_at": 1698765432
  }
}

// Batch get
event: batch-get
data: {
  "method": "batch-get",
  "params": {"keys": ["approvals.deploy-123", "approvals.deploy-456"]}
}
```

### Delete Operations

```json
// Delete (tombstone)
event: delete
data: {
  "method": "delete",
  "params": {"key": "approvals.deploy-123"}
}

// Response
data: {
  "result": {
    "key": "approvals.deploy-123",
    "version": 43,
    "tombstone": true,
    "deleted_at": 1698765500
  }
}
```

### Subscribe to Changes

```json
// Subscribe to key pattern
event: subscribe
data: {
  "method": "subscribe",
  "params": {
    "pattern": "approvals.*",  // Wildcard pattern
    "include_existing": false   // Only new changes
  }
}

// Notification of change
event: data-changed
data: {
  "method": "notification",
  "params": {
    "type": "update",
    "key": "approvals.deploy-123",
    "value": {...},
    "version": 44,
    "timestamp": 1698765600
  }
}
```

## Use Cases

### 1. User Approvals

```
Service: Deployment tool
Writes: approval data when users approve deployments
Predicate: prod.deployment.approval reads approval data
```

```json
// Service writes approval
PUT /data/approvals.deploy-123
{
  "deployment_id": "deploy-123",
  "approvers": [
    {"user": "alice", "role": "senior_engineer", "timestamp": 1698765000},
    {"user": "bob", "role": "senior_engineer", "timestamp": 1698765100}
  ]
}

// Predicate reads during evaluation
GET /data/approvals.deploy-123
// Evaluates: senior_approvers.length >= 2
```

### 2. Relationship Data

```
Service: Identity/RBAC system
Writes: user-group memberships, org hierarchies
Predicate: Checks if user in required group
```

```json
// Service writes relationships
PUT /data/relationships.user-alice
{
  "user_id": "alice",
  "groups": ["engineering", "senior-engineers"],
  "manager": "charlie",
  "org_unit": "platform"
}

// Predicate checks membership
GET /data/relationships.user-alice
// Evaluates: "senior-engineers" in user.groups
```

### 3. Rate Limiting

```
Service: API gateway
Writes: request counts per user/endpoint
Predicate: Enforces rate limits
```

```json
// Service writes rate limit state
PUT /data/rate_limits.user-alice (TTL: 3600)
{
  "user_id": "alice",
  "endpoint": "/api/deployments",
  "requests_in_window": 245,
  "window_start": 1698765000
}

// Predicate checks limits
GET /data/rate_limits.user-alice
// Evaluates: requests_in_window < max_requests
```

## Authorization Model

### Control Plane Authorization
- **Write policies**: Control plane operators only
- **Authentication**: mTLS, tokens
- **Audit**: Full logging

### Data Plane Authorization
- **Write data**: Services with valid credentials
- **Read data**: Predicate services only (local reads)
- **Authorization**: Service-level tokens, not user-level
- **Audit**: Sampling (high write volume)

**Example Authorization:**
```toml
[data_plane.auth]
# Services can write data
allowed_writers = [
  "deployment-service",
  "identity-service",
  "api-gateway"
]

# Each service has token
tokens_path = "/var/lib/ipe/data-tokens/"

# Predicate services read locally (no auth needed)
```

## Conflict Resolution

When network partitions heal, conflicts may arise. Use version vectors or last-write-wins (LWW).

### Last-Write-Wins (Simple)
```rust
fn resolve_conflict(local: &DataEntry, remote: &DataEntry) -> DataEntry {
    if remote.updated_at > local.updated_at {
        remote.clone()
    } else {
        local.clone()
    }
}
```

### Version Vectors (Advanced)
```rust
struct VersionVector {
    clocks: HashMap<String, u64>,  // instance_id → version
}

fn resolve_conflict(local: &DataEntry, remote: &DataEntry) -> DataEntry {
    // Use version vectors to detect concurrent updates
    // Merge or apply application-specific resolution
}
```

## Configuration

```toml
[predicate_service]
# Predicate service identity
id = "ipe-abc-123"
node_name = "k8s-node-1"

[predicate_service.data_plane]
# Data storage
backend = "rocksdb"
data_path = "/var/lib/ipe/data"

# Write optimizations
batch_size = 1000
flush_interval_ms = 100

# Replication
message_plane = "pubsub"  # or "gossip" or "hybrid"
pubsub_url = "nats://localhost:4222"
subscribe_topics = ["data.approvals.*", "data.relationships.*"]

# Conflict resolution
conflict_strategy = "lww"  # last-write-wins

[data_plane.auth]
# Authorization for services writing data
require_auth = true
allowed_writers = ["deployment-service", "identity-service"]
```

## Implementation Phases

| Week | Deliverables |
|------|--------------|
| 1 | RocksDB data store, basic put/get/delete API |
| 2 | Versioning, tombstones, TTL expiration |
| 3 | Pub/Sub message plane integration (NATS/Redis) |
| 4 | Subscribe API, change notifications |
| 5 | Batch operations, conflict resolution |
| 6 | Gossip protocol (fallback), hybrid mode |

## Success Metrics

- \>10K writes/sec per predicate service
- <1ms local read latency
- <10ms replication latency (p99)
- <100ms propagation time to all predicate services (p99)
- Zero data loss during network partitions
- Eventual consistency within seconds

## Alternatives Considered

| Approach | Rejected Because |
|----------|------------------|
| Control plane manages data | Bottleneck, wrong consistency model (CP not AP) |
| Strong consistency (CP) | Sacrifices availability, not suitable for high write loads |
| No replication | Each service writes locally, but no coordination |
| Centralized database | Single point of failure, doesn't scale |

## Security Considerations

### Data Sensitivity
- Data may contain PII, permissions, relationships
- Encryption at rest (RocksDB encryption)
- Encryption in transit (TLS for message plane)
- Access control per key namespace

### Example Security Policy
```toml
[data_plane.security]
# Encryption
encrypt_at_rest = true
encryption_key_path = "/var/lib/ipe/keys/data.key"

# Access control
enforce_namespaces = true
namespace_auth = [
  {namespace = "approvals.*", writers = ["deployment-service"]},
  {namespace = "relationships.*", writers = ["identity-service"]},
]
```

## Future Enhancements

### Transactional Writes
```json
// Atomic multi-key update
event: transaction
data: {
  "method": "transaction",
  "params": {
    "operations": [
      {"op": "put", "key": "a", "value": {...}},
      {"op": "put", "key": "b", "value": {...}},
      {"op": "delete", "key": "c"}
    ]
  }
}
```

### Query API
```json
// Query by pattern
event: query
data: {
  "method": "query",
  "params": {
    "pattern": "approvals.*",
    "filter": {"deployment_id": "deploy-123"}
  }
}
```

### Data Compaction
- Automatic cleanup of old versions
- Tombstone garbage collection
- TTL-based expiration

## References

- [CAP Theorem](https://en.wikipedia.org/wiki/CAP_theorem)
- [Conflict-free Replicated Data Types (CRDTs)](https://crdt.tech/)
- [Vector Clocks](https://en.wikipedia.org/wiki/Vector_clock)
- [NATS Pub/Sub](https://nats.io/)
- [Redis Pub/Sub](https://redis.io/topics/pubsub)
- [Gossip Protocol](https://en.wikipedia.org/wiki/Gossip_protocol)
- [RFC-001: Sidecar Service Architecture](001-sidecar-service-architecture.md)
- [RFC-003: Policy Tree Storage](003-policy-tree-storage.md)
