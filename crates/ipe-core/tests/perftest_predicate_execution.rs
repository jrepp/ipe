//! Performance test for predicate execution
//!
//! This test measures predicate evaluation performance at high scale on a single CPU
//! for short bursts (10 seconds) and collects comprehensive statistical data.
//!
//! Run with:
//!   cargo test --release --test perftest_predicate_execution -- --nocapture --test-threads=1
//!
//! Test configurations (18 tests total):
//! - Uniform random predicates: Test JIT optimization with varied predicates
//! - Cache-heavy predicates: Test cache efficiency with repeated predicates
//! - Mixed workload: 60% simple, 30% medium, 10% complex
//! - Bytecode stress: Deep nesting and many operations
//! - Jump-heavy: Branch prediction stress testing
//! - Cache hit rate comparison: Different JIT optimization strategies

use ipe_core::{
    bytecode::{CompOp, CompiledPolicy, Instruction, Value},
    interpreter::{FieldMapping, Interpreter},
    rar::{AttributeValue, EvaluationContext, ResourceTypeId},
};

#[cfg(feature = "jit")]
use ipe_core::jit::JitCompiler;
use rand::prelude::*;
use std::time::{Duration, Instant};

// =============================================================================
// Statistical Analysis
// =============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Statistics {
    #[serde(serialize_with = "serialize_duration")]
    min: Duration,
    #[serde(serialize_with = "serialize_duration")]
    max: Duration,
    #[serde(serialize_with = "serialize_duration")]
    mean: Duration,
    #[serde(serialize_with = "serialize_duration_option")]
    mode: Option<Duration>,
    #[serde(serialize_with = "serialize_duration")]
    stddev: Duration,
    #[serde(serialize_with = "serialize_duration")]
    p50: Duration,
    #[serde(serialize_with = "serialize_duration")]
    p95: Duration,
    #[serde(serialize_with = "serialize_duration")]
    p99: Duration,
    total_samples: usize,
    #[serde(serialize_with = "serialize_duration")]
    total_duration: Duration,
    throughput: f64, // operations per second
    sample_rate: f64, // samples per second
    outliers: OutlierInfo,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct OutlierInfo {
    total_outliers: usize,
    low_mild: usize,
    low_severe: usize,
    high_mild: usize,
    high_severe: usize,
    outlier_percentage: f64,
}

// Serialize Duration as microseconds (f64)
fn serialize_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64(duration.as_secs_f64() * 1_000_000.0)
}

