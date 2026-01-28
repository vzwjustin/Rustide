//! Protocol types for LSP communication.
//!
//! This module re-exports lsp-types and provides custom message wrappers
//! for JSON-RPC communication with language servers.

pub use lsp_types::*;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC protocol version.
pub const JSONRPC_VERSION: &str = "2.0";

/// A JSON-RPC request ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    /// Numeric ID.
    Number(i64),
    /// String ID.
    String(String),
}

impl From<i64> for RequestId {
    fn from(id: i64) -> Self {
        RequestId::Number(id)
    }
}

impl From<String> for RequestId {
    fn from(id: String) -> Self {
        RequestId::String(id)
    }
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::Number(n) => write!(f, "{}", n),
            RequestId::String(s) => write!(f, "{}", s),
        }
    }
}

/// A JSON-RPC request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,
    /// Request ID.
    pub id: RequestId,
    /// Method name.
    pub method: String,
    /// Optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request.
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

/// A JSON-RPC response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,
    /// Request ID this is responding to.
    pub id: RequestId,
    /// Result on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a successful response.
    pub fn success(id: impl Into<RequestId>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: impl Into<RequestId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }

    /// Check if this is a success response.
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    /// Check if this is an error response.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

/// A JSON-RPC error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i64,
    /// Error message.
    pub message: String,
    /// Optional additional data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new JSON-RPC error.
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Create a new JSON-RPC error with data.
    pub fn with_data(code: i64, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }

    /// Parse error (-32700).
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(-32700, message)
    }

    /// Invalid request (-32600).
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(-32600, message)
    }

    /// Method not found (-32601).
    pub fn method_not_found(message: impl Into<String>) -> Self {
        Self::new(-32601, message)
    }

    /// Invalid params (-32602).
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message)
    }

    /// Internal error (-32603).
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(-32603, message)
    }

    /// Server not initialized (-32002).
    pub fn server_not_initialized() -> Self {
        Self::new(-32002, "Server not initialized")
    }

    /// Request cancelled (-32800).
    pub fn request_cancelled() -> Self {
        Self::new(-32800, "Request cancelled")
    }

    /// Content modified (-32801).
    pub fn content_modified() -> Self {
        Self::new(-32801, "Content modified")
    }
}

impl std::fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

/// A JSON-RPC notification message (no ID, no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,
    /// Method name.
    pub method: String,
    /// Optional parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Create a new notification.
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

/// An incoming JSON-RPC message (request, response, or notification).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    /// A request message.
    Request(JsonRpcRequest),
    /// A response message.
    Response(JsonRpcResponse),
    /// A notification message.
    Notification(JsonRpcNotification),
}

impl JsonRpcMessage {
    /// Try to parse a message from a JSON value.
    pub fn from_value(value: Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }

    /// Convert the message to a JSON value.
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Check if this is a request.
    pub fn is_request(&self) -> bool {
        matches!(self, JsonRpcMessage::Request(_))
    }

    /// Check if this is a response.
    pub fn is_response(&self) -> bool {
        matches!(self, JsonRpcMessage::Response(_))
    }

    /// Check if this is a notification.
    pub fn is_notification(&self) -> bool {
        matches!(self, JsonRpcMessage::Notification(_))
    }
}

/// LSP message wrapper that includes the raw content.
#[derive(Debug, Clone)]
pub struct LspMessage {
    /// The parsed JSON-RPC message.
    pub message: JsonRpcMessage,
    /// The raw JSON content.
    pub raw: String,
}

impl LspMessage {
    /// Create a new LSP message from a JSON-RPC message.
    pub fn new(message: JsonRpcMessage) -> Result<Self, serde_json::Error> {
        let raw = serde_json::to_string(&message)?;
        Ok(Self { message, raw })
    }

    /// Parse an LSP message from raw JSON.
    pub fn parse(raw: &str) -> Result<Self, serde_json::Error> {
        let value: Value = serde_json::from_str(raw)?;
        let message = JsonRpcMessage::from_value(value)?;
        Ok(Self {
            message,
            raw: raw.to_string(),
        })
    }

    /// Get the content-length header value.
    pub fn content_length(&self) -> usize {
        self.raw.len()
    }

    /// Format as LSP wire protocol (with headers).
    pub fn to_wire_format(&self) -> String {
        format!("Content-Length: {}\r\n\r\n{}", self.content_length(), self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let request = JsonRpcRequest::new(
            1i64,
            "initialize",
            Some(serde_json::json!({"processId": 1234})),
        );
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"id\":1"));
        assert!(json.contains("\"method\":\"initialize\""));
    }

    #[test]
    fn test_response_success() {
        let response = JsonRpcResponse::success(1i64, serde_json::json!({"capabilities": {}}));
        assert!(response.is_success());
        assert!(!response.is_error());
    }

    #[test]
    fn test_response_error() {
        let response = JsonRpcResponse::error(1i64, JsonRpcError::method_not_found("unknown"));
        assert!(!response.is_success());
        assert!(response.is_error());
    }

    #[test]
    fn test_notification() {
        let notification = JsonRpcNotification::new(
            "textDocument/didOpen",
            Some(serde_json::json!({"textDocument": {}})),
        );
        let json = serde_json::to_string(&notification).unwrap();
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_lsp_message_wire_format() {
        let request = JsonRpcRequest::new(1i64, "test", None);
        let message = LspMessage::new(JsonRpcMessage::Request(request)).unwrap();
        let wire = message.to_wire_format();
        assert!(wire.starts_with("Content-Length:"));
        assert!(wire.contains("\r\n\r\n"));
    }
}
