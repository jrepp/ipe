//! Offline execution scenarios. Each case gets an isolated fact store and explicit time.
use crate::policy_document::{DocumentResult, ExecutionStatus, PolicyDocument};
use crate::runtime_facts::{
    CollectionConfig, FactMutation, FactStore, IngestionMode, Schema, Scope,
};
use crate::runtime_policy::CompileEnvironment;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioSuite {
    pub input_schema: Schema,
    pub collections: BTreeMap<String, CollectionConfig>,
    /// Cases select immutable document fixtures by index, permitting revision comparisons.
    pub documents: Vec<PolicyDocument>,
    pub cases: Vec<Scenario>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub name: String,
    pub document: usize,
    pub input: Value,
    pub now: i64,
    pub completed_at: i64,
    pub facts: Vec<FixtureFact>,
    pub expected: ExecutionStatus,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureFact {
    pub collection: String,
    pub key: String,
    pub value: Value,
    pub observed_at: i64,
    pub received_at: i64,
    pub ttl_seconds: u64,
}
#[derive(Serialize)]
pub struct ScenarioReport {
    pub passed: bool,
    pub cases: Vec<ScenarioResult>,
}
#[derive(Serialize)]
pub struct ScenarioResult {
    pub name: String,
    pub document_digest: String,
    pub expected: ExecutionStatus,
    pub passed: bool,
    pub result: DocumentResult,
}
impl ScenarioSuite {
    pub fn run(&self) -> Result<ScenarioReport, String> {
        if self.cases.is_empty()
            || self.cases.len() > 1000
            || self.documents.is_empty()
            || self.documents.len() > 100
        {
            return Err("suite requires 1..100 documents and 1..1000 cases".into());
        }
        let mut environment = CompileEnvironment::new(self.input_schema.clone());
        for (name, config) in &self.collections {
            environment = environment.collection(name.clone(), config.schema.clone());
        }
        let documents = self
            .documents
            .iter()
            .map(|d| d.prepare(&environment).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        let scope = Scope::new("scenario", "isolated");
        let mut names = BTreeSet::new();
        let mut cases = Vec::new();
        for case in &self.cases {
            if case.name.trim().is_empty()
                || !names.insert(&case.name)
                || case.now < 0
                || case.completed_at < case.now
            {
                return Err("invalid case name or clock".into());
            }
            let document =
                documents.get(case.document).ok_or("case references an unknown document")?;
            self.input_schema.validate_value(&case.input).map_err(|e| e.to_string())?;
            let store = FactStore::memory();
            for (name, config) in &self.collections {
                store
                    .register_collection(scope.clone(), name.clone(), config.clone())
                    .map_err(|e| e.to_string())?;
            }
            let mut keys = BTreeSet::new();
            for fact in &case.facts {
                if !keys.insert((&fact.collection, &fact.key)) || fact.received_at > case.now {
                    return Err("duplicate fixture fact or future receipt".into());
                }
                let config =
                    self.collections.get(&fact.collection).ok_or("unknown fixture collection")?;
                if config.mode == IngestionMode::Pull {
                    return Err("scenario fixtures currently require push collections".into());
                }
                store
                    .commit(
                        &scope,
                        &fact.collection,
                        &config.source_id,
                        vec![FactMutation {
                            key: fact.key.clone(),
                            value: Some(fact.value.clone()),
                            expected_revision: None,
                            source_revision: 1,
                            observed_at: fact.observed_at,
                            ttl_seconds: Some(fact.ttl_seconds),
                        }],
                        fact.received_at,
                    )
                    .map_err(|e| e.to_string())?;
            }
            let result = document.evaluate(
                &case.input,
                &store.snapshot(scope.clone()),
                case.now,
                case.completed_at,
            );
            cases.push(ScenarioResult {
                name: case.name.clone(),
                document_digest: self.documents[case.document].digest(),
                passed: result.status == case.expected,
                expected: case.expected.clone(),
                result,
            });
        }
        Ok(ScenarioReport {
            passed: cases.iter().all(|c| c.passed),
            cases,
        })
    }
}