fn serialize_duration_option<S>(
    duration: &Option<Duration>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match duration {
        Some(d) => serializer.serialize_some(&(d.as_secs_f64() * 1_000_000.0)),
        None => serializer.serialize_none(),
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
struct JitStatistics {
    cache_hits: usize,
    cache_misses: usize,
    cache_hit_rate: f64,
    unique_policies: usize,
    total_compilations: usize,
}

impl Statistics {
    fn from_samples(mut samples: Vec<Duration>, test_duration: Duration) -> Self {
        assert!(!samples.is_empty(), "Cannot compute statistics on empty samples");

        samples.sort();

        let total_samples = samples.len();
        let min = *samples.first().unwrap();
        let max = *samples.last().unwrap();

        // Calculate mean
        let sum_nanos: u128 = samples.iter().map(|d| d.as_nanos()).sum();
        let mean_nanos = sum_nanos / total_samples as u128;
        let mean = Duration::from_nanos(mean_nanos as u64);

        // Calculate standard deviation
        let variance: f64 = samples
            .iter()
            .map(|d| {
                let diff = d.as_nanos() as f64 - mean_nanos as f64;
                diff * diff
            })
            .sum::<f64>()
            / total_samples as f64;
        let stddev = Duration::from_nanos(variance.sqrt() as u64);

        // Calculate percentiles
        let p25 = samples[total_samples * 25 / 100];
        let p50 = samples[total_samples * 50 / 100];
        let p75 = samples[total_samples * 75 / 100];
        let p95 = samples[total_samples * 95 / 100];
        let p99 = samples[total_samples * 99 / 100];

        // Calculate mode (most common duration, grouped by microsecond)
        let mode = calculate_mode(&samples);

        // Calculate outliers using IQR (Interquartile Range) method
        let outliers = detect_outliers(&samples, p25, p75);

        // Calculate throughput and sample rate
        let throughput = total_samples as f64 / test_duration.as_secs_f64();
        let sample_rate = throughput; // Same as throughput for our use case

        Statistics {
            min,
            max,
            mean,
            mode,
            stddev,
            p50,
            p95,
            p99,
            total_samples,
            total_duration: test_duration,
            throughput,
            sample_rate,
            outliers,
        }
    }

    fn print(&self, test_name: &str) {
        println!("\n{}", "=".repeat(80));
        println!("Performance Test: {}", test_name);
        println!("{}", "=".repeat(80));
        println!("Total samples:    {}", self.total_samples);
        println!("Test duration:    {:.2}s", self.total_duration.as_secs_f64());
        println!("Throughput:       {:.0} ops/sec", self.throughput);
        println!("Sample rate:      {:.0} samples/sec", self.sample_rate);
        println!();
        println!("Latency Statistics:");
        println!("  Min:            {:>10.3} µs", self.min.as_secs_f64() * 1_000_000.0);
        println!("  Max:            {:>10.3} µs", self.max.as_secs_f64() * 1_000_000.0);
        println!("  Mean:           {:>10.3} µs", self.mean.as_secs_f64() * 1_000_000.0);
        if let Some(mode) = self.mode {
            println!("  Mode:           {:>10.3} µs", mode.as_secs_f64() * 1_000_000.0);
        }
        println!("  Std Dev:        {:>10.3} µs", self.stddev.as_secs_f64() * 1_000_000.0);
        println!();
        println!("Percentiles:");
        println!("  p50 (median):   {:>10.3} µs", self.p50.as_secs_f64() * 1_000_000.0);
        println!("  p95:            {:>10.3} µs", self.p95.as_secs_f64() * 1_000_000.0);
        println!("  p99:            {:>10.3} µs", self.p99.as_secs_f64() * 1_000_000.0);

        // Print outlier information
        if self.outliers.total_outliers > 0 {
            println!();
            println!("Outliers: Found {} outliers among {} measurements ({:.2}%)",
                self.outliers.total_outliers,
                self.total_samples,
                self.outliers.outlier_percentage
            );
            if self.outliers.low_severe > 0 {
                println!("  {} ({:.2}%) low severe",
                    self.outliers.low_severe,
                    (self.outliers.low_severe as f64 / self.total_samples as f64) * 100.0
                );
            }
            if self.outliers.low_mild > 0 {
                println!("  {} ({:.2}%) low mild",
                    self.outliers.low_mild,
                    (self.outliers.low_mild as f64 / self.total_samples as f64) * 100.0
                );
            }
            if self.outliers.high_mild > 0 {
                println!("  {} ({:.2}%) high mild",
                    self.outliers.high_mild,
                    (self.outliers.high_mild as f64 / self.total_samples as f64) * 100.0
                );
            }
            if self.outliers.high_severe > 0 {
                println!("  {} ({:.2}%) high severe",
                    self.outliers.high_severe,
                    (self.outliers.high_severe as f64 / self.total_samples as f64) * 100.0
                );
            }
        }

        println!("{}", "=".repeat(80));
    }
}

impl JitStatistics {
    fn new(unique_policies: usize) -> Self {
        Self {
            cache_hits: 0,
            cache_misses: 0,
            cache_hit_rate: 0.0,
            unique_policies,
            total_compilations: 0,
        }
    }

    fn record_compilation(&mut self) {
        self.cache_misses += 1;
        self.total_compilations += 1;
    }

    fn record_hit(&mut self) {
        self.cache_hits += 1;
    }

    fn finalize(&mut self) {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hit_rate = (self.cache_hits as f64 / total as f64) * 100.0;
        }
    }

    fn print(&self) {
        println!();
        println!("JIT Cache Statistics:");
        println!("  Unique policies:    {}", self.unique_policies);
        println!("  Total compilations: {}", self.total_compilations);
        println!("  Cache hits:         {}", self.cache_hits);
        println!("  Cache misses:       {}", self.cache_misses);
        println!("  Cache hit rate:     {:.2}%", self.cache_hit_rate);
    }
}

/// Calculate mode (most common duration, bucketed by microsecond)
fn calculate_mode(samples: &[Duration]) -> Option<Duration> {
    use std::collections::HashMap;

    if samples.is_empty() {
        return None;
    }

    let mut frequency_map: HashMap<u64, usize> = HashMap::new();

    // Bucket by microsecond for reasonable grouping
    for &sample in samples {
        let micros = sample.as_micros() as u64;
        *frequency_map.entry(micros).or_insert(0) += 1;
    }

    // Find the most common bucket
    let max_freq = frequency_map.values().max()?;
    let mode_micros = frequency_map
        .iter()
        .find(|(_, &freq)| freq == *max_freq)
        .map(|(&micros, _)| micros)?;

    Some(Duration::from_micros(mode_micros))
}

/// Detect outliers using IQR (Interquartile Range) method
///
/// Outliers are classified as:
/// - Low severe: value < Q1 - 3*IQR
/// - Low mild: Q1 - 3*IQR <= value < Q1 - 1.5*IQR
/// - High mild: Q3 + 1.5*IQR < value <= Q3 + 3*IQR
/// - High severe: value > Q3 + 3*IQR
///
/// This is the same method used by criterion.rs for benchmark outlier detection.
fn detect_outliers(samples: &[Duration], q1: Duration, q3: Duration) -> OutlierInfo {
    let iqr_nanos = q3.as_nanos().saturating_sub(q1.as_nanos()) as f64;
    let q1_nanos = q1.as_nanos() as f64;
    let q3_nanos = q3.as_nanos() as f64;

    let low_severe_threshold = q1_nanos - 3.0 * iqr_nanos;
    let low_mild_threshold = q1_nanos - 1.5 * iqr_nanos;
    let high_mild_threshold = q3_nanos + 1.5 * iqr_nanos;
    let high_severe_threshold = q3_nanos + 3.0 * iqr_nanos;

    let mut low_severe = 0;
    let mut low_mild = 0;
    let mut high_mild = 0;
    let mut high_severe = 0;

    for &sample in samples {
        let sample_nanos = sample.as_nanos() as f64;

        if sample_nanos < low_severe_threshold {
            low_severe += 1;
        } else if sample_nanos < low_mild_threshold {
            low_mild += 1;
        } else if sample_nanos > high_severe_threshold {
            high_severe += 1;
        } else if sample_nanos > high_mild_threshold {
            high_mild += 1;
        }
    }

    let total_outliers = low_severe + low_mild + high_mild + high_severe;
    let outlier_percentage = if samples.is_empty() {
        0.0
    } else {
        (total_outliers as f64 / samples.len() as f64) * 100.0
    };

    OutlierInfo {
        total_outliers,
        low_mild,
        low_severe,
        high_mild,
        high_severe,
        outlier_percentage,
    }
}

// =============================================================================
// Unit Tests for Statistics
// =============================================================================

#[cfg(test)]
mod stats_tests {
    use super::*;

    #[test]
    fn test_statistics_basic() {
        let samples = vec![
            Duration::from_micros(5),
            Duration::from_micros(10),
            Duration::from_micros(15),
            Duration::from_micros(20),
            Duration::from_micros(25),
        ];

        let stats = Statistics::from_samples(samples, Duration::from_secs(1));

        assert_eq!(stats.min, Duration::from_micros(5));
        assert_eq!(stats.max, Duration::from_micros(25));
        assert_eq!(stats.total_samples, 5);
        assert_eq!(stats.p50, Duration::from_micros(15)); // Middle value
    }

    #[test]
    fn test_calculate_mode() {
        let samples = vec![
            Duration::from_micros(10),
            Duration::from_micros(10),
            Duration::from_micros(10),
            Duration::from_micros(20),
            Duration::from_micros(30),
        ];

        let mode = calculate_mode(&samples);
        assert_eq!(mode, Some(Duration::from_micros(10)));
    }
}

// =============================================================================
// Predicate Generators
// =============================================================================

#[allow(dead_code)]
enum PredicateComplexity {
    Simple,      // 1-2 comparisons
    Medium,      // 3-5 comparisons
    Complex,     // 6-10 comparisons
    VeryComplex, // 10-20 comparisons
}

struct PredicateGenerator {
    rng: StdRng,
}

impl PredicateGenerator {
    fn new(seed: u64) -> Self {
        Self { rng: StdRng::seed_from_u64(seed) }
    }

    /// Generate a uniform random predicate with varying complexity
    fn generate_uniform_random(&mut self, complexity: PredicateComplexity) -> CompiledPolicy {
        let num_comparisons = match complexity {
            PredicateComplexity::Simple => self.rng.gen_range(1..=2),
            PredicateComplexity::Medium => self.rng.gen_range(3..=5),
            PredicateComplexity::Complex => self.rng.gen_range(6..=10),
            PredicateComplexity::VeryComplex => self.rng.gen_range(10..=20),
        };

        let mut policy = CompiledPolicy::new(self.rng.gen());

        // Generate random comparisons connected with AND/OR
        for i in 0..num_comparisons {
            // Generate random field offset and constant
            let field_offset = self.rng.gen_range(0..10);
            let const_value = self.rng.gen_range(0..100);

            let const_idx = policy.add_constant(Value::Int(const_value));

            policy.emit(Instruction::LoadField { offset: field_offset });
            policy.emit(Instruction::LoadConst { idx: const_idx });

            // Random comparison operator
            let op = match self.rng.gen_range(0..6) {
                0 => CompOp::Eq,
                1 => CompOp::Neq,
                2 => CompOp::Lt,
                3 => CompOp::Lte,
                4 => CompOp::Gt,
                _ => CompOp::Gte,
            };
            policy.emit(Instruction::Compare { op });

            // Connect with AND/OR if not first comparison
            if i > 0 {
                if self.rng.gen_bool(0.5) {
                    policy.emit(Instruction::And);
                } else {
                    policy.emit(Instruction::Or);
                }
            }
        }

        // Conditional return based on result
        policy.emit(Instruction::JumpIfFalse { offset: 2 });
        policy.emit(Instruction::Return { value: true });
        policy.emit(Instruction::Return { value: false });

        policy
    }

    /// Generate a cache-heavy predicate (repeating pattern)
    fn generate_cache_heavy(&mut self, pattern_size: usize) -> Vec<CompiledPolicy> {
        let mut policies = Vec::new();

        // Generate a small set of predicates that will be reused
        for i in 0..pattern_size {
            let mut policy = CompiledPolicy::new(i as u64);

            // Simple predicate: field_0 == (i * 10)
            let const_idx = policy.add_constant(Value::Int((i * 10) as i64));
            policy.emit(Instruction::LoadField { offset: 0 });
            policy.emit(Instruction::LoadConst { idx: const_idx });
            policy.emit(Instruction::Compare { op: CompOp::Eq });
            policy.emit(Instruction::JumpIfFalse { offset: 2 });
            policy.emit(Instruction::Return { value: true });
            policy.emit(Instruction::Return { value: false });

            policies.push(policy);
        }

        policies
    }

    /// Generate mixed workload: combination of simple and complex predicates
    fn generate_mixed_workload(&mut self, total: usize) -> Vec<CompiledPolicy> {
        let mut policies = Vec::new();

        for i in 0..total {
            // 60% simple, 30% medium, 10% complex
            let complexity = match i % 10 {
                0..=5 => PredicateComplexity::Simple,
                6..=8 => PredicateComplexity::Medium,
                _ => PredicateComplexity::Complex,
            };
            policies.push(self.generate_uniform_random(complexity));
        }

        policies
    }

    /// Generate bytecode stress test with deep nesting and many operations
    fn generate_bytecode_stress(&mut self) -> CompiledPolicy {
        let mut policy = CompiledPolicy::new(self.rng.gen());

        // Create a deeply nested expression tree
        // ((a < b) AND (c > d)) OR ((e == f) AND (g != h))
        for level in 0..5 {
            let field_a = self.rng.gen_range(0..10);
            let field_b = self.rng.gen_range(0..10);
            let val_a = self.rng.gen_range(0..100);
            let val_b = self.rng.gen_range(0..100);

            let const_a = policy.add_constant(Value::Int(val_a));
            let const_b = policy.add_constant(Value::Int(val_b));

            // First comparison
            policy.emit(Instruction::LoadField { offset: field_a });
            policy.emit(Instruction::LoadConst { idx: const_a });
            policy.emit(Instruction::Compare { op: CompOp::Lt });

            // Second comparison
            policy.emit(Instruction::LoadField { offset: field_b });
            policy.emit(Instruction::LoadConst { idx: const_b });
            policy.emit(Instruction::Compare { op: CompOp::Gt });

            // AND them together
            policy.emit(Instruction::And);

            // OR with previous level if not first
            if level > 0 {
                policy.emit(Instruction::Or);
            }
        }

        // Final return based on result
        policy.emit(Instruction::JumpIfFalse { offset: 2 });
        policy.emit(Instruction::Return { value: true });
        policy.emit(Instruction::Return { value: false });

        policy
    }

    /// Generate workload with jump-heavy bytecode (tests branch prediction)
    fn generate_jump_heavy(&mut self) -> CompiledPolicy {
        let mut policy = CompiledPolicy::new(self.rng.gen());

        // Create a series of conditional branches
        for i in 0..8 {
            let field = i % 10;
            let value = self.rng.gen_range(0..50);
            let const_idx = policy.add_constant(Value::Int(value));

            policy.emit(Instruction::LoadField { offset: field });
            policy.emit(Instruction::LoadConst { idx: const_idx });
            policy.emit(Instruction::Compare {
                op: if i % 2 == 0 { CompOp::Lt } else { CompOp::Gt },
            });

            // Jump forward if false (skip the return)
            policy.emit(Instruction::JumpIfFalse { offset: 2 });
            // Early return if condition met
            policy.emit(Instruction::Return { value: true });
        }

        // Default deny if no condition matched
        policy.emit(Instruction::Return { value: false });

        policy
    }

    /// Generate policies with logarithmic size distribution to reach target total size
    ///
    /// Creates a realistic distribution where:
    /// - 70% are small policies (1-5 comparisons)
    /// - 20% are medium policies (6-15 comparisons)
    /// - 8% are large policies (16-50 comparisons)
    /// - 2% are very large policies (51-200 comparisons)
    ///
    /// Returns (policies, total_bytes_estimate)
    fn generate_logarithmic_distribution(&mut self, target_bytes: usize) -> (Vec<CompiledPolicy>, usize) {
        let mut policies = Vec::new();
        let mut total_bytes = 0;

        // Rough estimate: each comparison is about 25 bytes (3-4 instructions + constant)
        const BYTES_PER_COMPARISON: usize = 25;
        const POLICY_OVERHEAD: usize = 50; // Base policy overhead

        while total_bytes < target_bytes {
            // Generate policy size using logarithmic distribution
            let rand_val = self.rng.gen::<f64>();
            let num_comparisons = if rand_val < 0.70 {
                // 70% small: 1-5 comparisons
                self.rng.gen_range(1..=5)
            } else if rand_val < 0.90 {
                // 20% medium: 6-15 comparisons
                self.rng.gen_range(6..=15)
            } else if rand_val < 0.98 {
                // 8% large: 16-50 comparisons
                self.rng.gen_range(16..=50)
            } else {
                // 2% very large: 51-200 comparisons
                self.rng.gen_range(51..=200)
            };

            let policy = self.generate_policy_with_comparisons(num_comparisons);
            let policy_bytes = POLICY_OVERHEAD + (num_comparisons * BYTES_PER_COMPARISON);
            total_bytes += policy_bytes;
            policies.push(policy);

            // Progress indicator every 10MB
            if policies.len() % 10000 == 0 {
                let mb = total_bytes as f64 / (1024.0 * 1024.0);
                println!("  Generated {} policies, ~{:.1} MB", policies.len(), mb);
            }
        }

        (policies, total_bytes)
    }

    /// Generate a policy with a specific number of comparisons
    fn generate_policy_with_comparisons(&mut self, num_comparisons: usize) -> CompiledPolicy {
        let mut policy = CompiledPolicy::new(self.rng.gen());

        for i in 0..num_comparisons {
            let field_offset = self.rng.gen_range(0..10);
            let const_value = self.rng.gen_range(0..100);
            let const_idx = policy.add_constant(Value::Int(const_value));

            policy.emit(Instruction::LoadField { offset: field_offset });
            policy.emit(Instruction::LoadConst { idx: const_idx });

            let op = match self.rng.gen_range(0..6) {
                0 => CompOp::Eq,
                1 => CompOp::Neq,
                2 => CompOp::Lt,
                3 => CompOp::Lte,
                4 => CompOp::Gt,
                _ => CompOp::Gte,
            };
            policy.emit(Instruction::Compare { op });

            if i > 0 {
                if self.rng.gen_bool(0.5) {
                    policy.emit(Instruction::And);
                } else {
                    policy.emit(Instruction::Or);
                }
            }
        }

        policy.emit(Instruction::JumpIfFalse { offset: 2 });
        policy.emit(Instruction::Return { value: true });
        policy.emit(Instruction::Return { value: false });

        policy
    }
}

/// Generate field mapping for test contexts
fn create_field_mapping() -> FieldMapping {
    let mut field_map = FieldMapping::new();

    for i in 0..10 {
        field_map.insert(i, vec!["resource".to_string(), format!("field_{}", i)]);
    }

    field_map
}

/// Generate test contexts with varying field values
fn create_test_contexts(count: usize, seed: u64) -> Vec<EvaluationContext> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut contexts = Vec::with_capacity(count);

    for _ in 0..count {
        let mut ctx = EvaluationContext::default();
        ctx.resource.type_id = ResourceTypeId(1);

        // Add random field values
        for i in 0..10 {
            let value = rng.gen_range(0..100);
            ctx.resource
                .attributes
                .insert(format!("field_{}", i), AttributeValue::Int(value));
        }

        contexts.push(ctx);
    }

    contexts
}

