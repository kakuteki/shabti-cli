pub mod protocol;
pub mod tools;

use serde_json::{Value, json};

/// A parameter definition for an MCP tool.
#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

impl ToolParam {
    pub fn required(name: &str, param_type: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            param_type: param_type.to_string(),
            description: description.to_string(),
            required: true,
        }
    }

    pub fn optional(name: &str, param_type: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            param_type: param_type.to_string(),
            description: description.to_string(),
            required: false,
        }
    }
}

/// An MCP tool definition.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub params: Vec<ToolParam>,
}

impl McpTool {
    pub fn new(name: &str, description: &str, params: Vec<ToolParam>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            params,
        }
    }

    /// Generate the JSON Schema for this tool's input.
    pub fn input_schema(&self) -> Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.params {
            properties.insert(
                param.name.clone(),
                json!({
                    "type": param.param_type,
                    "description": param.description,
                }),
            );
            if param.required {
                required.push(Value::String(param.name.clone()));
            }
        }

        json!({
            "type": "object",
            "properties": properties,
            "required": required,
        })
    }

    /// Serialize tool definition to JSON (MCP format).
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema(),
        })
    }
}

/// An MCP resource definition.
#[derive(Debug, Clone)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    pub mime_type: String,
}

impl McpResource {
    pub fn new(uri: &str, name: &str, description: &str, mime_type: &str) -> Self {
        Self {
            uri: uri.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            mime_type: mime_type.to_string(),
        }
    }

    /// Serialize resource definition to JSON (MCP format).
    pub fn to_json(&self) -> Value {
        json!({
            "uri": self.uri,
            "name": self.name,
            "description": self.description,
            "mimeType": self.mime_type,
        })
    }
}

/// MCP server that registers tools and resources.
pub struct McpServer {
    name: String,
    version: String,
    tools: Vec<McpTool>,
    resources: Vec<McpResource>,
}

impl McpServer {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            tools: Vec::new(),
            resources: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn register_tool(&mut self, tool: McpTool) {
        self.tools.push(tool);
    }

    pub fn register_resource(&mut self, resource: McpResource) {
        self.resources.push(resource);
    }

    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    pub fn resources(&self) -> &[McpResource] {
        &self.resources
    }

    pub fn find_tool(&self, name: &str) -> Option<&McpTool> {
        self.tools.iter().find(|t| t.name == name)
    }

    pub fn find_resource(&self, uri: &str) -> Option<&McpResource> {
        self.resources.iter().find(|r| r.uri == uri)
    }

    /// Generate server capabilities JSON (for initialize response).
    pub fn capabilities(&self) -> Value {
        let mut caps = serde_json::Map::new();

        if !self.tools.is_empty() {
            caps.insert("tools".to_string(), json!({}));
        }
        if !self.resources.is_empty() {
            caps.insert("resources".to_string(), json!({}));
        }

        Value::Object(caps)
    }
}
