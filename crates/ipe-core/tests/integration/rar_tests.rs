//! Integration tests for RAR (Resource, Action, Request) evaluation context

use ipe_core::rar::{
    Action, AttributeValue, EvaluationContext, Operation, Principal, Request, Resource,
    ResourceTypeId,
};
use std::collections::HashMap;

#[test]
fn test_resource_creation() {
    let resource = Resource::new(ResourceTypeId(1));
    assert_eq!(resource.type_id, ResourceTypeId(1));
    assert_eq!(resource.attributes.len(), 0);
}

#[test]
fn test_resource_with_attributes() {
    let resource = Resource::new(ResourceTypeId(1))
        .with_attribute("name", AttributeValue::String("test-resource".into()))
        .with_attribute("size", AttributeValue::Int(1024))
        .with_attribute("public", AttributeValue::Bool(true));

    assert_eq!(resource.attributes.len(), 3);
    assert_eq!(
        resource.attributes.get("name"),
        Some(&AttributeValue::String("test-resource".into()))
    );
    assert_eq!(
        resource.attributes.get("size"),
        Some(&AttributeValue::Int(1024))
    );
    assert_eq!(
        resource.attributes.get("public"),
        Some(&AttributeValue::Bool(true))
    );
}

#[test]
fn test_resource_url_helper() {
    let resource = Resource::url("https://api.example.com/data");
    assert_eq!(resource.type_id, ResourceTypeId(0));
    assert_eq!(
        resource.attributes.get("url"),
        Some(&AttributeValue::String("https://api.example.com/data".into()))
    );
}

#[test]
fn test_resource_default() {
    let resource = Resource::default();
    assert_eq!(resource.type_id, ResourceTypeId(0));
    assert_eq!(resource.attributes.len(), 0);
}

#[test]
fn test_action_creation() {
    let action = Action::new(Operation::Read, "document-123");
    assert_eq!(action.operation, Operation::Read);
    assert_eq!(action.target, "document-123");
    assert_eq!(action.attributes.len(), 0);
}

#[test]
fn test_action_with_attributes() {
    let action = Action::new(Operation::Update, "document-123")
        .with_attribute("method", AttributeValue::String("PUT".into()))
        .with_attribute("bulk", AttributeValue::Bool(false));

    assert_eq!(action.attributes.len(), 2);
    assert_eq!(
        action.attributes.get("method"),
        Some(&AttributeValue::String("PUT".into()))
    );
}

#[test]
fn test_action_default() {
    let action = Action::default();
    assert_eq!(action.operation, Operation::Read);
    assert_eq!(action.target, "");
    assert_eq!(action.attributes.len(), 0);
}

#[test]
fn test_operation_variants() {
    assert_eq!(Operation::Create, Operation::Create);
    assert_eq!(Operation::Read, Operation::Read);
    assert_eq!(Operation::Update, Operation::Update);
    assert_eq!(Operation::Delete, Operation::Delete);
    assert_eq!(Operation::Deploy, Operation::Deploy);
    assert_eq!(Operation::Execute, Operation::Execute);
    assert_eq!(Operation::Custom(1), Operation::Custom(1));
    assert_ne!(Operation::Read, Operation::Update);
}

#[test]
fn test_principal_creation() {
    let principal = Principal::new("alice");
    assert_eq!(principal.id, "alice");
    assert_eq!(principal.roles.len(), 0);
    assert_eq!(principal.attributes.len(), 0);
}

#[test]
fn test_principal_with_roles() {
    let principal = Principal::new("alice")
        .with_role("admin")
        .with_role("editor");

    assert_eq!(principal.roles.len(), 2);
    assert!(principal.roles.contains(&"admin".to_string()));
    assert!(principal.roles.contains(&"editor".to_string()));
}

#[test]
fn test_principal_with_attributes() {
    let principal = Principal::new("alice")
        .with_attribute("department", AttributeValue::String("engineering".into()))
        .with_attribute("level", AttributeValue::Int(5));

    assert_eq!(principal.attributes.len(), 2);
    assert_eq!(
        principal.attributes.get("department"),
        Some(&AttributeValue::String("engineering".into()))
    );
}

#[test]
fn test_principal_bot_helper() {
    let principal = Principal::bot("bot-123");
    assert_eq!(principal.id, "bot-123");
    assert_eq!(
        principal.attributes.get("type"),
        Some(&AttributeValue::String("bot".into()))
    );
}