// =============================================================================
// Test Runners
// =============================================================================

/// Run performance test with interpreter
fn run_interpreter_test(
    name: &str,
    policies: &[CompiledPolicy],
    contexts: &[EvaluationContext],
    field_map: &FieldMapping,
    duration: Duration,
) -> Statistics {
    let mut samples = Vec::new();
    let test_start = Instant::now();
    let mut policy_idx = 0;
    let mut context_idx = 0;

    println!("\nRunning interpreter test: {}", name);
    println!("Warming up...");

    // Warm-up phase (1 second)
    let warmup_end = Instant::now() + Duration::from_secs(1);
    while Instant::now() < warmup_end {
        let policy = &policies[policy_idx % policies.len()];
        let ctx = &contexts[context_idx % contexts.len()];
        let mut interp = Interpreter::new(field_map.clone());

        let _ = interp.evaluate(policy, ctx);

        policy_idx += 1;
        context_idx += 1;
    }

    println!("Starting measurement phase...");

    // Measurement phase
    let test_end = Instant::now() + duration;
    policy_idx = 0;
    context_idx = 0;

    while Instant::now() < test_end {
        let policy = &policies[policy_idx % policies.len()];
        let ctx = &contexts[context_idx % contexts.len()];
        let mut interp = Interpreter::new(field_map.clone());

        let start = Instant::now();
        let _ = interp.evaluate(policy, ctx);
        let elapsed = start.elapsed();

        samples.push(elapsed);

        policy_idx += 1;
        context_idx += 1;
    }

    let actual_duration = test_start.elapsed();
    Statistics::from_samples(samples, actual_duration)
}

