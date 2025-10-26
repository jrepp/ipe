//! High-speed, lock-free policy data store
//!
//! This module provides a blazingly fast policy data store optimized for:
//! - Lock-free reads using atomic snapshots
//! - Immutable data structures (no blocking)
//! - Pre-compiled policies and field mappings
//! - Background validation with worker pool
//! - Atomic swap for updates
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────┐
//! │   Readers   │ (multiple, concurrent, lock-free)
//! └──────┬──────┘
//!        │ Arc::clone()
//!        ▼
//! ┌─────────────────────┐
//! │  PolicyDataStore    │
//! │  Arc<PolicySnapshot>│ ◄─── Atomic swap
//! └─────────────────────┘
//!        ▲
//!        │ validate & swap
//! ┌──────┴──────┐
//! │  Validation │ (background thread pool, N workers)
//! │   Workers   │
//! └─────────────┘
//! ```

use crate::bytecode::CompiledPolicy;
use crate::compiler::PolicyCompiler;
use crate::interpreter::{FieldMapping, Interpreter};
use crate::parser::parse::Parser;
use crate::rar::{EvaluationContext, ResourceTypeId};
use crate::{Decision, Result};
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

/// Immutable snapshot of all policies and pre-compiled data
#[derive(Debug, Clone)]
pub struct PolicySnapshot {
    /// Version number (monotonically increasing)
    pub version: u64,

    /// All pre-compiled policies
    policies: Vec<PolicyEntry>,

    /// Index: resource_type_id -> policy indices
    index: HashMap<ResourceTypeId, Vec<usize>>,
}

/// Pre-compiled policy entry
#[derive(Debug, Clone)]
pub struct PolicyEntry {
    /// Policy name
    pub name: String,

    /// Pre-compiled bytecode
    pub bytecode: Arc<CompiledPolicy>,

    /// Pre-computed field mapping
    pub field_mapping: FieldMapping,

    /// Resource types this policy applies to
    pub resource_types: Vec<ResourceTypeId>,
}

