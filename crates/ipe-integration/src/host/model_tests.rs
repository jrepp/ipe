//! Deterministic authored-program proof; no GitHub account or network required.
use super::*;
use model::{Entity, Model};

pub(super) async fn capture(revision: &str) -> Vec<Model> {
    // These tests verify graph content, not throughput. Instrumented CI runners
    // can exhaust the unchanged guest deadline when all three fixtures compete.
    // Keep concurrency and deadline coverage in the dedicated host/script tests.
    static CAPTURE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _capture = CAPTURE.lock().await;
    let adapter = ScriptAdapter::from_source(
        include_str!("../../../../testing/integration/hosted/github-actions.js"),
        false,
    )
    .unwrap();
    let context = json!({"input":{"repository":"fixture/repo","revision":revision,"workflow":".github/workflows/docs.yml","allowed_repositories":["fixture/repo","fixture/action"]},"sdk_version":crate::script::SDK_VERSION});
    let mut state = Value::Null;
    let mut responses = json!([]);
    let mut models = Vec::new();
    for _ in 0..100 {
        let result = adapter.step(context.clone(), responses, state).await.unwrap();
        for wire in result.observations {
            let model: Model = serde_json::from_value(wire["model"].clone()).unwrap();
            model.validate().unwrap();
            models.push(model);
        }
        if result.outputs.is_some() {
            return models;
        }
        state = result.state;
        responses=Value::Array(result.requests.iter().map(|request| {
            let rows=match request.operation.as_str() {
                "resolve_revision" => json!([{"sha":revision}]),
                "read_definition" => {
                    let document=if request.params["repository"]=="fixture/repo" {
                        json!({"name":"Docs","on":"push","jobs":{
                            "build":{"name":"Build matrix","strategy":{"matrix":{"os":["linux","macos"]}},"steps":[
                                {"uses":"fixture/action@v1","with":{"artifact":"first"}},
                                {"uses":"fixture/action@v1"}]},
                            "publish":{"needs":"build","if":"github.ref == 'refs/heads/main'","environment":"pages","steps":[]}
                        }})
                    }else{json!({"name":"Upload","inputs":{"artifact":{"required":true,"default":"release"}},"runs":{"using":"node24","main":"index.js"}})};
                    json!([{"document":document,"locations":{}}])
                },
                "read_source" => json!([{"text":"core.getInput('artifact');"}]),
                "list_tree" => json!([]),
                "list_runs" => json!([{"id":42,"head_sha":revision,"name":"Docs","status":"completed","conclusion":"success","run_attempt":2,"created_at":"2026-10-29T00:00:00Z","run_started_at":"2026-10-29T00:00:01Z"}]),
                "list_jobs" => json!([{"id":7,"name":"Build matrix (linux)","status":"completed","conclusion":"success","head_sha":revision,"started_at":"2026-10-29T00:00:01Z","completed_at":"2026-10-29T00:00:03Z","steps":[]}]),
                name=>panic!("unexpected named operation {name}"),
            };
            json!({"id":request.id,"operation":request.operation,"receipt":format!("receipt_{}",request.id),"data":rows,"source_digest":"sha256:fixture","truncated":false})
        }).collect());
    }
    panic!("authored program exceeded fixture step budget")
}
#[tokio::test]
async fn authored_program_emits_typed_graph_with_distinct_calls_and_execution_attempts() {
    let models = capture(&"a".repeat(40)).await;
    let calls: Vec<_> = models.iter().filter(|m| m.kind() == "invocation").collect();
    assert_eq!(calls.len(), 2);
    assert_ne!(calls[0].key(), calls[1].key());
    for call in &calls {
        let Entity::Invocation { parent, callee, resolution, .. } = &call.entity else {
            unreachable!()
        };
        assert_eq!(*resolution, model::Resolution::Resolved);
        assert!(models.iter().any(|m|matches!(&m.entity,Entity::Definition {source,..} if source.definition==callee.as_ref().unwrap().definition && source.revision==callee.as_ref().unwrap().revision)));
        assert_eq!(parent.definition.resource, "fixture/repo");
    }
    let Entity::Invocation { bindings, .. } = &calls[0].entity else { unreachable!() };
    assert_eq!(bindings[0].origin, model::BindingOrigin::Caller);
    assert_eq!(bindings[0].value, "first");
    let Entity::Invocation { bindings, .. } = &calls[1].entity else { unreachable!() };
    assert_eq!(bindings[0].origin, model::BindingOrigin::DeclaredDefault);
    assert_eq!(bindings[0].value, "release");
    let constraints: Vec<_> = models.iter().filter(|m| m.kind() == "constraint").collect();
    assert_eq!(constraints.len(), 2);
    let Entity::Constraint { dependencies, .. } = &constraints[1].entity else { unreachable!() };
    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].key(), constraints[0].logical_key());
    let mut executions = Vec::new();
    for model in &models {
        if let Entity::Outcome {
            execution,
            declaration_candidates,
            correlation,
            ..
        } = &model.entity
        {
            assert_eq!(execution.attempt, 2);
            assert_ne!(*correlation, model::Resolution::Resolved);
            if execution.job.is_some() {
                assert!(
                    declaration_candidates.is_empty(),
                    "a matrix display name must not become an exact declaration match"
                );
            }
            executions.push(execution.key());
        }
    }
    for model in &models {
        if let Entity::Fact { subject, execution, .. } = &model.entity {
            assert_eq!(*subject, execution.as_ref().unwrap().key());
            assert!(executions.contains(subject));
        }
    }
    assert!(models.iter().any(|m| m.kind() == "evidence"));
    assert!(!models.iter().any(|m| m.kind() == "contract"));
    let next = capture(&"b".repeat(40)).await;
    let old = constraints[1];
    let new = next.iter().find(|m| m.logical_key() == old.logical_key()).unwrap();
    assert_eq!(old.logical_key(), new.logical_key());
    assert_ne!(old.key(), new.key());
    assert_ne!(old.revision("mapping").unwrap(), new.revision("mapping").unwrap());
}

#[tokio::test]
async fn resolved_invocations_require_captured_definitions_and_fact_execution_matches_subject() {
    let models = capture(&"a".repeat(40)).await;
    let records:Vec<Record>=models.iter().map(|m|serde_json::from_value(json!({"model":m,"revision_id":m.revision("mapping").unwrap(),"kind":m.kind(),"key":m.key(),"data":{},"receipts":["fixture"],"observed_at":1,"expires_at":null,"run_id":1,"adapter_digest":"mapping"})).unwrap()).collect();
    model::validate_graph(&records).unwrap();
    let missing: Vec<Record> = records.iter().filter(|r| r.kind != "definition").cloned().collect();
    assert_eq!(model::validate_graph(&missing), Err("resolved_definition_missing"));
    let mut bad = models.iter().find(|m| m.kind() == "fact").unwrap().clone();
    if let Entity::Fact { subject, .. } = &mut bad.entity {
        *subject = "other-execution".into();
    }
    assert_eq!(bad.validate(), Err("invalid_model"));
}
