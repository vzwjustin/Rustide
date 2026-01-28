//! Agent tools - Functions that agents can execute

use anyhow::Result;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

/// Errors that can occur when using tools
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout")]
    Timeout,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the execution succeeded
    pub success: bool,
    /// Result value (if successful)
    pub value: Option<Value>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Metadata about the execution
    pub metadata: HashMap<String, Value>,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(value: Value) -> Self {
        Self {
            success: true,
            value: Some(value),
            error: None,
            execution_time_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed result
    pub fn failure(error: &str) -> Self {
        Self {
            success: false,
            value: None,
            error: Some(error.to_string()),
            execution_time_ms: 0,
            metadata: HashMap::new(),
        }
    }

    /// Set execution time
    pub fn with_time(mut self, ms: u64) -> Self {
        self.execution_time_ms = ms;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

/// Parameter definition for a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type (string, number, boolean, object, array)
    pub param_type: String,
    /// Description
    pub description: String,
    /// Whether required
    pub required: bool,
    /// Default value
    pub default: Option<Value>,
    /// Enum values (if constrained)
    pub enum_values: Option<Vec<Value>>,
}

impl ToolParameter {
    /// Create a new parameter
    pub fn new(name: &str, param_type: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            param_type: param_type.to_string(),
            description: description.to_string(),
            required: false,
            default: None,
            enum_values: None,
        }
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set default value
    pub fn with_default(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    /// Set enum values
    pub fn with_enum(mut self, values: Vec<Value>) -> Self {
        self.enum_values = Some(values);
        self
    }
}

/// A tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this call
    pub id: Uuid,
    /// Tool name
    pub tool: String,
    /// Parameters
    pub parameters: HashMap<String, Value>,
    /// Calling agent ID
    pub caller_id: Option<Uuid>,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(tool: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            tool: tool.to_string(),
            parameters: HashMap::new(),
            caller_id: None,
        }
    }

    /// Add a parameter
    pub fn param(mut self, name: &str, value: Value) -> Self {
        self.parameters.insert(name.to_string(), value);
        self
    }

    /// Set caller ID
    pub fn from_agent(mut self, agent_id: Uuid) -> Self {
        self.caller_id = Some(agent_id);
        self
    }

    /// Get a parameter
    pub fn get_param(&self, name: &str) -> Option<&Value> {
        self.parameters.get(name)
    }

    /// Get a string parameter
    pub fn get_string(&self, name: &str) -> Option<&str> {
        self.parameters.get(name).and_then(|v| v.as_str())
    }

    /// Get a number parameter
    pub fn get_number(&self, name: &str) -> Option<f64> {
        self.parameters.get(name).and_then(|v| v.as_f64())
    }

    /// Get a boolean parameter
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.parameters.get(name).and_then(|v| v.as_bool())
    }
}

/// Type alias for async tool handler
pub type ToolHandler = Box<
    dyn Fn(ToolCall) -> Pin<Box<dyn Future<Output = ToolResult> + Send>> + Send + Sync,
>;

/// A tool definition
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Description
    pub description: String,
    /// Parameters
    pub parameters: Vec<ToolParameter>,
    /// Handler function
    handler: ToolHandler,
    /// Required permissions
    pub permissions: Vec<String>,
}

impl Tool {
    /// Create a new tool
    pub fn new<F, Fut>(name: &str, description: &str, handler: F) -> Self
    where
        F: Fn(ToolCall) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ToolResult> + Send + 'static,
    {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters: Vec::new(),
            handler: Box::new(move |call| Box::pin(handler(call))),
            permissions: Vec::new(),
        }
    }

    /// Add a parameter
    pub fn param(mut self, param: ToolParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add a permission requirement
    pub fn requires_permission(mut self, permission: &str) -> Self {
        self.permissions.push(permission.to_string());
        self
    }

    /// Validate parameters
    pub fn validate(&self, call: &ToolCall) -> Result<(), ToolError> {
        for param in &self.parameters {
            if param.required && !call.parameters.contains_key(&param.name) {
                return Err(ToolError::InvalidParameters(format!(
                    "Missing required parameter: {}",
                    param.name
                )));
            }

            // Validate enum values if specified
            if let (Some(value), Some(enum_values)) = (
                call.parameters.get(&param.name),
                &param.enum_values,
            ) {
                if !enum_values.contains(value) {
                    return Err(ToolError::InvalidParameters(format!(
                        "Invalid value for {}: must be one of {:?}",
                        param.name, enum_values
                    )));
                }
            }
        }
        Ok(())
    }

    /// Execute the tool
    pub async fn execute(&self, call: ToolCall) -> ToolResult {
        // Validate first
        if let Err(e) = self.validate(&call) {
            return ToolResult::failure(&e.to_string());
        }

        let start = std::time::Instant::now();
        let result = (self.handler)(call).await;
        let elapsed = start.elapsed().as_millis() as u64;

        result.with_time(elapsed)
    }

    /// Get JSON schema for this tool
    pub fn schema(&self) -> Value {
        let params: HashMap<String, Value> = self
            .parameters
            .iter()
            .map(|p| {
                (
                    p.name.clone(),
                    serde_json::json!({
                        "type": p.param_type,
                        "description": p.description,
                    }),
                )
            })
            .collect();

        let required: Vec<&str> = self
            .parameters
            .iter()
            .filter(|p| p.required)
            .map(|p| p.name.as_str())
            .collect();

        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "parameters": {
                "type": "object",
                "properties": params,
                "required": required,
            }
        })
    }
}

