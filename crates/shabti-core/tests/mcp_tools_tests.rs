use serde_json::json;
use shabti_core::mcp::tools::{build_shabti_server, handle_tool_call, ToolCallResult};

// --- 6-1c: MCP shabti tool implementations ---

#[test]
fn shabti_server_has_expected_tools() {
    let server = build_shabti_server();

    assert!(server.find_tool("memory_store").is_some());
    assert!(server.find_tool("memory_search").is_some());
    assert!(server.find_tool("memory_status").is_some());
    assert_eq!(server.tools().len(), 3);
}

#[test]
fn shabti_server_has_expected_resources() {
    let server = build_shabti_server();

    assert!(server.find_resource("shabti://status").is_some());
    assert!(server.find_resource("shabti://config").is_some());
    assert_eq!(server.resources().len(), 2);
}

#[test]
fn memory_store_tool_params() {
    let server = build_shabti_server();
    let tool = server.find_tool("memory_store").unwrap();

    let schema = tool.input_schema();
    assert!(schema["properties"]["content"].is_object());
    assert!(schema["properties"]["namespace"].is_object());
    assert!(schema["properties"]["tags"].is_object());

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("content")));
    assert!(!required.contains(&json!("namespace")));
}

#[test]
fn memory_search_tool_params() {
    let server = build_shabti_server();
    let tool = server.find_tool("memory_search").unwrap();

    let schema = tool.input_schema();
    assert!(schema["properties"]["query"].is_object());
    assert!(schema["properties"]["limit"].is_object());
    assert!(schema["properties"]["namespace"].is_object());

    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&json!("query")));
}

#[test]
fn memory_status_tool_has_no_required_params() {
    let server = build_shabti_server();
    let tool = server.find_tool("memory_status").unwrap();

    let schema = tool.input_schema();
    let required = schema["required"].as_array().unwrap();
    assert!(required.is_empty());
}

#[test]
fn handle_tool_call_memory_store() {
    let result = handle_tool_call(
        "memory_store",
        &json!({ "content": "The meeting is at 3pm", "namespace": "work" }),
    );

    assert!(result.is_ok());
    let call_result = result.unwrap();
    assert!(call_result.success);
    assert!(call_result.output["stored"].is_boolean());
    assert_eq!(call_result.output["content"], "The meeting is at 3pm");
    assert_eq!(call_result.output["namespace"], "work");
}

#[test]
fn handle_tool_call_memory_store_default_namespace() {
    let result = handle_tool_call("memory_store", &json!({ "content": "Hello" }));

    assert!(result.is_ok());
    let call_result = result.unwrap();
    assert!(call_result.success);
    assert_eq!(call_result.output["namespace"], "default");
}

#[test]
fn handle_tool_call_memory_store_missing_content() {
    let result = handle_tool_call("memory_store", &json!({}));

    assert!(result.is_err());
}

#[test]
fn handle_tool_call_memory_search() {
    let result = handle_tool_call(
        "memory_search",
        &json!({ "query": "meeting time", "limit": 5 }),
    );

    assert!(result.is_ok());
    let call_result = result.unwrap();
    assert!(call_result.success);
    assert_eq!(call_result.output["query"], "meeting time");
    assert_eq!(call_result.output["limit"], 5);
}

#[test]
fn handle_tool_call_memory_search_defaults() {
    let result = handle_tool_call("memory_search", &json!({ "query": "test" }));

    assert!(result.is_ok());
    let call_result = result.unwrap();
    assert_eq!(call_result.output["limit"], 10);
}

#[test]
fn handle_tool_call_memory_status() {
    let result = handle_tool_call("memory_status", &json!({}));

    assert!(result.is_ok());
    let call_result = result.unwrap();
    assert!(call_result.success);
    assert!(call_result.output["status"].is_string());
}

#[test]
fn handle_tool_call_unknown_tool() {
    let result = handle_tool_call("nonexistent_tool", &json!({}));
    assert!(result.is_err());
}

#[test]
fn tool_call_result_to_mcp_content() {
    let result = ToolCallResult {
        success: true,
        output: json!({"key": "value"}),
    };

    let content = result.to_mcp_content();
    assert_eq!(content[0]["type"], "text");
    assert!(content[0]["text"].as_str().unwrap().contains("key"));
}
