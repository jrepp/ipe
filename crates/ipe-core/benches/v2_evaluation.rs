//! Benchmarks for the v2 runtime policy evaluation path (language 2).
//!
//! Everything measured here runs the real evaluation stack end to end:
//! `CompiledPolicy::compile` (parse + typecheck against a `CompileEnvironment`)
//! and `CompiledPolicy::evaluate` (input schema validation, trigger
//! evaluation, scoped fact resolution against a `FactStore` snapshot under an
//! `EvalBudget`, and requirement evaluation). This is the product's hot path;
//! the mission's sub-50µs evaluation target must be judged on these numbers.
//!
//! Durable per-commit numbers (longer sampling, saved baseline):
//! `cargo bench -p ipe-core --features approvals --bench v2_evaluation -- \
//!  --warm-up-time 3 --measurement-time 10 --save-baseline v2-evaluation`

#![cfg(feature = "approvals")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use criterion::{black_box, Criterion, Throughput};
use serde::Deserialize;
use serde_json::{json, Value};

use ipe_core::runtime_facts::{
    CollectionConfig, FactMutation, FactSnapshot, FactStore, FieldSchema, IngestionMode, Schema,
    Scope,
};
use ipe_core::runtime_policy::{
    CompileEnvironment, CompiledPolicy, EvalBudget, PolicyData, PolicyFailure, PolicyOutcome,
};

/// Caller clock (Unix seconds) for the hot-path benches. Facts are committed
/// at this instant with TTLs that keep the driving fact resolvable, so every
/// hot-path evaluation resolves its evidence and returns `Allow`.
const NOW: i64 = 1_700_000_000;

/// Clock for `v2_evaluate_ttl_expired`: past the `expires_at` of the driving
/// fact, so `FactSnapshot::get` rejects it and evaluate returns
/// `Indeterminate(ExpiredFact)`.
const EXPIRED_NOW: i64 = NOW + 11;

const COLLECTION: &str = "pull_requests";
const SOURCE_ID: &str = "forge-runtime";
const HOT_COMMIT: &str = "commit-abc";
const EXPIRED_COMMIT: &str = "commit-expired";

/// Bounded budget for `v2_evaluate_budget_exhaustion`. The probe policy needs
/// ~16 instructions per evaluation, so an 8-step budget exhausts inside the
/// requirement expression (after the trigger and fact resolution have run),
/// which is exactly the indeterminate `BudgetExceeded` path.
const BENCH_BUDGET: EvalBudget = EvalBudget {
    max_steps: 8,
    max_collection_items: 1_000,
};

fn primary_scope() -> Scope {
    Scope::new("tenant-alpha", "production")
}

fn secondary_scope() -> Scope {
    Scope::new("tenant-beta", "production")
}

fn input_schema() -> Schema {
    Schema::Record {
        fields: BTreeMap::from([
            ("operation".into(), FieldSchema { schema: Schema::String, optional: false }),
            ("commit".into(), FieldSchema { schema: Schema::String, optional: false }),
        ]),
    }
}

fn pull_request_schema() -> Schema {
    Schema::Record {
        fields: BTreeMap::from([
            (
                "approvers".into(),
                FieldSchema {
                    schema: Schema::Array {
                        items: Box::new(Schema::String),
                        max_items: 20,
                    },
                    optional: false,
                },
            ),
            (
                "passed_checks".into(),
                FieldSchema {
                    schema: Schema::Array {
                        items: Box::new(Schema::String),
                        max_items: 20,
                    },
                    optional: false,
                },
            ),
        ]),
    }
}

fn policy_data() -> PolicyData {
    PolicyData {
        schema: Schema::Record {
            fields: BTreeMap::from([
                (
                    "review".into(),
                    FieldSchema {
                        schema: Schema::Record {
                            fields: BTreeMap::from([(
                                "minimum".into(),
                                FieldSchema { schema: Schema::Int, optional: false },
                            )]),
                        },
                        optional: false,
                    },
                ),
                (
                    "required_checks".into(),
                    FieldSchema {
                        schema: Schema::Array {
                            items: Box::new(Schema::String),
                            max_items: 20,
                        },
                        optional: false,
                    },
                ),
            ]),
        },
        value: json!({"review": {"minimum": 2}, "required_checks": ["test", "lint"]}),
    }
}

fn compile_environment() -> CompileEnvironment {
    CompileEnvironment::new(input_schema())
        .collection(COLLECTION, pull_request_schema())
        .policy_data(policy_data())
}