/// Run performance test with JIT
#[cfg(all(feature = "jit", not(miri)))] // Skip JIT tests under Miri
fn run_jit_test(
    name: &str,
    policies: &[CompiledPolicy],
    contexts: &[EvaluationContext],
    duration: Duration,
) -> (Statistics, JitStatistics) {
    let mut samples = Vec::new();
    let test_start = Instant::now();
    let mut policy_idx = 0;
    let mut context_idx = 0;
    let mut jit_stats = JitStatistics::new(policies.len());

    println!("\nRunning JIT test: {}", name);
    println!("Compiling {} policies...", policies.len());

    // Compile all policies
    let mut compiler = JitCompiler::new().expect("Failed to create JIT compiler");
    let mut jit_codes = Vec::new();

    for (i, policy) in policies.iter().enumerate() {
        let code = compiler
            .compile(policy, &format!("policy_{}", i))
            .expect("Failed to compile policy");
        jit_codes.push(code);
        jit_stats.record_compilation();
    }

    println!("Warming up...");

    // Warm-up phase (1 second)
    let warmup_end = Instant::now() + Duration::from_secs(1);
    while Instant::now() < warmup_end {
        let code = &jit_codes[policy_idx % jit_codes.len()];
        let ctx = &contexts[context_idx % contexts.len()];

        let _ = unsafe { code.execute(ctx as *const _) };

        policy_idx += 1;
        context_idx += 1;
    }

    println!("Starting measurement phase...");

    // Measurement phase
    let test_end = Instant::now() + duration;
    policy_idx = 0;
    context_idx = 0;

    while Instant::now() < test_end {
        let code = &jit_codes[policy_idx % jit_codes.len()];
        let ctx = &contexts[context_idx % contexts.len()];

        // Track cache hit (reusing compiled code)
        if policy_idx > 0 {
            jit_stats.record_hit();
        }

        let start = Instant::now();
        let _ = unsafe { code.execute(ctx as *const _) };
        let elapsed = start.elapsed();

        samples.push(elapsed);

        policy_idx += 1;
        context_idx += 1;
    }

    jit_stats.finalize();

    let actual_duration = test_start.elapsed();
    (Statistics::from_samples(samples, actual_duration), jit_stats)
}

