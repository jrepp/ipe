//! Relationship storage and retrieval for trust chains and role assignments
//!
//! This module provides RocksDB-backed storage for relationship records,
//! enabling efficient validation of direct relationships (e.g., "is editor")
//! and transitive trust chains (e.g., "is trusted through root CA").

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

// Import Scope from approval module
use crate::approval::Scope;

#[derive(Error, Debug)]
pub enum RelationshipError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Relationship not found: {subject}:{relation}:{object}")]
    NotFound { subject: String, relation: String, object: String },

    #[error("Invalid relationship: {0}")]
    InvalidRelationship(String),

    #[error("Cycle detected in relationship chain")]
    CycleDetected,

    #[error("Maximum traversal depth exceeded: {0}")]
    MaxDepthExceeded(usize),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, RelationshipError>;

/// Relationship type - defines the semantic meaning of the relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    /// Direct role assignment (e.g., "alice" is "editor" of "document")
    Role,

    /// Trust relationship (e.g., "cert-1" is "trusted_by" "root-ca")
    Trust,

    /// Membership (e.g., "alice" is "member_of" "admin-group")
    Membership,

    /// Ownership (e.g., "alice" is "owner" of "resource")
    Ownership,

    /// Delegation (e.g., "alice" can "delegate_to" "bob")
    Delegation,

    /// Custom relationship type
    Custom(String),
}

impl RelationType {
    /// Check if this relationship type is transitive
    /// Transitive relations can be chained (A -> B, B -> C implies A -> C)
    pub fn is_transitive(&self) -> bool {
        matches!(self, RelationType::Trust | RelationType::Membership)
    }
}

impl std::fmt::Display for RelationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RelationType::Role => write!(f, "role"),
            RelationType::Trust => write!(f, "trust"),
            RelationType::Membership => write!(f, "membership"),
            RelationType::Ownership => write!(f, "ownership"),
            RelationType::Delegation => write!(f, "delegation"),
            RelationType::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Relationship record representing a connection between two entities
///
/// Examples:
/// - Subject: "alice", Relation: "editor", Object: "document-123" (role)
/// - Subject: "cert-1", Relation: "trusted_by", Object: "root-ca" (trust)
/// - Subject: "alice", Relation: "member_of", Object: "admin-group" (membership)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    /// Subject entity (who/what has the relationship)
    pub subject: String,

    /// Relation type (what kind of relationship)
    pub relation: String,

    /// Object entity (to whom/what the relationship applies)
    pub object: String,

    /// Relationship type categorization
    pub relation_type: RelationType,

    /// Who/what established this relationship
    pub created_by: String,

    /// When this relationship was created (Unix timestamp)
    pub created_at: i64,

    /// Optional expiration time (Unix timestamp)
    pub expires_at: Option<i64>,

    /// Additional context (e.g., certificate chain, delegation scope)
    pub metadata: HashMap<String, String>,

    /// Scope for multi-tenant isolation
    #[serde(default)]
    pub scope: Scope,

    /// TTL in seconds for automatic cleanup
    pub ttl_seconds: Option<i64>,
}

impl Relationship {
    /// Create a new relationship
    pub fn new(
        subject: impl Into<String>,
        relation: impl Into<String>,
        object: impl Into<String>,
        relation_type: RelationType,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            relation: relation.into(),
            object: object.into(),
            relation_type,
            created_by: created_by.into(),
            created_at: Utc::now().timestamp(),
            expires_at: None,
            metadata: HashMap::new(),
            scope: Scope::Global,
            ttl_seconds: None,
        }
    }

    /// Create a role relationship (e.g., "alice" is "editor" of "document")
    pub fn role(
        subject: impl Into<String>,
        role: impl Into<String>,
        object: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self::new(subject, role, object, RelationType::Role, created_by)
    }

    /// Create a trust relationship (e.g., "cert" is "trusted_by" "root-ca")
    pub fn trust(
        subject: impl Into<String>,
        object: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self::new(subject, "trusted_by", object, RelationType::Trust, created_by)
    }

    /// Create a membership relationship (e.g., "alice" is "member_of" "group")
    pub fn membership(
        subject: impl Into<String>,
        object: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self::new(subject, "member_of", object, RelationType::Membership, created_by)
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

    /// Check if relationship is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            Utc::now().timestamp() >= expires_at
        } else {
            false
        }
    }

    /// Generate scoped storage key for direct lookup
    fn key(&self) -> String {
        format!(
            "relationships:{}:{}:{}:{}",
            self.scope.encode(),
            self.subject,
            self.relation,
            self.object
        )
    }

    /// Generate forward index key (subject -> relations)
    #[allow(dead_code)]
    fn forward_index_key(&self) -> String {
        format!("rel_fwd:{}:{}:{}", self.scope.encode(), self.subject, self.relation)
    }

    /// Generate reverse index key (object <- relations)
    #[allow(dead_code)]
    fn reverse_index_key(&self) -> String {
        format!("rel_rev:{}:{}:{}", self.scope.encode(), self.object, self.relation)
    }
}

