use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::types::Scope;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    Worker,
    Researcher,
    Planner,
    Reviewer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub role: AgentRole,
    pub memory_scope: Scope,
    pub specializations: Vec<String>,
}

impl Agent {
    pub fn new(name: &str, role: AgentRole) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.to_string(),
            role,
            memory_scope: Scope::Private,
            specializations: Vec::new(),
        }
    }

    pub fn builder(name: &str, role: AgentRole) -> AgentBuilder {
        AgentBuilder {
            name: name.to_string(),
            role,
            memory_scope: Scope::Private,
            specializations: Vec::new(),
        }
    }

    pub fn namespace(&self) -> String {
        format!("agent:{}", self.name)
    }
}

pub struct AgentBuilder {
    name: String,
    role: AgentRole,
    memory_scope: Scope,
    specializations: Vec<String>,
}

impl AgentBuilder {
    pub fn memory_scope(mut self, scope: Scope) -> Self {
        self.memory_scope = scope;
        self
    }

    pub fn specialization(mut self, spec: &str) -> Self {
        self.specializations.push(spec.to_string());
        self
    }

    pub fn build(self) -> Agent {
        Agent {
            id: Uuid::new_v4(),
            name: self.name,
            role: self.role,
            memory_scope: self.memory_scope,
            specializations: self.specializations,
        }
    }
}

#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<Uuid, Agent>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, agent: Agent) {
        self.agents.insert(agent.id, agent);
    }

    pub fn unregister(&mut self, id: Uuid) {
        self.agents.remove(&id);
    }

    pub fn get(&self, id: Uuid) -> Option<&Agent> {
        self.agents.get(&id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Agent> {
        self.agents.values().find(|a| a.name == name)
    }

    pub fn list(&self) -> Vec<&Agent> {
        self.agents.values().collect()
    }

    pub fn find_by_specialization(&self, spec: &str) -> Vec<&Agent> {
        self.agents
            .values()
            .filter(|a| a.specializations.iter().any(|s| s == spec))
            .collect()
    }
}
