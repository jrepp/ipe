//! Integration tests for scope and TTL features

use ipe_core::approval::{Approval, ApprovalStore, Scope, TTLConfig};
use ipe_core::relationship::{Relationship, RelationshipStore};
use std::sync::Arc;

// ============================================================================
// Scope Tests - Approvals
// ============================================================================

#[test]
fn test_approval_tenant_scope() {
    let store = ApprovalStore::new_temp().unwrap();
    let acme_scope = Scope::tenant("acme-corp");
    let widgets_scope = Scope::tenant("widgets-inc");

    // Grant approval in Acme Corp tenant
    store
        .grant_approval(
            Approval::new("alice", "document-1", "GET", "admin").with_scope(acme_scope.clone()),
        )
        .unwrap();

    // Grant approval in Widgets Inc tenant (different alice)
    store
        .grant_approval(
            Approval::new("alice", "document-1", "GET", "admin").with_scope(widgets_scope.clone()),
        )
        .unwrap();

    // Each scope sees only their own approvals
    assert!(store.has_approval_in_scope("alice", "document-1", "GET", &acme_scope).unwrap());
    assert!(store
        .has_approval_in_scope("alice", "document-1", "GET", &widgets_scope)
        .unwrap());

    // Different scopes are isolated
    let acme_approvals = store.list_approvals_in_scope("alice", &acme_scope).unwrap();
    let widgets_approvals = store.list_approvals_in_scope("alice", &widgets_scope).unwrap();

    assert_eq!(acme_approvals.len(), 1);
    assert_eq!(widgets_approvals.len(), 1);
    assert_eq!(acme_approvals[0].scope, acme_scope);
    assert_eq!(widgets_approvals[0].scope, widgets_scope);
}

#[test]
fn test_approval_environment_scope() {
    let store = ApprovalStore::new_temp().unwrap();
    let dev_scope = Scope::env("dev");
    let prod_scope = Scope::env("prod");

    // Grant in dev environment
    store
        .grant_approval(
            Approval::new("test-user", "api-endpoint", "POST", "dev-admin")
                .with_scope(dev_scope.clone())
                .with_ttl(3600), // Short TTL for dev
        )
        .unwrap();

    // Grant in prod environment
    store
        .grant_approval(
            Approval::new("prod-user", "api-endpoint", "POST", "prod-admin")
                .with_scope(prod_scope.clone())
                .with_ttl(30 * 24 * 3600), // Long TTL for prod
        )
        .unwrap();

    // Each environment is isolated
    assert!(store
        .has_approval_in_scope("test-user", "api-endpoint", "POST", &dev_scope)
        .unwrap());
    assert!(store
        .has_approval_in_scope("prod-user", "api-endpoint", "POST", &prod_scope)
        .unwrap());

    // Cross-environment access fails
    assert!(!store
        .has_approval_in_scope("test-user", "api-endpoint", "POST", &prod_scope)
        .unwrap());
}

#[test]
fn test_approval_tenant_environment_scope() {
    let store = ApprovalStore::new_temp().unwrap();

    let acme_dev = Scope::tenant_env("acme-corp", "dev");
    let acme_prod = Scope::tenant_env("acme-corp", "prod");
    let widgets_dev = Scope::tenant_env("widgets-inc", "dev");

    // Grant in Acme Corp dev
    store
        .grant_approval(
            Approval::new("alice", "resource", "GET", "admin").with_scope(acme_dev.clone()),
        )
        .unwrap();

    // Grant in Acme Corp prod
    store
        .grant_approval(
            Approval::new("alice", "resource", "GET", "admin").with_scope(acme_prod.clone()),
        )
        .unwrap();

    // Grant in Widgets Inc dev
    store
        .grant_approval(
            Approval::new("bob", "resource", "GET", "admin").with_scope(widgets_dev.clone()),
        )
        .unwrap();

    // Each tenant+environment combination is isolated
    assert!(store.has_approval_in_scope("alice", "resource", "GET", &acme_dev).unwrap());
    assert!(store.has_approval_in_scope("alice", "resource", "GET", &acme_prod).unwrap());
    assert!(store.has_approval_in_scope("bob", "resource", "GET", &widgets_dev).unwrap());

    // Cross-tenant access fails
    assert!(!store.has_approval_in_scope("alice", "resource", "GET", &widgets_dev).unwrap());
}

#[test]
fn test_approval_custom_scope() {
    let store = ApprovalStore::new_temp().unwrap();

    let custom_scope =
        Scope::Custom(vec!["region".to_string(), "us-west".to_string(), "zone-a".to_string()]);

    store
        .grant_approval(
            Approval::new("service", "resource", "GET", "admin").with_scope(custom_scope.clone()),
        )
        .unwrap();

    assert!(store
        .has_approval_in_scope("service", "resource", "GET", &custom_scope)
        .unwrap());
}

