//! IPE Policy Engine HTTP Server
//!
//! A minimal HTTP server that exposes the IPE policy engine for evaluation.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use ipe_core::{
    rar::{AttributeValue, Operation, ResourceTypeId},
    DecisionKind, EvaluationContext, PolicyEngine,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Application state shared across handlers
struct AppState {
    engine: PolicyEngine,
}

/// Health check response
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

/// Evaluation request
#[derive(Deserialize)]
struct EvalRequest {
    resource_type: Option<u32>,
    action: String,
    principal_id: Option<String>,
    #[serde(default)]
    attributes: std::collections::HashMap<String, serde_json::Value>,
}

/// Evaluation response
#[derive(Serialize)]
struct EvalResponse {
    decision: String,
    matched_policies: Vec<String>,
    reason: Option<String>,
}

/// Metrics response (Prometheus format)
async fn metrics() -> impl IntoResponse {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = prometheus::gather();
    match encoder.encode_to_string(&metric_families) {
        Ok(output) => (StatusCode::OK, output),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode metrics: {}", e),
        ),
    }
}

/// Health check endpoint
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy",
        version: env!("CARGO_PKG_VERSION"),
    })
}

/// Readiness check endpoint
async fn ready(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Check if engine is ready
    let _ = &state.engine;
    (StatusCode::OK, "ready")
}

/// Parse operation from string
fn parse_operation(s: &str) -> Operation {
    match s.to_lowercase().as_str() {
        "create" => Operation::Create,
        "read" | "get" => Operation::Read,
        "update" | "put" | "patch" => Operation::Update,
        "delete" => Operation::Delete,
        "deploy" => Operation::Deploy,
        "execute" | "run" => Operation::Execute,
        _ => Operation::Custom(0),
    }
}

/// Evaluate a policy
async fn evaluate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EvalRequest>,
) -> Result<Json<EvalResponse>, (StatusCode, String)> {
    // Build resource
    let resource_type_id = ResourceTypeId(req.resource_type.unwrap_or(0));
    let mut resource = ipe_core::Resource::new(resource_type_id);

    // Add attributes from request
    for (key, value) in &req.attributes {
        let attr_value = match value {
            serde_json::Value::String(s) => AttributeValue::String(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    AttributeValue::Int(i)
                } else {
                    continue;
                }
            }
            serde_json::Value::Bool(b) => AttributeValue::Bool(*b),
            _ => continue,
        };
        resource = resource.with_attribute(key.clone(), attr_value);
    }

    // Build action
    let operation = parse_operation(&req.action);
    let action = ipe_core::Action::new(operation, &req.action);

    // Build principal
    let principal = ipe_core::Principal::user(
        req.principal_id.as_deref().unwrap_or("anonymous"),
    );

    // Build request
    let request = ipe_core::Request {
        principal,
        timestamp: chrono::Utc::now().timestamp(),
        source_ip: None,
        metadata: std::collections::HashMap::new(),
    };

    let ctx = EvaluationContext::new(resource, action, request);

    // Evaluate
    match state.engine.evaluate(&ctx) {
        Ok(decision) => Ok(Json(EvalResponse {
            decision: match decision.kind {
                DecisionKind::Allow => "allow".to_string(),
                DecisionKind::Deny => "deny".to_string(),
            },
            matched_policies: decision.matched_policies,
            reason: decision.reason,
        })),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    // Get configuration from environment
    // Note: Using IPE_SERVER_ prefix to avoid conflicts with K8s service discovery env vars
    let host = std::env::var("IPE_SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("IPE_SERVER_PORT").unwrap_or_else(|_| "9001".to_string());
    let metrics_port = std::env::var("IPE_SERVER_METRICS_PORT").unwrap_or_else(|_| "9090".to_string());

    // Create policy engine (empty for now - policies loaded via control plane)
    let engine = PolicyEngine::new();

    let state = Arc::new(AppState { engine });

    // Build main application router
    let app = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready))
        .route("/evaluate", post(evaluate))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Build metrics router (separate port)
    let metrics_app = Router::new().route("/metrics", get(metrics));

    info!("IPE Server v{} starting", env!("CARGO_PKG_VERSION"));
    info!("Data plane listening on {}:{}", host, port);
    info!("Metrics listening on {}:{}", host, metrics_port);

    // Spawn metrics server
    let metrics_addr = format!("{}:{}", host, metrics_port);
    tokio::spawn(async move {
        let listener = TcpListener::bind(&metrics_addr).await.unwrap();
        axum::serve(listener, metrics_app).await.unwrap();
    });

    // Start main server
    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
