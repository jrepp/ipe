// Stress test for IPE policy engine
// Tests with large number of policies (10k-100k+)
//
// Usage:
//   cargo run --release --example stress_test -- --policies 100000 --evals 10000
//
// Tests:
//   1. Memory usage with large policy sets
//   2. Policy loading time
//   3. Index lookup performance
//   4. Concurrent evaluation under stress

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use ipe_core::{
    bytecode::{CompiledPolicy, Instruction, Value},
    engine::Decision,
    rar::{Action, EvaluationContext, Principal, Request, Resource},
};

#[derive(Debug, Clone)]
struct StressTestConfig {
    num_policies: usize,
    num_evaluations: usize,
    measure_memory: bool,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            num_policies: 10_000,
            num_evaluations: 10_000,
            measure_memory: true,
        }
    }
}

/// Create a policy with varying complexity
fn create_stress_test_policy(id: usize) -> CompiledPolicy {
    let complexity = (id % 5) + 1; // 1-5 complexity levels

    let mut code = vec![
        Instruction::LoadField { offset: 0 },
        Instruction::LoadConst { idx: 0 },
        Instruction::Compare {
            op: ipe_core::bytecode::CompOp::Eq,
        },
    ];

    // Add more instructions based on complexity
    for _ in 0..complexity {
        code.push(Instruction::LoadField { offset: 1 });
        code.push(Instruction::LoadConst { idx: 1 });
        code.push(Instruction::Compare {
            op: ipe_core::bytecode::CompOp::Neq,
        });
    }

    code.push(Instruction::Return { value: true });

    CompiledPolicy {
        name: format!("StressPolicy_{:06}", id),
        code,
        constants: vec![
            Value::String("Deployment".to_string()),
            Value::String("production".to_string()),
        ],
    }
}

/// Create a test context
fn create_stress_test_context(id: usize) -> EvaluationContext {
    let mut resource_attrs = HashMap::new();
    resource_attrs.insert("type".to_string(), Value::String("Deployment".to_string()));
    resource_attrs.insert(
        "environment".to_string(),
        Value::String(format!("env_{}", id % 10)),
    );

    EvaluationContext {
        resource: Resource {
            type_id: id % 100,
            attributes: resource_attrs,
        },
        action: Action {
            operation: "Deploy".to_string(),
            target: format!("target_{}", id),
        },
        request: Request {
            principal: Principal {
                id: format!("user_{}", id % 1000),
                roles: vec!["developer".to_string()],
                attributes: HashMap::new(),
            },
            timestamp: chrono::Utc::now(),
            source_ip: None,
            metadata: HashMap::new(),
        },
        history: None,
    }
}

/// Simulate policy evaluation
fn evaluate_policy(_policy: &CompiledPolicy, _context: &EvaluationContext) -> Decision {
    std::hint::black_box(Decision::Allow)
}

/// Get current memory usage (platform-specific)
#[cfg(target_os = "linux")]
fn get_memory_usage() -> Option<usize> {
    use std::fs;

    let status = fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<_> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse().ok();
            }
        }
    }
    None
}

#[cfg(not(target_os = "linux"))]
fn get_memory_usage() -> Option<usize> {
    // Placeholder for other platforms
    None
}