/// Query for checking multiple relationships in a batch
#[derive(Debug, Clone)]
pub struct RelationshipQuery {
    pub subject: String,
    pub relation: String,
    pub object: String,
}

impl RelationshipQuery {
    pub fn new(
        subject: impl Into<String>,
        relation: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            relation: relation.into(),
            object: object.into(),
        }
    }
}

/// Result of a transitive relationship check
#[derive(Debug, Clone)]
pub struct RelationshipPath {
    /// The chain of relationships that connect subject to object
    pub path: Vec<Relationship>,

    /// Total depth of the chain
    pub depth: usize,
}

#[cfg(feature = "approvals")]
mod rocksdb_impl {
    use super::*;
    use rocksdb::{ColumnFamilyDescriptor, Options, DB};

    /// Database context for relationship storage and retrieval
    #[derive(Debug)]
    pub struct RelationshipStore {
        db: Arc<DB>,
        #[allow(dead_code)]
        temp_dir: Option<tempfile::TempDir>,
        /// Maximum depth for transitive relationship traversal (prevent infinite loops)
        max_traversal_depth: usize,
    }

    impl RelationshipStore {
        /// Column family name for relationships
        const CF_RELATIONSHIPS: &'static str = "relationships";

        /// Default maximum traversal depth
        const DEFAULT_MAX_DEPTH: usize = 10;

        /// Create new store at the given path (for production)
        pub fn new(path: impl AsRef<Path>) -> Result<Self> {
            let path = path.as_ref();
            let db = Self::open_db(path)?;

            Ok(Self {
                db: Arc::new(db),
                temp_dir: None,
                max_traversal_depth: Self::DEFAULT_MAX_DEPTH,
            })
        }

        /// Create temporary store for testing
        pub fn new_temp() -> Result<Self> {
            let temp_dir = tempfile::tempdir()?;
            let db = Self::open_db(temp_dir.path())?;

            Ok(Self {
                db: Arc::new(db),
                temp_dir: Some(temp_dir),
                max_traversal_depth: Self::DEFAULT_MAX_DEPTH,
            })
        }

        /// Set maximum traversal depth
        pub fn with_max_depth(mut self, depth: usize) -> Self {
            self.max_traversal_depth = depth;
            self
        }

        /// Open database with column families
        fn open_db(path: &Path) -> Result<DB> {
            let mut opts = Options::default();
            opts.create_if_missing(true);
            opts.create_missing_column_families(true);
            opts.set_prefix_extractor(rocksdb::SliceTransform::create_fixed_prefix(20));

            // Column family for relationships
            let mut rel_opts = Options::default();
            rel_opts.optimize_for_point_lookup(64);

            let cfs = vec![ColumnFamilyDescriptor::new(Self::CF_RELATIONSHIPS, rel_opts)];

            DB::open_cf_descriptors(&opts, path, cfs)
                .map_err(|e| RelationshipError::DatabaseError(e.to_string()))
        }

        /// Get column family handle
        fn cf_relationships(&self) -> Result<&rocksdb::ColumnFamily> {
            self.db.cf_handle(Self::CF_RELATIONSHIPS).ok_or_else(|| {
                RelationshipError::DatabaseError("Relationships CF not found".into())
            })
        }

