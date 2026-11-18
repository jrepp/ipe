//! Exercise the installed GitHub collector through native reads and capture validation.
use super::*;
use axum::response::IntoResponse;
use std::sync::atomic::AtomicBool;

const SHA: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const COLLECTOR: &str = include_str!("../../../../testing/integration/hosted/github-actions.js");

#[tokio::test]
async fn github_collector_captures_governance_and_keeps_read_gaps_partial() {
    let denied = Arc::new(AtomicBool::new(false));
    let env_denied = denied.clone();
    let app = Router::new()
        .route("/resolve",get(||async { Json(json!({"sha":SHA})) }))
        .route("/definition",get(||async { "name: Release\non: workflow_dispatch\njobs:\n  deploy:\n    name: Deploy\n    runs-on: ubuntu-latest\n    environment: production\n    steps:\n      - run: echo deploy\n" }))
        .route("/runs",get(||async {Json(json!({"workflow_runs":[]}))}))
        .route("/environments",get(move || {let denied=env_denied.clone();async move {
            if denied.load(Ordering::SeqCst) {return (StatusCode::FORBIDDEN,"private provider denial").into_response();}
            Json(json!({"environments":[{"id":1,"name":"production","protection_rules":[
                {"type":"wait_timer","wait_timer":5},
                {"type":"required_reviewers","prevent_self_review":true,"reviewers":[
                    {"type":"User","reviewer":{"id":10,"type":"User","login":"release-reviewer","html_url":"https://github.com/release-reviewer"}},
                    {"type":"Team","reviewer":{"id":20,"name":"Release team","html_url":"https://github.com/orgs/fixture/teams/release"}}
                ]}],"deployment_branch_policy":{"protected_branches":true,"custom_branch_policies":false}}]})).into_response()
        }}))
        .route("/rulesets",get(||async {Json(json!([{"id":7,"name":"Release rules","target":"branch","source_type":"Repository","source":"fixture/repo","enforcement":"active"}]))}))
        .route("/ruleset",get(||async {Json(json!({"id":7,"name":"Release rules","target":"branch","source_type":"Repository","source":"fixture/repo","enforcement":"active","conditions":{"ref_name":{"include":["refs/heads/main"],"exclude":[]}},"rules":[{"type":"pull_request","parameters":{"required_approving_review_count":2}}],"bypass_actors":[]}))}));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let provider = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var(
        "IPE_GITHUB_GOVERNANCE_TEST_TOKEN",
        "github_governance_test_token_at_least_32_bytes",
    );
    let mut operations = Vec::new();
    for (name, path, parameters, fields, pointer) in [
        (
            "resolve_revision",
            "/resolve",
            json!({"repository":{"kind":"path","values":["fixture/repo"]},"revision":{"kind":"ref"}}),
            json!(["sha"]),
            "",
        ),
        (
            "read_definition",
            "/definition",
            json!({"repository":{"kind":"path","values":["fixture/repo"]},"revision":{"kind":"sha"},"path":{"kind":"path"}}),
            json!([]),
            "",
        ),
        (
            "list_runs",
            "/runs",
            json!({"repository":{"kind":"path","values":["fixture/repo"]},"revision":{"kind":"sha"},"workflow":{"kind":"identifier"}}),
            json!(["id", "head_sha", "name", "status", "conclusion", "run_attempt"]),
            "/workflow_runs",
        ),
        (
            "list_environments",
            "/environments",
            json!({"repository":{"kind":"path","values":["fixture/repo"]}}),
            json!(["id", "name", "protection_rules", "deployment_branch_policy"]),
            "/environments",
        ),
        (
            "list_rulesets",
            "/rulesets",
            json!({"repository":{"kind":"path","values":["fixture/repo"]}}),
            json!(["id", "name", "target", "source_type", "source", "enforcement"]),
            "",
        ),
        (
            "get_ruleset",
            "/ruleset",
            json!({"repository":{"kind":"path","values":["fixture/repo"]},"ruleset_id":{"kind":"integer"}}),
            json!([
                "id",
                "name",
                "target",
                "source_type",
                "source",
                "enforcement",
                "conditions",
                "rules",
                "bypass_actors"
            ]),
            "",
        ),
    ] {
        let mut operation = json!({"name":name,"connection":"fixture","path":path,"parameters":parameters,"decoder":"json","fields":fields,"pointer":pointer});
        if name == "read_definition" {
            operation["decoder"] = json!("yaml");
            operation["snapshot_source"] = json!({"namespace":"github","resource_parameter":"repository","path_parameter":"path","revision_parameter":"revision"});
        }
        operations.push(operation);
    }
    let config = json!({"listen":"127.0.0.1:0","api_token_env":"IPE_GITHUB_GOVERNANCE_TEST_TOKEN","state_dir":dir.path().join("state"),"connections":[{"id":"fixture","origin":origin}],"operations":operations});
    let host = Host::new(serde_json::from_value(config).unwrap()).await.unwrap();
    for (id, fail_read, grants) in
        [("complete", false, true), ("denied", true, true), ("ungranted", false, false)]
    {
        denied.store(fail_read, Ordering::SeqCst);
        let mut allowed =
            vec!["resolve_revision".into(), "read_definition".into(), "list_runs".into()];
        if grants {
            allowed.extend([
                "list_environments".into(),
                "list_rulesets".into(),
                "get_ruleset".into(),
            ]);
        }
        host.install(id,Installation {source:COLLECTOR.into(),operations:allowed,input:json!({"repository":"fixture/repo","revision":"main","workflow":".github/workflows/release.yml","allowed_repositories":["fixture/repo"]}),interval_seconds:300,enabled:false,fact_ttl_seconds:900,output_kinds:model::KINDS.iter().map(|s|s.to_string()).collect()}).await.unwrap();
        let run_id = host.start(id, "manual").await.unwrap();
        let run = tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let run = host.store.lock().await.integrations[id]
                    .runs
                    .iter()
                    .find(|r| r.id == run_id)
                    .unwrap()
                    .clone();
                if run.status != "running" {
                    break run;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(run.status, "succeeded", "{id}: {:?}", run.error);
        assert!(run.records.iter().any(|r| r.kind == "definition"));
        assert!(run.records.iter().any(|r| r.kind == "constraint"));
        model::validate_graph(&run.records).unwrap();
        let encoded = serde_json::to_value(&run).unwrap();
        let text = encoded.to_string();
        assert!(!text.contains("private provider denial"));
        if !grants {
            assert!(!run.records.iter().any(|r| r.kind == "governance"));
            assert!(text.contains("governance_not_granted"));
            continue;
        }
        let governance: Vec<_> = run.records.iter().filter(|r| r.kind == "governance").collect();
        assert!(!governance.is_empty());
        for record in &governance {
            let model = record.model.as_ref().unwrap();
            assert_eq!(model.schema_version, 2);
            if let model::Entity::Governance { record: description } = &model.entity {
                assert_ne!(description.kind(), governance::Kind::Contract);
                if description.is_temporal() {
                    assert_eq!(record.expires_at, Some(record.observed_at + 900));
                }
            }
            assert!(!record.receipts.is_empty());
            assert!(!record.source_evidence.is_empty());
        }
        if fail_read {
            assert!(text.contains("provider_access_denied"));
            assert!(!text.contains("environment:1"));
        } else {
            assert!(text.contains("environment:1"));
            assert!(text.contains("ruleset:7"));
            assert!(text.contains("wait_timer"));
            assert!(text.contains("pull_request"));
            assert!(!text.contains("\"principal_kind\":\"human\""));
            assert!(text.contains("\"principal_kind\":\"unknown\""));
            assert!(text.contains("\"principal_kind\":\"group\""));
            assert!(governance.iter().any(|r|r.model.as_ref().is_some_and(|m|matches!(&m.entity,model::Entity::Governance {record} if record.kind()==governance::Kind::ContactMethod))));
            if let Some(path) = std::env::var_os("IPE_GITHUB_GOVERNANCE_TEST_ARTIFACT") {
                std::fs::write(path, serde_json::to_vec_pretty(&run).unwrap()).unwrap();
            }
        }
    }
    host.stop().await;
    provider.abort();
}
