use shabti_core::a2a::{A2AMessage, A2ARegistry, AgentCard, MessageType};

// --- 6-2: A2A (Agent-to-Agent) protocol foundation ---

#[test]
fn create_agent_card() {
    let card = AgentCard::new(
        "shabti-memory",
        "Memory management agent",
        "https://localhost:8080",
    )
    .capability("memory/store")
    .capability("memory/search");

    assert_eq!(card.name, "shabti-memory");
    assert_eq!(card.description, "Memory management agent");
    assert_eq!(card.endpoint, "https://localhost:8080");
    assert_eq!(card.capabilities.len(), 2);
    assert!(card.capabilities.contains(&"memory/store".to_string()));
}

#[test]
fn agent_card_to_json() {
    let card = AgentCard::new("test-agent", "A test agent", "https://example.com")
        .capability("test/action");

    let json = card.to_json();
    assert_eq!(json["name"], "test-agent");
    assert_eq!(json["description"], "A test agent");
    assert_eq!(json["endpoint"], "https://example.com");
    assert!(json["capabilities"].is_array());
}

#[test]
fn create_a2a_message() {
    let msg = A2AMessage::new(
        "agent-a",
        "agent-b",
        MessageType::Request,
        serde_json::json!({"action": "memory/search", "query": "hello"}),
    );

    assert_eq!(msg.from, "agent-a");
    assert_eq!(msg.to, "agent-b");
    assert_eq!(msg.message_type, MessageType::Request);
    assert_eq!(msg.content["action"], "memory/search");
    assert!(!msg.id.is_empty());
}

#[test]
fn message_type_variants() {
    assert_ne!(MessageType::Request, MessageType::Response);
    assert_ne!(MessageType::Request, MessageType::Notification);
    assert_ne!(MessageType::Response, MessageType::Error);
}

#[test]
fn a2a_message_to_json() {
    let msg = A2AMessage::new(
        "sender",
        "receiver",
        MessageType::Request,
        serde_json::json!({"data": 42}),
    );

    let json = msg.to_json();
    assert_eq!(json["from"], "sender");
    assert_eq!(json["to"], "receiver");
    assert_eq!(json["type"], "request");
    assert_eq!(json["content"]["data"], 42);
    assert!(json["id"].is_string());
}

#[test]
fn registry_register_and_discover() {
    let mut registry = A2ARegistry::new();

    registry.register(
        AgentCard::new("memory-agent", "Handles memory", "https://mem:8080")
            .capability("memory/store")
            .capability("memory/search"),
    );
    registry.register(
        AgentCard::new("code-agent", "Handles code", "https://code:8080")
            .capability("code/analyze")
            .capability("code/refactor"),
    );

    assert_eq!(registry.agents().len(), 2);
}

#[test]
fn registry_discover_by_capability() {
    let mut registry = A2ARegistry::new();

    registry.register(
        AgentCard::new("memory-agent", "Memory", "https://mem:8080")
            .capability("memory/store")
            .capability("memory/search"),
    );
    registry.register(
        AgentCard::new("search-agent", "Search", "https://search:8080")
            .capability("memory/search")
            .capability("web/search"),
    );
    registry.register(
        AgentCard::new("code-agent", "Code", "https://code:8080").capability("code/analyze"),
    );

    let searchers = registry.discover("memory/search");
    assert_eq!(searchers.len(), 2);

    let coders = registry.discover("code/analyze");
    assert_eq!(coders.len(), 1);
    assert_eq!(coders[0].name, "code-agent");

    let none = registry.discover("nonexistent");
    assert!(none.is_empty());
}

#[test]
fn registry_find_by_name() {
    let mut registry = A2ARegistry::new();
    registry.register(AgentCard::new("my-agent", "Test", "https://test:8080"));

    assert!(registry.find("my-agent").is_some());
    assert!(registry.find("unknown").is_none());
}

#[test]
fn registry_unregister() {
    let mut registry = A2ARegistry::new();
    registry.register(AgentCard::new(
        "temp-agent",
        "Temporary",
        "https://tmp:8080",
    ));

    assert_eq!(registry.agents().len(), 1);
    registry.unregister("temp-agent");
    assert_eq!(registry.agents().len(), 0);
}

#[test]
fn registry_register_upsert() {
    let mut registry = A2ARegistry::new();
    registry.register(AgentCard::new("agent-a", "Version 1", "https://v1:8080"));
    registry.register(AgentCard::new("agent-a", "Version 2", "https://v2:8080"));

    assert_eq!(registry.agents().len(), 1);
    let agent = registry.find("agent-a").unwrap();
    assert_eq!(agent.description, "Version 2");
    assert_eq!(agent.endpoint, "https://v2:8080");
}
