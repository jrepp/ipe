//! Approval storage and retrieval for authorization decisions
//!
//! This module provides RocksDB-backed storage for approval records,
//! enabling efficient lookup and set membership tests for authorization.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

/// Scope defines the isolation boundary for data
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Scope {
    /// Global scope - accessible across all tenants (use sparingly)
    #[default]
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
    pub fn tenant(tenant: impl Into<String>) -> Self {
        Scope::Tenant(tenant.into())
    }

    pub fn env(environment: impl Into<String>) -> Self {
        Scope::Environment(environment.into())
    }

    pub fn tenant_env(tenant: impl Into<String>, environment: impl Into<String>) -> Self {
        Scope::TenantEnvironment {
            tenant: tenant.into(),
            environment: environment.into(),
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
            },
            Scope::Custom(parts) => format!("custom:{}", parts.join(":")),
        }
    }
}

/// TTL configuration for automatic cleanup
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TTLConfig {
    pub default_ttl_seconds: Option<i64>,
    pub min_ttl_seconds: i64,
    pub max_ttl_seconds: i64,
    pub enforce_ttl: bool,
}

impl Default for TTLConfig {
    fn default() -> Self {
        Self {
            default_ttl_seconds: None,
            min_ttl_seconds: 60,
            max_ttl_seconds: 365 * 24 * 3600,
            enforce_ttl: true,
        }
    }
}

impl TTLConfig {
    pub fn temporary() -> Self {
        Self {
            default_ttl_seconds: Some(3600),
            min_ttl_seconds: 60,
            max_ttl_seconds: 24 * 3600,
            enforce_ttl: true,
        }
    }

    pub fn short_lived() -> Self {
        Self {
            default_ttl_seconds: Some(24 * 3600),
            min_ttl_seconds: 3600,
            max_ttl_seconds: 7 * 24 * 3600,
            enforce_ttl: true,
        }
    }

    pub fn long_lived() -> Self {
        Self {
            default_ttl_seconds: Some(30 * 24 * 3600),
            min_ttl_seconds: 24 * 3600,
            max_ttl_seconds: 365 * 24 * 3600,
            enforce_ttl: true,
        }
    }
}

#[derive(Error, Debug)]
pub enum ApprovalError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Approval not found: {identity}:{resource}:{action}")]
    NotFound { identity: String, resource: String, action: String },

    #[error("Approval expired at {expired_at}")]
    Expired { expired_at: DateTime<Utc> },

    #[error("Invalid approval: {0}")]
    InvalidApproval(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ApprovalError>;

/// Approval record representing authorization granted by a privileged entity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Approval {
    /// Principal identifier (service account, bot ID, user)
    pub identity: String,

    /// URL or resource pattern
    pub resource: String,

    /// HTTP method or action type
    pub action: String,

    /// Privileged identity that granted this approval
    pub granted_by: String,

    /// When this approval was granted (Unix timestamp)
    pub granted_at: i64,

    /// Optional expiration time (Unix timestamp)
    pub expires_at: Option<i64>,

    /// Additional context (e.g., justification, ticket ID)
    pub metadata: HashMap<String, String>,

    /// Scope for multi-tenant isolation
    #[serde(default)]
    pub scope: Scope,

    /// TTL in seconds for automatic cleanup
    pub ttl_seconds: Option<i64>,
}

impl Approval {
    /// Create a new approval
    pub fn new(
        identity: impl Into<String>,
        resource: impl Into<String>,
        action: impl Into<String>,
        granted_by: impl Into<String>,
    ) -> Self {
        Self {
            identity: identity.into(),
            resource: resource.into(),
            action: action.into(),
            granted_by: granted_by.into(),
            granted_at: Utc::now().timestamp(),
            expires_at: None,
            metadata: HashMap::new(),
            scope: Scope::Global,
            ttl_seconds: None,
        }
    }

    /// Set scope
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Set TTL (also sets expires_at)
    pub fn with_ttl(mut self, ttl_seconds: i64) -> Self {
        self.ttl_seconds = Some(ttl_seconds);
        self.expires_at = Some(Utc::now().timestamp() + ttl_seconds);
        self
    }

