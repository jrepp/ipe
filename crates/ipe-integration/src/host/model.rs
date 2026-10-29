//! Versioned domain records. Raw provider JSON is evidence, never an implicit rule.
use super::{broker::digest, Result};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub(super) const VERSION: u16 = 1;
pub(super) const KINDS: &[&str] =
    &["definition", "evidence", "constraint", "invocation", "fact", "outcome"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Model {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u16,
    pub scope: String,
    pub claim: Claim,
    pub entity: Entity,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum Claim {
    Declared,
    ProviderReported,
    Inferred,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct DefinitionId {
    pub namespace: String,
    pub resource: String,
    pub path: String,
}
impl DefinitionId {
    pub fn key(&self) -> String {
        format!("definition:{}", json!([self.namespace, self.resource, self.path]))
    }
    fn validate(&self) -> bool {
        text(&self.namespace) && text(&self.resource) && text(&self.path)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Source {
    pub definition: DefinitionId,
    pub revision: String,
    pub pointer: String,
}
impl Source {
    fn validate(&self) -> bool {
        self.definition.validate() && text(&self.revision) && pointer(&self.pointer)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ControlId {
    pub namespace: String,
    pub resource: String,
    pub operation: String,
    pub local_id: String,
}
impl ControlId {
    pub fn key(&self) -> String {
        format!(
            "control:{}",
            json!([self.namespace, self.resource, self.operation, self.local_id])
        )
    }
    fn validate(&self) -> bool {
        text(&self.namespace)
            && text(&self.resource)
            && text(&self.operation)
            && text(&self.local_id)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ExecutionId {
    pub namespace: String,
    pub resource: String,
    pub run: String,
    pub attempt: u32,
    pub job: Option<String>,
    pub step: Option<u32>,
}
impl ExecutionId {
    pub fn key(&self) -> String {
        format!(
            "execution:{}",
            json!([self.namespace, self.resource, self.run, self.attempt, self.job, self.step])
        )
    }
    fn validate(&self) -> bool {
        text(&self.namespace)
            && text(&self.resource)
            && text(&self.run)
            && self.attempt > 0
            && self.job.as_ref().is_none_or(|s| text(s))
            && self.step.is_none_or(|s| s > 0 && self.job.is_some())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Input {
    pub name: String,
    pub required: Option<bool>,
    pub default: Option<Value>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Condition {
    pub dialect: String,
    pub expression: Option<String>,
    /// Omitted expressions do not grant applicability; provider defaults need context.
    pub default_behavior: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct FactQuery {
    pub subject: String,
    pub predicate: String,
    pub expected: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NextObservation {
    pub operation: String,
    pub reason: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Obligation {
    pub id: String,
    pub description: String,
    pub requirement: Requirement,
    pub next_observation: Option<NextObservation>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Requirement {
    Fact { query: FactQuery },
    Opaque { reason: String },
    Permission { requested: Value },
    Environment { reference: Value },
    Dependency { control: ControlId },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Binding {
    pub name: String,
    pub origin: BindingOrigin,
    pub value: Value,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum BindingOrigin {
    Caller,
    DeclaredDefault,
    Absent,
    Expression,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum Resolution {
    Resolved,
    Candidate,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum ExecutionStatus {
    Queued,
    InProgress,
    Completed,
    Unknown,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum EvidenceMethod {
    ProviderDocument,
    LexicalClue,
    SourceExcerpt,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Entity {
    Definition {
        source: Source,
        title: String,
        inputs: Vec<Input>,
        outputs: Vec<String>,
    },
    Evidence {
        source: Source,
        method: EvidenceMethod,
        description: String,
    },
    Constraint {
        control: ControlId,
        source: Source,
        title: String,
        actor_context: String,
        applicability: Condition,
        obligations: Vec<Obligation>,
        dependencies: Vec<ControlId>,
        enforcer: String,
    },
    Invocation {
        parent: Source,
        callee: Option<Source>,
        requested: String,
        resolution: Resolution,
        bindings: Vec<Binding>,
    },
    Fact {
        subject: String,
        predicate: String,
        value: Value,
        authority: String,
        execution: Option<ExecutionId>,
    },
    Outcome {
        execution: ExecutionId,
        definition: Source,
        title: String,
        status: ExecutionStatus,
        conclusion: Option<String>,
        duration_ms: Option<u64>,
        started_at: Option<String>,
        completed_at: Option<String>,
        declaration_candidates: Vec<ControlId>,
        correlation: Resolution,
    },
}
fn text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 2048 && !s.chars().any(char::is_control)
}
fn pointer(s: &str) -> bool {
    s.is_empty() || s.starts_with('/') && text(s)
}
fn unique<'a>(items: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = std::collections::HashSet::new();
    items.into_iter().all(|s| text(s) && seen.insert(s))
}
impl Model {
    pub fn kind(&self) -> &'static str {
        match self.entity {
            Entity::Definition { .. } => "definition",
            Entity::Evidence { .. } => "evidence",
            Entity::Constraint { .. } => "constraint",
            Entity::Invocation { .. } => "invocation",
            Entity::Fact { .. } => "fact",
            Entity::Outcome { .. } => "outcome",
        }
    }
    pub fn logical_key(&self) -> String {
        match &self.entity {
            Entity::Definition { source, .. } => source.definition.key(),
            Entity::Evidence { source, method, .. } => {
                format!("evidence:{}", json!([source.definition, source.pointer, method]))
            },
            Entity::Constraint { control, .. } => control.key(),
            Entity::Invocation { parent, .. } => {
                format!("invocation:{}", json!([parent.definition, parent.pointer]))
            },
            Entity::Fact { subject, predicate, .. } => {
                format!("fact:{}", json!([subject, predicate]))
            },
            Entity::Outcome { execution, .. } => execution.key(),
        }
    }
    pub fn key(&self) -> String {
        let source = match &self.entity {
            Entity::Definition { source, .. }
            | Entity::Evidence { source, .. }
            | Entity::Constraint { source, .. } => Some(source),
            Entity::Invocation { parent, .. } => Some(parent),
            _ => None,
        };
        source.map_or_else(
            || self.logical_key(),
            |s| format!("revision:{}", json!([self.logical_key(), s.revision])),
        )
    }
    pub fn revision(&self, adapter_digest: &str) -> Result<String> {
        let bytes =
            serde_json::to_vec(&json!([adapter_digest, self])).map_err(|_| "invalid_model")?;
        Ok(digest(&bytes))
    }
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != VERSION || !text(&self.scope) {
            return Err("unsupported_model_schema");
        }
        let valid = match &self.entity {
            Entity::Definition { source, title, inputs, outputs } => {
                source.validate()
                    && source.pointer.is_empty()
                    && text(title)
                    && inputs.len() <= 128
                    && outputs.len() <= 128
                    && unique(inputs.iter().map(|i| i.name.as_str()))
                    && unique(outputs.iter().map(String::as_str))
                    && self.claim == Claim::Declared
            },
            Entity::Evidence { source, description, .. } => {
                source.validate()
                    && text(description)
                    && matches!(
                        self.claim,
                        Claim::Declared | Claim::Inferred | Claim::ProviderReported
                    )
            },
            Entity::Constraint {
                control,
                source,
                title,
                actor_context,
                applicability,
                obligations,
                dependencies,
                enforcer,
            } => {
                control.validate()
                    && source.validate()
                    && text(title)
                    && text(actor_context)
                    && text(enforcer)
                    && text(&applicability.dialect)
                    && text(&applicability.default_behavior)
                    && applicability.expression.as_ref().is_none_or(|s| text(s))
                    && obligations.len() <= 128
                    && dependencies.len() <= 64
                    && dependencies.iter().all(ControlId::validate)
                    && unique(obligations.iter().map(|o| o.id.as_str()))
                    && obligations.iter().all(|o| {
                        text(&o.description)
                            && o.next_observation.as_ref().is_none_or(|n| {
                                super::broker::name(&n.operation) && text(&n.reason)
                            })
                            && match &o.requirement {
                                Requirement::Fact { query } => {
                                    text(&query.subject) && text(&query.predicate)
                                },
                                Requirement::Opaque { reason } => text(reason),
                                Requirement::Permission { requested } => {
                                    requested.is_object()
                                        || requested.is_string()
                                        || requested.is_null()
                                },
                                Requirement::Environment { reference } => {
                                    reference.is_object() || reference.is_string()
                                },
                                Requirement::Dependency { control } => control.validate(),
                            }
                    })
                    && matches!(
                        self.claim,
                        Claim::Declared | Claim::ProviderReported | Claim::Unknown
                    )
            },
            Entity::Invocation {
                parent,
                callee,
                requested,
                resolution,
                bindings,
            } => {
                parent.validate()
                    && text(requested)
                    && !parent.pointer.is_empty()
                    && callee.as_ref().is_none_or(Source::validate)
                    && (*resolution != Resolution::Resolved || callee.is_some())
                    && bindings.len() <= 128
                    && unique(bindings.iter().map(|b| b.name.as_str()))
                    && matches!(self.claim, Claim::Declared | Claim::Inferred | Claim::Unknown)
            },
            Entity::Fact {
                subject, predicate, authority, execution, ..
            } => {
                text(subject)
                    && text(predicate)
                    && text(authority)
                    && execution.as_ref().is_none_or(|e| e.validate() && e.key() == *subject)
                    && self.claim == Claim::ProviderReported
            },
            Entity::Outcome {
                execution,
                definition,
                title,
                status,
                conclusion,
                started_at,
                completed_at,
                declaration_candidates,
                correlation,
                ..
            } => {
                execution.validate() && definition.validate() && text(title)
                    && conclusion.as_ref().is_none_or(|s|text(s))
                    && (*status == ExecutionStatus::Completed || conclusion.is_none())
                    && started_at.as_ref().is_none_or(|s|text(s)) && completed_at.as_ref().is_none_or(|s|text(s))
                    && declaration_candidates.len() <= 64 && declaration_candidates.iter().all(ControlId::validate)
                    // A provider-specific display-name join is never silently upgraded.
                    && *correlation != Resolution::Resolved && self.claim == Claim::ProviderReported
            },
        };
        if !valid || self.key().len() > 8192 {
            return Err("invalid_model");
        }
        Ok(())
    }
}

/// A resolved call must link to a definition actually captured in this transaction.
pub(super) fn validate_graph(records: &[super::Record]) -> Result<()> {
    for record in records {
        if let Some(Model {
            entity:
                Entity::Invocation {
                    callee: Some(callee),
                    resolution: Resolution::Resolved,
                    ..
                },
            ..
        }) = &record.model
        {
            if !records.iter().any(|r|matches!(&r.model,Some(Model {entity:Entity::Definition {source,..},..}) if source.definition==callee.definition && source.revision==callee.revision)) {
                return Err("resolved_definition_missing");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(revision: &str, pointer: &str) -> Source {
        Source {
            definition: DefinitionId {
                namespace: "github".into(),
                resource: "org/repo".into(),
                path: ".github/workflows/docs.yml".into(),
            },
            revision: revision.into(),
            pointer: pointer.into(),
        }
    }
    #[test]
    fn stable_controls_have_distinct_revisions_and_calls_have_distinct_identities() {
        let first = Model {
            schema_version: 1,
            scope: "docs".into(),
            claim: Claim::Declared,
            entity: Entity::Invocation {
                parent: source("commit-a", "/jobs/build/steps/0"),
                callee: Some(source("action-sha", "")),
                requested: "action@v1".into(),
                resolution: Resolution::Resolved,
                bindings: vec![],
            },
        };
        first.validate().unwrap();
        let mut next = first.clone();
        if let Entity::Invocation { parent, .. } = &mut next.entity {
            parent.revision = "commit-b".into();
        }
        assert_eq!(first.logical_key(), next.logical_key());
        assert_ne!(first.key(), next.key());
        assert_ne!(first.revision("mapping-a").unwrap(), next.revision("mapping-a").unwrap());
        assert_ne!(first.revision("mapping-a").unwrap(), first.revision("mapping-b").unwrap());
        if let Entity::Invocation { parent, .. } = &mut next.entity {
            parent.pointer = "/jobs/build/steps/1".into();
        }
        assert_ne!(first.key(), next.key());
        let execution = ExecutionId {
            namespace: "github".into(),
            resource: "org/repo".into(),
            run: "42".into(),
            attempt: 1,
            job: Some("7".into()),
            step: None,
        };
        let mut retry = execution.clone();
        retry.attempt = 2;
        assert_ne!(execution.key(), retry.key());
    }
    #[test]
    fn schema_rejects_unknown_fields_unsupported_versions_and_inferred_facts() {
        let value = json!({"schema_version":1,"scope":"test","claim":"provider_reported","entity":{"kind":"fact","subject":"job:1","predicate":"conclusion","value":"success","authority":"github","execution":null}});
        let valid: Model = serde_json::from_value(value.clone()).unwrap();
        valid.validate().unwrap();
        let mut bad = value.clone();
        bad["entity"]["secret_override"] = json!(true);
        assert!(serde_json::from_value::<Model>(bad).is_err());
        let mut bad = valid.clone();
        bad.schema_version = 2;
        assert!(bad.validate().is_err());
        let mut bad = valid;
        bad.claim = Claim::Inferred;
        assert!(bad.validate().is_err());
    }
}
