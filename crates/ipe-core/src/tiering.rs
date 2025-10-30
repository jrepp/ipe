use crate::bytecode::CompiledPolicy;
#[cfg(feature = "jit")]
use crate::jit::{JitCode, JitCompiler};
use crate::rar::EvaluationContext;
use crate::{Decision, Result};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Execution tier for a policy
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExecutionTier {
    /// Interpreter-only (default for all policies)
    Interpreter = 0,
    /// Baseline JIT with minimal optimizations
    BaselineJIT = 1,
    /// Optimized JIT with full optimizations
    OptimizedJIT = 2,
    /// Ahead-of-time compiled native code
    NativeAOT = 3,
}

/// Statistics for adaptive tiering decisions
#[derive(Debug)]
pub struct ProfileStats {
    /// Total number of evaluations
    pub eval_count: AtomicU64,
    /// Sum of evaluation latencies (nanoseconds)
    pub total_latency_ns: AtomicU64,
    /// Last time the policy was promoted
    pub last_promoted: RwLock<Instant>,
    /// Current tier
    pub current_tier: RwLock<ExecutionTier>,
}

impl ProfileStats {
    pub fn new() -> Self {
        Self {
            eval_count: AtomicU64::new(0),
            total_latency_ns: AtomicU64::new(0),
            last_promoted: RwLock::new(Instant::now()),
            current_tier: RwLock::new(ExecutionTier::Interpreter),
        }
    }

    pub fn record_evaluation(&self, latency: Duration) {
        self.eval_count.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ns.fetch_add(latency.as_nanos() as u64, Ordering::Relaxed);
    }

    pub fn avg_latency_ns(&self) -> u64 {
        let count = self.eval_count.load(Ordering::Relaxed);
        if count == 0 {
            return 0;
        }
        self.total_latency_ns.load(Ordering::Relaxed) / count
    }

    pub fn should_promote(&self) -> bool {
        let count = self.eval_count.load(Ordering::Relaxed);
        let avg_latency = self.avg_latency_ns();
        let tier = *self.current_tier.read();
        let time_since_promotion = self.last_promoted.read().elapsed();

        // Require some cooldown between promotions
        if time_since_promotion < Duration::from_secs(10) {
            return false;
        }

        match tier {
            ExecutionTier::Interpreter => {
                // Promote to baseline JIT after 100 evaluations
                count >= 100
            },
            ExecutionTier::BaselineJIT => {
                // Promote to optimized JIT after 10k evals AND avg latency > 20μs
                count >= 10_000 && avg_latency > 20_000
            },
            ExecutionTier::OptimizedJIT | ExecutionTier::NativeAOT => {
                // Already at top tier
                false
            },
        }
    }

    pub fn promote(&self) -> ExecutionTier {
        let mut tier = self.current_tier.write();
        *tier = match *tier {
            ExecutionTier::Interpreter => ExecutionTier::BaselineJIT,
            ExecutionTier::BaselineJIT => ExecutionTier::OptimizedJIT,
            t => t,
        };
        *self.last_promoted.write() = Instant::now();
        *tier
    }
}

impl Default for ProfileStats {
    fn default() -> Self {
        Self::new()
    }
}

/// A policy with adaptive tiering support
pub struct TieredPolicy {
    /// Policy bytecode (always available)
    pub bytecode: Arc<CompiledPolicy>,

    /// JIT-compiled native code (optional)
    #[cfg(feature = "jit")]
    pub jit_code: RwLock<Option<Arc<JitCode>>>,

    /// Profiling statistics
    pub stats: Arc<ProfileStats>,

    /// Policy name (for JIT compilation)
    pub name: String,
}

impl TieredPolicy {
    pub fn new(bytecode: CompiledPolicy, name: String) -> Self {
        Self {
            bytecode: Arc::new(bytecode),
            #[cfg(feature = "jit")]
            jit_code: RwLock::new(None),
            stats: Arc::new(ProfileStats::new()),
            name,
        }
    }

    /// Evaluate the policy, using JIT code if available
    pub fn evaluate(&self, ctx: &EvaluationContext) -> Result<Decision> {
        let start = Instant::now();

        // Try JIT path first
        #[cfg(feature = "jit")]
        {
            if let Some(ref jit) = *self.jit_code.read() {
                let result = unsafe { jit.execute(ctx as *const _) };
                let latency = start.elapsed();
                self.stats.record_evaluation(latency);
                return Ok(Decision::from_bool(result));
            }
        }

        // Fallback to interpreter
        let result = self.interpret(ctx)?;
        let latency = start.elapsed();
        self.stats.record_evaluation(latency);

        // Check if we should promote to JIT
        #[cfg(feature = "jit")]
        {
            if self.stats.should_promote() {
                // Trigger async JIT compilation
                self.trigger_jit_compilation();
            }
        }

        Ok(result)
    }

