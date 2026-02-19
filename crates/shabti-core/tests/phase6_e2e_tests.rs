use serde_json::json;
use shabti_core::benchmark::{BenchmarkResult, BenchmarkRunner, BenchmarkScenario, MemoryContext};
use shabti_core::health::{ComponentStatus, HealthCheck};
use shabti_core::mcp::protocol::{McpRequest, McpRouter};
use shabti_core::mcp::tools::{build_shabti_server, handle_tool_call};
use shabti_core::metrics::EngineMetrics;

// --- Phase 6 E2E: Full MCP request flow ---

#[test]
fn e2e_mcp_full_session() {
    // Build shabti MCP server
    let server = build_shabti_server();
    let router = McpRouter::new(server);

    // 1. Initialize
    let init_req = McpRequest::from_json(&json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": { "name": "test-client", "version": "1.0" }
        }
    }))
    .unwrap();

    let init_resp = router.handle(&init_req);
    let init_json = init_resp.to_json();
    assert_eq!(init_json["result"]["serverInfo"]["name"], "shabti-memory");
    assert!(init_json["result"]["capabilities"]["tools"].is_object());
    assert!(init_json["result"]["capabilities"]["resources"].is_object());

    // 2. List tools
    let tools_req = McpRequest {
        id: json!(2),
        method: "tools/list".to_string(),
        params: json!({}),
    };
    let tools_resp = router.handle(&tools_req);
    let tools_json = tools_resp.to_json();
    let tools = tools_json["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 3);

    // Verify tool names
    let tool_names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(tool_names.contains(&"memory_store"));
    assert!(tool_names.contains(&"memory_search"));
    assert!(tool_names.contains(&"memory_status"));

    // 3. List resources
    let res_req = McpRequest {
        id: json!(3),
        method: "resources/list".to_string(),
        params: json!({}),
    };
    let res_resp = router.handle(&res_req);
    let res_json = res_resp.to_json();
    let resources = res_json["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 2);
}

#[test]
fn e2e_mcp_tool_call_flow() {
    // Store a memory
    let store_result = handle_tool_call(
        "memory_store",
        &json!({
            "content": "Project deadline is March 15th",
            "namespace": "work",
            "tags": ["deadline", "project"]
        }),
    )
    .unwrap();
    assert!(store_result.success);
    assert_eq!(
        store_result.output["content"],
        "Project deadline is March 15th"
    );

    // Search for it
    let search_result = handle_tool_call(
        "memory_search",
        &json!({ "query": "project deadline", "limit": 5 }),
    )
    .unwrap();
    assert!(search_result.success);
    assert_eq!(search_result.output["query"], "project deadline");

    // Check status
    let status_result = handle_tool_call("memory_status", &json!({})).unwrap();
    assert!(status_result.success);
    assert_eq!(status_result.output["status"], "ok");
}

// --- E2E: Health + metrics integration ---

#[test]
fn e2e_health_with_metrics() {
    let metrics = EngineMetrics::new();

    // Simulate some operations
    metrics.record_store();
    metrics.record_store();
    metrics.record_search(15);
    metrics.record_search(25);
    metrics.record_error();

    // Check metrics
    assert_eq!(metrics.total_requests(), 4);
    assert_eq!(metrics.avg_search_latency_ms(), 20);

    // Health check based on component status
    let health = HealthCheck::new()
        .component("storage", ComponentStatus::Healthy)
        .component("index", ComponentStatus::Healthy)
        .component("embedding", ComponentStatus::Healthy);

    assert!(health.is_healthy());

    // Combine into status report
    let health_json = health.to_json();
    let metrics_json = metrics.to_json();

    assert_eq!(health_json["status"], "healthy");
    assert_eq!(metrics_json["total_requests"], 4);
}

// --- E2E: Benchmark with memory context ---

#[test]
fn e2e_benchmark_memory_improvement() {
    let mut runner = BenchmarkRunner::new();

    // Add scenarios
    runner.add_scenario(BenchmarkScenario::new(
        "meeting-time",
        "What time is the meeting?",
        "3pm",
        vec![MemoryContext::new(
            "The team meeting is at 3pm in Room 201",
            0.95,
        )],
    ));
    runner.add_scenario(BenchmarkScenario::new(
        "project-language",
        "What language is the project written in?",
        "Rust",
        vec![
            MemoryContext::new("The core engine is written in Rust", 0.92),
            MemoryContext::new("Node.js CLI wraps the Rust core via napi-rs", 0.85),
        ],
    ));

    assert_eq!(runner.scenario_count(), 2);

    // Simulate evaluation: without memory gets partial, with memory gets full
    let results = vec![
        BenchmarkResult::new("meeting-time", 0.3, 1.0),
        BenchmarkResult::new("project-language", 0.5, 1.0),
    ];

    let summary = runner.aggregate(&results);
    assert_eq!(summary.total_scenarios, 2);
    assert_eq!(summary.improved_count, 2);
    assert!(summary.avg_improvement > 0.15); // Target: >= 15%
    assert!(summary.avg_score_with_memory > summary.avg_score_without_memory);

    // JSON export
    let json = summary.to_json();
    assert_eq!(json["total_scenarios"], 2);
    assert_eq!(json["improved_count"], 2);
}

// --- E2E: Error handling ---

#[test]
fn e2e_mcp_error_handling() {
    let server = build_shabti_server();
    let router = McpRouter::new(server);

    // Invalid method
    let req = McpRequest {
        id: json!(1),
        method: "invalid/method".to_string(),
        params: json!({}),
    };
    let resp = router.handle(&req);
    let json = resp.to_json();
    assert_eq!(json["error"]["code"], -32601);

    // Invalid tool call
    let result = handle_tool_call("nonexistent", &json!({}));
    assert!(result.is_err());

    // Missing required param
    let result = handle_tool_call("memory_store", &json!({}));
    assert!(result.is_err());
}
