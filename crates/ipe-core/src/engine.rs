use crate::{EvaluationContext, Result};
use serde::{Deserialize, Serialize};

/// Policy decision result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub kind: DecisionKind,
    pub reason: Option<String>,
    pub matched_policies: Vec<String>,
}

impl Decision {
    pub fn from_bool(allowed: bool) -> Self {
        Self {
            kind: if allowed { DecisionKind::Allow } else { DecisionKind::Deny },
            reason: None,
            matched_policies: vec![],
        }
    }
}

/// Decision kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecisionKind {
    Allow,
    Deny,
}

/// Main policy evaluation engine
pub struct PolicyEngine {
    // TODO: Add policy database
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {}
    }
    
    pub fn evaluate(&self, _ctx: &EvaluationContext) -> Result<Decision> {
        // TODO: Implement
        Ok(Decision {
            kind: DecisionKind::Allow,
            reason: None,
            matched_policies: vec![],
        })
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}
