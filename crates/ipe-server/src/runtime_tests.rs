use super::*;

fn scope() -> Scope {
    Scope::new("tenant", "prod")
}

fn input_schema() -> Schema {
    Schema::Record {
        fields: BTreeMap::from([(
            "enabled".into(),
            ipe_core::runtime_facts::FieldSchema { schema: Schema::Bool, optional: false },
        )]),
    }
}

fn compile(source: &str) -> CompiledPolicy {
    let env = CompileEnvironment::new(input_schema()).collection("evidence", Schema::Bool);
    CompiledPolicy::compile(source, &env).unwrap()
}

fn allow_policy() -> CompiledPolicy {
    compile(
        "language 2\npolicy Allow: \"allow\"\ntriggers when input.enabled == true\nrequires true\n",
    )
}

fn deny_policy() -> CompiledPolicy {
    compile(
        "language 2\npolicy Deny: \"deny\"\ntriggers when input.enabled == true\nrequires false\n",
    )
}

fn missing_fact_policy(trigger: &str) -> CompiledPolicy {
    compile(&format!("language 2\npolicy Missing: \"missing\"\ntriggers when {trigger}\nfacts\n  evidence = data.evidence[\"key\"]\nrequires evidence.value == true\n"))
}

fn state(policies: Vec<CompiledPolicy>) -> Arc<AppState> {
    let store = FactStore::memory();
    store
        .register_collection(
            scope(),
            "evidence",
            CollectionConfig {
                schema: Schema::Bool,
                max_ttl_seconds: 600,
                max_observation_age_seconds: 600,
                source_id: "publisher".into(),
                mode: IngestionMode::Push,
            },
        )
        .unwrap();
    let write_token = "w".repeat(32);
    let evaluate_token = "e".repeat(32);
    let credentials = vec![
        Credential {
            config: CredentialConfig {
                token_env: "unused".into(),
                scope: scope(),
                source_id: Some("publisher".into()),
                write_collections: vec!["evidence".into()],
                evaluate: false,
                inspect: false,
                manage_policies: false,
            },
            token: write_token.into_bytes(),
        },
        Credential {
            config: CredentialConfig {
                token_env: "unused".into(),
                scope: scope(),
                source_id: None,
                write_collections: vec![],
                evaluate: true,
                inspect: false,
                manage_policies: false,
            },
            token: evaluate_token.into_bytes(),
        },
    ];
    Arc::new(AppState {
        store,
        input_schema: input_schema(),
        credentials,
        policies: BTreeMap::from([(scope(), policies)]),
        policy_catalog: PolicyCatalog::memory(),
        compile_environments: BTreeMap::from([(
            scope(),
            CompileEnvironment::new(input_schema()).collection("evidence", Schema::Bool),
        )]),
        connectors: vec![],
        clock: Clock::new(),
    })
}

