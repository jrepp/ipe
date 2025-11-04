//! End-to-end integration tests combining policy evaluation with approval checks

use ipe_core::approval::{Approval, ApprovalStore};
use ipe_core::rar::{
    Action, AttributeValue, EvaluationContext, Operation, Principal, Request, Resource,
};
use std::sync::Arc;

/// Simulates a privileged data plane that writes approvals
struct PrivilegedDataPlane {
    store: Arc<ApprovalStore>,
    admin_id: String,
}

impl PrivilegedDataPlane {
    fn new(store: Arc<ApprovalStore>) -> Self {
        Self {
            store,
            admin_id: "privileged-admin".into(),
        }
    }

    fn grant_access(
        &self,
        identity: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), ipe_core::approval::ApprovalError> {
        self.store
            .grant_approval(Approval::new(identity, resource, action, &self.admin_id))
    }

    fn grant_access_with_expiration(
        &self,
        identity: &str,
        resource: &str,
        action: &str,
        expires_in_seconds: i64,
    ) -> Result<(), ipe_core::approval::ApprovalError> {
        self.store.grant_approval(
            Approval::new(identity, resource, action, &self.admin_id)
                .with_expiration(expires_in_seconds),
        )
    }

    fn revoke_access(
        &self,
        identity: &str,
        resource: &str,
        action: &str,
    ) -> Result<(), ipe_core::approval::ApprovalError> {
        self.store.revoke_approval(identity, resource, action)
    }
}

#[test]
fn test_e2e_bot_workflow_without_approval() {
    // Setup: Create database and privileged data plane
    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Scenario: Bot tries to access API without approval
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/sensitive-data"),
        Action::new(Operation::Read, "sensitive-data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("service-bot-alpha")
                .with_attribute("environment", AttributeValue::String("production".into())),
            timestamp: chrono::Utc::now().timestamp(),
            source_ip: Some("10.0.1.42".into()),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    // Expected: Access denied (no approval)
    assert!(!ctx.has_approval().unwrap());
}

#[test]
fn test_e2e_bot_workflow_with_approval() {
    // Setup: Create database and privileged data plane
    let store = Arc::new(ApprovalStore::new_temp().unwrap());
    let data_plane = PrivilegedDataPlane::new(store.clone());

    // Step 1: Privileged admin grants approval to bot
    data_plane
        .grant_access("service-bot-alpha", "https://api.example.com/sensitive-data", "GET")
        .unwrap();

    // Step 2: Bot makes request with approval
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/sensitive-data"),
        Action::new(Operation::Read, "sensitive-data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("service-bot-alpha")
                .with_attribute("environment", AttributeValue::String("production".into())),
            timestamp: chrono::Utc::now().timestamp(),
            source_ip: Some("10.0.1.42".into()),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    // Expected: Access allowed (approval exists)
    assert!(ctx.has_approval().unwrap());
}

#[test]
fn test_e2e_approval_lifecycle() {
    // Setup
    let store = Arc::new(ApprovalStore::new_temp().unwrap());
    let data_plane = PrivilegedDataPlane::new(store.clone());

    let identity = "bot-lifecycle-test";
    let resource = "https://api.example.com/data";
    let action = "GET";

    // Helper to create context
    let make_context = |store: Arc<ApprovalStore>| {
        EvaluationContext::new(
            Resource::url(resource),
            Action::new(Operation::Read, "data")
                .with_attribute("method", AttributeValue::String(action.into())),
            Request {
                principal: Principal::bot(identity),
                ..Default::default()
            },
        )
        .with_approval_store(store)
    };

    // 1. Initially denied (no approval)
    let ctx1 = make_context(store.clone());
    assert!(!ctx1.has_approval().unwrap());

    // 2. Grant approval
    data_plane.grant_access(identity, resource, action).unwrap();

    // 3. Now allowed
    let ctx2 = make_context(store.clone());
    assert!(ctx2.has_approval().unwrap());

    // 4. Revoke approval
    data_plane.revoke_access(identity, resource, action).unwrap();

    // 5. Denied again
    let ctx3 = make_context(store.clone());
    assert!(!ctx3.has_approval().unwrap());

    // 6. Grant temporary approval (expires in 2 seconds)
    data_plane.grant_access_with_expiration(identity, resource, action, 2).unwrap();

    // 7. Immediately allowed
    let ctx4 = make_context(store.clone());
    assert!(ctx4.has_approval().unwrap());

    // 8. Wait for expiration
    std::thread::sleep(std::time::Duration::from_secs(3));

    // 9. Denied after expiration
    let ctx5 = make_context(store);
    assert!(!ctx5.has_approval().unwrap());
}

#[test]
fn test_e2e_multiple_bots_different_resources() {
    // Setup
    let store = Arc::new(ApprovalStore::new_temp().unwrap());
    let data_plane = PrivilegedDataPlane::new(store.clone());

    // Grant different approvals to different bots
    data_plane
        .grant_access("analytics-bot", "https://api.example.com/analytics", "GET")
        .unwrap();
    data_plane
        .grant_access("deploy-bot", "https://api.example.com/deploy", "POST")
        .unwrap();
    data_plane
        .grant_access("backup-bot", "https://api.example.com/backup", "GET")
        .unwrap();

    // Test analytics bot
    let analytics_ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/analytics"),
        Action::new(Operation::Read, "analytics")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("analytics-bot"),
            ..Default::default()
        },
    )
    .with_approval_store(store.clone());
    assert!(analytics_ctx.has_approval().unwrap());

    // Analytics bot shouldn't access deploy endpoint
    let analytics_wrong = EvaluationContext::new(
        Resource::url("https://api.example.com/deploy"),
        Action::new(Operation::Create, "deploy")
            .with_attribute("method", AttributeValue::String("POST".into())),
        Request {
            principal: Principal::bot("analytics-bot"),
            ..Default::default()
        },
    )
    .with_approval_store(store.clone());
    assert!(!analytics_wrong.has_approval().unwrap());

    // Test deploy bot
    let deploy_ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/deploy"),
        Action::new(Operation::Create, "deploy")
            .with_attribute("method", AttributeValue::String("POST".into())),
        Request {
            principal: Principal::bot("deploy-bot"),
            ..Default::default()
        },
    )
    .with_approval_store(store.clone());
    assert!(deploy_ctx.has_approval().unwrap());

    // Test backup bot
    let backup_ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/backup"),
        Action::new(Operation::Read, "backup")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("backup-bot"),
            ..Default::default()
        },
    )
    .with_approval_store(store);
    assert!(backup_ctx.has_approval().unwrap());
}

