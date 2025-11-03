# Relationship Model Implementation Summary

## Overview

**Branch**: `feature/rocksdb-integration-tests`
**Status**: ✅ Complete - Relationship model implemented
**Total Tests**: 336 (272 unit + 64 integration)
**Relationship Tests**: 26 integration tests + 10 unit tests
**Test Result**: ✅ All passing
**Benchmarks**: 10 comprehensive suites

## Implementation Summary

### Core Relationship Model

The relationship system enables modeling of:
1. **Direct relationships** - Simple subject-relation-object triplets
2. **Transitive relationships** - Chain following (trust chains, org hierarchies)
3. **Multiple relationship types** - Roles, trust, membership, ownership, delegation

### Key Features

#### 1. Relationship Types

```rust
pub enum RelationType {
    Role,        // Non-transitive: "alice" is "editor" of "document"
    Trust,       // Transitive: Certificate chains, CA hierarchies
    Membership,  // Transitive: Org hierarchies, group memberships
    Ownership,   // Non-transitive: Resource ownership
    Delegation,  // Non-transitive: Authority delegation
    Custom(String), // User-defined relationship types
}
```

**Transitivity Rules**:
- ✅ Trust: Can chain (A trusts B, B trusts C → A trusts C)
- ✅ Membership: Can chain (alice → engineers → employees)
- ❌ Role: Cannot chain (editor of doc ≠ editor of section)
- ❌ Ownership: Cannot chain
- ❌ Delegation: Cannot chain

#### 2. Database Schema

```
RocksDB - "relationships" Column Family

Key Format: "relationships:{subject}:{relation}:{object}"
Example: "relationships:alice:editor:document-123"

Value: JSON-serialized Relationship struct
{
  "subject": "alice",
  "relation": "editor",
  "object": "document-123",
  "relation_type": "Role",
  "created_by": "admin",
  "created_at": 1704067200,
  "expires_at": null,
  "metadata": {
    "ticket": "JIRA-456",
    "scope": "full-access"
  }
}
```

#### 3. Core Operations

**RelationshipStore API**:
```rust
// Basic CRUD
add_relationship(Relationship) -> Result<()>
get_relationship(subject, relation, object) -> Result<Option<Relationship>>
has_relationship(subject, relation, object) -> Result<bool>
remove_relationship(subject, relation, object) -> Result<()>

// Transitive operations
has_transitive_relationship(subject, relation, object) -> Result<bool>
find_relationship_path(subject, relation, object) -> Result<Option<RelationshipPath>>

// Bulk operations
check_relationships(Vec<RelationshipQuery>) -> Result<Vec<bool>>
list_subject_relationships(subject) -> Result<Vec<Relationship>>
count_relationships() -> Result<usize>
```

**EvaluationContext Integration**:
```rust
ctx.with_relationship_store(store)
ctx.has_relationship(relation, object) -> Result<bool>
ctx.has_transitive_relationship(relation, object) -> Result<bool>
ctx.find_relationship_path(relation, object) -> Result<Option<RelationshipPath>>
```

### Use Cases

#### Use Case 1: Role-Based Access Control

```rust
// Alice is an editor of document-123
store.add_relationship(
    Relationship::role("alice", "editor", "document-123", "admin")
).unwrap();

// Check in policy evaluation
let ctx = EvaluationContext::new(
    Resource::url("document-123"),
    Action::new(Operation::Update, "document-123"),
    Request { principal: Principal::user("alice"), ..Default::default() },
).with_relationship_store(store);

assert!(ctx.has_relationship("editor", "document-123").unwrap());
```

**Policy Integration**:
```
policy "editors-can-update" {
    match {
        resource.type == "document"
        action.operation == "update"
    }

    when {
        has_relationship("editor", resource.id)
    }

    allow
}
```

#### Use Case 2: Certificate Trust Chains

```rust
// Build PKI hierarchy
store.add_relationship(Relationship::trust("leaf-cert", "intermediate-ca", "pki")).unwrap();
store.add_relationship(Relationship::trust("intermediate-ca", "root-ca", "pki")).unwrap();

// Validate trust chain (transitive)
assert!(store.has_transitive_relationship("leaf-cert", "trusted_by", "root-ca").unwrap());

// Get the path
let path = store.find_relationship_path("leaf-cert", "trusted_by", "root-ca").unwrap();
// path.depth == 2
// path.path[0]: leaf-cert -> intermediate-ca
// path.path[1]: intermediate-ca -> root-ca
```

