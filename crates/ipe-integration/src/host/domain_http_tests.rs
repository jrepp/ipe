//! Exercise the versioned read APIs through authenticated HTTP without provider I/O.
use super::*;

#[tokio::test]
async fn versioned_domain_reads_preserve_state_and_enforce_request_boundaries() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../testing/integration/hosted/cross-source-assessment.json"
    ))
    .unwrap();
    let time = now();
    let mut store = Store::default();
    for (id, models) in fixture["models"].as_object().unwrap() {
        let current = models
            .as_array()
            .unwrap()
            .iter()
            .map(|value| {
                let model: model::Model = serde_json::from_value(value.clone()).unwrap();
                model.validate().unwrap();
                let kind = model.kind().to_owned();
                Record {
                    key: model.key(),
                    revision_id: model.revision("http_fixture").unwrap(),
                    expires_at: (["fact", "association"].contains(&kind.as_str()))
                        .then_some(time + 900),
                    kind,
                    model: Some(model),
                    data: json!({}),
                    receipts: vec![format!("receipt_{id}")],
                    source_evidence: vec![],
                    observed_at: time,
                    run_id: 1,
                    adapter_digest: "http_fixture".into(),
                }
            })
            .collect();
        store.integrations.insert(
            id.clone(),
            Installed {
                package: Installation {
                    source: "function step() { return ipe.complete({}); }".into(),
                    operations: vec!["read".into()],
                    input: json!({}),
                    interval_seconds: 300,
                    enabled: false,
                    fact_ttl_seconds: 900,
                    output_kinds: vec!["constraint".into(), "fact".into(), "association".into()],
                },
                digest: "http_fixture".into(),
                next_due: 0,
                current,
                outcomes: Default::default(),
                revisions: Default::default(),
                runs: vec![],
            },
        );
    }
    let selected = domain::operations(&store.integrations["ci"]);
    let record_revision = selected["operations"][0]["record_revision"].clone();
    let mut rules = fixture["evidence_rules"].clone();
    for rule in rules.as_array_mut().unwrap() {
        rule["record_revision"] = record_revision.clone();
    }
    let dir = tempfile::tempdir().unwrap();
    let token = "domain_http_test_token_at_least_32_bytes";
    std::env::set_var("IPE_DOMAIN_HTTP_TEST_TOKEN", token);
    let config: Config = serde_json::from_value(json!({
        "listen":"127.0.0.1:0","api_token_env":"IPE_DOMAIN_HTTP_TEST_TOKEN",
        "state_dir":dir.path().join("state"),
        "connections":[{"id":"fixture","origin":"http://127.0.0.1:9/"}],
        "operations":[{"name":"read","connection":"fixture","path":"/unused",
            "parameters":{},"decoder":"json","fields":["value"]}],
        "evidence_rules":rules
    }))
    .unwrap();
    let host = Host::new(config).await.unwrap();
    *host.store.lock().await = store;
    host.store.lock().await.integrations.get_mut("ci").unwrap().runs.push(Run {
        id: 7,
        trigger: "manual".into(),
        status: "succeeded".into(),
        started_at: time,
        finished_at: Some(time),
        elapsed_ms: 0,
        adapter_digest: "fixture".into(),
        steps: 0,
        metrics: Metrics::default(),
        error: None,
        records: vec![],
        evidence: vec![],
        summary: json!({}),
        coverage: vec![],
        progress: vec![],
        trace: None,
    });
    let before = serde_json::to_value(&*host.store.lock().await).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = host.router();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = reqwest::Client::new();
    for (asset, content_type) in [
        ("d3.v7.9.0.min.js", "text/javascript"),
        ("trace-model.js", "text/javascript"),
        ("trace-view.js", "text/javascript"),
        ("trace-view.css", "text/css"),
        ("governance-view.js", "text/javascript"),
        ("governance-view.css", "text/css"),
    ] {
        let response = client.get(format!("http://{addr}/assets/{asset}")).send().await.unwrap();
        assert_eq!(response.status(), 200);
        assert!(response.headers()["content-type"].to_str().unwrap().starts_with(content_type));
        assert!(!response.bytes().await.unwrap().is_empty());
    }
    let operations = format!("http://{addr}/v2/integrations/ci/operations");
    let explanation = format!("http://{addr}/v2/integrations/ci/runs/7/coverage-explanation");
    let assessments = format!("http://{addr}/v2/integrations/ci/assessments");
    assert_eq!(client.get(&operations).send().await.unwrap().status(), 401);
    assert_eq!(client.get(&explanation).send().await.unwrap().status(), 401);
    assert_eq!(client.post(&assessments).body("{}").send().await.unwrap().status(), 401);
    let response = client.get(&operations).bearer_auth(token).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let value: Value = serde_json::from_slice(&response.bytes().await.unwrap()).unwrap();
    assert_eq!(value, selected);
    assert_eq!(value["schema"], "ipe.integration.operations.v2");
    let response = client.get(&explanation).bearer_auth(token).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let explanation_value: Value =
        serde_json::from_slice(&response.bytes().await.unwrap()).unwrap();
    assert_eq!(explanation_value["schema"], "ipe.integration.coverage-explanation.v1");
    assert_eq!(explanation_value["run_id"], 7);
    assert_eq!(explanation_value["nodes"], json!([]));
    assert_eq!(
        client
            .get(format!("http://{addr}/v2/integrations/ci/runs/not-a-run/coverage-explanation"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        client
            .get(format!("http://{addr}/v2/integrations/ci/runs/8/coverage-explanation"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        client
            .get(format!("http://{addr}/v2/integrations/missing/runs/7/coverage-explanation"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        client
            .get(format!("http://{addr}/v2/integrations/missing/operations"))
            .bearer_auth(token)
            .send()
            .await
            .unwrap()
            .status(),
        404
    );
    let body = json!({"operation":fixture["request"]["control"],
        "source_revision":fixture["request"]["revision"],"record_revision":record_revision,
        "context":fixture["request"]["context"]});
    let response = client
        .post(&assessments)
        .bearer_auth(token)
        .body(body.to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    let result: Value = serde_json::from_slice(&response.bytes().await.unwrap()).unwrap();
    assert_eq!(result["schema"], "ipe.integration.context-assessment.v2");
    assert_eq!(result["requirements"][1]["evidence_check"]["status"], "matched");
    assert_eq!(result["requirements"][1]["fulfillment"]["status"], "unknown");
    assert_eq!(result["authorization"], false);
    assert_eq!(result["publication"], false);

    let mut stale = body.clone();
    stale["record_revision"] = json!(format!("sha256:{}", "0".repeat(64)));
    let mut extra = body.clone();
    extra["unexpected"] = json!(true);
    let duplicate = serde_json::to_string(&body).unwrap().replacen(
        "\"context\":{",
        "\"context\":{\"artifact\":\"forged\",",
        1,
    );
    for invalid in [stale.to_string(), extra.to_string(), duplicate, "x".repeat(17000)] {
        assert_eq!(
            client
                .post(&assessments)
                .bearer_auth(token)
                .body(invalid)
                .send()
                .await
                .unwrap()
                .status(),
            400
        );
    }
    let legacy_response = client
        .post(format!("http://{addr}/v1/integrations/ci/assessments"))
        .bearer_auth(token)
        .body(fixture["request"].to_string())
        .send()
        .await
        .unwrap();
    assert_eq!(legacy_response.status(), 200);
    let legacy: Value = serde_json::from_slice(&legacy_response.bytes().await.unwrap()).unwrap();
    assert_eq!(legacy["schema"], "ipe.integration.context-assessment.v1");
    assert_eq!(legacy["requirements"][1]["assessment"]["status"], "satisfied");
    assert_eq!(serde_json::to_value(&*host.store.lock().await).unwrap(), before);
    host.stop().await;
    server.abort();
}
