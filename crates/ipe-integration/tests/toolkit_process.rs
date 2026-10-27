//! Independent wire tests complement SDK-client end-to-end acceptance.
mod sanitizer_support;
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

// The SDK can drain for five seconds and sanitizer runtimes add process-exit
// overhead. This observer budget still expires before the 35-second upstream
// timeout, so a missed outstanding-request cap cannot pass by waiting for HTTP.
const OBSERVER_TIMEOUT: Duration = Duration::from_secs(20);

// Preserve only OS startup and sanitizer controls across the isolated
// agent environment. In particular, child reports must reach the parent's
// retained evidence directory instead of filling an unread stderr pipe.
fn toolkit_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_integration-toolkit"));
    command.env_clear().env("IPE_INTEGRATION_TOKEN", "isolated-agent-token");
    command.envs(sanitizer_support::environment());
    command
}

struct Session {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    stderr: tokio::task::JoinHandle<std::io::Result<Vec<u8>>>,
}
impl Session {
    fn spawn() -> Self {
        Self::spawn_at("http://127.0.0.1:1")
    }
    fn spawn_at(url: &str) -> Self {
        let mut child = toolkit_command()
            .args(["--url", url, "mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        let stderr = child.stderr.take().unwrap();
        let stderr = tokio::spawn(async move {
            let mut bytes = Vec::new();
            stderr.take(65_537).read_to_end(&mut bytes).await?;
            if bytes.len() > 65_536 {
                return Err(std::io::Error::other("toolkit stderr limit exceeded"));
            }
            Ok(bytes)
        });
        Self {
            stderr,
            input: child.stdin.take().unwrap(),
            output: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }
    async fn send(&mut self, value: Value) {
        self.input.write_all(format!("{value}\n").as_bytes()).await.unwrap();
    }
    async fn read(&mut self) -> Value {
        let mut line = String::new();
        let bytes = tokio::time::timeout(OBSERVER_TIMEOUT, self.output.read_line(&mut line))
            .await
            .unwrap()
            .unwrap();
        assert_ne!(bytes, 0, "MCP closed unexpectedly");
        assert!(!line.contains("isolated-agent-token"));
        assert!(!line.contains("secret.internal"));
        serde_json::from_str(&line).unwrap()
    }
    async fn initialize(&mut self) {
        self.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"independent-test","version":"1"}}})).await;
        let response = self.read().await;
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["serverInfo"]["name"], "ipe-integration-toolkit");
        assert_eq!(response["result"]["protocolVersion"], "2025-11-25");
        self.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"})).await;
    }
    async fn stop(mut self) -> std::process::Output {
        self.input.shutdown().await.unwrap();
        drop(self.input);
        let mut output = tokio::time::timeout(OBSERVER_TIMEOUT, self.child.wait_with_output())
            .await
            .unwrap()
            .unwrap();
        output.stderr = self.stderr.await.unwrap().unwrap();
        output
    }
}
#[tokio::test]
async fn oversized_message_is_bounded_and_terminates_session() {
    let mut session = Session::spawn();
    session.input.write_all(&vec![b'x'; 65537]).await.unwrap();
    assert_eq!(session.read().await["error"]["code"], -32700);
    let output = session.stop().await;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty());
}
#[tokio::test]
async fn duplicate_key_error_preserves_next_frame_after_initialization() {
    let mut session = Session::spawn();
    session.initialize().await;
    session.input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"integration_discover\",\"arguments\":{\"connection_ref\":\"first\",\"connection_ref\":\"second\"}}}\n").await.unwrap();
    assert_eq!(session.read().await["error"]["code"], -32700);
    session.send(json!({"jsonrpc":"2.0","id":3,"method":"ping"})).await;
    let response = session.read().await;
    assert_eq!(response["id"], 3);
    assert_eq!(response["result"], json!({}));
    let output = session.stop().await;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}
#[tokio::test]
async fn typed_tools_reject_unknown_parameters_and_sanitize_errors() {
    // An accepted connection closed without HTTP data is a deterministic
    // transport failure; do not assume a fixed local port is unused.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let upstream = tokio::spawn(async move {
        let (stream, _) = tokio::time::timeout(OBSERVER_TIMEOUT, listener.accept())
            .await
            .expect("toolkit never reached the local HTTP fixture")
            .unwrap();
        drop(stream);
    });
    let mut session = Session::spawn_at(&endpoint);
    session.initialize().await;
    session.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).await;
    let listed = session.read().await;
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 4);
    for tool in tools {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
    for (id, name, args, expected) in [
        (
            3,
            "integration_discover",
            json!({"connection_ref":"conn_test","url":"https://secret.internal"}),
            Some("invalid_tool_arguments"),
        ),
        (
            4,
            "integration_discover",
            json!({"connection_ref":"https://secret.internal"}),
            Some("invalid_connection_ref"),
        ),
        (
            5,
            "integration_capabilities",
            json!({"credential":"secret.internal"}),
            Some("invalid_tool_arguments"),
        ),
        (6, "secret.internal", json!({}), None),
    ] {
        session.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})).await;
        let response = session.read().await;
        assert_eq!(response["id"], id);
        if let Some(expected) = expected {
            assert_eq!(response["result"]["isError"], true);
            assert_eq!(response["result"]["structuredContent"]["error"]["code"], expected);
        } else {
            assert_eq!(response["error"]["code"], -32602);
        }
    }
    session.send(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"integration_capabilities","arguments":{}}})).await;
    let response = session.read().await;
    assert_eq!(response["result"]["isError"], true);
    assert!(response.to_string().contains("workload_unavailable"));
    // The accept is bounded too: a child-side connection failure must fail,
    // not hang the suite while waiting for a connection that will never arrive.
    upstream.await.unwrap();
    let output = session.stop().await;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}
#[tokio::test]
async fn startup_error_does_not_contaminate_protocol_or_disclose_endpoint() {
    let output = toolkit_command()
        .args(["--url", "https://provider-secret@internal.example", "mcp"])
        .output()
        .await
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap(),
        json!({"error":{"code":"invalid_workload_endpoint"}})
    );
}

#[tokio::test]
async fn fragmented_frame_survives_concurrent_response() {
    let mut session = Session::spawn();
    session.initialize().await;
    session.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).await;
    session.input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":3,").await.unwrap();
    assert_eq!(session.read().await["id"], 2);
    tokio::time::sleep(Duration::from_millis(20)).await;
    session.input.write_all(b"\"method\":\"ping\"}\n").await.unwrap();
    assert_eq!(session.read().await["id"], 3);
    assert!(session.stop().await.status.success());
}

#[tokio::test]
async fn outstanding_requests_are_bounded() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut session = Session::spawn_at(&format!("http://{}", listener.local_addr().unwrap()));
    session.initialize().await;
    // Keep the upstream listener alive without responding, so requests remain in flight.
    let mut input = String::new();
    for id in 2..=10 {
        input.push_str(&format!("{}\n",json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"integration_capabilities","arguments":{}}})));
    }
    session.input.write_all(input.as_bytes()).await.unwrap();
    let status = tokio::time::timeout(OBSERVER_TIMEOUT, session.child.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}
