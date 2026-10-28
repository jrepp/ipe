//! Native authorization and bounded I/O for the pure JSON step guest.
use super::*;
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_STEPS: usize = 8;
const MAX_REQUESTS: usize = 16;
static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Operation {
    name: String,
    table: String,
    fields: Vec<String>,
    #[serde(default)]
    parameters: Vec<String>,
    limit: usize,
}

pub(super) fn validate_configuration(config: &Config) -> Result<(), &'static str> {
    if config.script_operations.len() > 16
        || (config.script_adapter_path.is_none() && !config.script_operations.is_empty())
    {
        return Err("invalid_script_operations");
    }
    let mut names = HashSet::new();
    for op in &config.script_operations {
        if !identifier(&op.name)
            || op.name.len() > 64
            || !names.insert(&op.name)
            || !identifier(&op.table)
            || op.fields.is_empty()
            || op.fields.len() > 16
            || op.parameters.len() > 8
            || !(1..=32).contains(&op.limit)
        {
            return Err("invalid_script_operations");
        }
        for identifiers in [&op.fields, &op.parameters] {
            let mut unique = HashSet::new();
            if identifiers.iter().any(|s| !identifier(s) || !unique.insert(s)) {
                return Err("invalid_script_operations");
            }
        }
    }
    Ok(())
}

pub(super) fn capability(adapter: &script::ScriptAdapter, operations: &[Operation]) -> Value {
    json!({"abi":"json-step-v1", "digest":adapter.digest,
        "engine":"quickjs", "sdk_version":script::SDK_VERSION, "operations":operations.iter().map(|op| json!({"name":op.name,"parameters":op.parameters})).collect::<Vec<_>>(),
        "limits":{"steps":MAX_STEPS,"requests":MAX_REQUESTS,"heap_bytes":16777216,"stack_bytes":1048576,"step_timeout_ms":100},
        "imports":false,"publication":false})
}

fn query(op: &Operation, params: &BTreeMap<String, String>) -> Result<String, &'static str> {
    if params.len() != op.parameters.len()
        || params
            .iter()
            .any(|(name, value)| !op.parameters.contains(name) || !identifier(value))
    {
        return Err("script_parameters_invalid");
    }
    if params.is_empty() {
        Ok("sys_idISNOTEMPTY".into())
    } else {
        Ok(params
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join("^"))
    }
}

// A provider may ignore sysparm_fields. Project again before any value enters the
// guest; nested objects (including reference links) are not part of this ABI.
fn project(op: &Operation, rows: Vec<Value>) -> Result<Value, &'static str> {
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let mut item = serde_json::Map::new();
        for field in &op.fields {
            if let Some(value) = row.get(field) {
                if value.is_array()
                    || value.is_object()
                    || value.as_str().is_some_and(|s| s.len() > 4096)
                {
                    return Err("script_response_invalid");
                }
                item.insert(field.clone(), value.clone());
            }
        }
        result.push(Value::Object(item));
    }
    Ok(Value::Array(result))
}

