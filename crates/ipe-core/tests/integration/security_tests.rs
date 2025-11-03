//! Security and validation tests for approval system

use ipe_core::approval::{Approval, ApprovalStore};
use ipe_core::rar::{
    Action, AttributeValue, EvaluationContext, Operation, Principal, Request, Resource,
};
use std::sync::Arc;

#[test]
fn test_case_sensitivity_identity() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval to lowercase "bot-123"
    store
        .grant_approval(Approval::new("bot-123", "https://api.example.com/data", "GET", "admin"))
        .unwrap();

    // Try with uppercase - should NOT match (case-sensitive)
    assert!(!store.has_approval("Bot-123", "https://api.example.com/data", "GET").unwrap());
    assert!(!store.has_approval("BOT-123", "https://api.example.com/data", "GET").unwrap());

    // Original lowercase should still work
    assert!(store.has_approval("bot-123", "https://api.example.com/data", "GET").unwrap());
}

#[test]
fn test_case_sensitivity_resource() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval with specific casing
    store
        .grant_approval(Approval::new("bot-123", "https://api.example.com/Data", "GET", "admin"))
        .unwrap();

    // Different casing should NOT match
    assert!(!store.has_approval("bot-123", "https://api.example.com/data", "GET").unwrap());
    assert!(!store.has_approval("bot-123", "https://api.example.com/DATA", "GET").unwrap());

    // Exact match should work
    assert!(store.has_approval("bot-123", "https://api.example.com/Data", "GET").unwrap());
}

#[test]
fn test_case_sensitivity_action() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant approval for lowercase "get"
    store
        .grant_approval(Approval::new("bot-123", "https://api.example.com/data", "get", "admin"))
        .unwrap();

    // Different casing should NOT match
    assert!(!store.has_approval("bot-123", "https://api.example.com/data", "GET").unwrap());
    assert!(!store.has_approval("bot-123", "https://api.example.com/data", "Get").unwrap());

    // Exact match should work
    assert!(store.has_approval("bot-123", "https://api.example.com/data", "get").unwrap());
}

#[test]
fn test_null_byte_in_identity() {
    let store = ApprovalStore::new_temp().unwrap();

    // Try to create approval with null byte in identity
    let approval = Approval::new("bot\0evil", "https://api.example.com/data", "GET", "admin");

    // Should be able to grant (no validation at approval level)
    store.grant_approval(approval).unwrap();

    // But lookup must be exact match (null byte preserved)
    assert!(store.has_approval("bot\0evil", "https://api.example.com/data", "GET").unwrap());
    assert!(!store.has_approval("bot", "https://api.example.com/data", "GET").unwrap());
    assert!(!store.has_approval("botevil", "https://api.example.com/data", "GET").unwrap());
}

#[test]
fn test_unicode_normalization() {
    let store = ApprovalStore::new_temp().unwrap();

    // NFD form: é as e + combining accent
    let identity_nfd = "café".to_string();
    // NFC form: é as single character
    let identity_nfc = "café".to_string();

    // Grant to NFC form
    store
        .grant_approval(Approval::new(identity_nfc.clone(), "resource", "GET", "admin"))
        .unwrap();

    // NFD and NFC forms should be treated as different (no normalization)
    // This documents current behavior - may want to add normalization in future
    if identity_nfd != identity_nfc {
        assert!(!store.has_approval(&identity_nfd, "resource", "GET").unwrap());
    } else {
        // If they're the same, then normalization happened at compile time
        assert!(store.has_approval(&identity_nfc, "resource", "GET").unwrap());
    }
}

#[test]
fn test_very_long_identity() {
    let store = ApprovalStore::new_temp().unwrap();

    // Create very long identity (10KB)
    let long_identity = "bot-".to_string() + &"x".repeat(10_000);

    store
        .grant_approval(Approval::new(&long_identity, "resource", "GET", "admin"))
        .unwrap();

    assert!(store.has_approval(&long_identity, "resource", "GET").unwrap());
}

#[test]
fn test_very_long_resource_url() {
    let store = ApprovalStore::new_temp().unwrap();

    // Create very long URL (100KB)
    let long_url = "https://api.example.com/data?param=".to_string() + &"x".repeat(100_000);

    store
        .grant_approval(Approval::new("bot-123", &long_url, "GET", "admin"))
        .unwrap();

    assert!(store.has_approval("bot-123", &long_url, "GET").unwrap());
}

#[test]
fn test_large_metadata() {
    let store = ApprovalStore::new_temp().unwrap();

    // Create approval with large metadata (1MB value)
    let large_value = "x".repeat(1_024 * 1024);
    let approval = Approval::new("bot-123", "resource", "GET", "admin")
        .with_metadata("large_field", large_value.clone());

    store.grant_approval(approval).unwrap();

    let retrieved = store
        .get_approval("bot-123", "resource", "GET")
        .unwrap()
        .expect("Approval should exist");

    assert_eq!(retrieved.metadata.get("large_field").unwrap(), &large_value);
}

#[test]
fn test_whitespace_handling() {
    let store = ApprovalStore::new_temp().unwrap();

    // Grant with leading/trailing whitespace
    store
        .grant_approval(Approval::new(" bot-123 ", " resource ", " GET ", "admin"))
        .unwrap();

    // Exact match required (whitespace preserved)
    assert!(store.has_approval(" bot-123 ", " resource ", " GET ").unwrap());
    assert!(!store.has_approval("bot-123", "resource", "GET").unwrap());
}

