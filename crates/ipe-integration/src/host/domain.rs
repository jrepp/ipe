//! Operation descriptions projected from v1 records without changing stored identities.
//! A declared enforcer names a source claim, not a verified operating control.
use super::{
    model::{Claim, Condition, ControlId, Entity, NextObservation, Requirement, Source},
    Installed,
};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize)]
pub(super) struct OperationRequirement<'a> {
    id: &'a str,
    description: &'a str,
    // The v1 dependency specification retains its `control` field for compatibility.
    // That field identifies a declared operation, not an enforcement mechanism.
    specification: &'a Requirement,
    next_observation: Option<&'a NextObservation>,
}

#[derive(Serialize)]
pub(super) struct OperationDescription<'a> {
    // Reuse the identity shape, without asserting that it identifies a control.
    operation: &'a ControlId,
    source: &'a Source,
    record_revision: &'a str,
    title: &'a str,
    claim: &'a Claim,
    scope: &'a str,
    actor_context: &'a str,
    applicability: &'a Condition,
    declared_enforcer: &'a str,
    requirements: Vec<OperationRequirement<'a>>,
    dependencies: &'a [ControlId],
}

pub(super) fn operations(installed: &Installed) -> Value {
    let catalog = installed.revisions.catalog(&installed.current);
    let operations: Vec<_> = catalog
        .iter()
        .filter_map(|record| {
            let model = record.model.as_ref()?;
            let Entity::Constraint {
                control,
                source,
                title,
                actor_context,
                applicability,
                obligations,
                dependencies,
                enforcer,
            } = &model.entity
            else {
                return None;
            };
            Some(OperationDescription {
                operation: control,
                source,
                record_revision: &record.revision_id,
                title,
                claim: &model.claim,
                scope: &model.scope,
                actor_context,
                applicability,
                declared_enforcer: enforcer,
                requirements: obligations
                    .iter()
                    .map(|item| OperationRequirement {
                        id: &item.id,
                        description: &item.description,
                        specification: &item.requirement,
                        next_observation: item
                            .next_observation
                            .as_ref()
                            .filter(|next| installed.package.operations.contains(&next.operation)),
                    })
                    .collect(),
                dependencies,
            })
        })
        .collect();
    json!({
        "schema": "ipe.integration.operations.v2",
        "accepted_revision": installed.revisions.accepted,
        "candidate_revision": installed.revisions.candidate,
        "operations": operations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{model::Model, Installation, Record};

    fn fixture() -> Installed {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../testing/integration/hosted/cross-source-assessment.json"
        ))
        .unwrap();
        let current = fixture["models"]["ci"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| {
                let model: Model = serde_json::from_value(value.clone()).unwrap();
                model.validate().unwrap();
                Record {
                    key: model.key(),
                    kind: model.kind().into(),
                    revision_id: model.revision("mapping-one").unwrap(),
                    model: Some(model),
                    data: json!({}),
                    receipts: vec!["native-receipt".into()],
                    source_evidence: vec![],
                    observed_at: 90,
                    expires_at: None,
                    run_id: 1,
                    adapter_digest: "mapping-one".into(),
                }
            })
            .collect();
        Installed {
            package: Installation {
                source: "fixture".into(),
                operations: vec!["read_allowed".into()],
                input: json!({}),
                interval_seconds: 300,
                enabled: false,
                fact_ttl_seconds: 20,
                output_kinds: vec![],
            },
            digest: "mapping-one".into(),
            next_due: 0,
            current,
            outcomes: Default::default(),
            revisions: Default::default(),
            runs: vec![],
        }
    }

    #[test]
    fn projection_preserves_operation_identity_requirements_and_exact_record_revision() {
        let installed = fixture();
        let before = serde_json::to_value(&installed).unwrap();
        let out = operations(&installed);
        assert_eq!(out["schema"], "ipe.integration.operations.v2");
        assert!(out["accepted_revision"].is_null());
        assert!(out["candidate_revision"].is_null());
        let entries = out["operations"].as_array().unwrap();
        assert_eq!(entries.len(), 1);
        let record = installed.current.iter().find(|r| r.kind == "constraint").unwrap();
        let model = record.model.as_ref().unwrap();
        let Entity::Constraint {
            control, source, obligations, enforcer, ..
        } = &model.entity
        else {
            panic!("fixture must contain an operation declaration");
        };
        let operation = &entries[0];
        assert_eq!(operation["operation"], serde_json::to_value(control).unwrap());
        assert_eq!(operation["source"], serde_json::to_value(source).unwrap());
        assert_eq!(operation["record_revision"], record.revision_id);
        assert_eq!(operation["claim"], "declared");
        assert_eq!(operation["declared_enforcer"], *enforcer);
        let requirements = operation["requirements"].as_array().unwrap();
        assert_eq!(requirements.len(), obligations.len());
        for (requirement, legacy) in requirements.iter().zip(obligations) {
            assert_eq!(requirement["id"], legacy.id);
            assert_eq!(requirement["description"], legacy.description);
            assert_eq!(
                requirement["specification"],
                serde_json::to_value(&legacy.requirement).unwrap()
            );
            assert!(requirement.get("obligation").is_none());
            assert!(requirement.get("assessment").is_none());
        }
        // No policy, agreement, verified control, or authorization is inferred.
        for field in ["policy", "contract", "control", "enforcer", "obligations", "authorization"] {
            assert!(operation.get(field).is_none(), "unexpected inferred field: {field}");
        }
        assert_eq!(serde_json::to_value(&installed).unwrap(), before);
    }

    #[test]
    fn selected_catalog_keeps_its_mapping_revision_when_current_mapping_changes() {
        let mut installed = fixture();
        let first = operations(&installed)["operations"][0].clone();
        installed
            .revisions
            .capture(&installed.package, "native", &installed.current, &[], 1, 90)
            .unwrap();
        let accepted = installed.revisions.candidate.clone().unwrap();
        installed.revisions.select(&accepted, false, 91).unwrap();
        let record = installed.current.iter_mut().find(|r| r.kind == "constraint").unwrap();
        let model = record.model.as_mut().unwrap();
        if let Entity::Constraint { obligations, .. } = &mut model.entity {
            obligations[0].description =
                "Changed interpretation at the same source revision".into();
        }
        record.revision_id = model.revision("mapping-two").unwrap();
        let changed_revision = record.revision_id.clone();
        assert_ne!(first["record_revision"], changed_revision);
        installed
            .revisions
            .capture(&installed.package, "native", &installed.current, &[], 2, 92)
            .unwrap();
        let candidate = installed.revisions.candidate.clone().unwrap();
        assert_ne!(candidate, accepted);
        let selected = operations(&installed);
        assert_eq!(selected["accepted_revision"], accepted);
        assert_eq!(selected["candidate_revision"], candidate);
        assert_eq!(selected["operations"][0], first);
        installed.revisions.select(&candidate, false, 93).unwrap();
        assert_eq!(operations(&installed)["operations"][0]["record_revision"], changed_revision);
    }

    #[test]
    fn next_observations_require_current_installation_grants() {
        let mut installed = fixture();
        let model = installed.current.iter_mut().find(|r| r.kind == "constraint").unwrap();
        let Entity::Constraint { obligations, .. } = &mut model.model.as_mut().unwrap().entity
        else {
            panic!("fixture must contain an operation declaration");
        };
        obligations[0].next_observation = Some(NextObservation {
            operation: "read_allowed".into(),
            reason: "Read evidence".into(),
        });
        obligations[1].next_observation = Some(NextObservation {
            operation: "read_denied".into(),
            reason: "Read evidence".into(),
        });
        let out = operations(&installed);
        let items = &out["operations"][0]["requirements"];
        assert_eq!(items[0]["next_observation"]["operation"], "read_allowed");
        assert!(items[1]["next_observation"].is_null());
        installed.package.operations.clear();
        assert!(operations(&installed)["operations"][0]["requirements"][0]["next_observation"]
            .is_null());
    }
}
