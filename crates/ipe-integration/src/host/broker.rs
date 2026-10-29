//! Shared named read operations. Packages cannot choose origins or credentials.
use crate::document::parse_yaml_located;
use reqwest::{header, Client, Url};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

type Result<T> = std::result::Result<T, &'static str>;
const BODY_LIMIT: usize = 512 * 1024;

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectionConfig {
    pub id: String,
    pub origin: String,
    pub credential_env: Option<String>,
    pub git_credential_url: Option<String>,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Parameter {
    pub kind: String,
    #[serde(default)]
    pub values: Vec<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub name: String,
    pub connection: String,
    pub path: String,
    #[serde(default)]
    pub parameters: BTreeMap<String, Parameter>,
    #[serde(default)]
    pub query: BTreeMap<String, String>,
    pub decoder: String,
    #[serde(default)]
    pub pointer: String,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default = "record_limit")]
    pub max_records: usize,
    pub accept: Option<String>,
    pub snapshot_source: Option<super::coverage::SnapshotSource>,
}
fn record_limit() -> usize {
    32
}
struct Connection {
    client: Client,
    origin: Url,
    token: Option<SecretString>,
    headers: header::HeaderMap,
}
pub(super) struct Broker {
    connections: HashMap<String, Connection>,
    pub operations: HashMap<String, Operation>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Metrics {
    pub requests: usize,
    pub retries: usize,
    pub bytes: usize,
    #[serde(default, alias = "provider_errors")]
    pub read_failures: usize,
    pub latency_ms: u64,
}

pub(super) fn name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
fn safe_parameter(value: &str, kind: &str) -> bool {
    if value.is_empty() || value.len() > 512 {
        return false;
    }
    match kind {
        "integer" => value.bytes().all(|b| b.is_ascii_digit()),
        "sha" => value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "identifier" => {
            value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                && value != "."
                && value != ".."
        },
        "path" | "ref" => value.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        }),
        _ => false,
    }
}
pub(super) fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
impl Broker {
    pub async fn new(configs: Vec<ConnectionConfig>, operations: Vec<Operation>) -> Result<Self> {
        if configs.is_empty() || configs.len() > 8 || operations.is_empty() || operations.len() > 32
        {
            return Err("invalid_broker_scope");
        }
        let mut connections = HashMap::new();
        for config in configs {
            let origin = Url::parse(&config.origin).map_err(|_| "invalid_origin")?;
            let local = origin
                .host_str()
                .and_then(|s| s.parse::<std::net::IpAddr>().ok())
                .is_some_and(|ip| ip.is_loopback());
            if !name(&config.id)
                || origin.host_str().is_none()
                || !origin.username().is_empty()
                || origin.password().is_some()
                || origin.path() != "/"
                || origin.query().is_some()
                || origin.fragment().is_some()
                || !(origin.scheme() == "https"
                    || origin.scheme() == "http"
                        && local
                        && config.credential_env.is_none()
                        && config.git_credential_url.is_none())
            {
                return Err("invalid_origin");
            }
            let token = match (&config.credential_env, &config.git_credential_url) {
                (Some(_), Some(_)) => return Err("ambiguous_credential_source"),
                (Some(key), None) => Some(SecretString::from(
                    std::env::var(key).map_err(|_| "missing_provider_credential")?,
                )),
                (None, Some(url)) => Some(super::credentials::git(url).await?),
                (None, None) => None,
            };
            if token
                .as_ref()
                .is_some_and(|t| t.expose_secret().is_empty() || t.expose_secret().len() > 8192)
            {
                return Err("invalid_credentials");
            }
            let mut headers = header::HeaderMap::new();
            if config.headers.len() > 8 {
                return Err("invalid_headers");
            }
            for (key, value) in config.headers {
                if !["accept", "x-github-api-version"].contains(&key.to_ascii_lowercase().as_str())
                {
                    return Err("invalid_headers");
                }
                headers.insert(
                    header::HeaderName::from_bytes(key.as_bytes())
                        .map_err(|_| "invalid_headers")?,
                    header::HeaderValue::from_str(&value).map_err(|_| "invalid_headers")?,
                );
            }
            let client = Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(8))
                .connect_timeout(Duration::from_secs(3))
                .user_agent("ipe-integration-host/0.1")
                .build()
                .map_err(|_| "client_failed")?;
            if connections
                .insert(config.id, Connection { client, origin, token, headers })
                .is_some()
            {
                return Err("duplicate_connection");
            }
        }
        let mut result = HashMap::new();
        for op in operations {
            if !name(&op.name)
                || !connections.contains_key(&op.connection)
                || !op.path.starts_with('/')
                || op.path.len() > 512
                || op.parameters.len() > 8
                || op.query.len() > 8
                || op.fields.len() > 32
                || !(1..=128).contains(&op.max_records)
                || !["json", "yaml", "text"].contains(&op.decoder.as_str())
                || op.decoder == "json" && op.fields.is_empty()
                || op.pointer.len() > 256
            {
                return Err("invalid_operation");
            }
            if let Some(scope) = &op.snapshot_source {
                if op.decoder != "yaml"
                    || !name(&scope.namespace)
                    || [&scope.resource_parameter, &scope.path_parameter, &scope.revision_parameter]
                        .iter()
                        .any(|p| !op.parameters.contains_key(*p))
                {
                    return Err("invalid_snapshot_source");
                }
            }
            for (key, param) in &op.parameters {
                if !name(key)
                    || !["identifier", "integer", "path", "ref", "sha"]
                        .contains(&param.kind.as_str())
                    || param.values.len() > 64
                    || param.values.iter().any(|v| !safe_parameter(v, &param.kind))
                {
                    return Err("invalid_parameter_schema");
                }
            }
            // Every template is either a declared parameter or a fixed safe segment.
            for part in op
                .path
                .trim_start_matches('/')
                .split('/')
                .chain(op.query.values().map(String::as_str))
            {
                if let Some(key) = part.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
                    if !op.parameters.contains_key(key) {
                        return Err("invalid_operation_template");
                    }
                } else if !safe_parameter(part, "identifier") {
                    return Err("invalid_operation_template");
                }
            }
            if result.insert(op.name.clone(), op).is_some() {
                return Err("duplicate_operation");
            }
        }
        Ok(Self { connections, operations: result })
    }
    pub fn validate(&self, operation: &str, params: &BTreeMap<String, String>) -> Result<()> {
        let op = self.operations.get(operation).ok_or("operation_forbidden")?;
        if params.len() != op.parameters.len() {
            return Err("invalid_operation_parameters");
        }
        for (key, schema) in &op.parameters {
            let value = params.get(key).ok_or("invalid_operation_parameters")?;
            if !safe_parameter(value, &schema.kind)
                || !schema.values.is_empty() && !schema.values.contains(value)
            {
                return Err("resource_not_permitted");
            }
        }
        Ok(())
    }
    fn url(&self, op: &Operation, params: &BTreeMap<String, String>) -> Result<Url> {
        self.validate(&op.name, params)?;
        let conn = self.connections.get(&op.connection).ok_or("connection_unavailable")?;
        let mut url = conn.origin.clone();
        {
            let mut segments = url.path_segments_mut().map_err(|_| "invalid_origin")?;
            segments.clear();
            for part in op.path.trim_start_matches('/').split('/') {
                if let Some(key) = part.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
                    if op.parameters[key].kind == "path" {
                        for segment in params[key].split('/') {
                            segments.push(segment);
                        }
                    } else {
                        segments.push(&params[key]);
                    }
                } else {
                    segments.push(part);
                }
            }
        }
        for (key, template) in &op.query {
            let value =
                if let Some(key) = template.strip_prefix('{').and_then(|v| v.strip_suffix('}')) {
                    params.get(key).ok_or("invalid_operation_parameters")?
                } else {
                    template
                };
            url.query_pairs_mut().append_pair(key, value);
        }
        Ok(url)
    }
    fn protect_credentials(&self, value: Value) -> Result<Value> {
        let encoded = serde_json::to_string(&value).map_err(|_| "provider_decode_failed")?;
        if self.connections.values().filter_map(|c| c.token.as_ref()).any(|token| {
            let escaped = serde_json::to_string(token.expose_secret()).unwrap_or_default();
            encoded.contains(token.expose_secret())
                || (escaped.len() > 2 && encoded.contains(&escaped[1..escaped.len() - 1]))
        }) {
            return Err("provider_credential_in_response");
        }
        Ok(value)
    }
    pub async fn read(
        &self,
        operation: &str,
        params: &BTreeMap<String, String>,
        metrics: &mut Metrics,
    ) -> Result<Value> {
        let op = self.operations.get(operation).ok_or("operation_forbidden")?;
        let url = self.url(op, params)?;
        let conn = &self.connections[&op.connection];
        let start = Instant::now();
        let mut attempts = 0;
        let result = loop {
            if metrics.requests >= 96 || metrics.bytes >= 8 * 1024 * 1024 {
                break Err("run_io_budget");
            }
            attempts += 1;
            metrics.requests += 1;
            let mut request = conn.client.get(url.clone()).headers(conn.headers.clone());
            if let Some(accept) = &op.accept {
                request = request.header(header::ACCEPT, accept);
            }
            if let Some(token) = &conn.token {
                let mut value =
                    header::HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
                        .map_err(|_| "invalid_credentials")?;
                value.set_sensitive(true);
                request = request.header(header::AUTHORIZATION, value);
            }
            let response = request.send().await;
            let retryable = response
                .as_ref()
                .map_or(true, |r| r.status().as_u16() == 429 || r.status().is_server_error());
            if retryable && attempts < 3 {
                let retry_after = response
                    .as_ref()
                    .ok()
                    .and_then(|r| r.headers().get(header::RETRY_AFTER))
                    .and_then(|v| v.to_str().ok());
                let delay = match retry_after {
                    Some(value) => match value.parse::<u64>() {
                        Ok(n) if n <= 2 => Duration::from_secs(n),
                        _ => break Err("retry_after_exceeds_budget"),
                    },
                    None => Duration::from_millis(100 * attempts),
                };
                metrics.retries += 1;
                tokio::time::sleep(delay).await;
                continue;
            }
            let mut response = match response {
                Ok(r) => r,
                Err(_) => break Err("provider_transport_failed"),
            };
            match response.status().as_u16() {
                200 => {},
                401 | 403 => break Err("provider_access_denied"),
                404 => break Err("provider_not_found"),
                429 => break Err("provider_throttled"),
                300..=399 => break Err("provider_redirect_rejected"),
                _ => break Err("provider_http_failed"),
            }
            if response.content_length().is_some_and(|n| n > BODY_LIMIT as u64) {
                break Err("provider_byte_limit");
            }
            let more = response
                .headers()
                .get(header::LINK)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v.contains("rel=\"next\"") || v.contains("rel=next"));
            let mut bytes = Vec::new();
            let mut failure = None;
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        metrics.bytes += chunk.len();
                        if bytes.len() + chunk.len() > BODY_LIMIT || metrics.bytes > 8 * 1024 * 1024
                        {
                            failure = Some("provider_byte_limit");
                            break;
                        }
                        bytes.extend_from_slice(&chunk);
                    },
                    Ok(None) => break,
                    Err(_) => {
                        failure = Some("provider_transport_failed");
                        break;
                    },
                }
            }
            if let Some(code) = failure {
                break Err(code);
            }
            break decode(op, &bytes).and_then(|mut value| {
                if more {
                    value["truncated"] = json!(true);
                }
                value["pagination"] = json!(if more {
                    "more_pages"
                } else if op.decoder == "yaml" {
                    "single_document"
                } else {
                    "unknown"
                });
                if value["truncated"] == false {
                    if let Some(scope) = &op.snapshot_source {
                        value["snapshot_source"] = json!({
                            "definition": {
                                "namespace":scope.namespace,
                                "resource":params[&scope.resource_parameter],
                                "path":params[&scope.path_parameter]
                            },
                            "revision":params[&scope.revision_parameter],
                            "pointer":""
                        });
                    }
                }
                self.protect_credentials(value)
            });
        };
        metrics.latency_ms += start.elapsed().as_millis() as u64;
        if result.is_err() {
            metrics.read_failures += 1;
        }
        result
    }
}
fn decode(op: &Operation, bytes: &[u8]) -> Result<Value> {
    let hash = digest(bytes);
    let (records, truncated) = match op.decoder.as_str() {
        "yaml" => {
            let text = std::str::from_utf8(bytes).map_err(|_| "provider_not_utf8")?;
            let (document, locations) = parse_yaml_located(text)?;
            (vec![json!({"document":document,"locations":locations})], false)
        },
        "text" => {
            let text = std::str::from_utf8(bytes).map_err(|_| "provider_not_utf8")?;
            let excerpt: String = text.chars().take(12000).collect();
            let truncated = excerpt.len() < text.len();
            (vec![json!({"text":excerpt})], truncated)
        },
        _ => {
            let value: Value = ipe_json::parse(bytes).map_err(|_| "provider_invalid_json")?;
            let selected = value.pointer(&op.pointer).ok_or("provider_shape_mismatch")?;
            let rows = if let Some(rows) = selected.as_array() {
                rows.clone()
            } else if selected.is_object() {
                vec![selected.clone()]
            } else {
                return Err("provider_shape_mismatch");
            };
            let truncated = rows.len() > op.max_records
                || value.get("truncated").and_then(Value::as_bool) == Some(true)
                || value
                    .get("total_count")
                    .and_then(Value::as_u64)
                    .is_some_and(|n| n > rows.len() as u64);
            let mut records = Vec::new();
            for row in rows.into_iter().take(op.max_records) {
                let object = row.as_object().ok_or("provider_shape_mismatch")?;
                let mut item = serde_json::Map::new();
                for field in &op.fields {
                    if let Some(value) = object.get(field) {
                        item.insert(field.clone(), value.clone());
                    }
                }
                records.push(Value::Object(item));
            }
            (records, truncated)
        },
    };
    let result = json!({"records":records,"source_digest":hash,"truncated":truncated});
    if serde_json::to_vec(&result).map_err(|_| "provider_shape_mismatch")?.len() > 60 * 1024 {
        return Err("provider_projection_limit");
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn operation() -> Operation {
        serde_json::from_value(json!({"name":"read","connection":"fixture","path":"/repos/{repository}/contents/{path}","parameters":{"repository":{"kind":"path","values":["owner/repo"]},"path":{"kind":"path"},"revision":{"kind":"sha"}},"query":{"ref":"{revision}"},"decoder":"json","fields":["value"]})).unwrap()
    }
    #[tokio::test]
    async fn operation_templates_cannot_escape_origin_or_resource_scope() {
        let broker = Broker::new(
            vec![ConnectionConfig {
                id: "fixture".into(),
                origin: "http://127.0.0.1:1234/".into(),
                credential_env: None,
                git_credential_url: None,
                headers: BTreeMap::new(),
            }],
            vec![operation()],
        )
        .await
        .unwrap();
        let mut params = BTreeMap::from([
            ("repository".into(), "owner/repo".into()),
            ("path".into(), ".github/workflows/ci.yml".into()),
            ("revision".into(), "a".repeat(40)),
        ]);
        assert!(broker
            .url(&operation(), &params)
            .unwrap()
            .as_str()
            .contains("/repos/owner/repo/contents/.github/workflows/ci.yml?ref="));
        for value in [
            "../secret",
            "%2e%2e/secret",
            "https://evil.test/path",
            "a?token=x",
            "/etc/passwd",
        ] {
            params.insert("path".into(), value.into());
            assert!(broker.validate("read", &params).is_err());
        }
        params.insert("path".into(), "safe".into());
        params.insert("repository".into(), "other/repo".into());
        assert!(broker.validate("read", &params).is_err());
    }
    #[tokio::test]
    async fn rejected_reads_do_not_follow_redirects_or_retry_denials() {
        use axum::{routing::get, Router};
        let app = Router::new()
            .route(
                "/redirect",
                get(|| async { (axum::http::StatusCode::FOUND, [("location", "/denied")], "") }),
            )
            .route(
                "/denied",
                get(|| async { (axum::http::StatusCode::FORBIDDEN, "private details") }),
            )
            .route("/large", get(|| async { "x".repeat(BODY_LIMIT + 1) }))
            .route(
                "/wait",
                get(|| async {
                    (axum::http::StatusCode::TOO_MANY_REQUESTS, [("retry-after", "60")], "")
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        for (path, code) in [
            ("redirect", "provider_redirect_rejected"),
            ("denied", "provider_access_denied"),
            ("large", "provider_byte_limit"),
            ("wait", "retry_after_exceeds_budget"),
        ] {
            let mut op = operation();
            op.path = format!("/{path}");
            op.parameters.clear();
            op.query.clear();
            let broker = Broker::new(
                vec![ConnectionConfig {
                    id: "fixture".into(),
                    origin: origin.clone(),
                    credential_env: None,
                    git_credential_url: None,
                    headers: BTreeMap::new(),
                }],
                vec![op],
            )
            .await
            .unwrap();
            let mut metrics = Metrics::default();
            assert_eq!(
                broker.read("read", &BTreeMap::new(), &mut metrics).await.unwrap_err(),
                code
            );
            assert_eq!(metrics.requests, 1);
            assert_eq!(metrics.read_failures, 1);
        }
        server.abort();
    }
    #[tokio::test]
    async fn credential_sources_are_exclusive_and_require_https() {
        for (origin, env, expected) in [
            ("http://127.0.0.1", None, "invalid_origin"),
            (
                "https://example.test",
                Some("UNUSED_SECRET".into()),
                "ambiguous_credential_source",
            ),
        ] {
            let result = Broker::new(
                vec![ConnectionConfig {
                    id: "fixture".into(),
                    origin: origin.into(),
                    credential_env: env,
                    git_credential_url: Some("https://github.com".into()),
                    headers: BTreeMap::new(),
                }],
                vec![operation()],
            )
            .await;
            assert_eq!(result.err(), Some(expected));
        }
    }
    #[tokio::test]
    async fn echoed_native_credentials_do_not_reach_guest() {
        let mut broker = Broker::new(
            vec![ConnectionConfig {
                id: "fixture".into(),
                origin: "https://example.test".into(),
                credential_env: None,
                git_credential_url: None,
                headers: BTreeMap::new(),
            }],
            vec![operation()],
        )
        .await
        .unwrap();
        broker.connections.get_mut("fixture").unwrap().token =
            Some(SecretString::from("private_test_credential"));
        assert!(broker.protect_credentials(json!({"value":"private_test_credential"})).is_err());
        assert!(broker.protect_credentials(json!({"value":"public"})).is_ok());
    }
    #[test]
    fn provider_counts_and_truncation_flags_preserve_incomplete_collection_status() {
        let mut op = operation();
        op.pointer = "/items".into();
        for body in [
            r#"{"items":[{"value":1}],"total_count":2}"#,
            r#"{"items":[{"value":1}],"truncated":true}"#,
        ] {
            assert_eq!(decode(&op, body.as_bytes()).unwrap()["truncated"], true);
        }
        assert_eq!(
            decode(&op, br#"{"items":[{"value":1}],"total_count":1}"#).unwrap()["truncated"],
            false
        );
        op.max_records = 1;
        assert_eq!(
            decode(&op, br#"{"items":[{"value":1},{"value":2}]}"#).unwrap()["truncated"],
            true
        );
    }
    #[test]
    fn response_projection_keeps_hidden_fields_out_and_rejects_duplicate_json() {
        let op = operation();
        assert_eq!(
            decode(&op, br#"{"value":42,"secret":"hidden"}"#).unwrap()["records"],
            json!([{"value":42}])
        );
        assert!(decode(&op, br#"{"value":1,"value":2}"#).is_err());
        assert!(decode(&op, b"null").is_err());
    }
}