/// One realistic language-2 policy over the PR-landing fact model. Every
/// policy triggers on the merge operation, binds the driving commit's
/// `pull_requests` fact with a freshness window, and runs a real aggregate or
/// membership requirement against policy data.
struct PolicySpec {
    name: &'static str,
    intent: &'static str,
    requires: &'static str,
}

const POLICIES: [PolicySpec; 8] = [
    PolicySpec {
        name: "LandingApprovers",
        intent: "Distinct approving reviewers meet the configured minimum",
        requires: "count_distinct(a for a in pr.value.approvers) >= policy.review.minimum",
    },
    PolicySpec {
        name: "LandingChecks",
        intent: "Every required check passed on the commit",
        requires: "all(check in pr.value.passed_checks for check in policy.required_checks)",
    },
    PolicySpec {
        name: "SeniorApprover",
        intent: "At least one approver is on the senior roster",
        requires: "any(a in [\"alice\", \"bob\", \"carol\"] for a in pr.value.approvers)",
    },
    PolicySpec {
        name: "CheckQuorum",
        intent: "At least two checks passed",
        requires: "count(c for c in pr.value.passed_checks) >= 2",
    },
    PolicySpec {
        name: "ApprovedAndTested",
        intent: "An approval exists and the test check passed",
        requires:
            "(count(a for a in pr.value.approvers) >= 1) and (\"test\" in pr.value.passed_checks)",
    },
    PolicySpec {
        name: "CommitGuard",
        intent: "The decision is pinned to the driving commit",
        requires:
            "(input.commit == \"commit-abc\") and (count(c for c in pr.value.passed_checks) >= 1)",
    },
    PolicySpec {
        name: "DualQuorum",
        intent: "Approvals and checks both clear their quorums",
        requires: "(count_distinct(a for a in pr.value.approvers) >= policy.review.minimum) \
and (count(c for c in pr.value.passed_checks) >= 1)",
    },
    PolicySpec {
        name: "MergeWithLint",
        intent: "Lint passed and at least one approval exists",
        requires: "(\"lint\" in pr.value.passed_checks) and (count(a for a in pr.value.approvers) \
>= 1)",
    },
];

/// Wide-and-deep requirement used only to exhaust `BENCH_BUDGET`.
const BUDGET_POLICY: PolicySpec = PolicySpec {
    name: "BudgetExhaustion",
    intent: "Nested requirement that exhausts a bounded instruction budget",
    requires: "not not not not not not not not not not not not true",
};

fn policy_source(spec: &PolicySpec) -> String {
    let mut source = String::from("language 2\npolicy ");
    source.push_str(spec.name);
    source.push_str(": \"");
    source.push_str(spec.intent);
    source.push_str("\"\ntriggers when input.operation == \"merge\"\nfacts\n  pr = data.");
    source.push_str(COLLECTION);
    source.push_str("[input.commit] fresh within 5m\nrequires ");
    source.push_str(spec.requires);
    source.push('\n');
    source
}

fn fact_mutation(
    key: &str,
    approvers: &[&str],
    checks: &[&str],
    source_revision: u64,
    ttl_seconds: u64,
) -> FactMutation {
    FactMutation {
        key: key.to_owned(),
        value: Some(json!({
            "approvers": approvers,
            "passed_checks": checks,
        })),
        source_revision,
        expected_revision: None,
        observed_at: NOW,
        ttl_seconds: Some(ttl_seconds),
    }
}

/// Durable fixtures for the evaluate benches. Everything expensive (RocksDB
/// open, registration, commits, compilation, snapshot capture) happens once,
/// outside the timed regions; the timed regions only run `evaluate` (or
/// `compile`, which recompiles from source every iteration).
struct BenchFixture {
    environment: CompileEnvironment,
    policies: Vec<CompiledPolicy>,
    hot_policy: CompiledPolicy,
    budget_policy: CompiledPolicy,
    input: Value,
    expired_input: Value,
    snapshot: FactSnapshot,
    _store: FactStore,
    _directory: tempfile::TempDir,
}

