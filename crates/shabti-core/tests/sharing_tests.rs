use shabti_core::agent::{Agent, AgentRole};
use shabti_core::namespace::NamespacePolicy;
use shabti_core::sharing::{ShareRequest, ShareResponse, SharingProtocol};
use shabti_core::types::Scope;

#[test]
fn share_request_creation() {
    let alice = Agent::new("alice", AgentRole::Worker);
    let bob = Agent::new("bob", AgentRole::Researcher);

    let request = ShareRequest::new(alice.id, bob.id, "agent:alice", "Found critical bug");
    assert_eq!(request.from_agent, alice.id);
    assert_eq!(request.to_agent, bob.id);
    assert_eq!(request.namespace, "agent:alice");
    assert!(request.is_pending());
}

#[test]
fn accept_share_request() {
    let alice = Agent::new("alice", AgentRole::Worker);
    let bob = Agent::new("bob", AgentRole::Researcher);

    let mut protocol = SharingProtocol::new();
    let mut policy = NamespacePolicy::new();

    let request_id = protocol.request_share(alice.id, bob.id, "agent:alice", "sharing data");

    // Bob cannot access alice's namespace yet
    assert_eq!(
        policy.check_access(&bob, "agent:alice", Scope::Private),
        shabti_core::namespace::AccessDecision::Denied
    );

    // Bob accepts
    let response = protocol.respond(request_id, ShareResponse::Accepted, &mut policy);
    assert!(response.is_ok());

    // Now bob can access
    assert_eq!(
        policy.check_access(&bob, "agent:alice", Scope::Private),
        shabti_core::namespace::AccessDecision::Allowed
    );
}

#[test]
fn reject_share_request() {
    let alice = Agent::new("alice", AgentRole::Worker);
    let bob = Agent::new("bob", AgentRole::Researcher);

    let mut protocol = SharingProtocol::new();
    let mut policy = NamespacePolicy::new();

    let request_id = protocol.request_share(alice.id, bob.id, "agent:alice", "sharing data");

    let response = protocol.respond(request_id, ShareResponse::Rejected, &mut policy);
    assert!(response.is_ok());

    // Still denied
    assert_eq!(
        policy.check_access(&bob, "agent:alice", Scope::Private),
        shabti_core::namespace::AccessDecision::Denied
    );
}

#[test]
fn list_pending_requests() {
    let alice = Agent::new("alice", AgentRole::Worker);
    let bob = Agent::new("bob", AgentRole::Researcher);
    let charlie = Agent::new("charlie", AgentRole::Worker);

    let mut protocol = SharingProtocol::new();

    protocol.request_share(alice.id, bob.id, "agent:alice", "data 1");
    protocol.request_share(charlie.id, bob.id, "agent:charlie", "data 2");
    protocol.request_share(alice.id, charlie.id, "agent:alice", "data 3");

    // Bob has 2 pending requests
    let bobs_pending = protocol.pending_for(bob.id);
    assert_eq!(bobs_pending.len(), 2);

    // Charlie has 1 pending request
    let charlies_pending = protocol.pending_for(charlie.id);
    assert_eq!(charlies_pending.len(), 1);
}

#[test]
fn invalid_request_id_returns_error() {
    let mut protocol = SharingProtocol::new();
    let mut policy = NamespacePolicy::new();

    let fake_id = uuid::Uuid::new_v4();
    let result = protocol.respond(fake_id, ShareResponse::Accepted, &mut policy);
    assert!(result.is_err());
}

#[test]
fn cannot_respond_twice() {
    let alice = Agent::new("alice", AgentRole::Worker);
    let bob = Agent::new("bob", AgentRole::Researcher);

    let mut protocol = SharingProtocol::new();
    let mut policy = NamespacePolicy::new();

    let request_id = protocol.request_share(alice.id, bob.id, "agent:alice", "data");

    // First response succeeds
    assert!(protocol
        .respond(request_id, ShareResponse::Accepted, &mut policy)
        .is_ok());

    // Second response fails
    assert!(protocol
        .respond(request_id, ShareResponse::Accepted, &mut policy)
        .is_err());
}
