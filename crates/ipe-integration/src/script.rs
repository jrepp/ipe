//! A bounded, pure JavaScript/TypeScript adapter step. Provider I/O stays native.
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{TransformOptions, Transformer};
use rquickjs::{Context, Function, Runtime, Value as JsValue};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

const SOURCE_LIMIT: usize = 256 * 1024;
const INPUT_LIMIT: usize = 128 * 1024;
const OUTPUT_LIMIT: usize = 64 * 1024;
const HEAP_LIMIT: usize = 16 * 1024 * 1024;
const STACK_LIMIT: usize = 256 * 1024;
const STEP_DEADLINE: Duration = Duration::from_millis(100);
const CONCURRENT_STEPS: usize = 1;
const SDK_SOURCE: &str = include_str!("../sdk/index.js");
pub(crate) const SDK_VERSION: &str = "0.1.0";

#[derive(Clone)]
pub(crate) struct ScriptAdapter {
    source: Arc<str>,
    permits: Arc<Semaphore>,
    pub(crate) digest: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Request {
    pub(crate) id: String,
    pub(crate) operation: String,
    pub(crate) params: BTreeMap<String, String>,
}

#[derive(Debug)]
pub(crate) struct StepResult {
    pub(crate) requests: Vec<Request>,
    pub(crate) outputs: Option<Value>,
    pub(crate) state: Value,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireResult {
    requests: Option<Vec<Request>>,
    #[serde(rename = "outputs")]
    _outputs: Option<Value>,
    state: Value,
}

impl ScriptAdapter {
    pub(crate) fn load(path: &str) -> Result<Self, &'static str> {
        let path = Path::new(path);
        let is_ts = match path.extension().and_then(|s| s.to_str()) {
            Some("ts") => true,
            Some("js") => false,
            _ => return Err("script_extension_invalid"),
        };
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "script_unavailable")?
            .take((SOURCE_LIMIT + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| "script_unavailable")?;
        if bytes.len() > SOURCE_LIMIT {
            return Err("script_too_large");
        }
        let source = String::from_utf8(bytes).map_err(|_| "script_invalid")?;
        Self::from_source(&source, is_ts)
    }

    fn from_source(source: &str, is_ts: bool) -> Result<Self, &'static str> {
        if source.len() > SOURCE_LIMIT || source.is_empty() {
            return Err("script_invalid");
        }
        let js = if is_ts { transpile_ts(source)? } else { source.to_owned() };
        if js.len() > SOURCE_LIMIT {
            return Err("script_too_large");
        }
        let mut hasher = Sha256::new();
        hasher.update(if is_ts { b"ts\0".as_slice() } else { b"js\0".as_slice() });
        hasher.update(source.as_bytes());
        hasher.update(b"\0");
        hasher.update(js.as_bytes());
        hasher.update(b"\0sdk\0");
        hasher.update(SDK_SOURCE.as_bytes());
        let adapter = Self {
            source: Arc::from(js),
            permits: Arc::new(Semaphore::new(CONCURRENT_STEPS)),
            digest: format!("sha256:{:x}", hasher.finalize()),
        };
        // Reject malformed source and a missing entry point at operator startup.
        validate_source(&adapter.source)?;
        Ok(adapter)
    }

    pub(crate) async fn step(
        &self,
        context: Value,
        responses: Value,
        state: Value,
    ) -> Result<StepResult, &'static str> {
        let context = bounded_json(&context, INPUT_LIMIT)?;
        let responses = bounded_json(&responses, INPUT_LIMIT)?;
        let state = bounded_json(&state, OUTPUT_LIMIT)?;
        if context.len() + responses.len() + state.len() > INPUT_LIMIT {
            return Err("script_input_invalid");
        }
        let permit = self.permits.clone().try_acquire_owned().map_err(|_| "script_busy")?;
        let source = self.source.clone();
        // The permit lives inside the worker, including when its caller is cancelled.
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let wire = run_step(&source, &context, &responses, &state)?;
            validate_result(wire)
        })
        .await
        .map_err(|_| "script_failed")?
    }
}

fn bounded_json(value: &Value, limit: usize) -> Result<String, &'static str> {
    let bytes = serde_json::to_vec(value).map_err(|_| "script_input_invalid")?;
    if bytes.len() > limit {
        return Err("script_input_invalid");
    }
    String::from_utf8(bytes).map_err(|_| "script_input_invalid")
}