fn auth(token: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(axum::http::header::AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
    headers
}

async fn evaluate_value(state: &Arc<AppState>, input: Value) -> Result<Value, StatusCode> {
    let body = Bytes::from(json!({"input":input}).to_string());
    super::evaluate(
        State(Arc::clone(state)),
        Path(("tenant".into(), "prod".into())),
        auth(&"e".repeat(32)),
        body,
    )
    .await
    .map(|result| result.0)
    .map_err(|error| error.0)
}

#[tokio::test]
async fn compiled_policies_combine_allow_deny_indeterminate_and_not_applicable() {
    let input = json!({"enabled":true});
    let app = state(vec![
        allow_policy(),
        deny_policy(),
        missing_fact_policy("input.enabled == false"),
    ]);
    let result = evaluate_value(&app, input.clone()).await.unwrap();
    assert_eq!(result["decision"], "deny");
    assert_eq!(result["evaluation_status"], "deny");
    assert_eq!(result["policies"][2]["status"], "not_applicable");

    let app = state(vec![allow_policy(), missing_fact_policy("input.enabled == true")]);
    let result = evaluate_value(&app, input.clone()).await.unwrap();
    assert_eq!(result["decision"], "deny");
    assert_eq!(result["evaluation_status"], "indeterminate");

    let app = state(vec![allow_policy(), missing_fact_policy("input.enabled == false")]);
    let result = evaluate_value(&app, input).await.unwrap();
    assert_eq!(result["decision"], "allow");
    assert_eq!(result["evaluation_status"], "allow");
}

#[tokio::test]
async fn missing_evidence_after_true_trigger_is_indeterminate_and_false_trigger_skips_it() {
    let app = state(vec![missing_fact_policy("input.enabled == true")]);
    let result = evaluate_value(&app, json!({"enabled":true})).await.unwrap();
    assert_eq!(result["evaluation_status"], "indeterminate");
    assert_eq!(result["policies"][0]["status"], "indeterminate");

    let result = evaluate_value(&app, json!({"enabled":false})).await.unwrap();
    assert_eq!(result["policies"][0]["status"], "not_applicable");
    assert_eq!(result["evaluation_status"], "deny");
}

#[tokio::test]
async fn input_schema_is_enforced_at_the_evaluation_boundary() {
    let app = state(vec![allow_policy()]);
    let error = evaluate_value(&app, json!({"unexpected":true})).await.unwrap_err();
    assert_eq!(error, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn write_only_publisher_can_inspect_its_fact_without_receiving_the_value() {
    let app = state(vec![allow_policy()]);
    let now = chrono::Utc::now().timestamp();
    let request = json!({"mutations":[{"operation":"put","key":"secret","value":true,"expected_revision":null,"source_revision":1,"observed_at":now,"ttl_seconds":60}]});
    let committed = super::commit(
        State(Arc::clone(&app)),
        Path(("tenant".into(), "prod".into(), "evidence".into())),
        auth(&"w".repeat(32)),
        Bytes::from(request.to_string()),
    )
    .await
    .map_err(|error| error.0)
    .unwrap();
    assert_eq!(committed.0["records"].as_array().unwrap().len(), 1);

    let inspected = super::inspect_fact(
        State(Arc::clone(&app)),
        Path(("tenant".into(), "prod".into(), "evidence".into(), "secret".into())),
        auth(&"w".repeat(32)),
    )
    .await
    .map_err(|error| error.0)
    .unwrap();
    assert_eq!(inspected.0["status"], "found");
    assert_eq!(inspected.0["revision"], 1);
    assert!(inspected.0.get("record").is_none());
}

fn policy_document(source: &str) -> PolicyDocument {
    serde_json::from_value(json!({
        "id":"managed", "title":"Managed policy", "external":null,
        "data":{"schema":{"type":"int"},"value":2},
        "sections":[{"id":"requirement","title":"Requirement","source_pointer":null,
            "rule":{"kind":"modeled","source":source}}]
    }))
    .unwrap()
}
fn management_state() -> Arc<AppState> {
    let mut app = state(vec![allow_policy()]);
    let state = Arc::get_mut(&mut app).unwrap();
    state.credentials.push(Credential {
        config: CredentialConfig {
            token_env: "manager".into(),
            scope: scope(),
            source_id: None,
            write_collections: vec![],
            evaluate: true,
            inspect: true,
            manage_policies: true,
        },
        token: "m".repeat(32).into_bytes(),
    });
    app
}
async fn publish_policy(
    app: &Arc<AppState>,
    token: &str,
    document: &PolicyDocument,
    expected: Option<u64>,
) -> Result<Value, StatusCode> {
    publish_document(
        State(app.clone()),
        Path(("tenant".into(), "prod".into(), "managed".into())),
        auth(token),
        Bytes::from(json!({"document":document,"expected_revision":expected}).to_string()),
    )
    .await
    .map(|v| v.0)
    .map_err(|e| e.0)
}

#[tokio::test]
async fn policy_management_grants_are_separate_and_revisions_affect_execution() {
    let app = management_state();
    let deny = policy_document(
        "language 2 policy Managed: \"managed\" triggers when true requires policy == 3",
    );
    for token in ["w".repeat(32), "e".repeat(32)] {
        assert_eq!(publish_policy(&app, &token, &deny, None).await, Err(StatusCode::FORBIDDEN));
    }
    assert_eq!(
        evaluate_value(&app, json!({"enabled":true})).await.unwrap()["decision"],
        "allow"
    );
    let revision = publish_policy(&app, &"m".repeat(32), &deny, None).await.unwrap();
    assert_eq!(revision["revision"], 1);
    assert_eq!(revision["publisher"], "manager");
    let result = evaluate_value(&app, json!({"enabled":true})).await.unwrap();
    assert_eq!(result["decision"], "deny");
    assert_eq!(result["policy_documents"][0]["revision"], 1);
    assert_eq!(result["policy_documents"][0]["document_digest"], revision["document_digest"]);
    let mut allow = deny.clone();
    allow.data.value = json!(3);
    assert_eq!(
        publish_policy(&app, &"m".repeat(32), &allow, None).await,
        Err(StatusCode::CONFLICT)
    );
    publish_policy(&app, &"m".repeat(32), &allow, Some(1)).await.unwrap();
    assert_eq!(
        evaluate_value(&app, json!({"enabled":true})).await.unwrap()["decision"],
        "allow"
    );
}

#[tokio::test]
async fn candidate_testing_requires_inspection_and_never_publishes() {
    let app = management_state();
    let candidate = policy_document(
        "language 2 policy Candidate: \"candidate\" triggers when true requires false",
    );
    let body = Bytes::from(json!({"document":candidate,"input":{"enabled":true}}).to_string());
    let denied = test_document(
        State(app.clone()),
        Path(("tenant".into(), "prod".into())),
        auth(&"e".repeat(32)),
        body.clone(),
    )
    .await;
    assert_eq!(denied.err().unwrap().0, StatusCode::FORBIDDEN);
    let result = test_document(
        State(app.clone()),
        Path(("tenant".into(), "prod".into())),
        auth(&"m".repeat(32)),
        body,
    )
    .await
    .unwrap()
    .0;
    assert_eq!(result["simulation"], true);
    assert_eq!(result["decision"], "deny");
    assert!(app.policy_catalog.active(&scope()).is_empty());
    assert_eq!(
        evaluate_value(&app, json!({"enabled":true})).await.unwrap()["decision"],
        "allow"
    );
}

#[tokio::test]
async fn expired_import_cannot_be_hidden_by_bootstrap_allow() {
    let app = management_state();
    let now = app.clock.now().unwrap();
    let mut document = policy_document(
        "language 2 policy External: \"external\" triggers when true requires true",
    );
    document.external = Some(ipe_core::policy_document::ExternalPolicy {
        system: "forge".into(),
        locator: "https://forge.example/rules/1".into(),
        source_revision: "v1".into(),
        source_digest: "a".repeat(64),
        observed_at: now - 10,
        valid_until: now,
    });
    document.sections[0].source_pointer = Some("/rules/0".into());
    app.policy_catalog
        .publish(
            scope(),
            document,
            None,
            "agent".into(),
            now - 1,
            &compile_environment(&app, &scope()),
        )
        .unwrap();
    let result = evaluate_value(&app, json!({"enabled":true})).await.unwrap();
    assert_eq!(result["decision"], "deny");
    assert_eq!(result["evaluation_status"], "indeterminate");
}

#[tokio::test]
async fn imports_cannot_bind_collections_removed_from_bootstrap() {
    let mut app = management_state();
    Arc::get_mut(&mut app).unwrap().compile_environments.clear();
    // The persisted fact schema remains, but is no longer a configured dependency.
    assert!(app.store.snapshot(scope()).schema("evidence").is_ok());
    let document = policy_document("language 2 policy Old: \"old\" triggers when true facts e = data.evidence[\"key\"] requires e.value == true");
    assert_eq!(
        publish_policy(&app, &"m".repeat(32), &document, None).await,
        Err(StatusCode::UNPROCESSABLE_ENTITY)
    );
}
