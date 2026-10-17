//! Export parser trees and reproducible interpreter microbenchmarks for the local docs.
use ipe_core::{
    ast::{Condition, Expression, Requirements},
    bytecode::{CompOp, CompiledPolicy, Instruction, Value},
    interpreter::Interpreter,
    parser::Parser,
    rar::{AttributeValue, EvaluationContext},
};
use serde_json::{json, Value as Json};
use std::{collections::HashMap, hint::black_box, time::Instant};

fn node(kind: &str, label: impl ToString, children: Vec<Json>) -> Json {
    json!({"kind": kind, "label": label.to_string(), "children": children})
}

fn expression(expr: &Expression) -> Json {
    match expr {
        Expression::Literal(value) => node("Literal", format!("{value:?}"), vec![]),
        Expression::Path(path) => node("Path", path, vec![]),
        Expression::Binary { left, op, right } => {
            node("Binary", format!("{op:?}"), vec![expression(left), expression(right)])
        },
        Expression::Logical { op, operands } => {
            node("Logical", op, operands.iter().map(expression).collect())
        },
        Expression::In { expr, list } => node(
            "In",
            "membership",
            vec![expression(expr), node("Array", format!("{list:?}"), vec![])],
        ),
        Expression::Aggregate { path, func, condition } => {
            node("Aggregate", func, vec![node("Path", path, vec![]), expression(&condition.expr)])
        },
        Expression::Call { name, args } => {
            node("Call", name, args.iter().map(expression).collect())
        },
    }
}

fn conditions(items: &[Condition]) -> Vec<Json> {
    items
        .iter()
        .map(|c| node("Condition", "expression", vec![expression(&c.expr)]))
        .collect()
}

fn ast_example(source: &str) -> Result<Json, Box<dyn std::error::Error>> {
    let policy = Parser::new(source).parse_policy()?;
    let requirements = match &policy.requirements {
        Requirements::Requires { conditions: items, where_clause } => {
            let mut children = conditions(items);
            if let Some(items) = where_clause {
                children.push(node("Where", "filter", conditions(items)));
            }
            node("Requires", "allow conditions", children)
        },
        Requirements::Denies { reason } => {
            node("Denies", reason.as_deref().unwrap_or("no reason"), vec![])
        },
    };
    let tree = node(
        "Policy",
        &policy.name,
        vec![
            node("Intent", &policy.intent, vec![]),
            node("Triggers", "when", conditions(&policy.triggers)),
            requirements,
        ],
    );
    Ok(json!({"name": policy.name, "source": source, "tree": tree}))
}

// Hand-built bytecode isolates interpreter execution from the unfinished compiler.
// Every comparison contributes to the decision; a final branch returns allow or deny.
fn workload(comparisons: usize) -> CompiledPolicy {
    let mut policy = CompiledPolicy::new(comparisons as u64);
    policy.constants.push(Value::Int(2));
    for i in 0..comparisons {
        policy.code.extend([
            Instruction::LoadField { offset: 0 },
            Instruction::LoadConst { idx: 0 },
            Instruction::Compare { op: CompOp::Gte },
        ]);
        if i > 0 {
            policy.code.push(Instruction::And);
        }
    }
    policy.code.extend([
        Instruction::JumpIfFalse { offset: 2 },
        Instruction::Return { value: true },
        Instruction::Return { value: false },
    ]);
    policy
}

fn measure(comparisons: usize) -> Json {
    let policy = workload(comparisons);
    let fields = HashMap::from([(0, vec!["resource".into(), "approvals".into()])]);
    let mut interpreter = Interpreter::new(fields);
    let mut context = EvaluationContext::default();
    context.resource.attributes.insert("approvals".into(), AttributeValue::Int(3));
    assert!(interpreter.evaluate(&policy, &context).unwrap());
    let mut denied = context.clone();
    denied.resource.attributes.insert("approvals".into(), AttributeValue::Int(1));
    assert!(!interpreter.evaluate(&policy, &denied).unwrap());
    let contexts = [context, denied];
    for i in 0..2_000 {
        black_box(interpreter.evaluate(black_box(&policy), black_box(&contexts[i % 2])).unwrap());
    }
    let mut samples = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        let start = Instant::now();
        black_box(interpreter.evaluate(black_box(&policy), black_box(&contexts[i % 2])).unwrap());
        samples.push(start.elapsed().as_secs_f64() * 1_000_000.0);
    }
    let start = Instant::now();
    for i in 0..100_000 {
        black_box(interpreter.evaluate(black_box(&policy), black_box(&contexts[i % 2])).unwrap());
    }
    let throughput = 100_000.0 / start.elapsed().as_secs_f64();
    samples.sort_by(f64::total_cmp);
    let percentile = |p: usize| samples[(p * samples.len()).div_ceil(100).saturating_sub(1)];
    let q1 = percentile(25);
    let q3 = percentile(75);
    let outlier_threshold = q3 + 1.5 * (q3 - q1);
    let quantiles: Vec<_> =
        (1..=100).map(|p| json!({"percentile": p, "us": percentile(p)})).collect();
    json!({
        "name": format!("{comparisons} comparison{}", if comparisons == 1 { "" } else { "s" }),
        "comparisons": comparisons, "instructions": policy.code.len(), "samples": samples.len(),
        "p50_us": percentile(50), "p95_us": percentile(95), "p99_us": percentile(99),
        "max_us": samples[samples.len()-1], "throughput": throughput,
        "outliers": samples.iter().filter(|s| **s > outlier_threshold).count(),
        "outlier_threshold_us": outlier_threshold, "quantiles": quantiles,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(debug_assertions) {
        return Err("Run this exporter with --release".into());
    }
    let examples = [
        "policy RequireApproval:\n  \"Production deployments need senior approval\"\n\n  triggers when\n    resource.type == \"Deployment\" and resource.environment in [\"production\", \"staging\"]\n\n  requires\n    resource.approvals >= 2\n    where request.principal.role == \"senior-engineer\"\n",
        "policy FeatureAccess:\n  \"Enable access for paid or beta accounts\"\n\n  triggers when\n    resource.type == \"Feature\"\n\n  requires\n    resource.plan == \"paid\" or resource.beta == true\n",
        "policy BlockProduction:\n  \"Freeze production deployments\"\n\n  triggers when\n    resource.environment == \"production\"\n\n  denies with reason \"Change freeze is active\"\n",
    ].iter().map(|source| ast_example(source)).collect::<Result<Vec<_>, _>>()?;
    let revision = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()?;
    let dirty = std::process::Command::new("git").args(["status", "--porcelain"]).output()?;
    let rustc = std::process::Command::new("rustc").arg("--version").output()?;
    let output = json!({
        "schema_version": 1,
        "generated_at": chrono::Utc::now().to_rfc3339(),
        "revision": String::from_utf8_lossy(&revision.stdout).trim(),
        "working_tree_modified": !dirty.stdout.is_empty(),
        "platform": format!("{} / {}", std::env::consts::OS, std::env::consts::ARCH),
        "rustc": String::from_utf8_lossy(&rustc.stdout).trim(),
        "profile": "release", "engine": "Interpreter", "examples": examples,
        "workloads": ([1, 8, 32].map(measure)),
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
