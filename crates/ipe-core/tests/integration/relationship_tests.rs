//! Integration tests for relationship-based authorization

use ipe_core::rar::{Action, EvaluationContext, Operation, Principal, Request, Resource};
use ipe_core::relationship::{RelationType, Relationship, RelationshipQuery, RelationshipStore};
use std::sync::Arc;

#[test]
fn test_direct_role_relationship() {
    let store = RelationshipStore::new_temp().unwrap();

    // Alice is an editor of document-123
    store
        .add_relationship(Relationship::role("alice", "editor", "document-123", "admin"))
        .unwrap();

    // Check direct relationship
    assert!(store.has_relationship("alice", "editor", "document-123").unwrap());
    assert!(!store.has_relationship("bob", "editor", "document-123").unwrap());
    assert!(!store.has_relationship("alice", "viewer", "document-123").unwrap());
}

#[test]
fn test_context_has_relationship() {
    let store = Arc::new(RelationshipStore::new_temp().unwrap());

    // Alice is an editor of document-123
    store
        .add_relationship(Relationship::role("alice", "editor", "document-123", "admin"))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Update, "document-123"),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    )
    .with_relationship_store(store);

    // Alice should be an editor of document-123
    assert!(ctx.has_relationship("editor", "document-123").unwrap());
    assert!(!ctx.has_relationship("viewer", "document-123").unwrap());
}

#[test]
fn test_trust_chain_two_levels() {
    let store = RelationshipStore::new_temp().unwrap();

    // Build trust chain: cert-1 -> intermediate-ca -> root-ca
    store
        .add_relationship(Relationship::trust("cert-1", "intermediate-ca", "pki"))
        .unwrap();

    store
        .add_relationship(Relationship::trust("intermediate-ca", "root-ca", "pki"))
        .unwrap();

    // Direct relationship
    assert!(store.has_relationship("cert-1", "trusted_by", "intermediate-ca").unwrap());

    // Transitive relationship (2 hops)
    assert!(store.has_transitive_relationship("cert-1", "trusted_by", "root-ca").unwrap());

    // No relationship to unrelated CA
    assert!(!store.has_transitive_relationship("cert-1", "trusted_by", "other-ca").unwrap());
}

#[test]
fn test_trust_chain_three_levels() {
    let store = RelationshipStore::new_temp().unwrap();

    // Build longer trust chain: leaf -> intermediate-1 -> intermediate-2 -> root
    store
        .add_relationship(Relationship::trust("leaf-cert", "intermediate-1", "pki"))
        .unwrap();
    store
        .add_relationship(Relationship::trust("intermediate-1", "intermediate-2", "pki"))
        .unwrap();
    store
        .add_relationship(Relationship::trust("intermediate-2", "root-ca", "pki"))
        .unwrap();

    // Should find path through chain
    assert!(store.has_transitive_relationship("leaf-cert", "trusted_by", "root-ca").unwrap());

    // Get the path
    let path = store
        .find_relationship_path("leaf-cert", "trusted_by", "root-ca")
        .unwrap()
        .expect("Path should exist");

    assert_eq!(path.depth, 3);
    assert_eq!(path.path[0].subject, "leaf-cert");
    assert_eq!(path.path[0].object, "intermediate-1");
    assert_eq!(path.path[1].subject, "intermediate-1");
    assert_eq!(path.path[1].object, "intermediate-2");
    assert_eq!(path.path[2].subject, "intermediate-2");
    assert_eq!(path.path[2].object, "root-ca");
}

#[test]
fn test_context_transitive_trust() {
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

    // Should be transitively trusted by root-ca
    assert!(ctx.has_transitive_relationship("trusted_by", "root-ca").unwrap());
}

#[test]
fn test_membership_hierarchy() {
    let store = RelationshipStore::new_temp().unwrap();

    // Build org hierarchy: alice -> engineers -> employees -> everyone
    store
        .add_relationship(Relationship::membership("alice", "engineers", "hr"))
        .unwrap();
    store
        .add_relationship(Relationship::membership("engineers", "employees", "hr"))
        .unwrap();
    store
        .add_relationship(Relationship::membership("employees", "everyone", "hr"))
        .unwrap();

    // Direct membership
    assert!(store.has_relationship("alice", "member_of", "engineers").unwrap());

    // Transitive memberships
    assert!(store.has_transitive_relationship("alice", "member_of", "employees").unwrap());
    assert!(store.has_transitive_relationship("alice", "member_of", "everyone").unwrap());

    // Not a member of unrelated group
    assert!(!store.has_transitive_relationship("alice", "member_of", "contractors").unwrap());
}

