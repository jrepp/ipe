//! Exercise the simulated release qualification collector through the native host.
use super::*;
use axum::extract::Path as RequestPath;
use axum::response::IntoResponse;
use std::sync::atomic::AtomicBool;

const COLLECTOR: &str =
    include_str!("../../../../testing/integration/hosted/product-ship/collector.js");
const FIXTURE: &str =
    include_str!("../../../../testing/integration/hosted/product-ship/product-ship.json");
const PROVIDER_TOKEN_ENV: &str = "IPE_PRODUCT_SHIP_TEST_TOKEN";
const FACT_TTL_SECONDS: u64 = 600;

async fn completed_run(host: &Host, integration: &str, run_id: u64) -> Run {
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let run = host.store.lock().await.integrations[integration]
                .runs
                .iter()
                .find(|run| run.id == run_id)
                .unwrap()
                .clone();
            if run.status != "running" {
                break run;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("product ship host run should finish")
}

#[tokio::test]
async fn product_ship_collector_emits_bounded_receipt_bound_hierarchy_and_unknown_facts() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let mut provider_payload = json!({
        "release": fixture["release"].clone(),
        "policy": fixture["policy"].clone(),
        "policies": fixture["policies"].clone(),
        "evidence": fixture["evidence"].clone(),
    });
    // Unknown evidence stays unknown through the provider, SDK, and native host.
    provider_payload["evidence"][0]["status"] = json!("unknown");
    // Keep a provider artifact mismatch visible. The collector must not rewrite it.
    provider_payload["evidence"][1]["artifact"] =
        json!("sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    let denied = Arc::new(AtomicBool::new(false));
    let provider_denied = denied.clone();
    let payload = provider_payload.clone();
    let router = Router::new().route(
        "/qualification/:release_id",
        get(move |RequestPath(release_id): RequestPath<String>| {
            let denied = provider_denied.clone();
            let payload = payload.clone();
            async move {
                if release_id != "release-2026.11.02-rc1" {
                    return (StatusCode::NOT_FOUND, "unknown release").into_response();
                }
                if denied.load(Ordering::SeqCst) {
                    return (StatusCode::FORBIDDEN, "synthetic provider denial").into_response();
                }
                Json(payload).into_response()
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let provider = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var(PROVIDER_TOKEN_ENV, "product_ship_test_token_at_least_32_bytes");
    let config = json!({
        "listen":"127.0.0.1:0",
        "api_token_env":PROVIDER_TOKEN_ENV,
        "state_dir":dir.path().join("state"),
        "connections":[{"id":"qualification","origin":origin}],
        "operations":[{"name":"read_release_qualification","connection":"qualification",
            "path":"/qualification/{release_id}",
            "parameters":{"release_id":{"kind":"identifier"}},"decoder":"json",
            "fields":["release","policy","policies","evidence"],"max_records":1}]
    });
    let host = Host::new(serde_json::from_value(config.clone()).unwrap()).await.unwrap();
    let operations = vec!["read_release_qualification".to_owned()];
    let output_kinds = vec!["governance".to_owned(), "fact".to_owned()];
    for integration in ["product_ship_complete", "product_ship_ungranted", "product_ship_denied"] {
        host.install(
            integration,
            Installation {
                source: COLLECTOR.into(),
                operations: operations.clone(),
                input: json!({"release":fixture["release"].clone()}),
                interval_seconds: 300,
                enabled: false,
                fact_ttl_seconds: FACT_TTL_SECONDS,
                output_kinds: if integration == "product_ship_ungranted" {
                    vec!["fact".to_owned()]
                } else {
                    output_kinds.clone()
                },
            },
        )
        .await
        .unwrap();
    }

    let successful_id = host.start("product_ship_complete", "manual").await.unwrap();
    let successful = completed_run(&host, "product_ship_complete", successful_id).await;
    assert_eq!(
        successful.status, "succeeded",
        "error={:?} steps={}",
        successful.error, successful.steps
    );
    assert_eq!(
        successful.summary["status"], "succeeded",
        "summary={:?}, evidence={:?}",
        successful.summary, successful.evidence
    );
    assert_eq!(
        successful.summary["controls_observed"],
        provider_payload["evidence"].as_array().unwrap().len()
    );
    assert_eq!(successful.summary["publication"], false);

    let receipt = successful.evidence[0]["receipt"].as_str().unwrap();
    assert_eq!(successful.evidence[0]["status"], "observed");
    assert_eq!(successful.evidence[0]["operation"], "read_release_qualification");
    assert_eq!(successful.records.len(), 1 + 1 + 9 + 18 + 18);
    assert!(successful.records.len() <= 256);
    let trace = successful.trace.as_ref().unwrap();
    assert_eq!(trace.schema_version, 1);
    assert_eq!(trace.omitted, 0);
    assert_eq!(trace.emissions.len(), successful.records.len());
    assert!(trace.emissions.len() <= 256);
    for (emission, record) in trace.emissions.iter().zip(&successful.records) {
        assert_eq!(emission.key, record.key);
        assert_eq!(emission.kind, record.kind);
        assert_eq!(emission.receipts, vec![receipt]);
    }
    for record in &successful.records {
        assert_eq!(record.receipts, vec![receipt]);
        assert_eq!(record.source_evidence.len(), 1);
        assert_eq!(record.source_evidence[0]["receipt"], receipt);
        assert_eq!(record.source_evidence[0]["status"], "observed");
    }

    let governance = successful
        .records
        .iter()
        .filter(|record| record.kind == "governance")
        .collect::<Vec<_>>();
    let facts = successful
        .records
        .iter()
        .filter(|record| record.kind == "fact")
        .collect::<Vec<_>>();
    assert_eq!(governance.len(), 1 + 1 + 9 + 18);
    assert_eq!(facts.len(), 18);
    let gov_json = governance
        .iter()
        .map(|record| serde_json::to_value(record).unwrap())
        .collect::<Vec<_>>();
    let root = gov_json
        .iter()
        .find(|record| record["model"]["entity"]["record"]["id"]["local_id"] == "product-ship")
        .unwrap();
    let child_ids = fixture["policy"]["children"].as_array().unwrap();
    assert_eq!(root["data"]["children"], fixture["policy"]["children"]);
    for child in child_ids {
        let child_id = child.as_str().unwrap();
        let policy_local = format!("policy:{child_id}");
        let child_record = gov_json
            .iter()
            .find(|record| record["model"]["entity"]["record"]["id"]["local_id"] == policy_local)
            .unwrap_or_else(|| panic!("missing child policy {child_id}"));
        assert_eq!(child_record["data"]["parent_id"], "product-ship");
    }
    for policy in fixture["policies"].as_array().unwrap() {
        for control in policy["controls"].as_array().unwrap() {
            let control_id = control["id"].as_str().unwrap();
            let record = gov_json
                .iter()
                .find(|record| {
                    record["model"]["entity"]["record"]["id"]["local_id"]
                        == format!("control:{control_id}")
                })
                .unwrap_or_else(|| panic!("missing control {control_id}"));
            assert_eq!(record["data"]["parent_id"], policy["id"]);
        }
    }
    let fact_by_control = facts
        .iter()
        .map(|record| {
            let data = &record.data;
            let entity = &record.model.as_ref().unwrap().entity;
            let model::Entity::Fact { subject, predicate, value, .. } = entity else {
                unreachable!()
            };
            assert_eq!(predicate, "product_ship.control_status");
            assert!(subject.contains(data["control_id"].as_str().unwrap()));
            assert_eq!(data["release_id"], fixture["release"]["id"]);
            assert_eq!(data["environment"], fixture["release"]["environment"]);
            let provider_row = provider_payload["evidence"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["control_id"] == data["control_id"])
                .unwrap();
            assert_eq!(data["artifact"], provider_row["artifact"]);
            assert_eq!(value, &data["status"]);
            assert_eq!(record.expires_at, Some(record.observed_at + FACT_TTL_SECONDS));
            (data["control_id"].as_str().unwrap().to_owned(), value.clone())
        })
        .collect::<std::collections::HashMap<_, _>>();
    assert_eq!(fact_by_control.len(), 18);
    assert_eq!(
        fact_by_control[provider_payload["evidence"][0]["control_id"].as_str().unwrap()],
        "unknown"
    );
    assert!(fact_by_control.values().filter(|status| *status == "met").count() == 17);
    let wrong_artifact_control = provider_payload["evidence"][1]["control_id"].as_str().unwrap();
    let wrong_artifact_fact = facts
        .iter()
        .find(|record| record.data["control_id"] == wrong_artifact_control)
        .unwrap();
    assert_eq!(
        wrong_artifact_fact.data["artifact"],
        provider_payload["evidence"][1]["artifact"]
    );
    assert_ne!(wrong_artifact_fact.data["artifact"], fixture["release"]["artifact"]);

    // Nine named child installations each emit only their own two control facts.
    for policy in fixture["policies"].as_array().unwrap() {
        let policy_id = policy["id"].as_str().unwrap();
        let integration = format!("ship_{}", policy_id.replace('-', "_"));
        let mut child_config = config.clone();
        child_config["state_dir"] = json!(dir.path().join(format!("state-{integration}")));
        let child_host = Host::new(serde_json::from_value(child_config).unwrap()).await.unwrap();
        child_host
            .install(
                &integration,
                Installation {
                    source: COLLECTOR.into(),
                    operations: operations.clone(),
                    input: json!({
                        "release":fixture["release"].clone(),
                        "policy_id":policy_id
                    }),
                    interval_seconds: 300,
                    enabled: false,
                    fact_ttl_seconds: FACT_TTL_SECONDS,
                    output_kinds: output_kinds.clone(),
                },
            )
            .await
            .unwrap();
        let run_id = child_host.start(&integration, "manual").await.unwrap();
        let run = completed_run(&child_host, &integration, run_id).await;
        assert_eq!(run.status, "succeeded", "{integration}: {:?}", run.error);
        assert_eq!(run.records.len(), 7, "{integration}");
        assert!(run.records.len() <= 256);
        let child_governance = run
            .records
            .iter()
            .filter(|record| record.kind == "governance")
            .collect::<Vec<_>>();
        assert_eq!(child_governance.len(), 5, "{integration}");
        let child_facts =
            run.records.iter().filter(|record| record.kind == "fact").collect::<Vec<_>>();
        assert_eq!(child_facts.len(), 2, "{integration}");
        let expected_controls = policy["controls"]
            .as_array()
            .unwrap()
            .iter()
            .map(|control| control["id"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>();
        let actual_controls = child_facts
            .iter()
            .map(|record| record.data["control_id"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(actual_controls, expected_controls, "{integration}");
        let mut expected_governance_ids = std::collections::HashSet::from([
            "principal:release-governance".to_owned(),
            "product-ship".to_owned(),
            format!("policy:{policy_id}"),
        ]);
        expected_governance_ids
            .extend(expected_controls.iter().map(|control_id| format!("control:{control_id}")));
        let actual_governance_ids = child_governance
            .iter()
            .map(|record| {
                let model = record.model.as_ref().unwrap();
                let model::Entity::Governance { record } = &model.entity else {
                    panic!("governance record has a non-governance model");
                };
                let value = serde_json::to_value(record).unwrap();
                assert_eq!(value["source"]["definition"]["namespace"], "ipe-demo");
                assert_eq!(value["source"]["definition"]["resource"], "product-ship");
                assert_eq!(value["source"]["revision"], "synthetic-qualification-v1");
                value["id"]["local_id"].as_str().unwrap().to_owned()
            })
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(actual_governance_ids, expected_governance_ids, "{integration}");
        let root_record = child_governance
            .iter()
            .find(|record| {
                record.model.as_ref().is_some_and(|model| {
                    matches!(&model.entity, model::Entity::Governance { record } if record.id().local_id == "product-ship")
                })
            })
            .unwrap();
        let root_model = root_record.model.as_ref().unwrap();
        let model::Entity::Governance { record } = &root_model.entity else { unreachable!() };
        let root_value = serde_json::to_value(record).unwrap();
        let declared_children = root_value["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .map(|requirement| requirement["id"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>();
        let expected_children = fixture["policy"]["children"]
            .as_array()
            .unwrap()
            .iter()
            .map(|child| child.as_str().unwrap())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(declared_children, expected_children, "{integration}");
        let receipt = run.evidence[0]["receipt"].as_str().unwrap();
        let trace = run.trace.as_ref().unwrap();
        assert_eq!(trace.omitted, 0);
        assert_eq!(trace.emissions.len(), run.records.len());
        assert!(trace.emissions.len() <= 256);
        for record in &run.records {
            assert_eq!(record.receipts, vec![receipt]);
            assert_eq!(record.source_evidence[0]["receipt"], receipt);
            assert_eq!(record.source_evidence[0]["status"], "observed");
            if record.kind == "fact" {
                assert_eq!(record.data["release_id"], fixture["release"]["id"]);
                assert_eq!(record.data["source"], "qualification-snapshot-v1");
                if let model::Entity::Fact { subject, .. } = &record.model.as_ref().unwrap().entity
                {
                    assert_eq!(
                        subject,
                        &format!(
                            "{}#{}",
                            fixture["release"]["id"].as_str().unwrap(),
                            record.data["control_id"].as_str().unwrap()
                        )
                    );
                } else {
                    panic!("fact record has a non-fact model");
                }
            }
        }
        child_host.stop().await;
    }

    // The host enforces the installation's output grant for each model kind.
    let ungranted_id = host.start("product_ship_ungranted", "manual").await.unwrap();
    let ungranted = completed_run(&host, "product_ship_ungranted", ungranted_id).await;
    assert_eq!(ungranted.status, "failed");
    assert_eq!(ungranted.error.as_deref(), Some("observation_not_permitted"));
    assert!(ungranted.records.is_empty());
    assert!(host.store.lock().await.integrations["product_ship_ungranted"]
        .current
        .is_empty());

    // A denied read produces unknown coverage and no staged or committed evidence.
    denied.store(true, Ordering::SeqCst);
    let denied_id = host.start("product_ship_denied", "manual").await.unwrap();
    let denied_run = completed_run(&host, "product_ship_denied", denied_id).await;
    assert_eq!(denied_run.status, "succeeded", "{:?}", denied_run.error);
    assert_eq!(denied_run.summary["status"], "unknown");
    assert_eq!(denied_run.summary["reason"], "provider_read_failed");
    assert!(denied_run.records.is_empty());
    assert_eq!(denied_run.evidence.len(), 1);
    assert_eq!(denied_run.evidence[0]["status"], "unavailable");
    assert_eq!(denied_run.evidence[0]["code"], "provider_access_denied");
    assert!(host.store.lock().await.integrations["product_ship_denied"].current.is_empty());
    let denied_text = serde_json::to_string(&denied_run).unwrap();
    assert!(!denied_text.contains("synthetic provider denial"));
    assert!(!denied_text.contains("\"status\":\"met\""));

    host.stop().await;
    provider.abort();
}
