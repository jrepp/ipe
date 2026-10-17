use super::*;
use crate::policy_scenarios::ScenarioSuite;
use crate::runtime_facts::{FactMutation, FactStore};
use serde_json::json;

fn suite() -> ScenarioSuite {
    serde_json::from_str(include_str!("../../../acceptance/policy-data/pr-landing.json")).unwrap()
}
fn environment(suite: &ScenarioSuite) -> CompileEnvironment {
    let mut environment = CompileEnvironment::new(suite.input_schema.clone());
    for (name, config) in &suite.collections {
        environment = environment.collection(name, config.schema.clone());
    }
    environment
}
fn scope() -> Scope {
    Scope::new("acme", "production")
}
fn seed(store: &FactStore, suite: &ScenarioSuite) {
    store
        .register_collection(scope(), "pull_requests", suite.collections["pull_requests"].clone())
        .unwrap();
    store
        .commit(
            &scope(),
            "pull_requests",
            "forge-runtime",
            vec![FactMutation {
                key: "commit-abc".into(),
                value: Some(json!({"approvers":["alice","bob"],"passed_checks":["test","lint"]})),
                expected_revision: None,
                source_revision: 1,
                observed_at: 110,
                ttl_seconds: Some(60),
            }],
            110,
        )
        .unwrap();
}

#[test]
fn pr_landing_scenarios() {
    let report = suite().run().unwrap();
    assert!(report.passed, "{}", serde_json::to_string_pretty(&report).unwrap());
    assert_eq!(report.cases.len(), 15);
}

#[test]
fn immutable_revisions_and_fact_snapshots_vary_independently() {
    let suite = suite();
    let env = environment(&suite);
    let catalog = PolicyCatalog::memory();
    let store = FactStore::memory();
    seed(&store, &suite);
    let original_facts = store.snapshot(scope());
    let first = catalog
        .publish(scope(), suite.documents[0].clone(), None, "agent".into(), 110, &env)
        .unwrap();
    let pinned = first.document.prepare(&env).unwrap();
    let second = catalog
        .publish(scope(), suite.documents[1].clone(), Some(1), "agent".into(), 111, &env)
        .unwrap();
    let current = second.document.prepare(&env).unwrap();
    assert_ne!(first.document_digest, second.document_digest);
    let input = &suite.cases[0].input;
    assert_eq!(pinned.evaluate(input, &original_facts, 120, 120).status, ExecutionStatus::Allow);
    assert_eq!(current.evaluate(input, &original_facts, 120, 120).status, ExecutionStatus::Deny);
    let previous = original_facts.key_revision("pull_requests", "commit-abc");
    store
        .commit(
            &scope(),
            "pull_requests",
            "forge-runtime",
            vec![FactMutation {
                key: "commit-abc".into(),
                value: Some(
                    json!({"approvers":["alice","bob","chris"],"passed_checks":["test","lint"]}),
                ),
                expected_revision: previous,
                source_revision: 2,
                observed_at: 121,
                ttl_seconds: Some(60),
            }],
            121,
        )
        .unwrap();
    assert_eq!(
        current.evaluate(input, &store.snapshot(scope()), 122, 122).status,
        ExecutionStatus::Allow
    );
    assert_eq!(current.evaluate(input, &original_facts, 122, 122).status, ExecutionStatus::Deny);
    assert_eq!(catalog.revision(&scope(), "pr-landing", 1).unwrap().document, first.document);
    assert_eq!(catalog.active(&scope())[0].revision, 2);
    assert!(catalog.active(&Scope::new("other", "production")).is_empty());
    assert!(catalog.revision(&Scope::new("other", "production"), "pr-landing", 1).is_none());
}

#[test]
fn invalid_refreshes_never_replace_active_policy() {
    let suite = suite();
    let env = environment(&suite);
    let catalog = PolicyCatalog::memory();
    catalog
        .publish(scope(), suite.documents[0].clone(), None, "agent".into(), 110, &env)
        .unwrap();
    assert!(matches!(
        catalog.publish(scope(), suite.documents[1].clone(), None, "agent".into(), 120, &env),
        Err(DocumentError::Conflict)
    ));
    let mut invalid = Vec::new();
    let mut doc = suite.documents[1].clone();
    doc.external.as_mut().unwrap().locator = "another resource".into();
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.external.as_mut().unwrap().observed_at = 100;
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.external.as_mut().unwrap().source_revision = "rules-7".into();
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.external.as_mut().unwrap().observed_at = 130;
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.external.as_mut().unwrap().valid_until = 120;
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.external = None;
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.sections[0].source_pointer = None;
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.sections[0].source_pointer = Some("/bad~pointer".into());
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.data.value["review"]["minimum"] = json!("three");
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.sections[0].rule = SectionRule::Modeled { source: "invalid language".into() };
    invalid.push(doc);
    let mut doc = suite.documents[1].clone();
    doc.sections.push(doc.sections[0].clone());
    invalid.push(doc);
    for doc in invalid {
        assert!(catalog.publish(scope(), doc, Some(1), "agent".into(), 120, &env).is_err());
        assert_eq!(catalog.active(&scope())[0].revision, 1);
        assert_eq!(catalog.all().len(), 1);
    }
}

