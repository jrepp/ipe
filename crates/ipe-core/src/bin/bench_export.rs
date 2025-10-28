//! Export Criterion benchmark results to JSON for GitHub Pages timeline visualization
//!
//! This binary collects all benchmark results from target/criterion/ and exports
//! them with timestamps for historical tracking and D3.js visualization.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct ConfidenceInterval {
    confidence_level: f64,
    lower_bound: f64,
    upper_bound: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Estimate {
    confidence_interval: ConfidenceInterval,
    point_estimate: f64,
    standard_error: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct CriterionEstimates {
    mean: Estimate,
    median: Estimate,
    median_abs_dev: Estimate,
    slope: Option<Estimate>,
    std_dev: Estimate,
}

#[derive(Debug, Serialize)]
struct BenchmarkResult {
    name: String,
    timestamp: String,
    mean_ns: f64,
    median_ns: f64,
    std_dev_ns: f64,
    mean_lower: f64,
    mean_upper: f64,
    throughput: Option<f64>,
}

#[derive(Debug, Serialize)]
struct BenchmarkExport {
    export_timestamp: String,
    git_commit: Option<String>,
    git_branch: Option<String>,
    benchmarks: Vec<BenchmarkResult>,
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

fn collect_benchmarks() -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
    let criterion_dir = Path::new("../../target/criterion");
    let mut results = Vec::new();
    let timestamp = Utc::now().to_rfc3339();

    if !criterion_dir.exists() {
        eprintln!("Warning: Criterion directory not found at {:?}", criterion_dir);
        return Ok(results);
    }

    // Iterate through benchmark directories
    for entry in fs::read_dir(criterion_dir)? {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let bench_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");

        // Skip the report directory
        if bench_name == "report" {
            continue;
        }

        // Look for estimates.json in the "new" subdirectory
        let estimates_path = path.join("new/estimates.json");
        if !estimates_path.exists() {
            eprintln!("Warning: No estimates.json found for benchmark: {}", bench_name);
            continue;
        }

        // Read and parse estimates
        let estimates_json = fs::read_to_string(&estimates_path)?;
        let estimates: CriterionEstimates = serde_json::from_str(&estimates_json)?;

        // Check for throughput data (in benchmark.json)
        let benchmark_path = path.join("new/benchmark.json");
        let throughput = if benchmark_path.exists() {
            let benchmark_json = fs::read_to_string(&benchmark_path)?;
            let benchmark_data: serde_json::Value = serde_json::from_str(&benchmark_json)?;
            benchmark_data
                .get("throughput")
                .and_then(|t| t.get("per_iteration"))
                .and_then(|p| p.as_f64())
        } else {
            None
        };

        results.push(BenchmarkResult {
            name: bench_name.to_string(),
            timestamp: timestamp.clone(),
            mean_ns: estimates.mean.point_estimate,
            median_ns: estimates.median.point_estimate,
            std_dev_ns: estimates.std_dev.point_estimate,
            mean_lower: estimates.mean.confidence_interval.lower_bound,
            mean_upper: estimates.mean.confidence_interval.upper_bound,
            throughput,
        });
    }

    Ok(results)
}

fn append_to_history(export: &BenchmarkExport) -> Result<(), Box<dyn std::error::Error>> {
    let history_path = Path::new("../../docs/benchmark-history.json");

    // Read existing history as raw JSON values
    let mut history: Vec<serde_json::Value> = if history_path.exists() {
        let history_json = fs::read_to_string(history_path)?;
        serde_json::from_str(&history_json)?
    } else {
        Vec::new()
    };

    // Append new export as JSON value
    history.push(serde_json::to_value(export)?);

    // Keep only last 100 runs
    if history.len() > 100 {
        history = history.split_off(history.len() - 100);
    }

    // Write updated history
    let history_json = serde_json::to_string_pretty(&history)?;
    fs::write(history_path, history_json)?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Collecting Criterion benchmark results...\n");

    let benchmarks = collect_benchmarks()?;

    if benchmarks.is_empty() {
        eprintln!("❌ No benchmark results found!");
        eprintln!("   Run 'cargo bench' first to generate results.");
        std::process::exit(1);
    }

    let (git_commit, git_branch) = get_git_info();

    let export = BenchmarkExport {
        export_timestamp: Utc::now().to_rfc3339(),
        git_commit,
        git_branch,
        benchmarks,
    };

    println!("📊 Found {} benchmarks:", export.benchmarks.len());
    for bench in &export.benchmarks {
        println!("  • {} - {:.2} ns (mean)", bench.name, bench.mean_ns);
    }

    // Write current snapshot
    let snapshot_json = serde_json::to_string_pretty(&export)?;
    fs::write("../../docs/benchmark-latest.json", &snapshot_json)?;
    println!("\n✅ Latest snapshot saved to: docs/benchmark-latest.json");

    // Append to history
    append_to_history(&export)?;
    println!("✅ History updated in: docs/benchmark-history.json");

    // Also write to local directory for development
    fs::write("benchmark-latest.json", &snapshot_json)?;
    println!("✅ Local copy saved to: benchmark-latest.json");

    println!("\n🎉 Export complete!");
    if let (Some(commit), Some(branch)) = (export.git_commit, export.git_branch) {
        println!("   Git: {} @ {}", commit, branch);
    }

    Ok(())
}