#[test]
fn test_principal_user_helper() {
    let principal = Principal::user("alice");
    assert_eq!(principal.id, "alice");
    assert_eq!(
        principal.attributes.get("type"),
        Some(&AttributeValue::String("user".into()))
    );
}

#[test]
fn test_principal_default() {
    let principal = Principal::default();
    assert_eq!(principal.id, "");
    assert_eq!(principal.roles.len(), 0);
    assert_eq!(principal.attributes.len(), 0);
}

#[test]
fn test_request_default() {
    let request = Request::default();
    assert_eq!(request.principal.id, "");
    assert_eq!(request.timestamp, 0);
    assert_eq!(request.source_ip, None);
    assert_eq!(request.metadata.len(), 0);
}

#[test]
fn test_request_with_metadata() {
    let mut metadata = HashMap::new();
    metadata.insert("user_agent".to_string(), AttributeValue::String("curl/7.0".into()));
    metadata.insert("trace_id".to_string(), AttributeValue::String("abc123".into()));

    let request = Request {
        principal: Principal::user("alice"),
        timestamp: 1234567890,
        source_ip: Some("192.168.1.1".to_string()),
        metadata,
    };

    assert_eq!(request.timestamp, 1234567890);
    assert_eq!(request.source_ip, Some("192.168.1.1".to_string()));
    assert_eq!(request.metadata.len(), 2);
}

#[test]
fn test_attribute_value_variants() {
    let str_val = AttributeValue::String("test".into());
    let int_val = AttributeValue::Int(42);
    let bool_val = AttributeValue::Bool(true);
    let array_val = AttributeValue::Array(vec![
        AttributeValue::String("a".into()),
        AttributeValue::Int(1),
    ]);

    assert_eq!(str_val, AttributeValue::String("test".into()));
    assert_eq!(int_val, AttributeValue::Int(42));
    assert_eq!(bool_val, AttributeValue::Bool(true));
    assert_eq!(array_val.clone(), array_val);
}

#[test]
fn test_evaluation_context_creation() {
    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Read, "document"),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    );

    assert_eq!(
        ctx.resource.attributes.get("url"),
        Some(&AttributeValue::String("document-123".into()))
    );
    assert_eq!(ctx.action.operation, Operation::Read);
    assert_eq!(ctx.request.principal.id, "alice");
}

