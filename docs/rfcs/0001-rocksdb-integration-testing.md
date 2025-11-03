# RFC 0001: RocksDB Integration Testing for Approval-Based Authorization

## Status
- **Status**: Draft
- **Author**: System
- **Created**: 2025-11-03
- **Updated**: 2025-11-03

## Summary

This RFC proposes an integration testing strategy for IPE that combines:
1. Request context (non-human identity with identity + URL)
2. Database context (approvals from privileged data plane stored in RocksDB)
3. Policy evaluation (authorization decisions based on approval presence)

The goal is efficient testing of approval-based authorization with real database operations, including efficient set membership tests.

## Motivation

### Current State
- IPE has in-memory policy storage (`PolicyDataStore`)
- 118+ unit tests using `testing.rs` utilities
- No persistent storage layer
- No approval/signature verification mechanism
- No integration tests with real database operations

### Desired State
- RocksDB-backed approval storage for privileged data plane operations
- Integration tests that verify end-to-end authorization flows
- Efficient approval lookups and set membership tests
- Test scenarios: non-human identity requests should fail without approval, succeed with approval
- Foundation for production approval system

### Use Case
```
Non-human identity (service account, bot) → Request(identity, URL) → Policy Engine
                                                                           ↓
                                            Check RocksDB for approval ←---+
                                                     ↓
                                    Deny (no approval) / Allow (approval exists)
```

## Design

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Integration Test Layer                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Test Setup                                                      │
│  ┌────────────────┐  ┌──────────────────┐  ┌────────────────┐ │
│  │ Test RocksDB   │  │ Privileged Data  │  │ Policy Engine  │ │
│  │ (temp dir)     │  │ Plane Writer     │  │ with Policies  │ │
│  └────────────────┘  └──────────────────┘  └────────────────┘ │
│                                                                  │
│  Test Execution                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ 1. Write approval to RocksDB (privileged operation)       │ │
│  │ 2. Create request context (identity + URL)                │ │
│  │ 3. Evaluate policy (checks approval in RocksDB)           │ │
│  │ 4. Assert decision (allow/deny)                           │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Components

#### 1. RocksDB Schema

Three column families for separation of concerns:

```rust
// Column Families
"policies"   // Policy storage (existing concept, new backend)
"approvals"  // Approval records (new)
"audit"      // Audit logs (future)
```

**Approvals Schema**:
```rust
// Key format: "approvals:{identity}:{resource}"
// Example: "approvals:bot-123:https://api.example.com/data"

struct Approval {
    identity: String,      // Principal identifier (service account, bot ID)
    resource: String,      // URL or resource pattern
    action: String,        // HTTP method or action type
    granted_by: String,    // Privileged identity that granted approval
    granted_at: i64,       // Unix timestamp
    expires_at: Option<i64>, // Optional expiration
    metadata: HashMap<String, String>, // Additional context
}

// Value: JSON-serialized Approval struct
```

**Set Membership for Efficient Lookups**:
```rust
// For set membership tests: "<identity> in <approved_identities>"
// Use key prefix scanning:
// Keys: "approvals:{identity}:*" → O(log n) seek + scan
// Bloom filters in RocksDB reduce disk I/O for negative lookups
```

#### 2. Database Context API

