// Example: JIT Compilation and Tiering
//
// This example demonstrates how the Idempotent Predicate Engine uses adaptive
// JIT compilation to optimize hot predicates at runtime.

use ipe_core::bytecode::{CompiledPolicy, Instruction, Value, CompOp};
use ipe_core::tiering::{TieredPolicy, TieredPolicyManager};
use ipe_core::rar::{EvaluationContext, Resource, Action, Request, Principal, AttributeValue, ResourceTypeId, Operation};
use std::time::Instant;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    println!("=== Idempotent Predicate Engine: JIT Compilation Demo ===\n");

    // Create a sample predicate bytecode
    // This predicate checks if resource.risk_level >= 3
    let mut policy = CompiledPolicy::new(1);
    
    // Load resource.risk_level field (assume offset 16)
    policy.emit(Instruction::LoadField { offset: 16 });
    
    // Load constant 3
    let const_idx = policy.add_constant(Value::Int(3));
    policy.emit(Instruction::LoadConst { idx: const_idx });
    
    // Compare: risk_level >= 3
    policy.emit(Instruction::Compare { op: CompOp::Gte });
    
    // Return the comparison result
    policy.emit(Instruction::Return { value: true });
    
    println!("Created predicate bytecode with {} instructions\n", policy.code.len());
    
    // Create tiered policy manager
    let manager = TieredPolicyManager::new()?;
    let tiered_policy = manager.create_policy(policy, "HighRiskPolicy".to_string());
    
    // Create evaluation context
    let mut ctx = EvaluationContext::default();
    ctx.resource = Resource {
        type_id: ResourceTypeId(1),
        attributes: {
            let mut attrs = HashMap::new();
            attrs.insert("type".to_string(), AttributeValue::String("Deployment".to_string()));
            attrs.insert("risk_level".to_string(), AttributeValue::Int(4));
            attrs
        },
    };
    ctx.action = Action {
        operation: Operation::Deploy,
        target: "production".to_string(),
    };
    ctx.request = Request {
        principal: Principal {
            id: "user:alice@example.com".to_string(),
            roles: vec!["developer".to_string()],
            attributes: HashMap::new(),
        },
        timestamp: chrono::Utc::now().timestamp(),
        source_ip: Some("10.0.1.42".to_string()),
        metadata: HashMap::new(),
    };
    
    // Phase 1: Interpreter (cold start)
    println!("Phase 1: Interpreter Mode (Cold Start)");
    println!("----------------------------------------");
    
    let mut interpreter_times = Vec::new();
    for i in 0..50 {
        let start = Instant::now();
        let _ = tiered_policy.evaluate(&ctx)?;
        let elapsed = start.elapsed();
        interpreter_times.push(elapsed.as_nanos() as f64);
        
        if i % 10 == 0 {
            println!("  Eval {}: {:>8.2}μs", i, elapsed.as_micros());
        }
    }
    
    let avg_interpreter = interpreter_times.iter().sum::<f64>() / interpreter_times.len() as f64 / 1000.0;
    println!("  Average: {:.2}μs\n", avg_interpreter);
    
    // Phase 2: Trigger JIT compilation
    println!("Phase 2: Triggering JIT Compilation");
    println!("----------------------------------------");
    
    // Simulate 100+ evaluations to trigger JIT
    for i in 0..60 {
        let _ = tiered_policy.evaluate(&ctx)?;
        if i % 20 == 0 {
            let stats = &tiered_policy.stats;
            let tier = *stats.current_tier.read();
            println!("  Evals: {}, Tier: {:?}", stats.eval_count.load(std::sync::atomic::Ordering::Relaxed), tier);
        }
    }
    
    // Give JIT time to compile (async)
    println!("\n  Waiting for JIT compilation...");
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Phase 3: JIT execution
    #[cfg(feature = "jit")]
    {
        println!("\nPhase 3: JIT Mode (Hot Path)");
        println!("----------------------------------------");
        
        let mut jit_times = Vec::new();
        for i in 0..50 {
            let start = Instant::now();
            let _ = tiered_policy.evaluate(&ctx)?;
            let elapsed = start.elapsed();
            jit_times.push(elapsed.as_nanos() as f64);
            
            if i % 10 == 0 {
                println!("  Eval {}: {:>8.2}μs", i, elapsed.as_micros());
            }
        }
        
        let avg_jit = jit_times.iter().sum::<f64>() / jit_times.len() as f64 / 1000.0;
        println!("  Average: {:.2}μs\n", avg_jit);
        
        // Performance comparison
        println!("Performance Summary");
        println!("----------------------------------------");
        println!("  Interpreter: {:.2}μs", avg_interpreter);
        println!("  JIT:         {:.2}μs", avg_jit);
        println!("  Speedup:     {:.2}x", avg_interpreter / avg_jit);
        
        // Final stats
        let stats = &tiered_policy.stats;
        println!("\nPolicy Statistics");
        println!("----------------------------------------");
        println!("  Total evaluations: {}", stats.eval_count.load(std::sync::atomic::Ordering::Relaxed));
        println!("  Current tier:      {:?}", *stats.current_tier.read());
        println!("  Avg latency:       {:.2}μs", stats.avg_latency_ns() as f64 / 1000.0);
    }
    
    #[cfg(not(feature = "jit"))]
    {
        println!("\nJIT compilation is disabled (compile with --features jit)");
    }
    
    println!("\n✓ Demo complete!");
    
    Ok(())
}