pub(super) async fn run(
    State(runtime): State<Arc<Runtime>>,
    headers: HeaderMap,
    body: Result<Json<Selection>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    authorize(&runtime, &headers)?;
    selection(&runtime, body)?;
    let adapter = runtime
        .script
        .as_ref()
        .ok_or_else(|| error(StatusCode::NOT_FOUND, "script_adapter_unavailable"))?;
    let _permit = runtime
        .permit
        .try_acquire()
        .map_err(|_| error(StatusCode::TOO_MANY_REQUESTS, "busy"))?;
    let context = json!({"connection_ref":runtime.config.connection_ref,
        "sdk_version":script::SDK_VERSION,
        "operations":runtime.config.script_operations.iter().map(|op| json!({"name":op.name,"parameters":op.parameters})).collect::<Vec<_>>()});
    let run_id = NEXT_RUN.fetch_add(1, Ordering::Relaxed);
    let mut state = Value::Null;
    let mut responses = json!([]);
    let mut request_ids = HashSet::new();
    let mut count = 0;
    for step_number in 1..=MAX_STEPS {
        let result = adapter
            .step(context.clone(), responses, state)
            .await
            .map_err(|code| error(StatusCode::UNPROCESSABLE_ENTITY, code))?;
        state = result.state;
        if let Some(outputs) = result.outputs {
            return Ok(Json(json!({"connection_ref":runtime.config.connection_ref,
                "adapter":{"abi":"json-step-v1","digest":adapter.digest},
                "outputs":outputs,"steps":step_number,"requests":count,"publication":false})));
        }
        if step_number == MAX_STEPS {
            return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "script_step_limit"));
        }
        let requests = result.requests;
        count += requests.len();
        if count > MAX_REQUESTS {
            return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "script_request_limit"));
        }
        // Validate the entire batch before issuing any upstream request.
        let mut planned = Vec::with_capacity(requests.len());
        for request in &requests {
            if !request_ids.insert(request.id.clone()) {
                return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "script_request_id_reused"));
            }
            let op = runtime
                .config
                .script_operations
                .iter()
                .find(|op| op.name == request.operation)
                .ok_or_else(|| {
                    error(StatusCode::UNPROCESSABLE_ENTITY, "script_operation_forbidden")
                })?;
            let query = query(op, &request.params)
                .map_err(|code| error(StatusCode::UNPROCESSABLE_ENTITY, code))?;
            planned.push((request, op, query));
        }
        let mut next = Vec::new();
        for (request, op, query) in planned {
            let data = read(&runtime, &op.table, &query, &op.fields.join(","), op.limit)
                .await
                .and_then(|rows| project(op, rows));
            next.push(match data {
                Ok(data) => {
                    // The identity key never enters the guest. Bind the local receipt
                    // to this run, adapter, operation, parameters and projected response.
                    let material = json!([run_id, adapter.digest, request.id, op.name, op.table, op.fields, query, data]);
                    let receipt = opaque(&runtime, "receipt", &material.to_string());
                    json!({"id":request.id,"operation":request.operation,"receipt":receipt,"data":data})
                },
                Err(code) => json!({"id":request.id,"operation":request.operation,"code":code}),
            });
            // Bound aggregated input too; individual upstream responses are also capped.
            if serde_json::to_vec(&next)
                .map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "script_response_invalid"))?
                .len()
                > 65536
            {
                return Err(error(StatusCode::UNPROCESSABLE_ENTITY, "script_response_limit"));
            }
        }
        responses = Value::Array(next);
    }
    Err(error(StatusCode::UNPROCESSABLE_ENTITY, "script_step_limit"))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn operation() -> Operation {
        serde_json::from_value(json!({"name":"fields","table":"sys_dictionary","fields":["element"],"parameters":["name"],"limit":8})).unwrap()
    }
    #[test]
    fn guest_parameters_cannot_select_routes_or_inject_queries() {
        let op = operation();
        for value in ["task^ORactive=true", "task/../../", "task,name", "", "task\n"] {
            assert_eq!(
                query(&op, &BTreeMap::from([("name".into(), value.into())])),
                Err("script_parameters_invalid")
            );
        }
        assert!(query(&op, &BTreeMap::new()).is_err());
        assert!(query(&op, &BTreeMap::from([("url".into(), "task".into())])).is_err());
        assert_eq!(
            query(&op, &BTreeMap::from([("name".into(), "task".into())])).unwrap(),
            "name=task"
        );
    }
    #[test]
    fn guest_receives_only_declared_primitive_fields() {
        let op = operation();
        assert_eq!(
            project(
                &op,
                vec![json!({"element":"state","secret":"hidden","url":"https://private"})]
            )
            .unwrap(),
            json!([{"element":"state"}])
        );
        assert!(project(&op, vec![json!({"element":{"link":"https://private"}})]).is_err());
        assert!(project(&op, vec![json!({"element":"x".repeat(4097)})]).is_err());
    }

    fn config() -> Config {
        serde_json::from_value(json!({"listen":"127.0.0.1:8091","api_token_env":"UNUSED","connection_ref":"conn_test","origin":"https://example.com","credential_env":"UNUSED","resources":[],"script_adapter_path":"adapter.js"})).unwrap()
    }

    #[test]
    fn operation_grants_reject_invalid_and_ambiguous_descriptors() {
        for descriptor in [
            json!({"name":"read","table":"../secrets","fields":["name"],"limit":1}),
            json!({"name":"read","table":"task","fields":["name","name"],"limit":1}),
            json!({"name":"read","table":"task","fields":["name"],"parameters":["name^OR"],"limit":1}),
            json!({"name":"read","table":"task","fields":["name"],"limit":33}),
        ] {
            let mut cfg = config();
            cfg.script_operations = vec![serde_json::from_value(descriptor).unwrap()];
            assert_eq!(validate_configuration(&cfg), Err("invalid_script_operations"));
        }
        let mut cfg = config();
        cfg.script_operations = vec![operation(), operation()];
        assert!(validate_configuration(&cfg).is_err());
        cfg.script_operations.pop();
        assert!(validate_configuration(&cfg).is_ok());
        cfg.script_adapter_path = None;
        assert!(validate_configuration(&cfg).is_err());
    }

    fn runtime(source: &str) -> Arc<Runtime> {
        let file = tempfile::Builder::new().suffix(".js").tempfile().unwrap();
        std::fs::write(file.path(), source).unwrap();
        Arc::new(Runtime {
            adapter: wasm::Adapter::load(None).unwrap(),
            script: Some(script::ScriptAdapter::load(file.path().to_str().unwrap()).unwrap()),
            config: config(),
            identity_key: Some("private-source-receipt-key-32-bytes".into()),
            token: "private-workload-token-32-characters".into(),
            credential: "private-provider-credential".into(),
            client: reqwest::Client::new(),
            origin: reqwest::Url::parse("https://example.com").unwrap(),
            permit: Semaphore::new(1),
        })
    }

    #[tokio::test]
    async fn run_enforces_auth_scope_step_budget_and_native_receipts() {
        let runtime = runtime("function step() { return {requests:[],state:null}; }");
        let selection_body = || Ok(Json(Selection { connection_ref: "conn_test".into() }));
        assert_eq!(
            run(State(runtime.clone()), HeaderMap::new(), selection_body())
                .await
                .unwrap_err()
                .0,
            StatusCode::UNAUTHORIZED
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer private-workload-token-32-characters".parse().unwrap(),
        );
        let wrong_scope = Ok(Json(Selection { connection_ref: "other".into() }));
        assert_eq!(
            run(State(runtime.clone()), headers.clone(), wrong_scope).await.unwrap_err().0,
            StatusCode::NOT_FOUND
        );
        let (_, Json(body)) =
            run(State(runtime.clone()), headers, selection_body()).await.unwrap_err();
        assert_eq!(body["code"], "script_step_limit");
        let first = opaque(&runtime, "receipt", "run1:read:data");
        assert_eq!(first, opaque(&runtime, "receipt", "run1:read:data"));
        assert_ne!(first, opaque(&runtime, "receipt", "run2:read:data"));
        assert_ne!(first, opaque(&runtime, "receipt", "run1:read:changed"));
        assert!(!first.contains("data"));
    }
}
