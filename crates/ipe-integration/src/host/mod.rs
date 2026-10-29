//! A local installation host: code, scheduling, brokered reads, typed records,
//! durable bounded history, and authenticated monitoring share one endpoint.
mod broker;
mod coverage;
mod credentials;
mod explanations;
mod model;
mod outcomes;
mod revisions;
use crate::script::ScriptAdapter;
use axum::{
    extract::{DefaultBodyLimit, Path as ApiPath, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{get, post, put},
    Form, Json, Router,
};
use broker::{Broker, Metrics};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::{File, OpenOptions},
    io::{Read, Write},
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use tokio::sync::{Mutex, Semaphore};
use tokio::task::JoinSet;

type Result<T> = std::result::Result<T, &'static str>;
type ApiError = (StatusCode, Json<Value>);
const MAX_STORE: usize = 16 * 1024 * 1024;
const MAX_RUN_BYTES: usize = 512 * 1024;
const HISTORY: usize = 8;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub listen: SocketAddr,
    api_token_env: String,
    state_dir: PathBuf,
    connections: Vec<broker::ConnectionConfig>,
    operations: Vec<broker::Operation>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Installation {
    source: String,
    operations: Vec<String>,
    input: Value,
    interval_seconds: u64,
    enabled: bool,
    fact_ttl_seconds: u64,
    output_kinds: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Record {
    #[serde(default)]
    model: Option<model::Model>,
    #[serde(default)]
    revision_id: String,
    kind: String,
    key: String,
    data: Value,
    receipts: Vec<String>,
    #[serde(default)]
    source_evidence: Vec<Value>,
    observed_at: u64,
    expires_at: Option<u64>,
    run_id: u64,
    adapter_digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
struct Run {
    id: u64,
    trigger: String,
    status: String,
    started_at: u64,
    finished_at: Option<u64>,
    elapsed_ms: u64,
    adapter_digest: String,
    steps: usize,
    metrics: Metrics,
    error: Option<String>,
    records: Vec<Record>,
    #[serde(default)]
    evidence: Vec<Value>,
    summary: Value,
    #[serde(default)]
    coverage: Vec<coverage::VerifiedScope>,
}
#[derive(Clone, Serialize, Deserialize)]
struct Installed {
    package: Installation,
    digest: String,
    next_due: u64,
    current: Vec<Record>,
    #[serde(default)]
    outcomes: outcomes::Archive,
    #[serde(default)]
    revisions: revisions::Archive,
    runs: Vec<Run>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
struct Store {
    next_run: u64,
    integrations: BTreeMap<String, Installed>,
}
pub struct Host {
    config: Config,
    broker: Broker,
    token: SecretString,
    store: Mutex<Store>,
    adapters: Mutex<HashMap<String, ScriptAdapter>>,
    workers: Mutex<JoinSet<()>>,
    slots: Arc<Semaphore>,
    stopping: AtomicBool,
    _lock: File,
}
fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}
fn error(status: StatusCode, code: &str) -> ApiError {
    (status, Json(json!({"code":code})))
}
fn validate(package: &Installation, broker: &Broker) -> Result<()> {
    if package.source.len() > 128 * 1024
        || package.source.is_empty()
        || package.operations.is_empty()
        || package.operations.len() > 16
        || package.operations.iter().any(|n| !broker.operations.contains_key(n))
        || !(1..=86400).contains(&package.interval_seconds)
        || !(1..=86400).contains(&package.fact_ttl_seconds)
        || package.output_kinds.is_empty()
        || package
            .output_kinds
            .iter()
            .any(|k| !model::KINDS.contains(&k.as_str()) && k != "contract")
        || serde_json::to_vec(&package.input).map_err(|_| "invalid_input")?.len() > 8192
    {
        return Err("invalid_installation");
    }
    Ok(())
}
fn private_file(path: &Path) -> Result<File> {
    if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("unsafe_state_path");
    }
    let mut options = OpenOptions::new();
    options.create(true).truncate(false).read(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).map_err(|_| "state_unavailable")
}
fn persist(dir: &Path, store: &Store) -> Result<()> {
    let bytes = serde_json::to_vec(store).map_err(|_| "state_invalid")?;
    if bytes.len() > MAX_STORE {
        return Err("state_capacity");
    }
    let tmp = dir.join("state.tmp");
    let mut file = private_file(&tmp)?;
    file.set_len(0).map_err(|_| "state_write_failed")?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "state_write_failed")?;
    std::fs::rename(tmp, dir.join("state.json")).map_err(|_| "state_write_failed")?;
    File::open(dir).and_then(|f| f.sync_all()).map_err(|_| "state_write_failed")
}
impl Host {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        if !config.listen.ip().is_loopback() {
            return Err("listen_must_be_loopback");
        }
        let token = SecretString::from(
            std::env::var(&config.api_token_env).map_err(|_| "missing_api_token")?,
        );
        if !(32..=8192).contains(&token.expose_secret().len()) {
            return Err("invalid_api_token");
        }
        let broker = Broker::new(config.connections.clone(), config.operations.clone()).await?;
        if !config.state_dir.exists() {
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            builder.create(&config.state_dir).map_err(|_| "state_unavailable")?;
        }
        let metadata =
            std::fs::symlink_metadata(&config.state_dir).map_err(|_| "state_unavailable")?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err("unsafe_state_path");
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("state_directory_not_private");
            }
        }
        let lock = private_file(&config.state_dir.join("writer.lock"))?;
        lock.try_lock().map_err(|_| "state_locked")?;
        let path = config.state_dir.join("state.json");
        let mut store: Store = if path.exists() {
            let mut bytes = Vec::new();
            private_file(&path)?
                .take((MAX_STORE + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| "state_unavailable")?;
            if bytes.len() > MAX_STORE {
                return Err("state_capacity");
            }
            ipe_json::parse(&bytes).map_err(|_| "state_invalid")?
        } else {
            Store::default()
        };
        if store.integrations.len() > 4 {
            return Err("state_capacity");
        }
        let mut adapters = HashMap::new();
        for (id, installed) in &mut store.integrations {
            if !broker::name(id) {
                return Err("state_invalid");
            }
            validate(&installed.package, &broker)?;
            let adapter = ScriptAdapter::from_source(&installed.package.source, false)?;
            installed.digest = adapter.digest.clone();
            for run in &mut installed.runs {
                if run.status == "running" {
                    run.status = "interrupted".into();
                    run.error = Some("host_restarted".into());
                    run.finished_at = Some(now());
                }
            }
            // Migrate retained legacy outcomes once without changing observation times.
            installed.outcomes.migrate_legacy();
            if installed.outcomes.executions.is_empty()
                && installed.outcomes.legacy_untyped.is_empty()
            {
                for run in &installed.runs {
                    if run.status == "succeeded" {
                        installed.outcomes.apply(&run.records)?;
                    }
                }
            }
            // Restart does not replay missed intervals or refresh evidence.
            installed.next_due = now() + installed.package.interval_seconds;
            adapters.insert(id.clone(), adapter);
        }
        persist(&config.state_dir, &store)?;
        Ok(Arc::new(Self {
            config,
            broker,
            token,
            store: Mutex::new(store),
            adapters: Mutex::new(adapters),
            workers: Mutex::new(JoinSet::new()),
            slots: Arc::new(Semaphore::new(2)),
            stopping: AtomicBool::new(false),
            _lock: lock,
        }))
    }
    pub fn router(self: &Arc<Self>) -> Router {
        Router::new()
            .route(
                "/",
                get(|| async {
                    Html(include_str!("../../../../tools/integration-host/index.html"))
                }),
            )
            .route("/session", post(session))
            .route("/v1/host/capabilities", get(capabilities))
            .route("/v1/host/record-schema", get(record_schema))
            .route("/v1/integrations", get(list))
            .route("/v1/integrations/:id", put(install).get(installation))
            .route("/v1/integrations/:id/runs", post(start).get(history))
            .route("/v1/integrations/:id/records", get(records))
            .route("/v1/integrations/:id/outcomes", get(outcome_archive))
            .route("/v1/integrations/:id/coverage", get(collection_coverage))
            .route("/v1/integrations/:id/revisions", get(revision_list))
            .route("/v1/integrations/:id/catalog", get(review_catalog))
            .route("/v1/integrations/:id/explanations", get(operation_explanations))
            .route("/v1/integrations/:id/revisions/:revision", get(revision_detail))
            .route("/v1/integrations/:id/revisions/:revision/accept", post(revision_accept))
            .route("/v1/integrations/:id/revisions/:revision/restore", post(revision_restore))
            .layer(DefaultBodyLimit::max(160 * 1024))
            .with_state(self.clone())
    }
    fn monitor_cookie(&self) -> String {
        broker::digest(format!("ipe-monitor:{}", self.token.expose_secret()).as_bytes())
            .trim_start_matches("sha256:")
            .to_owned()
    }
    fn local_origin(&self, headers: &HeaderMap) -> bool {
        let expected = self.config.listen.to_string();
        headers.get("host").and_then(|v| v.to_str().ok()) == Some(expected.as_str())
            && headers
                .get("origin")
                .is_none_or(|v| v.to_str().ok() == Some(format!("http://{expected}").as_str()))
    }
    fn authorize(&self, headers: &HeaderMap) -> std::result::Result<(), ApiError> {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        let cookie = headers
            .get("cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split(';').find_map(|part| part.trim().strip_prefix("ipe_monitor=")))
            .unwrap_or("");
        let monitor = headers.get("x-ipe-monitor").is_some_and(|v| v == "1")
            && self.local_origin(headers)
            && bool::from(cookie.as_bytes().ct_eq(self.monitor_cookie().as_bytes()));
        if !monitor && !bool::from(provided.as_bytes().ct_eq(self.token.expose_secret().as_bytes()))
        {
            return Err(error(StatusCode::UNAUTHORIZED, "unauthorized"));
        }
        Ok(())
    }
    async fn install(&self, id: &str, package: Installation) -> Result<String> {
        if !broker::name(id) {
            return Err("invalid_integration_id");
        }
        validate(&package, &self.broker)?;
        if package.output_kinds.iter().any(|kind| kind == "contract") {
            return Err("legacy_contract_output_not_supported");
        }
        let source = package.source.clone();
        let adapter =
            tokio::task::spawn_blocking(move || ScriptAdapter::from_source(&source, false))
                .await
                .map_err(|_| "script_failed")??;
        let mut store = self.store.lock().await;
        if store
            .integrations
            .get(id)
            .is_some_and(|i| i.runs.iter().any(|r| r.status == "running"))
        {
            return Err("integration_busy");
        }
        if !store.integrations.contains_key(id) && store.integrations.len() >= 4 {
            return Err("installation_limit");
        }
        let prior = store.clone();
        let digest = adapter.digest.clone();
        let old = store.integrations.remove(id);
        store.integrations.insert(
            id.into(),
            Installed {
                next_due: now() + package.interval_seconds,
                package,
                digest: digest.clone(),
                current: old.as_ref().map_or_else(Vec::new, |i| i.current.clone()),
                outcomes: old
                    .as_ref()
                    .map_or_else(outcomes::Archive::default, |i| i.outcomes.clone()),
                revisions: old
                    .as_ref()
                    .map_or_else(revisions::Archive::default, |i| i.revisions.clone()),
                runs: old.map_or_else(Vec::new, |i| i.runs),
            },
        );
        if let Err(code) = persist(&self.config.state_dir, &store) {
            *store = prior;
            return Err(code);
        }
        // Hold the state lock until the executable matches the durable installation.
        self.adapters.lock().await.insert(id.into(), adapter);
        Ok(digest)
    }
    async fn start(self: &Arc<Self>, id: &str, trigger: &str) -> Result<u64> {
        if self.stopping.load(Ordering::SeqCst) {
            return Err("host_stopping");
        }
        let permit = self.slots.clone().try_acquire_owned().map_err(|_| "host_busy")?;
        let mut store = self.store.lock().await;
        let installed = store.integrations.get(id).ok_or("integration_unavailable")?;
        if installed.runs.iter().any(|r| r.status == "running") {
            return Err("integration_busy");
        }
        let package = installed.package.clone();
        let adapter =
            self.adapters.lock().await.get(id).cloned().ok_or("integration_unavailable")?;
        let prior = store.clone();
        store.next_run = store.next_run.checked_add(1).ok_or("run_id_exhausted")?;
        let run_id = store.next_run;
        let installed = store.integrations.get_mut(id).ok_or("integration_unavailable")?;
        installed.next_due = now() + package.interval_seconds;
        if installed.runs.len() >= HISTORY {
            installed.runs.remove(0);
        }
        installed.runs.push(Run {
            id: run_id,
            trigger: trigger.into(),
            status: "running".into(),
            started_at: now(),
            finished_at: None,
            elapsed_ms: 0,
            adapter_digest: adapter.digest.clone(),
            steps: 0,
            metrics: Metrics::default(),
            error: None,
            records: Vec::new(),
            evidence: Vec::new(),
            summary: Value::Null,
            coverage: Vec::new(),
        });
        if let Err(code) = persist(&self.config.state_dir, &store) {
            *store = prior;
            return Err(code);
        }
        let host = self.clone();
        let id = id.to_owned();
        drop(store);
        let mut workers = self.workers.lock().await;
        if self.stopping.load(Ordering::SeqCst) {
            return Err("host_stopping");
        }
        workers.spawn(async move {
            let _permit = permit;
            let begin = Instant::now();
            let mut metrics = Metrics::default();
            let mut count = 0;
            let result = tokio::time::timeout(
                Duration::from_secs(180),
                host.execute(&id, run_id, &package, &adapter, &mut metrics, &mut count),
            )
            .await
            .unwrap_or(Err("run_deadline"));
            host.finish(&id, run_id, result, metrics, count, begin.elapsed().as_millis() as u64)
                .await;
        });
        Ok(run_id)
    }
    async fn execute(
        &self,
        id: &str,
        run_id: u64,
        package: &Installation,
        adapter: &ScriptAdapter,
        metrics: &mut Metrics,
        count: &mut usize,
    ) -> Result<(Value, Vec<Record>, Vec<Value>)> {
        let context = json!({"integration_id":id,"run_id":run_id,"observed_at":now(),"input":package.input,"sdk_version":crate::script::SDK_VERSION,"operations":package.operations});
        let mut state = Value::Null;
        let mut responses = json!([]);
        let mut observations = Vec::new();
        let mut receipts = HashSet::new();
        let mut evidence: Vec<Value> = Vec::new();
        let mut ids = HashSet::new();
        let mut output_bytes = 0;
        for step in 1..=128 {
            if self.stopping.load(Ordering::SeqCst) {
                return Err("host_stopping");
            }
            *count = step;
            let result = adapter.step(context.clone(), responses, state).await?;
            state = result.state;
            for observation in result.observations {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Wire {
                    model: model::Model,
                    data: Value,
                    receipts: Vec<String>,
                }
                let record: Wire =
                    serde_json::from_value(observation).map_err(|_| "invalid_observation")?;
                let kind = record.model.kind().to_owned();
                if !package.output_kinds.contains(&kind)
                    || record.receipts.is_empty()
                    || record.receipts.len() > 8
                    || record.receipts.iter().any(|r| !receipts.contains(r))
                {
                    return Err("observation_not_permitted");
                }
                record.model.validate()?;
                let key = record.model.key();
                if observations.iter().any(|r: &Record| r.kind == kind && r.key == key) {
                    return Err("duplicate_model_identity");
                }
                if observations.len() >= 256 {
                    return Err("observation_limit");
                }
                output_bytes += serde_json::to_vec(&json!([record.model, record.data]))
                    .map_err(|_| "invalid_observation")?
                    .len();
                if output_bytes > MAX_RUN_BYTES {
                    return Err("observation_byte_limit");
                }
                let observed_at = evidence
                    .iter()
                    .filter(|e| record.receipts.iter().any(|r| e["receipt"] == *r))
                    .filter_map(|e| e["observed_at"].as_u64())
                    .min()
                    .ok_or("observation_time_missing")?;
                observations.push(Record {
                    revision_id: record.model.revision(&adapter.digest)?,
                    model: Some(record.model),
                    kind: kind.clone(),
                    key,
                    data: record.data,
                    source_evidence: evidence
                        .iter()
                        .filter(|e: &&Value| record.receipts.iter().any(|r| e["receipt"] == *r))
                        .cloned()
                        .collect(),
                    receipts: record.receipts,
                    observed_at,
                    expires_at: (kind == "fact").then(|| observed_at + package.fact_ttl_seconds),
                    run_id,
                    adapter_digest: adapter.digest.clone(),
                });
            }
            if let Some(output) = result.outputs {
                model::validate_graph(&observations)?;
                coverage::verify(&output, &observations, &evidence)?;
                return Ok((output, observations, evidence));
            }
            // Validate the whole batch before any I/O; SDK checks are not grants.
            for request in &result.requests {
                if !package.operations.contains(&request.operation)
                    || !ids.insert(request.id.clone())
                {
                    return Err("operation_not_permitted");
                }
                self.broker.validate(&request.operation, &request.params)?;
            }
            let mut next = Vec::new();
            for request in result.requests {
                evidence.push(json!({"operation":request.operation,"parameters":request.params,"started_at":now(),"status":"pending"}));
                if let Some(run) = self
                    .store
                    .lock()
                    .await
                    .integrations
                    .get_mut(id)
                    .and_then(|i| i.runs.iter_mut().find(|r| r.id == run_id))
                {
                    run.evidence = evidence.clone();
                }
                match self.broker.read(&request.operation, &request.params, metrics).await {
                    Ok(data) => {
                        let receipt = broker::digest(
                            serde_json::to_string(&json!([
                                id,
                                run_id,
                                adapter.digest,
                                request.id,
                                request.operation,
                                request.params,
                                data
                            ]))
                            .map_err(|_| "receipt_failed")?
                            .as_bytes(),
                        );
                        if let Some(read) = evidence.last_mut() {
                            read["receipt"] = json!(receipt);
                            read["status"] = json!("observed");
                            read["source_digest"] = data["source_digest"].clone();
                            read["observed_at"] = json!(now());
                            read["truncated"] = data["truncated"].clone();
                            read["pagination"] = data["pagination"].clone();
                            read["snapshot_source"] = data["snapshot_source"].clone();
                        }
                        receipts.insert(receipt.clone());
                        next.push(json!({"id":request.id,"operation":request.operation,"receipt":receipt,"data":data["records"],"source_digest":data["source_digest"],"truncated":data["truncated"]}));
                    },
                    Err(code) => {
                        if let Some(read) = evidence.last_mut() {
                            read["status"] = json!("unavailable");
                            read["code"] = json!(code);
                            read["observed_at"] = json!(now());
                        }
                        next.push(
                            json!({"id":request.id,"operation":request.operation,"code":code}),
                        );
                    },
                }
            }
            responses = Value::Array(next);
            if serde_json::to_vec(&responses).map_err(|_| "response_failed")?.len() > 64 * 1024 {
                return Err("response_batch_limit");
            }
            if let Some(run) = self
                .store
                .lock()
                .await
                .integrations
                .get_mut(id)
                .and_then(|i| i.runs.iter_mut().find(|r| r.id == run_id))
            {
                run.evidence = evidence.clone();
                run.steps = step;
                run.metrics = metrics.clone();
            }
        }
        Err("run_step_limit")
    }
    async fn finish(
        &self,
        id: &str,
        run_id: u64,
        result: Result<(Value, Vec<Record>, Vec<Value>)>,
        metrics: Metrics,
        steps: usize,
        elapsed_ms: u64,
    ) {
        let mut store = self.store.lock().await;
        let prior = store.clone();
        if let Some(installed) = store.integrations.get_mut(id) {
            let mut terminal = result;
            let mut scopes = Vec::new();
            let mut next_current = None;
            if let Ok((summary, records, evidence)) = &terminal {
                match coverage::verify(summary, records, evidence).and_then(|verified| {
                    let next = coverage::reconcile(&installed.current, records, &verified)?;
                    scopes = verified;
                    Ok(next)
                }) {
                    Ok(next) => next_current = Some(next),
                    Err(code) => terminal = Err(code),
                }
            }
            if let (Ok((_, records, _)), Some(next)) = (&terminal, next_current) {
                // Commit all three catalogs together; capacity errors cannot publish a partial capture.
                let mut archive = installed.outcomes.clone();
                let mut revisions = installed.revisions.clone();
                let operations = installed
                    .package
                    .operations
                    .iter()
                    .filter_map(|name| self.broker.operations.get(name))
                    .collect::<Vec<_>>();
                let update = serde_json::to_vec(&operations)
                    .map_err(|_| "revision_invalid")
                    .and_then(|bytes| {
                        revisions.capture(
                            &installed.package,
                            &broker::digest(&bytes),
                            &next,
                            &scopes,
                            run_id,
                            now(),
                        )?;
                        archive.apply(records)
                    });
                match update {
                    Ok(()) => {
                        installed.current = next;
                        installed.outcomes = archive;
                        installed.revisions = revisions;
                    },
                    Err(code) => terminal = Err(code),
                }
            }
            if let Some(run) = installed.runs.iter_mut().find(|r| r.id == run_id) {
                run.finished_at = Some(now());
                run.elapsed_ms = elapsed_ms;
                run.metrics = metrics;
                run.steps = steps;
                match terminal {
                    Ok((summary, records, evidence)) => {
                        run.status = "succeeded".into();
                        run.coverage = scopes;
                        run.summary = summary;
                        run.records = records;
                        run.evidence = evidence;
                    },
                    Err(code) => {
                        run.status = "failed".into();
                        run.error = Some(code.into());
                    },
                }
            }
        }
        if let Err(code) = persist(&self.config.state_dir, &store) {
            *store = prior;
            if let Some(run) = store
                .integrations
                .get_mut(id)
                .and_then(|i| i.runs.iter_mut().find(|r| r.id == run_id))
            {
                run.status = "failed".into();
                run.error = Some(code.into());
                run.finished_at = Some(now());
            }
        }
    }
    pub async fn scheduler(self: Arc<Self>) {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            if self.stopping.load(Ordering::SeqCst) {
                break;
            }
            let due: Vec<_> = self
                .store
                .lock()
                .await
                .integrations
                .iter()
                .filter(|(_, i)| i.package.enabled && i.next_due <= now())
                .map(|(id, _)| id.clone())
                .collect();
            for id in due {
                let _ = self.start(&id, "schedule").await;
            }
            let mut workers = self.workers.lock().await;
            while workers.try_join_next().is_some() {}
        }
    }
    pub async fn stop(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        let mut workers = self.workers.lock().await;
        // Native calls time out in eight seconds; the guest is bounded to 100 ms.
        if tokio::time::timeout(Duration::from_secs(12), async {
            while workers.join_next().await.is_some() {}
        })
        .await
        .is_err()
        {
            workers.abort_all();
            while workers.join_next().await.is_some() {}
        }
        let mut store = self.store.lock().await;
        for installed in store.integrations.values_mut() {
            for run in &mut installed.runs {
                if run.status == "running" {
                    run.status = "interrupted".into();
                    run.error = Some("host_stopped".into());
                    run.finished_at = Some(now());
                }
            }
        }
        let _ = persist(&self.config.state_dir, &store);
    }
}
#[derive(Deserialize)]
struct SessionLogin {
    token: String,
}
async fn session(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    Form(login): Form<SessionLogin>,
) -> std::result::Result<(StatusCode, HeaderMap), ApiError> {
    // A private local launcher submits the token in the body, never in a URL.
    if headers.get("host").and_then(|v| v.to_str().ok())
        != Some(host.config.listen.to_string().as_str())
        || !bool::from(login.token.as_bytes().ct_eq(host.token.expose_secret().as_bytes()))
    {
        return Err(error(StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    let mut response = HeaderMap::new();
    response.insert(
        "set-cookie",
        format!(
            "ipe_monitor={}; HttpOnly; SameSite=Strict; Path=/v1/; Max-Age=28800",
            host.monitor_cookie()
        )
        .parse()
        .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "session_failed"))?,
    );
    response.insert("location", axum::http::HeaderValue::from_static("/"));
    response.insert("cache-control", axum::http::HeaderValue::from_static("no-store"));
    response.insert("referrer-policy", axum::http::HeaderValue::from_static("no-referrer"));
    Ok((StatusCode::SEE_OTHER, response))
}
async fn installation(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    Ok(Json(
        json!({"id":id,"adapter_digest":installed.digest,"package":installed.package}),
    ))
}
async fn record_schema(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    Ok(Json(
        json!({"schema_version":model::VERSION,"model":rmcp::schemars::schema_for!(model::Model),"validation":"Native validation also checks identity consistency, claim authority, grants, receipts, duplicate keys, and bounded work"}),
    ))
}
async fn capabilities(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    Ok(Json(
        json!({"host":"local_installations_v1","sdk_version":crate::script::SDK_VERSION,"operations":host.broker.operations.values().map(|op|json!({"name":op.name,"parameters":op.parameters,"decoder":op.decoder})).collect::<Vec<_>>(),"output_kinds":model::KINDS,"record_schema_version":model::VERSION,"history_runs":HISTORY,"durable":true,"publication":false}),
    ))
}
async fn install(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
    bytes: axum::body::Bytes,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let package = ipe_json::parse(&bytes)
        .map_err(|_| error(StatusCode::BAD_REQUEST, "invalid_installation"))?;
    let digest = host.install(&id, package).await.map_err(|code| {
        error(
            if code == "integration_busy" { StatusCode::CONFLICT } else { StatusCode::BAD_REQUEST },
            code,
        )
    })?;
    Ok(Json(json!({"id":id,"adapter_digest":digest,"installed":true})))
}
async fn start(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<(StatusCode, Json<Value>), ApiError> {
    host.authorize(&headers)?;
    let run = host
        .start(&id, "manual")
        .await
        .map_err(|code| error(StatusCode::CONFLICT, code))?;
    Ok((StatusCode::ACCEPTED, Json(json!({"run_id":run}))))
}
async fn list(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    Ok(Json(
        json!({"integrations":store.integrations.iter().map(|(id,i)|json!({"id":id,"adapter_digest":i.digest,"enabled":i.package.enabled,"interval_seconds":i.package.interval_seconds,"next_due":i.next_due,"current_records":i.current.len(),"expired_facts":i.current.iter().filter(|r|r.expires_at.is_some_and(|t|t<=now())).count(),"latest_run":i.runs.last().map(|r|json!({"id":r.id,"status":r.status,"elapsed_ms":r.elapsed_ms,"error":r.error,"metrics":r.metrics,"steps":r.steps}))})).collect::<Vec<_>>()}),
    ))
}
async fn history(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    Ok(Json(
        json!({"integration_id":id,"runs":installed.runs,"external_outcomes":installed.outcomes.executions.values().map(|e| &e.latest).collect::<Vec<_>>(),"retention_runs":HISTORY,"outcome_retention":"independent archive; export via /outcomes","legacy_untyped_outcomes":installed.outcomes.legacy_untyped.len(),"outcome_capacity":outcomes::EXECUTION_LIMIT,"deduplication":"one execution per outcome key; changed payloads retained separately"}),
    ))
}
async fn outcome_archive(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    Ok(Json(
        json!({"schema":"ipe.integration.outcome-archive.v1","integration_id":id,"executions":installed.outcomes.executions,"legacy_untyped":installed.outcomes.legacy_untyped,"capacity":outcomes::EXECUTION_LIMIT,"overflow":"reject new capture atomically; existing executions remain available for export"}),
    ))
}
async fn operation_explanations(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    Ok(Json(explanations::project(installed, now())))
}
async fn review_catalog(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    Ok(Json(
        json!({"schema":"ipe.integration.review-catalog.v1","accepted":installed.revisions.accepted,
        "records":installed.revisions.catalog(&installed.current).iter().filter(|r| !["fact","outcome"].contains(&r.kind.as_str())).collect::<Vec<_>>(),
        "facts":installed.current.iter().filter(|r|r.kind=="fact").map(|r|json!({"record":r,"expired":r.expires_at.is_none_or(|t|t<=now())})).collect::<Vec<_>>(),
        "publication":false}),
    ))
}
async fn revision_list(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    let archive = &installed.revisions;
    let accepted = archive.accepted.as_ref().and_then(|id| archive.get(id).ok());
    let operations = installed
        .package
        .operations
        .iter()
        .filter_map(|name| host.broker.operations.get(name))
        .collect::<Vec<_>>();
    let native_digest = broker::digest(
        &serde_json::to_vec(&operations)
            .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "revision_invalid"))?,
    );
    Ok(Json(
        json!({"schema":"ipe.integration.revisions.v1","candidate":archive.candidate,"accepted":archive.accepted,
        "revisions":archive.revisions.iter().map(|r|json!({"id":r.id,"created_at":r.created_at,"run_id":r.run_id,"records":r.records.len(),"accepted_at":archive.accepted_at.get(&r.id)})).collect::<Vec<_>>(),
        "diff":revisions::diff(accepted.map_or(&[],|r|r.records.as_slice()),&installed.current),
        "collection_context_changed":accepted.is_some_and(|r| serde_json::to_value(&r.package).ok()!=serde_json::to_value(&installed.package).ok() || r.sdk_version!=crate::script::SDK_VERSION || r.native_operations_digest!=native_digest),
        "capacity":revisions::LIMIT,"publication":false,"scope":"Acceptance selects a local review catalog. It does not activate policy, change collector code, or renew facts."}),
    ))
}
async fn revision_detail(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath((id, revision)): ApiPath<(String, String)>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    let revision = installed
        .revisions
        .get(&revision)
        .map_err(|e| error(StatusCode::NOT_FOUND, e))?;
    Ok(Json(
        json!({"schema":"ipe.integration.revision.v1","revision":revision,"publication":false}),
    ))
}
async fn select_revision(
    host: Arc<Host>,
    headers: HeaderMap,
    id: String,
    revision: String,
    restore: bool,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let mut store = host.store.lock().await;
    let prior = store.clone();
    let installed = store
        .integrations
        .get_mut(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    if installed.runs.iter().any(|r| r.status == "running") {
        return Err(error(StatusCode::CONFLICT, "integration_busy"));
    }
    installed
        .revisions
        .select(&revision, restore, now())
        .map_err(|e| error(StatusCode::BAD_REQUEST, e))?;
    if let Err(code) = persist(&host.config.state_dir, &store) {
        *store = prior;
        return Err(error(StatusCode::INTERNAL_SERVER_ERROR, code));
    }
    Ok(Json(json!({"accepted":revision,"restored":restore,"publication":false})))
}
async fn revision_accept(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath((id, revision)): ApiPath<(String, String)>,
) -> std::result::Result<Json<Value>, ApiError> {
    select_revision(host, headers, id, revision, false).await
}
async fn revision_restore(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath((id, revision)): ApiPath<(String, String)>,
) -> std::result::Result<Json<Value>, ApiError> {
    select_revision(host, headers, id, revision, true).await
}
async fn collection_coverage(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    let latest = installed.runs.last();
    Ok(Json(json!({
        "schema":"ipe.integration.collection-coverage.v1", "integration_id":id,
        "latest_run":latest.map(|r| json!({"id":r.id,"status":r.status,"error":r.error,"reads":r.evidence,"complete_declaration_scopes":r.coverage})),
        "retained_constraints":installed.current.iter().filter(|r|r.kind=="constraint").map(|r|json!({"key":r.key,"scope":r.model.as_ref().map(|m| &m.scope),"observed_at":r.observed_at,"run_id":r.run_id,"observed_in_latest_run":latest.is_some_and(|run|run.status=="succeeded" && run.id==r.run_id)})).collect::<Vec<_>>(),
        "semantics":"Complete declaration scopes authorize absence removal only for that source and kind. They do not prove effective external policy coverage. Other captures are upsert-only.",
        "publication":false
    })))
}
async fn records(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<String>,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    let store = host.store.lock().await;
    let installed = store
        .integrations
        .get(&id)
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "integration_unavailable"))?;
    Ok(Json(
        json!({"integration_id":id,"as_of":now(),"records":installed.current.iter().map(|r|json!({"record":r,"expired":r.expires_at.is_some_and(|t|t<=now())})).collect::<Vec<_>>(),"publication":false}),
    ))
}
pub async fn serve(config: Config) -> Result<()> {
    let address = config.listen;
    let host = Host::new(config).await?;
    let listener = tokio::net::TcpListener::bind(address).await.map_err(|_| "listen_failed")?;
    println!(
        "integration_host_listening {}",
        listener.local_addr().map_err(|_| "listen_failed")?
    );
    let scheduler = tokio::spawn(host.clone().scheduler());
    let shutdown = host.clone();
    let result = axum::serve(listener, host.router())
        .with_graceful_shutdown(async move {
            let _ = crate::shutdown_signal().await;
            shutdown.stop().await;
        })
        .await;
    host.stop().await;
    let _ = scheduler.await;
    result.map_err(|_| "serve_failed")
}

#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod tests;
