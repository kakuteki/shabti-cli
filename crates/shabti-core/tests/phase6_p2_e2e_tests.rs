use serde_json::json;
use shabti_core::a2a::{A2AMessage, A2ARegistry, AgentCard, MessageType};
use shabti_core::procedural::{ActionRecord, ProceduralMemory};
use shabti_core::wasm_store::{InMemoryStore, WasmMemoryStore};

// --- Phase 6 P2 E2E: Multi-agent with procedural memory ---

#[test]
fn e2e_multi_agent_workflow_with_procedural_learning() {
    // 1. Register agents
    let mut registry = A2ARegistry::new();
    registry.register(
        AgentCard::new(
            "search-agent",
            "Searches external sources",
            "http://localhost:3001",
        )
        .capability("web_search")
        .capability("document_search"),
    );
    registry.register(
        AgentCard::new(
            "memory-agent",
            "Manages memory storage",
            "http://localhost:3002",
        )
        .capability("memory_store")
        .capability("memory_search"),
    );

    // 2. Discover agents by capability
    let searchers = registry.discover("web_search");
    assert_eq!(searchers.len(), 1);
    assert_eq!(searchers[0].name, "search-agent");

    let storers = registry.discover("memory_store");
    assert_eq!(storers.len(), 1);
    assert_eq!(storers[0].name, "memory-agent");

    // 3. Simulate agent-to-agent messages
    let request = A2AMessage::new(
        "orchestrator",
        "search-agent",
        MessageType::Request,
        json!("Find information about Rust async patterns"),
    );
    assert_eq!(request.to, "search-agent");
    assert!(matches!(request.message_type, MessageType::Request));

    let response = A2AMessage::new(
        "search-agent",
        "orchestrator",
        MessageType::Response,
        json!("Found 5 results about Rust async/await patterns"),
    );
    assert_eq!(response.from, "search-agent");

    // 4. Record actions in procedural memory
    let mut proc_mem = ProceduralMemory::new();

    // Simulate repeated workflow: discover → request → store
    for i in 0..4 {
        proc_mem.record(ActionRecord::new(
            "discover",
            "web_search",
            "found agent",
            true,
            5,
        ));
        proc_mem.record(ActionRecord::new(
            "request",
            &format!("query {i}"),
            "results",
            true,
            100,
        ));
        proc_mem.record(ActionRecord::new(
            "store",
            "memory content",
            "stored",
            true,
            10,
        ));
    }

    // 5. Verify pattern detection
    let patterns = proc_mem.detect_patterns(3);
    assert!(!patterns.is_empty());

    // Find the 3-step core workflow pattern
    let workflow = patterns
        .iter()
        .find(|p| p.steps == vec!["discover", "request", "store"])
        .expect("3-step workflow pattern should be detected");
    assert!(workflow.occurrences >= 3);
    assert!(workflow.success_rate() > 0.9);

    // 6. Verify next action suggestion
    let suggestion = proc_mem.suggest_next("discover", 2);
    assert_eq!(suggestion, Some("request".to_string()));

    let suggestion = proc_mem.suggest_next("request", 2);
    assert_eq!(suggestion, Some("store".to_string()));
}

// --- Phase 6 P2 E2E: WASM store with agent memory workflow ---

#[test]
fn e2e_wasm_store_agent_memory_workflow() {
    let mut store = InMemoryStore::new();

    // 1. Store memories from different agent interactions
    let id1 = store.store(
        "User prefers dark mode and vim keybindings",
        &[0.9, 0.1, 0.0, 0.0],
        "preferences",
    );
    let id2 = store.store(
        "Project uses Rust with async/await patterns",
        &[0.1, 0.9, 0.0, 0.0],
        "technical",
    );
    let id3 = store.store(
        "Meeting scheduled for Friday 3pm",
        &[0.0, 0.1, 0.9, 0.0],
        "schedule",
    );
    store.store(
        "User likes concise responses",
        &[0.8, 0.2, 0.0, 0.0],
        "preferences",
    );

    assert_eq!(store.count(), 4);

    // 2. Search by semantic similarity (preference-like query)
    let results = store.search(&[0.85, 0.15, 0.0, 0.0], 2, None);
    assert_eq!(results.len(), 2);
    assert_eq!(
        results[0].content,
        "User prefers dark mode and vim keybindings"
    );
    assert_eq!(results[1].content, "User likes concise responses");

    // 3. Namespace-filtered search
    let tech_results = store.search(&[0.1, 0.9, 0.0, 0.0], 10, Some("technical"));
    assert_eq!(tech_results.len(), 1);
    assert_eq!(tech_results[0].namespace, "technical");

    // 4. Delete outdated memory
    assert!(store.delete(&id3));
    assert_eq!(store.count(), 3);
    assert!(store.get(&id3).is_none());

    // 5. Verify remaining entries
    assert!(store.get(&id1).is_some());
    assert!(store.get(&id2).is_some());
}

// --- Phase 6 P2 E2E: Agent registry lifecycle ---

#[test]
fn e2e_agent_registry_lifecycle() {
    let mut registry = A2ARegistry::new();

    // Register multiple agents
    registry.register(
        AgentCard::new("agent-a", "Agent A", "http://a:3000")
            .capability("search")
            .capability("analyze"),
    );
    registry.register(
        AgentCard::new("agent-b", "Agent B", "http://b:3000")
            .capability("search")
            .capability("store"),
    );
    registry.register(
        AgentCard::new("agent-c", "Agent C", "http://c:3000")
            .capability("store")
            .capability("summarize"),
    );

    // Discover by shared capability
    let search_agents = registry.discover("search");
    assert_eq!(search_agents.len(), 2);

    let store_agents = registry.discover("store");
    assert_eq!(store_agents.len(), 2);

    let summarizers = registry.discover("summarize");
    assert_eq!(summarizers.len(), 1);
    assert_eq!(summarizers[0].name, "agent-c");

    // Unregister an agent
    registry.unregister("agent-b");
    assert_eq!(registry.discover("search").len(), 1);
    assert_eq!(registry.discover("store").len(), 1);

    // Find specific agent
    assert!(registry.find("agent-a").is_some());
    assert!(registry.find("agent-b").is_none());
}

// --- Phase 6 P2 E2E: Procedural memory with failure recovery ---

#[test]
fn e2e_procedural_memory_failure_recovery() {
    let mut mem = ProceduralMemory::new();

    // Simulate workflow with occasional failures
    for i in 0..5 {
        let success = i != 2; // Fail on 3rd iteration
        mem.record(ActionRecord::new("fetch", "url", "data", success, 50));
        mem.record(ActionRecord::new("parse", "data", "parsed", true, 10));
        mem.record(ActionRecord::new("store", "parsed", "ok", true, 5));
    }

    // Tool success rates
    let fetch_rate = mem.tool_success_rate("fetch");
    assert!((fetch_rate - 0.8).abs() < 0.01); // 4/5 success

    let parse_rate = mem.tool_success_rate("parse");
    assert!((parse_rate - 1.0).abs() < f32::EPSILON); // 5/5 success

    // Pattern still detected despite failure
    let patterns = mem.detect_patterns(3);
    assert!(!patterns.is_empty());

    // Find the 3-step core pattern
    let main_pattern = patterns
        .iter()
        .find(|p| p.steps == vec!["fetch", "parse", "store"])
        .expect("3-step pattern should be detected");
    assert!(main_pattern.occurrences >= 3);
    // Successes count only windows where ALL steps succeeded
    assert!(main_pattern.successes < main_pattern.occurrences);
}