#[test]
fn test_approval_scope_revocation() {
    let store = ApprovalStore::new_temp().unwrap();
    let acme_scope = Scope::tenant("acme-corp");

    // Grant approval in specific scope
    store
        .grant_approval(
            Approval::new("alice", "document", "GET", "admin").with_scope(acme_scope.clone()),
        )
        .unwrap();

    assert!(store.has_approval_in_scope("alice", "document", "GET", &acme_scope).unwrap());

    // Revoke in correct scope
    store.revoke_approval_in_scope("alice", "document", "GET", &acme_scope).unwrap();

    assert!(!store.has_approval_in_scope("alice", "document", "GET", &acme_scope).unwrap());
}

#[test]
fn test_approval_global_scope_backward_compatibility() {
    let store = ApprovalStore::new_temp().unwrap();

    // Old code: no scope specified (defaults to Global)
    store
        .grant_approval(Approval::new("alice", "resource", "GET", "admin"))
        .unwrap();

    // Can retrieve with default methods (Global scope)
    assert!(store.has_approval("alice", "resource", "GET").unwrap());

    // Can also explicitly query Global scope
    assert!(store.has_approval_in_scope("alice", "resource", "GET", &Scope::Global).unwrap());
}

// ============================================================================
// TTL Tests - Approvals
// ============================================================================

#[test]
fn test_approval_with_ttl() {
    let store = ApprovalStore::new_temp().unwrap();

    let approval = Approval::new("user", "resource", "GET", "admin").with_ttl(3600); // 1 hour

    assert_eq!(approval.ttl_seconds, Some(3600));
    assert!(approval.expires_at.is_some());
    assert!(!approval.is_expired());

    store.grant_approval(approval).unwrap();

    let retrieved = store.get_approval("user", "resource", "GET").unwrap().unwrap();
    assert_eq!(retrieved.ttl_seconds, Some(3600));
}

#[test]
fn test_ttl_config_presets() {
    let temp_config = TTLConfig::temporary();
    assert_eq!(temp_config.default_ttl_seconds, Some(3600));
    assert_eq!(temp_config.min_ttl_seconds, 60);
    assert_eq!(temp_config.max_ttl_seconds, 24 * 3600);

    let short_config = TTLConfig::short_lived();
    assert_eq!(short_config.default_ttl_seconds, Some(24 * 3600));

    let long_config = TTLConfig::long_lived();
    assert_eq!(long_config.default_ttl_seconds, Some(30 * 24 * 3600));
}

#[test]
fn test_approval_ttl_and_scope_combined() {
    let store = ApprovalStore::new_temp().unwrap();
    let acme_dev = Scope::tenant_env("acme-corp", "dev");

    // Grant with both scope and TTL
    store
        .grant_approval(
            Approval::new("test-user", "api", "POST", "dev-admin")
                .with_scope(acme_dev.clone())
                .with_ttl(3600),
        )
        .unwrap();

    let approval = store
        .get_approval_in_scope("test-user", "api", "POST", &acme_dev)
        .unwrap()
        .unwrap();

    assert_eq!(approval.scope, acme_dev);
    assert_eq!(approval.ttl_seconds, Some(3600));
    assert!(!approval.is_expired());
}

// ============================================================================
// Scope Tests - Relationships
// ============================================================================

#[test]
fn test_relationship_tenant_scope() {
    let store = RelationshipStore::new_temp().unwrap();
    let acme_scope = Scope::tenant("acme-corp");
    let widgets_scope = Scope::tenant("widgets-inc");

    // Alice is editor in Acme Corp
    store
        .add_relationship(
            Relationship::role("alice", "editor", "document-1", "admin")
                .with_scope(acme_scope.clone()),
        )
        .unwrap();

    // Alice is also editor in Widgets Inc (different tenant)
    store
        .add_relationship(
            Relationship::role("alice", "editor", "document-1", "admin")
                .with_scope(widgets_scope.clone()),
        )
        .unwrap();

    // Both relationships exist in their respective scopes
    let acme_rel = store
        .get_relationship_in_scope("alice", "editor", "document-1", &acme_scope)
        .unwrap()
        .unwrap();
    let widgets_rel = store
        .get_relationship_in_scope("alice", "editor", "document-1", &widgets_scope)
        .unwrap()
        .unwrap();

    assert_eq!(acme_rel.scope, acme_scope);
    assert_eq!(widgets_rel.scope, widgets_scope);
}

#[test]
fn test_relationship_with_ttl() {
    let store = RelationshipStore::new_temp().unwrap();

    let relationship =
        Relationship::role("alice", "temp-editor", "document", "admin").with_ttl(3600);

    assert_eq!(relationship.ttl_seconds, Some(3600));
    assert!(relationship.expires_at.is_some());
    assert!(!relationship.is_expired());

    store.add_relationship(relationship).unwrap();
}

#[test]
fn test_relationship_scope_and_ttl_combined() {
    let store = RelationshipStore::new_temp().unwrap();
    let dev_scope = Scope::env("dev");

    // Temporary role in dev environment
    store
        .add_relationship(
            Relationship::role("test-user", "editor", "test-doc", "dev-admin")
                .with_scope(dev_scope.clone())
                .with_ttl(7200), // 2 hours
        )
        .unwrap();

    let relationship = store
        .get_relationship_in_scope("test-user", "editor", "test-doc", &dev_scope)
        .unwrap()
        .unwrap();

    assert_eq!(relationship.scope, dev_scope);
    assert_eq!(relationship.ttl_seconds, Some(7200));
}

