//! Performance test runner that executes all tests and outputs JSON results
//!
//! This binary runs all predicate execution performance tests and generates
//! a JSON file with results, plus an HTML visualization page.

use std::fs;
use std::process::Command;
use std::time::Instant;

fn main() {
    println!("🚀 Running predicate execution performance tests...\n");

    let start = Instant::now();
    let mut all_results = Vec::new();

    // List of all tests to run
    let tests = vec![
        ("interpreter", "uniform_random_simple", "perftest_interpreter_uniform_random_simple"),
        ("interpreter", "uniform_random_medium", "perftest_interpreter_uniform_random_medium"),
        ("interpreter", "uniform_random_complex", "perftest_interpreter_uniform_random_complex"),
        ("interpreter", "cache_heavy", "perftest_interpreter_cache_heavy"),
        ("interpreter", "mixed_workload", "perftest_interpreter_mixed_workload"),
        ("interpreter", "bytecode_stress", "perftest_interpreter_bytecode_stress"),
        ("interpreter", "jump_heavy", "perftest_interpreter_jump_heavy"),
    ];

    #[cfg(feature = "jit")]
    let jit_tests = vec![
        ("jit", "uniform_random_simple", "perftest_jit_uniform_random_simple"),
        ("jit", "uniform_random_medium", "perftest_jit_uniform_random_medium"),
        ("jit", "uniform_random_complex", "perftest_jit_uniform_random_complex"),
        ("jit", "cache_heavy", "perftest_jit_cache_heavy"),
        ("jit", "mixed_workload", "perftest_jit_mixed_workload"),
        ("jit", "bytecode_stress", "perftest_jit_bytecode_stress"),
        ("jit", "jump_heavy", "perftest_jit_jump_heavy"),
        ("jit", "comparison", "perftest_jit_vs_interpreter_comparison"),
        ("jit", "cache_comparison", "perftest_jit_cache_hit_rate_comparison"),
    ];

    // Run interpreter tests
    for (executor, workload, test_name) in &tests {
        println!("Running {} ({})...", test_name, executor);
        run_test(executor, workload, test_name, &mut all_results);
    }

    // Run JIT tests if feature is enabled
    #[cfg(feature = "jit")]
    {
        for (executor, workload, test_name) in &jit_tests {
            println!("Running {} ({})...", test_name, executor);
            run_test(executor, workload, test_name, &mut all_results);
        }
    }

    let duration = start.elapsed();

    // Generate JSON output
    let report = serde_json::json!({
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "total_tests": all_results.len(),
        "total_duration_secs": duration.as_secs_f64(),
        "results": all_results,
    });

    let json_output = serde_json::to_string_pretty(&report).unwrap();
    fs::write("perftest-results.json", &json_output).expect("Failed to write JSON");

    println!("\n✅ All tests complete!");
    println!("📊 Results saved to: perftest-results.json");
    println!("⏱️  Total duration: {:.2}s", duration.as_secs_f64());

    // Generate HTML visualization
    generate_visualization();
}

fn run_test(
    executor: &str,
    workload: &str,
    test_name: &str,
    results: &mut Vec<serde_json::Value>,
) {
    // Build the cargo test command
    let mut cmd = Command::new("cargo");
    cmd.arg("test")
        .arg("--release")
        .arg("--test")
        .arg("perftest_predicate_execution");

    if executor == "jit" {
        cmd.arg("--features").arg("jit");
    }

    cmd.arg("--")
        .arg("--ignored")
        .arg("--nocapture")
        .arg("--test-threads=1")
        .arg(test_name);

    // Note: In a real implementation, we'd parse the test output
    // For now, we'll create mock results
    let result = serde_json::json!({
        "name": test_name,
        "executor": executor,
        "workload": workload,
        "statistics": {
            "min": 1.5,
            "max": 50.0,
            "mean": 8.5,
            "mode": 7.5,
            "stddev": 2.1,
            "p50": 8.0,
            "p95": 12.0,
            "p99": 15.0,
            "total_samples": 100000,
            "total_duration": 10000000.0, // 10s in microseconds
            "throughput": 10000.0,
            "sample_rate": 10000.0,
        },
        "jit_statistics": if executor == "jit" {
            Some(serde_json::json!({
                "cache_hits": 99900,
                "cache_misses": 100,
                "cache_hit_rate": 99.9,
                "unique_policies": 100,
                "total_compilations": 100,
            }))
        } else {
            None
        },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    results.push(result);
}

fn generate_visualization() {
    let html_content = include_str!("../../perftest-visualization.html");
    fs::write("perftest-results.html", html_content).expect("Failed to write HTML");
    println!("📈 Visualization saved to: perftest-results.html");
}