```rust
/// Database context for approval storage and retrieval
pub struct ApprovalStore {
    db: Arc<rocksdb::DB>,
}

impl ApprovalStore {
    /// Create new store (for production and tests)
    pub fn new(path: &Path) -> Result<Self, Error>;

    /// Create temporary store for testing
    pub fn new_temp() -> Result<Self, Error>;

    /// Write approval (privileged operation - requires authorization)
    pub fn grant_approval(&self, approval: Approval) -> Result<(), Error>;

    /// Check if approval exists
    pub fn has_approval(&self, identity: &str, resource: &str, action: &str)
        -> Result<bool, Error>;

    /// Get approval details
    pub fn get_approval(&self, identity: &str, resource: &str, action: &str)
        -> Result<Option<Approval>, Error>;

    /// Revoke approval
    pub fn revoke_approval(&self, identity: &str, resource: &str, action: &str)
        -> Result<(), Error>;

    /// Check set membership: is identity in approved set for resource?
    pub fn is_in_approved_set(&self, identity: &str, resource_pattern: &str)
        -> Result<bool, Error>;

    /// Batch check for efficiency
    pub fn check_approvals(&self, checks: Vec<ApprovalCheck>)
        -> Result<Vec<bool>, Error>;
}

/// Request context extension for approval checking
pub trait ApprovalContext {
    /// Add approval store to evaluation context
    fn with_approval_store(self, store: Arc<ApprovalStore>) -> Self;

    /// Check if current request has approval
    fn has_approval(&self) -> Result<bool, Error>;
}
```

#### 3. Policy Integration

Extend existing policy predicates to support approval checks:

```
policy "require-approval-for-bots" {
    match {
        principal.type == "bot"
        resource.url matches "^https://api.example.com/.*"
    }

    // New predicate: checks RocksDB
    when {
        has_approval(principal.id, resource.url, action.method)
    }

    allow
}

policy "deny-without-approval" {
    match {
        principal.type == "bot"
    }

    when {
        not has_approval(principal.id, resource.url, action.method)
    }

    deny
}
```

Implementation in engine:
```rust
// In engine.rs evaluation
fn evaluate_approval_predicate(
    &self,
    ctx: &EvaluationContext,
    predicate: &ApprovalPredicate,
) -> Result<bool, Error> {
    let store = ctx.approval_store()
        .ok_or(Error::NoApprovalStore)?;

    store.has_approval(
        &ctx.request.principal.id,
        &ctx.resource.url,
        &ctx.action.method,
    )
}
```

### Testing Strategy

#### Test Structure

```
crates/ipe-core/tests/
└── integration/
    ├── mod.rs                    // Test utilities
    ├── approval_store_tests.rs   // ApprovalStore unit tests
    ├── approval_policy_tests.rs  // Policy + approval integration
    └── e2e_tests.rs              // Full end-to-end scenarios
```

#### Test Scenarios

**Scenario 1: Deny without approval**
```rust
#[test]
fn test_bot_denied_without_approval() {
    let store = ApprovalStore::new_temp().unwrap();
    let engine = PolicyEngine::new()
        .with_approval_store(Arc::new(store));

    let ctx = EvaluationContext {
        principal: Principal::bot("bot-123"),
        resource: Resource::url("https://api.example.com/data"),
        action: Action::http("GET"),
        ..Default::default()
    };

    let decision = engine.evaluate(&ctx).unwrap();
    assert_eq!(decision.effect, Effect::Deny);
}
```

**Scenario 2: Allow with approval**
```rust
#[test]
fn test_bot_allowed_with_approval() {
    let store = ApprovalStore::new_temp().unwrap();

    // Privileged operation: grant approval
    store.grant_approval(Approval {
        identity: "bot-123".into(),
        resource: "https://api.example.com/data".into(),
        action: "GET".into(),
        granted_by: "admin-user".into(),
        granted_at: now(),
        expires_at: None,
        metadata: HashMap::new(),
    }).unwrap();

    let engine = PolicyEngine::new()
        .with_approval_store(Arc::new(store));

    let ctx = EvaluationContext {
        principal: Principal::bot("bot-123"),
        resource: Resource::url("https://api.example.com/data"),
        action: Action::http("GET"),
        ..Default::default()
    };

    let decision = engine.evaluate(&ctx).unwrap();
    assert_eq!(decision.effect, Effect::Allow);
}
```

