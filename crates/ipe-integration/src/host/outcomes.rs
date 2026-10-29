//! External execution history has its own retention, independent of collector logs.
use super::{Record, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(super) const EXECUTION_LIMIT: usize = 256;
const CHANGE_LIMIT: usize = 16;
const BYTE_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct Archive {
    pub executions: BTreeMap<String, Execution>,
    #[serde(default)]
    pub legacy_untyped: BTreeMap<String, Execution>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Execution {
    pub first_observed_at: u64,
    pub last_observed_at: u64,
    pub latest: Record,
    /// One entry per changed provider payload, not one entry per poll.
    pub changes: Vec<Record>,
    pub omitted_changes: usize,
}
impl Archive {
    pub fn migrate_legacy(&mut self) {
        let keys: Vec<_> = self
            .executions
            .iter()
            .filter(|(_, e)| e.latest.model.is_none())
            .map(|(k, _)| k.clone())
            .collect();
        for key in keys {
            if let Some(entry) = self.executions.remove(&key) {
                self.legacy_untyped.insert(key, entry);
            }
        }
    }
    pub fn apply(&mut self, records: &[Record]) -> Result<()> {
        let mut candidate = self.clone();
        for record in records.iter().filter(|r| r.kind == "outcome") {
            let total = candidate.executions.len() + candidate.legacy_untyped.len();
            let collection = if record.model.is_some() {
                &mut candidate.executions
            } else {
                &mut candidate.legacy_untyped
            };
            if let Some(execution) = collection.get_mut(&record.key) {
                if execution.latest.data != record.data || execution.latest.model != record.model {
                    execution.changes.push(record.clone());
                    if execution.changes.len() > CHANGE_LIMIT {
                        execution.changes.remove(0);
                        execution.omitted_changes += 1;
                    }
                }
                execution.last_observed_at = record.observed_at;
                execution.latest = record.clone();
            } else {
                if total >= EXECUTION_LIMIT {
                    return Err("outcome_archive_capacity");
                }
                collection.insert(
                    record.key.clone(),
                    Execution {
                        first_observed_at: record.observed_at,
                        last_observed_at: record.observed_at,
                        latest: record.clone(),
                        changes: vec![record.clone()],
                        omitted_changes: 0,
                    },
                );
            }
        }
        if serde_json::to_vec(&candidate).map_err(|_| "outcome_archive_invalid")?.len() > BYTE_LIMIT
        {
            return Err("outcome_archive_capacity");
        }
        *self = candidate;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn record(key: &str, status: &str) -> Record {
        serde_json::from_value(json!({"kind":"outcome","key":key,"data":{"status":status},"receipts":["test"],"observed_at":1,"expires_at":null,"run_id":1,"adapter_digest":"test"})).unwrap()
    }
    #[test]
    fn archive_overflow_preserves_prior_outcomes_and_change_retention_is_explicit() {
        let mut archive = Archive::default();
        archive.apply(&[record("first", "queued")]).unwrap();
        for i in 0..20 {
            archive.apply(&[record("first", &format!("state_{i}"))]).unwrap();
        }
        assert_eq!(archive.legacy_untyped["first"].changes.len(), CHANGE_LIMIT);
        assert_eq!(archive.legacy_untyped["first"].omitted_changes, 5);
        let before = serde_json::to_value(&archive).unwrap();
        let records: Vec<_> =
            (0..EXECUTION_LIMIT).map(|i| record(&format!("extra_{i}"), "success")).collect();
        assert_eq!(archive.apply(&records), Err("outcome_archive_capacity"));
        assert_eq!(serde_json::to_value(&archive).unwrap(), before);
        let mut large = record("large", "success");
        large.data = json!({"body":"x".repeat(BYTE_LIMIT)});
        assert_eq!(archive.apply(&[large]), Err("outcome_archive_capacity"));
        assert_eq!(serde_json::to_value(&archive).unwrap(), before);
    }
}
