//! Version-2 runtime. No route invokes legacy compilation or tiered execution.
use axum::{
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use ipe_core::policy_document::{DocumentError, ExecutionStatus, PolicyCatalog, PolicyDocument};
use ipe_core::runtime_facts::{
    CollectionConfig, FactError, FactMutation, FactStore, IngestionMode, Schema, Scope,
    SnapshotInput,
};
use ipe_core::runtime_policy::{CompileEnvironment, CompiledPolicy, EvalBudget, PolicyOutcome};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;

const MAX_BODY: usize = 1_048_576;
type ApiResult = Result<Json<Value>, ApiError>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    database_path: String,
    #[serde(default = "default_bind")]
    bind: SocketAddr,
    input_schema: Schema,
    credentials: Vec<CredentialConfig>,
    collections: Vec<ConfiguredCollection>,
    policies: Vec<ConfiguredPolicy>,
    #[serde(default)]
    connectors: Vec<ConnectorConfig>,
}
fn default_bind() -> SocketAddr {
    "127.0.0.1:9001".parse().unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialConfig {
    token_env: String,
    scope: Scope,
    #[serde(default)]
    source_id: Option<String>,
    #[serde(default)]
    write_collections: Vec<String>,
    #[serde(default)]
    evaluate: bool,
    #[serde(default)]
    inspect: bool,
    #[serde(default)]
    manage_policies: bool,
}
struct Credential {
    config: CredentialConfig,
    token: SecretString,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredCollection {
    scope: Scope,
    name: String,
    config: CollectionConfig,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfiguredPolicy {
    scope: Scope,
    source: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConnectorConfig {
    scope: Scope,
    collection: String,
    source_id: String,
    url: String,
    #[serde(default)]
    token_env: Option<String>,
    #[serde(default)]
    ca_certificate: Option<String>,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
    max_bytes: usize,
}
struct Connector {
    config: ConnectorConfig,
    client: reqwest::Client,
    token: Option<SecretString>,
    gate: tokio::sync::Mutex<()>,
    status: Mutex<Value>,
}
struct Clock {
    wall: i64,
    started: Instant,
    high_water: Mutex<i64>,
}
impl Clock {
    fn new() -> Self {
        let wall = chrono::Utc::now().timestamp();
        Self {
            wall,
            started: Instant::now(),
            high_water: Mutex::new(wall),
        }
    }
    fn now(&self) -> Result<i64, ApiError> {
        let anchored = self.wall.saturating_add(self.started.elapsed().as_secs() as i64);
        let wall = chrono::Utc::now().timestamp();
        if (wall as i128 - anchored as i128).abs() > 5 {
            return Err(ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "clock_untrusted",
                "clock changed; restore trusted time and restart".into(),
            ));
        }
        let mut high = self.high_water.lock().unwrap();
        *high = (*high).max(wall).max(anchored);
        Ok(*high)
    }
}
#[derive(Default)]
struct WorkerLifecycle {
    cancellation: CancellationToken,
    // Serialize the cancellation boundary with synchronous snapshot publication.
    publication: Mutex<()>,
}
impl WorkerLifecycle {
    // An in-progress synchronous commit completes before this fence. Storage I/O
    // is not preemptible; shutdown deadlines cover cooperative async work only.
    fn cancel(&self) {
        let _gate = self.publication.lock().unwrap();
        self.cancellation.cancel();
    }
}
struct AppState {
    workers: WorkerLifecycle,
    store: FactStore,
    input_schema: Schema,
    credentials: Vec<Credential>,
    policies: BTreeMap<Scope, Vec<CompiledPolicy>>,
    policy_catalog: PolicyCatalog,
    compile_environments: BTreeMap<Scope, CompileEnvironment>,
    connectors: Vec<Arc<Connector>>,
    clock: Clock,
}
#[derive(Debug)]
struct ApiError(StatusCode, &'static str, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({"error":self.1,"message":self.2}))).into_response()
    }
}
impl From<FactError> for ApiError {
    fn from(e: FactError) -> Self {
        let (status, code) = match &e {
            FactError::UnauthorizedSource => (StatusCode::FORBIDDEN, "source_forbidden"),
            FactError::UnknownCollection => (StatusCode::NOT_FOUND, "unknown_collection"),
            FactError::RevisionConflict | FactError::Replay => {
                (StatusCode::CONFLICT, "revision_conflict")
            },
            FactError::Storage(_) | FactError::CorruptState(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "storage_unavailable")
            },
            FactError::StateLimit => (StatusCode::INSUFFICIENT_STORAGE, "state_limit"),
            _ => (StatusCode::UNPROCESSABLE_ENTITY, "invalid_fact"),
        };
        let message = if status == StatusCode::SERVICE_UNAVAILABLE {
            warn!(error = %e, "runtime storage failure");
            "fact storage unavailable".to_string()
        } else {
            e.to_string()
        };
        Self(status, code, message)
    }
}
fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, ApiError> {
    crate::strict_json::parse(bytes)
        .map_err(|e| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "invalid_json", e.to_string()))
}
fn authorize<'a>(
    state: &'a AppState,
    headers: &HeaderMap,
    scope: &Scope,
) -> Result<&'a CredentialConfig, ApiError> {
    let supplied = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| {
            ApiError(
                StatusCode::UNAUTHORIZED,
                "unauthenticated",
                "Bearer credential required".into(),
            )
        })?;
    let mut recognized = false;
    let mut matched = None;
    for credential in &state.credentials {
        let equal =
            bool::from(credential.token.expose_secret().as_bytes().ct_eq(supplied.as_bytes()));
        recognized |= equal;
        if equal && credential.config.scope == *scope {
            matched = Some(&credential.config);
        }
    }
    matched.ok_or_else(|| {
        if recognized {
            ApiError(
                StatusCode::FORBIDDEN,
                "scope_forbidden",
                "credential is not authorized for this scope".into(),
            )
        } else {
            ApiError(StatusCode::UNAUTHORIZED, "unauthenticated", "invalid credential".into())
        }
    })
}
fn forbidden() -> ApiError {
    ApiError(StatusCode::FORBIDDEN, "operation_forbidden", "operation not granted".into())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommitRequest {
    mutations: Vec<Mutation>,
}
#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum Mutation {
    Put {
        key: String,
        value: Value,
        expected_revision: Option<u64>,
        source_revision: u64,
        observed_at: i64,
        ttl_seconds: u64,
    },
    Delete {
        key: String,
        expected_revision: u64,
        source_revision: u64,
        observed_at: i64,
    },
}
impl Mutation {
    fn into_fact(self) -> FactMutation {
        match self {
            Self::Put {
                key,
                value,
                expected_revision,
                source_revision,
                observed_at,
                ttl_seconds,
            } => FactMutation {
                key,
                value: Some(value),
                expected_revision,
                source_revision,
                observed_at,
                ttl_seconds: Some(ttl_seconds),
            },
            Self::Delete {
                key,
                expected_revision,
                source_revision,
                observed_at,
            } => FactMutation {
                key,
                value: None,
                expected_revision: Some(expected_revision),
                source_revision,
                observed_at,
                ttl_seconds: None,
            },
        }
    }
}
async fn commit(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment, collection)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    let credential = authorize(&state, &headers, &scope)?;
    if !credential.write_collections.contains(&collection) {
        return Err(forbidden());
    }
    let source = credential.source_id.as_deref().ok_or_else(forbidden)?;
    let request: CommitRequest = decode(&body)?;
    let records = state.store.commit(
        &scope,
        &collection,
        source,
        request.mutations.into_iter().map(Mutation::into_fact).collect(),
        state.clock.now()?,
    )?;
    info!(tenant=%scope.tenant,environment=%scope.environment,collection,source,count=records.len(),"fact batch committed");
    Ok(Json(json!({"records":records})))
}
async fn inspect_fact(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment, collection, key)): Path<(String, String, String, String)>,
    headers: HeaderMap,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    let credential = authorize(&state, &headers, &scope)?;
    if !credential.inspect && !credential.write_collections.contains(&collection) {
        return Err(forbidden());
    }
    let snapshot = state.store.snapshot(scope);
    snapshot.collection_config(&collection)?;
    let revision = snapshot.key_revision(&collection, &key);
    Ok(Json(match snapshot.get(&collection, &key, state.clock.now()?, None) {
        Ok(record) if credential.inspect => {
            json!({"status":"found","record":record,"revision":revision})
        },
        Ok(_) => json!({"status":"found","revision":revision}),
        Err(e) => json!({"status":e.to_string(),"revision":revision}),
    }))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluateRequest {
    input: Value,
}
async fn evaluate(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    if !authorize(&state, &headers, &scope)?.evaluate {
        return Err(forbidden());
    }
    let request: EvaluateRequest = decode(&body)?;
    state.input_schema.validate_value(&request.input)?;
    let snapshot = state.store.snapshot(scope.clone());
    let now = state.clock.now()?;
    let policy_revisions = state.policy_catalog.active(&scope);
    let (mut allowed, mut denied, mut indeterminate) = (false, false, false);
    let mut earliest = None::<i64>;
    let mut results = Vec::new();
    for policy in state.policies.get(&scope).into_iter().flatten() {
        let result = policy.evaluate(&request.input, &snapshot, now, EvalBudget::default());
        if let Some(deadline) = result.evidence_deadline {
            earliest = Some(earliest.map_or(deadline, |old| old.min(deadline)));
        }
        let (status, reason) = match result.outcome {
            PolicyOutcome::Allow => {
                allowed = true;
                ("allow", None)
            },
            PolicyOutcome::Deny => {
                denied = true;
                ("deny", None)
            },
            PolicyOutcome::NotApplicable => ("not_applicable", None),
            PolicyOutcome::Indeterminate(reason) => {
                indeterminate = true;
                ("indeterminate", Some(format!("{reason:?}")))
            },
        };
        results.push(json!({"policy":policy.name,"status":status,"reason":reason}));
    }
    let compile_environment = compile_environment(&state, &scope);
    let mut documents = Vec::new();
    for revision in policy_revisions {
        let prepared = revision.prepare(&compile_environment).map_err(ApiError::from)?;
        let result = prepared.evaluate(&request.input, &snapshot, now, now);
        if let Some(deadline) = result.evidence_deadline {
            earliest = Some(earliest.map_or(deadline, |old| old.min(deadline)));
        }
        match result.status {
            ExecutionStatus::Allow => allowed = true,
            ExecutionStatus::Deny => denied = true,
            ExecutionStatus::Indeterminate => indeterminate = true,
            ExecutionStatus::NotApplicable => {},
        }
        documents.push(
            json!({"revision": revision.revision, "document_digest": revision.document_digest,
            "external": revision.document.external, "result": result}),
        );
    }
    let completed_at = state.clock.now()?;
    if earliest.is_some_and(|end| completed_at >= end) {
        indeterminate = true;
    }
    let decision = if allowed && !denied && !indeterminate { "allow" } else { "deny" };
    let status = if denied {
        "deny"
    } else if indeterminate {
        "indeterminate"
    } else {
        decision
    };
    Ok(Json(
        json!({"decision":decision,"evaluation_status":status,"evaluated_at":now,"completed_at":completed_at,"revision":snapshot.revision(),"evidence_deadline":earliest,"policies":results,"policy_documents":documents}),
    ))
}
async fn catalog(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    if !authorize(&state, &headers, &scope)?.inspect {
        return Err(forbidden());
    }
    Ok(Json(
        json!({"input_schema":state.input_schema,"collections":state.store.snapshot(scope.clone()).collections(),"policies":state.policies.get(&scope),"policy_documents":state.policy_catalog.active(&scope)}),
    ))
}
impl From<DocumentError> for ApiError {
    fn from(error: DocumentError) -> Self {
        let (status, code) = match &error {
            DocumentError::Conflict => (StatusCode::CONFLICT, "policy_revision_conflict"),
            DocumentError::Invalid(_) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "invalid_policy_document")
            },
            DocumentError::Capacity => (StatusCode::INSUFFICIENT_STORAGE, "policy_catalog_full"),
            DocumentError::Storage(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, "policy_storage_unavailable")
            },
        };
        let message = if matches!(error, DocumentError::Storage(_)) {
            warn!(%error, "policy catalog failure");
            "policy storage unavailable".into()
        } else {
            error.to_string()
        };
        Self(status, code, message)
    }
}
fn compile_environment(state: &AppState, scope: &Scope) -> CompileEnvironment {
    state
        .compile_environments
        .get(scope)
        .cloned()
        .unwrap_or_else(|| CompileEnvironment::new(state.input_schema.clone()))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PublishDocumentRequest {
    expected_revision: Option<u64>,
    document: PolicyDocument,
}
async fn publish_document(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment, id)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    let credential = authorize(&state, &headers, &scope)?;
    if !credential.manage_policies {
        return Err(forbidden());
    }
    let request: PublishDocumentRequest = decode(&body)?;
    if request.document.id != id {
        return Err(DocumentError::Invalid("document ID does not match path".into()).into());
    }
    let revision = state.policy_catalog.publish(
        scope.clone(),
        request.document,
        request.expected_revision,
        credential.token_env.clone(),
        state.clock.now()?,
        &compile_environment(&state, &scope),
    )?;
    Ok(Json(json!(revision)))
}
async fn inspect_document(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment, id, revision)): Path<(String, String, String, u64)>,
    headers: HeaderMap,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    if !authorize(&state, &headers, &scope)?.inspect {
        return Err(forbidden());
    }
    let revision = state.policy_catalog.revision(&scope, &id, revision).ok_or_else(|| {
        ApiError(
            StatusCode::NOT_FOUND,
            "policy_revision_missing",
            "policy revision not found".into(),
        )
    })?;
    Ok(Json(json!(revision)))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TestDocumentRequest {
    document: PolicyDocument,
    input: Value,
}
async fn test_document(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    let credential = authorize(&state, &headers, &scope)?;
    // Arbitrary candidate policies can reveal facts, so ordinary evaluate is insufficient.
    if !credential.inspect || !credential.evaluate {
        return Err(forbidden());
    }
    let request: TestDocumentRequest = decode(&body)?;
    state.input_schema.validate_value(&request.input)?;
    let prepared = request.document.prepare(&compile_environment(&state, &scope))?;
    let snapshot = state.store.snapshot(scope);
    let now = state.clock.now()?;
    let result = prepared.evaluate(&request.input, &snapshot, now, now);
    let completed_at = state.clock.now()?;
    let expired = result.evidence_deadline.is_some_and(|end| completed_at >= end);
    let status = if expired { ExecutionStatus::Indeterminate } else { result.status.clone() };
    Ok(Json(
        json!({"simulation":true,"decision":if status == ExecutionStatus::Allow {"allow"} else {"deny"},
        "evaluation_status":status,"document_digest":request.document.digest(),"runtime_revision":snapshot.revision(),
        "evaluated_at":now,"completed_at":completed_at,"result":result}),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PullEnvelope {
    source_revision: u64,
    observed_at: i64,
    ttl_seconds: u64,
    items: Vec<PullItem>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PullItem {
    key: String,
    value: Value,
}
async fn poll_connector(state: &AppState, connector: &Connector) -> Result<Value, String> {
    let _gate = connector.gate.try_lock().map_err(|_| "poll already in progress".to_string())?;
    let fetch = async {
        let mut request = connector.client.get(&connector.config.url);
        if let Some(token) = &connector.token {
            request = request.bearer_auth(token.expose_secret());
        }
        let mut response =
            request.send().await.map_err(|_| "upstream request failed".to_string())?;
        if !response.status().is_success() {
            return Err(format!("upstream returned {}", response.status()));
        }
        if response.content_length().is_some_and(|n| n > connector.config.max_bytes as u64) {
            return Err("upstream body exceeds limit".into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) =
            response.chunk().await.map_err(|_| "upstream body failed".to_string())?
        {
            if chunk.len() > connector.config.max_bytes.saturating_sub(bytes.len()) {
                return Err("upstream body exceeds limit".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        let envelope: PullEnvelope =
            crate::strict_json::parse(&bytes).map_err(|e| format!("invalid snapshot: {e}"))?;
        let mut items = BTreeMap::new();
        for item in envelope.items {
            if items.insert(item.key, item.value).is_some() {
                return Err("duplicate snapshot key".into());
            }
        }
        let _publication = state.workers.publication.lock().unwrap();
        if state.workers.cancellation.is_cancelled() {
            return Err("connector canceled".into());
        }
        let records = state
            .store
            .replace_snapshot(
                &connector.config.scope,
                &connector.config.collection,
                &connector.config.source_id,
                SnapshotInput {
                    source_revision: envelope.source_revision,
                    observed_at: envelope.observed_at,
                    ttl_seconds: envelope.ttl_seconds,
                    items,
                },
                state.clock.now().map_err(|e| e.2)?,
            )
            .map_err(|e| e.to_string())?;
        Ok(
            json!({"status":"ok","source_revision":envelope.source_revision,"items":records.len(),"checked_at":chrono::Utc::now().timestamp()}),
        )
    };
    let result = tokio::select! {
        biased;
        _ = state.workers.cancellation.cancelled() => Err("connector canceled".into()),
        result = tokio::time::timeout(Duration::from_secs(connector.config.timeout_seconds), fetch) => {
            result.unwrap_or_else(|_| Err("upstream deadline exceeded".into()))
        }
    };
    *connector.status.lock().unwrap() = match &result {
        Ok(value) => value.clone(),
        Err(message) => {
            json!({"status":"error","message":message,"checked_at":chrono::Utc::now().timestamp()})
        },
    };
    result
}
async fn connector_status(
    State(state): State<Arc<AppState>>,
    Path((tenant, environment)): Path<(String, String)>,
    headers: HeaderMap,
) -> ApiResult {
    let scope = Scope::new(tenant, environment);
    if !authorize(&state, &headers, &scope)?.inspect {
        return Err(forbidden());
    }
    let statuses: Vec<_> = state
        .connectors
        .iter()
        .filter(|c| c.config.scope == scope)
        .map(|c| json!({"collection":c.config.collection,"state":c.status.lock().unwrap().clone()}))
        .collect();
    Ok(Json(json!({"connectors":statuses})))
}
async fn ready(State(state): State<Arc<AppState>>) -> ApiResult {
    state.clock.now()?;
    Ok(Json(json!({"status":"ready"})))
}

fn load(config: Config) -> Result<(SocketAddr, Arc<AppState>), Box<dyn std::error::Error>> {
    if !config.bind.ip().is_loopback() {
        return Err(
            "runtime API must bind to loopback; use a local TLS proxy for remote access".into()
        );
    }
    let store = FactStore::open(&config.database_path)?;
    let policy_catalog =
        PolicyCatalog::open(std::path::Path::new(&config.database_path).join("policy-catalog"))?;
    let mut seen = BTreeSet::new();
    let mut catalog: BTreeMap<Scope, BTreeMap<String, CollectionConfig>> = BTreeMap::new();
    for entry in config.collections {
        if !seen.insert((entry.scope.clone(), entry.name.clone())) {
            return Err("duplicate scoped collection configuration".into());
        }
        catalog
            .entry(entry.scope.clone())
            .or_default()
            .insert(entry.name.clone(), entry.config.clone());
        store.register_collection(entry.scope, entry.name, entry.config)?;
    }
    let compile_environments = catalog
        .iter()
        .map(|(scope, collections)| {
            let mut environment = CompileEnvironment::new(config.input_schema.clone());
            for (name, collection) in collections {
                environment = environment.collection(name.clone(), collection.schema.clone());
            }
            (scope.clone(), environment)
        })
        .collect();
    let mut policies: BTreeMap<Scope, Vec<CompiledPolicy>> = BTreeMap::new();
    for entry in config.policies {
        let mut environment = CompileEnvironment::new(config.input_schema.clone());
        for (name, collection) in catalog.get(&entry.scope).into_iter().flatten() {
            environment = environment.collection(name.clone(), collection.schema.clone());
        }
        let policy = CompiledPolicy::compile(&entry.source, &environment)?;
        let group = policies.entry(entry.scope).or_default();
        if group.len() >= 100 || group.iter().any(|p| p.name == policy.name) {
            return Err("too many policies or duplicate policy name in scope".into());
        }
        group.push(policy);
    }
    for revision in policy_catalog.all() {
        let mut environment = CompileEnvironment::new(config.input_schema.clone());
        for (name, collection) in catalog.get(&revision.scope).into_iter().flatten() {
            environment = environment.collection(name.clone(), collection.schema.clone());
        }
        revision.prepare(&environment)?;
    }
    if policies.is_empty()
        && policy_catalog.all().is_empty()
        && !config.credentials.iter().any(|c| c.manage_policies)
    {
        return Err("an empty catalog requires a policy management credential".into());
    }
    let mut credentials = Vec::new();
    for entry in config.credentials {
        let token = SecretString::from(std::env::var(&entry.token_env).map_err(|_| {
            format!("credential environment variable {} is not set", entry.token_env)
        })?);
        if token.expose_secret().len() < 32 {
            return Err("API credentials must contain at least 32 bytes".into());
        }
        if credentials.iter().any(|c: &Credential| {
            c.token.expose_secret() == token.expose_secret() && c.config.scope == entry.scope
        }) {
            return Err("duplicate credential in scope; combine grants".into());
        }
        for collection in &entry.write_collections {
            if !seen.contains(&(entry.scope.clone(), collection.clone())) {
                return Err(
                    "writer references a collection absent from bootstrap configuration".into()
                );
            }
            let owner = store.collection_config(&entry.scope, collection)?;
            if owner.mode != IngestionMode::Push
                || entry.source_id.as_deref() != Some(&owner.source_id)
            {
                return Err("writer grant does not match push collection owner".into());
            }
        }
        credentials.push(Credential { config: entry, token });
    }
    if credentials.is_empty() {
        return Err("at least one credential is required".into());
    }
    let mut connectors = Vec::new();
    let mut destinations = BTreeSet::new();
    for entry in config.connectors {
        if !destinations.insert((entry.scope.clone(), entry.collection.clone())) {
            return Err("duplicate connector destination".into());
        }
        if !seen.contains(&(entry.scope.clone(), entry.collection.clone())) {
            return Err(
                "connector references a collection absent from bootstrap configuration".into()
            );
        }
        let owner = store.collection_config(&entry.scope, &entry.collection)?;
        if owner.mode != IngestionMode::Pull || owner.source_id != entry.source_id {
            return Err("connector does not own pull collection".into());
        }
        if entry.poll_interval_seconds == 0
            || entry.timeout_seconds == 0
            || entry.timeout_seconds > 60
            || entry.max_bytes == 0
            || entry.max_bytes > MAX_BODY
        {
            return Err("invalid connector limits".into());
        }
        let url = reqwest::Url::parse(&entry.url)?;
        if url.scheme() != "https"
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            return Err(
                "connector requires a fixed HTTPS URL without credentials or fragment".into()
            );
        }
        let mut client = reqwest::Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(entry.timeout_seconds));
        if let Some(path) = &entry.ca_certificate {
            client =
                client.add_root_certificate(reqwest::Certificate::from_pem(&std::fs::read(path)?)?);
        }
        let token = entry
            .token_env
            .as_ref()
            .map(|name| {
                std::env::var(name)
                    .map(SecretString::from)
                    .map_err(|_| format!("connector credential {name} is not set"))
            })
            .transpose()?;
        connectors.push(Arc::new(Connector {
            config: entry,
            client: client.build()?,
            token,
            gate: tokio::sync::Mutex::new(()),
            status: Mutex::new(json!({"status":"pending"})),
        }));
    }
    Ok((
        config.bind,
        Arc::new(AppState {
            workers: WorkerLifecycle::default(),
            store,
            input_schema: config.input_schema,
            credentials,
            policies,
            policy_catalog,
            compile_environments,
            connectors,
            clock: Clock::new(),
        }),
    ))
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    let path = std::env::var("IPE_RUNTIME_CONFIG")
        .map_err(|_| "set IPE_RUNTIME_CONFIG to a runtime bootstrap JSON file")?;
    let bytes = std::fs::read(path)?;
    if bytes.len() > 4 * MAX_BODY {
        return Err("bootstrap configuration exceeds 4 MiB".into());
    }
    let (bind, state) = load(crate::strict_json::parse(&bytes)?)?;
    // Binding must succeed before any background tasks can access the store.
    let (listener, mut workers) = bind_and_start_workers(bind, &state).await?;
    let app = Router::new()
        .route("/health", get(|| async { Json(json!({"status":"healthy"})) }))
        .route("/ready", get(ready))
        .route("/v1/scopes/:tenant/:environment/evaluate", post(evaluate))
        .route("/v1/scopes/:tenant/:environment/catalog", get(catalog))
        .route("/v1/scopes/:tenant/:environment/policies/test", post(test_document))
        .route("/v1/scopes/:tenant/:environment/policies/:id/revisions", post(publish_document))
        .route(
            "/v1/scopes/:tenant/:environment/policies/:id/revisions/:revision",
            get(inspect_document),
        )
        .route("/v1/scopes/:tenant/:environment/connectors", get(connector_status))
        .route("/v1/scopes/:tenant/:environment/facts/:collection/commit", post(commit))
        .route("/v1/scopes/:tenant/:environment/facts/:collection/:key", get(inspect_fact))
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(Arc::clone(&state));
    info!(address=%listener.local_addr()?,"runtime API listening");
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        let _ = stop_rx.await;
    });
    let server = std::future::IntoFuture::into_future(server);
    tokio::pin!(server);
    let result: Result<(), Box<dyn std::error::Error>> = tokio::select! {
        result = &mut server => result.map_err(Into::into),
        signal = shutdown_signal() => {
            // Fence publication before starting HTTP drain. Completed publications
            // precede this boundary; pending network responses cannot commit afterward.
            state.workers.cancel();
            let _ = stop_tx.send(());
            match signal {
                Err(error) => Err(error.into()),
                Ok(()) => match tokio::time::timeout(Duration::from_secs(13), &mut server).await {
                    Ok(result) => result.map_err(Into::into),
                    Err(_) => Err("HTTP shutdown deadline exceeded".into()),
                },
            }
        }
    };
    state.workers.cancel();
    finish_workers(&mut workers).await?;
    result
}

async fn bind_and_start_workers(
    bind: SocketAddr,
    state: &Arc<AppState>,
) -> Result<(TcpListener, JoinSet<()>), std::io::Error> {
    let listener = TcpListener::bind(bind).await?;
    Ok((listener, start_workers(state)))
}

fn start_workers(state: &Arc<AppState>) -> JoinSet<()> {
    let mut workers = JoinSet::new();
    let cleanup_state = Arc::clone(state);
    workers.spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = cleanup_state.workers.cancellation.cancelled() => break,
                _ = tokio::time::sleep(Duration::from_secs(30)) => {},
            }
            let _gate = cleanup_state.workers.publication.lock().unwrap();
            if cleanup_state.workers.cancellation.is_cancelled() {
                break;
            }
            if let Ok(now) = cleanup_state.clock.now() {
                if let Err(error) = cleanup_state.store.cleanup_expired(now) {
                    warn!(%error,"expired fact cleanup failed; resolver still enforces expiry");
                }
            }
        }
    });
    for connector in &state.connectors {
        let (state, connector) = (Arc::clone(state), Arc::clone(connector));
        workers.spawn(async move {
            loop {
                if state.workers.cancellation.is_cancelled() {
                    break;
                }
                if let Err(error) = poll_connector(&state, &connector).await {
                    if state.workers.cancellation.is_cancelled() {
                        break;
                    }
                    warn!(collection=%connector.config.collection,error,"refresh failed; expiry unchanged");
                }
                tokio::select! {
                    biased;
                    _ = state.workers.cancellation.cancelled() => break,
                    _ = tokio::time::sleep(Duration::from_secs(connector.config.poll_interval_seconds)) => {},
                }
            }
        });
    }
    workers
}

async fn finish_workers(workers: &mut JoinSet<()>) -> Result<(), &'static str> {
    if tokio::time::timeout(Duration::from_secs(2), async {
        while let Some(result) = workers.join_next().await {
            if let Err(error) = result {
                warn!(%error, "runtime worker stopped unexpectedly");
            }
        }
    })
    .await
    .is_err()
    {
        workers.abort_all();
        // Cancellation is cooperative: abort cannot preempt a synchronous RocksDB
        // call already executing. Bound the async join wait; do not claim a hard
        // process-exit deadline for blocked storage or OS calls.
        tokio::time::timeout(Duration::from_secs(2), async {
            while workers.join_next().await.is_some() {}
        })
        .await
        .map_err(|_| "worker shutdown deadline exceeded")?;
    }
    Ok(())
}

async fn shutdown_signal() -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}
