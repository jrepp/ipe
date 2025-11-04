# RFC 0002: Scope and TTL Model for Data Stores

## Status
- **Status**: Draft
- **Author**: System
- **Created**: 2025-11-03
- **Related**: RFC 0001 (RocksDB Integration)

## Summary

This RFC proposes a comprehensive scope and Time-To-Live (TTL) model for all data store components (approvals, relationships). The scope model enables multi-tenant isolation and environment separation, while the TTL model provides automatic cleanup of expired records.

## Motivation

### Current State
- ✅ Basic expiration timestamps (`expires_at`) exist
- ❌ No automatic cleanup of expired records
- ❌ No scope/tenant isolation
- ❌ No environment separation (dev/staging/prod)
- ❌ Manual cleanup required for expired data

### Desired State
- ✅ Automatic TTL-based cleanup using RocksDB compaction filters
- ✅ Multi-tenant isolation via scopes
- ✅ Environment separation (dev, staging, prod)
- ✅ Efficient garbage collection without scan overhead
- ✅ Query filtering by scope

### Use Cases

**Use Case 1: Multi-tenant SaaS**
```
Tenant A: approvals for their users/resources
Tenant B: approvals for their users/resources (isolated from A)
```

**Use Case 2: Environment Separation**
```
dev: Short-lived approvals for testing
staging: Medium-lived approvals for validation
prod: Long-lived approvals for production use
```

**Use Case 3: Automatic Cleanup**
```
Approval granted with 1-hour TTL
After 1 hour: automatically removed during compaction
No manual cleanup required
```

## Design

### Scope Model

#### Scope Definition

```rust
/// Scope defines the isolation boundary for data
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Scope {
    /// Global scope - accessible across all tenants (use sparingly)
    Global,

    /// Tenant-specific scope
    Tenant(String),

    /// Environment-specific scope (dev, staging, prod)
    Environment(String),

    /// Tenant + Environment combination (most common)
    TenantEnvironment { tenant: String, environment: String },

    /// Custom scope with hierarchical path
    Custom(Vec<String>),
}

impl Scope {
    /// Create tenant scope
    pub fn tenant(tenant: impl Into<String>) -> Self {
        Scope::Tenant(tenant.into())
    }

    /// Create environment scope
    pub fn env(env: impl Into<String>) -> Self {
        Scope::Environment(env.into())
    }

    /// Create tenant+environment scope
    pub fn tenant_env(tenant: impl Into<String>, env: impl Into<String>) -> Self {
        Scope::TenantEnvironment {
            tenant: tenant.into(),
            environment: env.into(),
        }
    }

    /// Encode scope as string for storage key
    pub fn encode(&self) -> String {
        match self {
            Scope::Global => "global".to_string(),
            Scope::Tenant(t) => format!("tenant:{}", t),
            Scope::Environment(e) => format!("env:{}", e),
            Scope::TenantEnvironment { tenant, environment } => {
                format!("tenant:{}:env:{}", tenant, environment)
            }
            Scope::Custom(parts) => format!("custom:{}", parts.join(":")),
        }
    }
}
```

#### Storage Key Format with Scope

**Before**:
```
approvals:{identity}:{resource}:{action}
relationships:{subject}:{relation}:{object}
```

**After**:
```
approvals:{scope}:{identity}:{resource}:{action}
relationships:{scope}:{subject}:{relation}:{object}
```

**Examples**:
```
approvals:tenant:acme-corp:alice:https://api.example.com/data:GET
approvals:tenant:acme-corp:env:prod:bob:https://api.example.com/admin:POST
relationships:tenant:widgets-inc:alice:editor:document-123
```

### TTL Model

#### TTL Configuration

