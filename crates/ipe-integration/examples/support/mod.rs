//! Black-box local deployment harness. Provider behavior lives in the TypeScript facade.
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
use tempfile::TempDir;

const SECRET: &str = "provider-secret-DO-NOT-DISCLOSE";
const TOPOLOGY: &str = "internal-remediation.corp.example";
const RAW_ID: &str = "1234567890abcdef1234567890abcdef";
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn require(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(message.into())
    }
}
struct Process(Child);
impl Drop for Process {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub struct Deployment {
    workload: Option<Process>,
    _fixture: Process,
    directory: TempDir,
    binary: PathBuf,
    toolkit: PathBuf,
    client: reqwest::Client,
    fixture: Value,
    config: Value,
    token: String,
    identity_key: String,
    url: String,
}
impl Deployment {
    pub async fn start(binary: &Path, toolkit: &Path) -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let fixture_dir = directory.path().join("fixture");
        fs::create_dir(&fixture_dir)?;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut fixture_process = Process(
            Command::new("node")
                .env_clear()
                .env("PATH", std::env::var_os("PATH").unwrap_or_default())
                .arg(root.join("testing/integration/servicenow-standin.ts"))
                .arg("--directory")
                .arg(&fixture_dir)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()?,
        );
        let stdout = fixture_process.0.stdout.take().ok_or("fixture output missing")?;
        let line = tokio::time::timeout(
            Duration::from_secs(20),
            tokio::task::spawn_blocking(move || {
                let mut line = String::new();
                BufReader::new(stdout).read_line(&mut line).map(|_| line)
            }),
        )
        .await???;
        let fixture: Value = serde_json::from_str(&line)?;
        let port = TcpListener::bind("127.0.0.1:0")?.local_addr()?.port();
        let token = format!(
            "agent-{}-local-only-token",
            directory.path().file_name().unwrap().to_string_lossy()
        );
        let identity_key = format!(
            "identity-{}-local-only-32-byte-key",
            directory.path().file_name().unwrap().to_string_lossy()
        );
        let config = json!({"listen":format!("127.0.0.1:{port}"), "api_token_env":"IPE_INTEGRATION_TOKEN", "connection_ref":"conn_acceptance", "origin":fixture["origin"], "credential_env":"SERVICENOW_TOKEN", "ca_cert_path":fixture["ca_cert_path"], "identity_key_env":"IPE_INTEGRATION_IDENTITY_KEY", "resources":[]});
        let mut deployment = Self {
            workload: None,
            _fixture: fixture_process,
            directory,
            binary: binary.into(),
            toolkit: toolkit.into(),
            client: reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(20))
                .build()?,
            fixture,
            config,
            token,
            identity_key,
            url: format!("http://127.0.0.1:{port}"),
        };
        deployment.restart().await?;
        Ok(deployment)
    }
    async fn restart(&mut self) -> Result<()> {
        self.workload.take();
        let path = self.directory.path().join("config.json");
        private_write(&path, &serde_json::to_string(&self.config)?)?;
        let log = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.directory.path().join("workload.log"))?;
        self.workload = Some(Process(
            Command::new(&self.binary)
                .arg(path)
                .env_clear()
                .env("IPE_INTEGRATION_TOKEN", &self.token)
                .env("SERVICENOW_TOKEN", SECRET)
                .env("IPE_INTEGRATION_IDENTITY_KEY", &self.identity_key)
                .stdout(log.try_clone()?)
                .stderr(log)
                .spawn()?,
        ));
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            require(
                self.workload.as_mut().unwrap().0.try_wait()?.is_none(),
                "workload exited before readiness",
            )?;
            if self
                .client
                .get(format!("{}/v1/capabilities", self.url))
                .bearer_auth(&self.token)
                .send()
                .await
                .is_ok()
            {
                return Ok(());
            }
            require(Instant::now() < deadline, "workload readiness timeout")?;
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
    }
    fn redacted(&self, value: &str) -> Result<()> {
        for marker in [
            SECRET,
            TOPOLOGY,
            RAW_ID,
            &self.token,
            &self.identity_key,
            self.fixture["origin"].as_str().unwrap(),
            self.fixture["ca_cert_path"].as_str().unwrap(),
        ] {
            require(!value.contains(marker), "public output exposed secret or topology")?;
        }
        Ok(())
    }
    async fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> Result<(u16, Value)> {
        let mut req = self
            .client
            .request(method, format!("{}{path}", self.url))
            .header("content-type", "application/json");
        if let Some(token) = token {
            req = req.bearer_auth(token);
        }
        if let Some(body) = body {
            req = req.body(body.to_string());
        }
        let response = req.send().await?;
        let status = response.status().as_u16();
        let text = response.text().await?;
        self.redacted(&text)?;
        Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
    }
    async fn discover(&self) -> Result<(u16, Value)> {
        self.request(
            reqwest::Method::POST,
            "/v1/discovery",
            Some(json!({"connection_ref":"conn_acceptance"})),
            Some(&self.token),
        )
        .await
    }
    async fn control(&self, body: Value) -> Result<Value> {
        let response = self
            .client
            .post(format!("{}/control", self.fixture["control_url"].as_str().unwrap()))
            .bearer_auth(self.fixture["control_token"].as_str().unwrap())
            .header("content-type", "application/json")
            .body(body.to_string())
            .send()
            .await?;
        require(response.status().is_success(), "fixture control failed")?;
        Ok(serde_json::from_str(&response.text().await?)?)
    }
    async fn requests(&self) -> Result<Vec<Value>> {
        let text = self
            .client
            .get(format!("{}/state", self.fixture["control_url"].as_str().unwrap()))
            .bearer_auth(self.fixture["control_token"].as_str().unwrap())
            .send()
            .await?
            .text()
            .await?;
        let state: Value = serde_json::from_str(&text)?;
        Ok(state["requests"].as_array().ok_or("fixture requests missing")?.clone())
    }
    async fn toolkit(&self, args: &[&str], input: Option<String>) -> Result<String> {
        let mut child = tokio::process::Command::new(&self.toolkit)
            .arg("--url")
            .arg(&self.url)
            .args(args)
            .env_clear()
            .env("IPE_INTEGRATION_TOKEN", &self.token)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        if let Some(input) = input {
            use tokio::io::AsyncWriteExt;
            child.stdin.take().unwrap().write_all(input.as_bytes()).await?;
        } else {
            child.stdin.take();
        }
        let output =
            tokio::time::timeout(Duration::from_secs(30), child.wait_with_output()).await??;
        self.redacted(&String::from_utf8_lossy(&output.stderr))?;
        let text = String::from_utf8(output.stdout)?;
        self.redacted(&text)?;
        require(output.status.success(), "Rust toolkit failed")?;
        Ok(text)
    }
    pub async fn agent_interfaces(&self) -> Result<()> {
        for args in [
            vec!["capabilities"],
            vec!["check", "conn_acceptance"],
            vec!["discover", "conn_acceptance"],
        ] {
            let value: Value = serde_json::from_str(&self.toolkit(&args, None).await?)?;
            let valid = match args[0] {
                "capabilities" => {
                    value["publication"] == false && value["adapter"]["engine"] == "wasmi"
                },
                "check" => value["status"] == "available",
                _ => {
                    value["coverage"] == "bounded" && value["resources"][0]["status"] == "observed"
                },
            };
            require(valid, "CLI operation did not succeed")?;
        }
        tokio::time::timeout(Duration::from_secs(30), self.sdk_mcp()).await?
    }
    async fn sdk_mcp(&self) -> Result<()> {
        use rmcp::{model::CallToolRequestParams, ServiceExt};
        let mut child = tokio::process::Command::new(&self.toolkit)
            .args(["--url", &self.url, "mcp"])
            .env_clear()
            .env("IPE_INTEGRATION_TOKEN", &self.token)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        let service = ().serve((child.stdout.take().unwrap(), child.stdin.take().unwrap())).await?;
        let listed = service.list_tools(None).await?;
        self.redacted(&serde_json::to_string(&listed)?)?;
        let names: Vec<_> = listed.tools.iter().map(|tool| tool.name.as_ref()).collect();
        require(
            names.len() == 3
                && names.contains(&"integration_discover")
                && names.contains(&"integration_capabilities")
                && names.contains(&"integration_check_connection"),
            "MCP surface changed",
        )?;
        for (name, args, key, expected) in [
            ("integration_capabilities", json!({}), "publication", json!(false)),
            (
                "integration_check_connection",
                json!({"connection_ref":"conn_acceptance"}),
                "status",
                json!("available"),
            ),
            (
                "integration_discover",
                json!({"connection_ref":"conn_acceptance"}),
                "coverage",
                json!("bounded"),
            ),
        ] {
            let response = service
                .call_tool(
                    CallToolRequestParams::new(name)
                        .with_arguments(args.as_object().unwrap().clone()),
                )
                .await?;
            self.redacted(&serde_json::to_string(&response)?)?;
            require(
                response.is_error != Some(true)
                    && response.structured_content.as_ref().is_some_and(|v| v[key] == expected),
                "SDK client received unsuccessful tool result",
            )?;
        }
        service.cancel().await?;
        let output = child.wait_with_output().await?;
        self.redacted(&String::from_utf8_lossy(&output.stderr))?;
        require(output.status.success(), "MCP child failed at shutdown")
    }
    pub async fn smoke(&self) -> Result<Value> {
        let (code, result) = self.discover().await?;
        require(
            code == 200
                && result["coverage"] == "bounded"
                && result["constraints"]["status"] == "unknown",
            "inventory coverage incorrect",
        )?;
        require(
            result["resources"].as_array().is_some_and(|v| v.len() == 1),
            "inventory missing resource",
        )?;
        require(
            result["resources"][0]["fields"][0]["role"] == "lifecycle_state",
            "WASM field classification missing",
        )?;
        require(
            result["adapter"]["abi"] == "metadata-field-v1"
                && result["adapter"]["digest"]
                    .as_str()
                    .is_some_and(|s| s.starts_with("sha256:") && s.len() == 71),
            "WASM provenance missing",
        )?;
        self.agent_interfaces().await?;
        Ok(result)
    }
    pub async fn acceptance(&mut self) -> Result<Value> {
        let mut checks = Vec::new();
        let success = self.smoke().await?;
        checks.push("credential-first inventory, hosted WASM classification, CLI and MCP");
        self.restart().await?;
        require(self.discover().await?.1 == success, "opaque identities unstable across restart")?;
        checks.push("stable opaque identities after restart");
        let module_path = self.directory.path().join("replacement.wasm");
        for (body, expected, fault) in [
            ("i32.const 2", "assignee", None),
            ("(loop $spin (br $spin)) i32.const 0", "unknown", Some("adapter_failed")),
            ("i32.const 99", "unknown", Some("adapter_output_invalid")),
        ] {
            fs::write(&module_path, wat::parse_str(format!("(module (memory (export \"memory\") 1) (func (export \"classify\") (param i32 i32 i32 i32) (result i32) {body}))"))?)?;
            self.config["adapter_path"] = json!(module_path);
            self.restart().await?;
            let (code, result) = self.discover().await?;
            require(
                code == 200
                    && result["resources"][0]["fields"][0]["role"] == expected
                    && result["adapter"]["digest"] != success["adapter"]["digest"],
                "replacement WASM did not execute",
            )?;
            if let Some(fault) = fault {
                require(
                    result["resources"][0]["field_code"] == fault,
                    "WASM failure did not remain unknown",
                )?;
            }
            require(
                result["resources"][0]["resource_ref"] == success["resources"][0]["resource_ref"],
                "adapter changed native opaque identity",
            )?;
        }
        self.config.as_object_mut().unwrap().remove("adapter_path");
        self.restart().await?;
        require(self.discover().await?.1 == success, "default adapter restoration failed")?;
        checks.push("replacement WASM, trap and invalid output isolation, restoration");
        let before = self.requests().await?.len();
        for token in [None, Some("wrong-token")] {
            for (method, path, body) in [
                (reqwest::Method::GET, "/v1/capabilities", None),
                (
                    reqwest::Method::POST,
                    "/v1/connections/check",
                    Some(json!({"connection_ref":"conn_acceptance"})),
                ),
                (
                    reqwest::Method::POST,
                    "/v1/discovery",
                    Some(json!({"connection_ref":"conn_acceptance"})),
                ),
            ] {
                require(
                    matches!(self.request(method, path, body, token).await?.0, 401 | 403),
                    "unauthorized operation accepted",
                )?;
            }
        }
        for reference in ["conn_foreign", "https://internal.example", "../conn_acceptance"] {
            let code = self
                .request(
                    reqwest::Method::POST,
                    "/v1/discovery",
                    Some(json!({"connection_ref":reference})),
                    Some(&self.token),
                )
                .await?
                .0;
            require((400..500).contains(&code), "unconfigured reference accepted")?;
        }
        require(
            self.requests().await?.len() == before,
            "unauthorized operation contacted provider",
        )?;
        checks.push("authentication and connection isolation");
        for key in ["url", "table", "sysparm_query", "credential", "publish"] {
            let body = json!({"connection_ref":"conn_acceptance",key:"forbidden"});
            require(
                (400..500).contains(
                    &self
                        .request(
                            reqwest::Method::POST,
                            "/v1/discovery",
                            Some(body),
                            Some(&self.token),
                        )
                        .await?
                        .0,
                ),
                "caller controlled provider parameter accepted",
            )?;
        }
        for path in ["/v1/publish", "/v1/evaluate", "/v1/runtime/facts", "/v1/connections"] {
            require(
                matches!(
                    self.request(reqwest::Method::POST, path, Some(json!({})), Some(&self.token))
                        .await?
                        .0,
                    404 | 405
                ),
                "unexpected publication capability",
            )?;
        }
        require(self.requests().await?.len() == before, "rejected request contacted provider")?;
        checks.push("strict input and no enforcement/publication surface");
        self.config["resources"] = json!([{"resource_ref":"resource_remediation","table":"sn_vul_vulnerable_item","fields":[{"field_ref":"field_state","name":"state"}]}]);
        self.restart().await?;
        self.control(json!({"clear_requests":true})).await?;
        let (code, result) = self.discover().await?;
        require(
            code == 200
                && result["resources"][0]["resource_ref"] == "resource_remediation"
                && result["resources"][0]["fields"][0]["required"] == true,
            "configured metadata discovery failed",
        )?;
        for request in self.requests().await? {
            let url = reqwest::Url::parse(&format!(
                "https://fixture{}",
                request["path"].as_str().unwrap()
            ))?;
            require(
                matches!(
                    url.path(),
                    "/api/now/table/sys_db_object" | "/api/now/table/sys_dictionary"
                ),
                "metadata allowlist escaped",
            )?;
            let query: std::collections::HashMap<_, _> = url.query_pairs().collect();
            require(
                query
                    .get("sysparm_limit")
                    .and_then(|v| v.parse::<u32>().ok())
                    .is_some_and(|v| v <= 33)
                    && query.contains_key("sysparm_fields")
                    && query
                        .get("sysparm_query")
                        .is_some_and(|v| v.starts_with("name=sn_vul_vulnerable_item")),
                "unbounded provider query",
            )?;
            require(
                request["headers"]["authorization"] == format!("Bearer {SECRET}"),
                "native credential missing",
            )?;
        }
        checks.push("bounded authenticated configured metadata reads");
        for mode in [
            "denied",
            "unauthorized",
            "failure",
            "unsupported",
            "empty",
            "wrong_shape",
            "malformed",
            "oversize",
            "redirect",
            "truncated",
        ] {
            self.control(json!({"mode":mode,"clear_requests":true})).await?;
            let (code, result) = self.discover().await?;
            require(
                code == 200
                    && result["constraints"]["status"] == "unknown"
                    && result["resources"][0]["status"] == "unknown"
                    && result["resources"][0]["code"].as_str().is_some_and(|v| v != "ok"),
                "provider fault treated as evidence",
            )?;
            let requests = self.requests().await?;
            require(
                requests.len() <= 2
                    && requests
                        .iter()
                        .all(|v| !v["path"].as_str().unwrap().contains("forbidden-redirect")),
                "unbounded retry or redirect",
            )?;
            checks.push(mode);
        }
        self.control(json!({"mode":"success","expected_token":"rotated"})).await?;
        require(
            self.discover().await?.1["resources"][0]["status"] == "unknown",
            "invalid provider credential accepted",
        )?;
        self.control(json!({"expected_token":SECRET})).await?;
        checks.push("provider credential authentication");
        self.config["resources"] = Value::Array((0..3).map(|i|json!({"resource_ref":format!("resource_{i}"),"table":"sn_vul_vulnerable_item","fields":[{"field_ref":format!("field_{i}"),"name":"state"}]})).collect());
        self.restart().await?;
        self.control(json!({"delay_seconds":3,"clear_requests":true})).await?;
        let started = Instant::now();
        let (first, second) = tokio::join!(self.discover(), async {
            while self.requests().await?.is_empty() && started.elapsed() < Duration::from_secs(3) {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            self.discover().await
        });
        let (code, result) = first?;
        require(second?.0 == 429, "concurrent discovery not rejected")?;
        require(
            code == 504
                && result["code"] == "run_deadline"
                && started.elapsed() >= Duration::from_secs(10)
                && started.elapsed() < Duration::from_secs(16),
            "global deadline failed",
        )?;
        self.control(json!({"delay_seconds":0})).await?;
        require(self.discover().await?.0 == 200, "deadline leaked concurrency permit")?;
        checks.push("global deadline and concurrency permit release");
        self.config["resources"] = json!([]);
        self.restart().await?;
        self.control(json!({"mode":"denied"})).await?;
        let (code, result) = self.discover().await?;
        require(
            code == 200
                && result["code"] == "access_denied"
                && result["constraints"]["status"] == "unknown",
            "inventory denial treated as absence",
        )?;
        self.control(json!({"mode":"success"})).await?;
        checks.push("unseeded discovery access denial");
        self.redacted(&fs::read_to_string(self.directory.path().join("workload.log"))?)?;
        checks.push("workload log redaction");
        Ok(json!({"suite":"rust-integration-discovery","passed":true,"checks":checks}))
    }
    pub async fn serve(&mut self) -> Result<()> {
        let handoff = self.directory.path().join("agent.env");
        private_write(&handoff, &format!("export IPE_INTEGRATION_TOKEN='{}'\n", self.token))?;
        println!("Local workload: {}\nOperator-only token file: {}\nConnection: conn_acceptance\nCtrl-C tears down the deployment.", self.url, handoff.display());
        tokio::select! { result = tokio::signal::ctrl_c() => { result?; }, result = self.workload.as_mut().unwrap().0.wait_async() => { result?; return Err("workload exited".into()); } }
        Ok(())
    }
}
// Keep child lifecycle synchronous and bounded; use polling for the interactive demo.
trait WaitAsync {
    async fn wait_async(&mut self) -> std::io::Result<()>;
}
impl WaitAsync for Child {
    async fn wait_async(&mut self) -> std::io::Result<()> {
        loop {
            if self.try_wait()?.is_some() {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}
fn private_write(path: &Path, text: &str) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.create(true).truncate(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)?.write_all(text.as_bytes())?;
    Ok(())
}
