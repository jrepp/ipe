//! Versioned policy definitions and provenance, separate from expiring runtime facts.
//! Imported source locators are audit references; evaluation never dereferences them.
use crate::runtime_facts::{FactSnapshot, Schema, Scope};
use crate::runtime_policy::{
    CompileEnvironment, CompiledPolicy, EvalBudget, PolicyData, PolicyOutcome,
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};
use thiserror::Error;

const MAX_DOCUMENT_BYTES: usize = 262_144;
const MAX_CATALOG_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDocument {
    pub id: String,
    pub title: String,
    pub data: PolicyData,
    /// None for locally authored policy. An external model must include provenance.
    pub external: Option<ExternalPolicy>,
    pub sections: Vec<PolicySection>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalPolicy {
    pub system: String,
    /// Stable locator for the external resource whose policy was observed.
    pub locator: String,
    /// Opaque upstream revision or ETag, not an IPE revision.
    pub source_revision: String,
    /// SHA-256 of the source material read by the importer, asserted by that importer.
    pub source_digest: String,
    pub observed_at: i64,
    /// Deadline for relying on this observation of mutable external policy.
    pub valid_until: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicySection {
    pub id: String,
    pub title: String,
    /// JSON Pointer into the source material; required for external sections.
    pub source_pointer: Option<String>,
    pub rule: SectionRule,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SectionRule {
    Modeled { source: String },
    Unsupported { reason: String },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRevision {
    pub scope: Scope,
    pub revision: u64,
    pub document_digest: String,
    pub published_at: i64,
    pub publisher: String,
    pub document: PolicyDocument,
    pub input_schema: Schema,
    pub collection_schemas: BTreeMap<String, Schema>,
}

impl PolicyRevision {
    pub fn prepare(
        &self,
        environment: &CompileEnvironment,
    ) -> Result<PreparedDocument, DocumentError> {
        if environment.input.as_ref() != Some(&self.input_schema)
            || self
                .collection_schemas
                .iter()
                .any(|(name, schema)| environment.collections.get(name) != Some(schema))
        {
            return Err(invalid("policy revision schema pins do not match the runtime catalog"));
        }
        if self.document.digest() != self.document_digest {
            return Err(invalid("policy revision content digest does not match"));
        }
        let prepared = self.document.prepare(environment)?;
        let referenced: BTreeMap<_, _> = prepared
            .policies
            .iter()
            .flatten()
            .flat_map(|p| p.collection_schemas.clone())
            .collect();
        if referenced != self.collection_schemas {
            return Err(invalid("policy revision dependency pins do not match the document"));
        }
        Ok(prepared)
    }
}

/// Compiled revisions cannot be mutated or supplied as unvalidated ASTs.
pub struct PreparedDocument {
    document: PolicyDocument,
    policies: Vec<Option<CompiledPolicy>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Allow,
    Deny,
    Indeterminate,
    NotApplicable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SectionResult {
    pub section: String,
    pub source_pointer: Option<String>,
    pub status: ExecutionStatus,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DocumentResult {
    pub document: String,
    pub status: ExecutionStatus,
    pub evidence_deadline: Option<i64>,
    pub sections: Vec<SectionResult>,
}

#[derive(Debug, Error)]
pub enum DocumentError {
    #[error("invalid policy document: {0}")]
    Invalid(String),
    #[error("policy revision conflict")]
    Conflict,
    #[error("policy catalog capacity exceeded")]
    Capacity,
    #[error("policy storage unavailable: {0}")]
    Storage(String),
}

fn invalid(message: impl Into<String>) -> DocumentError {
    DocumentError::Invalid(message.into())
}
fn nonempty(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 4096
}
fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}
fn json_pointer(s: &str) -> bool {
    if s.is_empty() {
        return true;
    }
    if !s.starts_with('/') || s.len() > 4096 {
        return false;
    }
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '~' && !matches!(chars.next(), Some('0' | '1')) {
            return false;
        }
    }
    true
}

impl PolicyDocument {
    pub fn prepare(
        &self,
        environment: &CompileEnvironment,
    ) -> Result<PreparedDocument, DocumentError> {
        if environment.input.is_none() {
            return Err(invalid("input schema is required"));
        }
        if serde_json::to_vec(&self.data).map_err(|e| invalid(e.to_string()))?.len() > 65_536 {
            return Err(invalid("policy data exceeds 65536 bytes"));
        }
        let bytes = serde_json::to_vec(self).map_err(|e| invalid(e.to_string()))?;
        if bytes.len() > MAX_DOCUMENT_BYTES || !identifier(&self.id) || !nonempty(&self.title) {
            return Err(invalid("invalid identity, title, or document size"));
        }
        self.data
            .schema
            .validate_value(&self.data.value)
            .map_err(|e| invalid(e.to_string()))?;
        if self.sections.is_empty() || self.sections.len() > 100 {
            return Err(invalid("a document requires 1..100 sections"));
        }
        if let Some(external) = &self.external {
            if !nonempty(&external.system)
                || !nonempty(&external.locator)
                || !nonempty(&external.source_revision)
                || external.source_digest.len() != 64
                || !external.source_digest.bytes().all(|c| c.is_ascii_hexdigit())
                || external.observed_at < 0
                || external.valid_until <= external.observed_at
            {
                return Err(invalid("invalid external provenance or validity interval"));
            }
        }
        let environment = environment.clone().policy_data(self.data.clone());
        let mut ids = BTreeSet::new();
        let mut policies = Vec::new();
        for section in &self.sections {
            if !identifier(&section.id) || !nonempty(&section.title) || !ids.insert(&section.id) {
                return Err(invalid("invalid or duplicate section identity"));
            }
            if self.external.is_some() && section.source_pointer.is_none()
                || section.source_pointer.as_ref().is_some_and(|p| !json_pointer(p))
            {
                return Err(invalid("external sections require a valid source JSON Pointer"));
            }
            policies.push(match &section.rule {
                SectionRule::Modeled { source } => Some(
                    CompiledPolicy::compile(source, &environment)
                        .map_err(|e| invalid(format!("section {}: {e}", section.id)))?,
                ),
                SectionRule::Unsupported { reason } if nonempty(reason) => None,
                SectionRule::Unsupported { .. } => {
                    return Err(invalid("unsupported sections require a reason"))
                },
            });
        }
        Ok(PreparedDocument { document: self.clone(), policies })
    }

    pub fn digest(&self) -> String {
        // serde_json's default object representation and the schema BTreeMaps sort keys.
        format!("{:x}", Sha256::digest(serde_json::to_vec(self).expect("serializable document")))
    }
}

impl PreparedDocument {
    /// One immutable policy model + one immutable fact snapshot + an explicit clock.
    pub fn evaluate(
        &self,
        input: &Value,
        facts: &FactSnapshot,
        now: i64,
        completed_at: i64,
    ) -> DocumentResult {
        let external = self.document.external.as_ref();
        let external_invalid =
            external.is_some_and(|p| now < p.observed_at || completed_at >= p.valid_until);
        let mut deadline = external.map(|p| p.valid_until);
        let sections: Vec<_> = self
            .document
            .sections
            .iter()
            .zip(&self.policies)
            .map(|(section, policy)| {
                let (status, reason) = if completed_at < now || external_invalid {
                    (
                        ExecutionStatus::Indeterminate,
                        Some("external policy observation or evaluation clock is invalid".into()),
                    )
                } else if let Some(policy) = policy {
                    let result = policy.evaluate(input, facts, now, EvalBudget::default());
                    if let Some(end) = result.evidence_deadline {
                        deadline = Some(deadline.map_or(end, |old| old.min(end)));
                    }
                    match result.outcome {
                        PolicyOutcome::Allow
                            if result.evidence_deadline.is_some_and(|end| completed_at >= end) =>
                        {
                            (
                                ExecutionStatus::Indeterminate,
                                Some("runtime evidence expired before completion".into()),
                            )
                        },
                        PolicyOutcome::Allow => (ExecutionStatus::Allow, None),
                        PolicyOutcome::Deny => (ExecutionStatus::Deny, None),
                        PolicyOutcome::NotApplicable => (ExecutionStatus::NotApplicable, None),
                        PolicyOutcome::Indeterminate(reason) => {
                            (ExecutionStatus::Indeterminate, Some(format!("{reason:?}")))
                        },
                    }
                } else {
                    let SectionRule::Unsupported { reason } = &section.rule else { unreachable!() };
                    (ExecutionStatus::Indeterminate, Some(reason.clone()))
                };
                SectionResult {
                    section: section.id.clone(),
                    source_pointer: section.source_pointer.clone(),
                    status,
                    reason,
                }
            })
            .collect();
        let status = combine_statuses(sections.iter().map(|s| &s.status));
        DocumentResult {
            document: self.document.id.clone(),
            status,
            evidence_deadline: deadline,
            sections,
        }
    }
}

/// A caller grants only Allow. An entirely inapplicable catalog is denied by its caller.
pub fn combine_statuses<'a>(
    statuses: impl IntoIterator<Item = &'a ExecutionStatus>,
) -> ExecutionStatus {
    let (mut allow, mut indeterminate) = (false, false);
    for status in statuses {
        match status {
            ExecutionStatus::Deny => return ExecutionStatus::Deny,
            ExecutionStatus::Indeterminate => indeterminate = true,
            ExecutionStatus::Allow => allow = true,
            ExecutionStatus::NotApplicable => {},
        }
    }
    if indeterminate {
        ExecutionStatus::Indeterminate
    } else if allow {
        ExecutionStatus::Allow
    } else {
        ExecutionStatus::NotApplicable
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct CatalogState {
    revisions: Vec<PolicyRevision>,
}

/// Append-only immutable revisions. Publishing atomically advances the active revision.
/// The bounded catalog fails writes instead of silently discarding audit history.
pub struct PolicyCatalog {
    state: Mutex<Arc<CatalogState>>,
    db: Option<rocksdb::DB>,
}
impl PolicyCatalog {
    pub fn memory() -> Self {
        Self {
            state: Mutex::new(Arc::new(CatalogState::default())),
            db: None,
        }
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DocumentError> {
        let db =
            rocksdb::DB::open_default(path).map_err(|e| DocumentError::Storage(e.to_string()))?;
        let state = match db
            .get(b"policy-catalog-v1")
            .map_err(|e| DocumentError::Storage(e.to_string()))?
        {
            Some(bytes) => {
                if bytes.len() > MAX_CATALOG_BYTES {
                    return Err(DocumentError::Capacity);
                }
                serde_json::from_slice(&bytes)
                    .map_err(|e| DocumentError::Storage(format!("invalid catalog: {e}")))?
            },
            None => CatalogState::default(),
        };
        Ok(Self {
            state: Mutex::new(Arc::new(state)),
            db: Some(db),
        })
    }
    pub fn active(&self, scope: &Scope) -> Vec<PolicyRevision> {
        let state = self.state.lock().clone();
        let mut seen = BTreeSet::new();
        state
            .revisions
            .iter()
            .rev()
            .filter(|r| &r.scope == scope && seen.insert(&r.document.id))
            .cloned()
            .collect()
    }
    pub fn revision(&self, scope: &Scope, id: &str, revision: u64) -> Option<PolicyRevision> {
        self.state
            .lock()
            .revisions
            .iter()
            .find(|r| &r.scope == scope && r.document.id == id && r.revision == revision)
            .cloned()
    }
    pub fn all(&self) -> Vec<PolicyRevision> {
        self.state.lock().revisions.clone()
    }
    pub fn publish(
        &self,
        scope: Scope,
        document: PolicyDocument,
        expected_revision: Option<u64>,
        publisher: String,
        now: i64,
        environment: &CompileEnvironment,
    ) -> Result<PolicyRevision, DocumentError> {
        let prepared = document.prepare(environment)?;
        if now < 0
            || !nonempty(&publisher)
            || scope.tenant.is_empty()
            || scope.environment.is_empty()
            || scope.tenant.len() > 1024
            || scope.environment.len() > 1024
        {
            return Err(invalid("invalid publication metadata"));
        }
        if document
            .external
            .as_ref()
            .is_some_and(|p| p.observed_at > now || p.valid_until <= now)
        {
            return Err(invalid("external policy observation is future-dated or expired"));
        }
        let mut guard = self.state.lock();
        let old = guard
            .revisions
            .iter()
            .rev()
            .find(|r| r.scope == scope && r.document.id == document.id);
        if old.map(|r| r.revision) != expected_revision {
            return Err(DocumentError::Conflict);
        }
        if let Some(old) = old {
            if now < old.published_at {
                return Err(invalid("publication clock moved backwards"));
            }
            match (&old.document.external, &document.external) {
                (Some(previous), Some(next)) => {
                    if previous.system != next.system || previous.locator != next.locator {
                        return Err(invalid(
                            "external authority cannot change for an existing document",
                        ));
                    }
                    if next.observed_at <= previous.observed_at {
                        return Err(invalid("external observation must advance"));
                    }
                    if next.source_revision == previous.source_revision
                        && next.source_digest != previous.source_digest
                    {
                        return Err(invalid(
                            "source content changed without advancing its revision",
                        ));
                    }
                },
                (None, None) => {},
                _ => {
                    return Err(invalid(
                        "local/external origin cannot change for an existing document",
                    ))
                },
            }
        } else if guard
            .revisions
            .iter()
            .filter(|r| r.scope == scope)
            .map(|r| &r.document.id)
            .collect::<BTreeSet<_>>()
            .len()
            >= 100
        {
            return Err(DocumentError::Capacity);
        }
        let revision = old
            .map_or(Some(1), |r| r.revision.checked_add(1))
            .ok_or(DocumentError::Capacity)?;
        let result = PolicyRevision {
            scope,
            revision,
            document_digest: document.digest(),
            published_at: now,
            publisher,
            input_schema: environment
                .input
                .clone()
                .ok_or_else(|| invalid("input schema is required"))?,
            collection_schemas: prepared
                .policies
                .iter()
                .flatten()
                .flat_map(|p| p.collection_schemas.clone())
                .collect(),
            document,
        };
        let mut next = (**guard).clone();
        if next.revisions.len() >= 10_000 {
            return Err(DocumentError::Capacity);
        }
        next.revisions.push(result.clone());
        let bytes = serde_json::to_vec(&next).map_err(|e| DocumentError::Storage(e.to_string()))?;
        if bytes.len() > MAX_CATALOG_BYTES {
            return Err(DocumentError::Capacity);
        }
        if let Some(db) = &self.db {
            let mut options = rocksdb::WriteOptions::default();
            options.set_sync(true);
            db.put_opt(b"policy-catalog-v1", bytes, &options)
                .map_err(|e| DocumentError::Storage(e.to_string()))?;
        }
        *guard = Arc::new(next);
        Ok(result)
    }
}

#[cfg(test)]
#[path = "policy_document_tests.rs"]
mod tests;
