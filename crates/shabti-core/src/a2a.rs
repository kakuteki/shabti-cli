use serde_json::{json, Value};
use uuid::Uuid;

/// An agent's public identity card for A2A discovery.
#[derive(Debug, Clone)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
}

impl AgentCard {
    pub fn new(name: &str, description: &str, endpoint: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            endpoint: endpoint.to_string(),
            capabilities: Vec::new(),
        }
    }

    pub fn capability(mut self, cap: &str) -> Self {
        self.capabilities.push(cap.to_string());
        self
    }

    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "url": self.endpoint,
            "capabilities": self.capabilities,
        })
    }
}

/// Type of A2A message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    Request,
    Response,
    Notification,
    Error,
}

impl MessageType {
    fn as_str(&self) -> &'static str {
        match self {
            MessageType::Request => "request",
            MessageType::Response => "response",
            MessageType::Notification => "notification",
            MessageType::Error => "error",
        }
    }
}

/// An A2A message between agents.
#[derive(Debug, Clone)]
pub struct A2AMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub message_type: MessageType,
    pub content: Value,
}

impl A2AMessage {
    pub fn new(from: &str, to: &str, message_type: MessageType, content: Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            from: from.to_string(),
            to: to.to_string(),
            message_type,
            content,
        }
    }

    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "from": self.from,
            "to": self.to,
            "type": self.message_type.as_str(),
            "content": self.content,
        })
    }
}

/// Registry for discovering agents by capability.
#[derive(Debug, Default)]
pub struct A2ARegistry {
    agents: Vec<AgentCard>,
}

impl A2ARegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, card: AgentCard) {
        self.agents.push(card);
    }

    pub fn unregister(&mut self, name: &str) {
        self.agents.retain(|a| a.name != name);
    }

    pub fn agents(&self) -> &[AgentCard] {
        &self.agents
    }

    pub fn find(&self, name: &str) -> Option<&AgentCard> {
        self.agents.iter().find(|a| a.name == name)
    }

    pub fn discover(&self, capability: &str) -> Vec<&AgentCard> {
        self.agents
            .iter()
            .filter(|a| a.capabilities.iter().any(|c| c == capability))
            .collect()
    }
}