```rust
/// TTL (Time-To-Live) configuration for automatic cleanup
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TTLConfig {
    /// Default TTL in seconds (if not specified per-record)
    pub default_ttl_seconds: Option<i64>,

    /// Minimum TTL allowed (prevents accidental short TTLs)
    pub min_ttl_seconds: i64,

    /// Maximum TTL allowed (prevents unbounded growth)
    pub max_ttl_seconds: i64,

    /// Whether to enforce TTL (if false, TTL is advisory only)
    pub enforce_ttl: bool,
}

impl Default for TTLConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: None, // No default expiration
            min_ttl_seconds: 60,       // 1 minute minimum
            max_ttl_seconds: 365 * 24 * 3600, // 1 year maximum
            enforce_ttl: true,
        }
    }
}

impl TTLConfig {
    /// Quick configs for common scenarios
    pub fn temporary() -> Self {
        Self {
            default_ttl_seconds: Some(3600), // 1 hour
            min_ttl_seconds: 60,
            max_ttl_seconds: 24 * 3600, // 1 day max
            enforce_ttl: true,
        }
    }

    pub fn short_lived() -> Self {
        Self {
            default_ttl_seconds: Some(24 * 3600), // 1 day
            min_ttl_seconds: 3600,
            max_ttl_seconds: 7 * 24 * 3600, // 1 week max
            enforce_ttl: true,
        }
    }

    pub fn long_lived() -> Self {
        Self {
            default_ttl_seconds: Some(30 * 24 * 3600), // 30 days
            min_ttl_seconds: 24 * 3600,
            max_ttl_seconds: 365 * 24 * 3600, // 1 year max
            enforce_ttl: true,
        }
    }
}
```

#### RocksDB Compaction Filter for TTL

RocksDB provides built-in TTL support via compaction filters:

```rust
use rocksdb::{Options, DB};

impl ApprovalStore {
    fn open_db_with_ttl(path: &Path, ttl_seconds: u64) -> Result<DB> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        // Enable TTL-based compaction filter
        // Records older than ttl_seconds will be automatically deleted during compaction
        let db = DB::open_with_ttl(&opts, path, Duration::from_secs(ttl_seconds))
            .map_err(|e| ApprovalError::DatabaseError(e.to_string()))?;

        Ok(db)
    }
}
```

**How TTL Works**:
1. Each record stores its creation timestamp
2. During compaction, RocksDB checks if `current_time - creation_time > TTL`
3. If expired, record is automatically deleted
4. No manual cleanup needed
5. Compaction happens periodically (configurable)

#### Updated Data Models

**Approval with Scope and TTL**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Approval {
    // Existing fields
    pub identity: String,
    pub resource: String,
    pub action: String,
    pub granted_by: String,
    pub granted_at: i64,
    pub expires_at: Option<i64>,
    pub metadata: HashMap<String, String>,

    // New fields
    pub scope: Scope,
    pub ttl_seconds: Option<i64>,
}

impl Approval {
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_ttl(mut self, ttl_seconds: i64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        // Also set expires_at for backward compatibility
        self.expires_at = Some(Utc::now().timestamp() + ttl_seconds);
        self
    }

    /// Generate scoped storage key
    fn key(&self) -> String {
        format!(
            "approvals:{}:{}:{}:{}",
            self.scope.encode(),
            self.identity,
            self.resource,
            self.action
        )
    }
}
```

**Relationship with Scope and TTL**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    // Existing fields
    pub subject: String,
    pub relation: String,
    pub object: String,
    pub relation_type: RelationType,
    pub created_by: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub metadata: HashMap<String, String>,

    // New fields
    pub scope: Scope,
    pub ttl_seconds: Option<i64>,
}

impl Relationship {
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_ttl(mut self, ttl_seconds: i64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self.expires_at = Some(Utc::now().timestamp() + ttl_seconds);
        self
    }

    fn key(&self) -> String {
        format!(
            "relationships:{}:{}:{}:{}",
            self.scope.encode(),
            self.subject,
            self.relation,
            self.object
        )
    }
}
```

### Query Filtering by Scope

```rust
impl ApprovalStore {
    /// Query approvals within a specific scope
    pub fn list_approvals_in_scope(
        &self,
        identity: &str,
        scope: &Scope,
    ) -> Result<Vec<Approval>> {
        let prefix = format!("approvals:{}:{}:", scope.encode(), identity);
        // ... scan with prefix
    }

    /// Count approvals in scope
    pub fn count_approvals_in_scope(&self, scope: &Scope) -> Result<usize> {
        let prefix = format!("approvals:{}:", scope.encode());
        // ... scan with prefix
    }

    /// Delete all approvals in scope (tenant cleanup)
    pub fn delete_scope(&self, scope: &Scope) -> Result<usize> {
        let prefix = format!("approvals:{}:", scope.encode());
        // ... delete all keys with prefix
    }
}

impl RelationshipStore {
    /// Similar scope-based operations for relationships
    pub fn list_relationships_in_scope(
        &self,
        subject: &str,
        scope: &Scope,
    ) -> Result<Vec<Relationship>> {
        // ... implementation
    }
}
```

