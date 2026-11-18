//! Bounded relationship projection for one retained integration run.
use super::{
    governance::GovernanceRecord,
    model::{Entity, Model},
    Record, Run,
};
use serde_json::{json, Value};
use std::collections::HashMap;

const NODE_LIMIT: usize = 256;
const LINK_LIMIT: usize = 512;
const RULE_LIMIT: usize = 128;
const GAP_LIMIT: usize = 256;
const OUTPUT_LIMIT: usize = 1024 * 1024;

fn push_gap(gaps: &mut Vec<Value>, omitted: &mut usize, gap: Value) {
    if gaps.len() < GAP_LIMIT {
        gaps.push(gap);
    } else {
        *omitted = omitted.saturating_add(1);
    }
}

#[derive(Clone)]
struct Item<'a> {
    record: &'a Record,
    model: &'a Model,
}

fn title(model: &Model) -> String {
    match &model.entity {
        Entity::Governance { record } => match record.as_ref() {
            GovernanceRecord::Policy { title, .. }
            | GovernanceRecord::Contract { title, .. }
            | GovernanceRecord::Control { title, .. }
            | GovernanceRecord::Handoff { title, .. }
            | GovernanceRecord::Principal { title, .. } => title.clone(),
            GovernanceRecord::RoleAssignment { id, .. }
            | GovernanceRecord::ContactMethod { id, .. } => id.local_id.clone(),
        },
        Entity::Definition { title, .. }
        | Entity::Constraint { title, .. }
        | Entity::Outcome { title, .. } => title.clone(),
        Entity::Fact { predicate, .. } => predicate.clone(),
        Entity::Evidence { description, .. } => description.clone(),
        Entity::Invocation { requested, .. } => requested.clone(),
        Entity::Association { predicate, .. } => predicate.clone(),
    }
}

fn source(model: &Model) -> Option<Value> {
    let value = match &model.entity {
        Entity::Governance { record } => Some(serde_json::to_value(record.source()).ok()?),
        Entity::Definition { source, .. }
        | Entity::Evidence { source, .. }
        | Entity::Constraint { source, .. } => Some(serde_json::to_value(source).ok()?),
        Entity::Invocation { parent, .. } => Some(serde_json::to_value(parent).ok()?),
        Entity::Outcome { definition, .. } => Some(serde_json::to_value(definition).ok()?),
        _ => None,
    };
    value
}

fn record_ref(record: &Record) -> Value {
    json!({"key":record.key,"revision_id":record.revision_id})
}

fn add_link(
    links: &mut Vec<Value>,
    omitted: &mut usize,
    kind: &str,
    from: &Record,
    to: &Record,
    resolution: &str,
    reason: &str,
) {
    if links.len() == LINK_LIMIT {
        *omitted = omitted.saturating_add(1);
        return;
    }
    links.push(json!({"kind":kind,"from":record_ref(from),"to":record_ref(to),"resolution":resolution,"reason":reason}));
}