        /// Add a relationship (privileged operation)
        pub fn add_relationship(&self, relationship: Relationship) -> Result<()> {
            if relationship.subject.is_empty() {
                return Err(RelationshipError::InvalidRelationship(
                    "subject cannot be empty".into(),
                ));
            }
            if relationship.relation.is_empty() {
                return Err(RelationshipError::InvalidRelationship(
                    "relation cannot be empty".into(),
                ));
            }
            if relationship.object.is_empty() {
                return Err(RelationshipError::InvalidRelationship(
                    "object cannot be empty".into(),
                ));
            }

            let key = relationship.key();
            let value = serde_json::to_vec(&relationship)?;
            let cf = self.cf_relationships()?;

            self.db
                .put_cf(cf, key.as_bytes(), &value)
                .map_err(|e| RelationshipError::DatabaseError(e.to_string()))
        }

        /// Check if a direct relationship exists (not transitive)
        /// Defaults to Global scope for backward compatibility
        pub fn has_relationship(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
        ) -> Result<bool> {
            self.has_relationship_in_scope(subject, relation, object, &Scope::Global)
        }

        /// Check if a direct relationship exists in specific scope
        pub fn has_relationship_in_scope(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
            scope: &Scope,
        ) -> Result<bool> {
            match self.get_relationship_in_scope(subject, relation, object, scope) {
                Ok(Some(rel)) => Ok(!rel.is_expired()),
                Ok(None) => Ok(false),
                Err(RelationshipError::NotFound { .. }) => Ok(false),
                Err(e) => Err(e),
            }
        }

        /// Get a specific relationship
        /// Defaults to Global scope for backward compatibility
        pub fn get_relationship(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
        ) -> Result<Option<Relationship>> {
            self.get_relationship_in_scope(subject, relation, object, &Scope::Global)
        }

        /// Get a specific relationship in scope
        pub fn get_relationship_in_scope(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
            scope: &Scope,
        ) -> Result<Option<Relationship>> {
            let key =
                format!("relationships:{}:{}:{}:{}", scope.encode(), subject, relation, object);
            let cf = self.cf_relationships()?;

            match self.db.get_cf(cf, key.as_bytes()) {
                Ok(Some(value)) => {
                    let relationship: Relationship = serde_json::from_slice(&value)?;
                    Ok(Some(relationship))
                },
                Ok(None) => Ok(None),
                Err(e) => Err(RelationshipError::DatabaseError(e.to_string())),
            }
        }

        /// Remove a relationship
        /// Defaults to Global scope for backward compatibility
        pub fn remove_relationship(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
        ) -> Result<()> {
            self.remove_relationship_in_scope(subject, relation, object, &Scope::Global)
        }

        /// Remove a relationship in specific scope
        pub fn remove_relationship_in_scope(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
            scope: &Scope,
        ) -> Result<()> {
            let key =
                format!("relationships:{}:{}:{}:{}", scope.encode(), subject, relation, object);
            let cf = self.cf_relationships()?;

            self.db
                .delete_cf(cf, key.as_bytes())
                .map_err(|e| RelationshipError::DatabaseError(e.to_string()))
        }

        /// Check if a relationship exists, considering transitive relationships
        ///
        /// For example, if:
        /// - "cert-1" is "trusted_by" "intermediate-ca"
        /// - "intermediate-ca" is "trusted_by" "root-ca"
        ///
        /// Then has_transitive_relationship("cert-1", "trusted_by", "root-ca") returns true
        pub fn has_transitive_relationship(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
        ) -> Result<bool> {
            // First check direct relationship
            if self.has_relationship(subject, relation, object)? {
                return Ok(true);
            }

            // If not direct, try transitive search
            self.find_relationship_path(subject, relation, object)
                .map(|path| path.is_some())
        }

        /// Find a path of relationships connecting subject to object
        /// Uses breadth-first search to find shortest path
        pub fn find_relationship_path(
            &self,
            subject: &str,
            relation: &str,
            object: &str,
        ) -> Result<Option<RelationshipPath>> {
            // BFS to find path
            let mut queue: VecDeque<(String, Vec<Relationship>)> = VecDeque::new();
            let mut visited: HashSet<String> = HashSet::new();

            queue.push_back((subject.to_string(), Vec::new()));
            visited.insert(subject.to_string());

            while let Some((current, path)) = queue.pop_front() {
                if path.len() >= self.max_traversal_depth {
                    return Err(RelationshipError::MaxDepthExceeded(self.max_traversal_depth));
                }

                // Get all outgoing relationships from current node
                let outgoing = self.get_outgoing_relationships(&current, relation)?;

                for rel in outgoing {
                    if rel.is_expired() {
                        continue;
                    }

                    // Check if we reached the target
                    if rel.object == object {
                        let mut final_path = path.clone();
                        final_path.push(rel);
                        return Ok(Some(RelationshipPath {
                            depth: final_path.len(),
                            path: final_path,
                        }));
                    }

                    // Continue searching if transitive
                    if rel.relation_type.is_transitive() && !visited.contains(&rel.object) {
                        visited.insert(rel.object.clone());
                        let mut new_path = path.clone();
                        new_path.push(rel.clone());
                        queue.push_back((rel.object.clone(), new_path));
                    }
                }
            }

            Ok(None)
        }