**Scenario 3: Set membership test**
```rust
#[test]
fn test_efficient_set_membership() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approvals to multiple bots
    for bot_id in 1..=1000 {
        store.grant_approval(Approval {
            identity: format!("bot-{}", bot_id),
            resource: "https://api.example.com/data".into(),
            action: "GET".into(),
            granted_by: "admin".into(),
            granted_at: now(),
            expires_at: None,
            metadata: HashMap::new(),
        }).unwrap();
    }

    // Test set membership: bot-500 in approved set
    let is_approved = store.is_in_approved_set(
        "bot-500",
        "https://api.example.com/data"
    ).unwrap();
    assert!(is_approved);

    // Test non-member
    let is_approved = store.is_in_approved_set(
        "bot-9999",
        "https://api.example.com/data"
    ).unwrap();
    assert!(!is_approved);
}
```

**Scenario 4: Batch approval checks**
```rust
#[test]
fn test_batch_approval_checks() {
    let store = ApprovalStore::new_temp().unwrap();

    // Setup: grant some approvals
    store.grant_approval(Approval { /* bot-1 for resource-A */ }).unwrap();
    store.grant_approval(Approval { /* bot-2 for resource-B */ }).unwrap();

    // Batch check
    let checks = vec![
        ApprovalCheck::new("bot-1", "resource-A", "GET"),
        ApprovalCheck::new("bot-2", "resource-B", "GET"),
        ApprovalCheck::new("bot-3", "resource-C", "GET"),
    ];

    let results = store.check_approvals(checks).unwrap();
    assert_eq!(results, vec![true, true, false]);
}
```

**Scenario 5: Approval expiration**
```rust
#[test]
fn test_expired_approval_denied() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval that expires in the past
    store.grant_approval(Approval {
        identity: "bot-123".into(),
        resource: "https://api.example.com/data".into(),
        action: "GET".into(),
        granted_by: "admin".into(),
        granted_at: now() - 3600,
        expires_at: Some(now() - 1800), // Expired 30 min ago
        metadata: HashMap::new(),
    }).unwrap();

    let engine = PolicyEngine::new()
        .with_approval_store(Arc::new(store));

    let ctx = EvaluationContext {
        principal: Principal::bot("bot-123"),
        resource: Resource::url("https://api.example.com/data"),
        action: Action::http("GET"),
        ..Default::default()
    };

    let decision = engine.evaluate(&ctx).unwrap();
    assert_eq!(decision.effect, Effect::Deny);
}
```

### Performance Considerations

#### RocksDB Optimizations

1. **Bloom Filters**: Reduce disk I/O for negative lookups (non-existent approvals)
   ```rust
   let mut opts = rocksdb::Options::default();
   opts.set_bloom_filter(10, false); // 10 bits per key
   ```

2. **Prefix Extraction**: Efficient scanning for set membership
   ```rust
   opts.set_prefix_extractor(
       rocksdb::SliceTransform::create_fixed_prefix(20) // "approvals:{identity}:"
   );
   ```

3. **Block Cache**: Keep hot approval data in memory
   ```rust
   opts.set_block_cache(&rocksdb::Cache::new_lru_cache(256 * 1024 * 1024)); // 256MB
   ```

4. **Column Family Options**: Tune per use case
   ```rust
   // Approvals: optimized for point lookups
   let mut approval_opts = rocksdb::Options::default();
   approval_opts.set_bloom_filter(10, false);
   approval_opts.optimize_for_point_lookup(64); // 64MB block cache
   ```

#### Benchmarking

```rust
#[bench]
fn bench_approval_lookup(b: &mut Bencher) {
    let store = ApprovalStore::new_temp().unwrap();
    // Pre-populate with 10k approvals
    for i in 0..10_000 {
        store.grant_approval(/* ... */).unwrap();
    }

    b.iter(|| {
        store.has_approval("bot-5000", "https://api.example.com/data", "GET")
    });
}

#[bench]
fn bench_set_membership(b: &mut Bencher) {
    let store = ApprovalStore::new_temp().unwrap();
    // Pre-populate with 10k approvals
    for i in 0..10_000 {
        store.grant_approval(/* ... */).unwrap();
    }

    b.iter(|| {
        store.is_in_approved_set("bot-5000", "https://api.example.com/data")
    });
}
```

