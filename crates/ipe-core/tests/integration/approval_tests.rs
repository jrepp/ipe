//! Integration tests for approval-based authorization

use ipe_core::approval::{Approval, ApprovalCheck, ApprovalStore};
use ipe_core::rar::{
    Action, AttributeValue, EvaluationContext, Operation, Principal, Request, Resource,
};
use std::sync::Arc;

#[test]
fn test_bot_denied_without_approval() {
    let store = ApprovalStore::new_temp().unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Should not have approval
    assert!(!ctx.has_approval().unwrap());
}

#[test]
fn test_bot_allowed_with_approval() {
    let store = ApprovalStore::new_temp().unwrap();

    // Privileged operation: grant approval
    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/data",
            "GET",
            "admin-user",
        ))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Should have approval
    assert!(ctx.has_approval().unwrap());
}

#[test]
fn test_different_bot_denied() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval to bot-123
    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/data",
            "GET",
            "admin-user",
        ))
        .unwrap();

    // Try with bot-456
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-456"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Should not have approval
    assert!(!ctx.has_approval().unwrap());
}

#[test]
fn test_different_resource_denied() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval for one resource
    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/data",
            "GET",
            "admin-user",
        ))
        .unwrap();

    // Try accessing different resource
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/other"),
        Action::new(Operation::Read, "other")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Should not have approval
    assert!(!ctx.has_approval().unwrap());
}

#[test]
fn test_different_action_denied() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval for GET
    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/data",
            "GET",
            "admin-user",
        ))
        .unwrap();

    // Try with POST
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Create, "data")
            .with_attribute("method", AttributeValue::String("POST".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Should not have approval
    assert!(!ctx.has_approval().unwrap());
}

#[test]
fn test_expired_approval_denied() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval that expires immediately
    let mut approval =
        Approval::new("bot-123", "https://api.example.com/data", "GET", "admin-user");
    approval.expires_at = Some(chrono::Utc::now().timestamp() - 100); // Expired 100s ago

    store.grant_approval(approval).unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Expired approval should be denied
    assert!(!ctx.has_approval().unwrap());
}

#[test]
fn test_valid_approval_with_future_expiration() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval that expires in the future
    store
        .grant_approval(
            Approval::new("bot-123", "https://api.example.com/data", "GET", "admin-user")
                .with_expiration(3600), // Expires in 1 hour
        )
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Valid approval should be allowed
    assert!(ctx.has_approval().unwrap());
}

#[test]
fn test_revoked_approval_denied() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant and then revoke approval
    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/data",
            "GET",
            "admin-user",
        ))
        .unwrap();

    store.revoke_approval("bot-123", "https://api.example.com/data", "GET").unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(Arc::new(store));

    // Revoked approval should be denied
    assert!(!ctx.has_approval().unwrap());
}

#[test]
fn test_multiple_approvals_for_same_identity() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant multiple approvals to the same bot
    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/data",
            "GET",
            "admin-user",
        ))
        .unwrap();

    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/users",
            "GET",
            "admin-user",
        ))
        .unwrap();

    store
        .grant_approval(Approval::new(
            "bot-123",
            "https://api.example.com/data",
            "POST",
            "admin-user",
        ))
        .unwrap();

    let store_arc = Arc::new(store);

    // Test first approval
    let ctx1 = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(store_arc.clone());

    assert!(ctx1.has_approval().unwrap());

    // Test second approval
    let ctx2 = EvaluationContext::new(
        Resource::url("https://api.example.com/users"),
        Action::new(Operation::Read, "users")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(store_arc.clone());

    assert!(ctx2.has_approval().unwrap());

    // Test third approval
    let ctx3 = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Create, "data")
            .with_attribute("method", AttributeValue::String("POST".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(store_arc);

    assert!(ctx3.has_approval().unwrap());
}

#[test]
fn test_approval_metadata() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval with metadata
    store
        .grant_approval(
            Approval::new("bot-123", "https://api.example.com/data", "GET", "admin-user")
                .with_metadata("ticket", "JIRA-123")
                .with_metadata("justification", "automated testing"),
        )
        .unwrap();

    let approval = store
        .get_approval("bot-123", "https://api.example.com/data", "GET")
        .unwrap()
        .expect("Approval should exist");

    assert_eq!(approval.metadata.get("ticket").unwrap(), "JIRA-123");
    assert_eq!(approval.metadata.get("justification").unwrap(), "automated testing");
}

