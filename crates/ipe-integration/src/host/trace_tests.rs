use super::*;

fn record(key: String) -> Record {
    Record {
        model: None,
        revision_id: format!("sha256:{}", "a".repeat(64)),
        kind: "fact".into(),
        key,
        data: Value::Null,
        receipts: vec![format!("sha256:{}", "b".repeat(64))],
        source_evidence: vec![],
        observed_at: 1,
        expires_at: Some(2),
        run_id: 1,
        adapter_digest: "fixture".into(),
    }
}

#[test]
fn trace_bounds_count_and_bytes_without_changing_records() {
    let mut count_trace = RunTrace::new();
    let small = record("subject".into());
    for step in 0..300 {
        count_trace.record(step, &small);
    }
    assert_eq!(count_trace.emissions.len(), MAX_TRACE_EMISSIONS);
    assert_eq!(count_trace.omitted, 300 - MAX_TRACE_EMISSIONS);
    assert!(serde_json::to_vec(&count_trace).unwrap().len() <= MAX_TRACE_BYTES);

    let mut byte_trace = RunTrace::new();
    let large = record("x".repeat(8192));
    for step in 0..256 {
        byte_trace.record(step, &large);
    }
    assert!(byte_trace.emissions.len() < MAX_TRACE_EMISSIONS);
    assert_eq!(byte_trace.emissions.len() + byte_trace.omitted, 256);
    assert!(byte_trace.omitted > 0);
    assert!(serde_json::to_vec(&byte_trace).unwrap().len() <= MAX_TRACE_BYTES);
    assert_eq!(large.key.len(), 8192);
    assert_eq!(large.receipts.len(), 1);
}

#[test]
fn old_run_without_trace_does_not_invent_emission_steps() {
    let legacy = json!({
        "id":1,"trigger":"manual","status":"succeeded","started_at":1,"finished_at":2,
        "elapsed_ms":1000,"adapter_digest":"old","steps":3,"metrics":Metrics::default(),
        "error":null,"records":[],"evidence":[],"summary":null,"coverage":[],"progress":[]
    });
    let run: Run = serde_json::from_value(legacy.clone()).unwrap();
    assert!(run.trace.is_none());
    assert_eq!(serde_json::to_value(&run).unwrap(), legacy);
}

