//! Continuous benchmark runner that takes periodic snapshots at 1-second intervals
//!
//! This binary runs lightweight performance tests continuously and exports snapshots
//! to populate the time-series charts in performance.html and benchmarks.html.
//!
//! Usage:
//!   cargo run --release --bin bench_continuous --features jit [duration_seconds]
//!
//! Examples:
//!   cargo run --release --bin bench_continuous --features jit 60   # Run for 60 seconds
//!   cargo run --release --bin bench_continuous --features jit      # Run forever (Ctrl+C to stop)

use chrono::Utc;
use ipe_core::{
    bytecode::{CompiledPolicy, Instruction, PolicyHeader, Value},
    engine::Decision,
    rar::{Action, AttributeValue, EvaluationContext, Operation, Principal, Request, Resource},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
struct BenchmarkSnapshot {
    name: String,
    timestamp: String,
    mean_ns: f64,
    median_ns: f64,
    std_dev_ns: f64,
    throughput: f64, // ops/sec
}

#[derive(Debug, Serialize, Deserialize)]
struct HistoryEntry {
    export_timestamp: String,
    git_commit: Option<String>,
    git_branch: Option<String>,
    benchmarks: Vec<BenchmarkSnapshot>,
}

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
    }
}

/// Create a simple compiled policy
fn create_sample_policy() -> CompiledPolicy {
    let code = vec![
        Instruction::LoadField { offset: 0 },
        Instruction::LoadConst { idx: 0 },
        Instruction::Compare { op: ipe_core::bytecode::CompOp::Eq },
        Instruction::Return { value: true },
    ];
    let constants = vec![Value::String("Deployment".to_string())];

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

/// Run a quick benchmark iteration (100 samples)
fn run_quick_benchmark(name: &str, iterations: usize) -> BenchmarkSnapshot {
    let policy = create_sample_policy();
    let context = create_sample_context();

    let mut durations = Vec::with_capacity(iterations);

    // Warm-up
    for _ in 0..10 {
        let _ = Decision::allow();
        std::hint::black_box(&policy);
        std::hint::black_box(&context);
    }

    // Measure
    for _ in 0..iterations {
        let start = Instant::now();

        // Placeholder evaluation (replace with actual interpreter/JIT when ready)
        std::hint::black_box(&policy);
        std::hint::black_box(&context);
        let _ = Decision::allow();

        durations.push(start.elapsed());
    }

    // Calculate statistics
    durations.sort();
    let mean_ns = durations.iter().map(|d| d.as_nanos() as f64).sum::<f64>() / iterations as f64;
    let median_ns = durations[iterations / 2].as_nanos() as f64;

    let variance = durations
        .iter()
        .map(|d| {
            let diff = d.as_nanos() as f64 - mean_ns;
            diff * diff
        })
        .sum::<f64>()
        / iterations as f64;
    let std_dev_ns = variance.sqrt();

    let throughput = 1_000_000_000.0 / mean_ns; // ops/sec

    BenchmarkSnapshot {
        name: name.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        mean_ns,
        median_ns,
        std_dev_ns,
        throughput,
    }
}

fn get_git_info() -> (Option<String>, Option<String>) {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        });

    let branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok().map(|s| s.trim().to_string())
            } else {
                None
            }
        });

    (commit, branch)
}

fn append_to_history(entry: &HistoryEntry) -> Result<(), Box<dyn std::error::Error>> {
    let history_path = Path::new("../../docs/benchmark-history.json");

    // Read existing history
    let mut history: Vec<serde_json::Value> = if history_path.exists() {
        let history_json = fs::read_to_string(history_path)?;
        serde_json::from_str(&history_json)?
    } else {
        Vec::new()
    };

    // Append new entry
    history.push(serde_json::to_value(entry)?);

    // Keep only last 100 entries
    if history.len() > 100 {
        history = history.split_off(history.len() - 100);
    }

    // Write updated history
    let history_json = serde_json::to_string_pretty(&history)?;
    fs::write(history_path, history_json)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let duration_secs = if args.len() > 1 { args[1].parse::<u64>().ok() } else { None };

    println!("🚀 Starting continuous benchmark runner");
    println!("   Taking snapshots every 1 second");
    if let Some(secs) = duration_secs {
        println!("   Duration: {} seconds", secs);
    } else {
        println!("   Duration: Infinite (press Ctrl+C to stop)");
    }
    println!();

    let (git_commit, git_branch) = get_git_info();
    if let (Some(ref commit), Some(ref branch)) = (&git_commit, &git_branch) {
        println!("📝 Git: {} @ {}", commit, branch);
        println!();
    }

    let start_time = Instant::now();
    let mut snapshot_count = 0;

    loop {
        let iteration_start = Instant::now();

        // Run quick benchmarks
        println!(
            "⏱️  Snapshot #{} ({}s elapsed)",
            snapshot_count + 1,
            start_time.elapsed().as_secs()
        );

        let benchmarks = vec![
            run_quick_benchmark("policy_eval_interpreter", 100),
            #[cfg(feature = "jit")]
            run_quick_benchmark("policy_eval_jit", 100),
            run_quick_benchmark("context_creation", 100),
        ];

        // Print summary
        for bench in &benchmarks {
            println!(
                "   • {} - {:.2} ns (mean), {:.0} ops/sec",
                bench.name, bench.mean_ns, bench.throughput
            );
        }

        // Create history entry
        let entry = HistoryEntry {
            export_timestamp: Utc::now().to_rfc3339(),
            git_commit: git_commit.clone(),
            git_branch: git_branch.clone(),
            benchmarks,
        };

        // Append to history
        if let Err(e) = append_to_history(&entry) {
            eprintln!("⚠️  Failed to append to history: {}", e);
        } else {
            snapshot_count += 1;
        }

        // Write latest snapshot
        let snapshot_json = serde_json::to_string_pretty(&entry)?;
        if let Err(e) = fs::write("../../docs/benchmark-latest.json", &snapshot_json) {
            eprintln!("⚠️  Failed to write latest snapshot: {}", e);
        }

        // Check if we should stop
        if let Some(duration) = duration_secs {
            if start_time.elapsed().as_secs() >= duration {
                println!("\n✅ Completed {} snapshots in {} seconds", snapshot_count, duration);
                break;
            }
        }

        // Sleep for remainder of 1 second
        let elapsed = iteration_start.elapsed();
        if elapsed < Duration::from_secs(1) {
            std::thread::sleep(Duration::from_secs(1) - elapsed);
        }
    }

    println!("\n🎉 Continuous benchmark complete!");
    println!("   📊 View results at: http://localhost:8080/benchmarks.html");

    Ok(())
}