#[test]
fn test_set_membership_with_many_approvals() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approvals to 1000 different bots
    for i in 1..=1000 {
        store
            .grant_approval(Approval::new(
                format!("bot-{}", i),
                "https://api.example.com/data",
                "GET",
                "admin",
            ))
            .unwrap();
    }

    // Test membership for bot in the middle
    assert!(store.is_in_approved_set("bot-500", "https://api.example.com/data").unwrap());

    // Test membership for bot at the end
    assert!(store.is_in_approved_set("bot-1000", "https://api.example.com/data").unwrap());

    // Test non-member
    assert!(!store.is_in_approved_set("bot-9999", "https://api.example.com/data").unwrap());
}

#[test]
fn test_batch_approval_checks() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant some approvals
    store
        .grant_approval(Approval::new("bot-1", "resource-A", "GET", "admin"))
        .unwrap();
    store
        .grant_approval(Approval::new("bot-2", "resource-B", "POST", "admin"))
        .unwrap();
    store
        .grant_approval(Approval::new("bot-3", "resource-C", "DELETE", "admin"))
        .unwrap();

    // Batch check
    let checks = vec![
        ApprovalCheck::new("bot-1", "resource-A", "GET"),
        ApprovalCheck::new("bot-2", "resource-B", "POST"),
        ApprovalCheck::new("bot-3", "resource-C", "DELETE"),
        ApprovalCheck::new("bot-4", "resource-D", "PUT"),
        ApprovalCheck::new("bot-1", "resource-B", "GET"), // Wrong resource
    ];

    let results = store.check_approvals(checks).unwrap();
    assert_eq!(results, vec![true, true, true, false, false]);
}

#[test]
fn test_list_approvals_for_identity() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant multiple approvals to bot-123
    store
        .grant_approval(Approval::new("bot-123", "resource-A", "GET", "admin"))
        .unwrap();
    store
        .grant_approval(Approval::new("bot-123", "resource-B", "POST", "admin"))
        .unwrap();
    store
        .grant_approval(Approval::new("bot-123", "resource-C", "DELETE", "admin"))
        .unwrap();

    // Grant approval to different bot
    store
        .grant_approval(Approval::new("bot-456", "resource-D", "GET", "admin"))
        .unwrap();

    let approvals = store.list_approvals("bot-123").unwrap();
    assert_eq!(approvals.len(), 3);

    let approvals = store.list_approvals("bot-456").unwrap();
    assert_eq!(approvals.len(), 1);
}

#[test]
fn test_no_approval_store_error() {
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data"),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    );

    // Should error when no approval store is configured
    let result = ctx.has_approval();
    assert!(result.is_err());
}

#[test]
fn test_scope_tenant_isolation() {
    use ipe_core::approval::Scope;

    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval in tenant-A scope
    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin")
                .with_scope(Scope::tenant("tenant-A")),
        )
        .unwrap();

    // Grant approval in tenant-B scope
    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin")
                .with_scope(Scope::tenant("tenant-B")),
        )
        .unwrap();

    // Check tenant-A scope
    assert!(store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::tenant("tenant-A"))
        .unwrap());

    // Check tenant-B scope
    assert!(store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::tenant("tenant-B"))
        .unwrap());

    // Global scope should not have access
    assert!(!store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::Global)
        .unwrap());

    // Different tenant should not have access
    assert!(!store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::tenant("tenant-C"))
        .unwrap());
}

#[test]
fn test_scope_environment_isolation() {
    use ipe_core::approval::Scope;

    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval in dev environment
    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin").with_scope(Scope::env("dev")),
        )
        .unwrap();

    // Grant approval in prod environment
    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin").with_scope(Scope::env("prod")),
        )
        .unwrap();

    // Check dev environment
    assert!(store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::env("dev"))
        .unwrap());

    // Check prod environment
    assert!(store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::env("prod"))
        .unwrap());

    // Staging should not have access
    assert!(!store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::env("staging"))
        .unwrap());
}