#[test]
fn test_non_transitive_role() {
    let store = RelationshipStore::new_temp().unwrap();

    // Roles are NOT transitive
    store
        .add_relationship(Relationship::role("alice", "editor", "document-1", "admin"))
        .unwrap();
    store
        .add_relationship(Relationship::role("document-1", "contains", "section-1", "system"))
        .unwrap();

    // Alice is editor of document-1
    assert!(store.has_relationship("alice", "editor", "document-1").unwrap());

    // But alice is NOT editor of section-1 (roles don't chain)
    assert!(!store.has_transitive_relationship("alice", "editor", "section-1").unwrap());
}

#[test]
fn test_relationship_expiration() {
    let store = RelationshipStore::new_temp().unwrap();

    // Add expired relationship
    let mut expired_rel = Relationship::role("alice", "editor", "document-123", "admin");
    expired_rel.expires_at = Some(chrono::Utc::now().timestamp() - 100);
    store.add_relationship(expired_rel).unwrap();

    // Expired relationship should not be found
    assert!(!store.has_relationship("alice", "editor", "document-123").unwrap());
}

#[test]
fn test_relationship_with_metadata() {
    let store = RelationshipStore::new_temp().unwrap();

    // Add relationship with rich metadata
    store
        .add_relationship(
            Relationship::role("alice", "editor", "document-123", "admin")
                .with_metadata("granted_date", "2024-01-15")
                .with_metadata("ticket", "JIRA-456")
                .with_metadata("scope", "full-access"),
        )
        .unwrap();

    let rel = store
        .get_relationship("alice", "editor", "document-123")
        .unwrap()
        .expect("Relationship should exist");

    assert_eq!(rel.metadata.get("ticket").unwrap(), "JIRA-456");
    assert_eq!(rel.metadata.get("scope").unwrap(), "full-access");
}

#[test]
fn test_multiple_roles_same_subject() {
    let store = RelationshipStore::new_temp().unwrap();

    // Alice has multiple roles on different documents
    store
        .add_relationship(Relationship::role("alice", "editor", "doc-1", "admin"))
        .unwrap();
    store
        .add_relationship(Relationship::role("alice", "viewer", "doc-2", "admin"))
        .unwrap();
    store
        .add_relationship(Relationship::role("alice", "owner", "doc-3", "admin"))
        .unwrap();

    assert!(store.has_relationship("alice", "editor", "doc-1").unwrap());
    assert!(store.has_relationship("alice", "viewer", "doc-2").unwrap());
    assert!(store.has_relationship("alice", "owner", "doc-3").unwrap());

    // List all of alice's relationships
    let rels = store.list_subject_relationships("alice").unwrap();
    assert_eq!(rels.len(), 3);
}

#[test]
fn test_remove_relationship() {
    let store = RelationshipStore::new_temp().unwrap();

    store
        .add_relationship(Relationship::role("alice", "editor", "document-123", "admin"))
        .unwrap();
    assert!(store.has_relationship("alice", "editor", "document-123").unwrap());

    store.remove_relationship("alice", "editor", "document-123").unwrap();
    assert!(!store.has_relationship("alice", "editor", "document-123").unwrap());
}

#[test]
fn test_batch_relationship_checks() {
    let store = RelationshipStore::new_temp().unwrap();

    // Setup some relationships
    store
        .add_relationship(Relationship::role("alice", "editor", "doc-1", "admin"))
        .unwrap();
    store
        .add_relationship(Relationship::role("bob", "viewer", "doc-2", "admin"))
        .unwrap();

    let queries = vec![
        RelationshipQuery::new("alice", "editor", "doc-1"),
        RelationshipQuery::new("bob", "viewer", "doc-2"),
        RelationshipQuery::new("charlie", "owner", "doc-3"),
    ];

    let results = store.check_relationships(queries).unwrap();
    assert_eq!(results, vec![true, true, false]);
}