// =============================================================================
// Performance Tests
// =============================================================================

#[test]
#[ignore] // Run with: cargo test --release -- --ignored --nocapture --test-threads=1
fn perftest_interpreter_uniform_random_simple() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..100)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::Simple))
        .collect();
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Uniform Random (Simple)",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Uniform Random (Simple)");
}

#[test]
#[ignore]
fn perftest_interpreter_uniform_random_medium() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..100)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::Medium))
        .collect();
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Uniform Random (Medium)",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Uniform Random (Medium)");
}

#[test]
#[ignore]
fn perftest_interpreter_uniform_random_complex() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..100)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::Complex))
        .collect();
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Uniform Random (Complex)",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Uniform Random (Complex)");
}

#[test]
#[ignore]
fn perftest_interpreter_cache_heavy() {
    let mut gen = PredicateGenerator::new(12345);
    let policies = gen.generate_cache_heavy(10); // Only 10 unique predicates
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Cache Heavy",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Cache Heavy (10 predicates)");
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_uniform_random_simple() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..100)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::Simple))
        .collect();
    let contexts = create_test_contexts(100, 54321);

    let (stats, jit_stats) = run_jit_test(
        "JIT - Uniform Random (Simple)",
        &policies,
        &contexts,
        Duration::from_secs(10),
    );

    stats.print("JIT - Uniform Random (Simple)");
    jit_stats.print();
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_uniform_random_medium() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..100)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::Medium))
        .collect();
    let contexts = create_test_contexts(100, 54321);

    let (stats, jit_stats) = run_jit_test(
        "JIT - Uniform Random (Medium)",
        &policies,
        &contexts,
        Duration::from_secs(10),
    );

    stats.print("JIT - Uniform Random (Medium)");
    jit_stats.print();
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_uniform_random_complex() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..100)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::Complex))
        .collect();
    let contexts = create_test_contexts(100, 54321);

    let (stats, jit_stats) = run_jit_test(
        "JIT - Uniform Random (Complex)",
        &policies,
        &contexts,
        Duration::from_secs(10),
    );

    stats.print("JIT - Uniform Random (Complex)");
    jit_stats.print();
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_cache_heavy() {
    let mut gen = PredicateGenerator::new(12345);
    let policies = gen.generate_cache_heavy(10); // Only 10 unique predicates
    let contexts = create_test_contexts(100, 54321);

    let (stats, jit_stats) =
        run_jit_test("JIT - Cache Heavy", &policies, &contexts, Duration::from_secs(10));

    stats.print("JIT - Cache Heavy (10 predicates)");
    jit_stats.print();
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_vs_interpreter_comparison() {
    println!("\n{}", "=".repeat(80));
    println!("JIT vs Interpreter Comparison");
    println!("{}", "=".repeat(80));

    // Test with cache-heavy workload (best case for JIT)
    let mut gen = PredicateGenerator::new(12345);
    let policies = gen.generate_cache_heavy(10);
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let interp_stats = run_interpreter_test(
        "Interpreter",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    let (jit_stats, jit_cache_stats) =
        run_jit_test("JIT", &policies, &contexts, Duration::from_secs(10));

    println!("\n{}", "=".repeat(80));
    println!("Comparison Results:");
    println!("{}", "=".repeat(80));
    println!("Interpreter throughput: {:.0} ops/sec", interp_stats.throughput);
    println!("JIT throughput:         {:.0} ops/sec", jit_stats.throughput);
    println!(
        "JIT speedup:            {:.2}x",
        jit_stats.throughput / interp_stats.throughput
    );
    println!();
    println!(
        "Interpreter p50:        {:.3} µs",
        interp_stats.p50.as_secs_f64() * 1_000_000.0
    );
    println!("JIT p50:                {:.3} µs", jit_stats.p50.as_secs_f64() * 1_000_000.0);
    println!();
    println!(
        "Interpreter p99:        {:.3} µs",
        interp_stats.p99.as_secs_f64() * 1_000_000.0
    );
    println!("JIT p99:                {:.3} µs", jit_stats.p99.as_secs_f64() * 1_000_000.0);
    jit_cache_stats.print();
    println!("{}", "=".repeat(80));
}

// =============================================================================
// Mixed Workload and Stress Tests
// =============================================================================

#[test]
#[ignore]
fn perftest_interpreter_mixed_workload() {
    let mut gen = PredicateGenerator::new(12345);
    let policies = gen.generate_mixed_workload(100);
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Mixed Workload",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Mixed Workload (60% simple, 30% medium, 10% complex)");
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_mixed_workload() {
    let mut gen = PredicateGenerator::new(12345);
    let policies = gen.generate_mixed_workload(100);
    let contexts = create_test_contexts(100, 54321);

    let (stats, jit_stats) = run_jit_test(
        "JIT - Mixed Workload",
        &policies,
        &contexts,
        Duration::from_secs(10),
    );

    stats.print("JIT - Mixed Workload (60% simple, 30% medium, 10% complex)");
    jit_stats.print();
}

#[test]
#[ignore]
fn perftest_interpreter_bytecode_stress() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..50).map(|_| gen.generate_bytecode_stress()).collect();
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Bytecode Stress",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Bytecode Stress (deep nesting, many operations)");
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_bytecode_stress() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..50).map(|_| gen.generate_bytecode_stress()).collect();
    let contexts = create_test_contexts(100, 54321);

    let (stats, jit_stats) = run_jit_test(
        "JIT - Bytecode Stress",
        &policies,
        &contexts,
        Duration::from_secs(10),
    );

    stats.print("JIT - Bytecode Stress (deep nesting, many operations)");
    jit_stats.print();
}