fn fixture() -> BenchFixture {
    let directory = tempfile::tempdir().expect("tempdir for fact store");
    let store = FactStore::open(directory.path()).expect("open fact store");

    let config = CollectionConfig {
        schema: pull_request_schema(),
        max_ttl_seconds: 600,
        max_observation_age_seconds: 600,
        source_id: SOURCE_ID.to_owned(),
        mode: IngestionMode::Push,
    };
    for scope in [primary_scope(), secondary_scope()] {
        store
            .register_collection(scope, COLLECTION, config.clone())
            .expect("register collection");
    }

    // Multi-tenant seed: several keys per tenant, differing TTLs, plus one
    // short-lived key that expires before EXPIRED_NOW.
    store
        .commit(
            &primary_scope(),
            COLLECTION,
            SOURCE_ID,
            vec![
                fact_mutation(HOT_COMMIT, &["alice", "bob"], &["test", "lint"], 1, 300),
                fact_mutation("commit-def", &["carol"], &["test"], 2, 120),
                fact_mutation(EXPIRED_COMMIT, &["dave"], &["test", "lint"], 3, 10),
            ],
            NOW,
        )
        .expect("commit primary tenant facts");
    store
        .commit(
            &secondary_scope(),
            COLLECTION,
            SOURCE_ID,
            vec![fact_mutation(HOT_COMMIT, &["erin"], &["test"], 1, 300)],
            NOW,
        )
        .expect("commit secondary tenant facts");

    let environment = compile_environment();
    let policies: Vec<CompiledPolicy> = POLICIES
        .iter()
        .map(|spec| {
            CompiledPolicy::compile(&policy_source(spec), &environment)
                .expect("hot policy compiles")
        })
        .collect();
    let hot_policy = policies[0].clone();
    let budget_policy = CompiledPolicy::compile(&policy_source(&BUDGET_POLICY), &environment)
        .expect("budget probe policy compiles");

    let snapshot = store.snapshot(primary_scope());
    let input = json!({"operation": "merge", "commit": HOT_COMMIT});
    let expired_input = json!({"operation": "merge", "commit": EXPIRED_COMMIT});

    // Setup sanity gates: the hot path allows on every policy, the expired
    // fact is rejected as indeterminate, and the bounded policy exhausts.
    for policy in &policies {
        let result = policy.evaluate(&input, &snapshot, NOW, EvalBudget::default());
        assert!(
            matches!(result.outcome, PolicyOutcome::Allow),
            "policy {} did not allow on the seeded facts: {:?}",
            policy.name,
            result.outcome
        );
    }
    let expired =
        hot_policy.evaluate(&expired_input, &snapshot, EXPIRED_NOW, EvalBudget::default());
    assert!(
        matches!(expired.outcome, PolicyOutcome::Indeterminate(PolicyFailure::ExpiredFact { .. })),
        "expired fact was not rejected: {:?}",
        expired.outcome
    );
    let exhausted = budget_policy.evaluate(&input, &snapshot, NOW, BENCH_BUDGET);
    assert!(
        matches!(exhausted.outcome, PolicyOutcome::Indeterminate(PolicyFailure::BudgetExceeded)),
        "budget probe did not exhaust: {:?}",
        exhausted.outcome
    );

    BenchFixture {
        environment,
        policies,
        hot_policy,
        budget_policy,
        input,
        expired_input,
        snapshot,
        _store: store,
        _directory: directory,
    }
}

fn bench_v2_compile(criterion: &mut Criterion) {
    let fixture = fixture();
    let mut group = criterion.benchmark_group("v2_compile");
    for count in [1usize, 8] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_function(count.to_string(), |bench| {
            bench.iter(|| {
                for spec in &POLICIES[..count] {
                    black_box(
                        CompiledPolicy::compile(&policy_source(spec), &fixture.environment)
                            .expect("policy compiles"),
                    );
                }
            });
        });
    }
    group.finish();
}

fn bench_v2_evaluate(criterion: &mut Criterion) {
    let fixture = fixture();
    let mut group = criterion.benchmark_group("v2_evaluate");
    for count in [1usize, 8] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_function(count.to_string(), |bench| {
            bench.iter(|| {
                for policy in &fixture.policies[..count] {
                    black_box(policy.evaluate(
                        &fixture.input,
                        &fixture.snapshot,
                        NOW,
                        EvalBudget::default(),
                    ));
                }
            });
        });
    }
    group.finish();
}

fn bench_v2_evaluate_ttl_expired(criterion: &mut Criterion) {
    let fixture = fixture();
    criterion.bench_function("v2_evaluate_ttl_expired", |bench| {
        bench.iter(|| {
            black_box(fixture.hot_policy.evaluate(
                &fixture.expired_input,
                &fixture.snapshot,
                EXPIRED_NOW,
                EvalBudget::default(),
            ))
        });
    });
}

