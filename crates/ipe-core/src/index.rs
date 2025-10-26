use crate::bytecode::CompiledPolicy;
use crate::interpreter::{FieldMapping, Interpreter};
use crate::rar::ResourceTypeId;
use std::collections::HashMap;

/// Policy database with indexing capabilities
#[derive(Default)]
pub struct PolicyDB {
    policies: Vec<StoredPolicy>,
    index_by_resource_type: HashMap<ResourceTypeId, Vec<usize>>,
}

/// A stored policy with metadata
pub struct StoredPolicy {
    pub name: String,
    pub policy: CompiledPolicy,
    pub field_map: FieldMapping,
    pub resource_types: Vec<ResourceTypeId>,
}

impl PolicyDB {
    /// Create a new empty policy database
    pub fn new() -> Self {
        Self {
            policies: Vec::new(),
            index_by_resource_type: HashMap::new(),
        }
    }

    /// Add a policy to the database
    pub fn add_policy(
        &mut self,
        name: String,
        policy: CompiledPolicy,
        field_map: FieldMapping,
        resource_types: Vec<ResourceTypeId>,
    ) {
        let policy_idx = self.policies.len();

        // Index by each resource type
        for resource_type in &resource_types {
            self.index_by_resource_type
                .entry(*resource_type)
                .or_insert_with(Vec::new)
                .push(policy_idx);
        }

        self.policies.push(StoredPolicy { name, policy, field_map, resource_types });
    }

    /// Get policies matching a specific resource type
    pub fn get_policies_for_resource(&self, resource_type: ResourceTypeId) -> Vec<&StoredPolicy> {
        if let Some(indices) = self.index_by_resource_type.get(&resource_type) {
            indices.iter().filter_map(|idx| self.policies.get(*idx)).collect()
        } else {
            Vec::new()
        }
    }

    /// Get all policies
    pub fn get_all_policies(&self) -> &[StoredPolicy] {
        &self.policies
    }

    /// Get policy by name
    pub fn get_policy_by_name(&self, name: &str) -> Option<&StoredPolicy> {
        self.policies.iter().find(|p| p.name == name)
    }

    /// Get the number of policies in the database
    pub fn len(&self) -> usize {
        self.policies.len()
    }

    /// Check if the database is empty
    pub fn is_empty(&self) -> bool {
        self.policies.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{Instruction, Value};

    #[test]
    fn test_policydb_new() {
        let db = PolicyDB::new();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
    }

    #[test]
    fn test_policydb_add_policy() {
        let mut db = PolicyDB::new();

        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::Return { value: true });

        let field_map = FieldMapping::new();
        let resource_types = vec![ResourceTypeId(1)];

        db.add_policy("test-policy".to_string(), policy, field_map, resource_types);

        assert_eq!(db.len(), 1);
        assert!(!db.is_empty());
    }

    #[test]
    fn test_policydb_get_policy_by_name() {
        let mut db = PolicyDB::new();

        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::Return { value: true });

        let field_map = FieldMapping::new();
        let resource_types = vec![ResourceTypeId(1)];

        db.add_policy("test-policy".to_string(), policy, field_map, resource_types);

        let found = db.get_policy_by_name("test-policy");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test-policy");

        let not_found = db.get_policy_by_name("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_policydb_index_by_resource_type() {
        let mut db = PolicyDB::new();

        let mut policy1 = CompiledPolicy::new(1);
        policy1.emit(Instruction::Return { value: true });

        let mut policy2 = CompiledPolicy::new(2);
        policy2.emit(Instruction::Return { value: false });

        let field_map = FieldMapping::new();

        // Policy 1 applies to resource type 1
        db.add_policy("policy1".to_string(), policy1, field_map.clone(), vec![ResourceTypeId(1)]);

        // Policy 2 applies to resource types 1 and 2
        db.add_policy(
            "policy2".to_string(),
            policy2,
            field_map,
            vec![ResourceTypeId(1), ResourceTypeId(2)],
        );

        // Get policies for resource type 1
        let policies_for_type1 = db.get_policies_for_resource(ResourceTypeId(1));
        assert_eq!(policies_for_type1.len(), 2);

        // Get policies for resource type 2
        let policies_for_type2 = db.get_policies_for_resource(ResourceTypeId(2));
        assert_eq!(policies_for_type2.len(), 1);
        assert_eq!(policies_for_type2[0].name, "policy2");

        // Get policies for non-existent resource type
        let policies_for_type3 = db.get_policies_for_resource(ResourceTypeId(3));
        assert_eq!(policies_for_type3.len(), 0);
    }

    #[test]
    fn test_policydb_get_all_policies() {
        let mut db = PolicyDB::new();

        let mut policy1 = CompiledPolicy::new(1);
        policy1.emit(Instruction::Return { value: true });

        let mut policy2 = CompiledPolicy::new(2);
        policy2.emit(Instruction::Return { value: false });

        let field_map = FieldMapping::new();

        db.add_policy("policy1".to_string(), policy1, field_map.clone(), vec![ResourceTypeId(1)]);

        db.add_policy("policy2".to_string(), policy2, field_map, vec![ResourceTypeId(2)]);

        let all_policies = db.get_all_policies();
        assert_eq!(all_policies.len(), 2);
        assert_eq!(all_policies[0].name, "policy1");
        assert_eq!(all_policies[1].name, "policy2");
    }

    #[test]
    fn test_policydb_multiple_resource_types() {
        let mut db = PolicyDB::new();

        let mut policy = CompiledPolicy::new(1);
        policy.emit(Instruction::Return { value: true });

        let field_map = FieldMapping::new();

        // Policy applies to multiple resource types
        db.add_policy(
            "multi-type-policy".to_string(),
            policy,
            field_map,
            vec![ResourceTypeId(1), ResourceTypeId(2), ResourceTypeId(3)],
        );

        // Check that it's indexed under all resource types
        assert_eq!(db.get_policies_for_resource(ResourceTypeId(1)).len(), 1);
        assert_eq!(db.get_policies_for_resource(ResourceTypeId(2)).len(), 1);
        assert_eq!(db.get_policies_for_resource(ResourceTypeId(3)).len(), 1);
    }
}
