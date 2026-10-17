// Criterion benchmarks for IPE policy evaluation
// These benchmarks validate performance targets:
// - Single policy eval: <50μs p99 (interpreter)
// - Single policy eval: <10μs p99 (JIT)
// - 1000 policies: <500μs p99

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use ipe_core::{
    bytecode::{CompOp, CompiledPolicy, Instruction, PolicyHeader, Value},
    engine::PolicyEngine,
    index::PolicyDB,
    interpreter::{FieldMapping, Interpreter},
    rar::{
        Action, AttributeValue, EvaluationContext, Operation, Principal, Request, Resource,
        ResourceTypeId,
    },
};
use std::collections::HashMap;
use std::time::Duration;

/// Create a sample RAR context for testing
fn create_sample_context() -> EvaluationContext {
    let mut resource_attrs = HashMap::new();
    resource_attrs.insert("type".to_string(), AttributeValue::String("Deployment".to_string()));
    resource_attrs
        .insert("environment".to_string(), AttributeValue::String("production".to_string()));
    resource_attrs.insert("risk_level".to_string(), AttributeValue::String("high".to_string()));

    let mut principal_attrs = HashMap::new();
    principal_attrs.insert("role".to_string(), AttributeValue::String("developer".to_string()));
    principal_attrs
        .insert("department".to_string(), AttributeValue::String("engineering".to_string()));

    EvaluationContext {
        resource: Resource {
            type_id: ipe_core::rar::ResourceTypeId(1),
            attributes: resource_attrs,
        },
        action: Action {
            operation: Operation::Deploy,
            target: "production/us-east-1".to_string(),
            attributes: HashMap::new(),
        },
        request: Request {
            principal: Principal {
                id: "user:alice".to_string(),
                roles: vec!["developer".to_string(), "senior-engineer".to_string()],
                attributes: principal_attrs,
            },
            timestamp: chrono::Utc::now().timestamp(),
            source_ip: Some("10.0.1.42".parse().unwrap()),
            metadata: HashMap::new(),
        },
        #[cfg(feature = "approvals")]
        approval_store: None,
        #[cfg(feature = "approvals")]
        relationship_store: None,
    }
}

/// Create a simple compiled policy for benchmarking
fn create_sample_policy() -> CompiledPolicy {
    let code = vec![
        Instruction::LoadField { offset: 0 },
        Instruction::LoadConst { idx: 0 },
        Instruction::Compare { op: ipe_core::bytecode::CompOp::Eq },
        Instruction::Return { value: true },
    ];
    let constants = vec![Value::String("production".to_string())];

    CompiledPolicy {
        header: PolicyHeader {
            magic: *b"IPE\0",
            version: 1,
            policy_id: 1,
            code_size: code.len() as u32,
            const_size: constants.len() as u32,
        },
        code,
        constants,
    }
}

/// Sample field map for the sample policy: offset 0 -> resource.type
fn sample_field_map() -> FieldMapping {
    let mut field_map = FieldMapping::new();
    field_map.insert(0, vec!["resource".to_string(), "environment".to_string()]);
    field_map
}

/// Build a PolicyDB with `policy_count` allow-policies matching resource type 1
fn build_policy_db(policy_count: usize) -> PolicyDB {
    let mut db = PolicyDB::new();
    for i in 0..policy_count {
        let mut policy = CompiledPolicy::new(i as u64);
        policy.emit(Instruction::LoadField { offset: 0 });
        let idx = policy.add_constant(Value::String("production".to_string()));
        policy.emit(Instruction::LoadConst { idx });
        policy.emit(Instruction::Compare { op: CompOp::Eq });
        policy.emit(Instruction::Return { value: true });
        db.add_policy(format!("policy-{i}"), policy, sample_field_map(), vec![ResourceTypeId(1)]);
    }
    db
}

/// Benchmark: Single policy evaluation (interpreter)
fn bench_single_policy_interpreter(c: &mut Criterion) {
    let policy = create_sample_policy();
    let context = create_sample_context();
    let mut interpreter = Interpreter::new(sample_field_map());

    c.bench_function("single_policy_interpreter", |b| {
        b.iter(|| {
            // Evaluate the policy for real through the bytecode interpreter
            let result = interpreter
                .evaluate(black_box(&policy), black_box(&context))
                .expect("policy evaluation failed");
            black_box(result)
        })
    });
}