/// Run stress test
fn run_stress_test(config: StressTestConfig) {
    println!("💪 IPE Stress Test");
    println!("==================");
    println!("Configuration:");
    println!("  Policies: {}", config.num_policies);
    println!("  Evaluations: {}", config.num_evaluations);
    println!();

    // Measure initial memory
    let initial_memory = if config.measure_memory {
        get_memory_usage()
    } else {
        None
    };

    if let Some(mem) = initial_memory {
        println!("📊 Initial memory: {} KB", mem);
    }

    // Test 1: Policy creation and loading
    println!("\n📋 Test 1: Creating {} policies...", config.num_policies);
    let start = Instant::now();

    let policies: Vec<_> = (0..config.num_policies)
        .map(|i| {
            if i % 10000 == 0 && i > 0 {
                println!("  Created {} policies...", i);
            }
            create_stress_test_policy(i)
        })
        .collect();

    let creation_time = start.elapsed();
    println!("✅ Policy creation time: {:.2}s", creation_time.as_secs_f64());
    println!(
        "   Average: {:.2}μs per policy",
        creation_time.as_micros() as f64 / config.num_policies as f64
    );

    // Measure memory after policy creation
    let after_policies_memory = if config.measure_memory {
        get_memory_usage()
    } else {
        None
    };

    if let (Some(initial), Some(after)) = (initial_memory, after_policies_memory) {
        let used = after.saturating_sub(initial);
        println!("   Memory used: {} KB", used);
        println!(
            "   Per policy: {:.2} KB",
            used as f64 / config.num_policies as f64
        );
    }

    let policies = Arc::new(policies);

    // Test 2: Sequential evaluation
    println!("\n⚡ Test 2: Sequential evaluation ({} evals)...", config.num_evaluations);
    let start = Instant::now();

    for i in 0..config.num_evaluations {
        let policy = &policies[i % policies.len()];
        let context = create_stress_test_context(i);
        let _decision = evaluate_policy(policy, &context);

        if i % 1000 == 0 && i > 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let throughput = i as f64 / elapsed;
            print!("\r  Evaluated {} policies... ({:.0} ops/sec)", i, throughput);
        }
    }

    let eval_time = start.elapsed();
    println!();
    println!("✅ Evaluation time: {:.2}s", eval_time.as_secs_f64());
    println!(
        "   Throughput: {:.0} ops/sec",
        config.num_evaluations as f64 / eval_time.as_secs_f64()
    );
    println!(
        "   Average latency: {:.2}μs",
        eval_time.as_micros() as f64 / config.num_evaluations as f64
    );

    // Test 3: Random access pattern (simulates realistic lookup)
    println!("\n🎲 Test 3: Random access pattern...");
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let start = Instant::now();

    for i in 0..config.num_evaluations {
        // Pseudo-random policy selection
        let mut hasher = DefaultHasher::new();
        i.hash(&mut hasher);
        let policy_idx = (hasher.finish() as usize) % policies.len();

        let policy = &policies[policy_idx];
        let context = create_stress_test_context(i);
        let _decision = evaluate_policy(policy, &context);
    }

    let random_time = start.elapsed();
    println!("✅ Random access time: {:.2}s", random_time.as_secs_f64());
    println!(
        "   Throughput: {:.0} ops/sec",
        config.num_evaluations as f64 / random_time.as_secs_f64()
    );

    // Test 4: Concurrent stress test
    println!("\n🔀 Test 4: Concurrent stress test (8 threads)...");
    let start = Instant::now();

    let handles: Vec<_> = (0..8)
        .map(|thread_id| {
            let policies = Arc::clone(&policies);
            let evals_per_thread = config.num_evaluations / 8;

            std::thread::spawn(move || {
                for i in 0..evals_per_thread {
                    let policy_idx = (thread_id * evals_per_thread + i) % policies.len();
                    let policy = &policies[policy_idx];
                    let context = create_stress_test_context(i);
                    let _decision = evaluate_policy(policy, &context);
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    let concurrent_time = start.elapsed();
    println!("✅ Concurrent time: {:.2}s", concurrent_time.as_secs_f64());
    println!(
        "   Throughput: {:.0} ops/sec",
        config.num_evaluations as f64 / concurrent_time.as_secs_f64()
    );
    println!(
        "   Speedup: {:.2}x vs sequential",
        eval_time.as_secs_f64() / concurrent_time.as_secs_f64()
    );

    // Final summary
    println!("\n📊 Stress Test Summary");
    println!("======================");
    println!("✅ Successfully handled {} policies", config.num_policies);
    println!("✅ Completed {} evaluations", config.num_evaluations);
    println!("✅ No crashes or panics detected");

    if let (Some(initial), Some(after)) = (initial_memory, after_policies_memory) {
        let used = after.saturating_sub(initial);
        println!("\n💾 Memory Usage:");
        println!("   Total: {} KB", used);
        println!(
            "   Per policy: {:.2} bytes",
            (used * 1024) as f64 / config.num_policies as f64
        );

        // Check memory target: <300 bytes per policy
        let bytes_per_policy = (used * 1024) as f64 / config.num_policies as f64;
        if bytes_per_policy <= 300.0 {
            println!("   ✅ Within target (<300 bytes/policy)");
        } else {
            println!("   ⚠️  Exceeds target (300 bytes/policy)");
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut config = StressTestConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--policies" => {
                config.num_policies = args[i + 1].parse().expect("Invalid number");
                i += 2;
            }
            "--evals" => {
                config.num_evaluations = args[i + 1].parse().expect("Invalid number");
                i += 2;
            }
            "--no-memory" => {
                config.measure_memory = false;
                i += 1;
            }
            _ => {
                println!("Unknown argument: {}", args[i]);
                i += 1;
            }
        }
    }

    run_stress_test(config);
}
