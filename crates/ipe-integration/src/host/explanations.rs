//! One bounded projection for agent clients and the human monitor.
use super::{
    model::{Entity, Requirement},
    Installed, Record,
};
use serde_json::{json, Value};

fn fact_requirement(query: &super::model::FactQuery, current: &[Record], time: u64) -> Value {
    let matches:Vec<&Record>=current.iter().filter(|r|matches!(&r.model,Some(m) if matches!(&m.entity,Entity::Fact {subject,predicate,..} if subject==&query.subject && predicate==&query.predicate))).collect();
    let fresh: Vec<&Record> = matches
        .iter()
        .copied()
        .filter(|r| r.expires_at.is_some_and(|t| t > time))
        .collect();
    let values: Vec<&Value> = fresh
        .iter()
        .filter_map(|r| match &r.model.as_ref()?.entity {
            Entity::Fact { value, .. } => Some(value),
            _ => None,
        })
        .collect();
    let status = if values.is_empty() || values.iter().any(|v| *v != values[0]) {
        "unknown"
    } else if values[0] == &query.expected {
        "satisfied"
    } else {
        "unsatisfied"
    };
    json!({"status":status,"reason":if fresh.is_empty(){"No fresh supporting fact"}else if status=="unknown"{"Conflicting fresh facts"}else{"Compared the expected value with fresh provider facts"},"evidence":matches.iter().map(|r|json!({"key":r.key,"observed_at":r.observed_at,"expires_at":r.expires_at,"expired":r.expires_at.is_none_or(|t|t<=time),"receipts":r.receipts})).collect::<Vec<_>>()})
}
pub(super) fn project(installed: &Installed, time: u64) -> Value {
    let catalog = installed.revisions.catalog(&installed.current);
    let operations:Vec<Value>=catalog.iter().filter_map(|record| {
        let model=record.model.as_ref()?;
        let Entity::Constraint{control,source,title,actor_context,applicability,obligations,dependencies,enforcer}=&model.entity else{return None};
        let requirements:Vec<Value>=obligations.iter().map(|o| {
            let assessment=match &o.requirement {
                Requirement::Fact{query}=>fact_requirement(query,&installed.current,time),
                Requirement::Opaque{reason}=>json!({"status":"unknown","reason":reason,"evidence":[]}),
                Requirement::Permission{..}=>json!({"status":"unknown","reason":"Requested permissions do not establish effective permissions","evidence":[]}),
                Requirement::Environment{..}=>json!({"status":"unknown","reason":"Environment protection requirements need a separate observation","evidence":[]}),
                Requirement::Dependency{..}=>json!({"status":"unknown","reason":"A dependency declaration does not establish success in the intended execution attempt","evidence":[]}),
            };
            json!({"id":o.id,"description":o.description,"requirement":o.requirement,"assessment":assessment,
                "next_observation":o.next_observation.as_ref().filter(|n|installed.package.operations.contains(&n.operation)),
                "observation_not_granted":o.next_observation.as_ref().is_some_and(|n|!installed.package.operations.contains(&n.operation))})
        }).collect();
        let outcomes:Vec<Value>=installed.outcomes.executions.values().filter_map(|entry| {
            let Entity::Outcome{definition,declaration_candidates,correlation,execution,status,conclusion,duration_ms,started_at,completed_at,..}=&entry.latest.model.as_ref()?.entity else{return None};
            if definition.definition!=source.definition || definition.revision!=source.revision || !declaration_candidates.contains(control) {return None}
            Some(json!({"execution":execution,"correlation":correlation,"status":status,"conclusion":conclusion,"duration_ms":duration_ms,"started_at":started_at,"completed_at":completed_at,"first_observed_at":entry.first_observed_at,"last_observed_at":entry.last_observed_at,"retained_changes":entry.changes.len(),"note":"Candidate association. This does not prove the intended execution prerequisites."}))
        }).collect();
        Some(json!({"control":control,"logical_key":control.key(),"title":title,"operation":control.operation,"source":source,"claim":model.claim,"scope":model.scope,
            "status":"unknown","reason":"This declaration is not an authorization decision. Actor authorization and provider condition semantics require evidence.",
            "actor_context":actor_context,"applicability":applicability,"enforcer":enforcer,"requirements":requirements,"dependencies":dependencies,
            "evidence":{"observed_at":record.observed_at,"run_id":record.run_id,"receipts":record.receipts,"native_reads":record.source_evidence},
            "outcomes":outcomes}))
    }).collect();
    json!({"schema":"ipe.integration.operation-explanations.v1","as_of":time,"accepted_revision":installed.revisions.accepted,"candidate_revision":installed.revisions.candidate,"operations":operations,
        "coverage":installed.runs.last().map(|r|json!({"run_id":r.id,"status":r.status,"error":r.error,"reads":r.evidence,"complete_declaration_scopes":r.coverage})),
        "publication":false,"interpretation":"Requirements describe external controls. Unknowns remain unknown. Historical success does not authorize a future attempt."})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fact(value: Value, expires_at: u64) -> Record {
        let model:super::super::model::Model=serde_json::from_value(json!({"schema_version":1,"scope":"fixture","claim":"provider_reported","entity":{"kind":"fact","subject":"attempt:1","predicate":"status","value":value,"authority":"fixture","execution":null}})).unwrap();
        Record {
            key: model.key(),
            revision_id: "revision".into(),
            model: Some(model),
            kind: "fact".into(),
            data: json!({}),
            receipts: vec!["receipt".into()],
            source_evidence: vec![],
            observed_at: 1,
            expires_at: Some(expires_at),
            run_id: 1,
            adapter_digest: "mapping".into(),
        }
    }
    #[test]
    fn expired_missing_and_conflicting_facts_never_satisfy_a_requirement() {
        let query = super::super::model::FactQuery {
            subject: "attempt:1".into(),
            predicate: "status".into(),
            expected: json!("success"),
        };
        for (facts, expected) in [
            (vec![], "unknown"),
            (vec![fact(json!("success"), 10)], "unknown"),
            (vec![fact(json!("success"), 11)], "satisfied"),
            (vec![fact(json!("failure"), 11)], "unsatisfied"),
            (vec![fact(json!("failure"), 11), fact(json!("success"), 11)], "unknown"),
        ] {
            assert_eq!(fact_requirement(&query, &facts, 10)["status"], expected);
        }
        let mut wrong = fact(json!("success"), 11);
        if let Some(super::super::model::Model { entity: Entity::Fact { subject, .. }, .. }) =
            &mut wrong.model
        {
            *subject = "attempt:2".into();
        }
        assert_eq!(fact_requirement(&query, &[wrong], 10)["status"], "unknown");
    }
    #[tokio::test]
    async fn authored_constraints_explain_unknowns_and_only_granted_observations() {
        let models = super::super::model_tests::capture(&"a".repeat(40)).await;
        let records: Vec<Record> = models
            .into_iter()
            .map(|m| Record {
                key: m.key(),
                kind: m.kind().into(),
                revision_id: m.revision("mapping").unwrap(),
                model: Some(m),
                data: json!({}),
                receipts: vec!["fixture".into()],
                source_evidence: vec![],
                observed_at: 1,
                expires_at: Some(10),
                run_id: 1,
                adapter_digest: "mapping".into(),
            })
            .collect();
        let mut installed = Installed {
            package: super::super::Installation {
                source: "fixture".into(),
                operations: vec![],
                input: json!({}),
                interval_seconds: 60,
                enabled: false,
                fact_ttl_seconds: 10,
                output_kinds: vec![],
            },
            digest: "mapping".into(),
            next_due: 0,
            current: records.clone(),
            outcomes: Default::default(),
            revisions: Default::default(),
            runs: vec![],
        };
        installed.outcomes.apply(&records).unwrap();
        let projected = project(&installed, 10);
        assert_eq!(projected["operations"].as_array().unwrap().len(), 2);
        let publish = &projected["operations"][1];
        assert_eq!(publish["status"], "unknown");
        assert!(publish["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["observation_not_granted"] == true && r["next_observation"].is_null()));
        assert_eq!(
            publish["outcomes"],
            json!([]),
            "a workflow result and a matrix display name must not prove this job's success"
        );
        installed.package.operations.push("list_jobs".into());
        let projected = project(&installed, 10);
        assert!(projected["operations"][1]["requirements"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["next_observation"]["operation"] == "list_jobs"));
        assert_eq!(projected["publication"], false);
        let control = match &installed
            .current
            .iter()
            .filter(|r| r.kind == "constraint")
            .nth(1)
            .unwrap()
            .model
            .as_ref()
            .unwrap()
            .entity
        {
            Entity::Constraint { control, .. } => control.clone(),
            _ => unreachable!(),
        };
        let execution_key=installed.outcomes.executions.iter().find(|(_,entry)|matches!(&entry.latest.model.as_ref().unwrap().entity,Entity::Outcome{execution,..} if execution.job.is_some())).unwrap().0.clone();
        if let Entity::Outcome { declaration_candidates, .. } = &mut installed
            .outcomes
            .executions
            .get_mut(&execution_key)
            .unwrap()
            .latest
            .model
            .as_mut()
            .unwrap()
            .entity
        {
            *declaration_candidates = vec![control];
        }
        assert_eq!(
            project(&installed, 10)["operations"][1]["outcomes"].as_array().unwrap().len(),
            1
        );
        if let Entity::Outcome { definition, .. } = &mut installed
            .outcomes
            .executions
            .get_mut(&execution_key)
            .unwrap()
            .latest
            .model
            .as_mut()
            .unwrap()
            .entity
        {
            definition.revision = "b".repeat(40);
        }
        assert_eq!(project(&installed, 10)["operations"][1]["outcomes"], json!([]));
    }
}
