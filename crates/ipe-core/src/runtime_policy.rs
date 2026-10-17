//! Versioned language-2 policy parser, type checker, and bounded evaluator.
//!
//! This module is intentionally independent from the legacy parser. Policies
//! must opt into `language 2`; parsing never falls back to the legacy grammar.

use crate::runtime_facts::{FactSnapshot, ResolveError, Schema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use thiserror::Error;

pub const LANGUAGE_VERSION: u32 = 2;

/// Schema environment used when compiling a language-2 policy. `input` is the
/// validated request schema and each collection entry is the value schema for
/// the named, statically registered collection.
#[derive(Clone, Debug, Default)]
pub struct CompileEnvironment {
    pub input: Option<Schema>,
    pub collections: BTreeMap<String, Schema>,
    pub policy_data: Option<PolicyData>,
}

impl CompileEnvironment {
    pub fn new(input: Schema) -> Self {
        Self {
            input: Some(input),
            collections: BTreeMap::new(),
            policy_data: None,
        }
    }
    pub fn policy_data(mut self, data: PolicyData) -> Self {
        self.policy_data = Some(data);
        self
    }
    pub fn collection(mut self, name: impl Into<String>, schema: Schema) -> Self {
        self.collections.insert(name.into(), schema);
        self
    }
}

/// Immutable typed parameters owned by a policy revision, exposed as `policy`.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyData {
    pub schema: Schema,
    pub value: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledPolicy {
    pub language_version: u32,
    pub name: String,
    pub intent: String,
    pub triggers: Expr,
    pub facts: Vec<FactBinding>,
    pub requires: Expr,
    pub input_schema: Schema,
    /// Collection schemas are pinned into the compiled artifact so reloads can
    /// reject a catalog that no longer matches the policy's compilation.
    pub collection_schemas: BTreeMap<String, Schema>,
    #[serde(default)]
    pub policy_data: Option<PolicyData>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FactBinding {
    pub name: String,
    pub collection: String,
    pub key: Expr,
    pub fresh_within_seconds: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    String(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Literal(Literal),
    Path {
        root: String,
        segments: Vec<String>,
    },
    List(Vec<Expr>),
    Not(Box<Expr>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Compare {
        left: Box<Expr>,
        op: CompareOp,
        right: Box<Expr>,
    },
    In {
        item: Box<Expr>,
        list: Box<Expr>,
    },
    Aggregate {
        kind: AggregateKind,
        value: Box<Expr>,
        variable: String,
        list: Box<Expr>,
        where_clause: Option<Box<Expr>>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum AggregateKind {
    Any,
    All,
    Count,
    CountDistinct,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyOutcome {
    NotApplicable,
    Allow,
    Deny,
    Indeterminate(PolicyFailure),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationResult {
    pub outcome: PolicyOutcome,
    /// Earliest exclusive deadline among successfully consumed facts. A caller
    /// must confirm this remains greater than its completion clock before it
    /// returns an allow to its caller.
    pub evidence_deadline: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PolicyFailure {
    MissingInput(String),
    MissingFact { binding: String },
    ExpiredFact { binding: String },
    StaleFact { binding: String },
    InvalidFact { binding: String },
    InvalidPolicy(String),
    TypeMismatch(String),
    BudgetExceeded,
    Evaluation(String),
}

#[derive(Clone, Copy, Debug)]
pub struct EvalBudget {
    pub max_steps: usize,
    pub max_collection_items: usize,
}

impl Default for EvalBudget {
    fn default() -> Self {
        Self {
            max_steps: 10_000,
            max_collection_items: 1_000,
        }
    }
}

#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum PolicyCompileError {
    #[error("parse error at {line}:{column}: {message}")]
    Parse { line: usize, column: usize, message: String },
    #[error("type error: {0}")]
    Type(String),
    #[error("language version {0} is not supported")]
    UnsupportedVersion(u32),
}

impl CompiledPolicy {
    pub fn compile(
        source: &str,
        environment: &CompileEnvironment,
    ) -> Result<Self, PolicyCompileError> {
        if source.len() > 65_536 {
            return Err(PolicyCompileError::Parse {
                line: 1,
                column: 1,
                message: "policy source exceeds 65536 bytes".into(),
            });
        }
        let mut parser = Parser::new(source)?;
        let mut policy = parser.parse_policy()?;
        let input_schema = environment
            .input
            .as_ref()
            .ok_or_else(|| PolicyCompileError::Type("an input schema is required".into()))?;
        let mut schemas: BTreeMap<String, Schema> = policy
            .facts
            .iter()
            .filter_map(|f| {
                environment
                    .collections
                    .get(&f.collection)
                    .map(|s| (f.name.clone(), fact_schema(s)))
            })
            .collect();
        let mut constants = BTreeMap::new();
        if let Some(data) = &environment.policy_data {
            data.schema.validate_value(&data.value).map_err(|e| type_error(e.to_string()))?;
            if serde_json::to_vec(data).map_err(|e| type_error(e.to_string()))?.len() > 65_536 {
                return Err(type_error("policy data exceeds 65536 bytes"));
            }
            constants.insert("policy".into(), data.schema.clone());
            schemas.extend(constants.clone());
        }
        policy.policy_data = environment.policy_data.clone();
        for binding in &policy.facts {
            if matches!(binding.name.as_str(), "input" | "data" | "policy") {
                return Err(PolicyCompileError::Type(format!(
                    "fact binding name {:?} is reserved",
                    binding.name
                )));
            }
            environment.collections.get(&binding.collection).ok_or_else(|| {
                PolicyCompileError::Type(format!(
                    "collection {:?} is not registered",
                    binding.collection
                ))
            })?;
            check_expr(&binding.key, input_schema, &constants, &[], None, &Type::String)?;
        }
        let bool_ty = Type::Bool;
        check_expr(&policy.triggers, input_schema, &constants, &[], None, &bool_ty)?;
        check_expr(&policy.requires, input_schema, &schemas, &policy.facts, None, &bool_ty)?;
        let aliases: HashSet<String> = policy.facts.iter().map(|fact| fact.name.clone()).collect();
        check_aggregate_names(&policy.requires, &aliases, &mut HashSet::new())?;
        policy.collection_schemas = policy
            .facts
            .iter()
            .map(|f| (f.collection.clone(), environment.collections[&f.collection].clone()))
            .collect();
        policy.input_schema = input_schema.clone();
        Ok(policy)
    }

    /// Evaluate against one immutable fact snapshot and one caller-supplied
    /// clock value (Unix seconds). Required dependencies resolve after a true
    /// trigger and before any requirement expression is evaluated.
    pub fn evaluate(
        &self,
        input: &Value,
        snapshot: &FactSnapshot,
        now: i64,
        budget: EvalBudget,
    ) -> EvaluationResult {
        if self.language_version != LANGUAGE_VERSION {
            return EvaluationResult {
                outcome: PolicyOutcome::Indeterminate(PolicyFailure::InvalidPolicy(format!(
                    "unsupported compiled language version {}",
                    self.language_version
                ))),
                evidence_deadline: None,
            };
        }
        if self.input_schema.validate_value(input).is_err() {
            return EvaluationResult {
                outcome: PolicyOutcome::Indeterminate(PolicyFailure::TypeMismatch(
                    "input does not match the compiled request schema".into(),
                )),
                evidence_deadline: None,
            };
        }
        let mut state = EvalState {
            input,
            policy_data: self.policy_data.as_ref().map(|data| &data.value),
            budget,
            steps: 0,
            items: 0,
            facts: HashMap::new(),
            locals: Vec::new(),
            evidence_deadline: None,
        };
        if let Some(data) = &self.policy_data {
            if data.schema.validate_value(&data.value).is_err() {
                return EvaluationResult {
                    outcome: PolicyOutcome::Indeterminate(PolicyFailure::InvalidPolicy(
                        "invalid policy data".into(),
                    )),
                    evidence_deadline: None,
                };
            }
        }
        match state.eval_bool(&self.triggers) {
            Ok(false) => {
                return EvaluationResult {
                    outcome: PolicyOutcome::NotApplicable,
                    evidence_deadline: None,
                }
            },
            Ok(true) => {},
            Err(e) => {
                return EvaluationResult {
                    outcome: PolicyOutcome::Indeterminate(e),
                    evidence_deadline: None,
                }
            },
        }
        // Resolve every declared dependency eagerly. A short-circuited `or`
        // cannot hide an absent, expired, or stale declared dependency.
        for binding in &self.facts {
            let expected_schema = match self.collection_schemas.get(&binding.collection) {
                Some(schema) => schema,
                None => {
                    return EvaluationResult {
                        outcome: PolicyOutcome::Indeterminate(PolicyFailure::InvalidFact {
                            binding: binding.name.clone(),
                        }),
                        evidence_deadline: state.evidence_deadline,
                    }
                },
            };
            if snapshot.schema(&binding.collection).ok().as_ref() != Some(expected_schema) {
                return EvaluationResult {
                    outcome: PolicyOutcome::Indeterminate(PolicyFailure::InvalidFact {
                        binding: binding.name.clone(),
                    }),
                    evidence_deadline: state.evidence_deadline,
                };
            }
            let key = match state.eval(&binding.key) {
                Ok(Value::String(key)) => key,
                Ok(_) => {
                    return EvaluationResult {
                        outcome: PolicyOutcome::Indeterminate(PolicyFailure::TypeMismatch(
                            format!("fact key for {} must be a string", binding.name),
                        )),
                        evidence_deadline: None,
                    }
                },
                Err(e) => {
                    return EvaluationResult {
                        outcome: PolicyOutcome::Indeterminate(e),
                        evidence_deadline: None,
                    }
                },
            };
            match snapshot.get(&binding.collection, &key, now, binding.fresh_within_seconds) {
                Ok(record) => {
                    let schema = match self.collection_schemas.get(&binding.collection) {
                        Some(s) => s,
                        None => {
                            return EvaluationResult {
                                outcome: PolicyOutcome::Indeterminate(PolicyFailure::InvalidFact {
                                    binding: binding.name.clone(),
                                }),
                                evidence_deadline: None,
                            }
                        },
                    };
                    if schema.validate_value(&record.value).is_err() {
                        return EvaluationResult {
                            outcome: PolicyOutcome::Indeterminate(PolicyFailure::InvalidFact {
                                binding: binding.name.clone(),
                            }),
                            evidence_deadline: None,
                        };
                    }
                    let freshness_deadline = binding.fresh_within_seconds.map(|age| {
                        record.observed_at.saturating_add(i64::try_from(age).unwrap_or(i64::MAX))
                    });
                    let deadline =
                        freshness_deadline.map_or(record.expires_at, |d| d.min(record.expires_at));
                    state.evidence_deadline =
                        Some(state.evidence_deadline.map_or(deadline, |prior| prior.min(deadline)));
                    let mut metadata = serde_json::Map::new();
                    metadata.insert("observed_at".into(), Value::from(record.observed_at));
                    metadata.insert("expires_at".into(), Value::from(record.expires_at));
                    metadata.insert(
                        "source_revision".into(),
                        Value::from(record.source_revision.to_string()),
                    );
                    metadata.insert("revision".into(), Value::from(record.revision.to_string()));
                    let mut fact = serde_json::Map::new();
                    fact.insert("value".into(), record.value);
                    fact.insert("metadata".into(), Value::Object(metadata));
                    state.facts.insert(binding.name.clone(), Value::Object(fact));
                },
                Err(ResolveError::Missing) => {
                    return EvaluationResult {
                        outcome: PolicyOutcome::Indeterminate(PolicyFailure::MissingFact {
                            binding: binding.name.clone(),
                        }),
                        evidence_deadline: state.evidence_deadline,
                    }
                },
                Err(ResolveError::Expired) => {
                    return EvaluationResult {
                        outcome: PolicyOutcome::Indeterminate(PolicyFailure::ExpiredFact {
                            binding: binding.name.clone(),
                        }),
                        evidence_deadline: state.evidence_deadline,
                    }
                },
                Err(ResolveError::Stale) => {
                    return EvaluationResult {
                        outcome: PolicyOutcome::Indeterminate(PolicyFailure::StaleFact {
                            binding: binding.name.clone(),
                        }),
                        evidence_deadline: state.evidence_deadline,
                    }
                },
            }
        }
        let outcome = match state.eval_bool(&self.requires) {
            Ok(true) => PolicyOutcome::Allow,
            Ok(false) => PolicyOutcome::Deny,
            Err(e) => PolicyOutcome::Indeterminate(e),
        };
        EvaluationResult {
            outcome,
            evidence_deadline: state.evidence_deadline,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Type {
    Bool,
    Int,
    String,
    Array(Box<Type>),
    Record(BTreeMap<String, (Type, bool)>),
    Unknown,
}

fn schema_type(schema: &Schema) -> Type {
    match schema {
        Schema::Bool => Type::Bool,
        Schema::Int => Type::Int,
        Schema::String => Type::String,
        Schema::Array { items, .. } => Type::Array(Box::new(schema_type(items))),
        Schema::Record { fields } => Type::Record(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), (schema_type(&v.schema), v.optional)))
                .collect(),
        ),
    }
}

fn fact_schema(value: &Schema) -> Schema {
    let mut metadata = BTreeMap::new();
    for field in ["observed_at", "expires_at", "source_revision", "revision"] {
        let schema = if field == "source_revision" || field == "revision" {
            Schema::String
        } else {
            Schema::Int
        };
        metadata.insert(
            field.to_owned(),
            crate::runtime_facts::FieldSchema { schema, optional: false },
        );
    }
    let mut fields = BTreeMap::new();
    fields.insert(
        "value".into(),
        crate::runtime_facts::FieldSchema { schema: value.clone(), optional: false },
    );
    fields.insert(
        "metadata".into(),
        crate::runtime_facts::FieldSchema {
            schema: Schema::Record { fields: metadata },
            optional: false,
        },
    );
    Schema::Record { fields }
}

fn type_error(msg: impl Into<String>) -> PolicyCompileError {
    PolicyCompileError::Type(msg.into())
}

fn check_aggregate_names(
    expr: &Expr,
    aliases: &HashSet<String>,
    locals: &mut HashSet<String>,
) -> Result<(), PolicyCompileError> {
    match expr {
        Expr::Not(inner) => check_aggregate_names(inner, aliases, locals),
        Expr::And(parts) | Expr::Or(parts) | Expr::List(parts) => {
            for part in parts {
                check_aggregate_names(part, aliases, locals)?;
            }
            Ok(())
        },
        Expr::Compare { left, right, .. } => {
            check_aggregate_names(left, aliases, locals)?;
            check_aggregate_names(right, aliases, locals)
        },
        Expr::In { item, list } => {
            check_aggregate_names(item, aliases, locals)?;
            check_aggregate_names(list, aliases, locals)
        },
        Expr::Aggregate { value, variable, list, where_clause, .. } => {
            if aliases.contains(variable) || locals.contains(variable) {
                return Err(type_error(format!(
                    "aggregate variable {variable:?} shadows a fact binding or enclosing variable"
                )));
            }
            locals.insert(variable.clone());
            check_aggregate_names(list, aliases, locals)?;
            if let Some(filter) = where_clause {
                check_aggregate_names(filter, aliases, locals)?;
            }
            check_aggregate_names(value, aliases, locals)?;
            locals.remove(variable);
            Ok(())
        },
        Expr::Literal(_) | Expr::Path { .. } => Ok(()),
    }
}

fn check_expr(
    expr: &Expr,
    input: &Schema,
    fact_schemas: &BTreeMap<String, Schema>,
    bindings: &[FactBinding],
    locals: Option<&HashMap<String, Type>>,
    expected: &Type,
) -> Result<Type, PolicyCompileError> {
    let got = match expr {
        Expr::Literal(Literal::Bool(_)) => Type::Bool,
        Expr::Literal(Literal::Int(_)) => Type::Int,
        Expr::Literal(Literal::String(_)) => Type::String,
        Expr::List(values) => {
            let mut ty = None;
            for value in values {
                let t = check_expr(value, input, fact_schemas, bindings, locals, &Type::Unknown)?;
                if let Some(prev) = &ty {
                    if prev != &t {
                        return Err(type_error("list items must have one type"));
                    }
                } else {
                    ty = Some(t);
                }
            }
            Type::Array(Box::new(ty.unwrap_or(Type::Unknown)))
        },
        Expr::Path { root, segments } => {
            if let Some(local) = locals.and_then(|m| m.get(root)) {
                walk_type(local.clone(), segments)?
            } else if root == "input" {
                walk_type(schema_type(input), segments)?
            } else if root == "policy" {
                let schema = fact_schemas
                    .get(root)
                    .ok_or_else(|| type_error("policy data is not configured"))?;
                walk_type(schema_type(schema), segments)?
            } else if let Some(binding) = bindings.iter().find(|b| &b.name == root) {
                let schema = fact_schemas
                    .get(root)
                    .ok_or_else(|| type_error(format!("no schema is pinned for fact {root:?}")))?;
                let _ = binding;
                walk_type(schema_type(schema), segments)?
            } else {
                return Err(type_error(format!("unknown path root {root:?}")));
            }
        },
        Expr::Not(inner) => {
            require(
                check_expr(inner, input, fact_schemas, bindings, locals, &Type::Bool)?,
                &Type::Bool,
                "not requires bool",
            )?;
            Type::Bool
        },
        Expr::And(parts) | Expr::Or(parts) => {
            for part in parts {
                require(
                    check_expr(part, input, fact_schemas, bindings, locals, &Type::Bool)?,
                    &Type::Bool,
                    "and/or require bool",
                )?;
            }
            Type::Bool
        },
        Expr::Compare { left, op, right } => {
            let l = check_expr(left, input, fact_schemas, bindings, locals, &Type::Unknown)?;
            let r = check_expr(right, input, fact_schemas, bindings, locals, &Type::Unknown)?;
            if l != r {
                return Err(type_error(format!(
                    "comparison requires matching types, got {l:?} and {r:?}"
                )));
            }
            if !matches!(l, Type::Bool | Type::Int | Type::String) {
                return Err(type_error("only bool, int, and string values can be compared"));
            }
            if !matches!(op, CompareOp::Eq | CompareOp::Ne) && l == Type::Bool {
                return Err(type_error("bool values support equality only"));
            }
            Type::Bool
        },
        Expr::In { item, list } => {
            let item_ty = check_expr(item, input, fact_schemas, bindings, locals, &Type::Unknown)?;
            let list_ty = check_expr(list, input, fact_schemas, bindings, locals, &Type::Unknown)?;
            match list_ty {
                Type::Array(elem) if *elem == item_ty => Type::Bool,
                Type::Array(elem) if *elem == Type::Unknown => Type::Bool,
                _ => {
                    return Err(type_error(
                        "membership requires an item and a list of the same type",
                    ))
                },
            }
        },
        Expr::Aggregate {
            kind,
            value,
            variable,
            list,
            where_clause,
        } => {
            let list_ty = check_expr(list, input, fact_schemas, bindings, locals, &Type::Unknown)?;
            let elem = match list_ty {
                Type::Array(elem) => *elem,
                _ => return Err(type_error("aggregate input must be a list")),
            };
            let mut nested = locals.cloned().unwrap_or_default();
            nested.insert(variable.clone(), elem);
            if let Some(filter) = where_clause {
                require(
                    check_expr(filter, input, fact_schemas, bindings, Some(&nested), &Type::Bool)?,
                    &Type::Bool,
                    "aggregate where clause requires bool",
                )?;
            }
            let val_ty =
                check_expr(value, input, fact_schemas, bindings, Some(&nested), &Type::Unknown)?;
            match kind {
                AggregateKind::Any | AggregateKind::All => {
                    require(val_ty, &Type::Bool, "any/all expression requires bool")?;
                    Type::Bool
                },
                AggregateKind::Count => Type::Int,
                AggregateKind::CountDistinct => {
                    if !matches!(val_ty, Type::String | Type::Int | Type::Bool) {
                        return Err(type_error("count_distinct requires a scalar value"));
                    }
                    Type::Int
                },
            }
        },
    };
    if *expected != Type::Unknown && got != *expected {
        return Err(type_error(format!("expected {expected:?}, got {got:?}")));
    }
    Ok(got)
}

fn require(got: Type, expected: &Type, message: &str) -> Result<(), PolicyCompileError> {
    if &got == expected {
        Ok(())
    } else {
        Err(type_error(format!("{message}: got {got:?}")))
    }
}

fn walk_type(mut ty: Type, segments: &[String]) -> Result<Type, PolicyCompileError> {
    for segment in segments {
        ty = match ty {
            Type::Record(fields) => {
                let (field, optional) = fields
                    .get(segment)
                    .ok_or_else(|| type_error(format!("unknown field {segment:?}")))?;
                if *optional {
                    return Err(type_error(format!("optional field {segment:?} must be explicitly matched before access (optional access is not yet supported)")));
                }
                field.clone()
            },
            _ => return Err(type_error(format!("cannot access field {segment:?} on {ty:?}"))),
        };
    }
    Ok(ty)
}

struct EvalState<'a> {
    input: &'a Value,
    policy_data: Option<&'a Value>,
    budget: EvalBudget,
    steps: usize,
    items: usize,
    facts: HashMap<String, Value>,
    locals: Vec<(String, Value)>,
    evidence_deadline: Option<i64>,
}
impl EvalState<'_> {
    fn step(&mut self) -> Result<(), PolicyFailure> {
        self.steps += 1;
        if self.steps > self.budget.max_steps {
            Err(PolicyFailure::BudgetExceeded)
        } else {
            Ok(())
        }
    }
    fn eval_bool(&mut self, expr: &Expr) -> Result<bool, PolicyFailure> {
        match self.eval(expr)? {
            Value::Bool(b) => Ok(b),
            _ => Err(PolicyFailure::TypeMismatch("expected bool".into())),
        }
    }
    fn eval(&mut self, expr: &Expr) -> Result<Value, PolicyFailure> {
        self.step()?;
        match expr {
            Expr::Literal(Literal::Bool(v)) => Ok(Value::Bool(*v)),
            Expr::Literal(Literal::Int(v)) => Ok(Value::from(*v)),
            Expr::Literal(Literal::String(v)) => Ok(Value::String(v.clone())),
            Expr::Path { root, segments } => {
                let base = self
                    .locals
                    .iter()
                    .rev()
                    .find(|(k, _)| k == root)
                    .map(|(_, v)| v)
                    .or_else(
                        || if root == "policy" { self.policy_data } else { self.facts.get(root) },
                    )
                    .or_else(|| if root == "input" { Some(self.input) } else { None })
                    .ok_or_else(|| PolicyFailure::MissingInput(root.clone()))?;
                let mut value = base;
                for segment in segments {
                    value = value
                        .get(segment)
                        .ok_or_else(|| PolicyFailure::MissingInput(format!("{root}.{segment}")))?;
                }
                Ok(value.clone())
            },
            Expr::List(items) => items
                .iter()
                .map(|e| self.eval(e))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array),
            Expr::Not(e) => Ok(Value::Bool(!self.eval_bool(e)?)),
            Expr::And(parts) => {
                for e in parts {
                    if !self.eval_bool(e)? {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            },
            Expr::Or(parts) => {
                for e in parts {
                    if self.eval_bool(e)? {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            },
            Expr::Compare { left, op, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                compare(&l, *op, &r).map(Value::Bool).ok_or_else(|| {
                    PolicyFailure::TypeMismatch(
                        "comparison operands have different or unsupported types".into(),
                    )
                })
            },
            Expr::In { item, list } => {
                let item = self.eval(item)?;
                let list = self.eval(list)?;
                let values = list.as_array().ok_or_else(|| {
                    PolicyFailure::TypeMismatch("membership target must be a list".into())
                })?;
                self.items = self.items.saturating_add(values.len());
                if self.items > self.budget.max_collection_items {
                    return Err(PolicyFailure::BudgetExceeded);
                }
                Ok(Value::Bool(values.iter().any(|v| v == &item)))
            },
            Expr::Aggregate {
                kind,
                value,
                variable,
                list,
                where_clause,
            } => {
                let items = match self.eval(list)? {
                    Value::Array(items) => items,
                    _ => {
                        return Err(PolicyFailure::TypeMismatch(
                            "aggregate input must be a list".into(),
                        ))
                    },
                };
                self.items = self.items.saturating_add(items.len());
                if self.items > self.budget.max_collection_items {
                    return Err(PolicyFailure::BudgetExceeded);
                }
                let mut count = 0usize;
                let mut distinct = HashSet::new();
                for item in items {
                    self.locals.push((variable.clone(), item));
                    let result = (|| -> Result<(bool, Option<Value>), PolicyFailure> {
                        let include = if let Some(filter) = where_clause {
                            self.eval_bool(filter)?
                        } else {
                            true
                        };
                        if !include || *kind == AggregateKind::Count {
                            return Ok((include, None));
                        }
                        Ok((true, Some(self.eval(value)?)))
                    })();
                    self.locals.pop();
                    let (included, value) = result?;
                    if included {
                        count = count.saturating_add(1);
                    }
                    if let Some(value) = value {
                        match kind {
                            AggregateKind::Any => {
                                if value.as_bool().ok_or_else(|| {
                                    PolicyFailure::TypeMismatch(
                                        "any expression must be bool".into(),
                                    )
                                })? {
                                    return Ok(Value::Bool(true));
                                }
                            },
                            AggregateKind::All => {
                                if !value.as_bool().ok_or_else(|| {
                                    PolicyFailure::TypeMismatch(
                                        "all expression must be bool".into(),
                                    )
                                })? {
                                    return Ok(Value::Bool(false));
                                }
                            },
                            AggregateKind::CountDistinct => {
                                distinct.insert(ScalarKey::from_value(&value).ok_or_else(
                                    || {
                                        PolicyFailure::TypeMismatch(
                                            "count_distinct expression must be scalar".into(),
                                        )
                                    },
                                )?);
                            },
                            AggregateKind::Count => unreachable!(),
                        }
                    }
                }
                match kind {
                    AggregateKind::Any => Ok(Value::Bool(false)),
                    AggregateKind::All => Ok(Value::Bool(true)),
                    AggregateKind::Count => Ok(Value::from(count as u64)),
                    AggregateKind::CountDistinct => Ok(Value::from(distinct.len() as u64)),
                }
            },
        }
    }
}

fn compare(a: &Value, op: CompareOp, b: &Value) -> Option<bool> {
    if let (Some(a), Some(b)) = (a.as_i64(), b.as_i64()) {
        return Some(match op {
            CompareOp::Eq => a == b,
            CompareOp::Ne => a != b,
            CompareOp::Lt => a < b,
            CompareOp::Le => a <= b,
            CompareOp::Gt => a > b,
            CompareOp::Ge => a >= b,
        });
    }
    if let (Some(a), Some(b)) = (a.as_str(), b.as_str()) {
        return Some(match op {
            CompareOp::Eq => a == b,
            CompareOp::Ne => a != b,
            CompareOp::Lt => a < b,
            CompareOp::Le => a <= b,
            CompareOp::Gt => a > b,
            CompareOp::Ge => a >= b,
        });
    }
    if let (Some(a), Some(b)) = (a.as_bool(), b.as_bool()) {
        return Some(match op {
            CompareOp::Eq => a == b,
            CompareOp::Ne => a != b,
            _ => return None,
        });
    }
    None
}

#[derive(Hash, Eq, PartialEq)]
enum ScalarKey {
    Bool(bool),
    Int(i64),
    String(String),
}

impl ScalarKey {
    fn from_value(value: &Value) -> Option<Self> {
        if let Some(v) = value.as_bool() {
            return Some(Self::Bool(v));
        }
        if let Some(v) = value.as_i64() {
            return Some(Self::Int(v));
        }
        value.as_str().map(|v| Self::String(v.to_owned()))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    line: usize,
    column: usize,
}
#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Ident(String),
    String(String),
    Int(i64),
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Dot,
    Colon,
    Eof,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}
impl Parser {
    fn new(source: &str) -> Result<Self, PolicyCompileError> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = source.chars().collect();
        let (mut i, mut line, mut col) = (0, 1, 1);
        let push =
            |tokens: &mut Vec<Token>, kind, line, column| tokens.push(Token { kind, line, column });
        while i < chars.len() {
            let c = chars[i];
            if c == ' ' || c == '\t' || c == '\r' || c == '\n' {
                if c == '\n' {
                    line += 1;
                    col = 1;
                } else {
                    col += 1;
                }
                i += 1;
                continue;
            }
            if c == '#' {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                    col += 1;
                }
                continue;
            }
            let (sl, sc) = (line, col);
            if c == '"' {
                i += 1;
                col += 1;
                let mut s = String::new();
                let mut closed = false;
                while i < chars.len() {
                    let ch = chars[i];
                    if ch == '"' {
                        i += 1;
                        col += 1;
                        closed = true;
                        break;
                    }
                    if ch == '\\' {
                        i += 1;
                        col += 1;
                        if i >= chars.len() {
                            break;
                        }
                        let e = chars[i];
                        s.push(match e {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '"' => '"',
                            '\\' => '\\',
                            _ => return Err(parse_error(line, col, "unsupported string escape")),
                        });
                        i += 1;
                        col += 1;
                    } else {
                        if ch == '\n' {
                            return Err(parse_error(
                                line,
                                col,
                                "string literal cannot cross lines",
                            ));
                        }
                        s.push(ch);
                        i += 1;
                        col += 1;
                    }
                }
                if !closed {
                    return Err(parse_error(sl, sc, "unterminated string"));
                }
                push(&mut tokens, TokenKind::String(s), sl, sc);
                continue;
            }
            if c == '-' && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit()) {
                let start = i;
                i += 1;
                col += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                    col += 1;
                }
                let n: String = chars[start..i].iter().collect();
                let value =
                    n.parse::<i64>().map_err(|_| parse_error(sl, sc, "integer is out of range"))?;
                push(&mut tokens, TokenKind::Int(value), sl, sc);
                continue;
            }
            if c.is_ascii_digit() {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                    col += 1;
                }
                let n: String = chars[start..i].iter().collect();
                let value =
                    n.parse::<i64>().map_err(|_| parse_error(sl, sc, "integer is out of range"))?;
                push(&mut tokens, TokenKind::Int(value), sl, sc);
                continue;
            }
            if c.is_alphabetic() || c == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                    col += 1;
                }
                push(&mut tokens, TokenKind::Ident(chars[start..i].iter().collect()), sl, sc);
                continue;
            }
            let next = chars.get(i + 1).copied();
            let (kind, width) = match (c, next) {
                ('=', Some('=')) => (TokenKind::Eq, 2),
                ('=', _) => (TokenKind::Eq, 1),
                ('!', Some('=')) => (TokenKind::Ne, 2),
                ('<', Some('=')) => (TokenKind::Le, 2),
                ('>', Some('=')) => (TokenKind::Ge, 2),
                ('<', _) => (TokenKind::Lt, 1),
                ('>', _) => (TokenKind::Gt, 1),
                ('(', _) => (TokenKind::LParen, 1),
                (')', _) => (TokenKind::RParen, 1),
                ('[', _) => (TokenKind::LBracket, 1),
                (']', _) => (TokenKind::RBracket, 1),
                (',', _) => (TokenKind::Comma, 1),
                ('.', _) => (TokenKind::Dot, 1),
                (':', _) => (TokenKind::Colon, 1),
                _ => return Err(parse_error(sl, sc, format!("unexpected character {c:?}"))),
            };
            i += width;
            col += width;
            push(&mut tokens, kind, sl, sc);
        }
        if tokens.len() > 20_000 {
            return Err(parse_error(line, col, "policy token limit exceeded"));
        }
        tokens.push(Token { kind: TokenKind::Eof, line, column: col });
        Ok(Self { tokens, pos: 0, depth: 0 })
    }
    fn parse_policy(&mut self) -> Result<CompiledPolicy, PolicyCompileError> {
        self.expect_word("language")?;
        let version = self.take_int()?;
        if version != LANGUAGE_VERSION as i64 {
            return Err(PolicyCompileError::UnsupportedVersion(version.max(0) as u32));
        }
        self.expect_word("policy")?;
        let name = self.take_name()?;
        self.expect(TokenKind::Colon)?;
        let intent = self.take_string()?;
        self.expect_word("triggers")?;
        self.expect_word("when")?;
        let triggers = self.parse_expr(0)?;
        let mut facts = Vec::new();
        if self.peek_word("facts") {
            self.expect_word("facts")?;
            while !self.peek_word("requires") && !matches!(self.peek().kind, TokenKind::Eof) {
                let name = self.take_name()?;
                self.expect(TokenKind::Eq)?;
                self.expect_word("data")?;
                self.expect(TokenKind::Dot)?;
                let collection = self.take_name()?;
                self.expect(TokenKind::LBracket)?;
                let key = self.parse_expr(0)?;
                self.expect(TokenKind::RBracket)?;
                let fresh = if self.peek_word("fresh") {
                    self.expect_word("fresh")?;
                    self.expect_word("within")?;
                    Some(self.take_duration()?)
                } else {
                    None
                };
                facts.push(FactBinding {
                    name,
                    collection,
                    key,
                    fresh_within_seconds: fresh,
                });
            }
        }
        self.expect_word("requires")?;
        let requires = self.parse_expr(0)?;
        if !matches!(self.peek().kind, TokenKind::Eof) {
            return Err(self.err("unexpected trailing tokens"));
        }
        let mut names = HashSet::new();
        for f in &facts {
            if !names.insert(f.name.clone()) {
                return Err(self.err(format!("duplicate fact binding {:?}", f.name)));
            }
        }
        Ok(CompiledPolicy {
            language_version: version as u32,
            name,
            intent,
            triggers,
            facts,
            requires,
            input_schema: Schema::Bool,
            collection_schemas: BTreeMap::new(),
            policy_data: None,
        })
    }
    fn parse_expr(&mut self, min_prec: u8) -> Result<Expr, PolicyCompileError> {
        self.depth += 1;
        if self.depth > 64 {
            return Err(self.err("expression nesting limit exceeded"));
        }
        let mut left = if self.peek_word("not") {
            self.next();
            Expr::Not(Box::new(self.parse_expr(3)?))
        } else {
            self.parse_primary()?
        };
        let mut comparison_seen = false;
        loop {
            let (prec, kind) = match self.peek().kind {
                TokenKind::Ident(ref s) if s == "or" => (1, Some(0)),
                TokenKind::Ident(ref s) if s == "and" => (2, Some(1)),
                TokenKind::Ident(ref s) if s == "in" => (3, Some(2)),
                TokenKind::Eq => (3, Some(3)),
                TokenKind::Ne => (3, Some(4)),
                TokenKind::Lt => (3, Some(5)),
                TokenKind::Le => (3, Some(6)),
                TokenKind::Gt => (3, Some(7)),
                TokenKind::Ge => (3, Some(8)),
                _ => (0, None),
            };
            if prec < min_prec || kind.is_none() {
                break;
            }
            let k = kind.unwrap();
            if (2..=8).contains(&k) && comparison_seen {
                return Err(self.err("chained comparisons require explicit parentheses"));
            }
            if (2..=8).contains(&k) {
                comparison_seen = true;
            }
            self.next();
            let right = self.parse_expr(prec + 1)?;
            left = match k {
                0 => join_bool(left, right, false),
                1 => join_bool(left, right, true),
                2 => Expr::In {
                    item: Box::new(left),
                    list: Box::new(right),
                },
                x => Expr::Compare {
                    left: Box::new(left),
                    op: match x {
                        3 => CompareOp::Eq,
                        4 => CompareOp::Ne,
                        5 => CompareOp::Lt,
                        6 => CompareOp::Le,
                        7 => CompareOp::Gt,
                        _ => CompareOp::Ge,
                    },
                    right: Box::new(right),
                },
            };
        }
        self.depth -= 1;
        Ok(left)
    }
    fn parse_primary(&mut self) -> Result<Expr, PolicyCompileError> {
        match self.next().clone() {
            Token { kind: TokenKind::String(s), .. } => Ok(Expr::Literal(Literal::String(s))),
            Token { kind: TokenKind::Int(n), .. } => Ok(Expr::Literal(Literal::Int(n))),
            Token { kind: TokenKind::Ident(s), .. } if s == "true" => {
                Ok(Expr::Literal(Literal::Bool(true)))
            },
            Token { kind: TokenKind::Ident(s), .. } if s == "false" => {
                Ok(Expr::Literal(Literal::Bool(false)))
            },
            Token { kind: TokenKind::Ident(s), .. }
                if matches!(s.as_str(), "any" | "all" | "count" | "count_distinct")
                    && self.peek_is(&TokenKind::LParen) =>
            {
                self.parse_aggregate(s)
            },
            Token { kind: TokenKind::Ident(root), .. } => {
                let mut segments = Vec::new();
                while self.peek_is(&TokenKind::Dot) {
                    self.next();
                    segments.push(self.take_name()?);
                }
                Ok(Expr::Path { root, segments })
            },
            Token { kind: TokenKind::LParen, .. } => {
                let e = self.parse_expr(0)?;
                self.expect(TokenKind::RParen)?;
                Ok(e)
            },
            Token { kind: TokenKind::LBracket, .. } => {
                let mut items = Vec::new();
                if !self.peek_is(&TokenKind::RBracket) {
                    loop {
                        items.push(self.parse_expr(0)?);
                        if !self.peek_is(&TokenKind::Comma) {
                            break;
                        }
                        self.next();
                    }
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::List(items))
            },
            t => {
                Err(parse_error(t.line, t.column, format!("expected expression, got {:?}", t.kind)))
            },
        }
    }
    fn parse_aggregate(&mut self, name: String) -> Result<Expr, PolicyCompileError> {
        self.expect(TokenKind::LParen)?;
        let value = self.parse_expr(0)?;
        self.expect_word("for")?;
        let variable = self.take_name()?;
        if matches!(variable.as_str(), "input" | "data" | "policy") {
            return Err(self.err("aggregate variable name is reserved"));
        }
        self.expect_word("in")?;
        let list = self.parse_expr(0)?;
        let where_clause = if self.peek_word("where") {
            self.expect_word("where")?;
            Some(Box::new(self.parse_expr(0)?))
        } else {
            None
        };
        self.expect(TokenKind::RParen)?;
        let kind = match name.as_str() {
            "any" => AggregateKind::Any,
            "all" => AggregateKind::All,
            "count" => AggregateKind::Count,
            _ => AggregateKind::CountDistinct,
        };
        Ok(Expr::Aggregate {
            kind,
            value: Box::new(value),
            variable,
            list: Box::new(list),
            where_clause,
        })
    }
    fn take_duration(&mut self) -> Result<u64, PolicyCompileError> {
        let n = self.take_int()?;
        if n <= 0 {
            return Err(self.err("freshness duration must be positive"));
        }
        let unit = self.take_name()?;
        let mult = match unit.as_str() {
            "s" | "sec" | "seconds" => 1u64,
            "m" | "min" | "minutes" => 60,
            "h" | "hours" => 3600,
            "d" | "days" => 86400,
            _ => return Err(self.err("unsupported freshness duration unit")),
        };
        u64::try_from(n)
            .ok()
            .and_then(|n| n.checked_mul(mult))
            .ok_or_else(|| self.err("freshness duration is too large"))
    }
    fn take_name(&mut self) -> Result<String, PolicyCompileError> {
        match self.next().clone() {
            Token { kind: TokenKind::Ident(s), .. } => Ok(s),
            t => Err(parse_error(t.line, t.column, "expected identifier")),
        }
    }
    fn take_int(&mut self) -> Result<i64, PolicyCompileError> {
        match self.next().clone() {
            Token { kind: TokenKind::Int(n), .. } => Ok(n),
            t => Err(parse_error(t.line, t.column, "expected integer")),
        }
    }
    fn take_string(&mut self) -> Result<String, PolicyCompileError> {
        match self.next().clone() {
            Token { kind: TokenKind::String(s), .. } => Ok(s),
            t => Err(parse_error(t.line, t.column, "expected string")),
        }
    }
    fn expect_word(&mut self, w: &str) -> Result<(), PolicyCompileError> {
        if self.peek_word(w) {
            self.next();
            Ok(())
        } else {
            Err(self.err(format!("expected {w:?}")))
        }
    }
    fn expect(&mut self, k: TokenKind) -> Result<(), PolicyCompileError> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&k) {
            self.next();
            Ok(())
        } else {
            Err(self.err(format!("expected {k:?}")))
        }
    }
    fn peek_word(&self, w: &str) -> bool {
        matches!(&self.peek().kind,TokenKind::Ident(s) if s==w)
    }
    fn peek_is(&self, k: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(k)
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }
    fn next(&mut self) -> &Token {
        let p = self.pos.min(self.tokens.len() - 1);
        self.pos = (p + 1).min(self.tokens.len() - 1);
        &self.tokens[p]
    }
    fn err(&self, msg: impl Into<String>) -> PolicyCompileError {
        parse_error(self.peek().line, self.peek().column, msg)
    }
}
fn join_bool(a: Expr, b: Expr, and: bool) -> Expr {
    match (a, b, and) {
        (Expr::And(mut xs), Expr::And(ys), true) => {
            xs.extend(ys);
            Expr::And(xs)
        },
        (Expr::And(mut xs), b, true) => {
            xs.push(b);
            Expr::And(xs)
        },
        (a, Expr::And(mut ys), true) => {
            let mut xs = vec![a];
            xs.append(&mut ys);
            Expr::And(xs)
        },
        (Expr::Or(mut xs), Expr::Or(ys), false) => {
            xs.extend(ys);
            Expr::Or(xs)
        },
        (Expr::Or(mut xs), b, false) => {
            xs.push(b);
            Expr::Or(xs)
        },
        (a, Expr::Or(mut ys), false) => {
            let mut xs = vec![a];
            xs.append(&mut ys);
            Expr::Or(xs)
        },
        (a, b, true) => Expr::And(vec![a, b]),
        (a, b, false) => Expr::Or(vec![a, b]),
    }
}
fn parse_error(line: usize, column: usize, message: impl Into<String>) -> PolicyCompileError {
    PolicyCompileError::Parse { line, column, message: message.into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime_facts::{
        CollectionConfig, FactMutation, FactStore, FieldSchema, IngestionMode, Scope,
    };

    fn field(schema: Schema) -> FieldSchema {
        FieldSchema { schema, optional: false }
    }
    fn record(fields: &[(&str, Schema)]) -> Schema {
        Schema::Record {
            fields: fields.iter().map(|(k, v)| ((*k).to_owned(), field(v.clone()))).collect(),
        }
    }
    fn env() -> CompileEnvironment {
        let input = record(&[
            (
                "resource",
                record(&[
                    ("id", Schema::String),
                    ("environment", Schema::String),
                    ("active", Schema::Bool),
                ]),
            ),
            ("principal", record(&[("id", Schema::String), ("team", Schema::String)])),
            ("action", record(&[("operation", Schema::String)])),
        ]);
        let approver = record(&[
            ("principal_id", Schema::String),
            ("role", Schema::String),
            ("team", Schema::String),
        ]);
        let approvals = record(&[(
            "approvers",
            Schema::Array {
                items: Box::new(approver),
                max_items: 100,
            },
        )]);
        CompileEnvironment::new(input).collection("approvals", approvals)
    }
    fn policy_source() -> &'static str {
        r#"language 2
policy ProductionDeployment: "Requires independent senior approvals"
  triggers when
    input.resource.environment == "prod"
    and input.action.operation == "Deploy"
  facts
    approvals = data.approvals[input.resource.id] fresh within 5m
  requires
    count_distinct(a.principal_id for a in approvals.value.approvers where a.role == "senior") >= 2
"#
    }
    fn install(value: Value, observed_at: i64, now: i64) -> FactSnapshot {
        let store = FactStore::memory();
        let scope = Scope::new("tenant", "prod");
        store
            .register_collection(
                scope.clone(),
                "approvals",
                CollectionConfig {
                    schema: env().collections["approvals"].clone(),
                    max_ttl_seconds: 600,
                    max_observation_age_seconds: 600,
                    source_id: "approval-service".into(),
                    mode: IngestionMode::Push,
                },
            )
            .unwrap();
        store
            .commit(
                &scope,
                "approvals",
                "approval-service",
                vec![FactMutation {
                    key: "deploy-7".into(),
                    value: Some(value),
                    source_revision: 1,
                    expected_revision: None,
                    observed_at,
                    ttl_seconds: Some(300),
                }],
                now,
            )
            .unwrap();
        store.snapshot(scope)
    }
    fn configured_empty() -> FactSnapshot {
        let store = FactStore::memory();
        let scope = Scope::new("tenant", "prod");
        store
            .register_collection(
                scope.clone(),
                "approvals",
                CollectionConfig {
                    schema: env().collections["approvals"].clone(),
                    max_ttl_seconds: 600,
                    max_observation_age_seconds: 600,
                    source_id: "approval-service".into(),
                    mode: IngestionMode::Push,
                },
            )
            .unwrap();
        store.snapshot(scope)
    }
    fn schema_mismatch_snapshot() -> FactSnapshot {
        let store = FactStore::memory();
        let scope = Scope::new("tenant", "prod");
        store
            .register_collection(
                scope.clone(),
                "approvals",
                CollectionConfig {
                    schema: record(&[(
                        "approvers",
                        Schema::Array {
                            items: Box::new(Schema::String),
                            max_items: 100,
                        },
                    )]),
                    max_ttl_seconds: 600,
                    max_observation_age_seconds: 600,
                    source_id: "approval-service".into(),
                    mode: IngestionMode::Push,
                },
            )
            .unwrap();
        store.snapshot(scope)
    }
    fn input(environment: &str) -> Value {
        serde_json::json!({"resource":{"id":"deploy-7","environment":environment,"active":true},"principal":{"id":"caller","team":"ops"},"action":{"operation":"Deploy"}})
    }
    fn approvals(names: &[&str]) -> Value {
        serde_json::json!({"approvers":names.iter().map(|name|serde_json::json!({"principal_id":name,"role":"senior","team":"security"})).collect::<Vec<_>>()})
    }

    #[test]
    fn parses_compiles_serializable_typed_ast_and_aggregates() {
        let policy = CompiledPolicy::compile(policy_source(), &env()).unwrap();
        assert_eq!(policy.language_version, 2);
        assert_eq!(policy.facts[0].fresh_within_seconds, Some(300));
        assert!(serde_json::to_vec(&policy).is_ok());
        let snap = install(approvals(&["alice", "bob"]), 980, 1000);
        assert_eq!(
            policy.evaluate(&input("prod"), &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Allow
        );
        let snap = install(approvals(&["alice"]), 980, 1000);
        assert_eq!(
            policy.evaluate(&input("prod"), &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Deny
        );
        let snap = install(approvals(&[]), 980, 1000);
        assert_eq!(
            policy.evaluate(&input("prod"), &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Deny
        );
    }

    #[test]
    fn false_trigger_skips_missing_evidence_but_true_trigger_is_indeterminate() {
        let policy = CompiledPolicy::compile(policy_source(), &env()).unwrap();
        let snap = configured_empty();
        assert_eq!(
            policy.evaluate(&input("dev"), &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::NotApplicable
        );
        assert!(matches!(
            policy.evaluate(&input("prod"), &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::MissingFact { .. })
        ));
        let invalid_input = serde_json::json!({"resource":{"environment":"prod"},"principal":{"id":"caller","team":"ops"},"action":{"operation":"Deploy"}});
        assert!(matches!(
            policy.evaluate(&invalid_input, &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::TypeMismatch(_))
        ));
    }

    #[test]
    fn stale_and_expired_evidence_never_allows_and_deadline_is_returned() {
        let policy = CompiledPolicy::compile(policy_source(), &env()).unwrap();
        let snap = install(approvals(&["alice", "bob"]), 980, 1000);
        let result = policy.evaluate(&input("prod"), &snap, 1000, EvalBudget::default());
        assert_eq!(result.evidence_deadline, Some(1280));
        assert!(matches!(result.outcome, PolicyOutcome::Allow));
        assert!(matches!(
            policy.evaluate(&input("prod"), &snap, 1280, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::StaleFact { .. })
        ));
        let stale = install(approvals(&["alice", "bob"]), 600, 1000);
        assert!(matches!(
            policy.evaluate(&input("prod"), &stale, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::StaleFact { .. })
        ));
        assert!(matches!(
            policy.evaluate(&input("prod"), &snap, 1300, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::ExpiredFact { .. })
        ));
    }

    #[test]
    fn rejects_unknown_paths_bad_types_trailing_input_and_unsupported_versions() {
        let policy = policy_source().replace("input.action.operation", "input.action.missing");
        assert!(matches!(
            CompiledPolicy::compile(&policy, &env()),
            Err(PolicyCompileError::Type(_))
        ));
        assert!(matches!(
            CompiledPolicy::compile(&format!("{} garbage", policy_source()), &env()),
            Err(PolicyCompileError::Parse { .. })
        ));
        assert!(matches!(
            CompiledPolicy::compile(&policy_source().replace("language 2", "language 7"), &env()),
            Err(PolicyCompileError::UnsupportedVersion(7))
        ));
        assert!(matches!(
            CompiledPolicy::compile(&policy_source().replace("== \"prod\"", "== 1"), &env()),
            Err(PolicyCompileError::Type(_))
        ));
        let duplicate = policy_source()
            .replace("  requires", "    approvals = data.approvals[input.resource.id]\n  requires");
        assert!(matches!(
            CompiledPolicy::compile(&duplicate, &env()),
            Err(PolicyCompileError::Parse { .. })
        ));
    }

    #[test]
    fn aggregate_budget_exhaustion_is_indeterminate() {
        let policy = CompiledPolicy::compile(policy_source(), &env()).unwrap();
        let snap = install(approvals(&["alice", "bob"]), 980, 1000);
        let budget = EvalBudget {
            max_collection_items: 1,
            ..EvalBudget::default()
        };
        assert_eq!(
            policy.evaluate(&input("prod"), &snap, 1000, budget).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::BudgetExceeded)
        );
    }

    #[test]
    fn boolean_precedence_signed_ints_and_membership_budgets_are_strict() {
        let schema = record(&[(
            "resource",
            record(&[
                ("number", Schema::Int),
                (
                    "items",
                    Schema::Array {
                        items: Box::new(Schema::String),
                        max_items: 10,
                    },
                ),
            ]),
        )]);
        let env = CompileEnvironment::new(schema);
        let compile_logic = |expression: &str| {
            CompiledPolicy::compile(&format!("language 2\npolicy Logic: \"logic\"\ntriggers when true\nrequires {expression}\n"), &env).unwrap()
        };
        let snap = FactStore::memory().snapshot(Scope::new("tenant", "prod"));
        let input = serde_json::json!({"resource":{"number":-2,"items":[]}});
        assert_eq!(
            compile_logic("false or true and false")
                .evaluate(&input, &snap, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Deny
        );
        assert_eq!(
            compile_logic("(false or true) and true")
                .evaluate(&input, &snap, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Allow
        );
        assert_eq!(
            compile_logic("input.resource.number == -2")
                .evaluate(&input, &snap, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Allow
        );
        let membership = compile_logic("\"x\" in input.resource.items");
        let budget = EvalBudget {
            max_collection_items: 0,
            ..EvalBudget::default()
        };
        let input = serde_json::json!({"resource":{"number":0,"items":["x"]}});
        assert_eq!(
            membership.evaluate(&input, &snap, 0, budget).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::BudgetExceeded)
        );
    }

    #[test]
    fn all_declared_dependencies_resolve_before_short_circuited_requirements() {
        let source = r#"language 2
policy Eager: "Resolve all dependencies"
triggers when input.resource.environment == "prod"
facts
  a = data.approvals[input.resource.id]
  b = data.approvals["missing"]
requires true or a.value.approvers in []
"#;
        let policy = CompiledPolicy::compile(source, &env()).unwrap();
        let snap = install(approvals(&["alice"]), 980, 1000);
        assert_eq!(
            policy.evaluate(&input("prod"), &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::MissingFact { binding: "b".into() })
        );
        for expression in ["false and a.value.approvers in []", "true or a.value.approvers in []"] {
            let source = source.replace("true or a.value.approvers in []", expression);
            let policy = CompiledPolicy::compile(&source, &env()).unwrap();
            assert_eq!(
                policy.evaluate(&input("prod"), &snap, 1000, EvalBudget::default()).outcome,
                PolicyOutcome::Indeterminate(PolicyFailure::MissingFact { binding: "b".into() })
            );
        }
    }

    #[test]
    fn rejects_reserved_binding_names_and_invalid_serialized_versions() {
        let source = policy_source().replace("approvals =", "input =");
        assert!(matches!(
            CompiledPolicy::compile(&source, &env()),
            Err(PolicyCompileError::Type(_))
        ));
        let source = policy_source().replace("approvals =", "data =");
        assert!(matches!(
            CompiledPolicy::compile(&source, &env()),
            Err(PolicyCompileError::Type(_))
        ));
        let source = policy_source()
            .replace("for a in", "for approvals in")
            .replace("a.principal_id", "approvals.principal_id")
            .replace("a.role", "approvals.role");
        assert!(matches!(
            CompiledPolicy::compile(&source, &env()),
            Err(PolicyCompileError::Type(_))
        ));
        let mut policy = CompiledPolicy::compile(policy_source(), &env()).unwrap();
        policy.language_version = 77;
        let snap = configured_empty();
        assert!(matches!(
            policy.evaluate(&input("dev"), &snap, 1000, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::InvalidPolicy(_))
        ));
    }

    #[test]
    fn type_checker_rejects_record_and_boolean_ordering_and_checks_every_branch() {
        let source = policy_source().replace("count_distinct(a.principal_id for a in approvals.value.approvers where a.role == \"senior\") >= 2", "input.resource < input.resource");
        assert!(matches!(
            CompiledPolicy::compile(&source, &env()),
            Err(PolicyCompileError::Type(_))
        ));
        let source = policy_source().replace("count_distinct(a.principal_id for a in approvals.value.approvers where a.role == \"senior\") >= 2", "input.resource.active < true");
        assert!(matches!(
            CompiledPolicy::compile(&source, &env()),
            Err(PolicyCompileError::Type(_))
        ));
        let source = policy_source().replace("count_distinct(a.principal_id for a in approvals.value.approvers where a.role == \"senior\") >= 2", "false and input.resource.missing == 1");
        assert!(matches!(
            CompiledPolicy::compile(&source, &env()),
            Err(PolicyCompileError::Type(_))
        ));
        let source = policy_source().replace("count_distinct(a.principal_id for a in approvals.value.approvers where a.role == \"senior\") >= 2", "input.resource.environment == 1");
        assert!(matches!(
            CompiledPolicy::compile(&source, &env()),
            Err(PolicyCompileError::Type(_))
        ));
    }

    #[test]
    fn parser_bounds_nesting_and_source_size_and_keeps_wide_lists_bounded() {
        let nested = format!(
            "language 2\npolicy Deep: \"deep\"\ntriggers when true\nrequires {}true{}\n",
            "(".repeat(65),
            ")".repeat(65)
        );
        assert!(matches!(
            CompiledPolicy::compile(&nested, &env()),
            Err(PolicyCompileError::Parse { .. })
        ));
        let nested_not = format!(
            "language 2\npolicy Deep: \"deep\"\ntriggers when true\nrequires {}true\n",
            "not ".repeat(65)
        );
        assert!(matches!(
            CompiledPolicy::compile(&nested_not, &env()),
            Err(PolicyCompileError::Parse { .. })
        ));
        let chain = format!(
            "language 2\npolicy Chain: \"chain\"\ntriggers when true\nrequires {}\n",
            vec!["true == true"; 2_000].join(" == ")
        );
        assert!(matches!(
            CompiledPolicy::compile(&chain, &env()),
            Err(PolicyCompileError::Parse { .. })
        ));
        let oversized = format!(
            "language 2\npolicy Large: \"large\"\ntriggers when true\nrequires true {}\n",
            " ".repeat(65_536)
        );
        assert!(matches!(
            CompiledPolicy::compile(&oversized, &env()),
            Err(PolicyCompileError::Parse { .. })
        ));
        let list = vec!["\"x\""; 1_001].join(",");
        let source = format!(
            "language 2\npolicy Wide: \"wide\"\ntriggers when true\nrequires \"x\" in [{list}]\n"
        );
        let policy = CompiledPolicy::compile(&source, &env()).unwrap();
        let snap = FactStore::memory().snapshot(Scope::new("tenant", "prod"));
        assert_eq!(
            policy.evaluate(&input("prod"), &snap, 0, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::BudgetExceeded)
        );
    }

    #[test]
    fn schema_mismatch_and_freshness_boundary_are_indeterminate() {
        let policy = CompiledPolicy::compile(policy_source(), &env()).unwrap();
        assert!(matches!(
            policy
                .evaluate(&input("prod"), &schema_mismatch_snapshot(), 1000, EvalBudget::default())
                .outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::InvalidFact { .. })
        ));
        let snapshot = install(approvals(&["alice", "bob"]), 980, 1000);
        assert_eq!(
            policy.evaluate(&input("prod"), &snapshot, 1280, EvalBudget::default()).outcome,
            PolicyOutcome::Indeterminate(PolicyFailure::StaleFact { binding: "approvals".into() })
        );
    }

    #[test]
    fn aggregates_have_typed_scalar_distinctness_and_defined_empty_semantics() {
        let env = CompileEnvironment::new(Schema::Array {
            items: Box::new(Schema::Int),
            max_items: 10,
        });
        let compile = |expression: &str| {
            CompiledPolicy::compile(&format!("language 2\npolicy Aggregate: \"aggregate\"\ntriggers when true\nrequires {expression}\n"), &env).unwrap()
        };
        let snapshot = FactStore::memory().snapshot(Scope::new("tenant", "prod"));
        assert_eq!(
            compile("count_distinct(x for x in input) == 2")
                .evaluate(&serde_json::json!([1, 1, 3]), &snapshot, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Allow
        );
        assert_eq!(
            compile("count(x for x in input where x > 0) == 3")
                .evaluate(&serde_json::json!([1, 1, 3]), &snapshot, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Allow
        );
        assert_eq!(
            compile("all(x > 0 for x in input)")
                .evaluate(&serde_json::json!([]), &snapshot, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Allow
        );
        assert_eq!(
            compile("any(x > 0 for x in input)")
                .evaluate(&serde_json::json!([]), &snapshot, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Deny
        );
        assert_eq!(
            compile("count(x for x in input) == 0")
                .evaluate(&serde_json::json!([]), &snapshot, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Allow
        );
        assert_eq!(
            compile("count_distinct(x for x in input) == 0")
                .evaluate(&serde_json::json!([]), &snapshot, 0, EvalBudget::default())
                .outcome,
            PolicyOutcome::Allow
        );
        assert!(matches!(CompiledPolicy::compile("language 2\npolicy Aggregate: \"aggregate\"\ntriggers when true\nrequires count_distinct(x for x in input) == \"1\"\n", &env), Err(PolicyCompileError::Type(_))));
    }
}