        /// Get all outgoing relationships from a subject with a specific relation
        fn get_outgoing_relationships(
            &self,
            subject: &str,
            relation: &str,
        ) -> Result<Vec<Relationship>> {
            // NOTE: This searches across ALL scopes for transitive traversal
            // For a more restricted version, use get_outgoing_relationships_in_scope
            let prefix = "relationships:".to_string();
            let cf = self.cf_relationships()?;

            let mut relationships = Vec::new();
            let mut iter = self.db.raw_iterator_cf(cf);
            iter.seek(prefix.as_bytes());

            while iter.valid() {
                if let Some(key) = iter.key() {
                    if let Ok(key_str) = std::str::from_utf8(key) {
                        if !key_str.starts_with(&prefix) {
                            break;
                        }

                        if let Some(value) = iter.value() {
                            if let Ok(relationship) = serde_json::from_slice::<Relationship>(value)
                            {
                                // Filter by subject and relation
                                if relationship.subject == subject
                                    && relationship.relation == relation
                                {
                                    relationships.push(relationship);
                                }
                            }
                        }
                    }
                }
                iter.next();
            }

            Ok(relationships)
        }

        /// List all relationships for a subject
        /// Defaults to Global scope for backward compatibility
        pub fn list_subject_relationships(&self, subject: &str) -> Result<Vec<Relationship>> {
            self.list_subject_relationships_in_scope(subject, &Scope::Global)
        }

        /// List all relationships for a subject in specific scope
        pub fn list_subject_relationships_in_scope(
            &self,
            subject: &str,
            scope: &Scope,
        ) -> Result<Vec<Relationship>> {
            let prefix = format!("relationships:{}:{}:", scope.encode(), subject);
            let cf = self.cf_relationships()?;

            let mut relationships = Vec::new();
            let mut iter = self.db.raw_iterator_cf(cf);
            iter.seek(prefix.as_bytes());

            while iter.valid() {
                if let Some(key) = iter.key() {
                    if let Ok(key_str) = std::str::from_utf8(key) {
                        if !key_str.starts_with(&prefix) {
                            break;
                        }

                        if let Some(value) = iter.value() {
                            if let Ok(relationship) = serde_json::from_slice::<Relationship>(value)
                            {
                                relationships.push(relationship);
                            }
                        }
                    }
                }
                iter.next();
            }

            Ok(relationships)
        }

        /// Batch check relationships
        pub fn check_relationships(&self, queries: Vec<RelationshipQuery>) -> Result<Vec<bool>> {
            queries
                .iter()
                .map(|q| self.has_relationship(&q.subject, &q.relation, &q.object))
                .collect()
        }

