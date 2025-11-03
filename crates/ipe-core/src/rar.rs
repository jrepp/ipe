use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[cfg(feature = "approvals")]
use std::sync::Arc;

/// Complete evaluation context for a policy decision
#[derive(Debug, Clone, Default)]
pub struct EvaluationContext {
    pub resource: Resource,
    pub action: Action,
    pub request: Request,

    #[cfg(feature = "approvals")]
    pub approval_store: Option<Arc<crate::approval::ApprovalStore>>,

    #[cfg(feature = "approvals")]
    pub relationship_store: Option<Arc<crate::relationship::RelationshipStore>>,
}

impl EvaluationContext {
    /// Create a new evaluation context
    pub fn new(resource: Resource, action: Action, request: Request) -> Self {
        Self {
            resource,
            action,
            request,
            #[cfg(feature = "approvals")]
            approval_store: None,
            #[cfg(feature = "approvals")]
            relationship_store: None,
        }
    }

    #[cfg(feature = "approvals")]
    /// Add approval store to evaluation context
    pub fn with_approval_store(mut self, store: Arc<crate::approval::ApprovalStore>) -> Self {
        self.approval_store = Some(store);
        self
    }

    #[cfg(feature = "approvals")]
    /// Add relationship store to evaluation context
    pub fn with_relationship_store(mut self, store: Arc<crate::relationship::RelationshipStore>) -> Self {
        self.relationship_store = Some(store);
        self
    }

    #[cfg(feature = "approvals")]
    /// Check if current request has approval
    pub fn has_approval(&self) -> crate::Result<bool> {
        let store = self.approval_store
            .as_ref()
            .ok_or(crate::Error::NoApprovalStore)?;

        // Extract URL from resource attributes or use a default
        let resource_url = self.resource.attributes
            .get("url")
            .and_then(|v| match v {
                AttributeValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| self.action.target.clone());

        // Extract HTTP method from action or use operation as string
        let action_method = self.action.attributes
            .get("method")
            .and_then(|v| match v {
                AttributeValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| format!("{:?}", self.action.operation));

        store.has_approval(&self.request.principal.id, &resource_url, &action_method)
            .map_err(|e| e.into())
    }

    #[cfg(feature = "approvals")]
    /// Check if the principal has a specific relationship to an object
    ///
    /// Examples:
    /// - ctx.has_relationship("editor", "document-123") - is the principal an editor of the document?
    /// - ctx.has_relationship("member_of", "admin-group") - is the principal a member of the group?
    pub fn has_relationship(&self, relation: &str, object: &str) -> crate::Result<bool> {
        let store = self.relationship_store
            .as_ref()
            .ok_or(crate::Error::NoRelationshipStore)?;

        store.has_relationship(&self.request.principal.id, relation, object)
            .map_err(|e| e.into())
    }

    #[cfg(feature = "approvals")]
    /// Check if the principal has a transitive relationship to an object
    ///
    /// This follows relationship chains. For example:
    /// - "cert-1" is trusted_by "intermediate-ca"
    /// - "intermediate-ca" is trusted_by "root-ca"
    /// - Then has_transitive_relationship("trusted_by", "root-ca") returns true
    pub fn has_transitive_relationship(&self, relation: &str, object: &str) -> crate::Result<bool> {
        let store = self.relationship_store
            .as_ref()
            .ok_or(crate::Error::NoRelationshipStore)?;

        store.has_transitive_relationship(&self.request.principal.id, relation, object)
            .map_err(|e| e.into())
    }

    #[cfg(feature = "approvals")]
    /// Find the relationship path from principal to object
    pub fn find_relationship_path(&self, relation: &str, object: &str) -> crate::Result<Option<crate::relationship::RelationshipPath>> {
        let store = self.relationship_store
            .as_ref()
            .ok_or(crate::Error::NoRelationshipStore)?;

        store.find_relationship_path(&self.request.principal.id, relation, object)
            .map_err(|e| e.into())
    }
}

/// Resource being accessed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub type_id: ResourceTypeId,
    pub attributes: HashMap<String, AttributeValue>,
}

impl Resource {
    pub fn new(type_id: ResourceTypeId) -> Self {
        Self {
            type_id,
            attributes: HashMap::new(),
        }
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    pub fn url(url: impl Into<String>) -> Self {
        Self::new(ResourceTypeId(0))
            .with_attribute("url", AttributeValue::String(url.into()))
    }
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
    pub attributes: HashMap<String, AttributeValue>,
}

impl Action {
    pub fn new(operation: Operation, target: impl Into<String>) -> Self {
        Self {
            operation,
            target: target.into(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }
}

impl Default for Action {
    fn default() -> Self {
        Self {
            operation: Operation::Read,
            target: String::new(),
            attributes: HashMap::new(),
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

impl Principal {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            roles: Vec::new(),
            attributes: HashMap::new(),
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    pub fn with_attribute(mut self, key: impl Into<String>, value: AttributeValue) -> Self {
        self.attributes.insert(key.into(), value);
        self
    }

    pub fn bot(id: impl Into<String>) -> Self {
        Self::new(id)
            .with_attribute("type", AttributeValue::String("bot".into()))
    }

    pub fn user(id: impl Into<String>) -> Self {
        Self::new(id)
            .with_attribute("type", AttributeValue::String("user".into()))
    }
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
        ctx.resource
            .attributes
            .insert("name".to_string(), AttributeValue::String("test-deployment".to_string()));

        assert_eq!(ctx.resource.attributes.len(), 1);
    }
}