    /// Set expiration time (seconds from now)
    pub fn with_expiration(mut self, seconds: i64) -> Self {
        self.expires_at = Some(Utc::now().timestamp() + seconds);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Check if approval is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now().timestamp() >= expires_at
        } else {
            false
        }
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

/// Request for checking multiple approvals in a batch
#[derive(Debug, Clone)]
pub struct ApprovalCheck {
    pub identity: String,
    pub resource: String,
    pub action: String,
}

impl ApprovalCheck {
    pub fn new(
        identity: impl Into<String>,
        resource: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        Self {
            identity: identity.into(),
            resource: resource.into(),
            action: action.into(),
        }
    }
}

#[cfg(feature = "approvals")]
mod rocksdb_impl {
    use super::*;
    use rocksdb::{ColumnFamilyDescriptor, Options, DB};

    /// Database context for approval storage and retrieval
    #[derive(Debug)]
    pub struct ApprovalStore {
        db: Arc<DB>,
        #[allow(dead_code)]
        temp_dir: Option<tempfile::TempDir>,
    }

    impl ApprovalStore {
        /// Column family names
        const CF_APPROVALS: &'static str = "approvals";
        const CF_POLICIES: &'static str = "policies";
        const CF_AUDIT: &'static str = "audit";

        /// Create new store at the given path (for production)
        pub fn new(path: impl AsRef<Path>) -> Result<Self> {
            let path = path.as_ref();
            let db = Self::open_db(path)?;

            Ok(Self { db: Arc::new(db), temp_dir: None })
        }

        /// Create temporary store for testing
        pub fn new_temp() -> Result<Self> {
            let temp_dir = tempfile::tempdir()?;
            let db = Self::open_db(temp_dir.path())?;

            Ok(Self {
                db: Arc::new(db),
                temp_dir: Some(temp_dir),
            })
        }

        /// Open database with column families
        fn open_db(path: &Path) -> Result<DB> {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);

            // Optimizations
            opts.set_prefix_extractor(rocksdb::SliceTransform::create_fixed_prefix(20));

            // Column family for approvals
            let mut approval_opts = Options::default();
            approval_opts.optimize_for_point_lookup(64); // 64MB block cache

            // Column families
            let cfs = vec![
                ColumnFamilyDescriptor::new(Self::CF_APPROVALS, approval_opts),
                ColumnFamilyDescriptor::new(Self::CF_POLICIES, Options::default()),
                ColumnFamilyDescriptor::new(Self::CF_AUDIT, Options::default()),
            ];

            DB::open_cf_descriptors(&opts, path, cfs)
                .map_err(|e| ApprovalError::DatabaseError(e.to_string()))
        }

        /// Get column family handle
        fn cf_approvals(&self) -> Result<&rocksdb::ColumnFamily> {
            self.db
                .cf_handle(Self::CF_APPROVALS)
                .ok_or_else(|| ApprovalError::DatabaseError("Approvals CF not found".into()))
        }

        /// Write approval (privileged operation - requires authorization)
        pub fn grant_approval(&self, approval: Approval) -> Result<()> {
            if approval.identity.is_empty() {
                return Err(ApprovalError::InvalidApproval("identity cannot be empty".into()));
            }
            if approval.resource.is_empty() {
                return Err(ApprovalError::InvalidApproval("resource cannot be empty".into()));
            }
            if approval.action.is_empty() {
                return Err(ApprovalError::InvalidApproval("action cannot be empty".into()));
            }

            let key = approval.key();
            let value = serde_json::to_vec(&approval)?;
            let cf = self.cf_approvals()?;

            self.db
                .put_cf(cf, key.as_bytes(), &value)
                .map_err(|e| ApprovalError::DatabaseError(e.to_string()))
        }

        /// Check if approval exists and is valid (not expired)
        /// Defaults to Global scope for backward compatibility
        pub fn has_approval(&self, identity: &str, resource: &str, action: &str) -> Result<bool> {
            self.has_approval_in_scope(identity, resource, action, &Scope::Global)
        }

