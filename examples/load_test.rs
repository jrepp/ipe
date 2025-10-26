// Load test for IPE policy engine
// Validates performance under sustained load
//
// Usage:
//   cargo run --release --example load_test -- --evals 1000000 --threads 8 --policies 1000
//
// Performance Targets:
//   - Throughput: >20k ops/sec (single-thread)
//   - Throughput: >100k ops/sec (8 threads)
//   - P99 latency: <50μs (interpreter)
//   - P99 latency: <10μs (JIT)

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use ipe_core::{
    bytecode::{CompiledPolicy, Instruction, Value},
    engine::Decision,
    rar::{Action, EvaluationContext, Principal, Request, Resource},
};

#[derive(Debug, Clone)]
struct LoadTestConfig {
    num_evaluations: usize,
    num_threads: usize,
    num_policies: usize,
    warmup_seconds: u64,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            num_evaluations: 100_000,
            num_threads: 1,
            num_policies: 100,
            warmup_seconds: 5,
        }
    }
}

#[derive(Debug)]
struct LoadTestResults {
    total_evaluations: usize,
    total_duration: Duration,
    throughput_ops_per_sec: f64,
    latencies_us: Vec<u64>,
    p50_latency_us: u64,
    p99_latency_us: u64,
    p999_latency_us: u64,
    max_latency_us: u64,
}

/// Create a sample policy for load testing
fn create_test_policy(id: usize) -> CompiledPolicy {
    CompiledPolicy {
        name: format!("Policy_{}", id),
        code: vec![
            Instruction::LoadField { offset: 0 },
            Instruction::LoadConst { idx: 0 },
            Instruction::Compare {
                op: ipe_core::bytecode::CompOp::Eq,
            },
            Instruction::JumpIfFalse { offset: 2 },
            Instruction::Return { value: true },
            Instruction::Return { value: false },
        ],
        constants: vec![Value::String("Deployment".to_string())],
    }
}

/// Create a sample context for evaluation
fn create_test_context(id: usize) -> EvaluationContext {
    let mut resource_attrs = HashMap::new();
    resource_attrs.insert("type".to_string(), Value::String("Deployment".to_string()));
    resource_attrs.insert(
        "environment".to_string(),
        Value::String(if id % 2 == 0 {
            "production".to_string()
        } else {
            "staging".to_string()
        }),
    );
    resource_attrs.insert(
        "risk_level".to_string(),
        Value::String(match id % 4 {
            0 => "low",
            1 => "medium",
            2 => "high",
            _ => "critical",
        }
        .to_string()),
    );

    let mut principal_attrs = HashMap::new();
    principal_attrs.insert("role".to_string(), Value::String("developer".to_string()));
    principal_attrs.insert(
        "department".to_string(),
        Value::String("engineering".to_string()),
    );

    EvaluationContext {
        resource: Resource {
            type_id: 1,
            attributes: resource_attrs,
        },
        action: Action {
            operation: "Deploy".to_string(),
            target: format!("env-{}/region-{}", id % 3, id % 5),
        },
        request: Request {
            principal: Principal {
                id: format!("user:{}", id % 100),
                roles: vec!["developer".to_string()],
                attributes: principal_attrs,
            },
            timestamp: chrono::Utc::now(),
            source_ip: Some(format!("10.0.{}.{}", (id / 256) % 256, id % 256).parse().unwrap()),
            metadata: HashMap::new(),
        },
        history: None,
    }
}

/// Simulate policy evaluation (placeholder)
fn evaluate_policy(_policy: &CompiledPolicy, _context: &EvaluationContext) -> Decision {
    // This is a placeholder - actual evaluation logic would go here
    // For load testing, we simulate some work
    std::hint::black_box(Decision::Allow)
}

/// Run load test on a single thread
fn run_single_thread_test(
    policies: Arc<Vec<CompiledPolicy>>,
    num_evals: usize,
) -> Vec<Duration> {
    let mut latencies = Vec::with_capacity(num_evals);

    for i in 0..num_evals {
        let policy = &policies[i % policies.len()];
        let context = create_test_context(i);

        let start = Instant::now();
        let _decision = evaluate_policy(policy, &context);
        let elapsed = start.elapsed();

        latencies.push(elapsed);
    }

    latencies
}

/// Run load test with multiple threads
fn run_multi_thread_test(
    policies: Arc<Vec<CompiledPolicy>>,
    num_evals: usize,
    num_threads: usize,
) -> Vec<Duration> {
    let evals_per_thread = num_evals / num_threads;
    let counter = Arc::new(AtomicU64::new(0));

    let mut handles = vec![];

    for thread_id in 0..num_threads {
        let policies = Arc::clone(&policies);
        let counter = Arc::clone(&counter);

        let handle = thread::spawn(move || {
            let mut thread_latencies = Vec::with_capacity(evals_per_thread);

            for i in 0..evals_per_thread {
                let global_i = counter.fetch_add(1, Ordering::Relaxed) as usize;
                let policy = &policies[global_i % policies.len()];
                let context = create_test_context(global_i);

                let start = Instant::now();
                let _decision = evaluate_policy(policy, &context);
                let elapsed = start.elapsed();

                thread_latencies.push(elapsed);
            }

            thread_latencies
        });

        handles.push(handle);
    }

    // Collect results from all threads
    let mut all_latencies = Vec::new();
    for handle in handles {
        let thread_latencies = handle.join().unwrap();
        all_latencies.extend(thread_latencies);
    }

    all_latencies
}