#[test]
fn same_upstream_revision_can_be_reobserved_without_changing_old_revision() {
    let suite = suite();
    let env = environment(&suite);
    let catalog = PolicyCatalog::memory();
    let original = suite.documents[0].clone();
    catalog
        .publish(scope(), original.clone(), None, "agent".into(), 110, &env)
        .unwrap();
    let mut refreshed = original.clone();
    refreshed.external.as_mut().unwrap().observed_at = 120;
    refreshed.external.as_mut().unwrap().valid_until = 500;
    catalog.publish(scope(), refreshed, Some(1), "agent".into(), 120, &env).unwrap();
    assert_eq!(catalog.revision(&scope(), "pr-landing", 1).unwrap().document, original);
}

#[test]
fn persistence_and_concurrent_compare_and_swap() {
    let suite = suite();
    let env = environment(&suite);
    let directory = tempfile::tempdir().unwrap();
    {
        let catalog = Arc::new(PolicyCatalog::open(directory.path()).unwrap());
        catalog
            .publish(scope(), suite.documents[0].clone(), None, "agent".into(), 110, &env)
            .unwrap();
        let barrier = Arc::new(std::sync::Barrier::new(3));
        let threads: Vec<_> = (0..2)
            .map(|_| {
                let (catalog, barrier, doc, env) =
                    (catalog.clone(), barrier.clone(), suite.documents[1].clone(), env.clone());
                std::thread::spawn(move || {
                    barrier.wait();
                    catalog.publish(scope(), doc, Some(1), "agent".into(), 120, &env)
                })
            })
            .collect();
        barrier.wait();
        let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|r| matches!(r, Err(DocumentError::Conflict))).count(), 1);
    }
    let catalog = PolicyCatalog::open(directory.path()).unwrap();
    assert_eq!(catalog.active(&scope())[0].revision, 2);
    assert_eq!(
        catalog.revision(&scope(), "pr-landing", 1).unwrap().document,
        suite.documents[0]
    );
}

#[test]
fn policy_namespace_is_typed_reserved_and_available_in_triggers_and_keys() {
    let suite = suite();
    let env = environment(&suite).policy_data(suite.documents[0].data.clone());
    let good = "language 2 policy Good: \"typed\" triggers when policy.review.minimum == 2 requires all(c != \"\" for c in policy.required_checks)";
    assert!(CompiledPolicy::compile(good, &env).is_ok());
    for source in [
        "language 2 policy Bad: \"bad\" triggers when true requires policy.unknown == 1",
        "language 2 policy Bad: \"bad\" triggers when true requires policy.review.minimum == \"2\"",
        "language 2 policy Bad: \"bad\" triggers when true requires all(policy == \"test\" for policy in policy.required_checks)",
        "language 2 policy Bad: \"bad\" triggers when true facts policy = data.pull_requests[\"x\"] requires true",
    ] { assert!(CompiledPolicy::compile(source, &env).is_err(), "{source}"); }
    assert!(CompiledPolicy::compile(good, &environment(&suite)).is_err());
    let data = PolicyData {
        schema: crate::runtime_facts::Schema::String,
        value: json!("commit-abc"),
    };
    let env = environment(&suite).policy_data(data);
    let policy = CompiledPolicy::compile("language 2 policy Key: \"key\" triggers when true facts pr = data.pull_requests[policy] requires true", &env).unwrap();
    let store = FactStore::memory();
    seed(&store, &suite);
    assert_eq!(
        policy
            .evaluate(&suite.cases[0].input, &store.snapshot(scope()), 120, EvalBudget::default())
            .outcome,
        PolicyOutcome::Allow
    );
}

#[test]
fn published_revisions_pin_schemas_and_verify_content_digest() {
    let suite = suite();
    let env = environment(&suite);
    let catalog = PolicyCatalog::memory();
    let revision = catalog
        .publish(scope(), suite.documents[0].clone(), None, "agent".into(), 110, &env)
        .unwrap();
    assert!(revision.prepare(&env).is_ok());
    let mut altered_env = env.clone();
    altered_env.collections.insert("pull_requests".into(), Schema::Bool);
    assert!(revision.prepare(&altered_env).is_err());
    let mut altered = revision.clone();
    altered.document.data.value["review"]["minimum"] = json!(0);
    assert!(altered.prepare(&env).is_err());
    let mut altered = revision;
    altered.collection_schemas.clear();
    assert!(altered.prepare(&env).is_err());
}
