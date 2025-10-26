//! Test utilities and helper functions for IPE tests
//!
//! This module provides common test setup functions to reduce duplication
//! across test suites and make tests more readable.

use crate::bytecode::{CompiledPolicy, Instruction, Value};
use crate::index::PolicyDB;
use crate::interpreter::FieldMapping;
use crate::rar::{AttributeValue, EvaluationContext, ResourceTypeId};
use std::collections::HashMap;

/// Create a simple policy that always returns the given boolean value
///
/// # Examples
/// ```
/// use ipe_core::testing::simple_policy;
///
/// let allow_policy = simple_policy(1, true);
/// let deny_policy = simple_policy(2, false);
/// ```
pub fn simple_policy(policy_id: u64, allow: bool) -> CompiledPolicy {
    let mut policy = CompiledPolicy::new(policy_id);
    policy.emit(Instruction::Return { value: allow });
    policy
}

/// Create a test context with customizable resource type and attributes
///
/// # Examples
/// ```
/// use ipe_core::testing::test_context_with_resource;
/// use ipe_core::rar::{AttributeValue, ResourceTypeId};
/// use std::collections::HashMap;
///
/// let mut attrs = HashMap::new();
/// attrs.insert("priority".to_string(), AttributeValue::Int(5));
///
/// let ctx = test_context_with_resource(ResourceTypeId(1), attrs);
/// ```
pub fn test_context_with_resource(
    type_id: ResourceTypeId,
    attributes: HashMap<String, AttributeValue>,
) -> EvaluationContext {
    let mut ctx = EvaluationContext::default();
    ctx.resource.type_id = type_id;
    ctx.resource.attributes = attributes;
    ctx
}

/// Create a test context with a single resource attribute
///
/// # Examples
/// ```
/// use ipe_core::testing::test_context_with_attr;
/// use ipe_core::rar::{AttributeValue, ResourceTypeId};
///
/// let ctx = test_context_with_attr(
///     ResourceTypeId(1),
///     "name",
///     AttributeValue::String("test-resource".to_string())
/// );
/// ```
pub fn test_context_with_attr(
    type_id: ResourceTypeId,
    attr_name: &str,
    attr_value: AttributeValue,
) -> EvaluationContext {
    let mut attributes = HashMap::new();
    attributes.insert(attr_name.to_string(), attr_value);
    test_context_with_resource(type_id, attributes)
}

/// Create a policy database with a single policy
///
/// # Examples
/// ```
/// use ipe_core::testing::{simple_policy, policy_db_with_policy};
/// use ipe_core::interpreter::FieldMapping;
/// use ipe_core::rar::ResourceTypeId;
///
/// let policy = simple_policy(1, true);
/// let db = policy_db_with_policy(
///     "test-policy",
///     policy,
///     FieldMapping::new(),
///     vec![ResourceTypeId(1)]
/// );
/// ```
pub fn policy_db_with_policy(
    name: &str,
    policy: CompiledPolicy,
    field_map: FieldMapping,
    resource_types: Vec<ResourceTypeId>,
) -> PolicyDB {
    let mut db = PolicyDB::new();
    db.add_policy(name.to_string(), policy, field_map, resource_types);
    db
}

/// Create a field mapping from a list of (offset, path) tuples
///
/// # Examples
/// ```
/// use ipe_core::testing::field_mapping_from_paths;
///
/// let mapping = field_mapping_from_paths(&[
///     (0, vec!["resource", "priority"]),
///     (1, vec!["resource", "enabled"]),
/// ]);
/// ```
pub fn field_mapping_from_paths(paths: &[(u16, Vec<&str>)]) -> FieldMapping {
    let mut mapping = FieldMapping::new();
    for (offset, path) in paths {
        mapping.insert(
            *offset,
            path.iter().map(|s| s.to_string()).collect(),
        );
    }
    mapping
}