    /// Interpret the bytecode (slow path)
    fn interpret(&self, _ctx: &EvaluationContext) -> Result<Decision> {
        // TODO: Implement interpreter
        // For now, just return a dummy decision
        Ok(Decision {
            kind: crate::engine::DecisionKind::Allow,
            reason: None,
            matched_policies: vec![],
        })
    }

    /// Trigger JIT compilation in background
    #[cfg(feature = "jit")]
    fn trigger_jit_compilation(&self) {
        use std::thread;

        let bytecode = Arc::clone(&self.bytecode);
        let jit_code = Arc::new(RwLock::new(self.jit_code.read().clone()));
        let name = self.name.clone();
        let stats = Arc::clone(&self.stats);

        thread::spawn(move || {
            let mut compiler = match JitCompiler::new() {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to create JIT compiler: {}", e);
                    return;
                },
            };

            match compiler.compile(&bytecode, &name) {
                Ok(compiled) => {
                    *jit_code.write() = Some(compiled);
                    stats.promote();
                    tracing::info!("JIT compiled policy: {}", name);
                },
                Err(e) => {
                    tracing::error!("JIT compilation failed for {}: {}", name, e);
                },
            }
        });
    }
}

/// Manager for tiered policies
pub struct TieredPolicyManager {
    #[cfg(feature = "jit")]
    compiler: RwLock<JitCompiler>,
}

impl TieredPolicyManager {
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "jit")]
            compiler: RwLock::new(JitCompiler::new()?),
        })
    }

    /// Create a tiered policy from bytecode
    pub fn create_policy(&self, bytecode: CompiledPolicy, name: String) -> TieredPolicy {
        TieredPolicy::new(bytecode, name)
    }

    /// Synchronously compile a policy to JIT (for critical policies)
    #[cfg(feature = "jit")]
    pub fn compile_sync(&self, policy: &TieredPolicy) -> Result<()> {
        let compiled = self.compiler.write().compile(&policy.bytecode, &policy.name)?;
        *policy.jit_code.write() = Some(compiled);
        *policy.stats.current_tier.write() = ExecutionTier::BaselineJIT;
        Ok(())
    }

    /// Get statistics for all policies
    pub fn get_stats(&self) -> Vec<PolicyStats> {
        // TODO: Track all policies and return their stats
        vec![]
    }
}

impl Default for TieredPolicyManager {
    fn default() -> Self {
        Self::new().expect("Failed to create tiered policy manager")
    }
}

/// Public statistics for a policy
#[derive(Debug, Clone)]
pub struct PolicyStats {
    pub name: String,
    pub tier: ExecutionTier,
    pub eval_count: u64,
    pub avg_latency_ns: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_promotion_thresholds() {
        let stats = ProfileStats::new();

        // Should not promote immediately
        assert!(!stats.should_promote());

        // Simulate 100 evaluations
        for _ in 0..100 {
            stats.record_evaluation(Duration::from_micros(50));
        }

        // Wait for cooldown
        std::thread::sleep(Duration::from_millis(11000));

        // Should promote to baseline JIT
        assert!(stats.should_promote());
        stats.promote();
        assert_eq!(*stats.current_tier.read(), ExecutionTier::BaselineJIT);
    }

    #[test]
    fn test_avg_latency_calculation() {
        let stats = ProfileStats::new();

        stats.record_evaluation(Duration::from_micros(10));
        stats.record_evaluation(Duration::from_micros(20));
        stats.record_evaluation(Duration::from_micros(30));

        let avg = stats.avg_latency_ns();
        assert_eq!(avg, 20_000); // 20μs
    }

    #[test]
    fn test_avg_latency_with_no_evaluations() {
        let stats = ProfileStats::new();
        assert_eq!(stats.avg_latency_ns(), 0);
    }

    #[test]
    fn test_profile_stats_default() {
        let stats = ProfileStats::default();
        assert_eq!(stats.eval_count.load(Ordering::Relaxed), 0);
        assert_eq!(stats.total_latency_ns.load(Ordering::Relaxed), 0);
        assert_eq!(*stats.current_tier.read(), ExecutionTier::Interpreter);
    }

    #[test]
    fn test_promotion_to_baseline_jit() {
        let stats = ProfileStats::new();

        // Record 100 evaluations
        for _ in 0..100 {
            stats.record_evaluation(Duration::from_micros(50));
        }

        // Should not promote immediately (cooldown)
        assert!(!stats.should_promote());

        // Manually set last_promoted to past
        *stats.last_promoted.write() = Instant::now() - Duration::from_secs(11);

        // Should promote now
        assert!(stats.should_promote());
        let new_tier = stats.promote();
        assert_eq!(new_tier, ExecutionTier::BaselineJIT);
        assert_eq!(*stats.current_tier.read(), ExecutionTier::BaselineJIT);
    }

