use super::*;
use crate::host::{model::Model, Installation, Installed};
fn fixture() -> (Store, Vec<Rule>, Request) {
    let value: Value = serde_json::from_str(include_str!(
        "../../../../testing/integration/hosted/cross-source-assessment.json"
    ))
    .unwrap();
    let mut store = Store::default();
    for (id, models) in value["models"].as_object().unwrap() {
        let records = models
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                let model: Model = serde_json::from_value(v.clone()).unwrap();
                model.validate().unwrap();
                let kind = model.kind().to_owned();
                Record {
                    key: model.key(),
                    revision_id: model.revision("fixture").unwrap(),
                    kind: kind.clone(),
                    model: Some(model),
                    data: json!({}),
                    receipts: vec![format!("receipt_{id}")],
                    source_evidence: vec![],
                    observed_at: 90,
                    expires_at: (["fact", "association"].contains(&kind.as_str())).then_some(110),
                    run_id: 1,
                    adapter_digest: "fixture".into(),
                }
            })
            .collect();
        store.integrations.insert(
            id.clone(),
            Installed {
                package: Installation {
                    source: "fixture".into(),
                    operations: vec![],
                    input: json!({}),
                    interval_seconds: 300,
                    enabled: false,
                    fact_ttl_seconds: 20,
                    output_kinds: vec![],
                },
                digest: "fixture".into(),
                next_due: 0,
                current: records,
                outcomes: Default::default(),
                revisions: Default::default(),
                runs: vec![],
            },
        );
    }
    let rules = serde_json::from_value(value["evidence_rules"].clone()).unwrap();
    let request = serde_json::from_value(value["request"].clone()).unwrap();
    (store, rules, request)
}
fn result(store: &Store, rules: &[Rule], request: &Request) -> Value {
    assess("ci", request, store, rules, 100).unwrap()
}
#[test]
fn cross_source_assessment_preserves_candidates_and_proves_exact_ticket_and_compliance_links() {
    let (store, rules, request) = fixture();
    validate_rules(&rules).unwrap();
    let out = result(&store, &rules, &request);
    assert_eq!(out["requirements"][0]["assessment"]["reason"], "candidate_association");
    assert_eq!(out["requirements"][0]["assessment"]["evidence"][0]["value"], "success");
    assert_eq!(out["requirements"][1]["assessment"]["status"], "satisfied");
    assert_eq!(out["requirements"][2]["assessment"]["status"], "satisfied");
    assert_eq!(
        out["requirements"][1]["assessment"]["evidence"][0]["provenance"]["integration"],
        "tickets"
    );
    assert_eq!(out["status"], "unknown");
    assert_eq!(out["authorization"], false);
    assert!(out["requirements"][0]["rule_digest"].as_str().unwrap().starts_with("sha256:"));
}
#[test]
fn exact_dependency_link_is_contextual_and_never_reused_for_another_attempt_or_revision() {
    let (mut store, rules, mut request) = fixture();
    let model = store.integrations.get_mut("ci").unwrap().current[1].model.as_mut().unwrap();
    model.claim = Claim::ProviderReported;
    if let Entity::Association { resolution, .. } = &mut model.entity {
        *resolution = Resolution::Resolved;
    }
    assert_eq!(
        result(&store, &rules, &request)["requirements"][0]["assessment"]["status"],
        "satisfied"
    );
    request.context.insert("execution".into(), "execution:attempt3".into());
    assert_eq!(
        result(&store, &rules, &request)["requirements"][0]["assessment"]["reason"],
        "no_fresh_association"
    );
    request.revision = "other".into();
    assert!(assess("ci", &request, &store, &rules, 100).is_err());
}
#[test]
fn missing_context_expired_links_wrong_artifacts_and_untrusted_authority_remain_unknown() {
    for mode in [
        "missing",
        "expired_link",
        "expired_fact",
        "wrong_artifact",
        "wrong_authority",
        "wrong_producer",
        "future",
    ] {
        let (mut store, mut rules, mut request) = fixture();
        match mode {
            "missing" => {
                request.context.remove("artifact");
            },
            "expired_link" => {
                store.integrations.get_mut("tickets").unwrap().current[0].expires_at = Some(100)
            },
            "expired_fact" => {
                store.integrations.get_mut("tickets").unwrap().current[1].expires_at = Some(100)
            },
            "wrong_artifact" => {
                request.context.insert("artifact".into(), "sha256:other".into());
            },
            "wrong_authority" => rules[1].authority = "untrusted".into(),
            "wrong_producer" => rules[1].producer = "missing".into(),
            "future" => store.integrations.get_mut("tickets").unwrap().current[1].observed_at = 101,
            _ => unreachable!(),
        }
        assert_eq!(
            result(&store, &rules, &request)["requirements"][1]["assessment"]["status"],
            "unknown",
            "{mode}"
        );
    }
}
#[test]
fn ambiguity_conflict_and_negative_evidence_do_not_become_satisfied() {
    let (mut store, rules, request) = fixture();
    let tickets = &mut store.integrations.get_mut("tickets").unwrap().current;
    if let Entity::Fact { value, .. } = &mut tickets[1].model.as_mut().unwrap().entity {
        *value = json!(false);
    }
    assert_eq!(
        result(&store, &rules, &request)["requirements"][1]["assessment"]["status"],
        "unsatisfied"
    );
    let tickets = &mut store.integrations.get_mut("tickets").unwrap().current;
    let mut conflicting = tickets[1].clone();
    if let Entity::Fact { value, .. } = &mut conflicting.model.as_mut().unwrap().entity {
        *value = json!(true);
    }
    tickets.push(conflicting);
    assert_eq!(
        result(&store, &rules, &request)["requirements"][1]["assessment"]["reason"],
        "conflicting_facts"
    );
    let tickets = &mut store.integrations.get_mut("tickets").unwrap().current;
    let mut second = tickets[0].clone();
    if let Entity::Association { target, .. } = &mut second.model.as_mut().unwrap().entity {
        *target = "ticket:other".into();
    }
    tickets.push(second);
    assert_eq!(
        result(&store, &rules, &request)["requirements"][1]["assessment"]["reason"],
        "ambiguous_association"
    );
}
#[test]
fn rules_are_explicit_unique_and_bounded_and_context_cannot_override_revision() {
    let (store, mut rules, mut request) = fixture();
    assert_eq!(
        result(&store, &[], &request)["requirements"][1]["assessment"]["reason"],
        "no_evidence_rule"
    );
    rules.push(rules[0].clone());
    assert!(validate_rules(&rules).is_err());
    rules.pop();
    if let Selector::Association { context_keys, .. } = &mut rules[0].subject {
        *context_keys = vec!["execution".into()];
    }
    assert!(validate_rules(&rules).is_err());
    request.context.insert("revision".into(), "forged".into());
    assert!(assess("ci", &request, &store, &[], 100).is_err());
}
#[test]
fn association_identity_separates_attempts_and_inferred_links_cannot_claim_resolution() {
    let (store, _, _) = fixture();
    let original = store.integrations["ci"].current[1].model.as_ref().unwrap();
    let mut changed = original.clone();
    if let Entity::Association { context, .. } = &mut changed.entity {
        context.insert("execution".into(), "another_attempt".into());
    }
    assert_ne!(original.key(), changed.key());
    if let Entity::Association { resolution, .. } = &mut changed.entity {
        *resolution = Resolution::Resolved;
    }
    assert!(changed.validate().is_err());
}