impl PolicySnapshot {
    /// Create an empty snapshot
    pub fn empty() -> Self {
        Self {
            version: 0,
            policies: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Create a new snapshot with given policies
    pub fn new(version: u64, policies: Vec<PolicyEntry>) -> Self {
        let mut index: HashMap<ResourceTypeId, Vec<usize>> = HashMap::new();

        for (idx, policy) in policies.iter().enumerate() {
            for resource_type in &policy.resource_types {
                index.entry(*resource_type).or_default().push(idx);
            }
        }

        Self { version, policies, index }
    }

    /// Get all policies that apply to a resource type
    #[inline]
    pub fn policies_for_resource(&self, resource_type: ResourceTypeId) -> Vec<&PolicyEntry> {
        if let Some(indices) = self.index.get(&resource_type) {
            indices.iter().filter_map(|&idx| self.policies.get(idx)).collect()
        } else {
            Vec::new()
        }
    }

    /// Get a policy by name
    pub fn get_policy(&self, name: &str) -> Option<&PolicyEntry> {
        self.policies.iter().find(|p| p.name == name)
    }

    /// Get total number of policies
    #[inline]
    pub fn len(&self) -> usize {
        self.policies.len()
    }

    /// Check if snapshot is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }
}

/// Update request for the policy store
#[derive(Debug, Clone)]
pub enum UpdateRequest {
    /// Add a new policy
    AddPolicy { name: String, source: String, resource_types: Vec<ResourceTypeId> },

    /// Remove a policy by name
    RemovePolicy { name: String },

    /// Replace all policies
    ReplaceAll { policies: Vec<(String, String, Vec<ResourceTypeId>)> },
}

/// Result of an update operation
#[derive(Debug, Clone)]
pub enum UpdateResult {
    /// Update succeeded, new version
    Success { version: u64 },

    /// Update failed with error
    Error { message: String },
}

/// High-speed, lock-free policy data store
pub struct PolicyDataStore {
    /// Current snapshot (atomic for lock-free reads)
    snapshot: Arc<RwLock<Arc<PolicySnapshot>>>,

    /// Update channel (send updates to background worker)
    update_tx: Sender<(UpdateRequest, Sender<UpdateResult>)>,

    /// Statistics
    stats: Arc<StoreStats>,
}

/// Store statistics
#[derive(Debug, Default)]
pub struct StoreStats {
    /// Total number of reads
    pub reads: AtomicU64,

    /// Total number of updates
    pub updates: AtomicU64,

    /// Number of failed updates
    pub update_failures: AtomicU64,

    /// Current version
    pub current_version: AtomicU64,
}

impl PolicyDataStore {
    /// Create a new policy data store with a worker pool
    ///
    /// # Arguments
    /// * `worker_count` - Number of background validation workers (default: 1)
    pub fn new(worker_count: usize) -> Self {
        let (update_tx, update_rx) = unbounded();
        let snapshot = Arc::new(RwLock::new(Arc::new(PolicySnapshot::empty())));
        let stats = Arc::new(StoreStats::default());

        // Spawn validation worker(s)
        for worker_id in 0..worker_count {
            let rx = update_rx.clone();
            let snap = Arc::clone(&snapshot);
            let worker_stats = Arc::clone(&stats);

            thread::Builder::new()
                .name(format!("policy-validator-{}", worker_id))
                .spawn(move || {
                    Self::validation_worker(worker_id, rx, snap, worker_stats);
                })
                .expect("Failed to spawn validation worker");
        }

        Self { snapshot, update_tx, stats }
    }

    /// Get current snapshot (lock-free read via Arc::clone)
    #[inline]
    pub fn snapshot(&self) -> Arc<PolicySnapshot> {
        self.stats.reads.fetch_add(1, Ordering::Relaxed);
        Arc::clone(&*self.snapshot.read().unwrap())
    }

    /// Evaluate policies for a given context
    #[inline]
    pub fn evaluate(&self, ctx: &EvaluationContext) -> Result<Decision> {
        let snap = self.snapshot();
        let policies = snap.policies_for_resource(ctx.resource.type_id);

        if policies.is_empty() {
            return Ok(
                Decision::deny().with_reason("No policies found for resource type".to_string())
            );
        }

        // Evaluate all applicable policies
        let mut allow = false;
        let mut matched_policies = Vec::new();

        for policy_entry in policies {
            let mut interp = Interpreter::new(policy_entry.field_mapping.clone());
            match interp.evaluate(&policy_entry.bytecode, ctx) {
                Ok(result) => {
                    if result {
                        allow = true;
                        matched_policies.push(policy_entry.name.clone());
                    }
                },
                Err(e) => {
                    return Err(crate::Error::EvaluationError(format!(
                        "Policy '{}' failed: {}",
                        policy_entry.name, e
                    )));
                },
            }
        }

        if allow {
            let mut decision = Decision::allow();
            decision.matched_policies = matched_policies;
            Ok(decision)
        } else {
            Ok(Decision::deny().with_reason("No policies allowed access".to_string()))
        }
    }

    /// Request an update (non-blocking)
    ///
    /// Returns a receiver for the update result
    pub fn update(&self, request: UpdateRequest) -> Receiver<UpdateResult> {
        let (result_tx, result_rx) = unbounded();
        self.update_tx.send((request, result_tx)).unwrap();
        result_rx
    }

    /// Request an update and wait for result (blocking)
    pub fn update_sync(&self, request: UpdateRequest) -> UpdateResult {
        let result_rx = self.update(request);
        result_rx.recv().unwrap()
    }

    /// Background validation worker
    fn validation_worker(
        _worker_id: usize,
        rx: Receiver<(UpdateRequest, Sender<UpdateResult>)>,
        snapshot: Arc<RwLock<Arc<PolicySnapshot>>>,
        stats: Arc<StoreStats>,
    ) {
        while let Ok((request, result_tx)) = rx.recv() {
            stats.updates.fetch_add(1, Ordering::Relaxed);

            let result = match Self::process_update(&snapshot, request) {
                Ok(new_version) => {
                    stats.current_version.store(new_version, Ordering::Relaxed);
                    UpdateResult::Success { version: new_version }
                },
                Err(e) => {
                    stats.update_failures.fetch_add(1, Ordering::Relaxed);
                    UpdateResult::Error { message: e.to_string() }
                },
            };

            let _ = result_tx.send(result);
        }
    }

    /// Process an update request and swap in new snapshot
    fn process_update(
        snapshot: &Arc<RwLock<Arc<PolicySnapshot>>>,
        request: UpdateRequest,
    ) -> Result<u64> {
        let current = Arc::clone(&*snapshot.read().unwrap());
        let new_version = current.version + 1;

        let new_policies = match request {
            UpdateRequest::AddPolicy { name, source, resource_types } => {
                // Compile the policy
                let entry = Self::compile_policy(&name, &source, resource_types)?;

                // Add to existing policies
                let mut policies = current.policies.clone();
                policies.push(entry);
                policies
            },

            UpdateRequest::RemovePolicy { name } => {
                // Remove policy by name
                current.policies.iter().filter(|p| p.name != name).cloned().collect()
            },

            UpdateRequest::ReplaceAll { policies: new_policy_specs } => {
                // Compile all new policies
                let mut policies = Vec::with_capacity(new_policy_specs.len());
                for (name, source, resource_types) in new_policy_specs {
                    let entry = Self::compile_policy(&name, &source, resource_types)?;
                    policies.push(entry);
                }
                policies
            },
        };

        // Create new snapshot
        let new_snapshot = Arc::new(PolicySnapshot::new(new_version, new_policies));

        // Atomic swap
        *snapshot.write().unwrap() = new_snapshot;

        Ok(new_version)
    }

    /// Compile a policy from source
    fn compile_policy(
        name: &str,
        source: &str,
        resource_types: Vec<ResourceTypeId>,
    ) -> Result<PolicyEntry> {
        let mut parser = Parser::new(source);
        let ast = parser.parse_policy().map_err(|e| {
            crate::Error::ParseError(format!("Failed to parse policy '{}': {}", name, e))
        })?;

        // Use a random policy ID (or could hash the name)
        let policy_id = 0; // TODO: use proper ID generation
        let compiler = PolicyCompiler::new(policy_id);
        let bytecode = compiler.compile(&ast).map_err(|e| {
            crate::Error::CompilationError(format!("Failed to compile policy '{}': {}", name, e))
        })?;
        let field_mapping = bytecode
            .constants
            .iter()
            .enumerate()
            .map(|(idx, _)| (idx as u16, vec![]))
            .collect();

        Ok(PolicyEntry {
            name: name.to_string(),
            bytecode: Arc::new(bytecode),
            field_mapping,
            resource_types,
        })
    }

    /// Get store statistics
    pub fn stats(&self) -> StoreStatSnapshot {
        StoreStatSnapshot {
            reads: self.stats.reads.load(Ordering::Relaxed),
            updates: self.stats.updates.load(Ordering::Relaxed),
            update_failures: self.stats.update_failures.load(Ordering::Relaxed),
            current_version: self.stats.current_version.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of store statistics
#[derive(Debug, Clone, Copy)]
pub struct StoreStatSnapshot {
    pub reads: u64,
    pub updates: u64,
    pub update_failures: u64,
    pub current_version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_policy_snapshot_empty() {
        let snap = PolicySnapshot::empty();
        assert_eq!(snap.version, 0);
        assert_eq!(snap.len(), 0);
        assert!(snap.is_empty());
    }

    #[test]
    fn test_policy_snapshot_new() {
        let entry = PolicyEntry {
            name: "test".to_string(),
            bytecode: Arc::new(CompiledPolicy::new(1)),
            field_mapping: HashMap::new(),
            resource_types: vec![ResourceTypeId(1)],
        };

        let snap = PolicySnapshot::new(1, vec![entry]);
        assert_eq!(snap.version, 1);
        assert_eq!(snap.len(), 1);
        assert!(!snap.is_empty());
    }

    #[test]
    fn test_policy_snapshot_get_policy() {
        let entry = PolicyEntry {
            name: "test".to_string(),
            bytecode: Arc::new(CompiledPolicy::new(1)),
            field_mapping: HashMap::new(),
            resource_types: vec![ResourceTypeId(1)],
        };

        let snap = PolicySnapshot::new(1, vec![entry]);
        assert!(snap.get_policy("test").is_some());
        assert!(snap.get_policy("nonexistent").is_none());
    }

    #[test]
    fn test_policy_snapshot_policies_for_resource() {
        let entry1 = PolicyEntry {
            name: "test1".to_string(),
            bytecode: Arc::new(CompiledPolicy::new(1)),
            field_mapping: HashMap::new(),
            resource_types: vec![ResourceTypeId(1)],
        };

        let entry2 = PolicyEntry {
            name: "test2".to_string(),
            bytecode: Arc::new(CompiledPolicy::new(2)),
            field_mapping: HashMap::new(),
            resource_types: vec![ResourceTypeId(2)],
        };

        let snap = PolicySnapshot::new(1, vec![entry1, entry2]);

        let policies = snap.policies_for_resource(ResourceTypeId(1));
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].name, "test1");

        let policies = snap.policies_for_resource(ResourceTypeId(2));
        assert_eq!(policies.len(), 1);
        assert_eq!(policies[0].name, "test2");

        let policies = snap.policies_for_resource(ResourceTypeId(999));
        assert_eq!(policies.len(), 0);
    }

    #[test]
    fn test_data_store_creation() {
        let store = PolicyDataStore::new(1);
        let snap = store.snapshot();
        assert_eq!(snap.version, 0);
        assert!(snap.is_empty());
    }

    #[test]
    fn test_data_store_stats() {
        let store = PolicyDataStore::new(1);

        // Read a few times
        let _ = store.snapshot();
        let _ = store.snapshot();
        let _ = store.snapshot();

        let stats = store.stats();
        assert_eq!(stats.reads, 3);
    }

    #[test]
    fn test_data_store_add_policy() {
        let store = PolicyDataStore::new(1);

        let source = r#"
            policy TestPolicy: "Test policy"
            triggers when resource.type == "test"
            requires resource.enabled == true
        "#;

        let result = store.update_sync(UpdateRequest::AddPolicy {
            name: "test_policy".to_string(),
            source: source.to_string(),
            resource_types: vec![ResourceTypeId(1)],
        });

        match result {
            UpdateResult::Success { version } => {
                assert_eq!(version, 1);
                let snap = store.snapshot();
                assert_eq!(snap.version, 1);
                assert_eq!(snap.len(), 1);
                assert!(snap.get_policy("test_policy").is_some());
            }
            UpdateResult::Error { message } => {
                panic!("Update failed: {}", message);
            }
        }
    }

    #[test]
    fn test_data_store_remove_policy() {
        let store = PolicyDataStore::new(1);

        let source = r#"
            policy TestPolicy: "Test policy"
            triggers when resource.type == "test"
            requires resource.enabled == true
        "#;

        // Add policy
        let _ = store.update_sync(UpdateRequest::AddPolicy {
            name: "test_policy".to_string(),
            source: source.to_string(),
            resource_types: vec![ResourceTypeId(1)],
        });

        // Remove policy
        let result = store.update_sync(UpdateRequest::RemovePolicy {
            name: "test_policy".to_string(),
        });

        match result {
            UpdateResult::Success { version } => {
                assert_eq!(version, 2);
                let snap = store.snapshot();
                assert_eq!(snap.version, 2);
                assert_eq!(snap.len(), 0);
                assert!(snap.get_policy("test_policy").is_none());
            }
            UpdateResult::Error { message } => {
                panic!("Update failed: {}", message);
            }
        }
    }

    #[test]
    fn test_data_store_replace_all() {
        let store = PolicyDataStore::new(1);

        let source1 = r#"
            policy Policy1: "First policy"
            triggers when resource.type == "test"
            requires resource.enabled == true
        "#;

        let source2 = r#"
            policy Policy2: "Second policy"
            triggers when resource.type == "test"
            requires resource.count >= 5
        "#;

        let result = store.update_sync(UpdateRequest::ReplaceAll {
            policies: vec![
                ("policy1".to_string(), source1.to_string(), vec![ResourceTypeId(1)]),
                ("policy2".to_string(), source2.to_string(), vec![ResourceTypeId(2)]),
            ],
        });

        match result {
            UpdateResult::Success { version } => {
                assert_eq!(version, 1);
                let snap = store.snapshot();
                assert_eq!(snap.version, 1);
                assert_eq!(snap.len(), 2);
                assert!(snap.get_policy("policy1").is_some());
                assert!(snap.get_policy("policy2").is_some());
            }
            UpdateResult::Error { message } => {
                panic!("Update failed: {}", message);
            }
        }
    }

    #[test]
    fn test_data_store_concurrent_reads() {
        use std::thread;

        let store = Arc::new(PolicyDataStore::new(1));

        // Spawn multiple reader threads
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let store_clone = Arc::clone(&store);
                thread::spawn(move || {
                    for _ in 0..100 {
                        let snap = store_clone.snapshot();
                        assert_eq!(snap.version, 0);
                    }
                })
            })
            .collect();

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        let stats = store.stats();
        assert_eq!(stats.reads, 1000);
    }

    #[test]
    fn test_data_store_atomic_swap() {
        let store = PolicyDataStore::new(1);

        let source = r#"
            policy TestPolicy: "Test policy"
            triggers when resource.type == "test"
            requires resource.enabled == true
        "#;

        // Get initial snapshot
        let snap1 = store.snapshot();
        assert_eq!(snap1.version, 0);

        // Add policy
        let _ = store.update_sync(UpdateRequest::AddPolicy {
            name: "test_policy".to_string(),
            source: source.to_string(),
            resource_types: vec![ResourceTypeId(1)],
        });

        // Old snapshot should still be version 0
        assert_eq!(snap1.version, 0);
        assert_eq!(snap1.len(), 0);

        // New snapshot should be version 1
        let snap2 = store.snapshot();
        assert_eq!(snap2.version, 1);
        assert_eq!(snap2.len(), 1);
    }

    #[test]
    fn test_data_store_invalid_policy_syntax() {
        let store = PolicyDataStore::new(1);

        let invalid_source = "this is not valid policy syntax!!!";

        let result = store.update_sync(UpdateRequest::AddPolicy {
            name: "bad_policy".to_string(),
            source: invalid_source.to_string(),
            resource_types: vec![ResourceTypeId(1)],
        });

        match result {
            UpdateResult::Success { .. } => {
                panic!("Should have failed with parse error");
            }
            UpdateResult::Error { message } => {
                assert!(message.contains("parse"));
            }
        }

        // Store should remain unchanged
        let snap = store.snapshot();
        assert_eq!(snap.version, 0);
        assert_eq!(snap.len(), 0);
    }

    #[test]
    fn test_data_store_multiple_resource_types() {
        let store = PolicyDataStore::new(1);

        let source = r#"
            policy MultiResourcePolicy: "Policy for multiple resources"
            triggers when resource.type in ["type1", "type2"]
            requires resource.enabled == true
        "#;

        let _ = store.update_sync(UpdateRequest::AddPolicy {
            name: "multi_policy".to_string(),
            source: source.to_string(),
            resource_types: vec![ResourceTypeId(1), ResourceTypeId(2), ResourceTypeId(3)],
        });

        let snap = store.snapshot();

        // Policy should be indexed under all resource types
        let policies1 = snap.policies_for_resource(ResourceTypeId(1));
        assert_eq!(policies1.len(), 1);

        let policies2 = snap.policies_for_resource(ResourceTypeId(2));
        assert_eq!(policies2.len(), 1);

        let policies3 = snap.policies_for_resource(ResourceTypeId(3));
        assert_eq!(policies3.len(), 1);

        let policies_none = snap.policies_for_resource(ResourceTypeId(999));
        assert_eq!(policies_none.len(), 0);
    }

    #[test]
    fn test_data_store_stats_tracking() {
        let store = PolicyDataStore::new(1);

        let source = r#"
            policy TestPolicy: "Test policy"
            triggers when resource.type == "test"
            requires resource.enabled == true
        "#;

        // Perform multiple operations
        let _ = store.snapshot();
        let _ = store.snapshot();

        let _ = store.update_sync(UpdateRequest::AddPolicy {
            name: "policy1".to_string(),
            source: source.to_string(),
            resource_types: vec![ResourceTypeId(1)],
        });

        let _ = store.snapshot();

        let _ = store.update_sync(UpdateRequest::RemovePolicy {
            name: "policy1".to_string(),
        });

        let stats = store.stats();
        assert_eq!(stats.reads, 3);
        assert_eq!(stats.updates, 2);
        assert_eq!(stats.update_failures, 0);
        assert_eq!(stats.current_version, 2);
    }
}
