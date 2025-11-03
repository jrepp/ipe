//! Integration tests for approval-based authorization

use ipe_core::approval::{Approval, ApprovalCheck, ApprovalStore};
use ipe_core::rar::{Action, AttributeValue, EvaluationContext, Operation, Principal, Request, Resource};
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
    let mut approval = Approval::new(
        "bot-123",
        "https://api.example.com/data",
        "GET",
        "admin-user",
    );
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

    store
        .revoke_approval("bot-123", "https://api.example.com/data", "GET")
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
    assert_eq!(
        approval.metadata.get("justification").unwrap(),
        "automated testing"
    );
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
    assert!(store
        .is_in_approved_set("bot-500", "https://api.example.com/data")
        .unwrap());

    // Test membership for bot at the end
    assert!(store
        .is_in_approved_set("bot-1000", "https://api.example.com/data")
        .unwrap());

    // Test non-member
    assert!(!store
        .is_in_approved_set("bot-9999", "https://api.example.com/data")
        .unwrap());
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