    #[test]
    fn test_promotion_to_optimized_jit() {
        let stats = ProfileStats::new();

        // Start at BaselineJIT
        *stats.current_tier.write() = ExecutionTier::BaselineJIT;
        *stats.last_promoted.write() = Instant::now() - Duration::from_secs(11);

        // Record 10k evaluations with high latency (>20μs)
        for _ in 0..10_000 {
            stats.record_evaluation(Duration::from_micros(25));
        }

        // Should promote to OptimizedJIT
        assert!(stats.should_promote());
        let new_tier = stats.promote();
        assert_eq!(new_tier, ExecutionTier::OptimizedJIT);
        assert_eq!(*stats.current_tier.read(), ExecutionTier::OptimizedJIT);
    }

    #[test]
    fn test_no_promotion_from_optimized_jit() {
        let stats = ProfileStats::new();

        // Start at OptimizedJIT
        *stats.current_tier.write() = ExecutionTier::OptimizedJIT;
        *stats.last_promoted.write() = Instant::now() - Duration::from_secs(11);

        // Record many evaluations
        for _ in 0..20_000 {
            stats.record_evaluation(Duration::from_micros(5));
        }

        // Should not promote further
        assert!(!stats.should_promote());
    }

    #[test]
    fn test_no_promotion_baseline_jit_low_latency() {
        let stats = ProfileStats::new();

        // Start at BaselineJIT
        *stats.current_tier.write() = ExecutionTier::BaselineJIT;
        *stats.last_promoted.write() = Instant::now() - Duration::from_secs(11);

        // Record 10k evaluations with LOW latency (<20μs)
        for _ in 0..10_000 {
            stats.record_evaluation(Duration::from_micros(10));
        }

        // Should NOT promote (latency too low)
        assert!(!stats.should_promote());
    }

    #[test]
    fn test_no_promotion_native_aot() {
        let stats = ProfileStats::new();

        // Start at NativeAOT
        *stats.current_tier.write() = ExecutionTier::NativeAOT;
        *stats.last_promoted.write() = Instant::now() - Duration::from_secs(11);

        // Record many evaluations
        for _ in 0..20_000 {
            stats.record_evaluation(Duration::from_micros(5));
        }

        // Should not promote further
        assert!(!stats.should_promote());
    }

    #[test]
    fn test_promote_stays_at_top_tier() {
        let stats = ProfileStats::new();

        // Start at OptimizedJIT
        *stats.current_tier.write() = ExecutionTier::OptimizedJIT;

        // Try to promote (should stay at OptimizedJIT)
        let new_tier = stats.promote();
        assert_eq!(new_tier, ExecutionTier::OptimizedJIT);
    }

    #[test]
    fn test_tiered_policy_creation() {
        use crate::testing::simple_policy;

        let bytecode = simple_policy(1, true);
        let policy = TieredPolicy::new(bytecode, "TestPolicy".to_string());

        assert_eq!(policy.name, "TestPolicy");
        assert_eq!(*policy.stats.current_tier.read(), ExecutionTier::Interpreter);
    }

    #[test]
    fn test_tiered_policy_evaluate() {
        use crate::rar::{EvaluationContext, ResourceTypeId};
        use crate::testing::simple_policy;

        let bytecode = simple_policy(1, true);
        let policy = TieredPolicy::new(bytecode, "TestPolicy".to_string());
        let mut ctx = EvaluationContext::default();
        ctx.resource.type_id = ResourceTypeId(1);

        // Evaluate the policy
        let result = policy.evaluate(&ctx);
        assert!(result.is_ok());

        // Stats should be updated
        assert_eq!(policy.stats.eval_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    #[cfg_attr(miri, ignore = "TieredPolicyManager creates JIT compiler not supported by Miri")]
    fn test_tiered_policy_manager_creation() {
        let manager = TieredPolicyManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    #[cfg_attr(miri, ignore = "TieredPolicyManager creates JIT compiler not supported by Miri")]
    fn test_tiered_policy_manager_default() {
        let manager = TieredPolicyManager::default();
        // Should not panic, just verify it was created
        let _ = manager;
    }

    #[test]
    #[cfg_attr(miri, ignore = "TieredPolicyManager creates JIT compiler not supported by Miri")]
    fn test_tiered_policy_manager_create_policy() {
        use crate::testing::simple_policy;

        let manager = TieredPolicyManager::new().unwrap();
        let bytecode = simple_policy(1, true);
        let policy = manager.create_policy(bytecode, "TestPolicy".to_string());

        assert_eq!(policy.name, "TestPolicy");
    }

    #[test]
    fn test_execution_tier_ordering() {
        // Test that tiers are ordered correctly
        assert!(ExecutionTier::Interpreter < ExecutionTier::BaselineJIT);
        assert!(ExecutionTier::BaselineJIT < ExecutionTier::OptimizedJIT);
        assert!(ExecutionTier::OptimizedJIT < ExecutionTier::NativeAOT);
    }
}