#[test]
fn test_scope_tenant_environment_combination() {
    use ipe_core::approval::Scope;

    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval in tenant-A dev environment
    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin")
                .with_scope(Scope::tenant_env("tenant-A", "dev")),
        )
        .unwrap();

    // Check correct tenant and environment
    assert!(store
        .has_approval_in_scope(
            "bot-123",
            "resource-1",
            "GET",
            &Scope::tenant_env("tenant-A", "dev")
        )
        .unwrap());

    // Wrong tenant, correct environment
    assert!(!store
        .has_approval_in_scope(
            "bot-123",
            "resource-1",
            "GET",
            &Scope::tenant_env("tenant-B", "dev")
        )
        .unwrap());

    // Correct tenant, wrong environment
    assert!(!store
        .has_approval_in_scope(
            "bot-123",
            "resource-1",
            "GET",
            &Scope::tenant_env("tenant-A", "prod")
        )
        .unwrap());
}

#[test]
fn test_scope_custom_hierarchy() {
    use ipe_core::approval::Scope;

    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval in custom scope
    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin").with_scope(Scope::Custom(vec![
                "region".into(),
                "us-west".into(),
                "az-1".into(),
            ])),
        )
        .unwrap();

    // Check correct custom scope
    assert!(store
        .has_approval_in_scope(
            "bot-123",
            "resource-1",
            "GET",
            &Scope::Custom(vec!["region".into(), "us-west".into(), "az-1".into()])
        )
        .unwrap());

    // Different custom scope
    assert!(!store
        .has_approval_in_scope(
            "bot-123",
            "resource-1",
            "GET",
            &Scope::Custom(vec!["region".into(), "eu-west".into(), "az-1".into()])
        )
        .unwrap());
}

#[test]
fn test_scope_encoding() {
    use ipe_core::approval::Scope;

    assert_eq!(Scope::Global.encode(), "global");
    assert_eq!(Scope::tenant("acme").encode(), "tenant:acme");
    assert_eq!(Scope::env("prod").encode(), "env:prod");
    assert_eq!(Scope::tenant_env("acme", "prod").encode(), "tenant:acme:env:prod");
    assert_eq!(Scope::Custom(vec!["a".into(), "b".into()]).encode(), "custom:a:b");
}

#[test]
fn test_list_approvals_in_scope() {
    use ipe_core::approval::Scope;

    let store = ApprovalStore::new_temp().unwrap();

    // Grant approvals in different scopes
    store
        .grant_approval(
            Approval::new("bot-123", "resource-A", "GET", "admin")
                .with_scope(Scope::tenant("tenant-A")),
        )
        .unwrap();

    store
        .grant_approval(
            Approval::new("bot-123", "resource-B", "POST", "admin")
                .with_scope(Scope::tenant("tenant-A")),
        )
        .unwrap();

    store
        .grant_approval(
            Approval::new("bot-123", "resource-C", "DELETE", "admin")
                .with_scope(Scope::tenant("tenant-B")),
        )
        .unwrap();

    // List approvals in tenant-A
    let approvals = store.list_approvals_in_scope("bot-123", &Scope::tenant("tenant-A")).unwrap();
    assert_eq!(approvals.len(), 2);

    // List approvals in tenant-B
    let approvals = store.list_approvals_in_scope("bot-123", &Scope::tenant("tenant-B")).unwrap();
    assert_eq!(approvals.len(), 1);

    // List approvals in global scope (none)
    let approvals = store.list_approvals_in_scope("bot-123", &Scope::Global).unwrap();
    assert_eq!(approvals.len(), 0);
}

#[test]
fn test_revoke_approval_in_scope() {
    use ipe_core::approval::Scope;

    let store = ApprovalStore::new_temp().unwrap();

    // Grant approvals in different scopes
    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin")
                .with_scope(Scope::tenant("tenant-A")),
        )
        .unwrap();

    store
        .grant_approval(
            Approval::new("bot-123", "resource-1", "GET", "admin")
                .with_scope(Scope::tenant("tenant-B")),
        )
        .unwrap();

    // Revoke from tenant-A only
    store
        .revoke_approval_in_scope("bot-123", "resource-1", "GET", &Scope::tenant("tenant-A"))
        .unwrap();

    // tenant-A should not have approval
    assert!(!store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::tenant("tenant-A"))
        .unwrap());

    // tenant-B should still have approval
    assert!(store
        .has_approval_in_scope("bot-123", "resource-1", "GET", &Scope::tenant("tenant-B"))
        .unwrap());
}