**Policy Integration**:
```
policy "require-trusted-certificate" {
    match {
        principal.type == "certificate"
    }

    when {
        has_transitive_relationship("trusted_by", "root-ca")
    }

    allow
}
```

#### Use Case 3: Organizational Hierarchy

```rust
// Build org structure
store.add_relationship(Relationship::membership("alice", "engineers", "hr")).unwrap();
store.add_relationship(Relationship::membership("engineers", "employees", "hr")).unwrap();
store.add_relationship(Relationship::membership("employees", "everyone", "hr")).unwrap();

// Check memberships (transitive)
assert!(store.has_relationship("alice", "member_of", "engineers").unwrap());
assert!(store.has_transitive_relationship("alice", "member_of", "employees").unwrap());
assert!(store.has_transitive_relationship("alice", "member_of", "everyone").unwrap());
```

**Policy Integration**:
```
policy "employees-only-resource" {
    match {
        resource.requires_employment == true
    }

    when {
        has_transitive_relationship("member_of", "employees")
    }

    allow
}
```

### Test Coverage

#### Unit Tests (10 tests in relationship.rs)
- ✅ Relationship creation
- ✅ Trust relationship helper
- ✅ Expiration handling
- ✅ Store CRUD operations
- ✅ Transitive trust chains (2-level)
- ✅ Transitive membership
- ✅ Relationship path finding
- ✅ Max depth limiting

#### Integration Tests (26 tests)

**Direct Relationships**:
- ✅ Direct role relationship
- ✅ Context has_relationship
- ✅ Multiple roles same subject
- ✅ Remove relationship
- ✅ Relationship with metadata
- ✅ Batch relationship checks
- ✅ List subject relationships

**Transitive Relationships**:
- ✅ Trust chain (2 levels)
- ✅ Trust chain (3 levels)
- ✅ Context transitive trust
- ✅ Membership hierarchy
- ✅ Complex trust graph
- ✅ Shortest path found

**Relationship Types**:
- ✅ Non-transitive role
- ✅ Delegation chain (non-transitive)
- ✅ Ownership relationships
- ✅ Custom relationship type
- ✅ Relationship type display
- ✅ Relationship type transitivity

**Edge Cases**:
- ✅ Relationship expiration
- ✅ Max depth prevents cycles
- ✅ Empty field validation
- ✅ No relationship store error

**Integration**:
- ✅ Combined approval and relationship
- ✅ Large relationship graph (1000 relationships)
- ✅ Count relationships

### Performance Benchmarks

#### Created Benchmarks (10 suites)

1. **bench_relationship_add** - Single write performance
2. **bench_relationship_lookup** - Direct lookup (100, 1k, 10k)
3. **bench_transitive_relationship** - Transitive checks (2, 5, 10 hops)
4. **bench_trust_chain_traversal** - Path finding (2, 5, 10 hops)
5. **bench_membership_hierarchy** - Org hierarchy traversal
6. **bench_batch_relationship_checks** - Batch operations (10, 100, 1000)
7. **bench_list_subject_relationships** - List operations (1, 10, 100 per subject)
8. **bench_complex_graph_traversal** - Multi-path graphs
9. **bench_role_lookups_vs_trust_chains** - Direct vs transitive comparison
10. **bench_with_expiration** - Expiration check overhead

#### Expected Performance

- **Direct lookup**: ~1-5 μs (O(log n))
- **Transitive check (2 hops)**: ~5-15 μs
- **Transitive check (5 hops)**: ~20-50 μs
- **Path finding (BFS)**: ~10-100 μs depending on graph complexity
- **Batch checks (100 items)**: ~500 μs - 1 ms
- **List relationships**: ~10-50 μs per subject

### Key Design Decisions

