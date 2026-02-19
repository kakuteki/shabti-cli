use shabti_core::agent::{Agent, AgentRegistry, AgentRole};
use shabti_core::contradiction::detect_contradiction;
use shabti_core::namespace::{AccessDecision, NamespacePolicy};
use shabti_core::orchestrator::{Orchestrator, TaskResult, TaskStatus};
use shabti_core::sharing::{ShareResponse, SharingProtocol};
use shabti_core::types::Scope;

// --- 5-7: Contradiction resolution with agent coordination ---

#[test]
fn detect_contradiction_between_agent_memories() {
    let text_a = "The deadline is March 15";
    let text_b = "The deadline is March 20";

    let result = detect_contradiction(text_a, text_b);
    assert!(result.is_some());
}

#[test]
fn no_contradiction_for_consistent_memories() {
    let text_a = "The project uses Rust";
    let text_b = "The project uses Rust for the core";

    let result = detect_contradiction(text_a, text_b);
    assert!(result.is_none());
}

#[test]
fn negation_contradiction_between_agents() {
    let text_a = "The feature is ready";
    let text_b = "The feature is not ready";

    let result = detect_contradiction(text_a, text_b);
    assert!(result.is_some());
}

// --- 5-9: E2E multi-agent scenario tests ---

#[test]
fn e2e_research_team_collaboration() {
    // Scenario: A planner creates tasks, workers execute, results are aggregated.
    let mut registry = AgentRegistry::new();

    let planner = Agent::builder("planner", AgentRole::Planner)
        .specialization("task-decomposition")
        .build();
    let planner_id = planner.id;

    let researcher = Agent::builder("researcher", AgentRole::Researcher)
        .specialization("literature-review")
        .specialization("analysis")
        .build();
    let researcher_id = researcher.id;

    let coder = Agent::builder("coder", AgentRole::Worker)
        .specialization("implementation")
        .specialization("rust")
        .build();
    let coder_id = coder.id;

    registry.register(planner);
    registry.register(researcher);
    registry.register(coder);

    let mut orch = Orchestrator::new(registry);

    // Planner submits research and implementation tasks
    let t1 = orch.submit(
        "Review papers",
        "Find relevant papers",
        &["literature-review"],
    );
    let t2 = orch.submit(
        "Implement algorithm",
        "Code the solution",
        &["implementation", "rust"],
    );
    let t3 = orch.submit(
        "Decompose project",
        "Break into subtasks",
        &["task-decomposition"],
    );

    // Verify routing
    assert_eq!(orch.get_task(t1).unwrap().assigned_to, Some(researcher_id));
    assert_eq!(orch.get_task(t2).unwrap().assigned_to, Some(coder_id));
    assert_eq!(orch.get_task(t3).unwrap().assigned_to, Some(planner_id));

    // Complete tasks
    orch.complete_task(t1, TaskResult::success("Found 5 relevant papers"))
        .unwrap();
    orch.complete_task(t2, TaskResult::success("Algorithm implemented"))
        .unwrap();
    orch.complete_task(t3, TaskResult::success("Project decomposed into 3 phases"))
        .unwrap();

    // All completed
    assert_eq!(orch.tasks_by_status(TaskStatus::Completed).len(), 3);
    assert_eq!(orch.tasks_by_status(TaskStatus::Pending).len(), 0);
}

#[test]
fn e2e_namespace_isolation_between_agents() {
    // Scenario: Two agents can only access their own namespaces until sharing is granted.
    let alice = Agent::builder("alice", AgentRole::Worker)
        .memory_scope(Scope::Private)
        .build();
    let bob = Agent::builder("bob", AgentRole::Researcher)
        .memory_scope(Scope::Private)
        .build();

    let policy = NamespacePolicy::new();

    // Each agent can access their own namespace
    assert_eq!(
        policy.check_access(&alice, "agent:alice", Scope::Private),
        AccessDecision::Allowed
    );
    assert_eq!(
        policy.check_access(&bob, "agent:bob", Scope::Private),
        AccessDecision::Allowed
    );

    // Cannot access each other's
    assert_eq!(
        policy.check_access(&alice, "agent:bob", Scope::Private),
        AccessDecision::Denied
    );
    assert_eq!(
        policy.check_access(&bob, "agent:alice", Scope::Private),
        AccessDecision::Denied
    );

    // Both can access shared scope
    assert_eq!(
        policy.check_access(&alice, "shared:team", Scope::Shared),
        AccessDecision::Allowed
    );
    assert_eq!(
        policy.check_access(&bob, "shared:team", Scope::Shared),
        AccessDecision::Allowed
    );
}

#[test]
fn e2e_memory_sharing_workflow() {
    // Scenario: Alice discovers something, shares with Bob through the protocol.
    let alice = Agent::new("alice", AgentRole::Worker);
    let bob = Agent::new("bob", AgentRole::Researcher);

    let mut policy = NamespacePolicy::new();
    let mut protocol = SharingProtocol::new();

    // Alice requests to share her namespace with Bob
    let req_id = protocol.request_share(
        alice.id,
        bob.id,
        "agent:alice",
        "Found critical info about the bug",
    );

    // Bob sees the pending request
    let pending = protocol.pending_for(bob.id);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].reason, "Found critical info about the bug");

    // Bob accepts
    protocol
        .respond(req_id, ShareResponse::Accepted, &mut policy)
        .unwrap();

    // Now Bob can access Alice's namespace
    assert_eq!(
        policy.check_access(&bob, "agent:alice", Scope::Private),
        AccessDecision::Allowed
    );

    // Alice can still access her own namespace
    assert_eq!(
        policy.check_access(&alice, "agent:alice", Scope::Private),
        AccessDecision::Allowed
    );

    // But Alice still cannot access Bob's
    assert_eq!(
        policy.check_access(&alice, "agent:bob", Scope::Private),
        AccessDecision::Denied
    );
}

#[test]
fn e2e_task_failure_and_reassignment() {
    // Scenario: A task fails, gets resubmitted.
    let mut registry = AgentRegistry::new();

    let primary = Agent::builder("primary", AgentRole::Worker)
        .specialization("analysis")
        .build();
    let primary_id = primary.id;
    registry.register(primary);

    let mut orch = Orchestrator::new(registry);

    let task_id = orch.submit("Analyze data", "Process dataset", &["analysis"]);
    assert_eq!(
        orch.get_task(task_id).unwrap().assigned_to,
        Some(primary_id)
    );

    // Task fails
    orch.complete_task(task_id, TaskResult::failure("Dataset corrupted"))
        .unwrap();
    assert_eq!(orch.get_task(task_id).unwrap().status, TaskStatus::Failed);

    // Resubmit with adjusted parameters
    let retry_id = orch.submit(
        "Analyze data (retry)",
        "Re-process with cleanup",
        &["analysis"],
    );
    assert_eq!(
        orch.get_task(retry_id).unwrap().status,
        TaskStatus::Assigned
    );
    assert_eq!(
        orch.get_task(retry_id).unwrap().assigned_to,
        Some(primary_id)
    );

    // Retry succeeds
    orch.complete_task(retry_id, TaskResult::success("Analysis complete"))
        .unwrap();

    assert_eq!(orch.tasks_by_status(TaskStatus::Completed).len(), 1);
    assert_eq!(orch.tasks_by_status(TaskStatus::Failed).len(), 1);
}
