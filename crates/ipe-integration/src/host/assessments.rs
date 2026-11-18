//! Bounded, provider-neutral joins. Caller context selects evidence; it never supplies evidence.
use super::{
    broker::digest,
    model::{Claim, ControlId, Entity, Requirement, Resolution},
    Record, Result, Store,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Rule {
    pub id: String,
    pub consumer: String,
    pub control: ControlId,
    pub revision: String,
    /// Exact mapped operation record. Optional for v1, required by v2 evidence checks.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_revision: Option<String>,
    pub obligation: String,
    pub producer: String,
    pub authority: String,
    pub predicate: String,
    pub expected: Value,
    pub subject: Selector,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Selector {
    Literal {
        subject: String,
    },
    Association {
        producer: String,
        authority: String,
        subject: Anchor,
        predicate: String,
        context_keys: Vec<String>,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Anchor {
    Literal { subject: String },
    Dependency,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub control: ControlId,
    pub revision: String,
    pub context: BTreeMap<String, String>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RequestV2 {
    pub operation: ControlId,
    pub source_revision: String,
    pub record_revision: String,
    pub context: BTreeMap<String, String>,
}
fn record_revision(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2048 && !value.chars().any(char::is_control)
}
fn control(value: &ControlId) -> bool {
    [&value.namespace, &value.resource, &value.operation, &value.local_id]
        .into_iter()
        .all(|v| text(v))
}
pub(super) fn validate_rules(rules: &[Rule]) -> Result<()> {
    if rules.len() > 128 {
        return Err("evidence_rule_limit");
    }
    let mut ids = BTreeSet::new();
    let mut bindings = BTreeSet::new();
    for rule in rules {
        if ![
            &rule.id,
            &rule.consumer,
            &rule.revision,
            &rule.obligation,
            &rule.producer,
            &rule.authority,
            &rule.predicate,
        ]
        .into_iter()
        .all(|v| text(v))
            || !control(&rule.control)
            || rule.record_revision.as_deref().is_some_and(|pin| !record_revision(pin))
            || !ids.insert(&rule.id)
            || !bindings.insert((
                &rule.consumer,
                rule.control.key(),
                &rule.revision,
                &rule.obligation,
            ))
            || serde_json::to_vec(&rule.expected).map_err(|_| "invalid_evidence_rule")?.len() > 4096
        {
            return Err("invalid_evidence_rule");
        }
        match &rule.subject {
            Selector::Literal { subject } if text(subject) => {},
            Selector::Association {
                producer,
                authority,
                subject,
                predicate,
                context_keys,
            } if text(producer)
                && text(authority)
                && text(predicate)
                && !context_keys.is_empty()
                && context_keys.len() <= 8
                && context_keys.iter().all(|k| text(k))
                && context_keys.iter().collect::<BTreeSet<_>>().len() == context_keys.len()
                && match subject {
                    Anchor::Literal { subject } => text(subject),
                    Anchor::Dependency => {
                        context_keys.iter().any(|k| k == "revision")
                            && context_keys.iter().any(|k| k == "execution")
                    },
                } => {},
            _ => return Err("invalid_evidence_rule"),
        }
    }
    Ok(())
}
// Index once per request. Joins use exact keys, never names or an all-pairs scan.
struct Index<'a>(BTreeMap<(String, String, String), Vec<&'a Record>>);
impl<'a> Index<'a> {
    fn new(store: &'a Store, producers: &BTreeSet<&str>) -> Self {
        let mut index = BTreeMap::<_, Vec<_>>::new();
        for producer in producers {
            if let Some(installed) = store.integrations.get(*producer) {
                for record in &installed.current {
                    if let Some(model) = &record.model {
                        let pair = match &model.entity {
                            Entity::Fact { subject, predicate, .. }
                            | Entity::Association { subject, predicate, .. } => {
                                Some((subject, predicate))
                            },
                            _ => None,
                        };
                        if let Some((subject, predicate)) = pair {
                            index
                                .entry((producer.to_string(), subject.clone(), predicate.clone()))
                                .or_default()
                                .push(record);
                        }
                    }
                }
            }
        }
        Self(index)
    }
    fn get(&self, producer: &str, subject: &str, predicate: &str) -> Vec<&'a Record> {
        self.0
            .get(&(producer.to_owned(), subject.to_owned(), predicate.to_owned()))
            .cloned()
            .unwrap_or_default()
    }
}
fn fresh(record: &Record, time: u64) -> bool {
    record.observed_at <= time
        && record.expires_at.is_some_and(|t| t > time)
        && !record.receipts.is_empty()
}
fn evidence(producer: &str, record: &Record, time: u64) -> Value {
    json!({"integration":producer,"key":record.key,"revision_id":record.revision_id,"run_id":record.run_id,"collector":record.adapter_digest,
        "observed_at":record.observed_at,"expires_at":record.expires_at,"fresh":fresh(record,time),"receipts":record.receipts,"claim":record.model.as_ref().map(|m|&m.claim)})
}
fn unknown(reason: &str) -> Value {
    json!({"status":"unknown","reason":reason,"evidence":[],"associations":[]})
}
fn evaluate(
    rule: &Rule,
    requirement: &Requirement,
    context: &BTreeMap<String, String>,
    index: &Index<'_>,
    time: u64,
) -> Value {
    if matches!(requirement, Requirement::Dependency { .. })
        && !matches!(&rule.subject, Selector::Association { subject: Anchor::Dependency, .. })
    {
        return unknown("dependency_requires_contextual_association");
    }
    let mut links = Vec::new();
    let mut exact = true;
    let subjects = match &rule.subject {
        Selector::Literal { subject } => vec![subject.clone()],
        Selector::Association {
            producer,
            authority,
            subject,
            predicate,
            context_keys,
        } => {
            if context_keys.iter().any(|k| !context.contains_key(k)) {
                return unknown("missing_context");
            }
            let anchor = match subject {
                Anchor::Literal { subject } => subject.clone(),
                Anchor::Dependency => match requirement {
                    Requirement::Dependency { control } => control.key(),
                    _ => return unknown("rule_requires_dependency"),
                },
            };
            let matches:Vec<_>=index.get(producer,&anchor,predicate).into_iter().filter(|r|matches!(r.model.as_ref().map(|m|&m.entity),Some(Entity::Association{context:stored,..}) if context_keys.iter().all(|k|stored.get(k)==context.get(k)))).collect();
            if matches.len() > 16 {
                return unknown("association_limit");
            }
            let mut targets = BTreeSet::new();
            for record in matches {
                let Some(model) = record.model.as_ref() else { continue };
                let Entity::Association {
                    target, authority: reported, resolution, ..
                } = &model.entity
                else {
                    continue;
                };
                links.push(json!({"provenance":evidence(producer,record,time),"target":target,"resolution":resolution,"authority":reported}));
                if !fresh(record, time) || reported != authority {
                    continue;
                }
                if !matches!(model.claim, Claim::ProviderReported | Claim::Inferred) {
                    continue;
                }
                targets.insert(target.clone());
                exact &=
                    *resolution == Resolution::Resolved && model.claim == Claim::ProviderReported;
            }
            if targets.len() != 1 {
                return json!({"status":"unknown","reason":if targets.is_empty(){"no_fresh_association"}else{"ambiguous_association"},"evidence":[],"associations":links});
            }
            targets.into_iter().collect()
        },
    };
    let mut observed = Vec::new();
    let mut values = Vec::new();
    for subject in &subjects {
        let matches = index.get(&rule.producer, subject, &rule.predicate);
        if matches.len() > 16 {
            return unknown("fact_match_limit");
        }
        for record in matches {
            let Some(model) = record.model.as_ref() else { continue };
            let Entity::Fact { value, authority, .. } = &model.entity else { continue };
            observed.push(json!({"provenance":evidence(&rule.producer,record,time),"subject":subject,"predicate":rule.predicate,"value":value,"authority":authority}));
            if fresh(record, time)
                && authority == &rule.authority
                && model.claim == Claim::ProviderReported
            {
                values.push(value);
            }
        }
    }
    let (status, reason) = if !exact {
        ("unknown", "candidate_association")
    } else if values.is_empty() {
        ("unknown", "no_fresh_trusted_fact")
    } else if values.iter().any(|v| *v != values[0]) {
        ("unknown", "conflicting_facts")
    } else if values[0] == &rule.expected {
        ("satisfied", "exact_evidence_match")
    } else {
        ("unsatisfied", "observed_value_mismatch")
    };
    json!({"status":status,"reason":reason,"expected":rule.expected,"subjects":subjects,"evidence":observed,"associations":links})
}
pub(super) fn assess(
    consumer: &str,
    request: &Request,
    store: &Store,
    rules: &[Rule],
    time: u64,
) -> Result<Value> {
    if !control(&request.control)
        || !text(&request.revision)
        || request.context.len() > 8
        || request.context.iter().any(|(k, v)| !text(k) || !text(v))
        || request.context.contains_key("revision")
    {
        return Err("invalid_assessment_context");
    }
    let installed = store.integrations.get(consumer).ok_or("integration_unavailable")?;
    let catalog = installed.revisions.catalog(&installed.current);
    let matching:Vec<_>=catalog.iter().filter(|r|matches!(&r.model,Some(m) if matches!(&m.entity,Entity::Constraint{control,source,..} if control==&request.control&&source.revision==request.revision))).collect();
    if matching.len() != 1 {
        return Err("selected_constraint_unavailable_or_ambiguous");
    }
    let constraint = matching[0];
    let Some(model) = &constraint.model else { return Err("constraint_unavailable") };
    let Entity::Constraint { obligations, source, .. } = &model.entity else {
        return Err("constraint_unavailable");
    };
    let relevant: Vec<_> = rules
        .iter()
        .filter(|r| {
            r.consumer == consumer && r.control == request.control && r.revision == request.revision
        })
        .collect();
    let mut producers = BTreeSet::new();
    for rule in relevant
        .iter()
        .filter(|r| r.record_revision.as_deref().is_none_or(|pin| pin == constraint.revision_id))
    {
        producers.insert(rule.producer.as_str());
        if let Selector::Association { producer, .. } = &rule.subject {
            producers.insert(producer.as_str());
        }
    }
    let index = Index::new(store, &producers);
    let mut context = request.context.clone();
    context.insert("revision".into(), source.revision.clone());
    let requirements:Vec<_>=obligations.iter().map(|o|{
        let rule=relevant.iter().find(|r|r.obligation==o.id);
        let assessment=rule.map_or_else(||unknown("no_evidence_rule"),|r| {
            if r.record_revision.as_deref().is_some_and(|pin| pin != constraint.revision_id) {
                unknown("evidence_rule_record_revision_mismatch")
            } else {
                evaluate(r,&o.requirement,&context,&index,time)
            }
        });
        json!({"id":o.id,"description":o.description,"requirement":o.requirement,"rule":rule.map(|r|&r.id),"rule_digest":rule.and_then(|r|serde_json::to_vec(r).ok().map(|bytes|digest(&bytes))),"assessment":assessment})
    }).collect();
    Ok(
        json!({"schema":"ipe.integration.context-assessment.v1","integration":consumer,"control":request.control,"source":source,"context":context,"context_authority":"caller_selected_not_evidence","as_of":time,
        "accepted_revision":installed.revisions.accepted,"requirements":requirements,"status":"unknown","authorization":false,"publication":false,
        "reason":"Evidence assessments do not establish actor authorization or provider condition applicability.","constraint_evidence":evidence(consumer,constraint,time)}),
    )
}

/// Assess a pinned interpretation without promoting an evidence match to fulfillment.
pub(super) fn assess_v2(
    consumer: &str,
    request: &RequestV2,
    store: &Store,
    rules: &[Rule],
    time: u64,
) -> Result<Value> {
    if !control(&request.operation)
        || !text(&request.source_revision)
        || !record_revision(&request.record_revision)
        || request.context.len() > 8
        || request.context.iter().any(|(k, v)| !text(k) || !text(v))
        || request.context.contains_key("revision")
    {
        return Err("invalid_assessment_context");
    }
    let installed = store.integrations.get(consumer).ok_or("integration_unavailable")?;
    let catalog = installed.revisions.catalog(&installed.current);
    let matching: Vec<_> = catalog
        .iter()
        .filter(|r| {
            matches!(&r.model, Some(m) if matches!(&m.entity,
            Entity::Constraint { control, source, .. }
                if control == &request.operation && source.revision == request.source_revision))
        })
        .collect();
    if matching.len() != 1 {
        return Err("selected_operation_unavailable_or_ambiguous");
    }
    let record = matching[0];
    if record.revision_id != request.record_revision {
        return Err("selected_record_revision_mismatch");
    }
    let Some(model) = &record.model else { return Err("operation_unavailable") };
    let Entity::Constraint { obligations, source, .. } = &model.entity else {
        return Err("operation_unavailable");
    };
    let relevant: Vec<_> = rules
        .iter()
        .filter(|r| {
            r.consumer == consumer
                && r.control == request.operation
                && r.revision == request.source_revision
        })
        .collect();
    // Unbound rules cannot select evidence producers for this assessment.
    let mut producers = BTreeSet::new();
    for rule in relevant
        .iter()
        .filter(|r| r.record_revision.as_deref() == Some(request.record_revision.as_str()))
    {
        producers.insert(rule.producer.as_str());
        if let Selector::Association { producer, .. } = &rule.subject {
            producers.insert(producer.as_str());
        }
    }
    let index = Index::new(store, &producers);
    let mut context = request.context.clone();
    context.insert("revision".into(), source.revision.clone());
    let requirements: Vec<_> = obligations
        .iter()
        .map(|item| {
            let rule = relevant.iter().find(|r| r.obligation == item.id);
            let mut check = match rule {
                None => unknown("no_evidence_rule"),
                Some(rule) => match rule.record_revision.as_deref() {
                    None => unknown("evidence_rule_record_revision_missing"),
                    Some(pin) if pin != record.revision_id => {
                        unknown("evidence_rule_record_revision_mismatch")
                    },
                    Some(_) => evaluate(rule, &item.requirement, &context, &index, time),
                },
            };
            check["status"] = json!(match check["status"].as_str() {
                Some("satisfied") => "matched",
                Some("unsatisfied") => "not_matched",
                _ => "unknown",
            });
            check["method"] = json!("configured_evidence_rule");
            json!({
                "id":item.id,"description":item.description,"specification":item.requirement,
                "rule":rule.map(|r| &r.id),
                "rule_digest":rule.and_then(|r| serde_json::to_vec(r).ok().map(|bytes| digest(&bytes))),
                "evidence_check":check,
                "fulfillment":{"status":"unknown","reason":"evidence_check_does_not_establish_fulfillment"}
            })
        })
        .collect();
    Ok(json!({
        "schema":"ipe.integration.context-assessment.v2","integration":consumer,
        "operation":request.operation,"source":source,"record_revision":record.revision_id,
        "context":context,"context_authority":"caller_selected_not_evidence","as_of":time,
        "accepted_revision":installed.revisions.accepted,"requirements":requirements,"status":"unknown",
        "applicability":{"status":"unknown","reason":"provider_applicability_not_evaluated"},
        "authorization":false,"publication":false,
        "reason":"Evidence checks do not establish requirement fulfillment, actor authorization, or provider condition applicability.",
        "operation_evidence":evidence(consumer,record,time)
    }))
}

#[cfg(test)]
#[path = "assessment_tests.rs"]
mod tests;
