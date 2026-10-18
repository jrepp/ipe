//! Exercise real stdio framing and channel isolation, beyond the RPC state unit tests.
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Output, Stdio};
fn toolkit(args: &[&str], input: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_integration-toolkit"))
        .args(args)
        .env_clear()
        .env("IPE_INTEGRATION_TOKEN", "isolated-agent-token")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(input).unwrap();
    child.wait_with_output().unwrap()
}
#[test]
fn oversized_message_is_bounded_and_terminates_session() {
    let output = toolkit(&["mcp"], &vec![b'x'; 65537]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["error"]["code"],
        -32700
    );
    assert!(output.stderr.is_empty());
}
#[test]
fn duplicate_key_error_preserves_next_frame() {
    let output = toolkit(&["mcp"], b"{\"jsonrpc\":\"2.0\",\"id\":1,\"id\":2,\"method\":\"ping\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}\n");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let replies: Vec<Value> =
        stdout.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0]["error"]["code"], -32700);
    assert_eq!(replies[1]["id"], 2);
    assert_eq!(replies[1]["result"], serde_json::json!({}));
}
#[test]
fn startup_error_does_not_contaminate_protocol_or_disclose_endpoint() {
    let output = toolkit(&["--url", "https://provider-secret@internal.example", "mcp"], b"");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stderr).unwrap(),
        serde_json::json!({"error":{"code":"invalid_workload_endpoint"}})
    );
}
