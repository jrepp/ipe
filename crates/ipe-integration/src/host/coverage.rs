//! Reconcile only explicitly authorized, complete document scopes.
use super::{
    model::{DefinitionId, Entity, Source},
    Record, Result,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SnapshotSource {
    pub namespace: String,
    pub resource_parameter: String,
    pub path_parameter: String,
    pub revision_parameter: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Claim {
    pub schema_version: u16,
    pub scope: String,
    pub kind: String,
    pub receipt: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct VerifiedScope {
    pub scope: String,
    pub kind: String,
    pub source: Source,
    pub receipt: String,
    pub members: Vec<String>,
}

pub(super) fn verify(
    summary: &Value,
    records: &[Record],
    evidence: &[Value],
) -> Result<Vec<VerifiedScope>> {
    let claims: Vec<Claim> = serde_json::from_value(
        summary.get("collections").cloned().unwrap_or_else(|| serde_json::json!([])),
    )
    .map_err(|_| "invalid_collection_coverage")?;
    if claims.len() > 32 {
        return Err("collection_coverage_limit");
    }
    let mut scopes = Vec::new();
    for claim in claims {
        if claim.schema_version != 1
            || claim.kind != "constraint"
            || claim.scope.is_empty()
            || claim.scope.len() > 2048
        {
            return Err("invalid_collection_coverage");
        }
        let read = evidence
            .iter()
            .find(|e| e["receipt"] == claim.receipt)
            .ok_or("coverage_receipt_unknown")?;
        if read["status"] != "observed" || read["truncated"] != false {
            return Err("coverage_read_incomplete");
        }
        let source: Source = serde_json::from_value(read["snapshot_source"].clone())
            .map_err(|_| "coverage_not_authorized")?;
        let mut members = Vec::new();
        for record in records.iter().filter(|r| {
            r.kind == claim.kind && r.model.as_ref().is_some_and(|m| m.scope == claim.scope)
        }) {
            let Some(super::model::Model {
                entity: Entity::Constraint { source: record_source, .. },
                ..
            }) = &record.model
            else {
                return Err("coverage_member_invalid");
            };
            if record_source.definition != source.definition
                || record_source.revision != source.revision
            {
                return Err("coverage_member_source_mismatch");
            }
            if !record.receipts.contains(&claim.receipt) {
                return Err("coverage_member_receipt_mismatch");
            }
            members.push(record.key.clone());
        }
        scopes.push(VerifiedScope {
            scope: claim.scope,
            kind: claim.kind,
            source,
            receipt: claim.receipt,
            members,
        });
    }
    Ok(scopes)
}
fn belongs(record: &Record, scope: &str, definition: &DefinitionId) -> bool {
    record.model.as_ref().is_some_and(|m| {
        m.scope == scope
            && matches!(&m.entity,Entity::Constraint {source,..} if &source.definition==definition)
    })
}
pub(super) fn reconcile(
    current: &[Record],
    records: &[Record],
    scopes: &[VerifiedScope],
) -> Result<Vec<Record>> {
    let mut next = current.to_vec();
    // A partial capture without a verified scope is upsert-only. No absence deletion.
    next.retain(|old| {
        !scopes
            .iter()
            .any(|s| old.kind == s.kind && belongs(old, &s.scope, &s.source.definition))
    });
    for record in records.iter().filter(|r| r.kind != "outcome") {
        next.retain(|old| old.kind != record.kind || old.key != record.key);
        next.push(record.clone());
    }
    if next.len() > 256 {
        return Err("current_record_limit");
    }
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn source(revision: &str) -> Value {
        json!({"definition":{"namespace":"fixture","resource":"repo","path":"workflow.yml"},"revision":revision,"pointer":""})
    }
    fn record(id: &str, revision: &str, scope: &str) -> Record {
        let model: super::super::model::Model = serde_json::from_value(json!({
            "schema_version":1,"scope":scope,"claim":"declared","entity":{
                "kind":"constraint","control":{"namespace":"fixture","resource":"repo","operation":"run_job","local_id":id},
                "source":source(revision),"title":id,"actor_context":"unknown",
                "applicability":{"dialect":"fixture","expression":null,"default_behavior":"unknown"},
                "obligations":[],"dependencies":[],"enforcer":"fixture"
            }
        })).unwrap();
        model.validate().unwrap();
        Record {
            key: model.key(),
            revision_id: model.revision("adapter").unwrap(),
            model: Some(model),
            kind: "constraint".into(),
            data: json!({}),
            receipts: vec!["read".into()],
            source_evidence: vec![],
            observed_at: 10,
            expires_at: None,
            run_id: 1,
            adapter_digest: "adapter".into(),
        }
    }
    fn summary() -> Value {
        json!({"collections":[{"schema_version":1,"scope":"workflow","kind":"constraint","receipt":"read"}]})
    }
    fn read() -> Value {
        json!({"receipt":"read","status":"observed","truncated":false,"snapshot_source":source("new")})
    }
    #[test]
    fn complete_document_replaces_only_its_declared_scope_including_empty_collection() {
        let old = vec![record("removed", "old", "workflow"), record("other", "old", "other")];
        let new = vec![record("added", "new", "workflow")];
        let scopes = verify(&summary(), &new, &[read()]).unwrap();
        assert_eq!(scopes[0].members, vec![new[0].key.clone()]);
        let next = reconcile(&old, &new, &scopes).unwrap();
        assert_eq!(next.len(), 2);
        assert_eq!(next[0].key, old[1].key);
        assert_eq!(next[1].key, new[0].key);
        let empty = verify(&summary(), &[], &[read()]).unwrap();
        let next = reconcile(&old, &[], &empty).unwrap();
        assert_eq!(next.len(), 1);
        assert_eq!(next[0].key, old[1].key);
    }
    #[test]
    fn partial_and_missing_collections_preserve_prior_requirements() {
        let old = vec![record("retained", "old", "workflow")];
        let new = vec![record("added", "new", "workflow")];
        let scopes = verify(&json!({"coverage":"partial"}), &new, &[read()]).unwrap();
        assert!(scopes.is_empty());
        assert_eq!(reconcile(&old, &new, &scopes).unwrap().len(), 2);
        assert_eq!(reconcile(&old, &[], &[]).unwrap()[0].key, old[0].key);
    }
    #[test]
    fn incomplete_unavailable_and_unauthorized_reads_cannot_authorize_removal() {
        for (field, value, expected) in [
            ("truncated", json!(true), "coverage_read_incomplete"),
            ("status", json!("unavailable"), "coverage_read_incomplete"),
            ("status", json!("pending"), "coverage_read_incomplete"),
            ("snapshot_source", Value::Null, "coverage_not_authorized"),
            ("receipt", json!("other"), "coverage_receipt_unknown"),
        ] {
            let mut evidence = read();
            evidence[field] = value;
            assert_eq!(verify(&summary(), &[], &[evidence]).err(), Some(expected));
        }
        assert_eq!(verify(&summary(), &[], &[]).err(), Some("coverage_receipt_unknown"));
        let mut member = record("new", "new", "workflow");
        member.receipts = vec!["other".into()];
        assert_eq!(
            verify(&summary(), &[member], &[read()]).err(),
            Some("coverage_member_receipt_mismatch")
        );
    }
    #[test]
    fn source_authority_and_scope_limit_deletion() {
        let old = vec![record("retained", "old", "workflow")];
        assert_eq!(
            verify(&summary(), &old, &[read()]).err(),
            Some("coverage_member_source_mismatch")
        );
        let mut evidence = read();
        evidence["snapshot_source"]["definition"]["resource"] = json!("other-repo");
        let scopes = verify(&summary(), &[], &[evidence]).unwrap();
        assert_eq!(reconcile(&old, &[], &scopes).unwrap()[0].key, old[0].key);
        let mut invalid = summary();
        invalid["collections"][0]["kind"] = json!("fact");
        assert_eq!(verify(&invalid, &[], &[read()]).err(), Some("invalid_collection_coverage"));
    }
}
