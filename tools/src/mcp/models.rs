//! MCP Models and Data Structures
//!
//! This module contains the data structures for MCP protocol communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// MCP Server information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerInfo {
    /// Unique identifier for the server
    pub id: Uuid,
    /// Server name
    pub name: String,
    /// Server description
    pub description: String,
    /// Server version
    pub version: String,
    /// Connection endpoint (URL or command)
    pub endpoint: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Connection status
    pub status: ConnectionStatus,
    /// Last connection attempt
    pub last_connected: Option<DateTime<Utc>>,
    /// Server metadata
    pub metadata: HashMap<String, String>,
}

/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerCapabilities {
    /// List of available tools
    pub tools: Vec<ToolCapability>,
    /// List of available resources
    pub resources: Vec<ResourceCapability>,
    /// List of available prompts
    pub prompts: Vec<PromptCapability>,
    /// Logging support
    pub logging: bool,
    /// Sampling support
    pub sampling: bool,
}

/// Tool capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCapability {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema
    pub input_schema: serde_json::Value,
    /// Output schema
    pub output_schema: Option<serde_json::Value>,
}

/// Resource capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceCapability {
    /// Resource URI template
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    pub description: String,
    /// MIME type
    pub mime_type: Option<String>,
}

/// Prompt capability
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptCapability {
    /// Prompt name
    pub name: String,
    /// Prompt description
    pub description: String,
    /// Arguments schema
    pub arguments: Vec<PromptArgument>,
}

/// Prompt argument
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    pub description: String,
    /// Whether argument is required
    pub required: bool,
    /// Argument type
    pub r#type: String,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    /// Not connected
    Disconnected,
    /// Connecting
    Connecting,
    /// Connected
    Connected,
    /// Connection failed
    Failed(String),
    /// Authentication failed
    AuthenticationFailed(String),
}

/// MCP Request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: Option<serde_json::Value>,
    /// Method name
    pub method: String,
    /// Method parameters
    pub params: Option<serde_json::Value>,
}

/// MCP Response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: serde_json::Value,
    /// Result (if successful)
    pub result: Option<serde_json::Value>,
    /// Error (if failed)
    pub error: Option<McpError>,
}

/// MCP Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Error data
    pub data: Option<serde_json::Value>,
}

/// Tool call request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    /// Tool name
    pub name: String,
    /// Tool arguments
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Tool call response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResponse {
    /// Tool output
    pub content: Vec<ToolContent>,
    /// Whether the call was successful
    pub is_error: bool,
}

/// Tool content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource {
        resource: ResourceReference,
        annotations: Option<HashMap<String, String>>,
    },
}

/// Resource reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    /// Resource URI
    pub uri: String,
    /// MIME type
    pub mime_type: Option<String>,
    /// Resource text
    pub text: Option<String>,
    /// Resource blob
    pub blob: Option<String>,
}

/// Initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    /// Protocol version
    pub protocol_version: String,
    /// Client capabilities
    pub capabilities: ClientCapabilities,
    /// Client info
    pub client_info: ClientInfo,
}

/// Client capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Root URIs
    pub roots: Option<RootCapability>,
    /// Sampling capability
    pub sampling: Option<SamplingCapability>,
}

/// Root capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootCapability {
    /// List of root URIs
    pub root_uris: Vec<String>,
}

/// Sampling capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {}

/// Client info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client name
    pub name: String,
    /// Client version
    pub version: String,
}

/// Initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    /// Protocol version
    pub protocol_version: String,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Server info
    pub server_info: ServerInfo,
}

/// Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Server name
    pub name: String,
    /// Server version
    pub version: String,
}

impl Default for McpRequest {
    fn default() -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: String::new(),
            params: None,
        }
    }
}

impl McpRequest {
    /// Create a new MCP request
    pub fn new(method: String, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(serde_json::Number::from(0))),
            method,
            params,
        }
    }

    /// Create a new request with specific ID
    pub fn with_id(
        method: String,
        params: Option<serde_json::Value>,
        id: serde_json::Value,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id),
            method,
            params,
        }
    }
}

impl McpResponse {
    /// Create a successful response
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(
        id: serde_json::Value,
        code: i32,
        message: String,
        data: Option<serde_json::Value>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(McpError {
                code,
                message,
                data,
            }),
        }
    }

    /// Check if response is successful
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    /// Get the error message if any
    pub fn error_message(&self) -> Option<&str> {
        self.error.as_ref().map(|e| e.message.as_str())
    }
}

impl McpError {
    /// Create a new MCP error
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    /// Create a parse error
    pub fn parse_error(message: String) -> Self {
        Self::new(-32700, message)
    }

    /// Create an invalid request error
    pub fn invalid_request(message: String) -> Self {
        Self::new(-32600, message)
    }

    /// Create a method not found error
    pub fn method_not_found(message: String) -> Self {
        Self::new(-32601, message)
    }

    /// Create invalid params error
    pub fn invalid_params(message: String) -> Self {
        Self::new(-32602, message)
    }

    /// Create internal error
    pub fn internal_error(message: String) -> Self {
        Self::new(-32603, message)
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MCP Error (code {}): {}", self.code, self.message)
    }
}

impl std::error::Error for McpError {}