fn transpile_ts(source: &str) -> Result<String, &'static str> {
    let allocator = Allocator::default();
    let path = Path::new("adapter.ts");
    let source_type = SourceType::from_path(path).map_err(|_| "script_invalid")?;
    let parsed = Parser::new(&allocator, source, source_type).parse();
    if !parsed.diagnostics.is_empty() {
        return Err("script_invalid");
    }
    let mut program = parsed.program;
    let semantic = SemanticBuilder::new().build(&program);
    if !semantic.diagnostics.is_empty() {
        return Err("script_invalid");
    }
    let transformed = Transformer::new(&allocator, path, &TransformOptions::default())
        .build_with_scoping(semantic.semantic.into_scoping(), &mut program);
    if !transformed.diagnostics.is_empty() {
        return Err("script_invalid");
    }
    Ok(Codegen::new().build(&program).code)
}

fn runtime(deadline: Instant) -> Result<Runtime, &'static str> {
    let runtime = Runtime::new().map_err(|_| "script_failed")?;
    runtime.set_memory_limit(HEAP_LIMIT);
    runtime.set_max_stack_size(STACK_LIMIT);
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    Ok(runtime)
}

fn validate_source(source: &str) -> Result<(), &'static str> {
    let deadline = Instant::now() + STEP_DEADLINE;
    let runtime = runtime(deadline)?;
    let ctx = Context::full(&runtime).map_err(|_| "script_invalid")?;
    let result = ctx.with(|ctx| {
        remove_ambient_clock(&ctx).map_err(|_| "script_invalid")?;
        ctx.eval::<(), _>(source.as_bytes()).map_err(|_| "script_invalid")?;
        ctx.globals().get::<_, Function>("step").map_err(|_| "script_invalid")?;
        Ok::<(), &'static str>(())
    });
    if Instant::now() >= deadline {
        return Err("script_invalid");
    }
    result
}

fn remove_ambient_clock(ctx: &rquickjs::Ctx<'_>) -> rquickjs::Result<()> {
    ctx.eval::<(), _>("delete globalThis.Date; Math.random = undefined;")?;
    ctx.eval::<(), _>(SDK_SOURCE)
}

fn run_step(
    source: &str,
    context: &str,
    responses: &str,
    state: &str,
) -> Result<String, &'static str> {
    let deadline = Instant::now() + STEP_DEADLINE;
    let runtime = runtime(deadline)?;
    let ctx = Context::full(&runtime).map_err(|_| "script_failed")?;
    let result = ctx.with(|ctx| {
        remove_ambient_clock(&ctx).map_err(|_| "script_failed")?;
        ctx.eval::<(), _>(source.as_bytes()).map_err(|_| "script_failed")?;
        let step: Function = ctx.globals().get("step").map_err(|_| "script_failed")?;
        let context = ctx.json_parse(context.as_bytes()).map_err(|_| "script_input_invalid")?;
        let responses = ctx.json_parse(responses.as_bytes()).map_err(|_| "script_input_invalid")?;
        let state = ctx.json_parse(state.as_bytes()).map_err(|_| "script_input_invalid")?;
        let result: JsValue =
            step.call((context, responses, state)).map_err(|_| "script_failed")?;
        if result.is_promise() {
            return Err("script_output_invalid");
        }
        let output = ctx
            .json_stringify(result)
            .map_err(|_| "script_output_invalid")?
            .ok_or("script_output_invalid")?
            .to_string()
            .map_err(|_| "script_output_invalid")?;
        if output.len() > OUTPUT_LIMIT {
            return Err("script_output_invalid");
        }
        Ok::<String, &'static str>(output)
    });
    if Instant::now() >= deadline {
        return Err("script_timeout");
    }
    result
}

