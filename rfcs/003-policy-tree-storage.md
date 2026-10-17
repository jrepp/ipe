# RFC-003: Policy Tree Storage

**Status:** 📝 Draft
**Created:** 2025-10-27
**Author:** IPE Contributors
**Depends On:** RFC-001

## Summary

Content-addressable policy storage using a Merkle-style tree with SHA-256 hashing. Enables hierarchical namespacing, incremental updates, version tracking, and tamper detection.

## Motivation

Requirements:
- **Hierarchical:** Natural paths like `prod.deployment.approval`
- **Versioned:** Content-addressable with SHA-256 hashes
- **Incremental:** Only changed nodes recomputed
- **Navigable:** Easy tree traversal and queries
- **Tamper-proof:** Cryptographic integrity
- **Efficient:** O(log n) lookups

## Key Features

✅ Merkle tree structure
✅ SHA-256 content addressing
✅ Immutable policy versions
✅ Bottom-up hash recomputation
✅ Multiple storage backends

## Tree Structure

### Conceptual Model

```
root (hash: abc123...)
├── prod (hash: def456...)
│   ├── deployment (hash: ghi789...)
│   │   ├── approval (hash: jkl012...)  [POLICY]
│   │   └── validation (hash: mno345...)  [POLICY]
│   └── api (hash: pqr678...)
│       └── rate_limit (hash: stu901...)  [POLICY]
└── staging (hash: vwx234...)
    └── deployment (hash: yzab567...)
        └── approval (hash: cdef890...)  [POLICY]
```

### Node Types

**Directory Node** (namespace component):
```rust
struct DirectoryNode {
    name: String,
    hash: Hash,                         // SHA-256 of children
    children: BTreeMap<String, Hash>,  // name → hash
    updated_at: u64,
}
```

**Policy Node** (compiled policy):
```rust
struct PolicyNode {
    name: String,
    hash: Hash,                    // SHA-256 of bytecode
    parent_hash: Hash,             // Previous version
    policy: CompiledPolicy,        // Bytecode
    compiled_at: u64,
}
```

### Hash Computation (SHA-256)

```rust
// Directory: hash(name + sorted(child_name, child_hash)*)
fn compute_dir_hash(dir: &DirectoryNode) -> Hash {
    SHA256(dir.name + sorted(dir.children))
}

// Policy: hash(bytecode + name + parent_hash)
fn compute_policy_hash(policy: &PolicyNode) -> Hash {
    SHA256(policy.bytecode + policy.name + policy.parent_hash)
}
```

## Storage Backends

### Backend Comparison

| Backend | Persistence | Performance | Use Case |
|---------|-------------|-------------|----------|
| **In-Memory** | ❌ | 1M ops/s | Development/Testing |
| **RocksDB** | ✅ | 100K ops/s | Production (Primary) |

### 1. In-Memory (Default)

```rust
struct InMemoryStore {
    objects: DashMap<Hash, Object>,   // Content-addressable
    path_index: DashMap<String, Hash>, // Fast path lookup
    root: ArcSwap<Hash>,               // Lock-free root
}
```

- O(1) hash lookups
- Lock-free reads
- No persistence

### 2. RocksDB (Production)

```
# Key-value store with column families
objects:     hash → bincode(object)
path_index:  path → hash
metadata:    "root" → root_hash
```

- Persistent, LSM-tree
- LRU caching
- Atomic batch writes
- Production-ready with proven reliability
- Excellent write throughput for GitOps updates

## Core Operations

### 1. Path Lookup
```
prod.deployment.approval → hash lookup → PolicyNode
```
- Fast path: O(1) via path index cache
- Slow path: O(log n) tree traversal

### 2. Policy Update (Bottom-Up)
```
1. Hash new policy bytecode
2. Store policy object
3. Update parent directory → recompute hash
4. Propagate up to root → recompute hashes
5. Atomic root swap (arc-swap)
6. Update path index cache
```

**Incremental:** Only changed path recomputed (not entire tree)

### 3. Tree Traversal and Enumeration
```rust
// List all policies with prefix
fn list_policies(prefix: &str) -> Vec<String>

// Get subtree structure
fn get_subtree(path: &str, depth: Option<usize>) -> TreeNode

// Enumerate directory children
fn list_children(path: &str) -> Vec<(String, NodeType)>
```

**Enumeration Features:**
- DFS/BFS traversal with configurable depth limits
- Filter by path prefix for scoped queries
- Return full tree structure with metadata
- Distinguish between directory and policy nodes
- Efficient caching of frequently accessed subtrees

**Implementation Notes:**
- Use path index for O(1) prefix lookups
- Lazy-load policy content (return metadata only by default)
- Support pagination for large directories
- Cache directory listings (invalidate on update)

### 4. Version History
```
current_hash → parent_hash → parent_hash → ...
```
- Chain of previous versions
- Optional GC for old versions

## Update Flow

```
Client → Control Plane → Policy Store
         1. Validate
         2. Compile
         3. Store + recompute hashes
         4. Atomic root swap
         5. Notify data planes
```

### Incremental Update Example

```
Before: root(abc) → prod(def) → deployment(ghi) ← UPDATE
                              → api(pqr)

After:  root(xyz) → prod(uvw) → deployment(jkl) ← NEW
                              → api(pqr)         ← REUSED
```

**Efficiency:** Only 3 new objects (deployment, prod, root). API subtree reused.

## Memory Management

```rust
struct TieredStore {
    hot_cache: LruCache<Hash, PolicyNode>,  // 10MB, last 5min
    path_index: DashMap<String, Hash>,      // 1-2MB, always hot
    backend: Box<dyn StorageBackend>,       // Disk/persistent
}
```

- Hot: Recent policies (LRU eviction)
- Warm: Path index (always in memory)
- Cold: All objects (disk)

## Persistence

### Startup
1. Load root hash from metadata
2. Warm path index cache
3. Lazy-load policies on access
4. Verify root hash integrity

### Backup
- **Incremental:** Export new objects only
- **Full:** Periodic complete snapshot
- **Recovery:** Restore to any root hash

## Implementation Phases

| Week | Deliverables |
|------|--------------|
| 1 | In-memory store, hash computation, CRUD ops |
| 2 | RocksDB backend, snapshot/restore |
| 3 | Hot/cold caching, path index warming |
| 4 | Tree enumeration API, subtree queries, GitOps integration |

## Success Metrics

- <100μs lookup (hot), <1ms (cold)
- <5ms policy update
- <20MB for 1000 policies
- <10KB per policy (compressed)
- <100ms startup (1000 policies)

## Future: Distributed Sync

```
Sidecar A (root:abc) ←→ Sidecar B (root:xyz)
                ↓
        Compare root hashes
                ↓
    Same? → In sync
    Diff? → Reconcile subtrees (Merkle property)
```

## Future: Garbage Collection

```rust
// Prune unreachable old versions
fn gc_old_versions(retention_days: u64) {
    let live_set = compute_reachable_from_root();
    delete_objects_not_in(live_set, older_than(retention_days));
}
```

## References

- [Content-Addressable Storage](https://en.wikipedia.org/wiki/Content-addressable_storage)
- [Merkle Tree](https://en.wikipedia.org/wiki/Merkle_tree)
- [RocksDB](https://rocksdb.org/)
- [SQLite](https://www.sqlite.org/)
- [Git Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects) (inspiration)