### Default Scope Behavior

**Option 1: Required Scope (Strict)**
```rust
// All operations require explicit scope
store.grant_approval(
    Approval::new("alice", "resource", "GET", "admin")
        .with_scope(Scope::tenant("acme-corp"))
)?;
```

**Option 2: Default to Global (Lenient)**
```rust
// If no scope specified, default to Global
impl Default for Approval {
    fn default() -> Self {
        Self {
            // ... other fields
            scope: Scope::Global,
            ttl_seconds: None,
        }
    }
}
```

**Recommendation**: Start with Option 2 (backward compatible), add warnings, then migrate to Option 1.

## Migration Strategy

### Phase 1: Add Fields (Backward Compatible)
- Add `scope` and `ttl_seconds` fields with defaults
- Existing code continues to work
- New code can opt-in to scoping

### Phase 2: Encourage Scoping
- Add warnings for unscoped operations
- Provide migration tools
- Update documentation

### Phase 3: Enforce Scoping (Breaking Change)
- Make scope required
- Remove `Scope::Global` or restrict to admin only

### Data Migration

```rust
/// Migrate existing approvals to scoped format
pub fn migrate_to_scoped_storage(
    old_store: &ApprovalStore,
    new_store: &ApprovalStore,
    default_scope: Scope,
) -> Result<usize> {
    let mut migrated = 0;

    // Iterate all existing approvals
    for approval in old_store.list_all_approvals()? {
        // Add scope and re-insert
        let scoped_approval = approval.with_scope(default_scope.clone());
        new_store.grant_approval(scoped_approval)?;
        migrated += 1;
    }

    Ok(migrated)
}
```

## Usage Examples

### Example 1: Multi-Tenant SaaS

```rust
// Tenant A - Acme Corp
let acme_scope = Scope::tenant("acme-corp");

store.grant_approval(
    Approval::new("alice@acme.com", "document-1", "GET", "admin")
        .with_scope(acme_scope.clone())
        .with_ttl(24 * 3600) // 1 day
)?;

// Tenant B - Widgets Inc
let widgets_scope = Scope::tenant("widgets-inc");

store.grant_approval(
    Approval::new("bob@widgets.com", "document-1", "GET", "admin")
        .with_scope(widgets_scope.clone())
        .with_ttl(24 * 3600)
)?;

// Query isolated by tenant
let acme_approvals = store.list_approvals_in_scope("alice@acme.com", &acme_scope)?;
// Only sees Acme Corp approvals

let widgets_approvals = store.list_approvals_in_scope("bob@widgets.com", &widgets_scope)?;
// Only sees Widgets Inc approvals
```

### Example 2: Environment Separation

```rust
// Development environment - short TTL
let dev_scope = Scope::tenant_env("acme-corp", "dev");

store.grant_approval(
    Approval::new("test-user", "api-endpoint", "POST", "dev-admin")
        .with_scope(dev_scope)
        .with_ttl(3600) // 1 hour - short-lived for testing
)?;

// Production environment - long TTL
let prod_scope = Scope::tenant_env("acme-corp", "prod");

store.grant_approval(
    Approval::new("service-account", "api-endpoint", "POST", "prod-admin")
        .with_scope(prod_scope)
        .with_ttl(30 * 24 * 3600) // 30 days
)?;
```

### Example 3: Automatic Cleanup

```rust
// Open store with TTL compaction filter
let store = ApprovalStore::new_with_ttl(
    "/var/lib/ipe/approvals.db",
    TTLConfig::short_lived() // 1 day default TTL
)?;

// Grant temporary approval
store.grant_approval(
    Approval::new("temp-user", "resource", "GET", "admin")
        .with_scope(Scope::tenant("acme-corp"))
        .with_ttl(3600) // 1 hour
)?;

// After 1 hour + next compaction:
// Record is automatically deleted
// No manual cleanup needed
```

