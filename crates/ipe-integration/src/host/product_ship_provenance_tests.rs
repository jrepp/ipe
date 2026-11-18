//! Exercise the ServiceNow product provenance collector through the native host.
use super::*;
use axum::extract::Path as RequestPath;
use axum::response::IntoResponse;
use std::sync::atomic::AtomicBool;

const COLLECTOR: &str =
    include_str!("../../../../testing/integration/hosted/product-ship/provenance-collector.js");
const FIXTURE: &str =
    include_str!("../../../../testing/integration/hosted/product-ship/provenance.json");
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
    .expect("provenance host run should finish")
}

fn governance(record: &Record) -> Value {
    let Some(model::Model {
        entity: model::Entity::Governance { record },
        ..
    }) = &record.model
    else {
        panic!("expected governance model")
    };
    serde_json::to_value(record).unwrap()
}

#[tokio::test]
async fn product_ship_provenance_is_bounded_receipt_bound_and_keeps_unknown_joins_unknown() {
    let fixture: Value = serde_json::from_str(FIXTURE).unwrap();
    let contexts = fixture["contexts"].as_array().unwrap();
    let provider_contexts = contexts.to_vec();
    let denied = Arc::new(AtomicBool::new(false));
    let provider_denied = denied.clone();
    let router = Router::new().route(
        "/control-context/:control_id",
        get(move |RequestPath(control_id): RequestPath<String>| {
            let denied = provider_denied.clone();
            let contexts = provider_contexts.clone();
            async move {
                if denied.load(Ordering::SeqCst) {
                    return (StatusCode::FORBIDDEN, "synthetic provider denial").into_response();
                }
                let Some(context) = contexts.iter().find(|row| row["control_id"] == control_id)
                else {
                    return (StatusCode::NOT_FOUND, "unknown control").into_response();
                };
                Json(json!({"context":context})).into_response()
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let provider = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let dir = tempfile::tempdir().unwrap();
    let config = json!({
        "listen":"127.0.0.1:0",
        "api_token_env":"IPE_PRODUCT_SHIP_PROVENANCE_TEST_TOKEN",
        "state_dir":dir.path().join("state"),
        "connections":[{"id":"servicenow","origin":origin}],
        "operations":[{"name":"read_control_context","connection":"servicenow",
            "path":"/control-context/{control_id}",
            "parameters":{"control_id":{"kind":"identifier"}},"decoder":"json",
            "fields":["context"],"max_records":1}]
    });
    std::env::set_var(
        "IPE_PRODUCT_SHIP_PROVENANCE_TEST_TOKEN",
        "product_ship_provenance_test_token_at_least_32_bytes",
    );
    let host = Host::new(serde_json::from_value(config).unwrap()).await.unwrap();

    let available_context = contexts
        .iter()
        .find(|context| context["collection"]["status"] == "available")
        .expect("fixture should have an available context");
    assert!(
        available_context["people"]
            .as_array()
            .unwrap()
            .iter()
            .any(|person| { person["email"]["status"] == "unavailable" }),
        "the selected context should exercise unavailable email provenance"
    );
    let missing_person_context = contexts
        .iter()
        .find(|context| {
            context["assignments"].as_array().is_some_and(|assignments| {
                assignments.iter().any(|assignment| {
                    assignment["resolution"] != "resolved"
                        || !context["people"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|person| person["id"] == assignment["person_id"])
                })
            })
        })
        .expect("fixture should include an unresolved or missing person");

    let output_kinds = vec!["evidence".to_owned(), "governance".to_owned()];
    for (name, context, grant) in [
        ("provenance_complete", available_context, true),
        ("provenance_ungranted", available_context, false),
        ("provenance_missing_person", missing_person_context, true),
        ("provenance_denied", available_context, true),
    ] {
        host.install(
            name,
            Installation {
                source: COLLECTOR.into(),
                operations: vec!["read_control_context".to_owned()],
                input: json!({"control_id":context["control_id"],"observed_at":fixture["as_of"]}),
                interval_seconds: 300,
                enabled: false,
                fact_ttl_seconds: FACT_TTL_SECONDS,
                output_kinds: if grant { output_kinds.clone() } else { vec!["evidence".into()] },
            },
        )
        .await
        .unwrap();
    }

    let id = host.start("provenance_complete", "manual").await.unwrap();
    let run = completed_run(&host, "provenance_complete", id).await;
    assert_eq!(
        run.status, "succeeded",
        "error={:?} steps={} evidence={:?}",
        run.error, run.steps, run.evidence
    );
    assert!(run.records.len() <= 256);
    assert!(!run.records.is_empty());
    assert_eq!(run.summary["status"], "available");
    assert_eq!(run.summary["control_id"], available_context["control_id"]);
    assert!(serde_json::to_vec(&run.records).unwrap().len() <= MAX_RUN_BYTES);
    let trace = run.trace.as_ref().unwrap();
    assert_eq!(trace.schema_version, 1);
    assert_eq!(trace.omitted, 0);
    assert_eq!(trace.emissions.len(), run.records.len());
    let operation_receipt = run.evidence[0]["receipt"].as_str().unwrap();
    for (emission, record) in trace.emissions.iter().zip(&run.records) {
        assert_eq!(emission.key, record.key);
        assert_eq!(emission.kind, record.kind);
        assert_eq!(emission.receipts, vec![operation_receipt]);
        assert_eq!(record.receipts, vec![operation_receipt]);
        assert_eq!(record.source_evidence.len(), 1);
        assert_eq!(record.source_evidence[0]["receipt"], operation_receipt);
    }

    let governance_records = run
        .records
        .iter()
        .filter(|record| record.kind == "governance")
        .collect::<Vec<_>>();
    assert!(!governance_records.is_empty());
    let json_records = governance_records.iter().copied().map(governance).collect::<Vec<_>>();
    assert!(json_records.iter().any(|record| record["type"] == "principal"));
    assert!(json_records.iter().any(|record| record["type"] == "role_assignment"));
    assert!(json_records.iter().any(|record| record["type"] == "contact_method"));
    for record in governance_records {
        let model::Model {
            entity: model::Entity::Governance { record: entity },
            ..
        } = record.model.as_ref().unwrap()
        else {
            unreachable!()
        };
        assert!(entity.validate(), "invalid governance record: {:?}", entity);
        if entity.is_temporal() {
            if let Some(until) = entity.valid_until() {
                // Governance timestamps use Unix seconds, matching host TTL arithmetic.
                assert!(until < 10_000_000_000, "valid_until must use seconds, not milliseconds");
            }
            assert_eq!(
                record.expires_at,
                entity
                    .valid_until()
                    .map(|until| until.min(record.observed_at + FACT_TTL_SECONDS))
            );
        }
    }
    // Every emitted resolved role/principal reference must be closed inside this run.
    model::validate_graph(&run.records).unwrap();

    let unavailable = run
        .records
        .iter()
        .filter(|record| record.kind == "governance")
        .filter(|record| record.data["contact"]["status"] == "unavailable")
        .map(governance)
        .filter(|record| record["type"] == "contact_method" && record["channel"] == "email")
        .collect::<Vec<_>>();
    assert!(!unavailable.is_empty());
    assert!(unavailable
        .iter()
        .all(|record| { record["availability"] == "unavailable" && record["endpoint"].is_null() }));
    let source_text = serde_json::to_string(&run.records).unwrap();
    let canonical_record = available_context["records"].as_array().unwrap().first().unwrap();
    assert!(source_text.contains(canonical_record["sys_id"].as_str().unwrap()));
    let source_receipt = canonical_record
        .get("source_receipt")
        .and_then(Value::as_str)
        .or_else(|| canonical_record["receipt"].as_str())
        .unwrap();
    assert!(source_text.contains(source_receipt));

    let missing_id = host.start("provenance_missing_person", "manual").await.unwrap();
    let missing_run = completed_run(&host, "provenance_missing_person", missing_id).await;
    assert_eq!(missing_run.status, "succeeded", "{:?}", missing_run.error);
    let missing_assignments = missing_person_context["assignments"].as_array().unwrap();
    for assignment in missing_assignments.iter().filter(|assignment| {
        assignment["resolution"] != "resolved"
            || !missing_person_context["people"]
                .as_array()
                .unwrap()
                .iter()
                .any(|person| person["id"] == assignment["person_id"])
    }) {
        let person_id = assignment["person_id"].as_str().unwrap();
        let records = missing_run
            .records
            .iter()
            .filter(|record| record.kind == "governance")
            .map(governance)
            .collect::<Vec<_>>();
        assert!(!records.iter().any(|record| {
            record["type"] == "principal"
                && record["id"]["local_id"]
                    .as_str()
                    .is_some_and(|id| id.ends_with(&format!(":{person_id}")))
        }));
        let assignment_record = records
            .iter()
            .find(|record| {
                record["type"] == "role_assignment"
                    && record["id"]["local_id"]
                        .as_str()
                        .is_some_and(|id| id.ends_with(assignment["id"].as_str().unwrap()))
            })
            .expect("unresolved source assignment should remain visible");
        assert_eq!(assignment_record["principal"]["resolution"], "unknown");
    }

    let ungranted_id = host.start("provenance_ungranted", "manual").await.unwrap();
    let ungranted = completed_run(&host, "provenance_ungranted", ungranted_id).await;
    assert_eq!(ungranted.status, "failed");
    assert_eq!(ungranted.error.as_deref(), Some("observation_not_permitted"));
    assert!(ungranted.records.is_empty());
    assert!(host.store.lock().await.integrations["provenance_ungranted"].current.is_empty());

    denied.store(true, Ordering::SeqCst);
    let denied_id = host.start("provenance_denied", "manual").await.unwrap();
    let denied_run = completed_run(&host, "provenance_denied", denied_id).await;
    assert_eq!(denied_run.status, "succeeded", "{:?}", denied_run.error);
    assert!(denied_run.records.is_empty());
    assert_eq!(denied_run.summary["status"], "unavailable");
    assert_eq!(denied_run.evidence[0]["status"], "unavailable");
    assert!(host.store.lock().await.integrations["provenance_denied"].current.is_empty());
    assert!(!serde_json::to_string(&denied_run)
        .unwrap()
        .contains("synthetic provider denial"));

    host.stop().await;
    provider.abort();
}