#[test]
fn test_special_characters_in_resource() {
    let store = ApprovalStore::new_temp().unwrap();

    // URL with special characters
    let resource = "https://api.example.com/data?foo=bar&baz=qux#anchor";

    store
        .grant_approval(Approval::new("bot-123", resource, "GET", "admin"))
        .unwrap();

    assert!(store.has_approval("bot-123", resource, "GET").unwrap());
}

#[test]
fn test_sql_injection_like_patterns() {
    let store = ApprovalStore::new_temp().unwrap();

    // Try SQL-like injection patterns (should be treated as literals)
    let malicious_identity = "bot-123'; DROP TABLE approvals; --";

    store
        .grant_approval(Approval::new(malicious_identity, "resource", "GET", "admin"))
        .unwrap();

    // Should be stored and retrieved as literal string
    assert!(store.has_approval(malicious_identity, "resource", "GET").unwrap());

    // Verify database still works
    assert_eq!(store.count_approvals().unwrap(), 1);
}

#[test]
fn test_empty_string_fields() {
    let store = ApprovalStore::new_temp().unwrap();

    // Empty identity should be rejected
    let empty_identity = Approval::new("", "resource", "GET", "admin");
    assert!(store.grant_approval(empty_identity).is_err());

    // Empty resource should be rejected
    let empty_resource = Approval::new("bot-123", "", "GET", "admin");
    assert!(store.grant_approval(empty_resource).is_err());

    // Empty action should be rejected
    let empty_action = Approval::new("bot-123", "resource", "", "admin");
    assert!(store.grant_approval(empty_action).is_err());
}

#[test]
fn test_missing_resource_url_attribute() {
    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Grant approval using action.target as fallback
    store
        .grant_approval(Approval::new("bot-123", "fallback-target", "GET", "admin"))
        .unwrap();

    // Create resource WITHOUT "url" attribute
    let mut resource = Resource::new(ipe_core::rar::ResourceTypeId(0));
    resource
        .attributes
        .insert("name".into(), AttributeValue::String("some-resource".into()));

    let ctx = EvaluationContext::new(
        resource,
        Action::new(Operation::Read, "fallback-target")
            .with_attribute("method", AttributeValue::String("GET".into())),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    // Should use action.target as fallback for resource URL
    assert!(ctx.has_approval().unwrap());
}

#[test]
fn test_missing_action_method_attribute() {
    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Grant approval using Debug format of Operation
    // Operation::Read -> "Read"
    store
        .grant_approval(Approval::new("bot-123", "https://api.example.com/data", "Read", "admin"))
        .unwrap();

    // Create action WITHOUT "method" attribute
    let ctx = EvaluationContext::new(
        Resource::url("https://api.example.com/data"),
        Action::new(Operation::Read, "data"),
        Request {
            principal: Principal::bot("bot-123"),
            ..Default::default()
        },
    )
    .with_approval_store(store);

    // Should use Debug format of Operation as fallback
    assert!(ctx.has_approval().unwrap());
}

#[test]
fn test_expiration_at_exact_timestamp() {
    let store = ApprovalStore::new_temp().unwrap();

    // Create approval that expires at current timestamp
    let now = chrono::Utc::now().timestamp();
    let mut approval = Approval::new("bot-123", "resource", "GET", "admin");
    approval.expires_at = Some(now);

    store.grant_approval(approval).unwrap();

    // Should be expired (>= check)
    assert!(!store.has_approval("bot-123", "resource", "GET").unwrap());
}

#[test]
fn test_concurrent_grant_same_approval() {
    use std::thread;

    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Multiple threads try to grant the same approval
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let store = store.clone();
            thread::spawn(move || {
                store
                    .grant_approval(Approval::new(
                        "bot-123",
                        "resource",
                        "GET",
                        &format!("admin-{}", i),
                    ))
                    .unwrap();
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    // Last write wins - should have exactly 1 approval
    let approvals = store.list_approvals("bot-123").unwrap();
    assert_eq!(approvals.len(), 1);
}

#[test]
fn test_concurrent_grant_revoke() {
    use std::thread;
    use std::time::Duration;

    let store = Arc::new(ApprovalStore::new_temp().unwrap());

    // Initial grant
    store
        .grant_approval(Approval::new("bot-123", "resource", "GET", "admin"))
        .unwrap();

    // Thread 1: Repeatedly grant
    let store1 = store.clone();
    let grant_handle = thread::spawn(move || {
        for _ in 0..100 {
            store1
                .grant_approval(Approval::new("bot-123", "resource", "GET", "admin"))
                .unwrap();
            thread::sleep(Duration::from_micros(10));
        }
    });

    // Thread 2: Repeatedly revoke
    let store2 = store.clone();
    let revoke_handle = thread::spawn(move || {
        for _ in 0..100 {
            let _ = store2.revoke_approval("bot-123", "resource", "GET");
            thread::sleep(Duration::from_micros(10));
        }
    });

    grant_handle.join().unwrap();
    revoke_handle.join().unwrap();

    // Final state should be consistent (either exists or doesn't)
    let result = store.has_approval("bot-123", "resource", "GET").unwrap();
    // Just verify we don't panic or get inconsistent state
    println!("Final state: approval exists = {}", result);
}
