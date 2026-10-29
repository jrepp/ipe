//! Immutable review catalogs. Acceptance never activates enforcement or renews facts.
use super::{broker, coverage::VerifiedScope, model::Entity, Installation, Record, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) const LIMIT: usize = 8;
const BYTE_LIMIT: usize = 4 * 1024 * 1024;
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Revision {
    pub schema_version: u16,
    pub sdk_version: String,
    pub id: String,
    pub created_at: u64,
    pub run_id: u64,
    pub package: Installation,
    pub native_operations_digest: String,
    pub records: Vec<Record>,
    pub coverage: Vec<VerifiedScope>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Archive {
    pub revisions: Vec<Revision>,
    pub candidate: Option<String>,
    pub accepted: Option<String>,
    pub accepted_at: BTreeMap<String, u64>,
}
impl Archive {
    pub fn capture(
        &mut self,
        package: &Installation,
        native_operations_digest: &str,
        records: &[Record],
        coverage: &[VerifiedScope],
        run_id: u64,
        time: u64,
    ) -> Result<()> {
        let records: Vec<Record> = records
            .iter()
            .filter(|r| r.model.is_some() && !["fact", "outcome"].contains(&r.kind.as_str()))
            .cloned()
            .collect();
        let mut identities:Vec<Value> = records.iter().map(|r|json!({"model":r.model,"adapter_digest":r.adapter_digest,"sources":r.source_evidence.iter().map(|e| &e["source_digest"]).collect::<Vec<_>>()})).collect();
        identities.sort_by_cached_key(Value::to_string);
        let id=broker::digest(&serde_json::to_vec(&json!({"schema":1,"sdk_version":crate::script::SDK_VERSION,"package":package,"native_operations_digest":native_operations_digest,"records":identities,"coverage":coverage.iter().map(|c|json!({"scope":c.scope,"kind":c.kind,"source":c.source,"members":c.members})).collect::<Vec<_>>()})).map_err(|_| "revision_invalid")?);
        if self.revisions.iter().any(|r| r.id == id) {
            self.candidate = Some(id);
            return Ok(());
        }
        let mut next = self.clone();
        if next.revisions.len() >= LIMIT {
            let index = next
                .revisions
                .iter()
                .position(|r| !next.accepted_at.contains_key(&r.id))
                .ok_or("revision_capacity")?;
            next.revisions.remove(index);
        }
        next.revisions.push(Revision {
            schema_version: 1,
            sdk_version: crate::script::SDK_VERSION.into(),
            id: id.clone(),
            created_at: time,
            run_id,
            package: package.clone(),
            native_operations_digest: native_operations_digest.into(),
            records,
            coverage: coverage.to_vec(),
        });
        next.candidate = Some(id);
        if serde_json::to_vec(&next).map_err(|_| "revision_invalid")?.len() > BYTE_LIMIT {
            return Err("revision_capacity");
        }
        *self = next;
        Ok(())
    }
    pub fn get(&self, id: &str) -> Result<&Revision> {
        self.revisions.iter().find(|r| r.id == id).ok_or("revision_unavailable")
    }
    pub fn select(&mut self, id: &str, restore: bool, time: u64) -> Result<()> {
        self.get(id)?;
        if restore && !self.accepted_at.contains_key(id) {
            return Err("revision_not_accepted");
        }
        self.accepted_at.entry(id.into()).or_insert(time);
        self.accepted = Some(id.into());
        Ok(())
    }
    pub fn catalog<'a>(&'a self, current: &'a [Record]) -> &'a [Record] {
        self.accepted
            .as_ref()
            .and_then(|id| self.get(id).ok())
            .map_or(current, |r| r.records.as_slice())
    }
}

// Remove only typed source revision fields. Expressions, defaults and bindings remain semantic.
fn semantic(record: &Record) -> Value {
    let mut model = serde_json::to_value(&record.model).unwrap_or(Value::Null);
    for field in ["source", "parent", "callee", "definition"] {
        if let Some(source) = model["entity"].get_mut(field).and_then(Value::as_object_mut) {
            source.remove("revision");
        }
    }
    model
}
fn groups(records: &[Record]) -> BTreeMap<String, Vec<&Record>> {
    let mut groups: BTreeMap<String, Vec<&Record>> = BTreeMap::new();
    for r in records.iter().filter(|r| !["fact", "outcome"].contains(&r.kind.as_str())) {
        if let Some(model) = &r.model {
            groups.entry(model.logical_key()).or_default().push(r);
        }
    }
    groups
}
fn values(records: &[&Record], semantics: bool) -> BTreeSet<String> {
    records.iter().map(|r| if semantics {semantic(r).to_string()} else {json!({"model":r.model,"sources":r.source_evidence.iter().map(|e| &e["source_digest"]).collect::<Vec<_>>()}).to_string()}).collect()
}
pub(super) fn diff(before: &[Record], after: &[Record]) -> Value {
    let old = groups(before);
    let new = groups(after);
    let keys: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let changes:Vec<Value>=keys.into_iter().filter_map(|key| {
        let a=old.get(key).map(Vec::as_slice).unwrap_or(&[]);
        let b=new.get(key).map(Vec::as_slice).unwrap_or(&[]);
        let category=if a.is_empty(){"added"}else if b.is_empty(){"removed"}
        else if values(a,true)!=values(b,true) {
            if b.iter().any(|r|r.model.as_ref().is_some_and(|m|matches!(m.entity,Entity::Constraint{..}|Entity::Definition{..}|Entity::Invocation{..}))) {"requirement_changed"}else{"evidence_changed"}
        }else if values(a,false)!=values(b,false){"source_changed"}
        else if a.iter().map(|r| &r.adapter_digest).collect::<BTreeSet<_>>() != b.iter().map(|r| &r.adapter_digest).collect::<BTreeSet<_>>() {"mapping_changed"}
        else if a.iter().map(|r|(r.observed_at,&r.receipts)).collect::<Vec<_>>() != b.iter().map(|r|(r.observed_at,&r.receipts)).collect::<Vec<_>>(){"evidence_refreshed"}
        else{return None};
        Some(json!({"logical_key":key,"category":category,"before":a.iter().map(|r|&r.model).collect::<Vec<_>>(),"after":b.iter().map(|r|&r.model).collect::<Vec<_>>()}))
    }).collect();
    json!({"schema":"ipe.integration.catalog-diff.v1","changes":changes,"semantics":"Compare typed declarations, not effective enforcement. Opaque expressions remain opaque. Fact freshness is independent."})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(revision: &str, condition: &str, time: u64) -> Record {
        let model:super::super::model::Model=serde_json::from_value(json!({"schema_version":1,"scope":"workflow","claim":"declared","entity":{
            "kind":"constraint","control":{"namespace":"fixture","resource":"repo","operation":"run_job","local_id":"build"},
            "source":{"definition":{"namespace":"fixture","resource":"repo","path":"ci.yml"},"revision":revision,"pointer":"/jobs/build"},
            "title":"Build","actor_context":"unknown","applicability":{"dialect":"fixture","expression":condition,"default_behavior":"unknown"},
            "obligations":[],"dependencies":[],"enforcer":"fixture"
        }})).unwrap();
        Record {
            key: model.key(),
            revision_id: model.revision("mapping").unwrap(),
            kind: "constraint".into(),
            model: Some(model),
            data: json!({}),
            receipts: vec![format!("read-{time}")],
            source_evidence: vec![],
            observed_at: time,
            expires_at: None,
            run_id: time,
            adapter_digest: "mapping".into(),
        }
    }
    fn package() -> Installation {
        Installation {
            source: "function step(){}".into(),
            operations: vec!["read".into()],
            input: json!({}),
            interval_seconds: 60,
            enabled: false,
            fact_ttl_seconds: 30,
            output_kinds: vec!["constraint".into()],
        }
    }
    #[test]
    fn diffs_separate_requirements_source_and_refresh() {
        let original = record("a", "enabled", 1);
        for (next, expected) in [
            (record("a", "enabled", 2), "evidence_refreshed"),
            (record("b", "enabled", 2), "source_changed"),
            (record("b", "other", 2), "requirement_changed"),
        ] {
            assert_eq!(
                diff(std::slice::from_ref(&original), &[next])["changes"][0]["category"],
                expected
            );
        }
        let mut remapped = original.clone();
        remapped.adapter_digest = "new mapping".into();
        assert_eq!(
            diff(std::slice::from_ref(&original), &[remapped])["changes"][0]["category"],
            "mapping_changed"
        );
        let mut dependency = original.clone();
        if let Some(super::super::model::Model {
            entity: Entity::Constraint { dependencies, .. },
            ..
        }) = &mut dependency.model
        {
            dependencies.push(super::super::model::ControlId {
                namespace: "fixture".into(),
                resource: "repo".into(),
                operation: "run_job".into(),
                local_id: "test".into(),
            });
        }
        assert_eq!(
            diff(std::slice::from_ref(&original), &[dependency])["changes"][0]["category"],
            "requirement_changed"
        );
        assert_eq!(diff(&[original], &[])["changes"][0]["category"], "removed");
    }
    #[test]
    fn changed_call_bindings_are_requirement_changes() {
        let mut before = record("a", "enabled", 1);
        let source = json!({"definition":{"namespace":"fixture","resource":"repo","path":"action.yml"},"revision":"a","pointer":"/steps/0"});
        let entity = json!({"kind":"invocation","parent":source,"callee":source,"requested":"repo@v1","resolution":"resolved","bindings":[{"name":"artifact","origin":"caller","value":"first"}]});
        let mut wire = serde_json::to_value(before.model.as_ref().unwrap()).unwrap();
        wire["entity"] = entity;
        before.model = Some(serde_json::from_value(wire.clone()).unwrap());
        before.kind = "invocation".into();
        before.key = before.model.as_ref().unwrap().key();
        let mut after = before.clone();
        wire["entity"]["bindings"][0]["value"] = json!("second");
        after.model = Some(serde_json::from_value(wire).unwrap());
        assert_eq!(diff(&[before], &[after])["changes"][0]["category"], "requirement_changed");
    }
    #[test]
    fn immutable_catalogs_coalesce_refreshes_and_restore_prior_acceptance() {
        let mut archive = Archive::default();
        let p = package();
        let old = record("a", "enabled", 1);
        archive.capture(&p, "ops", std::slice::from_ref(&old), &[], 1, 1).unwrap();
        let first = archive.candidate.clone().unwrap();
        assert_eq!(archive.select(&first, true, 2), Err("revision_not_accepted"));
        archive.select(&first, false, 2).unwrap();
        archive.capture(&p, "ops", &[record("a", "enabled", 3)], &[], 3, 3).unwrap();
        assert_eq!(archive.revisions.len(), 1);
        assert_eq!(archive.get(&first).unwrap().records[0].observed_at, 1);
        let new = record("b", "changed", 4);
        archive.capture(&p, "ops", std::slice::from_ref(&new), &[], 4, 4).unwrap();
        let second = archive.candidate.clone().unwrap();
        assert_ne!(first, second);
        assert_eq!(archive.catalog(&[])[0].key, old.key);
        archive.select(&second, false, 5).unwrap();
        assert_eq!(archive.catalog(&[])[0].key, new.key);
        archive.select(&first, true, 6).unwrap();
        assert_eq!(archive.catalog(&[])[0].key, old.key);
        assert_eq!(archive.candidate.as_deref(), Some(second.as_str()));
        let restored: Archive =
            serde_json::from_value(serde_json::to_value(&archive).unwrap()).unwrap();
        assert_eq!(restored.catalog(&[])[0].observed_at, 1);
    }
    #[test]
    fn accepted_revisions_are_retained_and_capacity_errors_are_atomic() {
        let mut archive = Archive::default();
        let p = package();
        for n in 0..LIMIT {
            archive
                .capture(
                    &p,
                    "ops",
                    &[record(&n.to_string(), "enabled", n as u64)],
                    &[],
                    n as u64,
                    n as u64,
                )
                .unwrap();
            let id = archive.candidate.clone().unwrap();
            archive.select(&id, false, n as u64).unwrap();
        }
        let before = serde_json::to_value(&archive).unwrap();
        assert_eq!(
            archive.capture(&p, "ops", &[record("overflow", "enabled", 99)], &[], 99, 99),
            Err("revision_capacity")
        );
        assert_eq!(serde_json::to_value(&archive).unwrap(), before);
    }
}