/// Registry of available tools
pub struct ToolRegistry {
    tools: RwLock<HashMap<String, Arc<Tool>>>,
}

impl ToolRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Register a tool
    pub fn register(&self, tool: Tool) {
        self.tools.write().insert(tool.name.clone(), Arc::new(tool));
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<Tool>> {
        self.tools.read().get(name).cloned()
    }

    /// Check if a tool exists
    pub fn has(&self, name: &str) -> bool {
        self.tools.read().contains_key(name)
    }

    /// Get all tool names
    pub fn list(&self) -> Vec<String> {
        self.tools.read().keys().cloned().collect()
    }

    /// Get schemas for all tools
    pub fn schemas(&self) -> Vec<Value> {
        self.tools.read().values().map(|t| t.schema()).collect()
    }

    /// Execute a tool call
    pub async fn execute(&self, call: ToolCall) -> Result<ToolResult, ToolError> {
        let tool = self.get(&call.tool)
            .ok_or_else(|| ToolError::NotFound(call.tool.clone()))?;

        Ok(tool.execute(call).await)
    }

    /// Remove a tool
    pub fn unregister(&self, name: &str) -> bool {
        self.tools.write().remove(name).is_some()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create standard IDE tools
pub fn create_standard_tools() -> ToolRegistry {
    let registry = ToolRegistry::new();

    // Read file tool
    registry.register(
        Tool::new("read_file", "Read contents of a file", |call| async move {
            let path = match call.get_string("path") {
                Some(p) => p,
                None => return ToolResult::failure("Missing path parameter"),
            };
            
            match tokio::fs::read_to_string(path).await {
                Ok(content) => ToolResult::success(serde_json::json!({
                    "content": content,
                    "path": path,
                })),
                Err(e) => ToolResult::failure(&format!("Failed to read file: {}", e)),
            }
        })
        .param(ToolParameter::new("path", "string", "Path to the file").required())
    );

    // Write file tool
    registry.register(
        Tool::new("write_file", "Write contents to a file", |call| async move {
            let path = match call.get_string("path") {
                Some(p) => p.to_string(),
                None => return ToolResult::failure("Missing path parameter"),
            };
            let content = match call.get_string("content") {
                Some(c) => c.to_string(),
                None => return ToolResult::failure("Missing content parameter"),
            };
            
            match tokio::fs::write(&path, &content).await {
                Ok(()) => ToolResult::success(serde_json::json!({
                    "path": path,
                    "bytes_written": content.len(),
                })),
                Err(e) => ToolResult::failure(&format!("Failed to write file: {}", e)),
            }
        })
        .param(ToolParameter::new("path", "string", "Path to the file").required())
        .param(ToolParameter::new("content", "string", "Content to write").required())
        .requires_permission("file:write")
    );

    // List files tool
    registry.register(
        Tool::new("list_files", "List files in a directory", |call| async move {
            let path = call.get_string("path").unwrap_or(".");
            
            match tokio::fs::read_dir(path).await {
                Ok(mut entries) => {
                    let mut files = Vec::new();
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        if let Ok(name) = entry.file_name().into_string() {
                            files.push(name);
                        }
                    }
                    ToolResult::success(serde_json::json!({
                        "path": path,
                        "files": files,
                    }))
                }
                Err(e) => ToolResult::failure(&format!("Failed to list directory: {}", e)),
            }
        })
        .param(ToolParameter::new("path", "string", "Directory path").with_default(serde_json::json!(".")))
    );

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_result() {
        let result = ToolResult::success(serde_json::json!({"key": "value"}))
            .with_time(100)
            .with_metadata("source", serde_json::json!("test"));

        assert!(result.success);
        assert_eq!(result.execution_time_ms, 100);
    }

    #[test]
    fn test_tool_call() {
        let call = ToolCall::new("test_tool")
            .param("name", serde_json::json!("test"))
            .param("count", serde_json::json!(42));

        assert_eq!(call.tool, "test_tool");
        assert_eq!(call.get_string("name"), Some("test"));
        assert_eq!(call.get_number("count"), Some(42.0));
    }

    #[test]
    fn test_tool_parameter() {
        let param = ToolParameter::new("name", "string", "User name")
            .required()
            .with_default(serde_json::json!("anonymous"));

        assert!(param.required);
        assert!(param.default.is_some());
    }

    #[tokio::test]
    async fn test_tool_execution() {
        let tool = Tool::new("echo", "Echo the input", |call| async move {
            let input = call.get_string("input").unwrap_or("no input");
            ToolResult::success(serde_json::json!({"echo": input}))
        })
        .param(ToolParameter::new("input", "string", "Input to echo"));

        let call = ToolCall::new("echo").param("input", serde_json::json!("hello"));
        let result = tool.execute(call).await;

        assert!(result.success);
    }

    #[test]
    fn test_tool_registry() {
        let registry = ToolRegistry::new();

        registry.register(Tool::new("test", "A test tool", |_| async {
            ToolResult::success(serde_json::json!({}))
        }));

        assert!(registry.has("test"));
        assert_eq!(registry.list().len(), 1);
    }

    #[test]
    fn test_tool_schema() {
        let tool = Tool::new("test", "Test tool", |_| async {
            ToolResult::success(serde_json::json!({}))
        })
        .param(ToolParameter::new("required_param", "string", "A required param").required())
        .param(ToolParameter::new("optional_param", "number", "An optional param"));

        let schema = tool.schema();
        assert_eq!(schema["name"], "test");
    }
}
