use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent::Agent;
use crate::types::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessDecision {
    Allowed,
    Denied,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NamespacePolicy {
    /// Explicit access grants: agent_id -> set of namespaces they can access.
    grants: HashMap<Uuid, HashSet<String>>,
}

impl NamespacePolicy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_access(&self, agent: &Agent, namespace: &str, scope: Scope) -> AccessDecision {
        match scope {
            Scope::Global | Scope::Shared => AccessDecision::Allowed,
            Scope::Private => {
                // Owner always has access
                if agent.namespace() == namespace {
                    return AccessDecision::Allowed;
                }
                // Check explicit grants
                if let Some(granted) = self.grants.get(&agent.id)
                    && granted.contains(namespace)
                {
                    return AccessDecision::Allowed;
                }
                AccessDecision::Denied
            }
        }
    }

    pub fn grant_access(&mut self, agent_id: Uuid, namespace: &str) {
        self.grants
            .entry(agent_id)
            .or_default()
            .insert(namespace.to_string());
    }

    pub fn revoke_access(&mut self, agent_id: Uuid, namespace: &str) {
        if let Some(granted) = self.grants.get_mut(&agent_id) {
            granted.remove(namespace);
        }
    }

    pub fn accessible_namespaces(&self, agent: &Agent) -> Vec<String> {
        let mut result = vec![agent.namespace()];
        if let Some(granted) = self.grants.get(&agent.id) {
            result.extend(granted.iter().cloned());
        }
        result
    }
}
