use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete evaluation context for a policy decision
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationContext {
    pub resource: Resource,
    pub action: Action,
    pub request: Request,
}

/// Resource being accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub type_id: ResourceTypeId,
    pub attributes: HashMap<String, AttributeValue>,
}

impl Default for Resource {
    fn default() -> Self {
        Self {
            type_id: ResourceTypeId(0),
            attributes: HashMap::new(),
        }
    }
}

/// Resource type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceTypeId(pub u32);

/// Action being performed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub operation: Operation,
    pub target: String,
}

impl Default for Action {
    fn default() -> Self {
        Self {
            operation: Operation::Read,
            target: String::new(),
        }
    }
}

/// Operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    Create,
    Read,
    Update,
    Delete,
    Deploy,
    Execute,
    Custom(u32),
}

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub principal: Principal,
    pub timestamp: i64,
    pub source_ip: Option<String>,
    pub metadata: HashMap<String, AttributeValue>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            principal: Principal::default(),
            timestamp: 0,
            source_ip: None,
            metadata: HashMap::new(),
        }
    }
}

/// Principal (user/service) making the request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Principal {
    pub id: String,
    pub roles: Vec<String>,
    pub attributes: HashMap<String, AttributeValue>,
}

/// Attribute values (typed)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttributeValue {
    String(String),
    Int(i64),
    Bool(bool),
    Array(Vec<AttributeValue>),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_context_creation() {
        let mut ctx = EvaluationContext::default();
        ctx.resource.attributes.insert(
            "name".to_string(),
            AttributeValue::String("test-deployment".to_string())
        );
        
        assert_eq!(ctx.resource.attributes.len(), 1);
    }
}
