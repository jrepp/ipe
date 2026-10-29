use super::*;
use axum::response::IntoResponse;
use std::sync::atomic::AtomicUsize;

const PROGRAM: &str = r#"
function step(context,responses,state) {
  if(state===null)return ipe.exchange([ipe.call('read','sample')],{phase:'read'});
  if(state.phase==='done')return ipe.complete({coverage:'sample'});
  const r=ipe.response(responses,'read');
  if(r.code)throw new Error('read_failed');
  const value=ipe.records(responses,'read')[0];
  const observations=['contract','fact','outcome'].map(kind=>({kind,key:'control_1',data:value,receipts:[r.receipt]}));
  return ipe.exchange([],{phase:'done'},observations);
}
"#;
fn package(enabled: bool) -> Installation {
    Installation {
        source: PROGRAM.into(),
        operations: vec!["sample".into()],
        input: json!({}),
        interval_seconds: 1,
        enabled,
        fact_ttl_seconds: 1,
        output_kinds: vec!["contract".into(), "fact".into(), "outcome".into()],
    }
}
async fn fixture() -> (String, Arc<AtomicUsize>, tokio::task::JoinHandle<()>) {
    let attempts = Arc::new(AtomicUsize::new(0));
    let count = attempts.clone();
    let app = Router::new().route(
        "/sample",
        get(move || {
            let count = count.clone();
            async move {
                let attempt = count.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    (StatusCode::SERVICE_UNAVAILABLE, "private upstream message").into_response()
                } else {
                    tokio::time::sleep(Duration::from_millis(80)).await;
                    Json(json!({"status":"success","duration_ms":42,"hidden":"private"}))
                        .into_response()
                }
            }
        }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}/"), attempts, task)
}
fn config(dir: &Path, origin: &str) -> Config {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let key = format!("IPE_HOST_TEST_{}", NEXT.fetch_add(1, Ordering::SeqCst));
    std::env::set_var(&key, "host_test_token_with_at_least_32_bytes");
    serde_json::from_value(json!({"listen":"127.0.0.1:0","api_token_env":key,"state_dir":dir.join("state"),"connections":[{"id":"test","origin":origin}],"operations":[{"name":"sample","connection":"test","path":"/sample","parameters":{},"decoder":"json","fields":["status","duration_ms"]}]})).unwrap()
}
async fn completed(host: &Host, id: &str, run_id: u64) -> Run {
    for _ in 0..100 {
        if let Some(run) = host.store.lock().await.integrations[id]
            .runs
            .iter()
            .find(|r| r.id == run_id && r.status != "running")
            .cloned()
        {
            return run;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    panic!("run did not finish");
}
#[tokio::test]
async fn hosted_code_retries_retains_typed_records_and_survives_restart() {
    let (origin, attempts, provider) = fixture().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::new(config(dir.path(), &origin)).await.unwrap();
    host.install("example", package(false)).await.unwrap();
    let first = host.start("example", "manual").await.unwrap();
    assert!(host.start("example", "manual").await.is_err());
    let run = completed(&host, "example", first).await;
    assert_eq!(run.status, "succeeded");
    assert_eq!(run.metrics.retries, 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(run.records.len(), 3);
    assert_eq!(run.evidence.len(), 1);
    assert_eq!(run.records[0].source_evidence[0]["operation"], "sample");
    assert!(run.records.iter().all(|r| r.data.get("hidden").is_none()));
    let fact = run.records.iter().find(|r| r.kind == "fact").unwrap();
    assert_eq!(fact.expires_at, Some(fact.observed_at + 1));
    let before = host.store.lock().await.integrations["example"].current.clone();
    let mut bad = package(false);
    bad.source = "function step(){throw new Error('secret details');}".into();
    host.install("example", bad).await.unwrap();
    let failed = host.start("example", "manual").await.unwrap();
    let run = completed(&host, "example", failed).await;
    assert_eq!(run.status, "failed");
    assert_eq!(run.error.as_deref(), Some("script_failed"));
    assert_eq!(
        host.store.lock().await.integrations["example"].current[0].observed_at,
        before[0].observed_at
    );
    host.stop().await;
    drop(host);
    tokio::time::sleep(Duration::from_secs(2)).await;
    let restored = Host::new(config(dir.path(), &origin)).await.unwrap();
    let store = restored.store.lock().await;
    assert_eq!(store.integrations["example"].runs.len(), 2);
    assert!(
        store.integrations["example"]
            .current
            .iter()
            .find(|r| r.kind == "fact")
            .unwrap()
            .expires_at
            .unwrap()
            <= now()
    );
    drop(store);
    restored.stop().await;
    provider.abort();
    let _ = provider.await;
}
#[tokio::test]
async fn scheduler_runs_hosted_code_and_manual_calls_do_not_overlap() {
    let (origin, _, provider) = fixture().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::new(config(dir.path(), &origin)).await.unwrap();
    host.install("scheduled", package(true)).await.unwrap();
    let scheduler = tokio::spawn(host.clone().scheduler());
    let mut success = false;
    for _ in 0..100 {
        if host.store.lock().await.integrations["scheduled"]
            .runs
            .iter()
            .any(|r| r.trigger == "schedule" && r.status == "succeeded")
        {
            success = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert!(success);
    host.stop().await;
    scheduler.await.unwrap();
    provider.abort();
    let _ = provider.await;
}
#[tokio::test]
async fn unauthenticated_install_is_rejected_and_forged_operations_do_no_io() {
    let (origin, attempts, provider) = fixture().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::new(config(dir.path(), &origin)).await.unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app = host.router();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/v1/integrations/test");
    let body = serde_json::to_string(&package(false)).unwrap();
    assert_eq!(
        client.put(&url).body(body.clone()).send().await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let token = host.token.expose_secret();
    assert_eq!(
        client.put(&url).bearer_auth(token).body(body).send().await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        client
            .put(&url)
            .bearer_auth(token)
            .body("{\"source\":\"a\",\"source\":\"b\"}")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut bad = package(false);
    bad.source = "function step(){return ipe.exchange([ipe.call('r','not_granted')]);}".into();
    host.install("test", bad).await.unwrap();
    let run = host.start("test", "manual").await.unwrap();
    assert_eq!(
        completed(&host, "test", run).await.error.as_deref(),
        Some("operation_not_permitted")
    );
    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    let mut forged = package(false);
    forged.source="function step(){return ipe.exchange([],{},[{kind:'fact',key:'x',data:true,receipts:['fake']}]);}".into();
    host.install("test", forged).await.unwrap();
    let run = host.start("test", "manual").await.unwrap();
    assert_eq!(
        completed(&host, "test", run).await.error.as_deref(),
        Some("observation_not_permitted")
    );
    host.stop().await;
    server.abort();
    provider.abort();
    let _ = server.await;
    let _ = provider.await;
}
#[tokio::test]
async fn durable_state_has_one_writer_and_interrupted_runs_are_not_success() {
    let (origin, _, provider) = fixture().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::new(config(dir.path(), &origin)).await.unwrap();
    assert!(Host::new(config(dir.path(), &origin)).await.is_err());
    host.install("test", package(false)).await.unwrap();
    {
        let mut store = host.store.lock().await;
        store.integrations.get_mut("test").unwrap().runs.push(Run {
            id: 99,
            trigger: "test".into(),
            status: "running".into(),
            started_at: now(),
            finished_at: None,
            elapsed_ms: 0,
            adapter_digest: "old".into(),
            steps: 0,
            metrics: Metrics::default(),
            error: None,
            records: vec![],
            evidence: vec![],
            summary: Value::Null,
        });
        persist(&dir.path().join("state"), &store).unwrap();
    }
    drop(host);
    let restored = Host::new(config(dir.path(), &origin)).await.unwrap();
    assert_eq!(restored.store.lock().await.integrations["test"].runs[0].status, "interrupted");
    restored.stop().await;
    provider.abort();
    let _ = provider.await;
}
#[test]
fn authored_program_compiles_and_time_helpers_are_deterministic() {
    ScriptAdapter::from_source(
        include_str!("../../../../testing/integration/hosted/github-actions.js"),
        false,
    )
    .unwrap();
}

#[tokio::test]
async fn monitor_session_requires_token_and_protects_cookie_requests() {
    let (origin, _, provider) = fixture().await;
    let dir = tempfile::tempdir().unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let mut settings = config(dir.path(), &origin);
    settings.listen = address;
    let host = Host::new(settings).await.unwrap();
    host.install("example", package(false)).await.unwrap();
    let app = host.router();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let base = format!("http://{address}");
    let login = |value: &str| {
        client
            .post(format!("{base}/session"))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(format!("token={value}"))
    };
    assert_eq!(login("invalid").send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
    let response = login(host.token.expose_secret()).send().await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers()["location"], "/");
    let cookie = response.headers()["set-cookie"].to_str().unwrap();
    assert!(cookie.contains("HttpOnly; SameSite=Strict"));
    assert!(!cookie.contains(host.token.expose_secret()));
    let cookie = cookie.split(';').next().unwrap();
    let url = format!("{base}/v1/integrations/example");
    let read = || client.get(&url).header("cookie", cookie);
    assert_eq!(read().send().await.unwrap().status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        read()
            .header("x-ipe-monitor", "1")
            .header("origin", "https://other.test")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        read()
            .header("x-ipe-monitor", "1")
            .header("host", "other.test")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = read().header("x-ipe-monitor", "1").send().await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let text = response.text().await.unwrap();
    assert!(text.contains("source"));
    assert!(!text.contains(host.token.expose_secret()));
    assert_eq!(
        client
            .post(format!("{base}/v1/integrations/example/runs"))
            .header("cookie", cookie)
            .header("x-ipe-monitor", "1")
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::ACCEPTED
    );
    host.stop().await;
    server.abort();
    provider.abort();
}

#[tokio::test]
async fn external_outcomes_survive_polling_collector_eviction_and_restart() {
    let (origin, _, provider) = fixture().await;
    let dir = tempfile::tempdir().unwrap();
    let host = Host::new(config(dir.path(), &origin)).await.unwrap();
    host.install("example", package(false)).await.unwrap();
    for _ in 0..2 {
        let id = host.start("example", "manual").await.unwrap();
        assert_eq!(completed(&host, "example", id).await.status, "succeeded");
    }
    {
        let store = host.store.lock().await;
        let execution = &store.integrations["example"].outcomes.executions["control_1"];
        assert_eq!(
            execution.changes.len(),
            1,
            "identical polling is not a new execution or outcome change"
        );
    }
    let mut changed = package(false);
    changed.source = PROGRAM.replace("data:value", "data:{...value,status:'failure'}");
    host.install("example", changed).await.unwrap();
    let id = host.start("example", "manual").await.unwrap();
    assert_eq!(completed(&host, "example", id).await.status, "succeeded");
    let mut no_outcomes = package(false);
    no_outcomes.source = PROGRAM.replace("['contract','fact','outcome']", "['contract','fact']");
    host.install("example", no_outcomes).await.unwrap();
    for _ in 0..HISTORY + 1 {
        let id = host.start("example", "manual").await.unwrap();
        assert_eq!(completed(&host, "example", id).await.status, "succeeded");
    }
    host.stop().await;
    drop(host);
    let restored = Host::new(config(dir.path(), &origin)).await.unwrap();
    {
        let store = restored.store.lock().await;
        let installed = &store.integrations["example"];
        assert_eq!(installed.runs.len(), HISTORY);
        assert!(installed.runs.iter().all(|r| r.records.iter().all(|r| r.kind != "outcome")));
        assert_eq!(installed.outcomes.executions.len(), 1);
        let execution = &installed.outcomes.executions["control_1"];
        assert_eq!(execution.changes.len(), 2);
        assert_eq!(execution.latest.data["status"], "failure");
        assert_eq!(execution.changes[0].data["status"], "success");
    }
    assert!(
        outcome_archive(State(restored.clone()), HeaderMap::new(), ApiPath("example".into()))
            .await
            .is_err()
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        "authorization",
        format!("Bearer {}", restored.token.expose_secret()).parse().unwrap(),
    );
    let Json(export) = outcome_archive(State(restored.clone()), headers, ApiPath("example".into()))
        .await
        .unwrap();
    assert_eq!(export["schema"], "ipe.integration.outcome-archive.v1");
    assert_eq!(export["executions"]["control_1"]["changes"].as_array().unwrap().len(), 2);
    assert!(!export.to_string().contains(restored.token.expose_secret()));
    restored.stop().await;
    provider.abort();
}
