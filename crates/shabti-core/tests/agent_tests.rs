use shabti_core::agent::{Agent, AgentRegistry, AgentRole};
use shabti_core::types::Scope;

#[test]
fn create_agent_with_defaults() {
    let agent = Agent::new("researcher", AgentRole::Researcher);
    assert_eq!(agent.name, "researcher");
    assert_eq!(agent.role, AgentRole::Researcher);
    assert_eq!(agent.namespace(), "agent:researcher");
    assert_eq!(agent.memory_scope, Scope::Private);
    assert!(agent.specializations.is_empty());
}

#[test]
fn create_agent_with_builder() {
    let agent = Agent::builder("planner", AgentRole::Planner)
        .memory_scope(Scope::Shared)
        .specialization("task-decomposition")
        .specialization("scheduling")
        .build();

    assert_eq!(agent.name, "planner");
    assert_eq!(agent.role, AgentRole::Planner);
    assert_eq!(agent.memory_scope, Scope::Shared);
    assert_eq!(agent.specializations.len(), 2);
    assert!(
        agent
            .specializations
            .contains(&"task-decomposition".to_string())
    );
}

#[test]
fn agent_id_is_unique() {
    let a = Agent::new("a", AgentRole::Worker);
    let b = Agent::new("b", AgentRole::Worker);
    assert_ne!(a.id, b.id);
}

#[test]
fn agent_serialization_roundtrip() {
    let agent = Agent::builder("coder", AgentRole::Worker)
        .memory_scope(Scope::Shared)
        .specialization("rust")
        .build();

    let json = serde_json::to_string(&agent).unwrap();
    let restored: Agent = serde_json::from_str(&json).unwrap();

    assert_eq!(agent.id, restored.id);
    assert_eq!(agent.name, restored.name);
    assert_eq!(agent.role, restored.role);
    assert_eq!(agent.memory_scope, restored.memory_scope);
    assert_eq!(agent.specializations, restored.specializations);
}

#[test]
fn registry_register_and_lookup() {
    let mut registry = AgentRegistry::new();
    let agent = Agent::new("analyst", AgentRole::Researcher);
    let id = agent.id;

    registry.register(agent);

    assert!(registry.get(id).is_some());
    assert_eq!(registry.get(id).unwrap().name, "analyst");
}

#[test]
fn registry_lookup_by_name() {
    let mut registry = AgentRegistry::new();
    registry.register(Agent::new("alpha", AgentRole::Worker));
    registry.register(Agent::new("beta", AgentRole::Researcher));

    let found = registry.get_by_name("alpha");
    assert!(found.is_some());
    assert_eq!(found.unwrap().role, AgentRole::Worker);

    assert!(registry.get_by_name("gamma").is_none());
}

#[test]
fn registry_list_all() {
    let mut registry = AgentRegistry::new();
    registry.register(Agent::new("a", AgentRole::Worker));
    registry.register(Agent::new("b", AgentRole::Planner));

    let all = registry.list();
    assert_eq!(all.len(), 2);
}

#[test]
fn registry_find_by_specialization() {
    let mut registry = AgentRegistry::new();
    registry.register(
        Agent::builder("coder", AgentRole::Worker)
            .specialization("rust")
            .specialization("python")
            .build(),
    );
    registry.register(
        Agent::builder("reviewer", AgentRole::Worker)
            .specialization("code-review")
            .build(),
    );

    let rust_agents = registry.find_by_specialization("rust");
    assert_eq!(rust_agents.len(), 1);
    assert_eq!(rust_agents[0].name, "coder");

    let empty = registry.find_by_specialization("java");
    assert!(empty.is_empty());
}

#[test]
fn registry_unregister() {
    let mut registry = AgentRegistry::new();
    let agent = Agent::new("temp", AgentRole::Worker);
    let id = agent.id;
    registry.register(agent);

    assert!(registry.get(id).is_some());
    registry.unregister(id);
    assert!(registry.get(id).is_none());
}
