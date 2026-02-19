use shabti_core::agent::{Agent, AgentRegistry, AgentRole};
use shabti_core::orchestrator::{Orchestrator, Task, TaskStatus, TaskResult};

#[test]
fn create_task() {
    let task = Task::new("Analyze code", "Review main.rs for security issues");
    assert_eq!(task.description, "Analyze code");
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(task.assigned_to.is_none());
}

#[test]
fn orchestrator_submit_and_assign() {
    let mut registry = AgentRegistry::new();
    let agent = Agent::builder("coder", AgentRole::Worker)
        .specialization("code-analysis")
        .build();
    let agent_id = agent.id;
    registry.register(agent);

    let mut orch = Orchestrator::new(registry);
    let task_id = orch.submit("Analyze code", "Review for bugs", &["code-analysis"]);

    // Task should be assigned to the agent with matching specialization
    let task = orch.get_task(task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Assigned);
    assert_eq!(task.assigned_to, Some(agent_id));
}

#[test]
fn orchestrator_routes_to_best_match() {
    let mut registry = AgentRegistry::new();

    let rust_dev = Agent::builder("rust-dev", AgentRole::Worker)
        .specialization("rust")
        .specialization("performance")
        .build();
    let rust_id = rust_dev.id;
    registry.register(rust_dev);

    let py_dev = Agent::builder("py-dev", AgentRole::Worker)
        .specialization("python")
        .specialization("data")
        .build();
    registry.register(py_dev);

    let mut orch = Orchestrator::new(registry);
    let task_id = orch.submit("Optimize loop", "Improve performance", &["rust", "performance"]);

    let task = orch.get_task(task_id).unwrap();
    assert_eq!(task.assigned_to, Some(rust_id));
}

#[test]
fn unmatched_task_stays_pending() {
    let mut registry = AgentRegistry::new();
    registry.register(
        Agent::builder("frontend", AgentRole::Worker)
            .specialization("react")
            .build(),
    );

    let mut orch = Orchestrator::new(registry);
    let task_id = orch.submit("Fix kernel bug", "Debug driver", &["kernel", "c"]);

    let task = orch.get_task(task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(task.assigned_to.is_none());
}

#[test]
fn complete_task_with_result() {
    let mut registry = AgentRegistry::new();
    let agent = Agent::builder("worker", AgentRole::Worker)
        .specialization("general")
        .build();
    registry.register(agent);

    let mut orch = Orchestrator::new(registry);
    let task_id = orch.submit("Do work", "Something", &["general"]);

    let result = orch.complete_task(task_id, TaskResult::success("Done!"));
    assert!(result.is_ok());

    let task = orch.get_task(task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.result.is_some());
    assert_eq!(task.result.as_ref().unwrap().output, "Done!");
}

#[test]
fn fail_task() {
    let mut registry = AgentRegistry::new();
    let agent = Agent::builder("worker", AgentRole::Worker)
        .specialization("general")
        .build();
    registry.register(agent);

    let mut orch = Orchestrator::new(registry);
    let task_id = orch.submit("Risky work", "Might fail", &["general"]);

    let result = orch.complete_task(task_id, TaskResult::failure("Out of memory"));
    assert!(result.is_ok());

    let task = orch.get_task(task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Failed);
}

#[test]
fn list_tasks_by_status() {
    let mut registry = AgentRegistry::new();
    registry.register(
        Agent::builder("a", AgentRole::Worker)
            .specialization("x")
            .build(),
    );

    let mut orch = Orchestrator::new(registry);
    let t1 = orch.submit("Task 1", "desc", &["x"]);
    orch.submit("Task 2", "desc", &["y"]); // unmatched -> pending
    orch.complete_task(t1, TaskResult::success("ok")).unwrap();

    assert_eq!(orch.tasks_by_status(TaskStatus::Completed).len(), 1);
    assert_eq!(orch.tasks_by_status(TaskStatus::Pending).len(), 1);
}

#[test]
fn list_tasks_for_agent() {
    let mut registry = AgentRegistry::new();
    let agent = Agent::builder("worker", AgentRole::Worker)
        .specialization("all")
        .build();
    let agent_id = agent.id;
    registry.register(agent);

    let mut orch = Orchestrator::new(registry);
    orch.submit("Task A", "desc", &["all"]);
    orch.submit("Task B", "desc", &["all"]);
    orch.submit("Task C", "desc", &["none"]); // unmatched

    let agent_tasks = orch.tasks_for_agent(agent_id);
    assert_eq!(agent_tasks.len(), 2);
}
