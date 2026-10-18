//! Agent CLI and MCP transport. Provider credentials belong exclusively to the workload.
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{json, Map, Value};
use std::io::{self, BufRead, Write};
use std::time::Duration;

const MAX_MESSAGE: usize = 65536;
const MAX_RESPONSE: usize = 1048576;
const PROTOCOL_VERSION: &str = "2025-11-25";
type Result<T> = std::result::Result<T, &'static str>;

// Reject duplicate keys throughout documents rather than silently interpreting the last value.
struct Unique(Value);
impl<'de> Deserialize<'de> for Unique {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        struct JsonVisitor;
        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = Unique;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> std::result::Result<Unique, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Unique(Value::Number(n)))
                    .ok_or_else(|| E::custom("invalid number"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> std::result::Result<Unique, E> {
                Ok(Unique(v.into()))
            }
            fn visit_none<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_unit<E: de::Error>(self) -> std::result::Result<Unique, E> {
                Ok(Unique(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut items = Vec::new();
                while let Some(Unique(v)) = seq.next_element()? {
                    items.push(v);
                }
                Ok(Unique(Value::Array(items)))
            }
            fn visit_map<A: MapAccess<'de>>(
                self,
                mut map: A,
            ) -> std::result::Result<Unique, A::Error> {
                let mut items = Map::new();
                while let Some((key, Unique(value))) = map.next_entry::<String, Unique>()? {
                    if items.insert(key, value).is_some() {
                        return Err(de::Error::custom("duplicate key"));
                    }
                }
                Ok(Unique(Value::Object(items)))
            }
        }
        d.deserialize_any(JsonVisitor)
    }
}
fn decode(data: &[u8]) -> Result<Value> {
    serde_json::from_slice::<Unique>(data)
        .map(|v| v.0)
        .map_err(|_| "invalid_workload_response")
}
fn valid_ref(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

struct Client {
    origin: reqwest::Url,
    token: String,
    http: reqwest::Client,
}
impl Client {
    fn new(origin: &str, token: String) -> Result<Self> {
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
        if token.is_empty() || !token.bytes().all(|b| b > 32 && b < 127) {
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
            "check" | "discover" => {
                let reference =
                    reference.filter(|r| valid_ref(r)).ok_or("invalid_connection_ref")?;
                (
                    if operation == "check" { "/v1/connections/check" } else { "/v1/discovery" },
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
            .bearer_auth(&self.token)
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
fn tool(name: &str, description: &str, has_ref: bool) -> Value {
    let mut schema = json!({"type":"object", "properties":{}, "additionalProperties":false});
    if has_ref {
        schema["properties"] = json!({"connection_ref":{"type":"string","pattern":"^[A-Za-z0-9_-]{1,128}$","description":"Opaque connection reference returned by integration_capabilities"}});
        schema["required"] = json!(["connection_ref"]);
    }
    json!({"name":name,"description":description,"inputSchema":schema,"annotations":{"readOnlyHint":true,"openWorldHint":has_ref}})
}
fn tools() -> Value {
    json!([
    tool("integration_capabilities", "List authorized connections and implemented discovery capabilities. No provider credentials are returned.", false),
    tool("integration_check_connection", "Check configured ServiceNow metadata access through the native broker. This bounded upstream read does not prove business-record access.", true),
    tool("integration_discover", "Discover bounded permitted ServiceNow metadata and explicit coverage gaps. Does not publish facts or execute discovered scripts.", true)
])
}
fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}
#[derive(Default)]
struct Session {
    initialized: bool,
    ready: bool,
}
impl Session {
    async fn handle(&mut self, client: &Client, raw: &[u8]) -> Option<Value> {
        let message = match decode(raw) {
            Ok(v) => v,
            Err(_) => return Some(error(Value::Null, -32700, "Invalid JSON")),
        };
        let id = message.get("id").cloned().unwrap_or(Value::Null);
        let has_id = message.get("id").is_some();
        if !message.is_object()
            || message["jsonrpc"] != "2.0"
            || !message["method"].is_string()
            || (has_id && !(id.is_string() || id.is_i64() || id.is_u64()))
        {
            return Some(error(Value::Null, -32600, "Invalid request"));
        }
        let method = message["method"].as_str().unwrap();
        let empty = json!({});
        let params = message.get("params").unwrap_or(&empty);
        if !params.is_object() {
            return has_id.then(|| error(id, -32602, "Invalid params"));
        }
        if !has_id {
            if method == "notifications/initialized" && self.initialized {
                self.ready = true;
            }
            return None;
        }
        let result = match method {
            "initialize" => {
                if self.initialized || !params["protocolVersion"].is_string() {
                    return Some(error(id, -32602, "Invalid initialization"));
                }
                self.initialized = true;
                json!({"protocolVersion":PROTOCOL_VERSION,"capabilities":{"tools":{}},"serverInfo":{"name":"ipe-integration-toolkit","version":env!("CARGO_PKG_VERSION")},"instructions":"Credentials are operator-configured. Discovery is bounded and may be partial; these tools do not deploy adapters or policy facts."})
            },
            "ping" => json!({}),
            _ if !self.ready => return Some(error(id, -32000, "Initialization required")),
            "tools/list" => json!({"tools":tools()}),
            "tools/call" => {
                let operation = match params["name"].as_str() {
                    Some("integration_capabilities") => "capabilities",
                    Some("integration_check_connection") => "check",
                    Some("integration_discover") => "discover",
                    _ => return Some(error(id, -32602, "Unknown tool")),
                };
                let arguments = params.get("arguments").unwrap_or(&empty);
                let reference = arguments["connection_ref"].as_str();
                let valid = arguments.as_object().is_some_and(|a| {
                    if operation == "capabilities" {
                        a.is_empty()
                    } else {
                        a.len() == 1 && reference.is_some_and(valid_ref)
                    }
                });
                if !valid {
                    return Some(error(id, -32602, "Invalid tool arguments"));
                }
                match client.call(operation, reference).await {
                    Ok(data) => {
                        json!({"content":[{"type":"text","text":data.to_string()}],"structuredContent":data,"isError":false})
                    },
                    Err(code) => {
                        json!({"content":[{"type":"text","text":json!({"error":{"code":code}}).to_string()}],"isError":true})
                    },
                }
            },
            _ => return Some(error(id, -32601, "Method not found")),
        };
        Some(json!({"jsonrpc":"2.0","id":id,"result":result}))
    }
}
fn emit(output: &mut impl Write, value: &Value) -> io::Result<()> {
    writeln!(output, "{value}")?;
    output.flush()
}
async fn mcp(client: &Client) -> i32 {
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let mut session = Session::default();
    loop {
        let mut raw = Vec::new();
        // take() bounds allocation even when the sender never supplies a newline.
        match io::Read::take(&mut input, (MAX_MESSAGE + 1) as u64).read_until(b'\n', &mut raw) {
            Ok(0) => return 0,
            Ok(_) => {},
            Err(_) => return 1,
        }
        if raw.len() > MAX_MESSAGE {
            let _ = emit(&mut output, &error(Value::Null, -32700, "Message exceeds limit"));
            return 1;
        }
        if let Some(reply) = session.handle(client, &raw).await {
            if emit(&mut output, &reply).is_err() {
                return 0;
            }
        }
    }
}
#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let mut url =
        std::env::var("IPE_INTEGRATION_URL").unwrap_or_else(|_| "http://127.0.0.1:9010".into());
    let mut token_env = "IPE_INTEGRATION_TOKEN".to_owned();
    let mut positional = Vec::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--url" => {
                let Some(value) = args.next() else {
                    usage();
                };
                url = value;
            },
            "--token-env" => {
                let Some(value) = args.next() else {
                    usage();
                };
                token_env = value;
            },
            "--help" | "-h" => {
                println!("integration-toolkit [--url ORIGIN] [--token-env VARIABLE] capabilities|check REF|discover REF|mcp");
                return;
            },
            _ if arg.starts_with('-') => usage(),
            _ => positional.push(arg),
        }
    }
    let Some(command) = positional.first().map(String::as_str) else {
        usage();
    };
    if !matches!(
        (command, positional.len()),
        ("capabilities" | "mcp", 1) | ("check" | "discover", 2)
    ) {
        usage();
    }
    let result = match Client::new(&url, std::env::var(token_env).unwrap_or_default()) {
        Ok(client) if command == "mcp" => std::process::exit(mcp(&client).await),
        Ok(client) => client.call(command, positional.get(1).map(String::as_str)).await,
        Err(code) => Err(code),
    };
    let (value, status) = match result {
        Ok(data) => (data, 0),
        Err(code) => (json!({"error":{"code":code}}), 1),
    };
    if command == "mcp" {
        let _ = emit(&mut io::stderr(), &value);
    } else {
        let _ = emit(&mut io::stdout(), &value);
    }
    std::process::exit(status);
}
fn usage() -> ! {
    eprintln!("Usage: integration-toolkit [--url ORIGIN] [--token-env VARIABLE] capabilities|check REF|discover REF|mcp");
    std::process::exit(2)
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
    #[tokio::test]
    async fn mcp_initialization_and_validation() {
        let client = client("http://127.0.0.1:1");
        let mut session = Session::default();
        let requests = [
            ("{bad json", Some(-32700)),
            (r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#, Some(-32000)),
            (
                r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#,
                None,
            ),
            (r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#, None),
            (r#"{"jsonrpc":"2.0","id":3,"method":"tools/list"}"#, None),
            (
                r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"arbitrary_http","arguments":{"url":"secret.internal"}}}"#,
                Some(-32602),
            ),
            (
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"integration_discover","arguments":{"connection_ref":"conn_test","url":"secret.internal"}}}"#,
                Some(-32602),
            ),
            (
                r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"integration_discover","arguments":{"connection_ref":"../../secrets"}}}"#,
                Some(-32602),
            ),
            (r#"{"jsonrpc":"2.0","id":true,"method":"ping"}"#, Some(-32600)),
        ];
        for (raw, expected) in requests {
            let response = session.handle(&client, raw.as_bytes()).await;
            if let Some(code) = expected {
                assert_eq!(response.as_ref().unwrap()["error"]["code"], code);
            } else if let Some(ref response) = response {
                assert!(response.get("result").is_some());
                if response["id"] == 3 {
                    assert_eq!(response["result"]["tools"].as_array().unwrap().len(), 3);
                }
            }
            assert!(!response
                .map(|v| v.to_string())
                .unwrap_or_default()
                .contains("secret.internal"));
        }
    }
}
