//! Source-backed external governance descriptions. These records never grant authority.
use super::model::{Resolution, Source};
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use serde_json::json;

fn text(s: &str) -> bool {
    !s.is_empty() && s.len() <= 2048 && !s.chars().any(char::is_control)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Id {
    pub namespace: String,
    pub authority: String,
    pub local_id: String,
}
impl Id {
    fn validate(&self) -> bool {
        text(&self.namespace) && text(&self.authority) && text(&self.local_id)
    }
}

macro_rules! enumeration {
    ($name:ident { $($variant:ident),+ }) => {
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
        #[serde(rename_all = "snake_case")]
        pub(super) enum $name { $($variant),+ }
    };
}
enumeration!(Kind {
    Policy,
    Contract,
    Control,
    Handoff,
    Principal,
    RoleAssignment,
    ContactMethod
});
enumeration!(PrincipalKind { Human, Service, Group, Role, Unknown });
enumeration!(Relationship {
    Membership,
    Responsibility,
    ApprovalAuthority,
    Delegation
});
enumeration!(ContactKind {
    Email,
    ProviderProfile,
    TeamChannel,
    TicketQueue,
    OnCall
});
enumeration!(Availability { Available, Unavailable, Unknown });

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct Reference {
    pub kind: Kind,
    pub id: Id,
    pub source_revision: Option<String>,
    pub resolution: Resolution,
}
impl Reference {
    fn validate(&self) -> bool {
        self.id.validate()
            && self.source_revision.as_ref().is_none_or(|s| text(s))
            && (self.resolution != Resolution::Resolved || self.source_revision.is_some())
    }
    pub fn matches(&self, target: &GovernanceRecord) -> bool {
        self.kind == target.kind()
            && self.id == *target.id()
            && self.source_revision.as_deref() == Some(target.source().revision.as_str())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct NamedRequirement {
    pub id: String,
    pub description: String,
    pub specification: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum GovernanceRecord {
    Policy {
        id: Id,
        source: Source,
        title: String,
        authority: Reference,
        subjects: Vec<String>,
        requirements: Vec<NamedRequirement>,
    },
    Contract {
        id: Id,
        source: Source,
        title: String,
        parties: Vec<Reference>,
        subjects: Vec<String>,
        requirements: Vec<NamedRequirement>,
    },
    Control {
        id: Id,
        source: Source,
        title: String,
        governing_rules: Vec<Reference>,
        mechanism: String,
    },
    Handoff {
        id: Id,
        source: Source,
        title: String,
        policy: Reference,
        from: Reference,
        to: Reference,
        trigger: String,
        required_action: String,
        completion: String,
        continuation: String,
    },
    Principal {
        id: Id,
        source: Source,
        title: String,
        principal_kind: PrincipalKind,
    },
    RoleAssignment {
        id: Id,
        source: Source,
        principal: Reference,
        role: Reference,
        relationship: Relationship,
        valid_from: Option<u64>,
        valid_until: Option<u64>,
    },
    ContactMethod {
        id: Id,
        source: Source,
        owner: Reference,
        channel: ContactKind,
        endpoint: Option<String>,
        availability: Availability,
        purpose: String,
        valid_until: Option<u64>,
    },
}
impl GovernanceRecord {
    pub fn kind(&self) -> Kind {
        match self {
            Self::Policy { .. } => Kind::Policy,
            Self::Contract { .. } => Kind::Contract,
            Self::Control { .. } => Kind::Control,
            Self::Handoff { .. } => Kind::Handoff,
            Self::Principal { .. } => Kind::Principal,
            Self::RoleAssignment { .. } => Kind::RoleAssignment,
            Self::ContactMethod { .. } => Kind::ContactMethod,
        }
    }
    pub fn id(&self) -> &Id {
        match self {
            Self::Policy { id, .. }
            | Self::Contract { id, .. }
            | Self::Control { id, .. }
            | Self::Handoff { id, .. }
            | Self::Principal { id, .. }
            | Self::RoleAssignment { id, .. }
            | Self::ContactMethod { id, .. } => id,
        }
    }
    pub fn source(&self) -> &Source {
        match self {
            Self::Policy { source, .. }
            | Self::Contract { source, .. }
            | Self::Control { source, .. }
            | Self::Handoff { source, .. }
            | Self::Principal { source, .. }
            | Self::RoleAssignment { source, .. }
            | Self::ContactMethod { source, .. } => source,
        }
    }
    pub fn logical_key(&self) -> String {
        let id = self.id();
        format!("governance:{}", json!([self.kind(), id.namespace, id.authority, id.local_id]))
    }
    pub fn valid_until(&self) -> Option<u64> {
        match self {
            Self::RoleAssignment { valid_until, .. } | Self::ContactMethod { valid_until, .. } => {
                *valid_until
            },
            _ => None,
        }
    }
    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::RoleAssignment { .. } | Self::ContactMethod { .. })
    }
    pub fn references(&self) -> Vec<&Reference> {
        match self {
            Self::Policy { authority, .. } => vec![authority],
            Self::Contract { parties, .. } => parties.iter().collect(),
            Self::Control { governing_rules, .. } => governing_rules.iter().collect(),
            Self::Handoff { policy, from, to, .. } => vec![policy, from, to],
            Self::Principal { .. } => vec![],
            Self::RoleAssignment { principal, role, .. } => vec![principal, role],
            Self::ContactMethod { owner, .. } => vec![owner],
        }
    }
    pub fn validate(&self) -> bool {
        let refs = self.references();
        self.id().validate()
            && self.source().validate()
            && refs.len() <= 128
            && refs.iter().all(|r| r.validate())
            && match self {
                Self::Policy {
                    title, authority, subjects, requirements, ..
                } => {
                    valid_subjects(subjects)
                        && text(title)
                        && authority.kind == Kind::Principal
                        && valid_requirements(requirements)
                },
                Self::Contract {
                    title, parties, subjects, requirements, ..
                } => {
                    valid_subjects(subjects)
                        && text(title)
                        && parties.len() >= 2
                        && {
                            let mut seen = std::collections::HashSet::new();
                            parties.iter().all(|r| {
                                seen.insert(serde_json::to_string(&r.id).unwrap_or_default())
                            })
                        }
                        && parties.iter().all(|r| r.kind == Kind::Principal)
                        && valid_requirements(requirements)
                },
                Self::Control { title, governing_rules, mechanism, .. } => {
                    text(title)
                        && text(mechanism)
                        && governing_rules
                            .iter()
                            .all(|r| matches!(r.kind, Kind::Policy | Kind::Contract))
                },
                Self::Handoff {
                    title,
                    policy,
                    from,
                    to,
                    trigger,
                    required_action,
                    completion,
                    continuation,
                    ..
                } => {
                    text(title)
                        && policy.kind == Kind::Policy
                        && from.kind == Kind::Principal
                        && to.kind == Kind::Principal
                        && [trigger, required_action, completion, continuation]
                            .iter()
                            .all(|s| text(s))
                },
                Self::Principal { title, .. } => text(title),
                Self::RoleAssignment {
                    principal, role, valid_from, valid_until, ..
                } => {
                    principal.kind == Kind::Principal
                        && role.kind == Kind::Principal
                        && valid_from.zip(*valid_until).is_none_or(|(a, b)| a < b)
                },
                Self::ContactMethod {
                    owner, endpoint, availability, purpose, ..
                } => {
                    owner.kind == Kind::Principal
                        && text(purpose)
                        && (*availability != Availability::Available || endpoint.is_some())
                        && endpoint.as_ref().is_none_or(|s| valid_endpoint(s))
                },
            }
    }
}
fn valid_requirements(items: &[NamedRequirement]) -> bool {
    let mut ids = std::collections::HashSet::new();
    items.len() <= 128
        && items.iter().all(|r| {
            text(&r.id) && ids.insert(&r.id) && text(&r.description) && text(&r.specification)
        })
}
fn valid_endpoint(s: &str) -> bool {
    if !text(s) || s.chars().any(char::is_whitespace) {
        return false;
    }
    if s.contains("://") {
        reqwest::Url::parse(s).is_ok_and(|u| {
            u.scheme() == "https"
                && u.username().is_empty()
                && u.password().is_none()
                && u.query().is_none()
                && u.fragment().is_none()
        })
    } else {
        !s.contains(':') && !s.starts_with("//")
    }
}

fn valid_subjects(items: &[String]) -> bool {
    let mut seen = std::collections::HashSet::new();
    items.len() <= 128 && items.iter().all(|s| text(s) && seen.insert(s))
}

#[cfg(test)]
mod tests {
    use super::super::model::{validate_graph, Claim, Entity, Model};
    use super::*;
    fn principal() -> serde_json::Value {
        json!({"type":"principal","id":{"namespace":"fixture","authority":"directory","local_id":"reviewer"},"source":{"definition":{"namespace":"fixture","resource":"directory","path":"roles"},"revision":"r1","pointer":"/0"},"title":"Release reviewer","principal_kind":"role"})
    }
    fn model(record: serde_json::Value) -> Model {
        serde_json::from_value(json!({"schema_version":2,"scope":"release","claim":"provider_reported","entity":{"kind":"governance","record":record}})).unwrap()
    }
    fn reference() -> serde_json::Value {
        json!({"kind":"principal","id":principal()["id"],"source_revision":"r1","resolution":"resolved"})
    }
    fn policy() -> serde_json::Value {
        json!({"type":"policy","id":{"namespace":"fixture","authority":"policy","local_id":"release"},"source":principal()["source"],"title":"Release approval","authority":reference(),"subjects":["provider:release"],"requirements":[{"id":"approval","description":"Reviewer approval","specification":"One reviewer must approve"}]})
    }
    fn stored(model: Model) -> super::super::Record {
        super::super::Record {
            kind: model.kind().into(),
            key: model.key(),
            revision_id: model.revision("collector").unwrap(),
            model: Some(model),
            data: json!({}),
            receipts: vec!["receipt".into()],
            source_evidence: vec![],
            observed_at: 1,
            expires_at: None,
            run_id: 1,
            adapter_digest: "collector".into(),
        }
    }
    #[test]
    fn unknown_principal_kind_does_not_assert_a_human_identity() {
        let mut account = principal();
        account["principal_kind"] = json!("unknown");
        let account = model(account);
        account.validate().unwrap();
        assert_eq!(
            serde_json::to_value(account).unwrap()["entity"]["record"]["principal_kind"],
            json!("unknown")
        );
    }
    #[test]
    fn governance_versions_and_source_revisions_remain_explicit() {
        let a = model(policy());
        a.validate().unwrap();
        let mut changed = policy();
        changed["source"]["revision"] = json!("r2");
        let b = model(changed);
        assert_eq!(a.logical_key(), b.logical_key());
        assert_ne!(a.key(), b.key());
        let mut old = a.clone();
        old.schema_version = 1;
        assert_eq!(old.validate(), Err("unsupported_model_schema"));
        let mut inferred = a;
        inferred.claim = Claim::Inferred;
        assert_eq!(inferred.validate(), Err("invalid_model"));
        let mut unknown = policy();
        unknown["secret"] = json!(true);
        assert!(serde_json::from_value::<GovernanceRecord>(unknown).is_err());
    }
    #[test]
    fn resolved_references_require_exact_captured_source_revision() {
        let owner = stored(model(principal()));
        let rule = stored(model(policy()));
        assert!(validate_graph(&[owner.clone(), rule.clone()]).is_ok());
        assert_eq!(validate_graph(std::slice::from_ref(&rule)), Err("resolved_governance_missing"));
        let mut stale = principal();
        stale["source"]["revision"] = json!("r2");
        assert_eq!(
            validate_graph(&[stored(model(stale)), rule]),
            Err("resolved_governance_missing")
        );
        let mut candidate = policy();
        candidate["authority"]["resolution"] = json!("candidate");
        candidate["authority"]["source_revision"] = serde_json::Value::Null;
        let candidate = model(candidate);
        candidate.validate().unwrap();
        assert!(validate_graph(&[stored(candidate)]).is_ok());
        let mut unpinned = policy();
        unpinned["authority"]["source_revision"] = serde_json::Value::Null;
        assert_eq!(model(unpinned).validate(), Err("invalid_model"));
    }
    #[test]
    fn contact_routes_are_data_with_safe_bounds_and_no_credential_urls() {
        for endpoint in
            ["reviewer@example.test", "https://example.test/teams/reviewers", "queue/release"]
        {
            assert!(valid_endpoint(endpoint));
        }
        for endpoint in [
            "https://token@example.test/profile",
            "https://example.test/profile?token=secret",
            "javascript:alert(1)",
            "//token@example.test",
            "a\nb",
        ] {
            assert!(!valid_endpoint(endpoint));
        }
        let contact = json!({"type":"contact_method","id":principal()["id"],"source":principal()["source"],"owner":reference(),"channel":"email","endpoint":null,"availability":"available","purpose":"Approval request","valid_until":null});
        assert!(model(contact.clone()).validate().is_err());
        let mut unknown = contact;
        unknown["availability"] = json!("unknown");
        let unknown = model(unknown);
        unknown.validate().unwrap();
        if let Entity::Governance { record } = unknown.entity {
            assert!(record.is_temporal());
        } else {
            panic!("governance expected");
        }
    }
    #[test]
    fn parties_are_distinct_and_role_targets_are_roles_or_groups() {
        let contract = json!({"type":"contract","id":principal()["id"],"source":principal()["source"],"title":"Release contract","subjects":[],"parties":[reference(),reference()],"requirements":[]});
        assert!(model(contract).validate().is_err());
        let assignment = json!({"type":"role_assignment","id":principal()["id"],"source":principal()["source"],"principal":reference(),"role":reference(),"relationship":"membership","valid_from":1,"valid_until":20});
        let assignment = model(assignment);
        assignment.validate().unwrap();
        if let Entity::Governance { record } = &assignment.entity {
            assert_eq!(record.valid_until(), Some(20));
        }
        assert!(validate_graph(&[stored(assignment.clone()), stored(model(principal()))]).is_ok());
        let mut human = principal();
        human["principal_kind"] = json!("human");
        assert_eq!(
            validate_graph(&[stored(assignment), stored(model(human))]),
            Err("invalid_governance_role_target")
        );
    }
    #[test]
    fn duplicate_requirements_and_inverted_validity_are_rejected() {
        let mut rule = policy();
        let duplicate = rule["requirements"][0].clone();
        rule["requirements"].as_array_mut().unwrap().push(duplicate);
        assert!(model(rule).validate().is_err());
        let assignment = json!({"type":"role_assignment","id":principal()["id"],"source":principal()["source"],"principal":reference(),"role":reference(),"relationship":"approval_authority","valid_from":20,"valid_until":10});
        assert!(model(assignment).validate().is_err());
    }
}
