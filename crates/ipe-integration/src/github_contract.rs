//! Local, read-only GitHub capture. Source text never executes; only the bundled
//! projection adapter executes in the existing bounded QuickJS host.
use crate::script::ScriptAdapter;
use reqwest::{header, Client, Url};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FILE_LIMIT: usize = 512 * 1024;
const YAML_LIMIT: usize = 48 * 1024;
const TOTAL_LIMIT: usize = 16 * 1024 * 1024;
const REQUEST_LIMIT: usize = 96;
const DOCUMENT_LIMIT: usize = 24;
const MAX_DEPTH: usize = 4;
const ADAPTER: &str = include_str!("../adapters/github-contract/project.js");

type Result<T> = std::result::Result<T, &'static str>;

pub struct CaptureOptions {
    pub repository: String,
    pub revision: String,
    pub workflow: String,
    pub allowed_repositories: Vec<String>,
}

struct Reader {
    client: Client,
    token: Option<SecretString>,
    allowed: HashSet<String>,
    requests: usize,
    bytes: usize,
    deadline: Instant,
    revisions: HashMap<(String, String), String>,
}

fn repository(value: &str) -> bool {
    let parts: Vec<_> = value.split('/').collect();
    parts.len() == 2
        && parts.iter().all(|p| {
            !p.is_empty()
                && *p != "."
                && *p != ".."
                && p.len() <= 100
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}
fn path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && value.split('/').all(|p| {
            !p.is_empty()
                && p != "."
                && p != ".."
                && p.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
}
fn revision(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && !value.contains("..")
        && value.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_./".contains(&b))
}
fn sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}
fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

impl Reader {
    fn new(options: &CaptureOptions) -> Result<Self> {
        if !repository(&options.repository)
            || !revision(&options.revision)
            || !path(&options.workflow)
            || !options.workflow.starts_with(".github/workflows/")
            || options.allowed_repositories.len() > 32
            || options.allowed_repositories.iter().any(|r| !repository(r))
        {
            return Err("invalid_capture_scope");
        }
        let mut allowed: HashSet<_> =
            options.allowed_repositories.iter().map(|s| s.to_ascii_lowercase()).collect();
        allowed.insert(options.repository.to_ascii_lowercase());
        Ok(Self {
            client: Client::builder()
                .https_only(true)
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(15))
                .user_agent("ipe-github-contract/0.1")
                .build()
                .map_err(|_| "http_client_failed")?,
            token: std::env::var("IPE_GITHUB_TOKEN").ok().map(SecretString::from),
            allowed,
            requests: 0,
            bytes: 0,
            deadline: Instant::now() + Duration::from_secs(180),
            revisions: HashMap::new(),
        })
    }
    fn authorize(&self, repo: &str) -> Result<()> {
        if !repository(repo) || !self.allowed.contains(&repo.to_ascii_lowercase()) {
            return Err("repository_not_permitted");
        }
        Ok(())
    }
    async fn get(
        &mut self,
        repo: &str,
        parts: &[&str],
        query: &[(&str, &str)],
        raw: bool,
    ) -> Result<Vec<u8>> {
        self.authorize(repo)?;
        if self.requests >= REQUEST_LIMIT
            || Instant::now() >= self.deadline
            || self.bytes >= TOTAL_LIMIT
        {
            return Err("capture_budget_exhausted");
        }
        self.requests += 1;
        let mut url = Url::parse("https://api.github.com/").map_err(|_| "invalid_source")?;
        {
            let mut segments = url.path_segments_mut().map_err(|_| "invalid_source")?;
            segments.clear().push("repos");
            for part in repo.split('/') {
                segments.push(part);
            }
            for part in parts {
                segments.push(part);
            }
        }
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let mut request = self.client.get(url).header("X-GitHub-Api-Version", "2022-11-28").header(
            header::ACCEPT,
            if raw { "application/vnd.github.raw+json" } else { "application/vnd.github+json" },
        );
        if let Some(token) = &self.token {
            let mut value =
                header::HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
                    .map_err(|_| "invalid_credential")?;
            value.set_sensitive(true);
            request = request.header(header::AUTHORIZATION, value);
        }
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        let response = tokio::time::timeout(remaining, request.send())
            .await
            .map_err(|_| "capture_deadline")?
            .map_err(|_| "source_transport_failed")?;
        self.body(response).await
    }
    async fn body(&mut self, mut response: reqwest::Response) -> Result<Vec<u8>> {
        match response.status().as_u16() {
            200 => {},
            401 | 403 => return Err("source_access_denied_or_rate_limited"),
            404 => return Err("source_not_found"),
            429 => return Err("source_rate_limited"),
            300..=399 => return Err("source_redirect_rejected"),
            _ => return Err("source_http_failed"),
        }
        if response.content_length().is_some_and(|n| n > FILE_LIMIT as u64) {
            return Err("source_byte_limit");
        }
        let mut data = Vec::new();
        loop {
            let remaining = self.deadline.saturating_duration_since(Instant::now());
            let chunk = tokio::time::timeout(remaining, response.chunk())
                .await
                .map_err(|_| "capture_deadline")?
                .map_err(|_| "source_transport_failed")?;
            let Some(chunk) = chunk else { break };
            self.bytes = self.bytes.saturating_add(chunk.len());
            if self.bytes > TOTAL_LIMIT || data.len() + chunk.len() > FILE_LIMIT {
                return Err("source_byte_limit");
            }
            data.extend_from_slice(&chunk);
        }
        Ok(data)
    }
    async fn resolve(&mut self, repo: &str, reference: &str) -> Result<String> {
        self.authorize(repo)?;
        if !revision(reference) {
            return Err("invalid_source_ref");
        }
        if sha(reference) {
            return Ok(reference.to_ascii_lowercase());
        }
        let key = (repo.to_owned(), reference.to_owned());
        if let Some(commit) = self.revisions.get(&key) {
            return Ok(commit.clone());
        }
        let data = self.get(repo, &["commits", reference], &[], false).await?;
        let value: Value = ipe_json::parse(&data).map_err(|_| "invalid_commit_response")?;
        let commit = value["sha"]
            .as_str()
            .filter(|s| sha(s))
            .ok_or("invalid_commit_response")?
            .to_owned();
        self.revisions.insert(key, commit.clone());
        Ok(commit)
    }
    async fn file(&mut self, repo: &str, commit: &str, name: &str) -> Result<String> {
        if !sha(commit) || !path(name) {
            return Err("invalid_source_path");
        }
        let mut parts = vec!["contents"];
        parts.extend(name.split('/'));
        String::from_utf8(self.get(repo, &parts, &[("ref", commit)], true).await?)
            .map_err(|_| "source_not_utf8")
    }
}

// Use parser events rather than an expanding YAML loader. Bound depth and nodes
// before materialization; reject aliases, tags, merge keys, and duplicate keys.
fn parse_yaml_located(source: &str) -> Result<(Value, Value)> {
    use yaml_rust2::{
        parser::{Event, Parser},
        scanner::TScalarStyle,
        Yaml,
    };
    if source.len() > YAML_LIMIT {
        return Err("yaml_byte_limit");
    }
    fn node(
        parser: &mut Parser<std::str::Chars<'_>>,
        depth: usize,
        nodes: &mut usize,
        pointer: &str,
        locations: &mut serde_json::Map<String, Value>,
    ) -> Result<Value> {
        *nodes += 1;
        if depth > 24 || *nodes > 8192 {
            return Err("yaml_structure_limit");
        }
        let (event, mark) = parser.next_token().map_err(|_| "invalid_yaml")?;
        locations
            .entry(pointer.to_owned())
            .or_insert_with(|| json!({"line":mark.line(),"column":mark.col()+1}));
        match event {
            Event::Scalar(text, style, 0, None) => {
                if style != TScalarStyle::Plain {
                    return Ok(json!(text));
                }
                Ok(match Yaml::from_str(&text) {
                    Yaml::String(v) => json!(v),
                    Yaml::Integer(v) => json!(v),
                    Yaml::Boolean(v) => json!(v),
                    Yaml::Null => Value::Null,
                    Yaml::Real(v) => {
                        let n = v.parse::<f64>().map_err(|_| "invalid_yaml_number")?;
                        if !n.is_finite() {
                            return Err("invalid_yaml_number");
                        }
                        json!(n)
                    },
                    _ => return Err("unsupported_yaml_scalar"),
                })
            },
            Event::MappingStart(0, None) => {
                let mut object = serde_json::Map::new();
                loop {
                    if matches!(parser.peek().map_err(|_| "invalid_yaml")?.0, Event::MappingEnd) {
                        parser.next_token().map_err(|_| "invalid_yaml")?;
                        break;
                    }
                    let key = node(parser, depth + 1, nodes, pointer, locations)?;
                    let key = key.as_str().ok_or("yaml_non_string_key")?.to_owned();
                    if key == "<<" {
                        return Err("yaml_merge_unsupported");
                    }
                    if object.contains_key(&key) {
                        return Err("yaml_duplicate_key");
                    }
                    let child = format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1"));
                    object.insert(key, node(parser, depth + 1, nodes, &child, locations)?);
                }
                Ok(Value::Object(object))
            },
            Event::SequenceStart(0, None) => {
                let mut values = Vec::new();
                loop {
                    if matches!(parser.peek().map_err(|_| "invalid_yaml")?.0, Event::SequenceEnd) {
                        parser.next_token().map_err(|_| "invalid_yaml")?;
                        break;
                    }
                    values.push(node(
                        parser,
                        depth + 1,
                        nodes,
                        &format!("{pointer}/{}", values.len()),
                        locations,
                    )?);
                }
                Ok(Value::Array(values))
            },
            Event::Alias(_) => Err("yaml_alias_unsupported"),
            Event::Scalar(_, _, _, _) | Event::MappingStart(_, _) | Event::SequenceStart(_, _) => {
                Err("yaml_anchor_or_tag_unsupported")
            },
            _ => Err("invalid_yaml"),
        }
    }
    let mut parser = Parser::new_from_str(source);
    for expected in [Event::StreamStart, Event::DocumentStart] {
        if parser.next_token().map_err(|_| "invalid_yaml")?.0 != expected {
            return Err("yaml_document_required");
        }
    }
    let mut locations = serde_json::Map::new();
    let result = node(&mut parser, 0, &mut 0, "", &mut locations)?;
    for expected in [Event::DocumentEnd, Event::StreamEnd] {
        if parser.next_token().map_err(|_| "invalid_yaml")?.0 != expected {
            return Err("single_yaml_document_required");
        }
    }
    if !result.is_object() {
        return Err("yaml_object_required");
    }
    if serde_json::to_vec(&result).map_err(|_| "invalid_yaml")?.len() > 60 * 1024 {
        return Err("yaml_projection_limit");
    }
    Ok((result, Value::Object(locations)))
}
#[cfg(test)]
fn parse_yaml(source: &str) -> Result<Value> {
    parse_yaml_located(source).map(|(value, _)| value)
}

async fn project(adapter: &ScriptAdapter, kind: &str, document: Value) -> Result<Value> {
    if kind == "workflow" && !document["jobs"].is_object() {
        return Err("workflow_jobs_required");
    }
    if kind == "action" && !document["runs"].is_object() {
        return Err("action_runs_required");
    }
    let objects: Vec<&Value> = if kind == "workflow" {
        document["jobs"].as_object().ok_or("workflow_jobs_required")?.values().collect()
    } else {
        vec![&document["runs"]]
    };
    for object in objects {
        if !object.is_object() {
            return Err("invalid_declaration_shape");
        }
        if let Some(steps) = object.get("steps") {
            let steps = steps.as_array().ok_or("invalid_steps_shape")?;
            if steps.iter().any(|s| !s.is_object()) {
                return Err("invalid_steps_shape");
            }
        }
    }
    let context = json!({"connection_ref":"github_capture","sdk_version":crate::script::SDK_VERSION,"operations":[{"name":"captured_document","parameters":[]}]});
    let request = adapter.step(context.clone(), json!([]), Value::Null).await?;
    if request.requests.len() != 1
        || request.requests[0].operation != "captured_document"
        || !request.requests[0].params.is_empty()
    {
        return Err("projection_operation_forbidden");
    }
    let response = json!([{"id":request.requests[0].id,"operation":"captured_document","receipt":digest(serde_json::to_string(&document).map_err(|_| "projection_failed")?.as_bytes()),"data":[{"kind":kind,"document":document}]}]);
    let output = adapter.step(context, response, request.state).await?;
    output.outputs.ok_or("projection_incomplete")
}

#[derive(Clone, Debug)]
struct Pending {
    repo: String,
    reference: String,
    file: String,
    kind: String,
    depth: usize,
    parent: Option<usize>,
    pointer: String,
}

fn reference_target(
    uses: &str,
    kind: &str,
    current: &Pending,
    commit: &str,
    pointer: &str,
) -> Result<Pending> {
    if uses.contains("${{") {
        return Err("dynamic_reference_unresolved");
    }
    if uses.starts_with("docker://") {
        return Err("container_image_not_inspected");
    }
    let (repo, reference, file) = if let Some(local) = uses.strip_prefix("./") {
        // Local action references resolve in the checkout workspace, which can
        // differ from the workflow repository. Reusable workflows use the source repo.
        if kind == "action" {
            return Err("workspace_action_reference_unresolved");
        }
        if !path(local) {
            return Err("invalid_source_path");
        }
        (current.repo.clone(), commit.to_owned(), local.to_owned())
    } else {
        let (location, reference) = uses.rsplit_once('@').ok_or("invalid_action_reference")?;
        let parts: Vec<_> = location.split('/').collect();
        if parts.len() < 2 || !revision(reference) {
            return Err("invalid_action_reference");
        }
        let repo = parts[..2].join("/");
        if !repository(&repo) {
            return Err("invalid_action_reference");
        }
        (repo, reference.to_owned(), parts[2..].join("/"))
    };
    let file = if kind == "action" {
        if file.is_empty() {
            "action.yml".into()
        } else {
            format!("{file}/action.yml")
        }
    } else {
        file
    };
    if !path(&file) || (kind == "workflow" && !file.starts_with(".github/workflows/")) {
        return Err("invalid_source_path");
    }
    Ok(Pending {
        repo,
        reference,
        file,
        kind: kind.into(),
        depth: current.depth + 1,
        parent: None,
        pointer: pointer.into(),
    })
}

fn source(repo: &str, commit: &str, file: &str, text: &str) -> Value {
    json!({"repository":repo,"commit":commit,"path":file,"digest":digest(text.as_bytes()),"bytes":text.len(),"url":format!("https://github.com/{repo}/blob/{commit}/{file}")})
}

async fn implementation(
    reader: &mut Reader,
    pending: &Pending,
    commit: &str,
    model: &Value,
) -> Vec<Value> {
    let base = pending.file.rsplit_once('/').map_or("", |(base, _)| base);
    let join = |s: &str| if base.is_empty() { s.to_owned() } else { format!("{base}/{s}") };
    let mut files = Vec::new();
    for key in ["pre", "main", "post", "entrypoint", "pre-entrypoint", "post-entrypoint"] {
        if let Some(file) = model["runs"][key].as_str() {
            let file = file.strip_prefix("./").unwrap_or(file);
            if path(file) {
                files.push((join(file), "declared_entrypoint"));
            }
        }
    }
    if let Some(image) = model["runs"]["image"]
        .as_str()
        .filter(|s| !s.starts_with("docker://") && path(s))
    {
        files.push((join(image), "container_recipe"));
    }
    // Source files are discovered through the pinned tree, never guessed from
    // action names. They provide review clues, not a verified executable trace.
    let tree = reader
        .get(&pending.repo, &["git", "trees", commit], &[("recursive", "1")], false)
        .await;
    let mut evidence = Vec::new();
    match tree {
        Ok(bytes) => match ipe_json::parse::<Value>(&bytes) {
            Ok(tree) => {
                if tree["truncated"] == true {
                    evidence.push(json!({"status":"unknown","code":"source_tree_truncated"}));
                }
                if let Some(entries) = tree["tree"].as_array() {
                    let prefix = join("src/");
                    let mut candidates: Vec<_> = entries
                        .iter()
                        .filter(|e| e["type"] == "blob")
                        .filter_map(|e| e["path"].as_str())
                        .filter(|p| {
                            p.starts_with(&prefix)
                                && !p.contains(".test.")
                                && !p.contains("__tests__")
                                && (p.ends_with(".ts") || p.ends_with(".js"))
                        })
                        .collect();
                    let runtime_scope = model["runs"]["main"]
                        .as_str()
                        .and_then(|p| p.strip_prefix("dist/"))
                        .and_then(|p| {
                            p.rsplit_once('/').map(|(scope, _)| join(&format!("src/{scope}/")))
                        });
                    candidates.sort_by_key(|p| {
                        (
                            runtime_scope.as_ref().is_some_and(|scope| !p.starts_with(scope)),
                            !p.ends_with("/main.ts")
                                && !p.ends_with("/main.js")
                                && !p.ends_with("/index.ts")
                                && !p.ends_with("/index.js")
                                && !p.ends_with("/setup-python.ts")
                                && !p.ends_with("/upload-inputs.ts")
                                && !p.ends_with("/input-helper.ts"),
                            *p,
                        )
                    });
                    let count = candidates.len();
                    files.extend(
                        candidates
                            .into_iter()
                            .take(3)
                            .map(|p| (p.to_owned(), "author_source_clue")),
                    );
                    if count > 3 {
                        evidence.push(json!({"status":"partial","code":"author_source_sample","available_files":count,"selected_files":3}));
                    }
                    if let Some(license) = entries
                        .iter()
                        .filter(|e| e["type"] == "blob")
                        .filter_map(|e| e["path"].as_str())
                        .find(|p| {
                            ["license", "license.md", "license.txt"]
                                .contains(&p.to_ascii_lowercase().as_str())
                        })
                    {
                        files.push((license.to_owned(), "license_text"));
                    } else {
                        evidence.push(json!({"status":"unknown","code":"license_not_captured"}));
                    }
                    if entries
                        .iter()
                        .any(|e| e["path"].as_str() == Some(join("package.json").as_str()))
                    {
                        files.push((join("package.json"), "dependency_manifest"));
                    }
                }
            },
            Err(_) => evidence.push(json!({"status":"unknown","code":"invalid_tree_response"})),
        },
        Err(code) => evidence.push(json!({"status":"unknown","code":code})),
    }
    let mut seen = HashSet::new();
    if files.len() > 12 {
        evidence.push(json!({"status":"partial","code":"implementation_file_limit"}));
    }
    for (file, role) in files.into_iter().filter(|(file, _)| seen.insert(file.clone())).take(12) {
        match reader.file(&pending.repo, commit, &file).await {
            Ok(text) => {
                let clues: Vec<_> = text.lines().enumerate().filter(|(_,line)| ["getInput(","getBooleanInput(","setOutput(","getOctokit(","exec(","fetch(","process.env","saveState(","getState("].iter().any(|marker| line.contains(marker)))
                    .take(32).map(|(i,line)| json!({"line":i+1,"text":line.chars().take(400).collect::<String>(),"interpretation":"lexical_clue_only"})).collect();
                let excerpt: String = text.chars().take(12000).collect();
                evidence.push(json!({"status":"captured","role":role,"source":source(&pending.repo,commit,&file,&text),"excerpt_truncated":excerpt.len()<text.len(),"excerpt":excerpt,"clues":clues}));
            },
            Err(code) => {
                evidence.push(json!({"status":"unknown","role":role,"path":file,"code":code}))
            },
        }
    }
    evidence
}

// Bind advertised inputs to each call site. This records declarations only;
// action code can override defaults or enforce inputs differently at runtime.
fn input_bindings(parent: &Value, child: &Value, pointer: &str) -> Value {
    let call = parent["projection"]["references"]
        .as_array()
        .and_then(|refs| refs.iter().find(|r| r["pointer"] == pointer));
    let supplied = call.map(|c| &c["supplied"]).unwrap_or(&Value::Null);
    let model = &child["projection"]["model"];
    let declared = if child["kind"] == "workflow" {
        &model["on"]["workflow_call"]["inputs"]
    } else {
        &model["inputs"]
    };
    let mut bindings = Vec::new();
    if let Some(inputs) = declared.as_object() {
        for (name, spec) in inputs {
            let (origin, value) = if !supplied.is_object() {
                ("unresolved", Value::Null)
            } else if let Some(value) = supplied.get(name) {
                ("caller", value.clone())
            } else if let Some(value) = spec.get("default") {
                ("declared_default", value.clone())
            } else {
                ("unspecified", Value::Null)
            };
            bindings.push(json!({"input":name,"origin":origin,"value":value,"declared_required":spec.get("required"),"runtime_validation":"not_observed"}));
        }
    }
    let undeclared: Vec<_> = supplied
        .as_object()
        .map(|values| {
            values
                .keys()
                .filter(|key| declared.get(key.as_str()).is_none())
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    json!({"inputs":bindings,"undeclared_caller_inputs":undeclared,"supplied":supplied,"status":"declarations_only"})
}

pub async fn capture(options: CaptureOptions) -> Result<Value> {
    let mut reader = Reader::new(&options)?;
    let adapter = ScriptAdapter::from_source(ADAPTER, false)?;
    let observed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "clock_failed")?
        .as_secs();
    let mut queue = VecDeque::from([Pending {
        repo: options.repository.clone(),
        reference: options.revision.clone(),
        file: options.workflow.clone(),
        kind: "workflow".into(),
        depth: 0,
        parent: None,
        pointer: String::new(),
    }]);
    let mut documents = Vec::new();
    let mut edges = Vec::new();
    let mut gaps = Vec::new();
    let mut visited = HashMap::new();
    while let Some(mut item) = queue.pop_front() {
        if documents.len() >= DOCUMENT_LIMIT || item.depth > MAX_DEPTH {
            gaps.push(json!({"parent":item.parent,"pointer":item.pointer,"code":"dependency_capture_limit"}));
            continue;
        }
        let commit = match reader.resolve(&item.repo, &item.reference).await {
            Ok(commit) => commit,
            Err(code) if item.parent.is_none() => return Err(code),
            Err(code) => {
                gaps.push(json!({"parent":item.parent,"pointer":item.pointer,"repository":item.repo,"reference":item.reference,"code":code}));
                continue;
            },
        };
        let key = (item.repo.to_ascii_lowercase(), commit.clone(), item.file.clone());
        if let Some(id) = visited.get(&key) {
            edges.push(json!({"from":item.parent,"to":id,"pointer":item.pointer}));
            continue;
        }
        let fetched = match reader.file(&item.repo, &commit, &item.file).await {
            Err("source_not_found") if item.kind == "action" => {
                item.file = format!("{}aml", item.file.trim_end_matches("ml"));
                reader.file(&item.repo, &commit, &item.file).await
            },
            other => other,
        };
        let captured = match fetched {
            Ok(text) => match parse_yaml_located(&text) {
                Ok((parsed, locations)) => project(&adapter, &item.kind, parsed)
                    .await
                    .map(|projection| (text, projection, locations)),
                Err(code) => Err(code),
            },
            Err(code) => Err(code),
        };
        let (text, projection, locations) = match captured {
            Ok(result) => result,
            Err(code) if item.parent.is_none() => return Err(code),
            Err(code) => {
                gaps.push(json!({"parent":item.parent,"pointer":item.pointer,"repository":item.repo,"commit":commit,"path":item.file,"code":code}));
                continue;
            },
        };
        let id = documents.len();
        visited.insert(key, id);
        if item.parent.is_some() {
            edges.push(json!({"from":item.parent,"to":id,"pointer":item.pointer}));
        }
        if let Some(refs) = projection["references"].as_array() {
            for r in refs {
                let target = reference_target(
                    r["uses"].as_str().unwrap_or(""),
                    r["kind"].as_str().unwrap_or(""),
                    &item,
                    &commit,
                    r["pointer"].as_str().unwrap_or(""),
                );
                match target {
                    Ok(mut target) => {
                        if queue.len() >= 128 { gaps.push(json!({"parent":id,"pointer":r["pointer"],"code":"reference_queue_limit"})); }
                        else { target.parent=Some(id); queue.push_back(target); }
                    },
                    Err(code) => gaps.push(json!({"parent":id,"pointer":r["pointer"],"reference":r["uses"],"code":code})),
                }
            }
        }
        let evidence = if item.kind == "action" {
            implementation(&mut reader, &item, &commit, &projection["model"]).await
        } else {
            vec![]
        };
        documents.push(json!({"id":id,"source":source(&item.repo,&commit,&item.file,&text),"requested_ref":item.reference,"ref_was_pinned":sha(&item.reference),"kind":item.kind,"source_text":text,"locations":locations,"projection":projection,"implementation":evidence}));
    }
    for edge in &mut edges {
        if let (Some(from), Some(to), Some(pointer)) =
            (edge["from"].as_u64(), edge["to"].as_u64(), edge["pointer"].as_str())
        {
            let bindings =
                input_bindings(&documents[from as usize], &documents[to as usize], pointer);
            edge["bindings"] = bindings;
        }
    }
    Ok(
        json!({"schema":"ipe.github-contract.v1","observed_at_unix":observed,"coverage":"partial","publication":false,
        "scope":{"repository":options.repository,"workflow":options.workflow,"requested_ref":options.revision,"allowed_repositories":reader.allowed},
        "adapter":{"digest":adapter.digest,"sdk_version":crate::script::SDK_VERSION},
        "limits":{"requests":REQUEST_LIMIT,"file_bytes":FILE_LIMIT,"yaml_bytes":YAML_LIMIT,"total_bytes":TOTAL_LIMIT,"documents":DOCUMENT_LIMIT,"depth":MAX_DEPTH,"deadline_seconds":180},
        "usage":{"requests":reader.requests,"bytes":reader.bytes},"documents":documents,"edges":edges,"gaps":gaps,
        "limitations":["Declarations are not execution outcomes.","Environment protection, repository rules, secrets, action downloads and runtime-generated behavior are not captured.","Source clues are lexical matches, not verified effects or grants.","A local file digest is provenance, not a signed external attestation.","Capture retains source text. Treat the report with the same access controls as the source repositories."]}),
    )
}

pub fn write_report(directory: &Path, report: &Value) -> Result<()> {
    std::fs::create_dir_all(directory).map_err(|_| "report_directory_failed")?;
    let bytes = serde_json::to_vec_pretty(report).map_err(|_| "report_encoding_failed")?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err("report_byte_limit");
    }
    std::fs::write(directory.join("capture.json"), bytes).map_err(|_| "report_write_failed")?;
    // Escape HTML-significant characters so captured source cannot terminate the
    // inert JSON script element. The view renders all source fields as text.
    let embedded = serde_json::to_string(report)
        .map_err(|_| "report_encoding_failed")?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");
    let standalone = include_str!("../../../tools/github-contract/index.html")
        .replace("<link rel=\"stylesheet\" href=\"style.css\">", &format!("<style>{}</style>",include_str!("../../../tools/github-contract/style.css")))
        .replace("<script src=\"view.js\"></script>", &format!("<script id=\"capture-data\" type=\"application/json\">{embedded}</script><script>{}</script>",include_str!("../../../tools/github-contract/view.js")));
    std::fs::write(directory.join("report.html"), standalone).map_err(|_| "report_write_failed")?;
    for (name, text) in [
        ("index.html", include_str!("../../../tools/github-contract/index.html")),
        ("view.js", include_str!("../../../tools/github-contract/view.js")),
        ("style.css", include_str!("../../../tools/github-contract/style.css")),
    ] {
        std::fs::write(directory.join(name), text).map_err(|_| "report_write_failed")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> Pending {
        Pending {
            repo: "owner/repo".into(),
            reference: "main".into(),
            file: ".github/workflows/ci.yml".into(),
            kind: "workflow".into(),
            depth: 0,
            parent: None,
            pointer: String::new(),
        }
    }

    #[test]
    fn source_scope_rejects_urls_traversal_and_credential_shapes() {
        for value in [
            "https://github.com/owner/repo",
            "owner/repo/extra",
            "owner/..",
            "token@github.com/repo",
            "owner/repo?token=x",
        ] {
            assert!(!repository(value));
        }
        for value in ["../secrets", "/etc/passwd", "a/../b", "a%2fb", "a?token=x", "a\\b"] {
            assert!(!path(value));
        }
        assert!(repository("actions/setup-node"));
        assert!(path(".github/workflows/ci.yml"));
        assert!(!revision("main?token=x"));
    }

    #[test]
    fn yaml_preserves_on_and_rejects_duplicate_keys_and_unsupported_shapes() {
        let value = parse_yaml("on: [push, pull_request]\njobs: {}\n").unwrap();
        assert_eq!(value["on"][0], "push");
        for source in [
            "jobs: {}\njobs: {}",
            "jobs: {x: {if: true, if: false}}",
            "[one,two]",
            "x: !custom value",
            "x: {<<: {a: b}}",
            "true: value",
            "a: 1\n---\nb: 2",
        ] {
            assert!(parse_yaml(source).is_err(), "accepted {source}");
        }
        assert!(parse_yaml(&"a".repeat(YAML_LIMIT + 1)).is_err());
        assert!(parse_yaml(&format!("a: {}0{}", "[".repeat(30), "]".repeat(30))).is_err());
    }

    #[test]
    fn references_keep_repository_commit_and_unknowns_separate() {
        let commit = "a".repeat(40);
        let p = reference_target(
            "actions/checkout@v7",
            "action",
            &root(),
            &commit,
            "/jobs/a/steps/0/uses",
        )
        .unwrap();
        assert_eq!(p.repo, "actions/checkout");
        assert_eq!(p.file, "action.yml");
        assert_eq!(p.reference, "v7");
        assert_eq!(
            reference_target("./.github/actions/test", "action", &root(), &commit, "").unwrap_err(),
            "workspace_action_reference_unresolved"
        );
        let p = reference_target("./.github/workflows/reuse.yml", "workflow", &root(), &commit, "")
            .unwrap();
        assert_eq!(p.reference, commit);
        let p = reference_target(
            "org/repo/.github/workflows/reuse.yml@main",
            "workflow",
            &root(),
            &commit,
            "",
        )
        .unwrap();
        assert_eq!(p.kind, "workflow");
        assert!(reference_target("docker://alpine:3", "action", &root(), &commit, "").is_err());
        assert!(reference_target("a/b@${{ inputs.ref }}", "action", &root(), &commit, "").is_err());
        assert!(reference_target("./../secret", "action", &root(), &commit, "").is_err());
        let mut composite = root();
        composite.kind = "action".into();
        assert_eq!(
            reference_target("./action", "action", &composite, &commit, "").unwrap_err(),
            "workspace_action_reference_unresolved"
        );
    }

    #[tokio::test]
    async fn workflow_projection_preserves_guards_permissions_and_provenance_pointers() {
        let adapter = ScriptAdapter::from_source(ADAPTER, false).unwrap();
        let document=parse_yaml("name: Deploy\non: [push]\npermissions: {contents: read}\njobs:\n  deploy:\n    needs: build\n    if: github.ref == 'refs/heads/main'\n    permissions: {pages: write}\n    environment: production\n    steps:\n      - uses: actions/checkout@v7\n      - run: echo done\n").unwrap();
        let output = project(&adapter, "workflow", document).await.unwrap();
        assert_eq!(output["coverage"], "partial");
        assert_eq!(output["model"]["jobs"][0]["needs"], "build");
        assert_eq!(output["model"]["jobs"][0]["permissions"]["pages"], "write");
        assert_eq!(output["references"][0]["pointer"], "/jobs/deploy/steps/0/uses");
        assert_eq!(output["expressions"][0]["status"], "unresolved");
        assert!(output["model"]["jobs"][0].get("outcome").is_none());
    }

    #[tokio::test]
    async fn action_projection_preserves_composite_steps_and_lifecycle() {
        let adapter = ScriptAdapter::from_source(ADAPTER, false).unwrap();
        let document=parse_yaml("name: Composite\ninputs:\n  token:\n    required: true\nruns:\n  using: composite\n  steps:\n    - uses: actions/setup-node@v6\n    - shell: bash\n      run: echo '${{ inputs.token }}'\n").unwrap();
        let result = project(&adapter, "action", document).await.unwrap();
        assert_eq!(result["model"]["inputs"]["token"]["required"], true);
        assert_eq!(result["references"][0]["uses"], "actions/setup-node@v6");
        assert_eq!(result["expressions"][0]["pointer"], "/runs/steps/1/run");
        let result=project(&adapter,"action",parse_yaml("runs: {using: node24, main: dist/main.js, post: dist/cleanup.js, post-if: always()}\n").unwrap()).await.unwrap();
        assert_eq!(result["model"]["runs"]["post"], "dist/cleanup.js");
    }

    #[test]
    fn locations_bind_conditions_to_exact_source_lines() {
        let (_, locations) = parse_yaml_located(
            "name: Example\non: push\njobs:\n  build:\n    if: github.ref == 'refs/heads/main'\n",
        )
        .unwrap();
        assert_eq!(locations["/jobs/build/if"]["line"], 5);
        for source in ["a: &x [1]\nb: *x", "a: [1,2]\na: [3]"] {
            assert!(parse_yaml(source).is_err());
        }
    }

    #[test]
    fn input_bindings_distinguish_caller_default_and_absence() {
        let parent = json!({"projection":{"references":[{"pointer":"/steps/0/uses","supplied":{"mode":"fast","extra":true}}]}});
        let child = json!({"kind":"action","projection":{"model":{"inputs":{"mode":{"default":"slow"},"token":{"default":"${{ github.token }}"},"path":{"required":true}}}}});
        let result = input_bindings(&parent, &child, "/steps/0/uses");
        let inputs = result["inputs"].as_array().unwrap();
        assert_eq!(inputs.iter().find(|i| i["input"] == "mode").unwrap()["origin"], "caller");
        assert_eq!(
            inputs.iter().find(|i| i["input"] == "token").unwrap()["origin"],
            "declared_default"
        );
        assert_eq!(inputs.iter().find(|i| i["input"] == "path").unwrap()["origin"], "unspecified");
        assert_eq!(result["undeclared_caller_inputs"], json!(["extra"]));
    }

    #[tokio::test]
    async fn provider_failures_and_oversized_bodies_remain_errors() {
        let options = CaptureOptions {
            repository: "owner/repo".into(),
            revision: "main".into(),
            workflow: ".github/workflows/ci.yml".into(),
            allowed_repositories: vec![],
        };
        let mut reader = Reader::new(&options).unwrap();
        for (status, code) in [
            (302, "source_redirect_rejected"),
            (403, "source_access_denied_or_rate_limited"),
            (404, "source_not_found"),
            (429, "source_rate_limited"),
            (500, "source_http_failed"),
        ] {
            let response = axum::http::Response::builder()
                .status(status)
                .body("private provider details")
                .unwrap();
            assert_eq!(reader.body(response.into()).await.unwrap_err(), code);
        }
        let response = axum::http::Response::builder()
            .status(200)
            .body(vec![b'x'; FILE_LIMIT + 1])
            .unwrap();
        assert_eq!(reader.body(response.into()).await.unwrap_err(), "source_byte_limit");
        reader.bytes = TOTAL_LIMIT;
        let response = axum::http::Response::builder().status(200).body("x").unwrap();
        assert_eq!(reader.body(response.into()).await.unwrap_err(), "source_byte_limit");
    }

    #[tokio::test]
    async fn malformed_definitions_fail_instead_of_claiming_empty_contracts() {
        let adapter = ScriptAdapter::from_source(ADAPTER, false).unwrap();
        assert_eq!(
            project(&adapter, "workflow", json!({"jobs":[]})).await.unwrap_err(),
            "workflow_jobs_required"
        );
        assert_eq!(
            project(&adapter, "action", json!({"name":"missing runs"})).await.unwrap_err(),
            "action_runs_required"
        );
    }

    #[test]
    fn standalone_report_escapes_captured_html() {
        let dir = tempfile::tempdir().unwrap();
        let report = json!({"source":"</script><script>alert(1)</script>&"});
        write_report(dir.path(), &report).unwrap();
        let html = std::fs::read_to_string(dir.path().join("report.html")).unwrap();
        assert!(!html.contains("</script><script>alert(1)"));
        assert!(html.contains("\\u003c/script\\u003e"));
        let roundtrip: Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("capture.json")).unwrap())
                .unwrap();
        assert_eq!(roundtrip, report);
    }

    #[tokio::test]
    async fn unpermitted_repository_is_rejected_before_network() {
        let options = CaptureOptions {
            repository: "owner/repo".into(),
            revision: "main".into(),
            workflow: ".github/workflows/ci.yml".into(),
            allowed_repositories: vec![],
        };
        let mut reader = Reader::new(&options).unwrap();
        assert_eq!(
            reader.resolve("other/repo", "main").await.unwrap_err(),
            "repository_not_permitted"
        );
        assert_eq!(reader.requests, 0);
        reader.requests = REQUEST_LIMIT;
        assert_eq!(
            reader.file("owner/repo", &"a".repeat(40), "action.yml").await.unwrap_err(),
            "capture_budget_exhausted"
        );
    }
}
