use shabti_core::mcp::{McpResource, McpServer, McpTool, ToolParam};

// --- 6-1a: MCP tool/resource definition model ---

#[test]
fn create_mcp_tool() {
    let tool = McpTool::new(
        "memory_store",
        "Store a memory entry",
        vec![
            ToolParam::required("content", "string", "The text content to store"),
            ToolParam::optional("namespace", "string", "Target namespace"),
        ],
    );

    assert_eq!(tool.name, "memory_store");
    assert_eq!(tool.description, "Store a memory entry");
    assert_eq!(tool.params.len(), 2);
    assert!(tool.params[0].required);
    assert!(!tool.params[1].required);
}

#[test]
fn mcp_tool_generates_input_schema() {
    let tool = McpTool::new(
        "memory_search",
        "Search memories",
        vec![
            ToolParam::required("query", "string", "Search query text"),
            ToolParam::optional("limit", "integer", "Max results"),
        ],
    );

    let schema = tool.input_schema();
    assert_eq!(schema["type"], "object");

    let props = &schema["properties"];
    assert_eq!(props["query"]["type"], "string");
    assert_eq!(props["limit"]["type"], "integer");

    let required = schema["required"].as_array().unwrap();
    assert_eq!(required.len(), 1);
    assert_eq!(required[0], "query");
}

#[test]
fn create_mcp_resource() {
    let resource = McpResource::new(
        "shabti://memories",
        "Memory entries",
        "Access stored memory entries",
        "application/json",
    );

    assert_eq!(resource.uri, "shabti://memories");
    assert_eq!(resource.name, "Memory entries");
    assert_eq!(resource.mime_type, "application/json");
}

#[test]
fn mcp_server_register_tools_and_resources() {
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
    server.register_resource(McpResource::new(
        "shabti://status",
        "Engine status",
        "Current engine status",
        "application/json",
    ));

    assert_eq!(server.tools().len(), 2);
    assert_eq!(server.resources().len(), 1);
    assert_eq!(server.name(), "shabti-memory");
    assert_eq!(server.version(), "0.1.0");
}

#[test]
fn mcp_server_find_tool_by_name() {
    let mut server = McpServer::new("shabti-memory", "0.1.0");

    server.register_tool(McpTool::new("memory_store", "Store", vec![]));
    server.register_tool(McpTool::new("memory_search", "Search", vec![]));

    assert!(server.find_tool("memory_store").is_some());
    assert!(server.find_tool("memory_search").is_some());
    assert!(server.find_tool("nonexistent").is_none());
}

#[test]
fn mcp_server_find_resource_by_uri() {
    let mut server = McpServer::new("shabti-memory", "0.1.0");

    server.register_resource(McpResource::new(
        "shabti://memories",
        "Memories",
        "All memories",
        "application/json",
    ));
    server.register_resource(McpResource::new(
        "shabti://status",
        "Status",
        "Engine status",
        "application/json",
    ));

    assert!(server.find_resource("shabti://memories").is_some());
    assert!(server.find_resource("shabti://status").is_some());
    assert!(server.find_resource("shabti://unknown").is_none());
}

#[test]
fn mcp_tool_to_json() {
    let tool = McpTool::new(
        "memory_store",
        "Store a memory",
        vec![ToolParam::required("content", "string", "The content")],
    );

    let json = tool.to_json();
    assert_eq!(json["name"], "memory_store");
    assert_eq!(json["description"], "Store a memory");
    assert!(json["inputSchema"].is_object());
}

#[test]
fn mcp_resource_to_json() {
    let resource = McpResource::new(
        "shabti://memories",
        "Memory entries",
        "All stored memories",
        "application/json",
    );

    let json = resource.to_json();
    assert_eq!(json["uri"], "shabti://memories");
    assert_eq!(json["name"], "Memory entries");
    assert_eq!(json["mimeType"], "application/json");
}

#[test]
fn mcp_server_capabilities_json() {
    let mut server = McpServer::new("shabti-memory", "0.1.0");
    server.register_tool(McpTool::new("memory_store", "Store", vec![]));
    server.register_resource(McpResource::new(
        "shabti://status",
        "Status",
        "Status",
        "application/json",
    ));

    let caps = server.capabilities();
    assert!(caps["tools"].is_object());
    assert!(caps["resources"].is_object());
}
