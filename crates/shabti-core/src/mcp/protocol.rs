use serde_json::{Value, json};

use super::McpServer;

/// A parsed MCP JSON-RPC request.
#[derive(Debug, Clone)]
pub struct McpRequest {
    pub id: Value,
    pub method: String,
    pub params: Value,
}

impl McpRequest {
    /// Parse a JSON-RPC 2.0 request from a JSON value.
    pub fn from_json(raw: &Value) -> Result<Self, String> {
        let method = raw
            .get("method")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing 'method' field".to_string())?;

        let id = raw.get("id").cloned().unwrap_or(Value::Null);
        let params = raw.get("params").cloned().unwrap_or(json!({}));

        Ok(Self {
            id,
            method: method.to_string(),
            params,
        })
    }
}

/// An MCP JSON-RPC response.
#[derive(Debug, Clone)]
pub struct McpResponse {
    id: Value,
    result: Option<Value>,
    error: Option<Value>,
}

impl McpResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, code: i32, message: &str) -> Self {
        Self {
            id,
            result: None,
            error: Some(json!({
                "code": code,
                "message": message,
            })),
        }
    }

    pub fn to_json(&self) -> Value {
        let mut obj = serde_json::Map::new();
        obj.insert("jsonrpc".to_string(), json!("2.0"));
        obj.insert("id".to_string(), self.id.clone());

        if let Some(ref result) = self.result {
            obj.insert("result".to_string(), result.clone());
        }
        if let Some(ref error) = self.error {
            obj.insert("error".to_string(), error.clone());
        }

        Value::Object(obj)
    }
}

/// Routes MCP requests to the appropriate handler.
pub struct McpRouter {
    server: McpServer,
}

impl McpRouter {
    pub fn new(server: McpServer) -> Self {
        Self { server }
    }

    pub fn handle(&self, req: &McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "tools/list" => self.handle_tools_list(req),
            "resources/list" => self.handle_resources_list(req),
            _ => McpResponse::error(req.id.clone(), -32601, "Method not found"),
        }
    }

    fn handle_initialize(&self, req: &McpRequest) -> McpResponse {
        McpResponse::success(
            req.id.clone(),
            json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": self.server.name(),
                    "version": self.server.version(),
                },
                "capabilities": self.server.capabilities(),
            }),
        )
    }

    fn handle_tools_list(&self, req: &McpRequest) -> McpResponse {
        let tools: Vec<Value> = self.server.tools().iter().map(|t| t.to_json()).collect();
        McpResponse::success(req.id.clone(), json!({ "tools": tools }))
    }

    fn handle_resources_list(&self, req: &McpRequest) -> McpResponse {
        let resources: Vec<Value> = self
            .server
            .resources()
            .iter()
            .map(|r| r.to_json())
            .collect();
        McpResponse::success(req.id.clone(), json!({ "resources": resources }))
    }
}
