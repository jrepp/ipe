use crate::{EvaluationContext, Result, Error};
use crate::index::PolicyDB;
use crate::interpreter::Interpreter;
use serde::{Deserialize, Serialize};

/// Policy decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub kind: DecisionKind,
    pub reason: Option<String>,
    pub matched_policies: Vec<String>,
}

impl Decision {
    pub fn from_bool(allowed: bool) -> Self {
        Self {
            kind: if allowed { DecisionKind::Allow } else { DecisionKind::Deny },
            reason: None,
            matched_policies: vec![],
        }
    }

    pub fn allow() -> Self {
        Self {
            kind: DecisionKind::Allow,
            reason: None,
            matched_policies: vec![],
        }
    }

    pub fn deny() -> Self {
        Self {
            kind: DecisionKind::Deny,
            reason: None,
            matched_policies: vec![],
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }

    pub fn add_matched_policy(mut self, policy_name: String) -> Self {
        self.matched_policies.push(policy_name);
        self
    }
}

/// Decision kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionKind {
    Allow,
    Deny,
}

/// Main policy evaluation engine
#[derive(Default)]
pub struct PolicyEngine {
    policy_db: PolicyDB,
}

impl PolicyEngine {
    /// Create a new empty policy engine
    pub fn new() -> Self {
        Self {
            policy_db: PolicyDB::new(),
        }
    }

    /// Create a policy engine with the given policy database
    pub fn with_policy_db(policy_db: PolicyDB) -> Self {
        Self { policy_db }
    }

    /// Get a reference to the policy database
    pub fn policy_db(&self) -> &PolicyDB {
        &self.policy_db
    }

    /// Get a mutable reference to the policy database
    pub fn policy_db_mut(&mut self) -> &mut PolicyDB {
        &mut self.policy_db
    }

