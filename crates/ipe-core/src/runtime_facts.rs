//! Scoped, typed runtime facts with finite lifetimes and durable source ordering.
//!
//! The first implementation stores one serialized state value in RocksDB. This
//! keeps a batch commit atomic and makes restart recovery simple, at the cost of
//! rewriting the bounded state on every commit. Deployments should keep fact
//! volume bounded; a future indexed layout can replace this representation
//! without changing the resolver contract.

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;

const STATE_KEY: &[u8] = b"ipe/runtime-facts/state/v1";
const MAX_KEY_BYTES: usize = 1024;
const MAX_VALUE_BYTES: usize = 1_048_576;
const MAX_NESTING: usize = 32;
const MAX_STATE_KEYS: usize = 10_000;
const MAX_STATE_BYTES: usize = 16 * 1_048_576;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct Scope {
    pub tenant: String,
    pub environment: String,
}

impl Scope {
    pub fn new(tenant: impl Into<String>, environment: impl Into<String>) -> Self {
        Self {
            tenant: tenant.into(),
            environment: environment.into(),
        }
    }

    fn validate(&self) -> Result<(), FactError> {
        if self.tenant.is_empty()
            || self.environment.is_empty()
            || self.tenant.len() > MAX_KEY_BYTES
            || self.environment.len() > MAX_KEY_BYTES
        {
            return Err(FactError::InvalidAddress(
                "scope fields must be non-empty and bounded".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schema {
    Bool,
    Int,
    String,
    Array { items: Box<Schema>, max_items: usize },
    Record { fields: BTreeMap<String, FieldSchema> },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldSchema {
    pub schema: Schema,
    #[serde(default)]
    pub optional: bool,
}

impl Schema {
    pub fn validate_value(&self, value: &Value) -> Result<(), FactError> {
        self.validate_at(value, 0)
    }

    fn validate_at(&self, value: &Value, depth: usize) -> Result<(), FactError> {
        if depth > MAX_NESTING {
            return Err(FactError::InvalidValue("schema nesting limit exceeded".into()));
        }
        match self {
            Schema::Bool if value.is_boolean() => Ok(()),
            Schema::Int if value.as_i64().is_some() => Ok(()),
            Schema::String if value.as_str().is_some() => Ok(()),
            Schema::Array { items, max_items } => {
                let values = value.as_array().ok_or_else(|| type_error("array"))?;
                if values.len() > *max_items {
                    return Err(FactError::InvalidValue(format!(
                        "array has {} items; maximum is {max_items}",
                        values.len()
                    )));
                }
                for item in values {
                    items.validate_at(item, depth + 1)?;
                }
                Ok(())
            },
            Schema::Record { fields } => {
                let object = value.as_object().ok_or_else(|| type_error("record"))?;
                for key in object.keys() {
                    if !fields.contains_key(key) {
                        return Err(FactError::InvalidValue(format!(
                            "unknown record field {key:?}"
                        )));
                    }
                }
                for (name, field) in fields {
                    match object.get(name) {
                        Some(value) => field
                            .schema
                            .validate_at(value, depth + 1)
                            .map_err(|e| e.at_field(name))?,
                        None if field.optional => {},
                        None => {
                            return Err(FactError::InvalidValue(format!(
                                "required field {name:?} is missing"
                            )))
                        },
                    }
                }
                Ok(())
            },
            Schema::Bool | Schema::Int | Schema::String => Err(type_error(match self {
                Schema::Bool => "bool",
                Schema::Int => "signed int64",
                Schema::String => "string",
                _ => unreachable!(),
            })),
        }
    }
}

fn type_error(want: &str) -> FactError {
    FactError::InvalidValue(format!(
        "expected {want}; nulls and floating point values are not supported"
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionMode {
    Push,
    Pull,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CollectionConfig {
    pub schema: Schema,
    pub max_ttl_seconds: u64,
    pub max_observation_age_seconds: u64,
    pub source_id: String,
    pub mode: IngestionMode,
}

impl CollectionConfig {
    fn validate(&self) -> Result<(), FactError> {
        if self.source_id.is_empty() || self.source_id.len() > MAX_KEY_BYTES {
            return Err(FactError::InvalidConfig("source_id must be non-empty and bounded".into()));
        }
        if self.max_ttl_seconds == 0 || self.max_observation_age_seconds == 0 {
            return Err(FactError::InvalidConfig(
                "TTL and observation-age limits must be positive".into(),
            ));
        }
        if matches!(self.mode, IngestionMode::Pull) && self.max_ttl_seconds > i64::MAX as u64 {
            return Err(FactError::InvalidConfig("TTL exceeds timestamp range".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FactRecord {
    pub value: Value,
    pub observed_at: i64,
    pub expires_at: i64,
    pub source_revision: u64,
    pub revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotMetadata {
    pub source_revision: u64,
    pub observed_at: i64,
    pub expires_at: i64,
}

/// Complete pull source envelope applied as one collection generation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotInput {
    pub source_revision: u64,
    pub observed_at: i64,
    pub ttl_seconds: u64,
    pub items: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FactMutation {
    pub key: String,
    /// `Some(value)` writes a fact; `None` writes a tombstone.
    pub value: Option<Value>,
    pub source_revision: u64,
    /// `None` means the key has never had a revision. Deletions and expired
    /// facts retain their revision and must be named by subsequent writes.
    pub expected_revision: Option<u64>,
    pub observed_at: i64,
    /// Required for puts and ignored for deletes.
    pub ttl_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
struct CollectionAddress {
    scope: Scope,
    collection: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
struct FactAddress {
    collection: CollectionAddress,
    key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct KeyState {
    revision: u64,
    source_revision: u64,
    observed_at: i64,
    record: Option<FactRecord>,
    #[serde(default)]
    expires_at: Option<i64>,
    #[serde(default)]
    expired: bool,
    last_mutation: Vec<u8>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct PersistedState {
    next_revision: u64,
    #[serde(with = "map_as_pairs")]
    collections: BTreeMap<CollectionAddress, CollectionConfig>,
    #[serde(with = "map_as_pairs")]
    keys: BTreeMap<FactAddress, KeyState>,
    #[serde(with = "map_as_pairs")]
    snapshot_watermarks: BTreeMap<CollectionAddress, u64>,
    #[serde(with = "map_as_pairs")]
    snapshot_digests: BTreeMap<CollectionAddress, Vec<u8>>,
    #[serde(with = "map_as_pairs")]
    snapshot_expires_at: BTreeMap<CollectionAddress, i64>,
    #[serde(with = "map_as_pairs")]
    snapshot_observed_at: BTreeMap<CollectionAddress, i64>,
}

/// JSON object keys only support strings. Addresses are persisted as ordered
/// key/value pairs so the full structured address remains unambiguous.
mod map_as_pairs {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::BTreeMap;

    pub fn serialize<K: Serialize + Ord, V: Serialize, S: Serializer>(
        map: &BTreeMap<K, V>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        map.iter().collect::<Vec<_>>().serialize(serializer)
    }

    pub fn deserialize<
        'de,
        K: Deserialize<'de> + Ord,
        V: Deserialize<'de>,
        D: Deserializer<'de>,
    >(
        deserializer: D,
    ) -> Result<BTreeMap<K, V>, D::Error> {
        let pairs = Vec::<(K, V)>::deserialize(deserializer)?;
        Ok(pairs.into_iter().collect())
    }
}

struct Inner {
    state: Mutex<PersistedState>,
    db: Option<rocksdb::DB>,
}

/// Thread-safe fact store. Each successful commit persists the complete state
/// in one RocksDB key before publishing the corresponding in-memory state.
#[derive(Clone)]
pub struct FactStore {
    inner: Arc<Inner>,
}

impl FactStore {
    pub fn memory() -> Self {
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(PersistedState::default()),
                db: None,
            }),
        }
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, FactError> {
        let mut options = rocksdb::Options::default();
        options.create_if_missing(true);
        let db = rocksdb::DB::open(&options, path).map_err(storage_error)?;
        let state = match db.get(STATE_KEY).map_err(storage_error)? {
            Some(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| FactError::CorruptState(e.to_string()))?,
            None => PersistedState::default(),
        };
        Ok(Self {
            inner: Arc::new(Inner { state: Mutex::new(state), db: Some(db) }),
        })
    }

    pub fn register_collection(
        &self,
        scope: Scope,
        collection: impl Into<String>,
        config: CollectionConfig,
    ) -> Result<(), FactError> {
        scope.validate()?;
        config.validate()?;
        let collection = collection.into();
        validate_name(&collection, "collection")?;
        let address = CollectionAddress { scope, collection };
        let mut state = self.inner.state.lock();
        match state.collections.get(&address) {
            Some(existing) if existing == &config => Ok(()),
            Some(_) => Err(FactError::CollectionAlreadyConfigured),
            None => {
                let mut next = state.clone();
                next.collections.insert(address, config);
                self.persist_and_publish(&mut state, next)
            },
        }
    }

    pub fn collection_config(
        &self,
        scope: &Scope,
        collection: &str,
    ) -> Result<CollectionConfig, FactError> {
        let state = self.inner.state.lock();
        state
            .collections
            .get(&CollectionAddress {
                scope: scope.clone(),
                collection: collection.to_owned(),
            })
            .cloned()
            .ok_or(FactError::UnknownCollection)
    }

    /// Validates and atomically commits a bounded set of mutations in one
    /// scoped collection. `now` is supplied by the caller to make time
    /// behavior deterministic and shared across the batch.
    pub fn commit(
        &self,
        scope: &Scope,
        collection: &str,
        source_id: &str,
        mutations: Vec<FactMutation>,
        now: i64,
    ) -> Result<Vec<Option<FactRecord>>, FactError> {
        if mutations.is_empty() {
            return Err(FactError::InvalidMutation("empty batch".into()));
        }
        if mutations.len() > 10_000 {
            return Err(FactError::InvalidMutation("batch limit exceeded".into()));
        }
        let address = CollectionAddress {
            scope: scope.clone(),
            collection: collection.to_owned(),
        };
        let mut state = self.inner.state.lock();
        let config =
            state.collections.get(&address).cloned().ok_or(FactError::UnknownCollection)?;
        check_source(&config, source_id, IngestionMode::Push)?;
        let mut next = state.clone();
        let mut seen = BTreeSet::new();
        let mut results = Vec::with_capacity(mutations.len());
        for mutation in mutations {
            validate_key(&mutation.key)?;
            if mutation.observed_at > now {
                return Err(FactError::InvalidMutation(
                    "observed_at cannot be in the future".into(),
                ));
            }
            if !seen.insert(mutation.key.clone()) {
                return Err(FactError::DuplicateKey(mutation.key));
            }
            let fact_address = FactAddress {
                collection: address.clone(),
                key: mutation.key.clone(),
            };
            let prior = next.keys.get(&fact_address).cloned();
            let digest = mutation_digest(&mutation)?;
            if let Some(prior) = &prior {
                if mutation.source_revision == prior.source_revision {
                    if prior.last_mutation == digest {
                        let retried_record = prior.record.clone().or_else(|| {
                            let value = mutation.value.clone()?;
                            Some(FactRecord {
                                value,
                                observed_at: prior.observed_at,
                                expires_at: prior.expires_at?,
                                source_revision: prior.source_revision,
                                revision: prior.revision,
                            })
                        });
                        results.push(retried_record);
                        continue;
                    }
                    return Err(FactError::Replay);
                }
            }
            check_preconditions(
                prior.as_ref(),
                mutation.expected_revision,
                mutation.source_revision,
                mutation.observed_at,
            )?;
            if mutation.value.is_none() && mutation.ttl_seconds.is_some() {
                return Err(FactError::InvalidMutation(
                    "delete must not include ttl_seconds".into(),
                ));
            }
            let record = match mutation.value {
                Some(value) => {
                    validate_value(&config.schema, &value)?;
                    let bytes = serde_json::to_vec(&value)
                        .map_err(|e| FactError::InvalidValue(e.to_string()))?;
                    if bytes.len() > MAX_VALUE_BYTES {
                        return Err(FactError::InvalidValue(
                            "encoded value exceeds size limit".into(),
                        ));
                    }
                    let ttl = mutation.ttl_seconds.ok_or_else(|| {
                        FactError::InvalidMutation("put requires ttl_seconds".into())
                    })?;
                    let expires_at = expiry(&config, mutation.observed_at, ttl, now)?;
                    if prior.as_ref().is_some_and(|p| mutation.observed_at <= p.observed_at) {
                        return Err(FactError::ObservationNotAdvanced);
                    }
                    let revision = take_revision(&mut next)?;
                    Some(FactRecord {
                        value,
                        observed_at: mutation.observed_at.min(now),
                        expires_at,
                        source_revision: mutation.source_revision,
                        revision,
                    })
                },
                None => {
                    let revision = take_revision(&mut next)?;
                    results.push(None);
                    next.keys.insert(
                        fact_address,
                        KeyState {
                            revision,
                            source_revision: mutation.source_revision,
                            observed_at: mutation.observed_at,
                            record: None,
                            expires_at: None,
                            expired: false,
                            last_mutation: digest,
                        },
                    );
                    continue;
                },
            };
            let record = record.expect("put produced a record");
            next.keys.insert(
                fact_address,
                KeyState {
                    revision: record.revision,
                    source_revision: mutation.source_revision,
                    observed_at: mutation.observed_at,
                    record: Some(record.clone()),
                    expires_at: Some(record.expires_at),
                    expired: false,
                    last_mutation: digest,
                },
            );
            results.push(Some(record));
        }
        self.persist_and_publish(&mut state, next)?;
        Ok(results)
    }

    /// Atomically replaces a pull collection with a complete source snapshot.
    /// Reusing an identical source revision is an idempotent no-op, while a
    /// changed payload or envelope is rejected so retries cannot renew TTLs.
    pub fn replace_snapshot(
        &self,
        scope: &Scope,
        collection: &str,
        source_id: &str,
        snapshot: SnapshotInput,
        now: i64,
    ) -> Result<Vec<FactRecord>, FactError> {
        let SnapshotInput {
            source_revision,
            observed_at,
            ttl_seconds,
            items,
        } = snapshot;
        if items.len() > 10_000 {
            return Err(FactError::InvalidMutation("snapshot item limit exceeded".into()));
        }
        let address = CollectionAddress {
            scope: scope.clone(),
            collection: collection.to_owned(),
        };
        let mut state = self.inner.state.lock();
        let config =
            state.collections.get(&address).cloned().ok_or(FactError::UnknownCollection)?;
        check_source(&config, source_id, IngestionMode::Pull)?;
        if let Some(watermark) = state.snapshot_watermarks.get(&address) {
            if source_revision < *watermark {
                return Err(FactError::Replay);
            }
            if source_revision == *watermark {
                // A same-revision retry is idempotent only when it describes
                // the exact currently published generation. Return the old
                // records so retries cannot extend their expiry.
                let digest = snapshot_digest(source_revision, observed_at, ttl_seconds, &items)?;
                if state.snapshot_digests.get(&address) != Some(&digest) {
                    return Err(FactError::Replay);
                }
                let mut retry_records = Vec::with_capacity(items.len());
                for (key, value) in &items {
                    let prior = state
                        .keys
                        .get(&FactAddress {
                            collection: address.clone(),
                            key: key.clone(),
                        })
                        .ok_or(FactError::Replay)?;
                    if prior.source_revision != source_revision || prior.observed_at != observed_at
                    {
                        return Err(FactError::Replay);
                    }
                    retry_records.push(FactRecord {
                        value: value.clone(),
                        observed_at: prior.observed_at,
                        expires_at: prior.expires_at.ok_or(FactError::Replay)?,
                        source_revision,
                        revision: prior.revision,
                    });
                }
                return Ok(retry_records);
            }
            if state
                .snapshot_observed_at
                .get(&address)
                .is_some_and(|previous| observed_at <= *previous)
            {
                return Err(FactError::ObservationNotAdvanced);
            }
        }
        let expires_at = expiry(&config, observed_at, ttl_seconds, now)?;
        let mut next = state.clone();
        let digest = snapshot_digest(source_revision, observed_at, ttl_seconds, &items)?;
        let existing: Vec<(FactAddress, KeyState)> = next
            .keys
            .iter()
            .filter(|(key, _)| key.collection == address)
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        for (key, prior) in existing {
            if source_revision <= prior.source_revision {
                return Err(FactError::Replay);
            }
            if observed_at < prior.observed_at {
                return Err(FactError::ObservedTimeRegression);
            }
            if !items.contains_key(&key.key) {
                let revision = take_revision(&mut next)?;
                next.keys.insert(
                    key,
                    KeyState {
                        revision,
                        source_revision,
                        observed_at,
                        record: None,
                        expires_at: None,
                        expired: false,
                        last_mutation: Vec::new(),
                    },
                );
            }
        }
        let mut result = Vec::with_capacity(items.len());
        for (key, value) in items {
            validate_key(&key)?;
            validate_value(&config.schema, &value)?;
            let bytes =
                serde_json::to_vec(&value).map_err(|e| FactError::InvalidValue(e.to_string()))?;
            if bytes.len() > MAX_VALUE_BYTES {
                return Err(FactError::InvalidValue("encoded value exceeds size limit".into()));
            }
            let fact_address = FactAddress { collection: address.clone(), key };
            if let Some(prior) = next.keys.get(&fact_address) {
                if source_revision <= prior.source_revision {
                    return Err(FactError::Replay);
                }
                if observed_at < prior.observed_at {
                    return Err(FactError::ObservedTimeRegression);
                }
            }
            let revision = take_revision(&mut next)?;
            let record = FactRecord {
                value,
                observed_at: observed_at.min(now),
                expires_at,
                source_revision,
                revision,
            };
            next.keys.insert(
                fact_address,
                KeyState {
                    revision,
                    source_revision,
                    observed_at,
                    record: Some(record.clone()),
                    expires_at: Some(record.expires_at),
                    expired: false,
                    last_mutation: Vec::new(),
                },
            );
            result.push(record);
        }
        next.snapshot_watermarks.insert(address.clone(), source_revision);
        next.snapshot_digests.insert(address.clone(), digest);
        next.snapshot_expires_at.insert(address.clone(), expires_at);
        next.snapshot_observed_at.insert(address, observed_at);
        self.persist_and_publish(&mut state, next)?;
        Ok(result)
    }

    /// Captures an immutable view. All subsequent reads from this value see the
    /// same set of facts even while writers commit newer state.
    pub fn snapshot(&self, scope: Scope) -> FactSnapshot {
        FactSnapshot {
            scope,
            state: Arc::new(self.inner.state.lock().clone()),
        }
    }

    /// Removes expired values while retaining source and local revision
    /// watermarks. Cleanup can reclaim payload space but never permits replay.
    pub fn cleanup_expired(&self, now: i64) -> Result<usize, FactError> {
        let mut state = self.inner.state.lock();
        let mut next = state.clone();
        let mut removed = 0;
        for key_state in next.keys.values_mut() {
            if key_state.record.as_ref().is_some_and(|record| now >= record.expires_at) {
                key_state.record = None;
                key_state.expired = true;
                removed += 1;
            }
        }
        if removed != 0 {
            self.persist_and_publish(&mut state, next)?;
        }
        Ok(removed)
    }

    fn persist_and_publish(
        &self,
        current: &mut PersistedState,
        next: PersistedState,
    ) -> Result<(), FactError> {
        if next.keys.len() > MAX_STATE_KEYS || next.collections.len() > MAX_STATE_KEYS {
            return Err(FactError::StateLimit);
        }
        let bytes = serde_json::to_vec(&next).map_err(|e| FactError::Storage(e.to_string()))?;
        if bytes.len() > MAX_STATE_BYTES {
            return Err(FactError::StateLimit);
        }
        if let Some(db) = &self.inner.db {
            let mut options = rocksdb::WriteOptions::default();
            options.set_sync(true);
            db.put_opt(STATE_KEY, bytes, &options).map_err(storage_error)?;
        }
        *current = next;
        Ok(())
    }
}

#[derive(Clone)]
pub struct FactSnapshot {
    scope: Scope,
    state: Arc<PersistedState>,
}

impl FactSnapshot {
    /// Latest local revision included in this immutable view.
    pub fn revision(&self) -> u64 {
        self.state.next_revision
    }

    pub fn snapshot_metadata(&self, collection: &str) -> Option<SnapshotMetadata> {
        let address = CollectionAddress {
            scope: self.scope.clone(),
            collection: collection.to_owned(),
        };
        Some(SnapshotMetadata {
            source_revision: *self.state.snapshot_watermarks.get(&address)?,
            observed_at: *self.state.snapshot_observed_at.get(&address)?,
            expires_at: *self.state.snapshot_expires_at.get(&address)?,
        })
    }

    pub fn collection_config(&self, collection: &str) -> Result<CollectionConfig, FactError> {
        self.state
            .collections
            .get(&CollectionAddress {
                scope: self.scope.clone(),
                collection: collection.to_owned(),
            })
            .cloned()
            .ok_or(FactError::UnknownCollection)
    }

    pub fn schema(&self, collection: &str) -> Result<Schema, FactError> {
        Ok(self.collection_config(collection)?.schema)
    }

    pub fn collections(&self) -> Vec<(String, CollectionConfig)> {
        self.state
            .collections
            .iter()
            .filter(|(address, _)| address.scope == self.scope)
            .map(|(address, config)| (address.collection.clone(), config.clone()))
            .collect()
    }

    /// Returns the last local revision for a key, including expired facts and
    /// tombstones. This is scoped to the snapshot's exact tenant/environment.
    pub fn key_revision(&self, collection: &str, key: &str) -> Option<u64> {
        self.state
            .keys
            .get(&FactAddress {
                collection: CollectionAddress {
                    scope: self.scope.clone(),
                    collection: collection.to_owned(),
                },
                key: key.to_owned(),
            })
            .map(|state| state.revision)
    }

    pub fn get(
        &self,
        collection: &str,
        key: &str,
        now: i64,
        max_age_seconds: Option<u64>,
    ) -> Result<FactRecord, ResolveError> {
        let address = FactAddress {
            collection: CollectionAddress {
                scope: self.scope.clone(),
                collection: collection.to_owned(),
            },
            key: key.to_owned(),
        };
        let key_state = self.state.keys.get(&address).ok_or(ResolveError::Missing)?;
        let record = match key_state.record.clone() {
            Some(record) => record,
            None if key_state.expired => return Err(ResolveError::Expired),
            None => return Err(ResolveError::Missing),
        };
        if now >= record.expires_at {
            return Err(ResolveError::Expired);
        }
        if let Some(max_age) = max_age_seconds {
            let deadline =
                record.observed_at.saturating_add(i64::try_from(max_age).unwrap_or(i64::MAX));
            if now >= deadline {
                return Err(ResolveError::Stale);
            }
        }
        Ok(record)
    }
}

#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum ResolveError {
    #[error("fact is missing")]
    Missing,
    #[error("fact has expired")]
    Expired,
    #[error("fact is stale")]
    Stale,
}

#[derive(Debug, Error)]
pub enum FactError {
    #[error("invalid address: {0}")]
    InvalidAddress(String),
    #[error("invalid collection configuration: {0}")]
    InvalidConfig(String),
    #[error("invalid fact value: {0}")]
    InvalidValue(String),
    #[error("invalid mutation: {0}")]
    InvalidMutation(String),
    #[error("unknown collection")]
    UnknownCollection,
    #[error("collection is already configured differently")]
    CollectionAlreadyConfigured,
    #[error("source does not own this collection or ingestion mode")]
    UnauthorizedSource,
    #[error("revision conflict")]
    RevisionConflict,
    #[error("source revision replay or regression")]
    Replay,
    #[error("observed_at moved backward")]
    ObservedTimeRegression,
    #[error("observed_at must advance before a write can extend expiry")]
    ObservationNotAdvanced,
    #[error("fact store bounded state limit exceeded")]
    StateLimit,
    #[error("duplicate key in batch: {0}")]
    DuplicateKey(String),
    #[error("write is already expired at acceptance")]
    AlreadyExpired,
    #[error("timestamp arithmetic overflow")]
    TimestampOverflow,
    #[error("durable state is corrupt: {0}")]
    CorruptState(String),
    #[error("storage error: {0}")]
    Storage(String),
}

impl FactError {
    fn at_field(self, field: &str) -> Self {
        match self {
            FactError::InvalidValue(message) => {
                FactError::InvalidValue(format!("field {field:?}: {message}"))
            },
            other => other,
        }
    }
}

fn validate_name(value: &str, what: &str) -> Result<(), FactError> {
    if value.is_empty() || value.len() > MAX_KEY_BYTES {
        return Err(FactError::InvalidAddress(format!("{what} must be non-empty and bounded")));
    }
    Ok(())
}

fn validate_key(key: &str) -> Result<(), FactError> {
    validate_name(key, "key")
}

fn validate_value(schema: &Schema, value: &Value) -> Result<(), FactError> {
    schema.validate_value(value)?;
    let bytes = serde_json::to_vec(value).map_err(|e| FactError::InvalidValue(e.to_string()))?;
    if bytes.len() > MAX_VALUE_BYTES {
        return Err(FactError::InvalidValue("encoded value exceeds size limit".into()));
    }
    Ok(())
}

fn check_source(
    config: &CollectionConfig,
    source_id: &str,
    mode: IngestionMode,
) -> Result<(), FactError> {
    if config.source_id != source_id || config.mode != mode {
        return Err(FactError::UnauthorizedSource);
    }
    Ok(())
}

fn check_preconditions(
    prior: Option<&KeyState>,
    expected: Option<u64>,
    source_revision: u64,
    observed_at: i64,
) -> Result<(), FactError> {
    if prior.map(|k| k.revision) != expected {
        return Err(FactError::RevisionConflict);
    }
    if prior.is_some_and(|k| source_revision <= k.source_revision) {
        return Err(FactError::Replay);
    }
    if prior.is_some_and(|k| observed_at < k.observed_at) {
        return Err(FactError::ObservedTimeRegression);
    }
    Ok(())
}

fn mutation_digest(mutation: &FactMutation) -> Result<Vec<u8>, FactError> {
    let canonical = serde_json::to_vec(&(
        &mutation.key,
        &mutation.value,
        mutation.source_revision,
        mutation.expected_revision,
        mutation.observed_at,
        mutation.ttl_seconds,
    ))
    .map_err(|e| FactError::InvalidMutation(e.to_string()))?;
    Ok(Sha256::digest(canonical).to_vec())
}

fn snapshot_digest(
    source_revision: u64,
    observed_at: i64,
    ttl_seconds: u64,
    items: &BTreeMap<String, Value>,
) -> Result<Vec<u8>, FactError> {
    let canonical = serde_json::to_vec(&(source_revision, observed_at, ttl_seconds, items))
        .map_err(|e| FactError::InvalidMutation(e.to_string()))?;
    Ok(Sha256::digest(canonical).to_vec())
}

fn expiry(
    config: &CollectionConfig,
    observed_at: i64,
    ttl: u64,
    now: i64,
) -> Result<i64, FactError> {
    if observed_at > now {
        return Err(FactError::InvalidMutation("observed_at cannot be in the future".into()));
    }
    if ttl == 0 || ttl > config.max_ttl_seconds {
        return Err(FactError::InvalidMutation(
            "TTL must be positive and within collection limit".into(),
        ));
    }
    let ttl = i64::try_from(ttl).map_err(|_| FactError::TimestampOverflow)?;
    let max_age = i64::try_from(config.max_observation_age_seconds)
        .map_err(|_| FactError::TimestampOverflow)?;
    let expires_at = now
        .checked_add(ttl)
        .ok_or(FactError::TimestampOverflow)?
        .min(observed_at.checked_add(max_age).ok_or(FactError::TimestampOverflow)?);
    if expires_at <= now {
        return Err(FactError::AlreadyExpired);
    }
    Ok(expires_at)
}

fn take_revision(state: &mut PersistedState) -> Result<u64, FactError> {
    state.next_revision = state.next_revision.checked_add(1).ok_or(FactError::TimestampOverflow)?;
    Ok(state.next_revision)
}

fn storage_error(error: rocksdb::Error) -> FactError {
    FactError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config(mode: IngestionMode) -> CollectionConfig {
        CollectionConfig {
            schema: Schema::Record {
                fields: BTreeMap::from([
                    ("enabled".into(), FieldSchema { schema: Schema::Bool, optional: false }),
                    ("count".into(), FieldSchema { schema: Schema::Int, optional: false }),
                    (
                        "labels".into(),
                        FieldSchema {
                            schema: Schema::Array {
                                items: Box::new(Schema::String),
                                max_items: 4,
                            },
                            optional: true,
                        },
                    ),
                ]),
            },
            max_ttl_seconds: 100,
            max_observation_age_seconds: 80,
            source_id: "approvals".into(),
            mode,
        }
    }

    fn store(mode: IngestionMode) -> (FactStore, Scope) {
        let store = FactStore::memory();
        let scope = Scope::new("tenant-a", "prod");
        store.register_collection(scope.clone(), "approvals", config(mode)).unwrap();
        (store, scope)
    }

    fn mutation(
        key: &str,
        value: Option<Value>,
        source_revision: u64,
        expected_revision: Option<u64>,
        observed_at: i64,
        ttl_seconds: Option<u64>,
    ) -> FactMutation {
        FactMutation {
            key: key.into(),
            value,
            source_revision,
            expected_revision,
            observed_at,
            ttl_seconds,
        }
    }

    fn snapshot_input(
        source_revision: u64,
        observed_at: i64,
        ttl_seconds: u64,
        items: BTreeMap<String, Value>,
    ) -> SnapshotInput {
        SnapshotInput {
            source_revision,
            observed_at,
            ttl_seconds,
            items,
        }
    }

    #[test]
    fn validates_closed_records_and_strict_json_types() {
        let schema = config(IngestionMode::Push).schema;
        assert!(schema.validate_value(&json!({"enabled": true, "count": 3})).is_ok());
        for invalid in [
            json!({"enabled": true, "count": 3.0}),
            json!({"enabled": true, "count": null}),
            json!({"enabled": true, "count": 3, "typo": 1}),
            json!({"enabled": true}),
            json!({"enabled": true, "count": 3, "labels": ["ok", 2]}),
        ] {
            assert!(schema.validate_value(&invalid).is_err(), "accepted {invalid}");
        }
        assert!(Schema::Int.validate_value(&json!(i64::MAX)).is_ok());
        assert!(Schema::Int.validate_value(&json!(i64::MAX as u64 + 1)).is_err());
    }

    #[test]
    fn ttl_and_freshness_boundaries_are_exclusive() {
        let (store, scope) = store(IngestionMode::Push);
        let record = store
            .commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation(
                    "k",
                    Some(json!({"enabled": true, "count": 1})),
                    1,
                    None,
                    90,
                    Some(30),
                )],
                100,
            )
            .unwrap()
            .remove(0)
            .unwrap();
        assert_eq!(record.expires_at, 130); // min(now + TTL, observed_at + max age)
        let snapshot = store.snapshot(scope);
        assert!(snapshot.get("approvals", "k", 129, None).is_ok());
        assert_eq!(snapshot.get("approvals", "k", 130, None), Err(ResolveError::Expired));
        assert!(snapshot.get("approvals", "k", 109, Some(20)).is_ok());
        assert_eq!(snapshot.get("approvals", "k", 110, Some(20)), Err(ResolveError::Stale));
    }

    #[test]
    fn batch_failure_is_atomic_and_snapshots_are_immutable() {
        let (store, scope) = store(IngestionMode::Push);
        let first =
            mutation("a", Some(json!({"enabled": true, "count": 1})), 1, None, 100, Some(10));
        let bad =
            mutation("b", Some(json!({"enabled": true, "count": 1.5})), 1, None, 100, Some(10));
        assert!(store.commit(&scope, "approvals", "approvals", vec![first, bad], 100).is_err());
        assert_eq!(
            store.snapshot(scope.clone()).get("approvals", "a", 100, None),
            Err(ResolveError::Missing)
        );
        store
            .commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation(
                    "a",
                    Some(json!({"enabled": true, "count": 1})),
                    1,
                    None,
                    100,
                    Some(10),
                )],
                100,
            )
            .unwrap();
        let old = store.snapshot(scope.clone());
        store
            .commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation("a", None, 2, Some(1), 101, None)],
                101,
            )
            .unwrap();
        assert!(old.get("approvals", "a", 101, None).is_ok());
        assert_eq!(
            store.snapshot(scope).get("approvals", "a", 101, None),
            Err(ResolveError::Missing)
        );
    }

    #[test]
    fn tombstones_preserve_replay_watermark_and_expected_revision() {
        let (store, scope) = store(IngestionMode::Push);
        store
            .commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation(
                    "k",
                    Some(json!({"enabled": true, "count": 1})),
                    10,
                    None,
                    100,
                    Some(10),
                )],
                100,
            )
            .unwrap();
        store
            .commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation("k", None, 11, Some(1), 101, None)],
                101,
            )
            .unwrap();
        assert!(matches!(
            store.commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation(
                    "k",
                    Some(json!({"enabled": true, "count": 2})),
                    10,
                    Some(2),
                    102,
                    Some(10)
                )],
                102
            ),
            Err(FactError::Replay)
        ));
        assert!(matches!(
            store.commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation(
                    "k",
                    Some(json!({"enabled": true, "count": 2})),
                    12,
                    None,
                    102,
                    Some(10)
                )],
                102
            ),
            Err(FactError::RevisionConflict)
        ));
    }

    #[test]
    fn scope_is_structured_and_sources_are_enforced() {
        let store = FactStore::memory();
        let scope1 = Scope::new("a|b", "c");
        let scope2 = Scope::new("a", "b|c");
        store
            .register_collection(scope1.clone(), "facts", config(IngestionMode::Push))
            .unwrap();
        store
            .register_collection(scope2.clone(), "facts", config(IngestionMode::Push))
            .unwrap();
        store
            .commit(
                &scope1,
                "facts",
                "approvals",
                vec![mutation(
                    "same",
                    Some(json!({"enabled": true, "count": 1})),
                    1,
                    None,
                    1,
                    Some(10),
                )],
                1,
            )
            .unwrap();
        assert_eq!(
            store.snapshot(scope2).get("facts", "same", 2, None),
            Err(ResolveError::Missing)
        );
        assert!(matches!(
            store.commit(&scope1, "facts", "other", vec![mutation("x", None, 1, None, 1, None)], 1),
            Err(FactError::UnauthorizedSource)
        ));
    }

    #[test]
    fn pull_snapshots_replace_as_one_generation_and_reject_retry_renewal() {
        let (store, scope) = store(IngestionMode::Pull);
        let values = BTreeMap::from([
            ("a".into(), json!({"enabled": true, "count": 1})),
            ("b".into(), json!({"enabled": true, "count": 2})),
        ]);
        store
            .replace_snapshot(
                &scope,
                "approvals",
                "approvals",
                snapshot_input(7, 100, 20, values.clone()),
                100,
            )
            .unwrap();
        let previous = store.snapshot(scope.clone());
        let retry = store
            .replace_snapshot(
                &scope,
                "approvals",
                "approvals",
                snapshot_input(7, 100, 20, values.clone()),
                105,
            )
            .unwrap();
        assert_eq!(retry[0].expires_at, 120);
        assert_eq!(retry[0].revision, 1);
        assert!(matches!(
            store.replace_snapshot(
                &scope,
                "approvals",
                "approvals",
                snapshot_input(
                    7,
                    100,
                    20,
                    BTreeMap::from([("a".into(), json!({"enabled": true, "count": 99}))])
                ),
                105
            ),
            Err(FactError::Replay)
        ));
        store
            .replace_snapshot(
                &scope,
                "approvals",
                "approvals",
                snapshot_input(
                    8,
                    101,
                    20,
                    BTreeMap::from([("b".into(), json!({"enabled": true, "count": 3}))]),
                ),
                105,
            )
            .unwrap();
        assert_eq!(
            store.snapshot(scope.clone()).get("approvals", "a", 106, None),
            Err(ResolveError::Missing)
        );
        assert_eq!(previous.get("approvals", "a", 106, None).unwrap().value["count"], 1);
        assert_eq!(
            store.snapshot(scope).get("approvals", "b", 106, None).unwrap().value["count"],
            3
        );
    }

    #[test]
    fn empty_pull_generation_still_orders_observations_and_deletes_reject_ttl() {
        {
            let (pull_store, scope) = store(IngestionMode::Pull);
            pull_store
                .replace_snapshot(
                    &scope,
                    "approvals",
                    "approvals",
                    snapshot_input(1, 100, 20, BTreeMap::new()),
                    100,
                )
                .unwrap();
            assert!(matches!(
                pull_store.replace_snapshot(
                    &scope,
                    "approvals",
                    "approvals",
                    snapshot_input(2, 99, 20, BTreeMap::new()),
                    101,
                ),
                Err(FactError::ObservationNotAdvanced)
            ));
        }

        let (store, scope) = store(IngestionMode::Push);
        assert!(matches!(
            store.commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation("k", None, 1, None, 100, Some(1))],
                100,
            ),
            Err(FactError::InvalidMutation(_))
        ));
    }

    #[test]
    fn rocksdb_reopen_preserves_facts_and_replay_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let scope = Scope::new("tenant", "prod");
        {
            let store = FactStore::open(directory.path()).unwrap();
            store
                .register_collection(scope.clone(), "approvals", config(IngestionMode::Push))
                .unwrap();
            store
                .commit(
                    &scope,
                    "approvals",
                    "approvals",
                    vec![mutation(
                        "x",
                        Some(json!({"enabled": true, "count": 1})),
                        3,
                        None,
                        100,
                        Some(10),
                    )],
                    100,
                )
                .unwrap();
        }
        let store = FactStore::open(directory.path()).unwrap();
        assert_eq!(
            store
                .snapshot(scope.clone())
                .get("approvals", "x", 101, None)
                .unwrap()
                .source_revision,
            3
        );
        assert!(matches!(
            store.commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation("x", None, 3, Some(1), 102, None)],
                102
            ),
            Err(FactError::Replay)
        ));
    }

    #[test]
    fn push_retry_returns_original_expiry_and_tombstone_survives_restart() {
        let directory = tempfile::tempdir().unwrap();
        let scope = Scope::new("tenant", "prod");
        {
            let store = FactStore::open(directory.path()).unwrap();
            store
                .register_collection(scope.clone(), "approvals", config(IngestionMode::Push))
                .unwrap();
            let write =
                mutation("x", Some(json!({"enabled": true, "count": 1})), 3, None, 100, Some(10));
            let original = store
                .commit(&scope, "approvals", "approvals", vec![write.clone()], 100)
                .unwrap()
                .remove(0)
                .unwrap();
            let retried = store
                .commit(&scope, "approvals", "approvals", vec![write], 105)
                .unwrap()
                .remove(0)
                .unwrap();
            assert_eq!(retried.revision, original.revision);
            assert_eq!(retried.expires_at, original.expires_at);
            store
                .commit(
                    &scope,
                    "approvals",
                    "approvals",
                    vec![mutation("x", None, 4, Some(original.revision), 106, None)],
                    106,
                )
                .unwrap();
        }
        let store = FactStore::open(directory.path()).unwrap();
        assert_eq!(
            store.snapshot(scope.clone()).get("approvals", "x", 107, None),
            Err(ResolveError::Missing)
        );
        assert!(matches!(
            store.commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation(
                    "x",
                    Some(json!({"enabled": true, "count": 5})),
                    3,
                    Some(2),
                    107,
                    Some(10)
                )],
                107
            ),
            Err(FactError::Replay)
        ));
    }

    #[test]
    fn cleanup_reclaims_values_and_keeps_expired_status_and_replay_watermark() {
        let (store, scope) = store(IngestionMode::Push);
        let write =
            mutation("x", Some(json!({"enabled": true, "count": 1})), 1, None, 100, Some(5));
        let original = store
            .commit(&scope, "approvals", "approvals", vec![write.clone()], 100)
            .unwrap()
            .remove(0)
            .unwrap();
        assert_eq!(store.cleanup_expired(104).unwrap(), 0);
        assert_eq!(store.cleanup_expired(105).unwrap(), 1);
        let snapshot = store.snapshot(scope.clone());
        assert_eq!(snapshot.key_revision("approvals", "x"), Some(original.revision));
        assert_eq!(snapshot.get("approvals", "x", 105, None), Err(ResolveError::Expired));
        assert!(matches!(
            store.commit(
                &scope,
                "approvals",
                "approvals",
                vec![mutation(
                    "x",
                    Some(json!({"enabled": true, "count": 2})),
                    2,
                    Some(original.revision),
                    100,
                    Some(10)
                )],
                106,
            ),
            Err(FactError::ObservationNotAdvanced)
        ));
        let retry = store
            .commit(&scope, "approvals", "approvals", vec![write], 106)
            .unwrap()
            .remove(0)
            .unwrap();
        assert_eq!(retry, original);
    }
}
