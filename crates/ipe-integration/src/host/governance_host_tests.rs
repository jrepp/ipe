use super::*;

#[tokio::test]
async fn governance_capture_preserves_grants_provenance_expiry_and_atomicity() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../testing/integration/hosted/governance-handoff.json"
    ))
    .unwrap();
    let models = fixture["models"].clone();
    let provider_models = models.clone();
    let router = Router::new().route(
        "/governance",
        get(move || {
            let models = provider_models.clone();
            async move { Json(json!({"models":models})) }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let provider = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("IPE_GOVERNANCE_HOST_TEST_TOKEN", "governance_test_token_at_least_32_bytes");
    let config = json!({"listen":"127.0.0.1:0","api_token_env":"IPE_GOVERNANCE_HOST_TEST_TOKEN",
        "state_dir":dir.path().join("state"),"connections":[{"id":"fixture","origin":origin}],
        "operations":[{"name":"read_governance","connection":"fixture","path":"/governance",
        "parameters":{},"decoder":"json","fields":["models"]}]});
    let host = Host::new(serde_json::from_value(config.clone()).unwrap()).await.unwrap();
    let source = r#"function step(context,responses,state) {
        if(state===null)return ipe.exchange([ipe.call('read','read_governance')],{phase:'read'});
        const response=ipe.response(responses,'read');
        const models=response.data[0].models;
        if(context.input.bad_ref)models.find(m=>m.entity.record?.type==='handoff').entity.record.policy.source_revision='missing';
        return {...ipe.complete({coverage:'synthetic governance'}),observations:models.map(m=>ipe.observation(m,{},[response.receipt]))};
    }"#;
    let mut prior = Value::Null;
    for (grant, bad_ref, expected) in
        [(false, false, "failed"), (true, false, "succeeded"), (true, true, "failed")]
    {
        let mut kinds =
            vec!["definition".into(), "constraint".into(), "outcome".into(), "fact".into()];
        if grant {
            kinds.push("governance".into());
        }
        host.install(
            "governance",
            Installation {
                source: source.into(),
                operations: vec!["read_governance".into()],
                input: json!({"bad_ref":bad_ref}),
                interval_seconds: 300,
                enabled: false,
                fact_ttl_seconds: 900,
                output_kinds: kinds,
            },
        )
        .await
        .unwrap();
        let id = host.start("governance", "manual").await.unwrap();
        let run = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let run = host.store.lock().await.integrations["governance"]
                    .runs
                    .iter()
                    .find(|r| r.id == id)
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
        assert_eq!(run.status, expected, "{:?}", run.error);
        let store = host.store.lock().await;
        let installed = &store.integrations["governance"];
        if !grant {
            assert_eq!(run.error.as_deref(), Some("observation_not_permitted"));
            assert!(installed.current.is_empty());
        } else if bad_ref {
            assert_eq!(run.error.as_deref(), Some("resolved_governance_missing"));
            assert!(run.records.is_empty());
            assert_eq!(serde_json::to_value(&installed.current).unwrap(), prior);
        } else {
            assert_eq!(run.records.len(), models.as_array().unwrap().len());
            assert_eq!(run.trace.as_ref().unwrap().emissions.len(), run.records.len());
            let revision = installed
                .revisions
                .get(installed.revisions.candidate.as_ref().unwrap())
                .unwrap();
            for record in &run.records {
                assert_eq!(record.receipts[0], run.evidence[0]["receipt"]);
                assert_eq!(record.source_evidence[0]["step"], 1);
                if let Some(model::Model {
                    entity: model::Entity::Governance { record: governance },
                    ..
                }) = &record.model
                {
                    if governance.is_temporal() {
                        let limit = record.observed_at + 900;
                        assert_eq!(
                            record.expires_at,
                            Some(governance.valid_until().map_or(limit, |v| v.min(limit)))
                        );
                        assert!(!revision.records.iter().any(|r| r.key == record.key));
                    } else {
                        assert!(revision.records.iter().any(|r| r.key == record.key));
                    }
                }
            }
            prior = serde_json::to_value(&installed.current).unwrap();
            // Optional synthetic proof artifact; this fixture has no provider credentials.
            if let Some(path) = std::env::var_os("IPE_GOVERNANCE_TEST_ARTIFACT") {
                std::fs::write(path, serde_json::to_vec_pretty(&run).unwrap()).unwrap();
            }
        }
    }
    host.stop().await;
    drop(host);
    let restored = Host::new(serde_json::from_value(config).unwrap()).await.unwrap();
    assert_eq!(
        serde_json::to_value(&restored.store.lock().await.integrations["governance"].current)
            .unwrap(),
        prior
    );
    restored.stop().await;
    provider.abort();
}
