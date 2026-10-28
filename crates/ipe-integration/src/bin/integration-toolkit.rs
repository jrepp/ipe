//! Agent CLI and MCP transport. Provider credentials belong exclusively to the workload.
use clap::{CommandFactory, Parser, Subcommand};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Duration;

const MAX_MESSAGE: usize = 65536;
const MAX_RESPONSE: usize = 1048576;

type Result<T, E = &'static str> = std::result::Result<T, E>;

fn decode(data: &[u8]) -> Result<Value> {
    ipe_json::parse_value(data).map_err(|_| "invalid_workload_response")
}
fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

struct Client {
    origin: reqwest::Url,
    token: SecretString,
    http: reqwest::Client,
}
impl Client {
    fn new(origin: &str, token: SecretString) -> Result<Self> {
        let origin = reqwest::Url::parse(origin).map_err(|_| "invalid_workload_endpoint")?;
        if !matches!(origin.scheme(), "http" | "https")
            || origin.host().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.query().is_some()
            || origin.fragment().is_some()
            || origin.path() != "/"
        {
            return Err("invalid_workload_endpoint");
        }
        if origin.scheme() == "http"
            && !origin
                .host_str()
                .and_then(|h| {
                    h.trim_start_matches('[').trim_end_matches(']').parse::<std::net::IpAddr>().ok()
                })
                .is_some_and(|ip| ip.is_loopback())
        {
            return Err("invalid_workload_endpoint");
        }
        if token.expose_secret().is_empty()
            || !token.expose_secret().bytes().all(|b| b > 32 && b < 127)
        {
            return Err("missing_or_invalid_workload_credential");
        }
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(35))
            .build()
            .map_err(|_| "workload_unavailable")?;
        Ok(Self { origin, token, http })
    }
    async fn call(&self, operation: &str, reference: Option<&str>) -> Result<Value> {
        let (path, payload) = match operation {
            "capabilities" => ("/v1/capabilities", None),
            "check" | "discover" | "run" => {
                let reference =
                    reference.filter(|r| valid_ref(r)).ok_or("invalid_connection_ref")?;
                (
                    match operation {
                        "check" => "/v1/connections/check",
                        "run" => "/v1/adapters/run",
                        _ => "/v1/discovery",
                    },
                    Some(json!({"connection_ref": reference}).to_string()),
                )
            },
            _ => return Err("unknown_operation"),
        };
        let mut url = self.origin.clone();
        url.set_path(path);
        let request = if let Some(payload) = payload {
            self.http.post(url).body(payload)
        } else {
            self.http.get(url)
        };
        let mut response = request
            .bearer_auth(self.token.expose_secret())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|_| "workload_unavailable")?;
        if !response.status().is_success() {
            return Err(match response.status().as_u16() {
                401 => "unauthenticated",
                403 => "forbidden",
                404 => "not_found",
                409 => "conflict",
                422 => "invalid_request",
                429 => "rate_limited",
                _ => "workload_request_failed",
            });
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| "workload_unavailable")? {
            if body.len() + chunk.len() > MAX_RESPONSE {
                return Err("workload_response_too_large");
            }
            body.extend_from_slice(&chunk);
        }
        let data = decode(&body)?;
        if !data.is_object() {
            return Err("invalid_workload_response");
        }
        Ok(data)
    }
}
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::{
    handler::server::wrapper::Parameters,
    model::{CallToolResult, ServerCapabilities, ServerConfig},
    schemars, tool, tool_handler, tool_router, ServerHandler, ServiceExt,
};
use rmcp::{transport::Transport, RoleServer};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct EmptyArguments {}
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct ConnectionArguments {
    /// Opaque connection reference returned by integration_capabilities.
    #[schemars(regex(pattern = "^[A-Za-z0-9_-]{1,128}$"))]
    connection_ref: String,
}
struct Toolkit {
    client: Client,
}
#[tool_router]
impl Toolkit {
    fn new(client: Client) -> Self {
        Self { client }
    }
    async fn invoke(&self, operation: &str, reference: Option<&str>) -> CallToolResult {
        match self.client.call(operation, reference).await {
            Ok(data) => CallToolResult::structured(data),
            Err(code) => CallToolResult::structured_error(json!({"error":{"code":code}})),
        }
    }
    #[tool(
        description = "List authorized connections and implemented discovery capabilities. No provider credentials are returned.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn integration_capabilities(
        &self,
        Parameters(_): Parameters<EmptyArguments>,
    ) -> CallToolResult {
        self.invoke("capabilities", None).await
    }
    #[tool(
        description = "Check configured ServiceNow metadata access through the native broker. This bounded upstream read does not prove business-record access.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn integration_check_connection(
        &self,
        Parameters(args): Parameters<ConnectionArguments>,
    ) -> CallToolResult {
        self.invoke("check", Some(&args.connection_ref)).await
    }
    #[tool(
        description = "Discover bounded permitted ServiceNow metadata and explicit coverage gaps. Does not publish facts or execute discovered scripts.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn integration_discover(
        &self,
        Parameters(args): Parameters<ConnectionArguments>,
    ) -> CallToolResult {
        self.invoke("discover", Some(&args.connection_ref)).await
    }
    #[tool(
        description = "Run the configured JavaScript or TypeScript adapter with operator-allowlisted reads. Results are untrusted observations, not enforcement facts.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn integration_run_adapter(
        &self,
        Parameters(args): Parameters<ConnectionArguments>,
    ) -> CallToolResult {
        self.invoke("run", Some(&args.connection_ref)).await
    }
}
#[tool_handler]
impl ServerHandler for Toolkit {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "ipe-integration-toolkit", env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions("Credentials are operator-configured. Discovery is bounded and may be partial; these tools do not deploy adapters or policy facts.")
    }
}