/// Produce the shared agent projection. Only succeeded runs expose retained records.
pub(super) fn project(run: &Run, time: u64) -> Value {
    let usable = run.status == "succeeded";
    let mut gaps = Vec::<Value>::new();
    let mut omitted_gaps = 0usize;
    let mut items = Vec::<Item>::new();
    if usable {
        for record in run.records.iter().take(NODE_LIMIT) {
            if let Some(model) = record.model.as_ref() {
                items.push(Item { record, model });
            } else {
                push_gap(
                    &mut gaps,
                    &mut omitted_gaps,
                    json!({"kind":"untyped_record","record":record_ref(record),"reason":"This retained record has no typed model."}),
                );
            }
        }
    } else if run.trace.as_ref().is_some_and(|trace| !trace.emissions.is_empty()) {
        push_gap(
            &mut gaps,
            &mut omitted_gaps,
            json!({"kind":"staged_emissions_excluded","reason":"The run did not succeed. Staged emissions are not retained as facts."}),
        );
    }
    let mut nodes = Vec::new();
    for item in &items {
        let fact = matches!(item.model.entity, Entity::Fact { .. });
        let (valid_from, valid_until) = match &item.model.entity {
            Entity::Governance { record } => match record.as_ref() {
                GovernanceRecord::RoleAssignment { valid_from, valid_until, .. } => {
                    (*valid_from, *valid_until)
                },
                GovernanceRecord::ContactMethod { valid_until, .. } => (None, *valid_until),
                _ => (None, None),
            },
            _ => (None, None),
        };
        let expired = if fact {
            item.record.observed_at > time
                || item.record.expires_at.is_none_or(|expiry| expiry <= time)
        } else {
            item.record.expires_at.is_some_and(|expiry| expiry <= time)
                || valid_until.is_some_and(|expiry| expiry <= time)
        };
        if item.record.observed_at > time {
            push_gap(
                &mut gaps,
                &mut omitted_gaps,
                json!({"kind":"future_observation","record":record_ref(item.record),"reason":"The record observation time is later than this explanation time."}),
            );
        }
        if fact && item.record.expires_at.is_none() {
            push_gap(
                &mut gaps,
                &mut omitted_gaps,
                json!({"kind":"missing_fact_expiry","record":record_ref(item.record),"reason":"A fact without an expiry is not treated as fresh."}),
            );
        }
        let mut node = json!({"record":record_ref(item.record),"kind":item.record.kind,"title":title(item.model),"claim":item.model.claim,"receipts":item.record.receipts,"observed_at":item.record.observed_at,"expires_at":item.record.expires_at,"expired":expired});
        if let Some(src) = source(item.model) {
            node["source"] = src;
        }
        match &item.model.entity {
            Entity::Outcome { execution, status, conclusion, .. } => {
                node["execution"] = json!(execution);
                node["status"] = json!(status);
                node["conclusion"] = json!(conclusion);
            },
            Entity::Fact {
                execution, predicate, value, authority, ..
            } => {
                node["execution"] = json!(execution);
                node["predicate"] = json!(predicate);
                node["value"] = value.clone();
                node["authority"] = json!(authority);
            },
            _ => {},
        }
        if valid_from.is_some() || valid_until.is_some() {
            node["valid_from"] = json!(valid_from);
            node["valid_until"] = json!(valid_until);
            node["temporal_state"] = json!(if valid_from.is_some_and(|start| start > time)
                || item.record.observed_at > time
            {
                "not_yet_valid"
            } else if expired {
                "expired"
            } else {
                "within_recorded_window"
            });
        }
        nodes.push(node);
    }
    let mut rules = Vec::new();
    let constraints: Vec<&Item<'_>> = items
        .iter()
        .filter(|item| matches!(item.model.entity, Entity::Constraint { .. }))
        .collect();
    let outcomes: Vec<&Item<'_>> = items
        .iter()
        .filter(|item| matches!(item.model.entity, Entity::Outcome { .. }))
        .collect();
    let facts: Vec<&Item<'_>> = items
        .iter()
        .filter(|item| matches!(item.model.entity, Entity::Fact { .. }))
        .collect();
    let mut links = Vec::new();
    let mut omitted_links = 0usize;
    let mut omitted_rules = 0usize;

    for item in &items {
        let Entity::Governance { record } = &item.model.entity else { continue };
        let (subjects, requirements, rule_kind) = match record.as_ref() {
            GovernanceRecord::Policy { subjects, requirements, .. } => {
                (subjects, requirements, "policy")
            },
            GovernanceRecord::Contract { subjects, requirements, .. } => {
                (subjects, requirements, "contract")
            },
            _ => continue,
        };
        if rules.len() >= RULE_LIMIT {
            omitted_rules += 1;
            continue;
        }
        let mut unmatched: Vec<String> = Vec::new();
        for subject in subjects {
            let matched: Vec<_> = constraints.iter().filter(|candidate| matches!(&candidate.model.entity, Entity::Constraint { control, .. } if control.key() == *subject)).collect();
            if matched.len() == 1 {
                add_link(
                    &mut links,
                    &mut omitted_links,
                    "rule_subject",
                    item.record,
                    matched[0].record,
                    "exact",
                    "The rule names this operation by its stored identity.",
                );
            } else {
                unmatched.push(subject.clone());
                push_gap(
                    &mut gaps,
                    &mut omitted_gaps,
                    json!({"kind":if matched.is_empty(){"unmatched_subject"}else{"ambiguous_subject"},"record":record_ref(item.record),"reason":if matched.is_empty(){"No operation in this run has this native subject identity."}else{"More than one operation record has this subject identity; no link was emitted."},"subject":subject}),
                );
            }
        }
        rules.push(json!({"record":record_ref(item.record),"kind":rule_kind,"requirements":requirements.iter().map(|r|json!({"id":r.id,"description":r.description,"specification":r.specification,"fulfillment":"unknown","reason":"No retained binding establishes that this requirement was checked."})).collect::<Vec<_>>(),"unmatched_subjects":unmatched}));
    }

    for operation in &constraints {
        let Entity::Constraint { control, source, .. } = &operation.model.entity else { continue };
        let mut linked_execution = false;
        for outcome in &outcomes {
            let Entity::Outcome { definition, declaration_candidates, .. } = &outcome.model.entity
            else {
                continue;
            };
            if definition.definition != source.definition
                || definition.revision != source.revision
                || !declaration_candidates.contains(control)
            {
                continue;
            }
            let matches = outcomes
                .iter()
                .filter(|candidate| {
                    candidate.record.key == outcome.record.key
                        && candidate.record.revision_id == outcome.record.revision_id
                })
                .count();
            if matches != 1 {
                push_gap(
                    &mut gaps,
                    &mut omitted_gaps,
                    json!({"kind":"ambiguous_execution","record":record_ref(outcome.record),"reason":"Duplicate execution record references make this relationship ambiguous."}),
                );
                continue;
            }
            add_link(&mut links, &mut omitted_links, "operation_execution", operation.record, outcome.record, "candidate", "The definition revision matches and the outcome lists this operation as a candidate; the execution link is not confirmed.");
            linked_execution = true;
        }
        if !linked_execution {
            push_gap(
                &mut gaps,
                &mut omitted_gaps,
                json!({"kind":"execution_not_found","record":record_ref(operation.record),"reason":"No outcome in this run matches the operation definition revision and candidate identity."}),
            );
        }
    }
    for outcome in &outcomes {
        let Entity::Outcome { execution, .. } = &outcome.model.entity else { continue };
        let mut linked_fact = false;
        for fact in &facts {
            let Entity::Fact { execution: fact_execution, .. } = &fact.model.entity else {
                continue;
            };
            if fact_execution.as_ref() != Some(execution) {
                continue;
            }
            let ambiguous = facts
                .iter()
                .filter(|candidate| {
                    candidate.record.key == fact.record.key
                        && candidate.record.revision_id == fact.record.revision_id
                })
                .count()
                != 1;
            if ambiguous {
                push_gap(
                    &mut gaps,
                    &mut omitted_gaps,
                    json!({"kind":"ambiguous_fact","record":record_ref(fact.record),"reason":"Duplicate fact record references make this relationship ambiguous."}),
                );
                continue;
            }
            add_link(
                &mut links,
                &mut omitted_links,
                "execution_fact",
                outcome.record,
                fact.record,
                "exact",
                "The observation and result identify the same run, attempt, and any job or step.",
            );
            linked_fact = true;
        }
        if !linked_fact {
            push_gap(
                &mut gaps,
                &mut omitted_gaps,
                json!({"kind":"execution_facts_missing","record":record_ref(outcome.record),"reason":"No fact in this run has this exact non-null execution identity."}),
            );
        }
    }
    // Duplicate references are ambiguous endpoints even when their contents are equal.
    let mut reference_counts: HashMap<(String, String), usize> = HashMap::new();
    for record in &run.records {
        *reference_counts
            .entry((record.key.clone(), record.revision_id.clone()))
            .or_default() += 1;
    }
    links.retain(|link| {
        let parse = |v: &Value| (v["key"].as_str().unwrap_or_default().to_owned(), v["revision_id"].as_str().unwrap_or_default().to_owned());
        let from = parse(&link["from"]); let to = parse(&link["to"]);
        let okay = reference_counts.get(&from) == Some(&1) && reference_counts.get(&to) == Some(&1);
        if !okay { push_gap(&mut gaps, &mut omitted_gaps, json!({"kind":"ambiguous_record_reference","reason":"A duplicate key and revision makes this relationship ambiguous."})); }
        okay
    });
    let omitted_nodes = run.records.len().saturating_sub(nodes.len());
    if !usable {
        push_gap(
            &mut gaps,
            &mut omitted_gaps,
            json!({"kind":"run_not_successful","reason":format!("Run status is {}; retained staged output is excluded.", run.status)}),
        );
    }
    if run.status == "running" {
        push_gap(
            &mut gaps,
            &mut omitted_gaps,
            json!({"kind":"run_incomplete","reason":"This run is still running; no staged records are projected."}),
        );
    }
    if usable {
        for read in &run.evidence {
            if read["status"] != "observed" || read["truncated"] == true {
                push_gap(
                    &mut gaps,
                    &mut omitted_gaps,
                    json!({"kind":"partial_collection","reason":"A native read failed or returned a truncated result. Missing records may reflect incomplete collection."}),
                );
                break;
            }
        }
    }
    if let Some(summary_gaps) = run.summary.get("gaps").and_then(Value::as_array) {
        for _ in summary_gaps.iter().take(GAP_LIMIT) {
            push_gap(
                &mut gaps,
                &mut omitted_gaps,
                json!({"kind":"run_summary_gap","reason":"The run summary reports a collection gap. Inspect the retained run for its details."}),
            );
        }
    }
    let output = json!({"schema":"ipe.integration.coverage-explanation.v1","run_id":run.id,"as_of":time,"run_status":run.status,"nodes":nodes,"links":links,"rules":rules,"gaps":gaps,"omissions":{"nodes":omitted_nodes,"links":omitted_links,"rules":omitted_rules,"gaps":omitted_gaps},"interpretation":"This explanation uses only records retained in this run. Candidate links do not confirm an execution. Exact execution identity links do not establish requirement fulfillment. Unknown or expired evidence cannot establish success."});
    // Serialize once. If unusually large model values exceed the response limit,
    // return an empty graph so no retained edge can point to a removed node.
    if serde_json::to_vec(&output).is_ok_and(|bytes| bytes.len() <= OUTPUT_LIMIT) {
        return output;
    }
    json!({"schema":"ipe.integration.coverage-explanation.v1","run_id":run.id,"as_of":time,"run_status":run.status,"nodes":[],"links":[],"rules":[],"gaps":[{"kind":"response_limit","reason":"The explanation exceeded the response byte limit; inspect the retained run record endpoint."}],"omissions":{"nodes":omitted_nodes + output["nodes"].as_array().map_or(0, Vec::len),"links":omitted_links + output["links"].as_array().map_or(0, Vec::len),"rules":omitted_rules + output["rules"].as_array().map_or(0, Vec::len),"gaps":omitted_gaps + output["gaps"].as_array().map_or(0, Vec::len)},"interpretation":"This explanation uses only records retained in this run. The response byte limit prevented the relationship graph from being returned."})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{broker::Metrics, model_tests};

    fn retained(models: Vec<Model>, status: &str) -> Run {
        let records = models
            .into_iter()
            .map(|model| {
                let kind = model.kind().to_owned();
                let key = model.key();
                let revision_id = model.revision("fixture").unwrap();
                Record {
                    model: Some(model),
                    kind,
                    key,
                    revision_id,
                    data: json!({}),
                    receipts: vec!["shared".into()],
                    source_evidence: vec![],
                    observed_at: 10,
                    expires_at: Some(20),
                    run_id: 7,
                    adapter_digest: "fixture".into(),
                }
            })
            .collect();
        Run {
            id: 7,
            trigger: "manual".into(),
            status: status.into(),
            started_at: 10,
            finished_at: Some(11),
            elapsed_ms: 1,
            adapter_digest: "fixture".into(),
            steps: 1,
            metrics: Metrics::default(),
            error: None,
            records,
            evidence: vec![],
            summary: json!({}),
            coverage: vec![],
            progress: vec![],
            trace: None,
        }
    }

    #[tokio::test]
    async fn projects_native_links_from_one_run_and_keeps_fulfillment_unknown() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value)
            .collect::<Result<Vec<Model>, _>>()
            .unwrap();
        let run = retained(models, "succeeded");
        let result = project(&run, 15);
        assert_eq!(result["run_id"], 7);
        assert!(result["links"]
            .as_array()
            .unwrap()
            .iter()
            .any(|link| link["kind"] == "rule_subject" && link["resolution"] == "exact"));
        assert!(
            result["links"]
                .as_array()
                .unwrap()
                .iter()
                .any(|link| link["kind"] == "operation_execution"
                    && link["resolution"] == "candidate")
        );
        assert!(result["links"]
            .as_array()
            .unwrap()
            .iter()
            .any(|link| link["kind"] == "execution_fact" && link["resolution"] == "exact"));
        assert!(result["rules"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|rule| rule["requirements"].as_array().unwrap())
            .all(|requirement| requirement["fulfillment"] == "unknown"));
        assert!(result["interpretation"]
            .as_str()
            .unwrap()
            .contains("only records retained in this run"));
    }

    #[tokio::test]
    async fn failed_and_running_runs_do_not_publish_staged_trace_records() {
        let mut run = retained(model_tests::capture(&"b".repeat(40)).await, "failed");
        run.trace = Some(super::super::RunTrace {
            schema_version: 1,
            emissions: vec![],
            omitted: 0,
        });
        run.trace.as_mut().unwrap().emissions.push(super::super::Emission {
            step: 1,
            key: "staged".into(),
            kind: "fact".into(),
            revision_id: "r".into(),
            receipts: vec![],
        });
        let failed = project(&run, 15);
        assert_eq!(failed["nodes"], json!([]));
        assert_eq!(failed["links"], json!([]));
        assert_eq!(failed["gaps"][0]["kind"], "staged_emissions_excluded");
        run.status = "running".into();
        let running = project(&run, 15);
        assert_eq!(running["nodes"], json!([]));
        assert!(running["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["kind"] == "run_incomplete"));
    }

    #[tokio::test]
    async fn missing_expiry_facts_are_not_fresh() {
        let mut run = retained(model_tests::capture(&"c".repeat(40)).await, "succeeded");
        for record in &mut run.records {
            if record.kind == "fact" {
                record.expires_at = None;
            }
        }
        let result = project(&run, 15);
        assert!(result["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node["kind"] == "fact")
            .all(|node| node["expired"] == true));
        assert!(result["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["kind"] == "missing_fact_expiry"));
        assert!(
            result["links"]
                .as_array()
                .unwrap()
                .iter()
                .all(|link| link["kind"] != "operation_execution"
                    || link["resolution"] == "candidate")
        );
    }

    #[test]
    fn source_revision_and_execution_attempt_mismatches_do_not_link() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let mut models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value::<Model>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for model in &mut models {
            match &mut model.entity {
                Entity::Constraint { source, .. } => {
                    source.revision = "wrong-source-revision".into()
                },
                Entity::Fact { execution: Some(execution), .. } => execution.attempt += 1,
                _ => {},
            }
        }
        let result = project(&retained(models, "succeeded"), 15);
        assert!(result["links"]
            .as_array()
            .unwrap()
            .iter()
            .all(|link| link["kind"] != "operation_execution"));
        assert!(result["links"]
            .as_array()
            .unwrap()
            .iter()
            .all(|link| link["kind"] != "execution_fact"));
    }

    #[test]
    fn matching_titles_and_shared_receipts_do_not_replace_native_identity() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let mut models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value::<Model>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let outcome_title = models
            .iter()
            .find_map(|model| match &model.entity {
                Entity::Outcome { title, .. } => Some(title.clone()),
                _ => None,
            })
            .unwrap();
        for model in &mut models {
            if let Entity::Constraint { source, title, .. } = &mut model.entity {
                source.definition.path = "different-workflow.yml".into();
                *title = outcome_title.clone();
            }
        }
        let result = project(&retained(models, "succeeded"), 15);
        assert!(result["links"]
            .as_array()
            .unwrap()
            .iter()
            .all(|link| link["kind"] != "operation_execution"));
        assert!(result["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|node| node["receipts"] == json!(["shared"])));
    }

    #[test]
    fn duplicate_record_references_are_reported_and_do_not_emit_edges() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value::<Model>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut run = retained(models, "succeeded");
        let duplicate =
            run.records.iter().find(|record| record.kind == "constraint").unwrap().clone();
        run.records.push(duplicate);
        let result = project(&run, 15);
        assert!(result["links"]
            .as_array()
            .unwrap()
            .iter()
            .all(|link| link["kind"] == "execution_fact"));
        assert!(result["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["kind"] == "ambiguous_subject"
                || gap["kind"] == "ambiguous_record_reference"));
    }

    #[test]
    fn untyped_duplicate_reference_suppresses_typed_endpoint_edges() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value::<Model>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut run = retained(models, "succeeded");
        let mut duplicate =
            run.records.iter().find(|record| record.kind == "fact").unwrap().clone();
        duplicate.model = None;
        run.records.push(duplicate);
        let result = project(&run, 15);
        assert!(result["links"]
            .as_array()
            .unwrap()
            .iter()
            .all(|link| link["kind"] != "execution_fact"));
        assert!(result["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|gap| gap["kind"] == "ambiguous_record_reference"));
    }

    #[test]
    fn node_capacity_reports_omitted_records() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let model: Model = serde_json::from_value(fixture["models"][0].clone()).unwrap();
        let result = project(&retained(vec![model; NODE_LIMIT + 3], "succeeded"), 15);
        assert_eq!(result["nodes"].as_array().unwrap().len(), NODE_LIMIT);
        assert_eq!(result["omissions"]["nodes"], 3);
    }

    #[test]
    fn node_omissions_count_untyped_and_over_capacity_records() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let model: Model = serde_json::from_value(fixture["models"][0].clone()).unwrap();
        let mut run = retained(vec![model; NODE_LIMIT - 1], "succeeded");
        let mut untyped = run.records[0].clone();
        untyped.model = None;
        run.records.push(untyped);
        let extra: Vec<_> = run.records.iter().take(5).cloned().collect();
        run.records.extend(extra);
        let result = project(&run, 15);
        assert_eq!(result["nodes"].as_array().unwrap().len(), NODE_LIMIT - 1);
        assert_eq!(result["omissions"]["nodes"], 6);
    }

    #[test]
    fn link_capacity_reports_omitted_edges() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value::<Model>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let operation = models
            .iter()
            .find(|model| matches!(model.entity, Entity::Constraint { .. }))
            .unwrap()
            .clone();
        let outcome = models
            .iter()
            .find(|model| matches!(model.entity, Entity::Outcome { .. }))
            .unwrap()
            .clone();
        let mut run = retained(
            (0..24)
                .map(|_| operation.clone())
                .chain((0..24).map(|_| outcome.clone()))
                .collect(),
            "succeeded",
        );
        for (index, record) in run.records.iter_mut().enumerate() {
            record.revision_id = format!("mapped-{index}");
        }
        let result = project(&run, 15);
        assert_eq!(result["links"].as_array().unwrap().len(), LINK_LIMIT);
        assert!(result["omissions"]["links"].as_u64().unwrap() >= 64);
    }

    #[test]
    fn record_expiry_limits_a_longer_governance_validity_window() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value::<Model>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut run = retained(models, "succeeded");
        for record in &mut run.records {
            if let Some(Model {
                entity: Entity::Governance { record }, ..
            }) = &mut record.model
            {
                match record.as_mut() {
                    GovernanceRecord::RoleAssignment { valid_from, valid_until, .. } => {
                        *valid_from = Some(0);
                        *valid_until = Some(100);
                    },
                    GovernanceRecord::ContactMethod { valid_until, .. } => *valid_until = Some(100),
                    _ => {},
                }
            }
        }
        let result = project(&run, 25);
        let temporal: Vec<_> = result["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|node| node.get("temporal_state").is_some())
            .collect();
        assert!(!temporal.is_empty());
        assert!(temporal
            .iter()
            .all(|node| node["expired"] == true && node["temporal_state"] == "expired"));
    }

    #[test]
    fn oversized_output_returns_bounded_empty_graph_without_dangling_links() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/governance-handoff.json"
        ))
        .unwrap();
        let mut models = fixture["models"]
            .as_array()
            .unwrap()
            .iter()
            .cloned()
            .map(serde_json::from_value::<Model>)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for model in &mut models {
            if let Entity::Fact { value, .. } = &mut model.entity {
                *value = Value::String("x".repeat(OUTPUT_LIMIT + 1));
            }
        }
        let result = project(&retained(models, "succeeded"), 15);
        assert!(result["nodes"].as_array().unwrap().is_empty());
        assert!(result["links"].as_array().unwrap().is_empty());
        assert_eq!(result["gaps"][0]["kind"], "response_limit");
        assert!(serde_json::to_vec(&result).unwrap().len() < OUTPUT_LIMIT);
    }
}