        /// Check if approval exists in specific scope
        pub fn has_approval_in_scope(
            &self,
            identity: &str,
            resource: &str,
            action: &str,
            scope: &Scope,
        ) -> Result<bool> {
            match self.get_approval_in_scope(identity, resource, action, scope) {
                Ok(Some(approval)) => Ok(!approval.is_expired()),
                Ok(None) => Ok(false),
                Err(ApprovalError::NotFound { .. }) => Ok(false),
                Err(e) => Err(e),
            }
        }

        /// Get approval details
        /// Defaults to Global scope for backward compatibility
        pub fn get_approval(
            &self,
            identity: &str,
            resource: &str,
            action: &str,
        ) -> Result<Option<Approval>> {
            self.get_approval_in_scope(identity, resource, action, &Scope::Global)
        }

        /// Get approval details in specific scope
        pub fn get_approval_in_scope(
            &self,
            identity: &str,
            resource: &str,
            action: &str,
            scope: &Scope,
        ) -> Result<Option<Approval>> {
            let key = format!("approvals:{}:{}:{}:{}", scope.encode(), identity, resource, action);
            let cf = self.cf_approvals()?;

            match self.db.get_cf(cf, key.as_bytes()) {
                Ok(Some(value)) => {
                    let approval: Approval = serde_json::from_slice(&value)?;
                    Ok(Some(approval))
                },
                Ok(None) => Ok(None),
                Err(e) => Err(ApprovalError::DatabaseError(e.to_string())),
            }
        }

        /// Revoke approval (delete from database)
        /// Defaults to Global scope for backward compatibility
        pub fn revoke_approval(&self, identity: &str, resource: &str, action: &str) -> Result<()> {
            self.revoke_approval_in_scope(identity, resource, action, &Scope::Global)
        }

        /// Revoke approval in specific scope
        pub fn revoke_approval_in_scope(
            &self,
            identity: &str,
            resource: &str,
            action: &str,
            scope: &Scope,
        ) -> Result<()> {
            let key = format!("approvals:{}:{}:{}:{}", scope.encode(), identity, resource, action);
            let cf = self.cf_approvals()?;

            self.db
                .delete_cf(cf, key.as_bytes())
                .map_err(|e| ApprovalError::DatabaseError(e.to_string()))
        }

        /// Check set membership: is identity in approved set for resource?
        /// Uses prefix scanning for efficient lookup
        /// Defaults to Global scope for backward compatibility
        pub fn is_in_approved_set(&self, identity: &str, resource_pattern: &str) -> Result<bool> {
            self.is_in_approved_set_in_scope(identity, resource_pattern, &Scope::Global)
        }

        /// Check set membership in specific scope
        pub fn is_in_approved_set_in_scope(
            &self,
            identity: &str,
            resource_pattern: &str,
            scope: &Scope,
        ) -> Result<bool> {
            let prefix = format!("approvals:{}:{}:{}", scope.encode(), identity, resource_pattern);
            let cf = self.cf_approvals()?;

            let mut iter = self.db.raw_iterator_cf(cf);
            iter.seek(prefix.as_bytes());

            if iter.valid() {
                if let Some(key) = iter.key() {
                    if let Ok(key_str) = std::str::from_utf8(key) {
                        // Check if key starts with our prefix
                        if key_str.starts_with(&prefix) {
                            // Found a match - check if it's expired
                            if let Some(value) = iter.value() {
                                if let Ok(approval) = serde_json::from_slice::<Approval>(value) {
                                    return Ok(!approval.is_expired());
                                }
                            }
                        }
                    }
                }
            }

            Ok(false)
        }

        /// Batch check for efficiency
        pub fn check_approvals(&self, checks: Vec<ApprovalCheck>) -> Result<Vec<bool>> {
            checks
                .iter()
                .map(|check| self.has_approval(&check.identity, &check.resource, &check.action))
                .collect()
        }

        /// List all approvals for a given identity
        /// Defaults to Global scope for backward compatibility
        pub fn list_approvals(&self, identity: &str) -> Result<Vec<Approval>> {
            self.list_approvals_in_scope(identity, &Scope::Global)
        }

