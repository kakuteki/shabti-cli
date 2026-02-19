use serde_json::{Value, json};

use super::{McpResource, McpServer, McpTool, ToolParam};

/// Result of a tool call execution.
#[derive(Debug, Clone)]
pub struct ToolCallResult {
    pub success: bool,
    pub output: Value,
}

impl ToolCallResult {
    /// Convert to MCP content array format.
    pub fn to_mcp_content(&self) -> Vec<Value> {
        vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&self.output).unwrap_or_default(),
        })]
    }
}

/// Build the shabti MCP server with all tools and resources registered.
pub fn build_shabti_server() -> McpServer {
    let mut server = McpServer::new("shabti-memory", "0.1.0");

    // Tools
    server.register_tool(McpTool::new(
        "memory_store",
        "Store a memory entry in shabti",
        vec![
            ToolParam::required("content", "string", "The text content to store"),
            ToolParam::optional(
                "namespace",
                "string",
                "Target namespace (default: 'default')",
            ),
            ToolParam::optional("tags", "array", "Tags to associate with the memory"),
        ],
    ));

    server.register_tool(McpTool::new(
        "memory_search",
        "Search stored memories by semantic similarity",
        vec![
            ToolParam::required("query", "string", "Search query text"),
            ToolParam::optional(
                "limit",
                "integer",
                "Maximum number of results (default: 10)",
            ),
            ToolParam::optional("namespace", "string", "Namespace to search within"),
        ],
    ));

    server.register_tool(McpTool::new(
        "memory_status",
        "Get the current status of the memory engine",
        vec![],
    ));

    // Resources
    server.register_resource(McpResource::new(
        "shabti://status",
        "Engine status",
        "Current shabti engine status and statistics",
        "application/json",
    ));

    server.register_resource(McpResource::new(
        "shabti://config",
        "Engine configuration",
        "Current engine configuration",
        "application/json",
    ));

    server
}

/// Handle a tool call by name with the given arguments.
/// Returns a ToolCallResult on success, or an error string.
pub fn handle_tool_call(tool_name: &str, arguments: &Value) -> Result<ToolCallResult, String> {
    match tool_name {
        "memory_store" => handle_memory_store(arguments),
        "memory_search" => handle_memory_search(arguments),
        "memory_status" => handle_memory_status(arguments),
        _ => Err(format!("unknown tool: {tool_name}")),
    }
}

fn handle_memory_store(args: &Value) -> Result<ToolCallResult, String> {
    let content = args
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing required parameter: content".to_string())?;

    let namespace = args
        .get("namespace")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    Ok(ToolCallResult {
        success: true,
        output: json!({
            "stored": true,
            "content": content,
            "namespace": namespace,
        }),
    })
}

fn handle_memory_search(args: &Value) -> Result<ToolCallResult, String> {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing required parameter: query".to_string())?;

    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);

    let namespace = args.get("namespace").and_then(|v| v.as_str());

    let mut output = json!({
        "query": query,
        "limit": limit,
        "results": [],
    });

    if let Some(ns) = namespace {
        output["namespace"] = json!(ns);
    }

    Ok(ToolCallResult {
        success: true,
        output,
    })
}

fn handle_memory_status(_args: &Value) -> Result<ToolCallResult, String> {
    Ok(ToolCallResult {
        success: true,
        output: json!({
            "status": "ok",
            "entry_count": 0,
            "model_id": "unknown",
        }),
    })
}
