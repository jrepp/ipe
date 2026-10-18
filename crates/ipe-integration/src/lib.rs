//! Bounded native discovery. No adapter execution or enforcement publication.
use axum::{
    extract::{DefaultBodyLimit, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub listen: SocketAddr,
    api_token_env: String,
    connection_ref: String,
    origin: String,
    credential_env: String,
    ca_cert_path: Option<String>,
    resources: Vec<Resource>,
    identity_key_env: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Resource {
    resource_ref: String,
    table: String,
    fields: Vec<Field>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Field {
    field_ref: String,
    name: String,
}
struct Runtime {
    identity_key: Option<String>,
    config: Config,
    token: String,
    credential: String,
    client: reqwest::Client,
    origin: reqwest::Url,
    permit: Semaphore,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    connection_ref: String,
}

type ApiError = (StatusCode, Json<Value>);
fn error(status: StatusCode, code: &'static str) -> ApiError {
    (status, Json(json!({"code":code})))
}
fn identifier(s: &str) -> bool {
    !s.is_empty() && s.len() <= 128 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn application(config: Config) -> Result<Router, &'static str> {
    if !config.listen.ip().is_loopback() {
        return Err("listen_must_be_loopback");
    }
    let origin = reqwest::Url::parse(&config.origin).map_err(|_| "invalid_origin")?;
    if origin.scheme() != "https"
        || origin.host_str().is_none()
        || !origin.username().is_empty()
        || origin.password().is_some()
        || origin.path() != "/"
        || origin.query().is_some()
        || origin.fragment().is_some()
    {
        return Err("invalid_origin");
    }
    if !identifier(&config.connection_ref) || config.resources.len() > 8 {
        return Err("invalid_scope");
    }
    let mut refs = std::collections::HashSet::new();
    for r in &config.resources {
        if !identifier(&r.resource_ref)
            || !identifier(&r.table)
            || !refs.insert(r.resource_ref.clone())
            || r.fields.len() > 32
        {
            return Err("invalid_scope");
        }
        let mut names = std::collections::HashSet::new();
        for f in &r.fields {
            if !identifier(&f.field_ref)
                || !identifier(&f.name)
                || !refs.insert(f.field_ref.clone())
                || !names.insert(&f.name)
            {
                return Err("invalid_scope");
            }
        }
    }
    let identity_key = match &config.identity_key_env {
        Some(key) => Some(std::env::var(key).map_err(|_| "missing_identity_key")?),
        None => None,
    };
    if config.resources.is_empty() && !identity_key.as_ref().is_some_and(|k| k.len() >= 32) {
        return Err("catalog_requires_identity_key");
    }
    let token = std::env::var(&config.api_token_env).map_err(|_| "missing_api_token")?;
    let credential =
        std::env::var(&config.credential_env).map_err(|_| "missing_provider_credential")?;
    if token.len() < 32
        || credential.is_empty()
        || credential.len() > 8192
        || token.len() > 8192
        || token == credential
        || identity_key.as_ref().is_some_and(|key| {
            key.len() < 32 || key.len() > 8192 || key == &token || key == &credential
        })
    {
        return Err("invalid_credentials");
    }
    reqwest::header::HeaderValue::from_str(&format!("Bearer {credential}"))
        .map_err(|_| "invalid_credentials")?;
    let mut builder = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3));
    if let Some(path) = &config.ca_cert_path {
        let pem = std::fs::read(path).map_err(|_| "invalid_ca")?;
        let cert = reqwest::Certificate::from_pem(&pem).map_err(|_| "invalid_ca")?;
        builder = builder.add_root_certificate(cert);
    }
    let client = builder.build().map_err(|_| "client_configuration_failed")?;
    let state = Arc::new(Runtime {
        identity_key,
        config,
        token,
        credential,
        client,
        origin,
        permit: Semaphore::new(1),
    });
    Ok(Router::new()
        .route("/v1/capabilities", get(capabilities))
        .route("/v1/connections/check", post(check))
        .route("/v1/discovery", post(discover))
        .layer(DefaultBodyLimit::max(1024))
        .with_state(state)
        .layer(axum::middleware::from_fn(deadline)))
}
fn authorize(state: &Runtime, headers: &HeaderMap) -> Result<(), ApiError> {
    let presented = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");
    if !bool::from(presented.as_bytes().ct_eq(state.token.as_bytes())) {
        return Err(error(StatusCode::UNAUTHORIZED, "unauthorized"));
    }
    Ok(())
}
fn selection(
    state: &Runtime,
    body: Result<Json<Selection>, axum::extract::rejection::JsonRejection>,
) -> Result<(), ApiError> {
    let Json(body) = body.map_err(|_| error(StatusCode::BAD_REQUEST, "invalid_request"))?;
    if body.connection_ref != state.config.connection_ref {
        return Err(error(StatusCode::NOT_FOUND, "connection_unavailable"));
    }
    Ok(())
}
async fn deadline(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    use axum::response::IntoResponse;
    match tokio::time::timeout(Duration::from_secs(12), next.run(request)).await {
        Ok(response) => response,
        Err(_) => error(StatusCode::GATEWAY_TIMEOUT, "run_deadline").into_response(),
    }
}
async fn capabilities(
    State(state): State<Arc<Runtime>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    authorize(&state, &headers)?;
    Ok(Json(
        json!({"version":"v1","connection_refs":[state.config.connection_ref],"operations":["connections.check","discovery.run"],"publication":false,"limits":{"resources":8,"fields_per_resource":32,"response_bytes":65536,"request_timeout_seconds":5,"run_timeout_seconds":12,"concurrent_runs":1},"coverage":if state.config.resources.is_empty() {"bounded_catalog"} else {"operator_configured_candidates"}}),
    ))
}
async fn check(
    State(state): State<Arc<Runtime>>,
    headers: HeaderMap,
    body: Result<Json<Selection>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    authorize(&state, &headers)?;
    selection(&state, body)?;
    let _permit = state
        .permit
        .try_acquire()
        .map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "busy"))?;
    let query = state
        .config
        .resources
        .first()
        .map(|r| format!("name={}", r.table))
        .unwrap_or_else(|| "sys_idISNOTEMPTY".into());
    let result = read(&state, "sys_db_object", &query, "name", 2).await;
    let (status, code) = match result {
        Ok(_) => ("available", "ok"),
        Err(code) => ("unknown", code),
    };
    Ok(Json(
        json!({"connection_ref":state.config.connection_ref,"status":status,"code":code}),
    ))
}
async fn discover(
    State(state): State<Arc<Runtime>>,
    headers: HeaderMap,
    body: Result<Json<Selection>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    authorize(&state, &headers)?;
    selection(&state, body)?;
    let _permit = state
        .permit
        .try_acquire()
        .map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "busy"))?;
    if state.config.resources.is_empty() {
        return Ok(Json(catalog(&state).await));
    }
    let mut resources = Vec::new();
    for r in &state.config.resources {
        let table = read(&state, "sys_db_object", &format!("name={}", r.table), "name", 2).await;
        let table_code = match &table {
            Ok(rows)
                if rows.len() == 1
                    && rows
                        .iter()
                        .any(|row| row.get("name").and_then(Value::as_str) == Some(&r.table)) =>
            {
                "ok"
            },
            Ok(_) => "not_observed",
            Err(code) => code,
        };
        let dictionary = if table_code == "ok" && !r.fields.is_empty() {
            read(
                &state,
                "sys_dictionary",
                &format!(
                    "name={}^elementIN{}",
                    r.table,
                    r.fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(",")
                ),
                "element,internal_type,mandatory",
                33,
            )
            .await
        } else {
            Err("not_observed")
        };
        let fields: Vec<Value> = r.fields.iter().map(|f| {
            let matches: Vec<&Value> = dictionary.as_ref().ok().map(|rows| rows.iter().filter(|row| row.get("element").and_then(Value::as_str)==Some(&f.name)).collect()).unwrap_or_default();
            if matches.len() != 1 { return json!({"field_ref":f.field_ref,"status":"unknown","code":dictionary.as_ref().err().copied().unwrap_or("not_observed"),"kind":"unknown","required":null}); }
            let row = matches[0];
            let kind = match row.get("internal_type").and_then(Value::as_str) { Some("string") => "string",Some("integer") => "integer",Some("boolean") => "boolean",Some("reference") => "reference",_=>"unknown" };
            let required = match row.get("mandatory") { Some(Value::Bool(v))=>Some(*v),Some(Value::String(v)) if v=="true"=>Some(true),Some(Value::String(v)) if v=="false"=>Some(false),_=>None };
            json!({"field_ref":f.field_ref,"status":"observed","code":"ok","kind":kind,"required":required})
        }).collect();
        resources.push(json!({"resource_ref":r.resource_ref,"status":if table_code=="ok" {"observed"} else {"unknown"},"code":table_code,"fields":fields}));
    }
    Ok(Json(
        json!({"connection_ref":state.config.connection_ref,"coverage":"bounded","resources":resources,"constraints":{"status":"unknown","code":"workflow_semantics_not_discovered"}}),
    ))
}
fn opaque(state: &Runtime, domain: &str, value: &str) -> String {
    use hmac::{Hmac, Mac};
    let mut mac =
        Hmac::<sha2::Sha256>::new_from_slice(state.identity_key.as_ref().unwrap().as_bytes())
            .expect("HMAC accepts any key length");
    mac.update(state.config.connection_ref.as_bytes());
    mac.update(b"\0");
    mac.update(domain.as_bytes());
    mac.update(b"\0");
    mac.update(value.as_bytes());
    let digest = mac.finalize().into_bytes();
    format!("{}_{}", domain, digest.iter().map(|b| format!("{b:02x}")).collect::<String>())
}
async fn catalog(state: &Runtime) -> Value {
    let rows = read(state, "sys_db_object", "sys_idISNOTEMPTY", "name", 9).await;
    let mut resources = Vec::new();
    let mut code = "ok";
    if let Ok(rows) = rows.as_ref() {
        if rows.len() > 8 {
            code = "record_limit";
        }
        if rows.is_empty() {
            code = "not_observed";
        }
        let mut seen = std::collections::HashSet::new();
        for row in rows.iter().take(8) {
            let Some(table) = row.get("name").and_then(Value::as_str).filter(|s| identifier(s))
            else {
                code = "invalid_metadata";
                continue;
            };
            if !seen.insert(table) {
                code = "duplicate_metadata";
                continue;
            }
            let dictionary = read(
                state,
                "sys_dictionary",
                &format!("name={table}^elementISNOTEMPTY"),
                "element,internal_type,mandatory",
                33,
            )
            .await;
            let mut fields = Vec::new();
            let mut field_seen = std::collections::HashSet::new();
            let mut field_code = dictionary.as_ref().err().copied().unwrap_or("ok");
            if let Ok(entries) = dictionary.as_ref() {
                if entries.len() > 32 {
                    field_code = "record_limit";
                }
                if entries.is_empty() {
                    field_code = "not_observed";
                }
                for entry in entries.iter().take(32) {
                    let Some(name) =
                        entry.get("element").and_then(Value::as_str).filter(|s| identifier(s))
                    else {
                        field_code = "invalid_metadata";
                        continue;
                    };
                    if !field_seen.insert(name) {
                        field_code = "duplicate_metadata";
                        continue;
                    }
                    let role = match name {
                        "state" => "lifecycle_state",
                        "assigned_to" => "assignee",
                        "assignment_group" => "assignment_group",
                        "active" => "active",
                        "sys_updated_on" => "last_updated",
                        _ => "unknown",
                    };
                    let kind = match entry.get("internal_type").and_then(Value::as_str) {
                        Some("string") => "string",
                        Some("integer") => "integer",
                        Some("boolean") => "boolean",
                        Some("reference") => "reference",
                        _ => "unknown",
                    };
                    let required = match entry.get("mandatory") {
                        Some(Value::Bool(v)) => Some(*v),
                        Some(Value::String(v)) if v == "true" => Some(true),
                        Some(Value::String(v)) if v == "false" => Some(false),
                        _ => None,
                    };
                    if required.is_none() || kind == "unknown" {
                        field_code = "metadata_gap";
                    }
                    fields.push(json!({"field_ref":opaque(state,"field",&format!("{table}.{name}")),"status":"observed","kind":kind,"required":required,"role":role,"role_basis":"field_identifier_hint"}));
                }
            }
            resources.push(json!({"resource_ref":opaque(state,"resource",table),"status":"observed","code":"ok","fields":fields,"field_coverage":"bounded","field_code":field_code}));
        }
    } else {
        code = rows.as_ref().err().copied().unwrap_or("unknown");
    }
    json!({"connection_ref":state.config.connection_ref,"coverage":"bounded","code":code,"resources":resources,"constraints":{"status":"unknown","code":"workflow_semantics_not_discovered"}})
}
async fn read(
    state: &Runtime,
    table: &str,
    query: &str,
    fields: &str,
    limit: usize,
) -> Result<Vec<Value>, &'static str> {
    let mut url = state.origin.clone();
    url.set_path(&format!("/api/now/table/{table}"));
    url.query_pairs_mut()
        .append_pair("sysparm_query", query)
        .append_pair("sysparm_fields", fields)
        .append_pair("sysparm_limit", &limit.to_string())
        .append_pair("sysparm_display_value", "false")
        .append_pair("sysparm_exclude_reference_link", "true");
    let mut response = state
        .client
        .get(url)
        .bearer_auth(&state.credential)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|_| "transport_unavailable")?;
    match response.status().as_u16() {
        200 => (),
        401 | 403 => return Err("access_denied"),
        404 => return Err("metadata_unavailable"),
        429 => return Err("rate_limited"),
        300..=399 => return Err("redirect_blocked"),
        _ => return Err("provider_unavailable"),
    }
    if response.content_length().is_some_and(|n| n > 65536) {
        return Err("response_limit");
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "transport_unavailable")? {
        if bytes.len() + chunk.len() > 65536 {
            return Err("response_limit");
        }
        bytes.extend_from_slice(&chunk);
    }
    let body: Value = serde_json::from_slice(&bytes).map_err(|_| "invalid_response")?;
    let rows = body.get("result").and_then(Value::as_array).ok_or("invalid_response")?;
    if rows.len() > limit {
        return Err("record_limit");
    }
    if rows.iter().any(|row| !row.is_object()) {
        return Err("invalid_response");
    }
    Ok(rows.clone())
}

