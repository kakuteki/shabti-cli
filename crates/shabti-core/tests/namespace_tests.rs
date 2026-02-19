use shabti_core::agent::{Agent, AgentRole};
use shabti_core::namespace::{NamespacePolicy, AccessDecision};
use shabti_core::types::Scope;

#[test]
fn private_scope_allows_owner_access() {
    let agent = Agent::new("alice", AgentRole::Worker);
    let policy = NamespacePolicy::new();

    let decision = policy.check_access(&agent, "agent:alice", Scope::Private);
    assert_eq!(decision, AccessDecision::Allowed);
}

#[test]
fn private_scope_denies_other_agent() {
    let agent = Agent::new("bob", AgentRole::Worker);
    let policy = NamespacePolicy::new();

    let decision = policy.check_access(&agent, "agent:alice", Scope::Private);
    assert_eq!(decision, AccessDecision::Denied);
}

#[test]
fn shared_scope_allows_any_agent() {
    let agent = Agent::new("charlie", AgentRole::Researcher);
    let policy = NamespacePolicy::new();

    let decision = policy.check_access(&agent, "shared:project-x", Scope::Shared);
    assert_eq!(decision, AccessDecision::Allowed);
}

#[test]
fn global_scope_allows_read_for_all() {
    let agent = Agent::new("dave", AgentRole::Worker);
    let policy = NamespacePolicy::new();

    let decision = policy.check_access(&agent, "global:system", Scope::Global);
    assert_eq!(decision, AccessDecision::Allowed);
}

#[test]
fn agent_namespace_derived_from_name() {
    let agent = Agent::new("researcher-1", AgentRole::Researcher);
    assert_eq!(agent.namespace(), "agent:researcher-1");
}

#[test]
fn explicit_share_grants_access() {
    let alice = Agent::new("alice", AgentRole::Worker);
    let bob = Agent::new("bob", AgentRole::Researcher);
    let mut policy = NamespacePolicy::new();

    // Before share: denied
    let decision = policy.check_access(&bob, "agent:alice", Scope::Private);
    assert_eq!(decision, AccessDecision::Denied);

    // Grant explicit share
    policy.grant_access(bob.id, "agent:alice");

    // After share: allowed
    let decision = policy.check_access(&bob, "agent:alice", Scope::Private);
    assert_eq!(decision, AccessDecision::Allowed);

    // Alice still has access to own namespace
    let decision = policy.check_access(&alice, "agent:alice", Scope::Private);
    assert_eq!(decision, AccessDecision::Allowed);
}

#[test]
fn revoke_access_removes_grant() {
    let bob = Agent::new("bob", AgentRole::Worker);
    let mut policy = NamespacePolicy::new();

    policy.grant_access(bob.id, "agent:alice");
    assert_eq!(
        policy.check_access(&bob, "agent:alice", Scope::Private),
        AccessDecision::Allowed
    );

    policy.revoke_access(bob.id, "agent:alice");
    assert_eq!(
        policy.check_access(&bob, "agent:alice", Scope::Private),
        AccessDecision::Denied
    );
}

#[test]
fn list_accessible_namespaces() {
    let agent = Agent::builder("eve", AgentRole::Worker)
        .memory_scope(Scope::Private)
        .build();
    let mut policy = NamespacePolicy::new();

    policy.grant_access(agent.id, "shared:team-a");
    policy.grant_access(agent.id, "shared:team-b");

    let accessible = policy.accessible_namespaces(&agent);
    // Should include own namespace + granted
    assert!(accessible.contains(&agent.namespace()));
    assert!(accessible.contains(&"shared:team-a".to_string()));
    assert!(accessible.contains(&"shared:team-b".to_string()));
}

#[test]
fn serialization_roundtrip() {
    let bob = Agent::new("bob", AgentRole::Worker);
    let mut policy = NamespacePolicy::new();
    policy.grant_access(bob.id, "agent:alice");

    let json = serde_json::to_string(&policy).unwrap();
    let restored: NamespacePolicy = serde_json::from_str(&json).unwrap();

    assert_eq!(
        restored.check_access(&bob, "agent:alice", Scope::Private),
        AccessDecision::Allowed
    );
}