#### 1. Transitive vs Non-Transitive
- Designed at the type level (`RelationType::is_transitive()`)
- Prevents accidental privilege escalation (roles don't chain)
- Enables efficient trust chains and org hierarchies

#### 2. Breadth-First Search for Paths
- Guarantees shortest path
- Prevents infinite loops with visited set
- Max depth configurable (default: 10 hops)

#### 3. Separate Column Family
- Isolated from approvals for different optimization profile
- Can tune compaction and caching independently
- Prefix scanning for subject lookups

#### 4. Expiration Support
- Time-based relationship validity
- Useful for temporary access grants
- Checked automatically in all operations

#### 5. Metadata Support
- Flexible HashMap for additional context
- Examples: ticket IDs, justification, certificate chains
- Useful for auditing and debugging

### Example Scenarios

#### Scenario 1: Editor Access Control

```rust
let rel_store = Arc::new(RelationshipStore::new_temp().unwrap());

// Admin grants editor role
rel_store.add_relationship(
    Relationship::role("alice", "editor", "document-123", "admin")
        .with_metadata("ticket", "JIRA-789")
).unwrap();

// Alice tries to edit document
let ctx = EvaluationContext::new(
    Resource::url("document-123"),
    Action::new(Operation::Update, "document"),
    Request { principal: Principal::user("alice"), ..Default::default() },
).with_relationship_store(rel_store);

// Policy checks relationship
if ctx.has_relationship("editor", "document-123").unwrap() {
    // Allow edit
} else {
    // Deny
}
```

#### Scenario 2: Certificate Validation Through Root CA

```rust
let rel_store = Arc::new(RelationshipStore::new_temp().unwrap());

// Build PKI chain
rel_store.add_relationship(
    Relationship::trust("cert-abc123", "intermediate-ca-1", "pki")
).unwrap();

rel_store.add_relationship(
    Relationship::trust("intermediate-ca-1", "root-ca", "pki")
).unwrap();

// Certificate connects to API
let ctx = EvaluationContext::new(
    Resource::url("https://api.example.com"),
    Action::new(Operation::Read, "api"),
    Request {
        principal: Principal::new("cert-abc123")
            .with_attribute("type", AttributeValue::String("certificate".into())),
        ..Default::default()
    },
).with_relationship_store(rel_store);

// Policy validates trust chain
if ctx.has_transitive_relationship("trusted_by", "root-ca").unwrap() {
    // Allow API access
} else {
    // Deny - untrusted certificate
}
```

#### Scenario 3: Organizational Access

```rust
let rel_store = Arc::new(RelationshipStore::new_temp().unwrap());

// HR builds org structure
rel_store.add_relationship(
    Relationship::membership("alice", "engineering", "hr")
).unwrap();

rel_store.add_relationship(
    Relationship::membership("engineering", "employees", "hr")
).unwrap();

// Alice accesses employee-only resource
let ctx = EvaluationContext::new(
    Resource::url("employee-portal"),
    Action::new(Operation::Read, "portal"),
    Request { principal: Principal::user("alice"), ..Default::default() },
).with_relationship_store(rel_store);

// Policy checks transitive membership
if ctx.has_transitive_relationship("member_of", "employees").unwrap() {
    // Allow access to employee portal
}
```

### Combined Approval + Relationship Example

```rust
let approval_store = Arc::new(ApprovalStore::new_temp().unwrap());
let rel_store = Arc::new(RelationshipStore::new_temp().unwrap());

// Alice is an editor
rel_store.add_relationship(
    Relationship::role("alice", "editor", "document-123", "admin")
).unwrap();

// Alice has approval for this specific operation
approval_store.grant_approval(
    Approval::new("alice", "document-123", "DELETE", "manager")
).unwrap();

let ctx = EvaluationContext::new(
    Resource::url("document-123"),
    Action::new(Operation::Delete, "document")
        .with_attribute("method", AttributeValue::String("DELETE".into())),
    Request { principal: Principal::user("alice"), ..Default::default() },
)
.with_approval_store(approval_store)
.with_relationship_store(rel_store);

// Policy requires BOTH editor role AND explicit approval for delete
if ctx.has_relationship("editor", "document-123").unwrap()
    && ctx.has_approval().unwrap() {
    // Allow delete
} else {
    // Deny - need both role and approval
}
```

**Policy DSL**:
```
policy "editors-can-delete-with-approval" {
    match {
        resource.type == "document"
        action.operation == "delete"
    }

    when {
        has_relationship("editor", resource.id)
        has_approval()
    }

    allow
}
```

## Test Results

```bash
$ cargo test --features approvals

running 336 tests
✅ 272 ipe-core unit tests (including 10 relationship tests)
✅ 64 integration tests (including 26 relationship tests)

Integration tests breakdown:
- 13 approval tests
- 8 e2e approval tests
- 17 security tests
- 26 relationship tests

test result: ok. 336 passed; 0 failed
```

## Files Created/Modified

### Core Implementation
- ✅ `crates/ipe-core/src/relationship.rs` (650 lines)
  - RelationshipStore with RocksDB backend
  - Transitive relationship traversal (BFS)
  - Multiple relationship types
  - Complete CRUD operations

### Integration Tests
- ✅ `crates/ipe-core/tests/integration/relationship_tests.rs` (550 lines)
  - 26 comprehensive test scenarios
  - Direct and transitive relationships
  - All relationship types covered
  - Edge case testing

### Benchmarks
- ✅ `crates/ipe-core/benches/relationship_benchmarks.rs` (350 lines)
  - 10 benchmark suites
  - Direct vs transitive comparison
  - Graph traversal performance
  - Batch operation testing

### Modified Files
- ✅ `crates/ipe-core/src/lib.rs` - Export relationship module
- ✅ `crates/ipe-core/src/rar.rs` - Add relationship checking to EvaluationContext
- ✅ `crates/ipe-core/Cargo.toml` - Add relationship benchmarks
- ✅ `crates/ipe-core/tests/integration/mod.rs` - Include relationship tests

## Comparison: Approvals vs Relationships

| Feature | Approvals | Relationships |
|---------|-----------|---------------|
| **Purpose** | Grant specific permissions | Model connections between entities |
| **Transitive** | No | Conditional (by type) |
| **Use Case** | "Can alice access resource-X?" | "Is alice an editor of doc-Y?" |
| **Traversal** | Direct lookup only | BFS graph traversal |
| **Complexity** | O(log n) lookup | O(V + E) traversal |
| **Expiration** | Yes | Yes |
| **Metadata** | Yes | Yes |
| **Examples** | API access approval | Roles, trust chains, org hierarchy |

## Future Enhancements

### Short-term
1. **Reverse index** - Query "who are the editors of doc-X?"
2. **Relationship templates** - Pre-defined role hierarchies
3. **Bulk operations** - Add/remove multiple relationships atomically

### Medium-term
4. **Temporal queries** - "Was alice an editor on date X?"
5. **Relationship constraints** - "Only one owner per resource"
6. **Path caching** - Cache frequently queried paths

### Long-term
7. **Distributed relationships** - Federated trust across systems
8. **Relationship analytics** - Who has most access? Unused relationships?
9. **Auto-expiration policies** - "Editor role expires after 90 days"

## Deployment Considerations

### Database Configuration
```rust
let store = RelationshipStore::new("/var/lib/ipe/relationships.db")?
    .with_max_depth(15); // Adjust based on org depth
```

### Performance Tuning
- **Max traversal depth**: Balance between flexibility and performance
- **Prefix scanning**: Optimize for common lookup patterns
- **Graph size**: Monitor relationship count and density

### Security
- ✅ Empty field validation
- ✅ Expiration enforcement
- ✅ Cycle prevention (max depth)
- ⚠️ Need: Audit logging for relationship changes
- ⚠️ Need: Authorization for relationship management

## Summary

✅ **Complete implementation** of relationship model
✅ **36 tests** (10 unit + 26 integration) all passing
✅ **10 benchmark suites** for performance validation
✅ **Transitive relationship support** with BFS traversal
✅ **Multiple relationship types** (role, trust, membership, etc.)
✅ **Full EvaluationContext integration**
✅ **Combined with approvals** for comprehensive authorization

The relationship model provides a powerful foundation for:
- Role-based access control
- Certificate trust chains
- Organizational hierarchies
- Custom authorization logic

Combined with the approval system, IPE now has comprehensive authorization capabilities covering both explicit permissions (approvals) and structural relationships (roles, trust, membership).