    /// Evaluate a single policy against the context
    pub fn evaluate(&self, ctx: &EvaluationContext) -> Result<Decision> {
        // Get policies for this resource type
        let policies = self.policy_db.get_policies_for_resource(ctx.resource.type_id);

        if policies.is_empty() {
            // No policies found - default deny
            return Ok(Decision::deny().with_reason("No policies found for resource type".to_string()));
        }

        let mut decision = Decision::deny();
        let mut any_allow = false;
        let mut any_deny = false;

        // Evaluate each policy
        for stored_policy in policies {
            let mut interp = Interpreter::new(stored_policy.field_map.clone());

            match interp.evaluate(&stored_policy.policy, ctx) {
                Ok(result) => {
                    if result {
                        // Policy allows
                        any_allow = true;
                        decision = decision.add_matched_policy(stored_policy.name.clone());
                    } else {
                        // Policy denies
                        any_deny = true;
                    }
                }
                Err(e) => {
                    return Err(Error::EvaluationError(format!(
                        "Policy '{}' evaluation failed: {}",
                        stored_policy.name, e
                    )));
                }
            }
        }

        // Decision logic: any deny overrides any allow (deny-by-default)
        if any_allow && !any_deny {
            decision.kind = DecisionKind::Allow;
            Ok(decision)
        } else if any_deny {
            Ok(Decision::deny().with_reason("One or more policies denied the request".to_string()))
        } else {
            Ok(Decision::deny().with_reason("No policies allowed the request".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::{CompiledPolicy, Instruction, Value, CompOp};
    use crate::interpreter::FieldMapping;
    use crate::rar::{AttributeValue, ResourceTypeId};

    #[test]
    fn test_decision_from_bool() {
        let allow = Decision::from_bool(true);
        assert_eq!(allow.kind, DecisionKind::Allow);

        let deny = Decision::from_bool(false);
        assert_eq!(deny.kind, DecisionKind::Deny);
    }

    #[test]
    fn test_decision_builders() {
        let decision = Decision::allow()
            .with_reason("Test reason".to_string())
            .add_matched_policy("policy1".to_string());

        assert_eq!(decision.kind, DecisionKind::Allow);
        assert_eq!(decision.reason, Some("Test reason".to_string()));
        assert_eq!(decision.matched_policies.len(), 1);
        assert_eq!(decision.matched_policies[0], "policy1");
    }

    #[test]
    fn test_engine_new() {
        let engine = PolicyEngine::new();
        assert!(engine.policy_db().is_empty());
    }

    #[test]
    fn test_engine_no_policies_defaults_deny() {
        let engine = PolicyEngine::new();
        let ctx = EvaluationContext::default();

        let decision = engine.evaluate(&ctx).unwrap();
        assert_eq!(decision.kind, DecisionKind::Deny);
        assert!(decision.reason.is_some());
    }

    #[test]
    fn test_engine_simple_allow_policy() {
        use crate::testing::{simple_policy, policy_db_with_policy, test_context_with_resource};
        use std::collections::HashMap;

        let policy = simple_policy(1, true);
        let db = policy_db_with_policy(
            "allow-all",
            policy,
            FieldMapping::new(),
            vec![ResourceTypeId(1)],
        );

        let engine = PolicyEngine::with_policy_db(db);
        let ctx = test_context_with_resource(ResourceTypeId(1), HashMap::new());

        let decision = engine.evaluate(&ctx).unwrap();
        assert_eq!(decision.kind, DecisionKind::Allow);
        assert_eq!(decision.matched_policies.len(), 1);
        assert_eq!(decision.matched_policies[0], "allow-all");
    }

    #[test]
    fn test_engine_simple_deny_policy() {
        use crate::testing::{simple_policy, policy_db_with_policy, test_context_with_resource};
        use std::collections::HashMap;

        let policy = simple_policy(1, false);
        let db = policy_db_with_policy(
            "deny-all",
            policy,
            FieldMapping::new(),
            vec![ResourceTypeId(1)],
        );

        let engine = PolicyEngine::with_policy_db(db);
        let ctx = test_context_with_resource(ResourceTypeId(1), HashMap::new());

        let decision = engine.evaluate(&ctx).unwrap();
        assert_eq!(decision.kind, DecisionKind::Deny);
    }

    #[test]
    fn test_engine_conditional_policy() {
        // Policy: resource.priority == 5 (allow if true)
        // We need to: compare, then conditionally return based on result
        let mut policy = CompiledPolicy::new(1);

        policy.emit(Instruction::LoadField { offset: 0 });
        let idx = policy.add_constant(Value::Int(5));
        policy.emit(Instruction::LoadConst { idx });
        policy.emit(Instruction::Compare { op: CompOp::Eq });

        // Jump if false to deny
        policy.emit(Instruction::JumpIfFalse { offset: 2 }); // Skip allow return
        policy.emit(Instruction::Return { value: true });    // Allow
        policy.emit(Instruction::Return { value: false });   // Deny

        let mut field_map = FieldMapping::new();
        field_map.insert(0, vec!["resource".to_string(), "priority".to_string()]);

        let mut db = PolicyDB::new();
        db.add_policy(
            "priority-check".to_string(),
            policy,
            field_map,
            vec![ResourceTypeId(1)],
        );

        let engine = PolicyEngine::with_policy_db(db);

        // Test with priority = 5 (should allow)
        let mut ctx = EvaluationContext::default();
        ctx.resource.type_id = ResourceTypeId(1);
        ctx.resource.attributes.insert("priority".to_string(), AttributeValue::Int(5));

        let decision = engine.evaluate(&ctx).unwrap();
        assert_eq!(decision.kind, DecisionKind::Allow);

        // Test with priority = 3 (should deny)
        let mut ctx2 = EvaluationContext::default();
        ctx2.resource.type_id = ResourceTypeId(1);
        ctx2.resource.attributes.insert("priority".to_string(), AttributeValue::Int(3));

        let decision2 = engine.evaluate(&ctx2).unwrap();
        assert_eq!(decision2.kind, DecisionKind::Deny);
    }

    #[test]
    fn test_engine_multiple_policies_all_allow() {
        use crate::testing::{simple_policy, test_context_with_resource};
        use std::collections::HashMap;

        let mut db = PolicyDB::new();
        db.add_policy(
            "policy1".to_string(),
            simple_policy(1, true),
            FieldMapping::new(),
            vec![ResourceTypeId(1)],
        );
        db.add_policy(
            "policy2".to_string(),
            simple_policy(2, true),
            FieldMapping::new(),
            vec![ResourceTypeId(1)],
        );

        let engine = PolicyEngine::with_policy_db(db);
        let ctx = test_context_with_resource(ResourceTypeId(1), HashMap::new());

        let decision = engine.evaluate(&ctx).unwrap();
        assert_eq!(decision.kind, DecisionKind::Allow);
        assert_eq!(decision.matched_policies.len(), 2);
    }

    #[test]
    fn test_engine_multiple_policies_one_denies() {
        use crate::testing::{simple_policy, test_context_with_resource};
        use std::collections::HashMap;

        let mut db = PolicyDB::new();
        db.add_policy(
            "allow-policy".to_string(),
            simple_policy(1, true),
            FieldMapping::new(),
            vec![ResourceTypeId(1)],
        );
        db.add_policy(
            "deny-policy".to_string(),
            simple_policy(2, false),
            FieldMapping::new(),
            vec![ResourceTypeId(1)],
        );

        let engine = PolicyEngine::with_policy_db(db);
        let ctx = test_context_with_resource(ResourceTypeId(1), HashMap::new());

        let decision = engine.evaluate(&ctx).unwrap();
        // Any deny overrides allow
        assert_eq!(decision.kind, DecisionKind::Deny);
    }

    #[test]
    fn test_engine_complex_policy() {
        // Policy: resource.priority > 3 AND resource.enabled == true
        let mut policy = CompiledPolicy::new(1);

        // Load resource.priority
        policy.emit(Instruction::LoadField { offset: 0 });
        let idx_three = policy.add_constant(Value::Int(3));
        policy.emit(Instruction::LoadConst { idx: idx_three });
        policy.emit(Instruction::Compare { op: CompOp::Gt });

        // Load resource.enabled
        policy.emit(Instruction::LoadField { offset: 1 });
        let idx_true = policy.add_constant(Value::Bool(true));
        policy.emit(Instruction::LoadConst { idx: idx_true });
        policy.emit(Instruction::Compare { op: CompOp::Eq });

        // AND
        policy.emit(Instruction::And);

        // Jump if false to deny
        policy.emit(Instruction::JumpIfFalse { offset: 2 }); // Skip allow return
        policy.emit(Instruction::Return { value: true });    // Allow
        policy.emit(Instruction::Return { value: false });   // Deny

        let mut field_map = FieldMapping::new();
        field_map.insert(0, vec!["resource".to_string(), "priority".to_string()]);
        field_map.insert(1, vec!["resource".to_string(), "enabled".to_string()]);

        let mut db = PolicyDB::new();
        db.add_policy(
            "complex-policy".to_string(),
            policy,
            field_map,
            vec![ResourceTypeId(1)],
        );

        let engine = PolicyEngine::with_policy_db(db);

        // Test case 1: priority=5, enabled=true (should allow)
        let mut ctx1 = EvaluationContext::default();
        ctx1.resource.type_id = ResourceTypeId(1);
        ctx1.resource.attributes.insert("priority".to_string(), AttributeValue::Int(5));
        ctx1.resource.attributes.insert("enabled".to_string(), AttributeValue::Bool(true));

        let decision1 = engine.evaluate(&ctx1).unwrap();
        assert_eq!(decision1.kind, DecisionKind::Allow);

        // Test case 2: priority=2, enabled=true (should deny - priority too low)
        let mut ctx2 = EvaluationContext::default();
        ctx2.resource.type_id = ResourceTypeId(1);
        ctx2.resource.attributes.insert("priority".to_string(), AttributeValue::Int(2));
        ctx2.resource.attributes.insert("enabled".to_string(), AttributeValue::Bool(true));

        let decision2 = engine.evaluate(&ctx2).unwrap();
        assert_eq!(decision2.kind, DecisionKind::Deny);

        // Test case 3: priority=5, enabled=false (should deny - not enabled)
        let mut ctx3 = EvaluationContext::default();
        ctx3.resource.type_id = ResourceTypeId(1);
        ctx3.resource.attributes.insert("priority".to_string(), AttributeValue::Int(5));
        ctx3.resource.attributes.insert("enabled".to_string(), AttributeValue::Bool(false));

        let decision3 = engine.evaluate(&ctx3).unwrap();
        assert_eq!(decision3.kind, DecisionKind::Deny);
    }
}