        /// List all approvals for a given identity in specific scope
        pub fn list_approvals_in_scope(
            &self,
            identity: &str,
            scope: &Scope,
        ) -> Result<Vec<Approval>> {
            let prefix = format!("approvals:{}:{}:", scope.encode(), identity);
            let cf = self.cf_approvals()?;

            let mut approvals = Vec::new();
            let mut iter = self.db.raw_iterator_cf(cf);
            iter.seek(prefix.as_bytes());

            while iter.valid() {
                if let Some(key) = iter.key() {
                    if let Ok(key_str) = std::str::from_utf8(key) {
                        if !key_str.starts_with(&prefix) {
                            break; // Moved past our prefix
                        }

                        if let Some(value) = iter.value() {
                            if let Ok(approval) = serde_json::from_slice::<Approval>(value) {
                                approvals.push(approval);
                            }
                        }
                    }
                }
                iter.next();
            }

            Ok(approvals)
        }

        /// Count total approvals
        pub fn count_approvals(&self) -> Result<usize> {
            let cf = self.cf_approvals()?;
            let mut count = 0;
            let mut iter = self.db.raw_iterator_cf(cf);
            iter.seek_to_first();

            while iter.valid() {
                count += 1;
                iter.next();
            }

            Ok(count)
        }
    }
}

#[cfg(feature = "approvals")]
pub use rocksdb_impl::ApprovalStore;

#[cfg(test)]
#[cfg(feature = "approvals")]
mod tests {
    use super::*;

    #[test]
    fn test_approval_creation() {
        let approval = Approval::new("bot-123", "https://api.example.com/data", "GET", "admin");

        assert_eq!(approval.identity, "bot-123");
        assert_eq!(approval.resource, "https://api.example.com/data");
        assert_eq!(approval.action, "GET");
        assert_eq!(approval.granted_by, "admin");
        assert!(approval.expires_at.is_none());
        assert!(!approval.is_expired());
    }

    #[test]
    fn test_approval_with_expiration() {
        let approval =
            Approval::new("bot-123", "resource", "action", "admin").with_expiration(3600);

        assert!(approval.expires_at.is_some());
        assert!(!approval.is_expired());
    }

    #[test]
    fn test_approval_expired() {
        let mut approval = Approval::new("bot-123", "resource", "action", "admin");
        approval.expires_at = Some(Utc::now().timestamp() - 100); // Expired 100s ago

        assert!(approval.is_expired());
    }

    #[test]
    fn test_approval_with_metadata() {
        let approval = Approval::new("bot-123", "resource", "action", "admin")
            .with_metadata("ticket", "JIRA-123")
            .with_metadata("justification", "automated testing");

        assert_eq!(approval.metadata.get("ticket").unwrap(), "JIRA-123");
        assert_eq!(approval.metadata.get("justification").unwrap(), "automated testing");
    }

    #[test]
    fn test_approval_key_generation() {
        let approval = Approval::new("bot-123", "https://api.example.com/data", "GET", "admin");
        let key = approval.key();

        // Now includes scope prefix (defaults to "global")
        assert_eq!(key, "approvals:global:bot-123:https://api.example.com/data:GET");
    }

    #[test]
    fn test_store_grant_and_get_approval() {
        let store = ApprovalStore::new_temp().unwrap();
        let approval = Approval::new("bot-123", "https://api.example.com/data", "GET", "admin");

        store.grant_approval(approval.clone()).unwrap();

        let retrieved = store
            .get_approval("bot-123", "https://api.example.com/data", "GET")
            .unwrap()
            .expect("Approval should exist");

        assert_eq!(retrieved.identity, approval.identity);
        assert_eq!(retrieved.resource, approval.resource);
        assert_eq!(retrieved.action, approval.action);
    }

    #[test]
    fn test_store_has_approval() {
        let store = ApprovalStore::new_temp().unwrap();
        let approval = Approval::new("bot-123", "https://api.example.com/data", "GET", "admin");

        store.grant_approval(approval).unwrap();

        assert!(store.has_approval("bot-123", "https://api.example.com/data", "GET").unwrap());
        assert!(!store.has_approval("bot-456", "https://api.example.com/data", "GET").unwrap());
    }