/// Serve only the loopback HTTP control plane; remote clients require an authenticated TLS proxy.
pub async fn serve(config: Config) -> Result<(), &'static str> {
    let address = config.listen;
    let app = application(config)?;
    let listener = tokio::net::TcpListener::bind(address).await.map_err(|_| "listen_failed")?;
    axum::serve(listener, app).await.map_err(|_| "serve_failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_config(origin: &str) -> Config {
        serde_json::from_value(json!({"listen":"127.0.0.1:8091","api_token_env":"UNUSED_TEST_TOKEN","connection_ref":"conn_test","origin":origin,"credential_env":"UNUSED_TEST_CREDENTIAL","resources":[{"resource_ref":"resource_01","table":"task","fields":[]}]})).unwrap()
    }
    #[test]
    fn selectors_cannot_inject_queries_or_paths() {
        for value in ["", "task^ORactive=true", "task/../../", "task?query", "task,name", "task\n"]
        {
            assert!(!identifier(value));
        }
        assert!(identifier("sn_vul_vulnerable_item"));
    }
    #[test]
    fn rejects_unsafe_origin_before_reading_secrets() {
        for origin in [
            "http://example.com",
            "https://user:secret@example.com",
            "https://example.com/path",
            "https://example.com/?x=y",
            "https://example.com/#private",
        ] {
            assert!(matches!(application(test_config(origin)), Err("invalid_origin")));
        }
    }
    #[test]
    fn refuses_non_loopback_control_plane() {
        let mut config = test_config("https://example.com");
        config.listen = "0.0.0.0:8091".parse().unwrap();
        assert!(matches!(application(config), Err("listen_must_be_loopback")));
    }
    #[test]
    fn rejects_scope_injection_and_alias_collision() {
        let mut config = test_config("https://example.com");
        config.resources[0].table = "task^ORactive=true".into();
        assert!(matches!(application(config), Err("invalid_scope")));
        let mut config = test_config("https://example.com");
        config.resources[0].fields.push(Field {
            field_ref: "resource_01".into(),
            name: "state".into(),
        });
        assert!(matches!(application(config), Err("invalid_scope")));
    }
    #[test]
    fn opaque_ids_are_stable_and_domain_and_connection_scoped() {
        let mut state = Runtime {
            config: test_config("https://example.com"),
            identity_key: Some("independent-test-identity-key-32-bytes".into()),
            token: String::new(),
            credential: String::new(),
            client: reqwest::Client::new(),
            origin: reqwest::Url::parse("https://example.com").unwrap(),
            permit: Semaphore::new(1),
        };
        let first = opaque(&state, "resource", "private_table");
        assert_eq!(first, opaque(&state, "resource", "private_table"));
        assert!(!first.contains("private_table"));
        assert_ne!(first, opaque(&state, "field", "private_table"));
        state.config.connection_ref = "conn_other".into();
        assert_ne!(first, opaque(&state, "resource", "private_table"));
    }
    #[test]
    fn selection_schema_rejects_agent_transport_overrides() {
        assert!(serde_json::from_value::<Selection>(
            json!({"connection_ref":"conn_test","url":"https://evil.example"})
        )
        .is_err());
        assert!(serde_json::from_value::<Config>(
            json!({"origin":"https://example.com","credential":"secret"})
        )
        .is_err());
    }
}