        /// Count total relationships
        pub fn count_relationships(&self) -> Result<usize> {
            let cf = self.cf_relationships()?;
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
pub use rocksdb_impl::RelationshipStore;

#[cfg(test)]
#[cfg(feature = "approvals")]
mod tests {
    use super::*;

    #[test]
    fn test_relationship_creation() {
        let rel = Relationship::role("alice", "editor", "document-123", "admin");

        assert_eq!(rel.subject, "alice");
        assert_eq!(rel.relation, "editor");
        assert_eq!(rel.object, "document-123");
        assert_eq!(rel.relation_type, RelationType::Role);
        assert_eq!(rel.created_by, "admin");
        assert!(!rel.is_expired());
    }

    #[test]
    fn test_trust_relationship() {
        let rel = Relationship::trust("cert-1", "root-ca", "pki-system");

        assert_eq!(rel.subject, "cert-1");
        assert_eq!(rel.relation, "trusted_by");
        assert_eq!(rel.object, "root-ca");
        assert_eq!(rel.relation_type, RelationType::Trust);
    }

    #[test]
    fn test_relationship_with_expiration() {
        let rel = Relationship::role("alice", "editor", "document", "admin").with_expiration(3600);

        assert!(rel.expires_at.is_some());
        assert!(!rel.is_expired());
    }

    #[test]
    fn test_store_add_and_get_relationship() {
        let store = RelationshipStore::new_temp().unwrap();
        let rel = Relationship::role("alice", "editor", "document-123", "admin");

        store.add_relationship(rel.clone()).unwrap();

        let retrieved = store
            .get_relationship("alice", "editor", "document-123")
            .unwrap()
            .expect("Relationship should exist");

        assert_eq!(retrieved.subject, rel.subject);
        assert_eq!(retrieved.relation, rel.relation);
        assert_eq!(retrieved.object, rel.object);
    }

    #[test]
    fn test_store_has_relationship() {
        let store = RelationshipStore::new_temp().unwrap();
        let rel = Relationship::role("alice", "editor", "document-123", "admin");

        store.add_relationship(rel).unwrap();

        assert!(store.has_relationship("alice", "editor", "document-123").unwrap());
        assert!(!store.has_relationship("bob", "editor", "document-123").unwrap());
    }

    #[test]
    fn test_store_remove_relationship() {
        let store = RelationshipStore::new_temp().unwrap();
        let rel = Relationship::role("alice", "editor", "document-123", "admin");

        store.add_relationship(rel).unwrap();
        assert!(store.has_relationship("alice", "editor", "document-123").unwrap());

        store.remove_relationship("alice", "editor", "document-123").unwrap();
        assert!(!store.has_relationship("alice", "editor", "document-123").unwrap());
    }

    #[test]
    fn test_transitive_trust_chain() {
        let store = RelationshipStore::new_temp().unwrap();

        // Build trust chain: cert-1 -> intermediate-ca -> root-ca
        store
            .add_relationship(Relationship::trust("cert-1", "intermediate-ca", "pki"))
            .unwrap();

        store
            .add_relationship(Relationship::trust("intermediate-ca", "root-ca", "pki"))
            .unwrap();

        // Direct relationship exists
        assert!(store.has_relationship("cert-1", "trusted_by", "intermediate-ca").unwrap());

        // Transitive relationship should be found
        assert!(store.has_transitive_relationship("cert-1", "trusted_by", "root-ca").unwrap());

        // No relationship to unrelated entity
        assert!(!store.has_transitive_relationship("cert-1", "trusted_by", "other-ca").unwrap());
    }

    #[test]
    fn test_transitive_membership() {
        let store = RelationshipStore::new_temp().unwrap();

        // alice -> engineers -> employees
        store
            .add_relationship(Relationship::membership("alice", "engineers", "system"))
            .unwrap();

        store
            .add_relationship(Relationship::membership("engineers", "employees", "system"))
            .unwrap();

        assert!(store.has_transitive_relationship("alice", "member_of", "employees").unwrap());
    }

    #[test]
    fn test_relationship_path() {
        let store = RelationshipStore::new_temp().unwrap();

        // Build chain
        store
            .add_relationship(Relationship::trust("cert-1", "intermediate", "pki"))
            .unwrap();
        store
            .add_relationship(Relationship::trust("intermediate", "root", "pki"))
            .unwrap();

        let path = store
            .find_relationship_path("cert-1", "trusted_by", "root")
            .unwrap()
            .expect("Path should exist");

        assert_eq!(path.depth, 2);
        assert_eq!(path.path.len(), 2);
        assert_eq!(path.path[0].subject, "cert-1");
        assert_eq!(path.path[0].object, "intermediate");
        assert_eq!(path.path[1].subject, "intermediate");
        assert_eq!(path.path[1].object, "root");
    }

    #[test]
    fn test_max_depth_limit() {
        let store = RelationshipStore::new_temp().unwrap().with_max_depth(3);

        // Build long chain
        for i in 0..10 {
            store
                .add_relationship(Relationship::trust(
                    format!("node-{}", i),
                    format!("node-{}", i + 1),
                    "system",
                ))
                .unwrap();
        }

        // Should fail due to max depth
        let result = store.find_relationship_path("node-0", "trusted_by", "node-10");
        assert!(matches!(result, Err(RelationshipError::MaxDepthExceeded(_))));
    }
}