/// Benchmark: Single policy evaluation (JIT) - when implemented
#[cfg(feature = "jit")]
fn bench_single_policy_jit(c: &mut Criterion) {
    let policy = create_sample_policy();
    let context = create_sample_context();

    c.bench_function("single_policy_jit", |b| {
        b.iter(|| {
            // Evaluate policy (JIT mode)
            // Note: This is a placeholder - actual implementation needed
            black_box(&policy);
            black_box(&context);
            Decision::allow()
        })
    });
}

/// Benchmark: Multiple policies with varying counts
fn bench_multiple_policies(c: &mut Criterion) {
    let context = create_sample_context();

    let mut group = c.benchmark_group("multiple_policies");
    for policy_count in [10, 100, 1000, 10000] {
        let engine = PolicyEngine::with_policy_db(build_policy_db(policy_count));

        group.throughput(Throughput::Elements(policy_count as u64));
        group.bench_with_input(BenchmarkId::from_parameter(policy_count), &engine, |b, engine| {
            b.iter(|| {
                // Evaluate all applicable policies through the engine
                let decision = engine.evaluate(&context).expect("evaluation failed");
                black_box(decision)
            })
        });
    }
    group.finish();
}

/// Benchmark: PolicyEngine::evaluate end to end (index lookup + interpreter)
/// at small policy counts - the per-request hot path
fn bench_engine_evaluate(c: &mut Criterion) {
    let context = create_sample_context();

    let mut group = c.benchmark_group("engine_evaluate");
    for policy_count in [1, 8] {
        let engine = PolicyEngine::with_policy_db(build_policy_db(policy_count));

        group.bench_with_input(BenchmarkId::from_parameter(policy_count), &engine, |b, engine| {
            b.iter(|| {
                let decision = engine.evaluate(&context).expect("evaluation failed");
                black_box(decision)
            })
        });
    }
    group.finish();
}

/// Benchmark: Policy compilation
fn bench_policy_compilation(c: &mut Criterion) {
    c.bench_function("policy_compilation", |b| {
        b.iter(|| {
            // Compile a policy from source
            // Note: This is a placeholder - actual implementation needed
            let _policy = create_sample_policy();
        })
    });
}

/// Benchmark: Context creation overhead
fn bench_context_creation(c: &mut Criterion) {
    c.bench_function("context_creation", |b| {
        b.iter(|| {
            black_box(create_sample_context());
        })
    });
}

/// Benchmark: Memory-mapped policy loading
fn bench_policy_loading(c: &mut Criterion) {
    c.bench_function("policy_loading", |b| {
        b.iter(|| {
            // Load policies from disk
            // Note: This is a placeholder - actual implementation needed
            black_box(create_sample_policy());
        })
    });
}

/// Benchmark: Concurrent evaluation (8 threads)
fn bench_concurrent_evaluation(c: &mut Criterion) {
    use std::sync::Arc;
    use std::thread;

    let policy = Arc::new(create_sample_policy());
    let context = Arc::new(create_sample_context());

    c.bench_function("concurrent_evaluation_8threads", |b| {
        b.iter(|| {
            let mut handles = vec![];

            for _ in 0..8 {
                let policy = Arc::clone(&policy);
                let context = Arc::clone(&context);

                let handle = thread::spawn(move || {
                    for _ in 0..100 {
                        black_box(&*policy);
                        black_box(&*context);
                        // Decision evaluation would go here
                    }
                });

                handles.push(handle);
            }

            for handle in handles {
                handle.join().unwrap();
            }
        })
    });
}

// Configure Criterion
fn configure_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100) // Number of samples
        .measurement_time(Duration::from_secs(10)) // Time to run each benchmark
        .warm_up_time(Duration::from_secs(3)) // Warm-up time
        .with_plots() // Generate plots
}

// Benchmark groups
criterion_group! {
    name = benches;
    config = configure_criterion();
    targets =
        bench_single_policy_interpreter,
        bench_multiple_policies,
        bench_engine_evaluate,
        bench_policy_compilation,
        bench_context_creation,
        bench_policy_loading,
        bench_concurrent_evaluation,
}

#[cfg(feature = "jit")]
criterion_group! {
    name = jit_benches;
    config = configure_criterion();
    targets = bench_single_policy_jit,
}

#[cfg(feature = "jit")]
criterion_main!(benches, jit_benches);

#[cfg(not(feature = "jit"))]
criterion_main!(benches);
