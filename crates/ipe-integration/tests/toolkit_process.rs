//! Independent wire tests complement SDK-client end-to-end acceptance.
use serde_json::{json, Value};
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

struct Session {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}
impl Session {
    fn spawn() -> Self {
        Self::spawn_at("http://127.0.0.1:1")
    }
    fn spawn_at(url: &str) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_integration-toolkit"))
            .args(["--url", url, "mcp"])
            .env_clear()
            .env("IPE_INTEGRATION_TOKEN", "isolated-agent-token")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .unwrap();
        Self {
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
        let bytes = tokio::time::timeout(Duration::from_secs(5), self.output.read_line(&mut line))
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
        tokio::time::timeout(Duration::from_secs(5), self.child.wait_with_output())
            .await
            .unwrap()
            .unwrap()
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
    let mut session = Session::spawn();
    session.initialize().await;
    session.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"})).await;
    let listed = session.read().await;
    let tools = listed["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);
    for tool in tools {
        assert_eq!(tool["inputSchema"]["additionalProperties"], false);
    }
    for (id, name, args) in [
        (
            3,
            "integration_discover",
            json!({"connection_ref":"conn_test","url":"https://secret.internal"}),
        ),
        (4, "integration_discover", json!({"connection_ref":"https://secret.internal"})),
        (5, "integration_capabilities", json!({"credential":"secret.internal"})),
        (6, "secret.internal", json!({})),
    ] {
        session.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}})).await;
        let response = session.read().await;
        assert_eq!(response["id"], id);
        assert!(response.get("error").is_some() || response["result"]["isError"] == true);
    }
    session.send(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"integration_capabilities","arguments":{}}})).await;
    let response = session.read().await;
    assert_eq!(response["result"]["isError"], true);
    assert!(response.to_string().contains("workload_unavailable"));
    let output = session.stop().await;
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}
#[tokio::test]
async fn startup_error_does_not_contaminate_protocol_or_disclose_endpoint() {
    let output = Command::new(env!("CARGO_BIN_EXE_integration-toolkit"))
        .args(["--url", "https://provider-secret@internal.example", "mcp"])
        .env_clear()
        .env("IPE_INTEGRATION_TOKEN", "isolated-agent-token")
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
    let status = tokio::time::timeout(Duration::from_secs(8), session.child.wait())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}