fn validate_result(raw: String) -> Result<StepResult, &'static str> {
    let raw_value: Value = serde_json::from_str(&raw).map_err(|_| "script_output_invalid")?;
    let object = raw_value.as_object().ok_or("script_output_invalid")?;
    let has_requests = object.contains_key("requests");
    let has_outputs = object.contains_key("outputs");
    if has_requests == has_outputs
        || (has_requests && !object.get("requests").is_some_and(Value::is_array))
        || !object.contains_key("state")
    {
        return Err("script_output_invalid");
    }
    let outputs = object.get("outputs").cloned();
    let wire: WireResult =
        serde_json::from_value(raw_value).map_err(|_| "script_output_invalid")?;
    let requests = wire.requests.unwrap_or_default();
    if requests.len() > 8 {
        return Err("script_output_invalid");
    }
    let mut ids = HashSet::new();
    for req in &requests {
        if !valid_name(&req.id)
            || !valid_name(&req.operation)
            || !ids.insert(&req.id)
            || req.params.len() > 8
            || req.params.iter().any(|(k, v)| !valid_name(k) || v.len() > 2048)
        {
            return Err("script_output_invalid");
        }
    }
    if bounded_json(&wire.state, OUTPUT_LIMIT).is_err() {
        return Err("script_output_invalid");
    }
    Ok(StepResult { requests, outputs, state: wire.state })
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn sdk_drives_broker_round_trip_and_validates_data() {
        let adapter = ScriptAdapter::from_source(r#"
            function step(context, responses, state) {
                if (state === null) return ipe.requests([ipe.request('read', 'metadata')], {phase:'read'});
                const rows = ipe.records(responses, 'read');
                return ipe.complete({row:ipe.pick(rows[0], ['name']), version:ipe.version, state:ipe.state(state)});
            }
        "#, false).unwrap();
        let first = adapter.step(json!({}), json!([]), Value::Null).await.unwrap();
        assert_eq!(first.requests[0].operation, "metadata");
        let result = adapter.step(json!({}), json!([{"id":"read","data":[{"name":"task","extra":"hidden"}],"receipt":"receipt_native"}]), first.state).await.unwrap();
        assert_eq!(
            result.outputs,
            Some(json!({"row":{"name":"task"},"version":SDK_VERSION,"state":{"phase":"read"}}))
        );
        for body in [
            "return ipe.requests([ipe.request('r','op',{name:'task^ORactive=true'})]);",
            "return ipe.requests([ipe.request('r','op'),ipe.request('r','op')]);",
            "return ipe.complete({rows:ipe.records([{id:'r',code:'access_denied'}],'r')});",
            "return ipe.complete({value:undefined});",
            "return ipe.complete('é'.repeat(40000));",
            "let x={};x.x=x;return ipe.complete(x);",
            "'use strict'; ipe.version='changed'; return ipe.complete(1);",
        ] {
            let adapter =
                ScriptAdapter::from_source(&format!("function step() {{{body}}}"), false).unwrap();
            assert!(adapter.step(json!({}), json!([]), Value::Null).await.is_err(), "{body}");
        }
    }

    #[tokio::test]
    async fn javascript_step_returns_native_requests_then_outputs() {
        let adapter = ScriptAdapter::from_source(
            "function step(context, responses, state) { if (!state.done) return {requests:[{id:'one',operation:'read',params:{table:context.table}}],state:{done:true}}; return {outputs:{row:responses.one},state}; }",
            false,
        ).unwrap();
        let first = adapter.step(json!({"table":"incident"}), json!({}), json!({})).await.unwrap();
        assert_eq!(first.requests[0].operation, "read");
        assert_eq!(first.state, json!({"done":true}));
        let second = adapter
            .step(json!({}), json!({"one":{"id":"opaque"}}), first.state)
            .await
            .unwrap();
        assert!(second.requests.is_empty());
        assert_eq!(second.outputs, Some(json!({"row":{"id":"opaque"}})));
    }

    #[tokio::test]
    async fn typescript_is_transpiled_and_executed() {
        let adapter = ScriptAdapter::from_source(
            "function step(context: { value: number }, responses: unknown, state: unknown) { return { outputs: { value: context.value + 1 }, state }; }",
            true,
        ).unwrap();
        let result = adapter.step(json!({"value":41}), json!({}), json!(null)).await.unwrap();
        assert_eq!(result.outputs, Some(json!({"value":42})));
    }

    #[tokio::test]
    async fn rejects_ambient_clock_async_and_invalid_results() {
        let adapter = ScriptAdapter::from_source(
            "function step() { return {outputs: {date: typeof Date, random: typeof Math.random}, state: null}; }",
            false,
        ).unwrap();
        let result = adapter.step(json!(null), json!(null), json!(null)).await.unwrap();
        assert_eq!(result.outputs, Some(json!({"date":"undefined","random":"undefined"})));
        let async_adapter = ScriptAdapter::from_source(
            "async function step() { return {outputs:1,state:null}; }",
            false,
        )
        .unwrap();
        assert!(async_adapter.step(json!(null), json!(null), json!(null)).await.is_err());
        let invalid = ScriptAdapter::from_source(
            "function step() { return {requests:[],outputs:1,state:null}; }",
            false,
        )
        .unwrap();
        assert!(invalid.step(json!(null), json!(null), json!(null)).await.is_err());
    }

    #[tokio::test]
    async fn infinite_loop_is_interrupted() {
        let adapter =
            ScriptAdapter::from_source("function step() { while(true) {} }", false).unwrap();
        let started = Instant::now();
        assert!(adapter.step(json!(null), json!(null), json!(null)).await.is_err());
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[tokio::test]
    async fn heap_stack_and_result_sizes_are_bounded() {
        for source in [
            "function step() { let a = []; for (;;) a.push('x'.repeat(100000)); }",
            "function step() { function recurse() { return recurse(); } return recurse(); }",
            "function step() { return {outputs:'x'.repeat(70000),state:null}; }",
            "function step() { return {outputs:1,state:'x'.repeat(70000)}; }",
        ] {
            let adapter = ScriptAdapter::from_source(source, false).unwrap();
            assert!(adapter.step(json!(null), json!(null), json!(null)).await.is_err());
        }
    }

    #[tokio::test]
    async fn no_ambient_host_apis_imports_or_persistent_globals() {
        assert!(
            ScriptAdapter::from_source("import x from 'fs'; function step() {}", false).is_err()
        );
        let adapter = ScriptAdapter::from_source(
            "let count = 0; function step() { count++; return {outputs:{count, process:typeof process, require:typeof require, fetch:typeof fetch, Deno:typeof Deno, std:typeof std, os:typeof os},state:null}; }",
            false,
        ).unwrap();
        for _ in 0..2 {
            let result = adapter.step(json!(null), json!(null), json!(null)).await.unwrap();
            assert_eq!(
                result.outputs,
                Some(json!({
                    "count":1,"process":"undefined","require":"undefined","fetch":"undefined",
                    "Deno":"undefined","std":"undefined","os":"undefined"
                }))
            );
        }
        let dynamic_import =
            ScriptAdapter::from_source("function step() { return import('fs'); }", false).unwrap();
        assert!(dynamic_import.step(json!(null), json!(null), json!(null)).await.is_err());
    }

    #[test]
    fn startup_rejects_invalid_and_unbounded_top_level_code() {
        assert!(ScriptAdapter::from_source("function step( {", false).is_err());
        assert!(ScriptAdapter::from_source("while(true) {} function step() {}", false).is_err());
        assert!(ScriptAdapter::from_source("const x = 1", false).is_err());
    }

    #[tokio::test]
    async fn null_output_is_distinct_from_absent_output() {
        let adapter = ScriptAdapter::from_source(
            "function step() { return {outputs:null,state:null}; }",
            false,
        )
        .unwrap();
        let result = adapter.step(json!(null), json!(null), json!(null)).await.unwrap();
        assert_eq!(result.outputs, Some(Value::Null));
    }

    #[tokio::test]
    async fn cancelled_caller_does_not_release_busy_worker_slot() {
        let adapter =
            ScriptAdapter::from_source("function step() { while(true) {} }", false).unwrap();
        let running = adapter.clone();
        let task =
            tokio::spawn(async move { running.step(json!(null), json!(null), json!(null)).await });
        tokio::time::timeout(Duration::from_secs(1), async {
            while adapter.permits.available_permits() != 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("worker should claim the slot");
        task.abort();
        let busy = adapter.step(json!(null), json!(null), json!(null)).await;
        assert!(matches!(busy, Err("script_busy")));
        tokio::time::timeout(Duration::from_secs(2), async {
            while adapter.permits.available_permits() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("interrupted worker should release the slot");
        assert!(matches!(
            adapter.step(json!(null), json!(null), json!(null)).await,
            Err("script_timeout") | Err("script_failed")
        ));
    }
}