#[test]
fn test_relationship_trust_chain_with_scope() {
    let store = RelationshipStore::new_temp().unwrap();
    let pki_scope = Scope::Custom(vec!["pki".to_string(), "production".to_string()]);

    // Build trust chain in PKI scope
    store
        .add_relationship(
            Relationship::trust("cert-1", "intermediate-ca", "pki").with_scope(pki_scope.clone()),
        )
        .unwrap();

    store
        .add_relationship(
            Relationship::trust("intermediate-ca", "root-ca", "pki").with_scope(pki_scope.clone()),
        )
        .unwrap();

    // Transitive trust should work within scope
    // Note: Current implementation searches across scopes,
    // but the relationships are stored with scope
    let path = store.find_relationship_path("cert-1", "trusted_by", "root-ca").unwrap();

    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.depth, 2);
    assert_eq!(path.path[0].scope, pki_scope);
    assert_eq!(path.path[1].scope, pki_scope);
}

// ============================================================================
// Multi-tenant Scenarios
// ============================================================================

#[test]
fn test_multi_tenant_isolation() {
    let approval_store = Arc::new(ApprovalStore::new_temp().unwrap());
    let rel_store = Arc::new(RelationshipStore::new_temp().unwrap());

    let tenant_a = Scope::tenant("tenant-a");
    let tenant_b = Scope::tenant("tenant-b");

    // Tenant A: alice is editor with approval
    rel_store
        .add_relationship(
            Relationship::role("alice", "editor", "doc-1", "admin-a").with_scope(tenant_a.clone()),
        )
        .unwrap();
    approval_store
        .grant_approval(
            Approval::new("alice", "doc-1", "UPDATE", "admin-a").with_scope(tenant_a.clone()),
        )
        .unwrap();

    // Tenant B: alice is viewer (different person, same name)
    rel_store
        .add_relationship(
            Relationship::role("alice", "viewer", "doc-1", "admin-b").with_scope(tenant_b.clone()),
        )
        .unwrap();
    approval_store
        .grant_approval(
            Approval::new("alice", "doc-1", "READ", "admin-b").with_scope(tenant_b.clone()),
        )
        .unwrap();

    // Verify isolation
    assert!(rel_store
        .get_relationship_in_scope("alice", "editor", "doc-1", &tenant_a)
        .unwrap()
        .is_some());
    assert!(rel_store
        .get_relationship_in_scope("alice", "editor", "doc-1", &tenant_b)
        .unwrap()
        .is_none());

    assert!(approval_store
        .has_approval_in_scope("alice", "doc-1", "UPDATE", &tenant_a)
        .unwrap());
    assert!(!approval_store
        .has_approval_in_scope("alice", "doc-1", "UPDATE", &tenant_b)
        .unwrap());
}

#[test]
fn test_environment_promotion_workflow() {
    let store = ApprovalStore::new_temp().unwrap();

    let dev = Scope::tenant_env("acme", "dev");
    let staging = Scope::tenant_env("acme", "staging");
    let prod = Scope::tenant_env("acme", "prod");

    // Grant temporary access in dev
    store
        .grant_approval(
            Approval::new("developer", "feature-x", "DEPLOY", "tech-lead")
                .with_scope(dev)
                .with_ttl(3600), // 1 hour
        )
        .unwrap();

    // Grant longer access in staging for validation
    store
        .grant_approval(
            Approval::new("qa-team", "feature-x", "TEST", "tech-lead")
                .with_scope(staging)
                .with_ttl(7 * 24 * 3600), // 1 week
        )
        .unwrap();

    // Grant permanent access in prod after approval
    store
        .grant_approval(
            Approval::new("release-manager", "feature-x", "DEPLOY", "vp-eng").with_scope(prod),
            // No TTL - permanent
        )
        .unwrap();

    // Verify different TTLs per environment
    let dev_approval = store
        .get_approval_in_scope(
            "developer",
            "feature-x",
            "DEPLOY",
            &Scope::tenant_env("acme", "dev"),
        )
        .unwrap()
        .unwrap();
    assert_eq!(dev_approval.ttl_seconds, Some(3600));

    let prod_approval = store
        .get_approval_in_scope(
            "release-manager",
            "feature-x",
            "DEPLOY",
            &Scope::tenant_env("acme", "prod"),
        )
        .unwrap()
        .unwrap();
    assert_eq!(prod_approval.ttl_seconds, None);
}

#[test]
fn test_scope_encoding() {
    assert_eq!(Scope::Global.encode(), "global");
    assert_eq!(Scope::tenant("acme").encode(), "tenant:acme");
    assert_eq!(Scope::env("prod").encode(), "env:prod");
    assert_eq!(Scope::tenant_env("acme", "prod").encode(), "tenant:acme:env:prod");
    assert_eq!(
        Scope::Custom(vec!["region".into(), "us-west".into()]).encode(),
        "custom:region:us-west"
    );
}