#[tokio::test]
async fn authenticated_endpoint_rejects_duplicate_keys_and_returns_cross_source_trace() {
    use crate::host::{Config, Host};
    let (store, rules, request) = fixture();
    let mut provider_router = axum::Router::new();
    let mut operations = Vec::new();
    for (id, installed) in &store.integrations {
        let models = serde_json::to_value(
            installed.current.iter().map(|r| r.model.as_ref().unwrap()).collect::<Vec<_>>(),
        )
        .unwrap();
        provider_router = provider_router.route(
            &format!("/{id}"),
            axum::routing::get(move || {
                let models = models.clone();
                async move { axum::Json(models) }
            }),
        );
        operations.push(json!({"name":format!("read_{id}"),"connection":"fixture","path":format!("/{id}"),"parameters":{},"decoder":"json","fields":["schema_version","scope","claim","entity"]}));
    }
    let provider_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let provider_addr = provider_listener.local_addr().unwrap();
    let provider = tokio::spawn(async move {
        axum::serve(provider_listener, provider_router).await.unwrap();
    });
    let dir = tempfile::tempdir().unwrap();
    let token = "assessment_test_token_at_least_32_bytes";
    std::env::set_var("IPE_ASSESSMENT_TEST_TOKEN", token);
    let config:Config=serde_json::from_value(json!({"listen":"127.0.0.1:0","api_token_env":"IPE_ASSESSMENT_TEST_TOKEN","state_dir":dir.path().join("state"),"connections":[{"id":"fixture","origin":format!("http://{provider_addr}/")}],"operations":operations,"evidence_rules":rules})).unwrap();
    let host = Host::new(config).await.unwrap();
    for id in store.integrations.keys() {
        let package=Installation{source:r#"function step(context,responses,state){
            if(state===null)return ipe.exchange([ipe.call('read',context.operations[0])],1);
            if(state===1){const read=ipe.readResult(responses,'read');return ipe.exchange([],2,read.data.map(model=>ipe.observation(model,{},[read.receipt])));}
            return ipe.complete({coverage:'sample'});
        }"#.into(),operations:vec![format!("read_{id}")],input:json!({}),interval_seconds:300,enabled:false,fact_ttl_seconds:900,output_kinds:vec!["constraint".into(),"association".into(),"fact".into()]};
        host.install(id, package).await.unwrap();
        let run_id = host.start(id, "manual").await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                if let Some(run) = host.store.lock().await.integrations[id]
                    .runs
                    .iter()
                    .find(|r| r.id == run_id && r.status != "running")
                {
                    assert_eq!(run.status, "succeeded", "{:?}", run.error);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
    let time = host.store.lock().await.integrations["tickets"].current[0].observed_at;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = host.router();
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let client = reqwest::Client::new();
    let url = format!("http://{addr}/v1/integrations/ci/assessments");
    let body = serde_json::to_string(&request).unwrap();
    assert_eq!(client.post(&url).body(body.clone()).send().await.unwrap().status(), 401);
    let response = client.post(&url).bearer_auth(token).body(body.clone()).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let value: Value = serde_json::from_slice(&response.bytes().await.unwrap()).unwrap();
    assert_eq!(value["requirements"][1]["assessment"]["status"], "satisfied");
    let duplicate = body.replacen("\"context\":{", "\"context\":{\"artifact\":\"forged\",", 1);
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(token)
            .body(duplicate)
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    assert_eq!(
        client
            .post(&url)
            .bearer_auth(token)
            .body("x".repeat(17000))
            .send()
            .await
            .unwrap()
            .status(),
        400
    );
    // Reads cannot renew evidence or mutate the captured record set.
    assert_eq!(host.store.lock().await.integrations["tickets"].current[0].observed_at, time);
    host.stop().await;
    server.abort();
    provider.abort();
}

#[test]
fn literal_historical_fact_cannot_satisfy_a_contextual_dependency() {
    let (store, mut rules, request) = fixture();
    rules[0].subject = Selector::Literal { subject: "job:42:2:build".into() };
    assert_eq!(
        result(&store, &rules, &request)["requirements"][0]["assessment"]["reason"],
        "dependency_requires_contextual_association"
    );
}

#[test]
fn selected_catalog_does_not_freeze_associations_or_bypass_match_bounds() {
    let (mut store, rules, request) = fixture();
    let ci = store.integrations.get_mut("ci").unwrap();
    ci.revisions.capture(&ci.package, "native", &ci.current, &[], 1, 90).unwrap();
    let revision = ci.revisions.candidate.clone().unwrap();
    ci.revisions.select(&revision, false, 91).unwrap();
    assert!(ci
        .revisions
        .get(&revision)
        .unwrap()
        .records
        .iter()
        .all(|r| r.kind != "association"));
    let tickets = store.integrations.get_mut("tickets").unwrap();
    tickets.current[0].expires_at = Some(100);
    assert_eq!(
        result(&store, &rules, &request)["requirements"][1]["assessment"]["reason"],
        "no_fresh_association"
    );
    let tickets = store.integrations.get_mut("tickets").unwrap();
    tickets.current[0].expires_at = Some(110);
    for i in 0..16 {
        let mut record = tickets.current[0].clone();
        if let Entity::Association { target, .. } = &mut record.model.as_mut().unwrap().entity {
            *target = format!("ticket:{i}");
        }
        tickets.current.push(record);
    }
    assert_eq!(
        result(&store, &rules, &request)["requirements"][1]["assessment"]["reason"],
        "association_limit"
    );
}

fn fixture_v2() -> (Store, Vec<Rule>, RequestV2) {
    let (store, mut rules, request) = fixture();
    let revision = store.integrations["ci"]
        .current
        .iter()
        .find(|r| r.kind == "constraint")
        .unwrap()
        .revision_id
        .clone();
    for rule in &mut rules {
        rule.record_revision = Some(revision.clone());
    }
    let request = RequestV2 {
        operation: request.control,
        source_revision: request.revision,
        record_revision: revision,
        context: request.context,
    };
    (store, rules, request)
}

#[test]
fn v2_evidence_matches_do_not_establish_opaque_requirement_fulfillment() {
    let (store, rules, request) = fixture_v2();
    validate_rules(&rules).unwrap();
    let out = assess_v2("ci", &request, &store, &rules, 100).unwrap();
    assert_eq!(out["schema"], "ipe.integration.context-assessment.v2");
    assert_eq!(out["record_revision"], request.record_revision);
    assert_eq!(out["requirements"][1]["specification"]["kind"], "opaque");
    assert_eq!(out["requirements"][1]["evidence_check"]["status"], "matched");
    assert_eq!(out["requirements"][1]["evidence_check"]["method"], "configured_evidence_rule");
    assert_eq!(out["requirements"][1]["fulfillment"]["status"], "unknown");
    assert_eq!(out["requirements"][0]["evidence_check"]["status"], "unknown");
    assert_eq!(out["requirements"][0]["evidence_check"]["reason"], "candidate_association");
    assert_eq!(
        out["requirements"][1]["evidence_check"]["evidence"][0]["provenance"]["integration"],
        "tickets"
    );
    assert_eq!(out["applicability"]["status"], "unknown");
    assert_eq!(out["authorization"], false);
    assert_eq!(out["publication"], false);
    assert!(out.get("control").is_none());
}

#[test]
fn v2_missing_or_mismatched_rule_pins_cannot_use_legacy_evidence_checks() {
    let (store, mut rules, request) = fixture_v2();
    for (pin, reason) in [
        (None, "evidence_rule_record_revision_missing"),
        (
            Some(format!("sha256:{}", "a".repeat(64))),
            "evidence_rule_record_revision_mismatch",
        ),
    ] {
        rules[1].record_revision = pin;
        validate_rules(&rules).unwrap();
        let out = assess_v2("ci", &request, &store, &rules, 100).unwrap();
        assert_eq!(out["requirements"][1]["evidence_check"]["status"], "unknown");
        assert_eq!(out["requirements"][1]["evidence_check"]["reason"], reason);
        assert_eq!(out["requirements"][1]["evidence_check"]["evidence"], json!([]));
    }
}

#[test]
fn v2_mapper_changes_invalidate_requests_and_rule_bindings_at_the_same_source_revision() {
    let (mut store, rules, mut request) = fixture_v2();
    let record = store
        .integrations
        .get_mut("ci")
        .unwrap()
        .current
        .iter_mut()
        .find(|r| r.kind == "constraint")
        .unwrap();
    let model = record.model.as_mut().unwrap();
    if let Entity::Constraint { obligations, .. } = &mut model.entity {
        obligations[1].requirement = Requirement::Opaque {
            reason: "Approval now requires two reviewers".into(),
        };
    }
    record.adapter_digest = "changed-mapper".into();
    record.revision_id = model.revision(&record.adapter_digest).unwrap();
    let changed = record.revision_id.clone();
    assert_ne!(request.record_revision, changed);
    assert_eq!(
        assess_v2("ci", &request, &store, &rules, 100).unwrap_err(),
        "selected_record_revision_mismatch"
    );
    request.record_revision = changed;
    let out = assess_v2("ci", &request, &store, &rules, 100).unwrap();
    assert_eq!(out["source"]["revision"], request.source_revision);
    assert_eq!(out["requirements"][1]["id"], "approval");
    assert_eq!(
        out["requirements"][1]["evidence_check"]["reason"],
        "evidence_rule_record_revision_mismatch"
    );
}

#[test]
fn v2_preserves_context_trust_freshness_and_conflict_checks() {
    for mode in [
        "missing_context",
        "expired_link",
        "expired_fact",
        "wrong_authority",
        "wrong_producer",
        "future",
        "conflicting",
    ] {
        let (mut store, mut rules, mut request) = fixture_v2();
        match mode {
            "missing_context" => {
                request.context.remove("artifact");
            },
            "expired_link" => {
                store.integrations.get_mut("tickets").unwrap().current[0].expires_at = Some(100)
            },
            "expired_fact" => {
                store.integrations.get_mut("tickets").unwrap().current[1].expires_at = Some(100)
            },
            "wrong_authority" => rules[1].authority = "untrusted".into(),
            "wrong_producer" => rules[1].producer = "missing".into(),
            "future" => store.integrations.get_mut("tickets").unwrap().current[1].observed_at = 101,
            "conflicting" => {
                let records = &mut store.integrations.get_mut("tickets").unwrap().current;
                let mut extra = records[1].clone();
                if let Entity::Fact { value, .. } = &mut extra.model.as_mut().unwrap().entity {
                    *value = json!(false);
                }
                records.push(extra);
            },
            _ => unreachable!(),
        }
        let out = assess_v2("ci", &request, &store, &rules, 100).unwrap();
        assert_eq!(out["requirements"][1]["evidence_check"]["status"], "unknown", "{mode}");
    }
    let (mut store, rules, request) = fixture_v2();
    if let Entity::Fact { value, .. } = &mut store.integrations.get_mut("tickets").unwrap().current
        [1]
    .model
    .as_mut()
    .unwrap()
    .entity
    {
        *value = json!(false);
    }
    let out = assess_v2("ci", &request, &store, &rules, 100).unwrap();
    assert_eq!(out["requirements"][1]["evidence_check"]["status"], "not_matched");
    assert_eq!(out["requirements"][1]["fulfillment"]["status"], "unknown");
}

#[test]
fn v2_pins_use_the_accepted_catalog_without_freezing_evidence() {
    let (mut store, rules, request) = fixture_v2();
    let ci = store.integrations.get_mut("ci").unwrap();
    ci.revisions.capture(&ci.package, "native", &ci.current, &[], 1, 90).unwrap();
    let accepted = ci.revisions.candidate.clone().unwrap();
    ci.revisions.select(&accepted, false, 91).unwrap();
    let record = ci.current.iter_mut().find(|r| r.kind == "constraint").unwrap();
    let model = record.model.as_mut().unwrap();
    if let Entity::Constraint { title, .. } = &mut model.entity {
        *title = "New mapping title".into();
    }
    record.adapter_digest = "new-mapper".into();
    record.revision_id = model.revision(&record.adapter_digest).unwrap();
    let out = assess_v2("ci", &request, &store, &rules, 100).unwrap();
    assert_eq!(out["accepted_revision"], accepted);
    assert_eq!(out["requirements"][1]["evidence_check"]["status"], "matched");
    let ci = store.integrations.get_mut("ci").unwrap();
    ci.revisions.capture(&ci.package, "native", &ci.current, &[], 2, 92).unwrap();
    let changed = ci.revisions.candidate.clone().unwrap();
    assert_ne!(changed, accepted);
    ci.revisions.select(&changed, false, 93).unwrap();
    assert_eq!(
        assess_v2("ci", &request, &store, &rules, 100).unwrap_err(),
        "selected_record_revision_mismatch"
    );
    store
        .integrations
        .get_mut("ci")
        .unwrap()
        .revisions
        .select(&accepted, true, 94)
        .unwrap();
    let out = assess_v2("ci", &request, &store, &rules, 100).unwrap();
    assert_eq!(out["accepted_revision"], accepted);
    assert_eq!(out["requirements"][1]["evidence_check"]["status"], "matched");
    let out = assess_v2("ci", &request, &store, &rules, 110).unwrap();
    assert_eq!(out["requirements"][1]["evidence_check"]["status"], "unknown");
}

#[test]
fn v2_requires_strict_request_fields_context_and_digest_format() {
    let (store, rules, mut request) = fixture_v2();
    for bad in ["", "sha256:abc", &format!("sha256:{}", "A".repeat(64)), &"a".repeat(64)] {
        request.record_revision = bad.into();
        assert_eq!(
            assess_v2("ci", &request, &store, &rules, 100).unwrap_err(),
            "invalid_assessment_context"
        );
        let mut invalid_rules = rules.clone();
        invalid_rules[0].record_revision = Some(bad.into());
        assert!(validate_rules(&invalid_rules).is_err());
    }
    let (store, rules, mut request) = fixture_v2();
    request.context.insert("revision".into(), "forged".into());
    assert!(assess_v2("ci", &request, &store, &rules, 100).is_err());
    let mut extra = serde_json::to_value(&request).unwrap();
    extra["control"] = extra["operation"].clone();
    assert!(serde_json::from_value::<RequestV2>(extra).is_err());
    let mut missing = serde_json::to_value(&request).unwrap();
    missing.as_object_mut().unwrap().remove("record_revision");
    assert!(serde_json::from_value::<RequestV2>(missing).is_err());
}

#[test]
fn legacy_rule_serialization_and_v1_results_remain_compatible() {
    let (store, rules, request) = fixture();
    let original: Value = serde_json::from_str(include_str!(
        "../../../../testing/integration/hosted/cross-source-assessment.json"
    ))
    .unwrap();
    assert_eq!(serde_json::to_value(&rules).unwrap(), original["evidence_rules"]);
    let out = result(&store, &rules, &request);
    assert_eq!(out["schema"], "ipe.integration.context-assessment.v1");
    assert_eq!(out["requirements"][1]["assessment"]["status"], "satisfied");
    assert!(out["requirements"][1].get("evidence_check").is_none());
    // Absence of the new field also preserves the original rule digest.
    let old_rule: Rule = serde_json::from_value(original["evidence_rules"][1].clone()).unwrap();
    assert_eq!(
        out["requirements"][1]["rule_digest"],
        digest(&serde_json::to_vec(&old_rule).unwrap())
    );
    let mut pinned = rules.clone();
    pinned[1].record_revision = Some(format!("sha256:{}", "a".repeat(64)));
    let stale = result(&store, &pinned, &request);
    assert_eq!(stale["requirements"][1]["assessment"]["status"], "unknown");
    assert_eq!(
        stale["requirements"][1]["assessment"]["reason"],
        "evidence_rule_record_revision_mismatch"
    );
    assert_eq!(stale["requirements"][1]["assessment"]["evidence"], json!([]));
    let record_revision = store.integrations["ci"]
        .current
        .iter()
        .find(|r| r.kind == "constraint")
        .unwrap()
        .revision_id
        .clone();
    let request_v2 = RequestV2 {
        operation: request.control.clone(),
        source_revision: request.revision.clone(),
        record_revision: record_revision.clone(),
        context: request.context.clone(),
    };
    let stale_v2 = assess_v2("ci", &request_v2, &store, &pinned, 100).unwrap();
    assert_eq!(
        stale_v2["requirements"][1]["evidence_check"]["reason"],
        "evidence_rule_record_revision_mismatch"
    );
    pinned[1].record_revision = Some(record_revision);
    assert_eq!(
        result(&store, &pinned, &request)["requirements"][1]["assessment"]["status"],
        "satisfied"
    );
}
