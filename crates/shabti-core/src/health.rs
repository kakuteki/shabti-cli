use std::fmt;

use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ComponentStatus {
    Healthy,
    Unhealthy(String),
}

impl fmt::Display for ComponentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ComponentStatus::Healthy => write!(f, "healthy"),
            ComponentStatus::Unhealthy(msg) => write!(f, "unhealthy: {msg}"),
        }
    }
}

#[derive(Debug, Clone)]
struct Component {
    name: String,
    status: ComponentStatus,
}

#[derive(Debug, Clone, Default)]
pub struct HealthCheck {
    components: Vec<Component>,
}

impl HealthCheck {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn component(mut self, name: &str, status: ComponentStatus) -> Self {
        self.components.push(Component {
            name: name.to_string(),
            status,
        });
        self
    }

    pub fn status(&self) -> HealthStatus {
        let unhealthy_count = self
            .components
            .iter()
            .filter(|c| matches!(c.status, ComponentStatus::Unhealthy(_)))
            .count();

        if unhealthy_count == 0 {
            HealthStatus::Healthy
        } else if unhealthy_count < self.components.len() {
            HealthStatus::Degraded
        } else {
            HealthStatus::Unhealthy
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.status() == HealthStatus::Healthy
    }

    pub fn components(&self) -> Vec<(&str, &ComponentStatus)> {
        self.components
            .iter()
            .map(|c| (c.name.as_str(), &c.status))
            .collect()
    }

    pub fn to_json(&self) -> Value {
        let mut components = serde_json::Map::new();
        for comp in &self.components {
            let val = match &comp.status {
                ComponentStatus::Healthy => json!({"status": "healthy"}),
                ComponentStatus::Unhealthy(msg) => json!({"status": "unhealthy", "message": msg}),
            };
            components.insert(comp.name.clone(), val);
        }

        json!({
            "status": self.status().to_string(),
            "components": components,
        })
    }
}