#[test]
#[ignore]
fn perftest_interpreter_jump_heavy() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..50).map(|_| gen.generate_jump_heavy()).collect();
    let contexts = create_test_contexts(100, 54321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Jump Heavy",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Jump Heavy (branch prediction stress)");
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_jump_heavy() {
    let mut gen = PredicateGenerator::new(12345);
    let policies: Vec<_> = (0..50).map(|_| gen.generate_jump_heavy()).collect();
    let contexts = create_test_contexts(100, 54321);

    let (stats, jit_stats) = run_jit_test(
        "JIT - Jump Heavy",
        &policies,
        &contexts,
        Duration::from_secs(10),
    );

    stats.print("JIT - Jump Heavy (branch prediction stress)");
    jit_stats.print();
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_cache_hit_rate_comparison() {
    println!("\n{}", "=".repeat(80));
    println!("JIT Cache Hit Rate Comparison Across Workload Types");
    println!("{}", "=".repeat(80));

    let mut gen = PredicateGenerator::new(12345);
    let contexts = create_test_contexts(100, 54321);

    // Test 1: Highly cacheable (10 unique policies)
    println!("\n--- Test 1: Highly Cacheable (10 unique policies) ---");
    let policies_10 = gen.generate_cache_heavy(10);
    let (stats_10, jit_stats_10) =
        run_jit_test("High Cache", &policies_10, &contexts, Duration::from_secs(10));
    println!("Throughput: {:.0} ops/sec", stats_10.throughput);
    println!("p99 latency: {:.3} µs", stats_10.p99.as_secs_f64() * 1_000_000.0);
    jit_stats_10.print();

    // Test 2: Moderately cacheable (50 unique policies)
    println!("\n--- Test 2: Moderately Cacheable (50 unique policies) ---");
    let policies_50 = gen.generate_cache_heavy(50);
    let (stats_50, jit_stats_50) =
        run_jit_test("Medium Cache", &policies_50, &contexts, Duration::from_secs(10));
    println!("Throughput: {:.0} ops/sec", stats_50.throughput);
    println!("p99 latency: {:.3} µs", stats_50.p99.as_secs_f64() * 1_000_000.0);
    jit_stats_50.print();

    // Test 3: Low cacheability (100 diverse policies)
    println!("\n--- Test 3: Low Cacheability (100 diverse policies) ---");
    let policies_100: Vec<_> = (0..100)
        .map(|_| gen.generate_uniform_random(PredicateComplexity::Medium))
        .collect();
    let (stats_100, jit_stats_100) =
        run_jit_test("Low Cache", &policies_100, &contexts, Duration::from_secs(10));
    println!("Throughput: {:.0} ops/sec", stats_100.throughput);
    println!("p99 latency: {:.3} µs", stats_100.p99.as_secs_f64() * 1_000_000.0);
    jit_stats_100.print();

    println!("\n{}", "=".repeat(80));
    println!("Summary:");
    println!("{}", "=".repeat(80));
    println!(
        "Highly cacheable (10):    {:.2}% hit rate, {:.0} ops/sec",
        jit_stats_10.cache_hit_rate, stats_10.throughput
    );
    println!(
        "Moderately cacheable (50): {:.2}% hit rate, {:.0} ops/sec",
        jit_stats_50.cache_hit_rate, stats_50.throughput
    );
    println!(
        "Low cacheability (100):    {:.2}% hit rate, {:.0} ops/sec",
        jit_stats_100.cache_hit_rate, stats_100.throughput
    );
    println!("{}", "=".repeat(80));
}

// =============================================================================
// Logarithmic Distribution Tests - 100MB Policy Set
// =============================================================================

#[test]
#[ignore]
fn perftest_interpreter_logarithmic_100mb() {
    let mut gen = PredicateGenerator::new(123456789);
    println!("\nGenerating ~100MB of policies with logarithmic size distribution...");

    let target_bytes = 100 * 1024 * 1024; // 100 MB
    let (policies, actual_bytes) = gen.generate_logarithmic_distribution(target_bytes);

    println!(
        "Generated {} policies totaling ~{:.1} MB",
        policies.len(),
        actual_bytes as f64 / (1024.0 * 1024.0)
    );

    let contexts = create_test_contexts(200, 987654321);
    let field_map = create_field_mapping();

    let stats = run_interpreter_test(
        "Interpreter - Logarithmic Distribution (100MB)",
        &policies,
        &contexts,
        &field_map,
        Duration::from_secs(10),
    );

    stats.print("Interpreter - Logarithmic Distribution (100MB)");
}

#[test]
#[ignore]
#[cfg(all(feature = "jit", not(miri)))]
fn perftest_jit_logarithmic_100mb() {
    let mut gen = PredicateGenerator::new(123456789);
    println!("\nGenerating ~100MB of policies with logarithmic size distribution...");

    let target_bytes = 100 * 1024 * 1024; // 100 MB
    let (policies, actual_bytes) = gen.generate_logarithmic_distribution(target_bytes);

    println!(
        "Generated {} policies totaling ~{:.1} MB",
        policies.len(),
        actual_bytes as f64 / (1024.0 * 1024.0)
    );

    let contexts = create_test_contexts(200, 987654321);

    let (stats, jit_stats) = run_jit_test(
        "JIT - Logarithmic Distribution (100MB)",
        &policies,
        &contexts,
        Duration::from_secs(10),
    );

    stats.print("JIT - Logarithmic Distribution (100MB)");
    jit_stats.print();
}