/// Builder for creating more complex test policies
///
/// # Examples
/// ```
/// use ipe_core::testing::PolicyBuilder;
/// use ipe_core::bytecode::{Instruction, Value, CompOp};
///
/// let policy = PolicyBuilder::new(1)
///     .load_field(0)
///     .load_const(Value::Int(5))
///     .compare(CompOp::Eq)
///     .jump_if_false(2)
///     .return_value(true)
///     .return_value(false)
///     .build();
/// ```
pub struct PolicyBuilder {
    policy: CompiledPolicy,
}

impl PolicyBuilder {
    /// Create a new policy builder
    pub fn new(policy_id: u64) -> Self {
        Self {
            policy: CompiledPolicy::new(policy_id),
        }
    }

    /// Load a field from the evaluation context
    pub fn load_field(mut self, offset: u16) -> Self {
        self.policy.emit(Instruction::LoadField { offset });
        self
    }

    /// Load a constant value
    pub fn load_const(mut self, value: Value) -> Self {
        let idx = self.policy.add_constant(value);
        self.policy.emit(Instruction::LoadConst { idx });
        self
    }

    /// Add a comparison instruction
    pub fn compare(mut self, op: crate::bytecode::CompOp) -> Self {
        self.policy.emit(Instruction::Compare { op });
        self
    }

    /// Add a jump if false instruction
    pub fn jump_if_false(mut self, offset: i16) -> Self {
        self.policy.emit(Instruction::JumpIfFalse { offset });
        self
    }

    /// Add an AND instruction
    pub fn and(mut self) -> Self {
        self.policy.emit(Instruction::And);
        self
    }

    /// Add an OR instruction
    pub fn or(mut self) -> Self {
        self.policy.emit(Instruction::Or);
        self
    }

    /// Add a NOT instruction
    pub fn not(mut self) -> Self {
        self.policy.emit(Instruction::Not);
        self
    }

    /// Add a return instruction
    pub fn return_value(mut self, value: bool) -> Self {
        self.policy.emit(Instruction::Return { value });
        self
    }

    /// Build and return the compiled policy
    pub fn build(self) -> CompiledPolicy {
        self.policy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_policy_allow() {
        let policy = simple_policy(1, true);
        assert_eq!(policy.code.len(), 1);
        assert!(matches!(policy.code[0], Instruction::Return { value: true }));
    }

    #[test]
    fn test_simple_policy_deny() {
        let policy = simple_policy(1, false);
        assert_eq!(policy.code.len(), 1);
        assert!(matches!(policy.code[0], Instruction::Return { value: false }));
    }

    #[test]
    fn test_test_context_with_attr() {
        let ctx = test_context_with_attr(
            ResourceTypeId(1),
            "name",
            AttributeValue::String("test".to_string()),
        );
        assert_eq!(ctx.resource.type_id, ResourceTypeId(1));
        assert_eq!(ctx.resource.attributes.len(), 1);
        assert!(ctx.resource.attributes.contains_key("name"));
    }

    #[test]
    fn test_policy_db_with_policy() {
        let policy = simple_policy(1, true);
        let db = policy_db_with_policy(
            "test",
            policy,
            FieldMapping::new(),
            vec![ResourceTypeId(1)],
        );
        assert_eq!(db.len(), 1);
        assert!(db.get_policy_by_name("test").is_some());
    }

    #[test]
    fn test_field_mapping_from_paths() {
        let mapping = field_mapping_from_paths(&[
            (0, vec!["resource", "priority"]),
            (1, vec!["resource", "enabled"]),
        ]);
        assert_eq!(mapping.len(), 2);
        assert_eq!(mapping[&0], vec!["resource", "priority"]);
        assert_eq!(mapping[&1], vec!["resource", "enabled"]);
    }

    #[test]
    fn test_policy_builder() {
        use crate::bytecode::CompOp;

        let policy = PolicyBuilder::new(1)
            .load_field(0)
            .load_const(Value::Int(42))
            .compare(CompOp::Eq)
            .return_value(true)
            .build();

        assert_eq!(policy.code.len(), 4);
        assert_eq!(policy.constants.len(), 1);
    }
}