// A framing/security transport only: rmcp owns protocol negotiation, lifecycle,
// request dispatch, typed tools, cancellation and responses. Reject ambiguous JSON
// before passing it to the SDK and prevent SDK diagnostics from echoing input.
struct StrictStdio {
    input: tokio::io::BufReader<tokio::io::Stdin>,
    raw: Vec<u8>,
    output: Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    failed: Arc<AtomicBool>,
    pending: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}
const MAX_MCP_RESPONSE: usize = 8 * MAX_RESPONSE;
async fn write_frame(
    output: &tokio::sync::Mutex<tokio::io::Stdout>,
    value: Value,
) -> io::Result<()> {
    let mut bytes =
        serde_json::to_vec(&value).map_err(|_| io::Error::other("invalid_mcp_output"))?;
    if bytes.len() > MAX_MCP_RESPONSE {
        return Err(io::Error::other("mcp_output_limit"));
    }
    bytes.push(b'\n');
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut output = output.lock().await;
        output.write_all(&bytes).await?;
        output.flush().await
    })
    .await
    .map_err(|_| io::Error::other("mcp_output_timeout"))?
}
fn framing_error(code: i64) -> Value {
    json!({"jsonrpc":"2.0","id":null,"error":{"code":code,"message":"Invalid request"}})
}
impl Transport<RoleServer> for StrictStdio {
    type Error = io::Error;
    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> impl std::future::Future<Output = io::Result<()>> + Send + 'static {
        let output = self.output.clone();
        let pending = self.pending.clone();
        async move {
            let mut value =
                serde_json::to_value(item).map_err(|_| io::Error::other("invalid_mcp_output"))?;
            if let Some(error) = value.get_mut("error") {
                let code = error["code"].clone();
                *error = json!({"code":code,"message":"Invalid request"});
            }
            // rmcp renders parameter deserialization failures as tool errors. Those
            // diagnostics may include caller-supplied unknown keys or values.
            if value["result"]["isError"] == true
                && value["result"]["structuredContent"]["error"]["code"].as_str().is_none()
            {
                value["result"] = serde_json::to_value(CallToolResult::structured_error(
                    json!({"error":{"code":"invalid_tool_arguments"}}),
                ))
                .map_err(|_| io::Error::other("invalid_mcp_output"))?;
            }
            let id = value.get("id").map(Value::to_string);
            let result = write_frame(&output, value).await;
            if let Some(id) = id {
                pending.lock().unwrap().remove(&id);
            }
            result
        }
    }
    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
        loop {
            if self.raw.last() != Some(&b'\n') && self.raw.len() <= MAX_MESSAGE {
                match (&mut self.input)
                    .take((MAX_MESSAGE + 1 - self.raw.len()) as u64)
                    .read_until(b'\n', &mut self.raw)
                    .await
                {
                    Ok(0) => return None,
                    Ok(_) => {},
                    Err(_) => {
                        self.failed.store(true, Ordering::Relaxed);
                        return None;
                    },
                }
            }
            if self.raw.len() > MAX_MESSAGE {
                self.failed.store(true, Ordering::Relaxed);
                let _ = write_frame(&self.output, framing_error(-32700)).await;
                return None;
            }
            let parsed = decode(&self.raw);
            self.raw.clear();
            let value = match parsed {
                Ok(value) => value,
                Err(_) => {
                    if write_frame(&self.output, framing_error(-32700)).await.is_err() {
                        return None;
                    }
                    continue;
                },
            };
            let id = if value.get("method").is_some() {
                value.get("id").map(Value::to_string)
            } else {
                None
            };
            match serde_json::from_value(value) {
                Ok(message) => {
                    if let Some(id) = id {
                        let mut pending = self.pending.lock().unwrap();
                        // Bound SDK-dispatched concurrent work without implementing protocol dispatch.
                        if pending.len() >= 8 || !pending.insert(id) {
                            self.failed.store(true, Ordering::Relaxed);
                            return None;
                        }
                    }
                    return Some(message);
                },

                Err(_) => {
                    if write_frame(&self.output, framing_error(-32600)).await.is_err() {
                        return None;
                    }
                },
            }
        }
    }
    async fn close(&mut self) -> io::Result<()> {
        self.output.lock().await.flush().await
    }
}
fn emit(output: &mut impl Write, value: &Value) -> io::Result<()> {
    writeln!(output, "{value}")?;
    output.flush()
}
async fn mcp(client: Client) -> i32 {
    let failed = Arc::new(AtomicBool::new(false));
    let transport = StrictStdio {
        raw: Vec::new(),
        input: tokio::io::BufReader::new(tokio::io::stdin()),
        output: Arc::new(tokio::sync::Mutex::new(tokio::io::stdout())),
        failed: failed.clone(),
        pending: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
    };
    match Toolkit::new(client).serve(transport).await {
        Ok(service) => {
            if service.waiting().await.is_err() {
                return 1;
            }
        },
        Err(_) => {
            if !failed.load(Ordering::Relaxed) {
                let _ = emit(&mut io::stderr(), &json!({"error":{"code":"mcp_session_failed"}}));
            }
            return 1;
        },
    }
    i32::from(failed.load(Ordering::Relaxed))
}
#[derive(Parser)]
#[command(
    name = "integration-toolkit",
    about = "Agent CLI and MCP access to the Integration Workload",
    disable_help_subcommand = true
)]
struct Arguments {
    /// Operator-configured workload origin. Plain HTTP requires numeric loopback.
    #[arg(
        long,
        env = "IPE_INTEGRATION_URL",
        default_value = "http://127.0.0.1:9010",
        global = true,
        hide_env_values = true
    )]
    url: String,
    /// Environment variable containing the workload bearer credential.
    #[arg(long, default_value = "IPE_INTEGRATION_TOKEN", global = true)]
    token_env: String,
    #[command(subcommand)]
    command: Operation,
}
#[derive(Subcommand)]
enum Operation {
    /// List authorized connections and discovery capabilities.
    Capabilities,
    /// Check metadata access for an opaque connection reference.
    Check { connection_ref: String },
    /// Discover bounded metadata for an opaque connection reference.
    Discover { connection_ref: String },
    /// Run the configured JavaScript or TypeScript adapter.
    Run { connection_ref: String },
    /// Serve MCP over standard input/output.
    Mcp,
}
fn main() -> ExitCode {
    // One agent session performs bounded asynchronous I/O. A worker per host CPU
    // multiplies threads across CLI/MCP children and adds no useful parallelism.
    // Keep blocking capacity for stdio and DNS independent of the async executor.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(4)
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => {
            eprintln!("toolkit_runtime_unavailable");
            return ExitCode::FAILURE;
        },
    };
    let status = runtime.block_on(run());
    // Tokio stdin is backed by a blocking read that cannot be canceled on all
    // platforms. Bound runtime teardown after protocol termination. Application
    // futures and owned credentials have already been dropped by run().
    runtime.shutdown_timeout(Duration::from_millis(100));
    status
}
async fn run() -> ExitCode {
    let raw: Vec<_> = std::env::args_os().collect();
    // Even parse failures/help must not write prose to an MCP protocol stream.
    let mcp_requested = raw.iter().skip(1).any(|arg| arg == "mcp");
    let args = match Arguments::try_parse_from(raw) {
        Ok(args) => args,
        Err(error) => {
            if error.kind() == clap::error::ErrorKind::DisplayHelp {
                let text = Arguments::command().render_long_help().to_string();
                if mcp_requested {
                    eprint!("{text}");
                } else {
                    print!("{text}");
                }
                return ExitCode::SUCCESS;
            }
            // clap diagnostics can include arbitrary user values, so expose only a fixed code.
            eprintln!("invalid_command_arguments");
            return ExitCode::from(2);
        },
    };
    let token = SecretString::from(std::env::var(&args.token_env).unwrap_or_default());
    let is_mcp = matches!(args.command, Operation::Mcp);
    let result = match Client::new(&args.url, token) {
        Ok(client) => match args.command {
            Operation::Mcp => return ExitCode::from(mcp(client).await as u8),
            Operation::Capabilities => client.call("capabilities", None).await,
            Operation::Check { connection_ref } => {
                client.call("check", Some(&connection_ref)).await
            },
            Operation::Run { connection_ref } => client.call("run", Some(&connection_ref)).await,
            Operation::Discover { connection_ref } => {
                client.call("discover", Some(&connection_ref)).await
            },
        },
        Err(code) => Err(code),
    };
    let (value, status) = match result {
        Ok(data) => (data, ExitCode::SUCCESS),
        Err(code) => (json!({"error":{"code":code}}), ExitCode::FAILURE),
    };
    if is_mcp {
        let _ = emit(&mut io::stderr(), &value);
    } else {
        let _ = emit(&mut io::stdout(), &value);
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    fn client(url: &str) -> Client {
        Client::new(url, "test-workload-token".into()).unwrap()
    }
    #[test]
    fn rejects_unsafe_origins_and_credentials() {
        for url in [
            "http://192.0.2.1",
            "http://localhost",
            "file:///tmp/token",
            "https://user:secret@example.com",
            "https://example.com/path",
            "https://example.com/?query=secret",
            "https://example.com/#secret",
        ] {
            assert_eq!(Client::new(url, "token".into()).err(), Some("invalid_workload_endpoint"));
        }
        for token in ["", "secret\r\nHeader", "secret secret", "☃"] {
            assert_eq!(
                Client::new("http://127.0.0.1:1", token.into()).err(),
                Some("missing_or_invalid_workload_credential")
            );
        }
        assert!(Client::new("http://[::1]:9000", "token".into()).is_ok());
    }
    #[test]
    fn strict_json_and_opaque_refs() {
        for raw in [
            r#"{"key":1,"key":2}"#,
            r#"{"nested":{"key":1,"key":2}}"#,
            r#"{"value":NaN}"#,
            "{} {}",
        ] {
            assert!(decode(raw.as_bytes()).is_err());
        }
        for reference in ["", "../secrets", "https://internal", "conn test"] {
            assert!(!valid_ref(reference));
        }
        assert!(!valid_ref(&"a".repeat(129)));
        assert!(valid_ref("conn_test-123"));
    }
    // Raw HTTP fixture reproduces transport failures without putting provider data in diagnostics.
    fn server(response: Vec<u8>) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut request = Vec::new();
            let mut byte = [0u8; 1];
            while !request.ends_with(b"\r\n\r\n") {
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let _ = stream.write_all(&response);
            String::from_utf8(request).unwrap()
        });
        (url, thread)
    }
    #[tokio::test]
    async fn truncated_http_is_sanitized() {
        let (url, thread) = server(b"HTTP/1.1 200 OK\r\nContent-Length: 999\r\nConnection: close\r\n\r\nprovider-secret internal.private.example".to_vec());
        assert_eq!(client(&url).call("capabilities", None).await, Err("workload_unavailable"));
        thread.join().unwrap();
    }
    #[tokio::test]
    async fn redirects_do_not_receive_credentials() {
        let sink = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        sink.set_nonblocking(true).unwrap();
        let response = format!("HTTP/1.1 302 Found\r\nLocation: http://{}/steal\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", sink.local_addr().unwrap());
        let (url, thread) = server(response.into_bytes());
        assert_eq!(client(&url).call("capabilities", None).await, Err("workload_request_failed"));
        assert!(thread.join().unwrap().contains("Bearer test-workload-token"));
        assert!(sink.accept().is_err());
    }
    #[tokio::test]
    async fn bounded_and_validated_responses() {
        for (body, expected) in [
            (vec![b'x'; MAX_RESPONSE + 1], "workload_response_too_large"),
            (b"{\"key\":1,\"key\":2}".to_vec(), "invalid_workload_response"),
            (b"[]".to_vec(), "invalid_workload_response"),
        ] {
            let mut response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .into_bytes();
            response.extend(body);
            let (url, thread) = server(response);
            assert_eq!(client(&url).call("capabilities", None).await, Err(expected));
            thread.join().unwrap();
        }
    }
}