### Example 4: Scoped Relationships

```rust
let tenant_scope = Scope::tenant("acme-corp");

// Alice is editor in Acme Corp tenant
rel_store.add_relationship(
    Relationship::role("alice", "editor", "document-123", "admin")
        .with_scope(tenant_scope.clone())
        .with_ttl(30 * 24 * 3600) // 30 days
)?;

// Bob is editor in different tenant
let other_scope = Scope::tenant("widgets-inc");
rel_store.add_relationship(
    Relationship::role("bob", "editor", "document-123", "admin")
        .with_scope(other_scope)
        .with_ttl(30 * 24 * 3600)
)?;

// alice and bob can both be "editor" of "document-123"
// but in different tenants (isolated by scope)
```

## Performance Considerations

### Storage Overhead
- Scope adds ~10-50 bytes per key (tenant/env strings)
- TTL uses existing timestamp field
- Minimal overhead: ~2-5% storage increase

### Query Performance
- Prefix scanning by scope: O(log n + k) where k = results
- Compaction filter: No query-time overhead
- TTL cleanup during compaction (background operation)

### Compaction Frequency
```rust
let mut opts = Options::default();
// Trigger compaction more frequently for aggressive TTL cleanup
opts.set_level_zero_file_num_compaction_trigger(4); // default: 4
opts.set_max_background_jobs(4); // parallelcompaction
```

## Security Considerations

### Scope Isolation
- ✅ Prevents cross-tenant data leaks
- ✅ Enforced at storage key level
- ⚠️ Application must validate scope matches caller
- ⚠️ No mixing of scopes in same query

### TTL Security
- ✅ Automatic cleanup of temporary access
- ✅ Reduces attack surface over time
- ⚠️ Ensure TTL cannot be extended without authorization
- ⚠️ Audit TTL extensions

### Scope Validation
```rust
/// Validate that caller has access to scope
pub fn validate_scope_access(
    caller_scope: &Scope,
    requested_scope: &Scope,
) -> Result<()> {
    match (caller_scope, requested_scope) {
        // Global can access anything
        (Scope::Global, _) => Ok(()),

        // Tenant can only access own tenant
        (Scope::Tenant(t1), Scope::Tenant(t2)) if t1 == t2 => Ok(()),

        // Tenant can access own tenant+env
        (Scope::Tenant(t1), Scope::TenantEnvironment { tenant: t2, .. })
            if t1 == t2 => Ok(()),

        _ => Err(Error::UnauthorizedScopeAccess),
    }
}
```

## Open Questions

1. **RocksDB TTL API**: RocksDB has `DB::open_with_ttl()` but it's global per DB. How to handle different TTLs per scope?
   - **Answer**: Use compaction filter with per-record TTL stored in value

2. **Scope hierarchy**: Should `Tenant` scope inherit access to `TenantEnvironment` scopes?
   - **Answer**: Yes, but with explicit validation

3. **TTL extension**: Should TTL be extendable after creation?
   - **Answer**: Yes, but requires re-writing the record (update operation)

4. **Compaction trigger**: How often should compaction run for TTL cleanup?
   - **Answer**: Configurable, default every 1 hour or on size threshold

## Success Metrics

1. **Scope Isolation**: 100% of queries respect scope boundaries
2. **TTL Cleanup**: Expired records removed within 2x compaction interval
3. **Performance**: <5% query overhead with scoping
4. **Storage**: <5% storage overhead with scope metadata

## Future Enhancements

1. **Scope wildcards**: Query multiple scopes (e.g., all environments for a tenant)
2. **TTL policies**: Different TTL defaults per scope
3. **Audit log**: Track all scope/TTL changes
4. **TTL refresh**: Automatic TTL extension on access
5. **Hierarchical scopes**: Parent/child scope relationships

## References

- [RocksDB TTL Documentation](https://github.com/facebook/rocksdb/wiki/Time-to-Live)
- [Multi-tenancy Patterns](https://docs.microsoft.com/en-us/azure/architecture/guide/multitenant/overview)