#[test]
fn test_complex_trust_graph() {
    let store = RelationshipStore::new_temp().unwrap();

    // Build a more complex trust graph
    //       root-ca
    //      /       \
    //  intermediate-1  intermediate-2
    //      |               |
    //   cert-1          cert-2

    store
        .add_relationship(Relationship::trust("intermediate-1", "root-ca", "pki"))
        .unwrap();
    store
        .add_relationship(Relationship::trust("intermediate-2", "root-ca", "pki"))
        .unwrap();
    store
        .add_relationship(Relationship::trust("cert-1", "intermediate-1", "pki"))
        .unwrap();
    store
        .add_relationship(Relationship::trust("cert-2", "intermediate-2", "pki"))
        .unwrap();

    // Both certs should trust root-ca through different paths
    assert!(store.has_transitive_relationship("cert-1", "trusted_by", "root-ca").unwrap());
    assert!(store.has_transitive_relationship("cert-2", "trusted_by", "root-ca").unwrap());

    // Certs should not trust each other
    assert!(!store.has_transitive_relationship("cert-1", "trusted_by", "cert-2").unwrap());
}

#[test]
fn test_max_depth_prevents_cycles() {
    let store = RelationshipStore::new_temp().unwrap().with_max_depth(5);

    // Create a long chain
    for i in 0..20 {
        store
            .add_relationship(Relationship::trust(
                format!("node-{}", i),
                format!("node-{}", i + 1),
                "system",
            ))
            .unwrap();
    }

    // Should fail due to max depth
    let result = store.find_relationship_path("node-0", "trusted_by", "node-20");
    assert!(result.is_err());
}

#[test]
fn test_shortest_path_found() {
    let store = RelationshipStore::new_temp().unwrap();

    // Create two paths from A to C:
    // 1. A -> B -> C (2 hops)
    // 2. A -> D -> E -> C (3 hops)
    store.add_relationship(Relationship::trust("A", "B", "system")).unwrap();
    store.add_relationship(Relationship::trust("B", "C", "system")).unwrap();
    store.add_relationship(Relationship::trust("A", "D", "system")).unwrap();
    store.add_relationship(Relationship::trust("D", "E", "system")).unwrap();
    store.add_relationship(Relationship::trust("E", "C", "system")).unwrap();

    let path = store
        .find_relationship_path("A", "trusted_by", "C")
        .unwrap()
        .expect("Path should exist");

    // BFS should find shortest path (2 hops)
    assert_eq!(path.depth, 2);
}

#[test]
fn test_delegation_chain() {
    let store = RelationshipStore::new_temp().unwrap();

    // Admin delegates to manager, manager delegates to alice
    store
        .add_relationship(Relationship::new(
            "manager",
            "can_delegate_from",
            "admin",
            RelationType::Delegation,
            "system",
        ))
        .unwrap();

    store
        .add_relationship(Relationship::new(
            "alice",
            "can_delegate_from",
            "manager",
            RelationType::Delegation,
            "system",
        ))
        .unwrap();

    // Delegation is not transitive by default
    assert!(store.has_relationship("alice", "can_delegate_from", "manager").unwrap());
    assert!(!store
        .has_transitive_relationship("alice", "can_delegate_from", "admin")
        .unwrap());
}

#[test]
fn test_ownership_relationships() {
    let store = RelationshipStore::new_temp().unwrap();

    // Alice owns project-1, project-1 owns resources
    store
        .add_relationship(Relationship::new(
            "alice",
            "owner",
            "project-1",
            RelationType::Ownership,
            "system",
        ))
        .unwrap();

    store
        .add_relationship(Relationship::new(
            "project-1",
            "owner",
            "database-1",
            RelationType::Ownership,
            "system",
        ))
        .unwrap();

    // Ownership is not transitive (alice doesn't own database-1 directly)
    assert!(store.has_relationship("alice", "owner", "project-1").unwrap());
    assert!(!store.has_transitive_relationship("alice", "owner", "database-1").unwrap());
}

#[test]
fn test_custom_relationship_type() {
    let store = RelationshipStore::new_temp().unwrap();

    store
        .add_relationship(Relationship::new(
            "service-a",
            "depends_on",
            "service-b",
            RelationType::Custom("dependency".into()),
            "devops",
        ))
        .unwrap();

    assert!(store.has_relationship("service-a", "depends_on", "service-b").unwrap());
}

#[test]
fn test_no_relationship_store_error() {
    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Read, "document"),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    );

    // Should error when no relationship store configured
    let result = ctx.has_relationship("editor", "document-123");
    assert!(result.is_err());
}