### Security Considerations

1. **Privileged Data Plane**: Only authorized entities can write approvals
   - Separate `ApprovalWriter` API with authentication
   - Audit all approval grants/revocations

2. **Approval Integrity**: Prevent tampering
   - Optional: Sign approvals with HMAC/digital signature
   - Store signature in `metadata` field

3. **DoS Protection**: Rate limiting for approval checks
   - Cache negative lookups (non-existent approvals)
   - Circuit breaker for database failures

4. **Isolation**: Test database should not affect production
   - Use `ApprovalStore::new_temp()` in tests
   - Separate RocksDB instances per environment

## Implementation Plan

### Phase 1: Foundation (Week 1)
- [ ] Create `ApprovalStore` struct with RocksDB backend
- [ ] Implement basic CRUD operations (grant, get, revoke)
- [ ] Add `ApprovalStore::new_temp()` for testing
- [ ] Write unit tests for `ApprovalStore`

### Phase 2: Policy Integration (Week 2)
- [ ] Extend `EvaluationContext` with optional approval store
- [ ] Implement `has_approval()` predicate in engine
- [ ] Add policy syntax for approval checks
- [ ] Write integration tests for policy + approval

### Phase 3: Advanced Features (Week 3)
- [ ] Implement set membership (`is_in_approved_set`)
- [ ] Add batch approval checks
- [ ] Implement expiration handling
- [ ] Add performance benchmarks

### Phase 4: Testing & Documentation (Week 4)
- [ ] Write comprehensive integration test suite
- [ ] Add end-to-end test scenarios
- [ ] Performance tuning and optimization
- [ ] Document API and usage patterns

## Alternatives Considered

### Alternative 1: In-Memory Approval Cache
- **Pros**: Faster, simpler for testing
- **Cons**: No persistence, doesn't test real database behavior
- **Decision**: Rejected - need real RocksDB testing

### Alternative 2: Mock Database
- **Pros**: Faster tests, no RocksDB dependency
- **Cons**: Doesn't test actual database performance/behavior
- **Decision**: Use both - mocks for unit tests, real DB for integration

### Alternative 3: SQL Database (PostgreSQL/SQLite)
- **Pros**: Rich query capabilities, transactions
- **Cons**: Heavier dependency, slower for key-value lookups
- **Decision**: RocksDB better for high-performance approval lookups

## Open Questions

1. **Approval Revocation**: Should revoked approvals be hard-deleted or soft-deleted (tombstone)?
   - Recommendation: Soft-delete for audit trail

2. **Approval Patterns**: Should we support glob patterns for resources (e.g., `https://api.example.com/*`)?
   - Recommendation: Start with exact match, add patterns in Phase 2

3. **Multi-Signature Approvals**: M-of-N approval requirement (e.g., 2 of 3 admins)?
   - Recommendation: Design schema to support, implement in future RFC

4. **Distributed Approvals**: Replication across multiple RocksDB instances?
   - Recommendation: Out of scope - future RFC for distributed IPE

## Success Metrics

1. **Test Coverage**: >90% coverage for approval-related code
2. **Performance**: <1ms p99 latency for approval lookup (10k approvals)
3. **Efficiency**: Set membership test scales to 100k+ approvals
4. **Correctness**: All test scenarios pass (deny without, allow with approval)

## References

- [RocksDB Documentation](https://github.com/facebook/rocksdb/wiki)
- [IPE Core Engine](crates/ipe-core/src/engine.rs:1)
- [IPE Request Context](crates/ipe-core/src/rar.rs:1)
- [IPE Testing Utilities](crates/ipe-core/src/testing.rs:1)