#[test]
fn test_evaluation_context_default() {
    let ctx = EvaluationContext::default();
    assert_eq!(ctx.resource.type_id, ResourceTypeId(0));
    assert_eq!(ctx.action.operation, Operation::Read);
    assert_eq!(ctx.request.principal.id, "");
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_without_approval_store() {
    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Read, "document"),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    );

    // Should error when approval store is not configured
    let result = ctx.has_approval();
    assert!(result.is_err());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_with_approval_store() {
    use ipe_core::approval::{Approval, ApprovalStore};
    use std::sync::Arc;

    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Grant approval
    store
        .grant_approval(Approval::new("alice", "document-123", "Read", "admin"))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Read, "document-123")
            .with_attribute("method", AttributeValue::String("Read".into())),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    assert!(ctx.has_approval().unwrap());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_without_relationship_store() {
    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Read, "document"),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    );

    // Should error when relationship store is not configured
    let result = ctx.has_relationship("editor", "document-123");
    assert!(result.is_err());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_with_relationship_store() {
    use ipe_core::relationship::{Relationship, RelationshipStore};
    use std::sync::Arc;

    let store = Arc::new(RelationshipStore::new_temp().unwrap());

    // Add relationship
    store
        .add_relationship(Relationship::role("alice", "editor", "document-123", "admin"))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Read, "document"),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    )
    .with_relationship_store(store);

    assert!(ctx.has_relationship("editor", "document-123").unwrap());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_has_approval_extracts_url_from_resource() {
    use ipe_core::approval::{Approval, ApprovalStore};
    use std::sync::Arc;

    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Grant approval for URL
    store
        .grant_approval(Approval::new(
            "alice",
            "https://api.example.com/data",
            "GET",
            "admin",
        ))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    assert!(ctx.has_approval().unwrap());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_has_approval_extracts_method_from_action() {
    use ipe_core::approval::{Approval, ApprovalStore};
    use std::sync::Arc;

    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Grant approval for POST method
    store
        .grant_approval(Approval::new("alice", "api/data", "POST", "admin"))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("api/data"),
        Action::new(Operation::Create, "data")
            .with_attribute("method", AttributeValue::String("POST".into())),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    assert!(ctx.has_approval().unwrap());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_has_approval_falls_back_to_operation() {
    use ipe_core::approval::{Approval, ApprovalStore};
    use std::sync::Arc;

    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Grant approval using operation name as string
    store
        .grant_approval(Approval::new("alice", "data", "Update", "admin"))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("data"),
        Action::new(Operation::Update, "data"),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    assert!(ctx.has_approval().unwrap());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_has_transitive_relationship() {
    use ipe_core::relationship::{Relationship, RelationshipStore};
    use std::sync::Arc;

    let store = Arc::new(RelationshipStore::new_temp().unwrap());

    // Build trust chain
    store
        .add_relationship(Relationship::trust("cert-123", "intermediate-ca", "pki"))
        .unwrap();
    store
        .add_relationship(Relationship::trust("intermediate-ca", "root-ca", "pki"))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("api.example.com"),
        Action::new(Operation::Read, "api"),
        Request {
            principal: Principal::new("cert-123"),
            ..Default::default()
        },
    )
    .with_relationship_store(store);

    assert!(ctx.has_transitive_relationship("trusted_by", "root-ca").unwrap());
}

#[cfg(feature = "approvals")]
#[test]
fn test_context_find_relationship_path() {
    use ipe_core::relationship::{Relationship, RelationshipStore};
    use std::sync::Arc;

    let store = Arc::new(RelationshipStore::new_temp().unwrap());

    // Build chain
    store
        .add_relationship(Relationship::trust("cert-123", "intermediate", "pki"))
        .unwrap();
    store
        .add_relationship(Relationship::trust("intermediate", "root", "pki"))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("api.example.com"),
        Action::new(Operation::Read, "api"),
        Request {
            principal: Principal::new("cert-123"),
            ..Default::default()
        },
    )
    .with_relationship_store(store);

    let path = ctx
        .find_relationship_path("trusted_by", "root")
        .unwrap()
        .expect("Path should exist");

    assert_eq!(path.depth, 2);
}

#[test]
fn test_resource_type_id_equality() {
    assert_eq!(ResourceTypeId(1), ResourceTypeId(1));
    assert_ne!(ResourceTypeId(1), ResourceTypeId(2));
}

#[test]
fn test_attribute_value_array() {
    let tags = AttributeValue::Array(vec![
        AttributeValue::String("tag1".into()),
        AttributeValue::String("tag2".into()),
        AttributeValue::String("tag3".into()),
    ]);

    match tags {
        AttributeValue::Array(ref items) => {
            assert_eq!(items.len(), 3);
        }
        _ => panic!("Expected array"),
    }
}

#[test]
fn test_complex_evaluation_context() {
    let ctx = EvaluationContext::new(
        Resource::new(ResourceTypeId(42))
            .with_attribute("name", AttributeValue::String("production-db".into()))
            .with_attribute("region", AttributeValue::String("us-west-2".into()))
            .with_attribute("replicas", AttributeValue::Int(3))
            .with_attribute("encrypted", AttributeValue::Bool(true)),
        Action::new(Operation::Deploy, "deployment-123")
            .with_attribute("method", AttributeValue::String("DEPLOY".into()))
            .with_attribute("strategy", AttributeValue::String("rolling".into())),
        Request {
            principal: Principal::new("ci-bot")
                .with_role("deployer")
                .with_role("monitoring")
                .with_attribute("service", AttributeValue::String("github-actions".into()))
                .with_attribute("build_id", AttributeValue::Int(12345)),
            timestamp: 1704067200,
            source_ip: Some("10.0.1.100".to_string()),
            metadata: {
                let mut m = HashMap::new();
                m.insert(
                    "commit_sha".to_string(),
                    AttributeValue::String("abc123def456".into()),
                );
                m.insert(
                    "branch".to_string(),
                    AttributeValue::String("main".into()),
                );
                m
            },
        },
    );

    // Verify resource attributes
    assert_eq!(ctx.resource.type_id, ResourceTypeId(42));
    assert_eq!(ctx.resource.attributes.len(), 4);

    // Verify action attributes
    assert_eq!(ctx.action.operation, Operation::Deploy);
    assert_eq!(ctx.action.attributes.len(), 2);

    // Verify request attributes
    assert_eq!(ctx.request.principal.id, "ci-bot");
    assert_eq!(ctx.request.principal.roles.len(), 2);
    assert_eq!(ctx.request.principal.attributes.len(), 2);
    assert_eq!(ctx.request.timestamp, 1704067200);
    assert_eq!(ctx.request.metadata.len(), 2);
}