/// Calculate percentiles from latency samples
fn calculate_percentiles(latencies: &[Duration]) -> (u64, u64, u64, u64) {
    let mut latencies_us: Vec<u64> = latencies.iter().map(|d| d.as_micros() as u64).collect();
    latencies_us.sort_unstable();

    let p50 = latencies_us[latencies_us.len() * 50 / 100];
    let p99 = latencies_us[latencies_us.len() * 99 / 100];
    let p999 = latencies_us[latencies_us.len() * 999 / 1000];
    let max = *latencies_us.last().unwrap();

    (p50, p99, p999, max)
}

/// Run the load test
fn run_load_test(config: LoadTestConfig) -> LoadTestResults {
    println!("🚀 IPE Load Test");
    println!("================");
    println!("Configuration:");
    println!("  Evaluations: {}", config.num_evaluations);
    println!("  Threads: {}", config.num_threads);
    println!("  Policies: {}", config.num_policies);
    println!("  Warmup: {}s", config.warmup_seconds);
    println!();

    // Create test policies
    println!("📋 Creating {} test policies...", config.num_policies);
    let policies: Vec<_> = (0..config.num_policies)
        .map(|i| create_test_policy(i))
        .collect();
    let policies = Arc::new(policies);
    println!("✅ Policies created\n");

    // Warmup
    if config.warmup_seconds > 0 {
        println!("🔥 Warming up for {}s...", config.warmup_seconds);
        let warmup_start = Instant::now();
        while warmup_start.elapsed() < Duration::from_secs(config.warmup_seconds) {
            let context = create_test_context(0);
            evaluate_policy(&policies[0], &context);
        }
        println!("✅ Warmup complete\n");
    }

    // Run load test
    println!("⚡ Running load test...");
    let start = Instant::now();

    let latencies = if config.num_threads == 1 {
        run_single_thread_test(Arc::clone(&policies), config.num_evaluations)
    } else {
        run_multi_thread_test(
            Arc::clone(&policies),
            config.num_evaluations,
            config.num_threads,
        )
    };

    let total_duration = start.elapsed();

    // Calculate statistics
    let (p50, p99, p999, max) = calculate_percentiles(&latencies);
    let throughput = config.num_evaluations as f64 / total_duration.as_secs_f64();

    LoadTestResults {
        total_evaluations: config.num_evaluations,
        total_duration,
        throughput_ops_per_sec: throughput,
        latencies_us: latencies.iter().map(|d| d.as_micros() as u64).collect(),
        p50_latency_us: p50,
        p99_latency_us: p99,
        p999_latency_us: p999,
        max_latency_us: max,
    }
}

/// Print test results
fn print_results(results: &LoadTestResults) {
    println!("\n📊 Results");
    println!("==========");
    println!("Total evaluations: {}", results.total_evaluations);
    println!("Total duration: {:.2}s", results.total_duration.as_secs_f64());
    println!(
        "Throughput: {:.0} ops/sec",
        results.throughput_ops_per_sec
    );
    println!();
    println!("Latency Percentiles:");
    println!("  P50:  {:6}μs", results.p50_latency_us);
    println!("  P99:  {:6}μs", results.p99_latency_us);
    println!("  P99.9: {:6}μs", results.p999_latency_us);
    println!("  Max:  {:6}μs", results.max_latency_us);
    println!();

    // Validate performance targets
    println!("🎯 Performance Validation:");

    let throughput_target = 20_000.0;
    if results.throughput_ops_per_sec >= throughput_target {
        println!("  ✅ Throughput: {:.0} ops/sec (target: {:.0})", results.throughput_ops_per_sec, throughput_target);
    } else {
        println!("  ❌ Throughput: {:.0} ops/sec (target: {:.0})", results.throughput_ops_per_sec, throughput_target);
    }

    let p99_target = 50;
    if results.p99_latency_us <= p99_target {
        println!(
            "  ✅ P99 latency: {}μs (target: <{}μs)",
            results.p99_latency_us, p99_target
        );
    } else {
        println!(
            "  ⚠️  P99 latency: {}μs (target: <{}μs)",
            results.p99_latency_us, p99_target
        );
    }
}

fn main() {
    // Parse command-line arguments (simplified)
    let args: Vec<String> = std::env::args().collect();

    let mut config = LoadTestConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--evals" => {
                config.num_evaluations = args[i + 1].parse().expect("Invalid number");
                i += 2;
            }
            "--threads" => {
                config.num_threads = args[i + 1].parse().expect("Invalid number");
                i += 2;
            }
            "--policies" => {
                config.num_policies = args[i + 1].parse().expect("Invalid number");
                i += 2;
            }
            "--warmup" => {
                config.warmup_seconds = args[i + 1].parse().expect("Invalid number");
                i += 2;
            }
            _ => {
                println!("Unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }

    let results = run_load_test(config);
    print_results(&results);
}