async fn completed(host: &Host, run_id: u64) -> Run {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Some(run) = host.store.lock().await.integrations["trace"]
                .runs
                .iter()
                .find(|r| r.id == run_id && r.status != "running")
                .cloned()
            {
                return run;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn native_trace_links_live_reads_and_staged_emissions_without_partial_publication() {
    let entered = Arc::new(tokio::sync::Notify::new());
    let gate = Arc::new(Semaphore::new(0));
    let read_entered = entered.clone();
    let read_gate = gate.clone();
    let provider_router = Router::new().route(
        "/sample",
        get(move || {
            let entered = read_entered.clone();
            let gate = read_gate.clone();
            async move {
                entered.notify_one();
                gate.acquire().await.unwrap().forget();
                Json(json!({"value":true}))
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("http://{}/", listener.local_addr().unwrap());
    let provider = tokio::spawn(async move {
        axum::serve(listener, provider_router).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("IPE_TRACE_TEST_TOKEN", "trace_fixture_token_with_at_least_32_bytes");
    let configuration = json!({
        "listen":"127.0.0.1:0","api_token_env":"IPE_TRACE_TEST_TOKEN","state_dir":dir.path().join("state"),
        "connections":[{"id":"fixture","origin":origin}],
        "operations":[{"name":"sample","connection":"fixture","path":"/sample","parameters":{},"decoder":"json","fields":["value"]}]
    });
    let host = Host::new(serde_json::from_value(configuration.clone()).unwrap()).await.unwrap();
    let source = r#"function step(context,responses,state) {
        if(state===null)return ipe.exchange([ipe.call('read','sample')],{phase:'read'});
        if(state.phase==='read') {
            const read=ipe.response(responses,'read');
            const model={schema_version:1,scope:'fixture',claim:'provider_reported',entity:{kind:'fact',subject:'fixture:subject',predicate:'ready',value:read.data[0].value,authority:'fixture',execution:null}};
            return ipe.exchange([ipe.call('hold','sample')],{phase:'emitted'},[ipe.observation(model,{},[read.receipt])]);
        }
        if(context.input.fail)throw new Error('private mapper failure');
        return ipe.complete({coverage:'sample'});
    }"#;
    let mut prior = json!([]);
    let mut failed_id = 0;
    for fail in [false, true] {
        host.install(
            "trace",
            Installation {
                source: source.into(),
                operations: vec!["sample".into()],
                input: json!({"fail":fail}),
                interval_seconds: 300,
                enabled: false,
                fact_ttl_seconds: 900,
                output_kinds: vec!["fact".into()],
            },
        )
        .await
        .unwrap();
        let run_id = host.start("trace", "manual").await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), entered.notified()).await.unwrap();
        {
            let store = host.store.lock().await;
            let run = store.integrations["trace"].runs.iter().find(|r| r.id == run_id).unwrap();
            assert_eq!(run.evidence[0]["status"], "pending");
            assert_eq!(run.evidence[0]["step"], 1);
            assert_eq!(run.evidence[0]["request_id"], "read");
            assert!(run.trace.as_ref().unwrap().emissions.is_empty());
        }
        gate.add_permits(1);
        tokio::time::timeout(Duration::from_secs(5), entered.notified()).await.unwrap();
        {
            let store = host.store.lock().await;
            let installed = &store.integrations["trace"];
            let run = installed.runs.iter().find(|r| r.id == run_id).unwrap();
            assert_eq!(run.status, "running");
            assert!(run.records.is_empty());
            assert_eq!(run.evidence[0]["status"], "observed");
            assert_eq!(run.evidence[1]["status"], "pending");
            assert_eq!(run.evidence[1]["step"], 2);
            assert_eq!(run.evidence[1]["request_id"], "hold");
            let trace = run.trace.as_ref().unwrap();
            assert_eq!(trace.schema_version, 1);
            assert_eq!(trace.emissions.len(), 1);
            assert_eq!(trace.emissions[0].step, 2);
            assert_eq!(trace.emissions[0].kind, "fact");
            assert_eq!(
                trace.emissions[0].receipts[0],
                run.evidence[0]["receipt"].as_str().unwrap()
            );
            assert_eq!(serde_json::to_value(&installed.current).unwrap(), prior);
        }
        gate.add_permits(1);
        let run = completed(&host, run_id).await;
        let trace = run.trace.as_ref().unwrap();
        assert_eq!(trace.emissions.len(), 1);
        assert_eq!(trace.omitted, 0);
        if fail {
            failed_id = run_id;
            assert_eq!(run.status, "failed");
            assert_eq!(run.error.as_deref(), Some("script_failed"));
            assert!(run.records.is_empty());
            assert_eq!(
                serde_json::to_value(&host.store.lock().await.integrations["trace"].current)
                    .unwrap(),
                prior
            );
        } else {
            assert_eq!(run.status, "succeeded");
            assert_eq!(trace.emissions[0].key, run.records[0].key);
            assert_eq!(trace.emissions[0].revision_id, run.records[0].revision_id);
            prior = serde_json::to_value(&host.store.lock().await.integrations["trace"].current)
                .unwrap();
        }
    }
    host.stop().await;
    drop(host);
    let restored = Host::new(serde_json::from_value(configuration).unwrap()).await.unwrap();
    {
        let store = restored.store.lock().await;
        let installed = &store.integrations["trace"];
        let run = installed.runs.iter().find(|r| r.id == failed_id).unwrap();
        assert_eq!(run.status, "failed");
        assert!(run.records.is_empty());
        assert_eq!(run.trace.as_ref().unwrap().emissions[0].step, 2);
        assert_eq!(serde_json::to_value(&installed.current).unwrap(), prior);
    }
    restored.stop().await;
    provider.abort();
}