#[test]
fn test_approval_with_ttl() {
    let store = ApprovalStore::new_temp().unwrap();

    // Create approval with TTL
    let approval = Approval::new("bot-123", "resource-1", "GET", "admin").with_ttl(3600);

    assert_eq!(approval.ttl_seconds, Some(3600));
    assert!(approval.expires_at.is_some());

    store.grant_approval(approval).unwrap();

    // Should be valid
    assert!(store.has_approval("bot-123", "resource-1", "GET").unwrap());
}

#[test]
fn test_ttl_config_defaults() {
    use ipe_core::approval::TTLConfig;

    let config = TTLConfig::default();
    assert_eq!(config.default_ttl_seconds, None);
    assert_eq!(config.min_ttl_seconds, 60);
    assert_eq!(config.max_ttl_seconds, 365 * 24 * 3600);
    assert!(config.enforce_ttl);
}

#[test]
fn test_ttl_config_temporary() {
    use ipe_core::approval::TTLConfig;

    let config = TTLConfig::temporary();
    assert_eq!(config.default_ttl_seconds, Some(3600));
    assert_eq!(config.min_ttl_seconds, 60);
    assert_eq!(config.max_ttl_seconds, 24 * 3600);
    assert!(config.enforce_ttl);
}

#[test]
fn test_ttl_config_short_lived() {
    use ipe_core::approval::TTLConfig;

    let config = TTLConfig::short_lived();
    assert_eq!(config.default_ttl_seconds, Some(24 * 3600));
    assert_eq!(config.min_ttl_seconds, 3600);
    assert_eq!(config.max_ttl_seconds, 7 * 24 * 3600);
    assert!(config.enforce_ttl);
}

#[test]
fn test_ttl_config_long_lived() {
    use ipe_core::approval::TTLConfig;

    let config = TTLConfig::long_lived();
    assert_eq!(config.default_ttl_seconds, Some(30 * 24 * 3600));
    assert_eq!(config.min_ttl_seconds, 24 * 3600);
    assert_eq!(config.max_ttl_seconds, 365 * 24 * 3600);
    assert!(config.enforce_ttl);
}

#[test]
fn test_is_in_approved_set_in_scope() {
    use ipe_core::approval::Scope;

    let store = ApprovalStore::new_temp().unwrap();

    // Grant approvals in tenant-A scope
    for i in 1..=10 {
        store
            .grant_approval(
                Approval::new(format!("bot-{}", i), "https://api.example.com/data", "GET", "admin")
                    .with_scope(Scope::tenant("tenant-A")),
            )
            .unwrap();
    }

    // Check set membership in correct scope
    assert!(store
        .is_in_approved_set_in_scope(
            "bot-5",
            "https://api.example.com/data",
            &Scope::tenant("tenant-A")
        )
        .unwrap());

    // Check set membership in wrong scope
    assert!(!store
        .is_in_approved_set_in_scope(
            "bot-5",
            "https://api.example.com/data",
            &Scope::tenant("tenant-B")
        )
        .unwrap());
}

#[test]
fn test_get_approval_returns_none() {
    let store = ApprovalStore::new_temp().unwrap();

    let result = store.get_approval("nonexistent", "resource", "action").unwrap();
    assert!(result.is_none());
}

#[test]
fn test_approval_count() {
    let store = ApprovalStore::new_temp().unwrap();

    assert_eq!(store.count_approvals().unwrap(), 0);

    store.grant_approval(Approval::new("bot-1", "res-1", "GET", "admin")).unwrap();
    assert_eq!(store.count_approvals().unwrap(), 1);

    store.grant_approval(Approval::new("bot-2", "res-2", "POST", "admin")).unwrap();
    assert_eq!(store.count_approvals().unwrap(), 2);

    // Granting same approval again should not increase count (update)
    store.grant_approval(Approval::new("bot-1", "res-1", "GET", "admin")).unwrap();
    assert_eq!(store.count_approvals().unwrap(), 2);
}
