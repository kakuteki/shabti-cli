use std::collections::HashMap;

use uuid::Uuid;

use crate::agent::AgentRegistry;
use crate::error::{ShabtiError, ShabtiResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub output: String,
    pub success: bool,
}

impl TaskResult {
    pub fn success(output: &str) -> Self {
        Self {
            output: output.to_string(),
            success: true,
        }
    }

    pub fn failure(output: &str) -> Self {
        Self {
            output: output.to_string(),
            success: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: Uuid,
    pub description: String,
    pub detail: String,
    pub required_specs: Vec<String>,
    pub status: TaskStatus,
    pub assigned_to: Option<Uuid>,
    pub result: Option<TaskResult>,
}

impl Task {
    pub fn new(description: &str, detail: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            description: description.to_string(),
            detail: detail.to_string(),
            required_specs: Vec::new(),
            status: TaskStatus::Pending,
            assigned_to: None,
            result: None,
        }
    }
}

pub struct Orchestrator {
    registry: AgentRegistry,
    tasks: HashMap<Uuid, Task>,
}

impl Orchestrator {
    pub fn new(registry: AgentRegistry) -> Self {
        Self {
            registry,
            tasks: HashMap::new(),
        }
    }

    /// Submit a task and attempt to route it to an agent with matching specializations.
    pub fn submit(&mut self, description: &str, detail: &str, required_specs: &[&str]) -> Uuid {
        let mut task = Task::new(description, detail);
        task.required_specs = required_specs.iter().map(|s| s.to_string()).collect();

        // Find best matching agent: most overlapping specializations
        let mut best_agent = None;
        let mut best_score = 0usize;

        for agent in self.registry.list() {
            let score = required_specs
                .iter()
                .filter(|spec| agent.specializations.iter().any(|s| s == *spec))
                .count();
            if score > best_score {
                best_score = score;
                best_agent = Some(agent.id);
            }
        }

        if let Some(agent_id) = best_agent {
            task.status = TaskStatus::Assigned;
            task.assigned_to = Some(agent_id);
        }

        let id = task.id;
        self.tasks.insert(id, task);
        id
    }

    pub fn get_task(&self, id: Uuid) -> Option<&Task> {
        self.tasks.get(&id)
    }

    pub fn complete_task(&mut self, id: Uuid, result: TaskResult) -> ShabtiResult<()> {
        let task = self
            .tasks
            .get_mut(&id)
            .ok_or_else(|| ShabtiError::Storage(format!("task {id} not found")))?;

        task.status = if result.success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        task.result = Some(result);
        Ok(())
    }

    pub fn tasks_by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks.values().filter(|t| t.status == status).collect()
    }

    pub fn tasks_for_agent(&self, agent_id: Uuid) -> Vec<&Task> {
        self.tasks
            .values()
            .filter(|t| t.assigned_to == Some(agent_id))
            .collect()
    }
}