#[test]
fn test_combined_approval_and_relationship() {
    let approval_store = Arc::new(ipe_core::approval::ApprovalStore::new_temp().unwrap());
    let relationship_store = Arc::new(RelationshipStore::new_temp().unwrap());

    // Alice is an editor of document-123
    relationship_store
        .add_relationship(Relationship::role("alice", "editor", "document-123", "admin"))
        .unwrap();

    // Alice has approval to modify document-123
    approval_store
        .grant_approval(ipe_core::approval::Approval::new(
            "alice",
            "document-123",
            "UPDATE",
            "admin",
        ))
        .unwrap();

    let ctx = EvaluationContext::new(
        Resource::url("document-123"),
        Action::new(Operation::Update, "document-123")
            .with_attribute("method", ipe_core::rar::AttributeValue::String("UPDATE".into())),
        Request {
            principal: Principal::user("alice"),
            ..Default::default()
        },
    )
    .with_approval_store(approval_store)
    .with_relationship_store(relationship_store);

    // Both checks should pass
    assert!(ctx.has_relationship("editor", "document-123").unwrap());
    assert!(ctx.has_approval().unwrap());
}

#[test]
fn test_list_subject_relationships() {
    let store = RelationshipStore::new_temp().unwrap();

    // Alice has multiple relationships
    store
        .add_relationship(Relationship::role("alice", "editor", "doc-1", "admin"))
        .unwrap();
    store
        .add_relationship(Relationship::role("alice", "viewer", "doc-2", "admin"))
        .unwrap();
    store
        .add_relationship(Relationship::membership("alice", "engineers", "hr"))
        .unwrap();

    // Bob has one relationship
    store
        .add_relationship(Relationship::role("bob", "viewer", "doc-3", "admin"))
        .unwrap();

    let alice_rels = store.list_subject_relationships("alice").unwrap();
    assert_eq!(alice_rels.len(), 3);

    let bob_rels = store.list_subject_relationships("bob").unwrap();
    assert_eq!(bob_rels.len(), 1);
}

#[test]
fn test_count_relationships() {
    let store = RelationshipStore::new_temp().unwrap();

    assert_eq!(store.count_relationships().unwrap(), 0);

    store
        .add_relationship(Relationship::role("alice", "editor", "doc-1", "admin"))
        .unwrap();
    assert_eq!(store.count_relationships().unwrap(), 1);

    store
        .add_relationship(Relationship::role("bob", "viewer", "doc-2", "admin"))
        .unwrap();
    assert_eq!(store.count_relationships().unwrap(), 2);
}

#[test]
fn test_relationship_type_display() {
    assert_eq!(RelationType::Role.to_string(), "role");
    assert_eq!(RelationType::Trust.to_string(), "trust");
    assert_eq!(RelationType::Membership.to_string(), "membership");
    assert_eq!(RelationType::Custom("foo".into()).to_string(), "foo");
}

#[test]
fn test_relationship_type_transitivity() {
    assert!(!RelationType::Role.is_transitive());
    assert!(RelationType::Trust.is_transitive());
    assert!(RelationType::Membership.is_transitive());
    assert!(!RelationType::Ownership.is_transitive());
    assert!(!RelationType::Delegation.is_transitive());
}

#[test]
fn test_empty_field_validation() {
    let store = RelationshipStore::new_temp().unwrap();

    // Empty subject
    let rel = Relationship::role("", "editor", "doc", "admin");
    assert!(store.add_relationship(rel).is_err());

    // Empty relation
    let rel = Relationship::new("alice", "", "doc", RelationType::Role, "admin");
    assert!(store.add_relationship(rel).is_err());

    // Empty object
    let rel = Relationship::role("alice", "editor", "", "admin");
    assert!(store.add_relationship(rel).is_err());
}

#[test]
fn test_large_relationship_graph() {
    let store = RelationshipStore::new_temp().unwrap();

    // Create a large graph: 100 users, each with 10 relationships
    for user_id in 0..100 {
        for doc_id in 0..10 {
            store
                .add_relationship(Relationship::role(
                    format!("user-{}", user_id),
                    "editor",
                    format!("doc-{}", doc_id),
                    "admin",
                ))
                .unwrap();
        }
    }

    assert_eq!(store.count_relationships().unwrap(), 1000);

    // Verify random access
    assert!(store.has_relationship("user-50", "editor", "doc-5").unwrap());
    assert!(!store.has_relationship("user-50", "viewer", "doc-5").unwrap());
}