    #[test]
    fn test_store_revoke_approval() {
        let store = ApprovalStore::new_temp().unwrap();
        let approval = Approval::new("bot-123", "https://api.example.com/data", "GET", "admin");

        store.grant_approval(approval).unwrap();
        assert!(store.has_approval("bot-123", "https://api.example.com/data", "GET").unwrap());

        store.revoke_approval("bot-123", "https://api.example.com/data", "GET").unwrap();
        assert!(!store.has_approval("bot-123", "https://api.example.com/data", "GET").unwrap());
    }

    #[test]
    fn test_store_expired_approval() {
        let store = ApprovalStore::new_temp().unwrap();
        let mut approval = Approval::new("bot-123", "https://api.example.com/data", "GET", "admin");
        approval.expires_at = Some(Utc::now().timestamp() - 100);

        store.grant_approval(approval).unwrap();

        // Expired approval should return false for has_approval
        assert!(!store.has_approval("bot-123", "https://api.example.com/data", "GET").unwrap());
    }

    #[test]
    fn test_store_invalid_approval() {
        let store = ApprovalStore::new_temp().unwrap();

        let invalid = Approval::new("", "resource", "action", "admin");
        assert!(store.grant_approval(invalid).is_err());

        let invalid = Approval::new("bot-123", "", "action", "admin");
        assert!(store.grant_approval(invalid).is_err());

        let invalid = Approval::new("bot-123", "resource", "", "admin");
        assert!(store.grant_approval(invalid).is_err());
    }

    #[test]
    fn test_batch_approval_checks() {
        let store = ApprovalStore::new_temp().unwrap();

        store
            .grant_approval(Approval::new("bot-1", "resource-A", "GET", "admin"))
            .unwrap();
        store
            .grant_approval(Approval::new("bot-2", "resource-B", "POST", "admin"))
            .unwrap();

        let checks = vec![
            ApprovalCheck::new("bot-1", "resource-A", "GET"),
            ApprovalCheck::new("bot-2", "resource-B", "POST"),
            ApprovalCheck::new("bot-3", "resource-C", "DELETE"),
        ];

        let results = store.check_approvals(checks).unwrap();
        assert_eq!(results, vec![true, true, false]);
    }

    #[test]
    fn test_list_approvals() {
        let store = ApprovalStore::new_temp().unwrap();

        store
            .grant_approval(Approval::new("bot-123", "resource-A", "GET", "admin"))
            .unwrap();
        store
            .grant_approval(Approval::new("bot-123", "resource-B", "POST", "admin"))
            .unwrap();
        store
            .grant_approval(Approval::new("bot-456", "resource-C", "DELETE", "admin"))
            .unwrap();

        let approvals = store.list_approvals("bot-123").unwrap();
        assert_eq!(approvals.len(), 2);

        let approvals = store.list_approvals("bot-456").unwrap();
        assert_eq!(approvals.len(), 1);
    }

    #[test]
    fn test_count_approvals() {
        let store = ApprovalStore::new_temp().unwrap();

        assert_eq!(store.count_approvals().unwrap(), 0);

        store.grant_approval(Approval::new("bot-1", "res-A", "GET", "admin")).unwrap();
        assert_eq!(store.count_approvals().unwrap(), 1);

        store.grant_approval(Approval::new("bot-2", "res-B", "POST", "admin")).unwrap();
        assert_eq!(store.count_approvals().unwrap(), 2);
    }

    #[test]
    fn test_is_in_approved_set() {
        let store = ApprovalStore::new_temp().unwrap();

        for i in 1..=100 {
            store
                .grant_approval(Approval::new(
                    format!("bot-{}", i),
                    "https://api.example.com/data",
                    "GET",
                    "admin",
                ))
                .unwrap();
        }

        assert!(store.is_in_approved_set("bot-50", "https://api.example.com/data").unwrap());
        assert!(!store.is_in_approved_set("bot-999", "https://api.example.com/data").unwrap());
    }
}