#[test]
fn test_e2e_high_volume_approvals() {
    // Setup
    let store = Arc::new(ApprovalStore::new_temp().unwrap());
    let data_plane = PrivilegedDataPlane::new(store.clone());

    // Grant 100 different approvals
    for i in 1..=100 {
        data_plane
            .grant_access(
                &format!("bot-{}", i),
                &format!("https://api.example.com/resource-{}", i % 10),
                if i % 3 == 0 { "POST" } else { "GET" },
            )
            .unwrap();
    }

    // Verify count
    assert_eq!(store.count_approvals().unwrap(), 100);

    // Test random access patterns
    let ctx_bot_50 = EvaluationContext::new(
        Resource::url("https://api.example.com/resource-0"),
        Action::new(Operation::Read, "resource-0")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-50"),
            ..Default::default()
        },
    )
    .with_approval_store(store.clone());
    assert!(ctx_bot_50.has_approval().unwrap());

    // bot-60 should have POST approval (60 % 3 == 0)
    let ctx_bot_60 = EvaluationContext::new(
        Resource::url("https://api.example.com/resource-0"),
        Action::new(Operation::Create, "resource-0")
            .with_attribute("method", AttributeValue::String("POST".into())),
        Request {
            principal: Principal::bot("bot-60"),
            ..Default::default()
        },
    )
    .with_approval_store(store.clone());
    assert!(ctx_bot_60.has_approval().unwrap());

    // Verify list operations
    let bot_1_approvals = store.list_approvals("bot-1").unwrap();
    assert_eq!(bot_1_approvals.len(), 1);
}

#[test]
fn test_e2e_approval_with_metadata_audit_trail() {
    // Setup
    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Grant approval with rich metadata for audit
    store
        .grant_approval(
            Approval::new(
                "critical-bot",
                "https://api.example.com/production-db",
                "DELETE",
                "senior-admin",
            )
            .with_metadata("ticket", "INCIDENT-2024-001")
            .with_metadata("justification", "Emergency cleanup after data corruption")
            .with_metadata("approved_by_email", "admin@example.com")
            .with_metadata("approval_timestamp", chrono::Utc::now().to_rfc3339())
            .with_expiration(3600), // 1 hour emergency access
        )
        .unwrap();

    // Retrieve and verify metadata
    let approval = store
        .get_approval("critical-bot", "https://api.example.com/production-db", "DELETE")
        .unwrap()
        .expect("Approval should exist");

    assert_eq!(approval.identity, "critical-bot");
    assert_eq!(approval.granted_by, "senior-admin");
    assert_eq!(approval.metadata.get("ticket").unwrap(), "INCIDENT-2024-001");
    assert_eq!(
        approval.metadata.get("justification").unwrap(),
        "Emergency cleanup after data corruption"
    );
    assert!(approval.expires_at.is_some());

    // Verify approval is active
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/production-db"),
        Action::new(Operation::Delete, "production-db")
            .with_attribute("method", AttributeValue::String("DELETE".into())),
        Request {
            principal: Principal::bot("critical-bot"),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    assert!(ctx.has_approval().unwrap());
}

#[test]
fn test_e2e_user_vs_bot_approvals() {
    // Setup
    let store = Arc::new(ApprovalStore::new_temp().unwrap());
    let data_plane = PrivilegedDataPlane::new(store.clone());

    // Grant approval to a bot
    data_plane
        .grant_access("automation-bot", "https://api.example.com/api", "GET")
        .unwrap();

    // Grant approval to a user (users can also have approvals)
    data_plane
        .grant_access("user-alice", "https://api.example.com/api", "GET")
        .unwrap();

    // Test bot access
    let bot_ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/api"),
        Action::new(Operation::Read, "api")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("automation-bot"),
            ..Default::default()
        },
    )
    .with_approval_store(store.clone());
    assert!(bot_ctx.has_approval().unwrap());

    // Test user access
    let user_ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/api"),
        Action::new(Operation::Read, "api")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::user("user-alice"),
            ..Default::default()
        },
    )
    .with_approval_store(store);
    assert!(user_ctx.has_approval().unwrap());
}