fn bench_v2_evaluate_budget_exhaustion(criterion: &mut Criterion) {
    let fixture = fixture();
    criterion.bench_function("v2_evaluate_budget_exhaustion", |bench| {
        bench.iter(|| {
            black_box(fixture.budget_policy.evaluate(
                &fixture.input,
                &fixture.snapshot,
                NOW,
                BENCH_BUDGET,
            ))
        });
    });
}

const BENCH_IDS: [&str; 6] = [
    "v2_compile/1",
    "v2_compile/8",
    "v2_evaluate/1",
    "v2_evaluate/8",
    "v2_evaluate_ttl_expired",
    "v2_evaluate_budget_exhaustion",
];

#[derive(Deserialize)]
struct CriterionEstimates {
    mean: CriterionEstimate,
    median: CriterionEstimate,
}

#[derive(Deserialize)]
struct CriterionEstimate {
    point_estimate: f64,
    confidence_interval: CriterionConfidenceInterval,
}

#[derive(Deserialize)]
struct CriterionConfidenceInterval {
    upper_bound: f64,
}

#[derive(Deserialize)]
struct CargoMetadata {
    target_directory: PathBuf,
}

/// Mirrors criterion 0.5's default output-directory resolution so the summary
/// below reads the same files criterion just wrote.
fn criterion_output_directory() -> PathBuf {
    if let Some(home) = std::env::var_os("CRITERION_HOME") {
        return PathBuf::from(home);
    }
    if let Some(target) = std::env::var_os("CARGO_TARGET_DIR") {
        return PathBuf::from(target).join("criterion");
    }
    if let Some(output) = std::env::var_os("CARGO")
        .and_then(|cargo| {
            Command::new(cargo).args(["metadata", "--format-version", "1"]).output().ok()
        })
        .and_then(|output| serde_json::from_slice::<CargoMetadata>(&output.stdout).ok())
    {
        return output.target_directory.join("criterion");
    }
    PathBuf::from("target/criterion")
}

fn format_ns(nanos: f64) -> String {
    if nanos.abs() >= 1_000_000.0 {
        format!("{:.2} ms", nanos / 1_000_000.0)
    } else if nanos.abs() >= 1_000.0 {
        format!("{:.2} µs", nanos / 1_000.0)
    } else {
        format!("{:.0} ns", nanos)
    }
}

/// Prints one p50/p99-style line per benchmark from the criterion estimates
/// criterion just persisted: mean, median (p50), and the upper bound of the
/// mean's 95% confidence interval as a conservative tail proxy.
fn print_estimate_summary(output_directory: &Path) {
    let skips_summary = std::env::args().any(|argument| {
        matches!(argument.as_str(), "--list" | "--test") || argument.starts_with("--profile-time")
    });
    if skips_summary {
        return;
    }
    println!();
    println!("v2 evaluation summary (criterion estimates from this run):");
    println!(
        "{:<34} {:>12} {:>14} {:>16}",
        "benchmark", "mean", "median (p50)", "mean CI95 upper"
    );
    for id in BENCH_IDS {
        let estimates = std::fs::read(output_directory.join(id).join("new").join("estimates.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<CriterionEstimates>(&bytes).ok());
        match estimates {
            Some(estimates) => println!(
                "{:<34} {:>12} {:>14} {:>16}",
                id,
                format_ns(estimates.mean.point_estimate),
                format_ns(estimates.median.point_estimate),
                format_ns(estimates.mean.confidence_interval.upper_bound),
            ),
            None => println!("{:<34} {:>12} {:>14} {:>16}", id, "n/a", "n/a", "n/a"),
        }
    }
    println!();
    println!("Quick-pass numbers are intentionally short and noisy; regenerate durable numbers on");
    println!("CI or known-good hardware with:");
    println!("  cargo bench -p ipe-core --features approvals --bench v2_evaluation -- \\");
    println!("    --warm-up-time 3 --measurement-time 10 --save-baseline v2-evaluation");
}

fn main() {
    let output_directory = criterion_output_directory();
    let mut criterion =
        Criterion::default().output_directory(&output_directory).configure_from_args();
    bench_v2_compile(&mut criterion);
    bench_v2_evaluate(&mut criterion);
    bench_v2_evaluate_ttl_expired(&mut criterion);
    bench_v2_evaluate_budget_exhaustion(&mut criterion);
    criterion.final_summary();
    print_estimate_summary(&output_directory);
}
