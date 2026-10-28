//! A local installation host: code, scheduling, brokered reads, typed records,
//! durable bounded history, and authenticated monitoring share one endpoint.
mod broker;
mod credentials;
use crate::script::ScriptAdapter;
use axum::{
    extract::{DefaultBodyLimit, Path as ApiPath, State},
    http::{HeaderMap, StatusCode},
    response::Html,
    routing::{get, post, put},
    Json, Router,
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
}
#[derive(Clone, Serialize, Deserialize)]
struct Installed {
    package: Installation,
    digest: String,
    next_due: u64,
    current: Vec<Record>,
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
            .any(|k| !["contract", "fact", "outcome"].contains(&k.as_str()))
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
            .route("/v1/host/capabilities", get(capabilities))
            .route("/v1/integrations", get(list))
            .route("/v1/integrations/:id", put(install))
            .route("/v1/integrations/:id/runs", post(start).get(history))
            .route("/v1/integrations/:id/records", get(records))
            .layer(DefaultBodyLimit::max(160 * 1024))
            .with_state(self.clone())
    }
    fn authorize(&self, headers: &HeaderMap) -> std::result::Result<(), ApiError> {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .unwrap_or("");
        if !bool::from(provided.as_bytes().ct_eq(self.token.expose_secret().as_bytes())) {
            return Err(error(StatusCode::UNAUTHORIZED, "unauthorized"));
        }
        Ok(())
    }
    async fn install(&self, id: &str, package: Installation) -> Result<String> {
        if !broker::name(id) {
            return Err("invalid_integration_id");
        }
        validate(&package, &self.broker)?;
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
        let mut evidence = Vec::new();
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
                    kind: String,
                    key: String,
                    data: Value,
                    receipts: Vec<String>,
                }
                let record: Wire =
                    serde_json::from_value(observation).map_err(|_| "invalid_observation")?;
                if !package.output_kinds.contains(&record.kind)
                    || record.key.is_empty()
                    || record.key.len() > 512
                    || record.receipts.is_empty()
                    || record.receipts.len() > 8
                    || record.receipts.iter().any(|r| !receipts.contains(r))
                {
                    return Err("observation_not_permitted");
                }
                if observations.len() >= 256 {
                    return Err("observation_limit");
                }
                output_bytes +=
                    serde_json::to_vec(&record.data).map_err(|_| "invalid_observation")?.len();
                if output_bytes > MAX_RUN_BYTES {
                    return Err("observation_byte_limit");
                }
                observations.push(Record {
                    kind: record.kind.clone(),
                    key: record.key,
                    data: record.data,
                    source_evidence: evidence
                        .iter()
                        .filter(|e: &&Value| record.receipts.iter().any(|r| e["receipt"] == *r))
                        .cloned()
                        .collect(),
                    receipts: record.receipts,
                    observed_at: now(),
                    expires_at: (record.kind == "fact").then(|| now() + package.fact_ttl_seconds),
                    run_id,
                    adapter_digest: adapter.digest.clone(),
                });
            }
            if let Some(output) = result.outputs {
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
                        evidence.push(json!({"receipt":receipt,"operation":request.operation,"parameters":request.params,"source_digest":data["source_digest"],"observed_at":now(),"truncated":data["truncated"]}));
                        receipts.insert(receipt.clone());
                        next.push(json!({"id":request.id,"operation":request.operation,"receipt":receipt,"data":data["records"],"source_digest":data["source_digest"],"truncated":data["truncated"]}));
                    },
                    Err(code) => next
                        .push(json!({"id":request.id,"operation":request.operation,"code":code})),
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
            if let Ok((_, records, _)) = &terminal {
                let keys: HashSet<_> = installed
                    .current
                    .iter()
                    .chain(records.iter().filter(|r| r.kind != "outcome"))
                    .map(|r| (&r.kind, &r.key))
                    .collect();
                if keys.len() > 256 {
                    terminal = Err("current_record_limit");
                }
            }
            if let Ok((_, records, _)) = &terminal {
                for record in records.iter().filter(|r| r.kind != "outcome") {
                    installed.current.retain(|r| r.kind != record.kind || r.key != record.key);
                    installed.current.push(record.clone());
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
async fn capabilities(
    State(host): State<Arc<Host>>,
    headers: HeaderMap,
) -> std::result::Result<Json<Value>, ApiError> {
    host.authorize(&headers)?;
    Ok(Json(
        json!({"host":"local_installations_v1","sdk_version":crate::script::SDK_VERSION,"operations":host.broker.operations.values().map(|op|json!({"name":op.name,"parameters":op.parameters,"decoder":op.decoder})).collect::<Vec<_>>(),"output_kinds":["contract","fact","outcome"],"history_runs":HISTORY,"durable":true,"publication":false}),
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
    let mut outcomes = BTreeMap::new();
    for run in &installed.runs {
        for record in run.records.iter().filter(|r| r.kind == "outcome") {
            outcomes.insert(record.key.clone(), record);
        }
    }
    Ok(Json(
        json!({"integration_id":id,"runs":installed.runs,"external_outcomes":outcomes.values().collect::<Vec<_>>(),"retention_runs":HISTORY,"deduplication":"latest record per adapter-defined outcome key; retained runs only"}),
    ))
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
mod tests;
