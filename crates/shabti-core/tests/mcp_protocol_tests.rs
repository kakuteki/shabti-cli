use serde_json::json;
use shabti_core::mcp::protocol::{McpRequest, McpResponse, McpRouter};
use shabti_core::mcp::{McpResource, McpServer, McpTool, ToolParam};

// --- 6-1b: MCP JSON-RPC protocol ---

#[test]
fn parse_initialize_request() {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": { "name": "test-client", "version": "1.0" }
        }
    });

    let req = McpRequest::from_json(&raw).unwrap();
    assert_eq!(req.method, "initialize");
    assert_eq!(req.id, json!(1));
}

#[test]
fn parse_tools_list_request() {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let req = McpRequest::from_json(&raw).unwrap();
    assert_eq!(req.method, "tools/list");
}

#[test]
fn parse_tools_call_request() {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "memory_store",
            "arguments": { "content": "Hello world" }
        }
    });

    let req = McpRequest::from_json(&raw).unwrap();
    assert_eq!(req.method, "tools/call");
    assert_eq!(req.params["name"], "memory_store");
    assert_eq!(req.params["arguments"]["content"], "Hello world");
}

#[test]
fn parse_invalid_request_missing_method() {
    let raw = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "params": {}
    });

    assert!(McpRequest::from_json(&raw).is_err());
}

#[test]
fn response_success_format() {
    let resp = McpResponse::success(json!(1), json!({"tools": []}));
    let output = resp.to_json();

    assert_eq!(output["jsonrpc"], "2.0");
    assert_eq!(output["id"], 1);
    assert!(output["result"].is_object());
    assert!(output.get("error").is_none());
}

#[test]
fn response_error_format() {
    let resp = McpResponse::error(json!(1), -32601, "Method not found");
    let output = resp.to_json();

    assert_eq!(output["jsonrpc"], "2.0");
    assert_eq!(output["id"], 1);
    assert_eq!(output["error"]["code"], -32601);
    assert_eq!(output["error"]["message"], "Method not found");
    assert!(output.get("result").is_none());
}

#[test]
fn router_handles_initialize() {
    let server = McpServer::new("shabti-memory", "0.1.0");
    let router = McpRouter::new(server);

    let req = McpRequest {
        id: json!(1),
        method: "initialize".to_string(),
        params: json!({
            "protocolVersion": "2024-11-05",
            "clientInfo": { "name": "test", "version": "1.0" }
        }),
    };

    let resp = router.handle(&req);
    let output = resp.to_json();
    assert_eq!(output["result"]["serverInfo"]["name"], "shabti-memory");
    assert_eq!(output["result"]["protocolVersion"], "2024-11-05");
    assert!(output["result"]["capabilities"].is_object());
}

#[test]
fn router_handles_tools_list() {
    let mut server = McpServer::new("shabti-memory", "0.1.0");
    server.register_tool(McpTool::new(
        "memory_store",
        "Store a memory",
        vec![ToolParam::required("content", "string", "Content")],
    ));
    server.register_tool(McpTool::new(
        "memory_search",
        "Search memories",
        vec![ToolParam::required("query", "string", "Query")],
    ));

    let router = McpRouter::new(server);
    let req = McpRequest {
        id: json!(2),
        method: "tools/list".to_string(),
        params: json!({}),
    };

    let resp = router.handle(&req);
    let output = resp.to_json();
    let tools = output["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 2);
    assert_eq!(tools[0]["name"], "memory_store");
    assert_eq!(tools[1]["name"], "memory_search");
}

#[test]
fn router_handles_resources_list() {
    let mut server = McpServer::new("shabti-memory", "0.1.0");
    server.register_resource(McpResource::new(
        "shabti://status",
        "Status",
        "Engine status",
        "application/json",
    ));

    let router = McpRouter::new(server);
    let req = McpRequest {
        id: json!(3),
        method: "resources/list".to_string(),
        params: json!({}),
    };

    let resp = router.handle(&req);
    let output = resp.to_json();
    let resources = output["result"]["resources"].as_array().unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0]["uri"], "shabti://status");
}

#[test]
fn router_returns_error_for_unknown_method() {
    let server = McpServer::new("shabti-memory", "0.1.0");
    let router = McpRouter::new(server);

    let req = McpRequest {
        id: json!(99),
        method: "unknown/method".to_string(),
        params: json!({}),
    };

    let resp = router.handle(&req);
    let output = resp.to_json();
    assert_eq!(output["error"]["code"], -32601);
}
