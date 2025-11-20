//! MCP Protocol Implementation
//!
//! This module provides the JSON-RPC 2.0 protocol implementation for MCP communication.

use crate::mcp::connection::StreamTrait;
use crate::mcp::models::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde_json;
use tokio::sync::Mutex;

/// MCP Protocol Handler for JSON-RPC 2.0 communication
pub struct McpProtocolHandler {
    /// Request ID counter
    request_id: AtomicU64,
}

impl McpProtocolHandler {
    /// Create a new MCP protocol handler
    pub fn new() -> Self {
        Self {
            request_id: AtomicU64::new(1),
        }
    }

    /// Generate next request ID
    fn next_request_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::SeqCst)
    }

    /// Initialize connection with MCP server
    pub async fn initialize(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
        request: InitializeRequest,
    ) -> Result<InitializeResponse, McpError> {
        let request_id = self.next_request_id();
        let mcp_request = McpRequest::with_id(
            "initialize".to_string(),
            Some(serde_json::to_value(request).map_err(|e| {
                McpError::invalid_params(format!("Failed to serialize initialize request: {}", e))
            })?),
            serde_json::Value::Number(serde_json::Number::from(request_id)),
        );

        let response = self.send_request(connection, mcp_request).await?;

        if !response.is_success() {
            return Err(response
                .error
                .unwrap_or_else(|| McpError::internal_error("Unknown error".to_string())));
        }

        serde_json::from_value(response.result.unwrap()).map_err(|e| {
            McpError::parse_error(format!("Failed to parse initialize response: {}", e))
        })
    }

    /// Call a tool on the server
    pub async fn call_tool(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
        request: ToolCallRequest,
    ) -> Result<ToolCallResponse, McpError> {
        let request_id = self.next_request_id();
        let mcp_request = McpRequest::with_id(
            "tools/call".to_string(),
            Some(serde_json::to_value(request).map_err(|e| {
                McpError::invalid_params(format!("Failed to serialize tool call request: {}", e))
            })?),
            serde_json::Value::Number(serde_json::Number::from(request_id)),
        );

        let response = self.send_request(connection, mcp_request).await?;

        if !response.is_success() {
            return Err(response
                .error
                .unwrap_or_else(|| McpError::internal_error("Unknown error".to_string())));
        }

        serde_json::from_value(response.result.unwrap()).map_err(|e| {
            McpError::parse_error(format!("Failed to parse tool call response: {}", e))
        })
    }

    /// List available tools
    pub async fn list_tools(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
    ) -> Result<Vec<ToolCapability>, McpError> {
        let request_id = self.next_request_id();
        let mcp_request = McpRequest::with_id(
            "tools/list".to_string(),
            None,
            serde_json::Value::Number(serde_json::Number::from(request_id)),
        );

        let response = self.send_request(connection, mcp_request).await?;

        if !response.is_success() {
            return Err(response
                .error
                .unwrap_or_else(|| McpError::internal_error("Unknown error".to_string())));
        }

        let result: serde_json::Value = response.result.unwrap();
        let tools: Vec<ToolCapability> = serde_json::from_value(
            result
                .get("tools")
                .unwrap_or(&serde_json::Value::Array(vec![]))
                .clone(),
        )
        .map_err(|e| McpError::parse_error(format!("Failed to parse tools list: {}", e)))?;

        Ok(tools)
    }

    /// List available resources
    pub async fn list_resources(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
    ) -> Result<Vec<ResourceCapability>, McpError> {
        let request_id = self.next_request_id();
        let mcp_request = McpRequest::with_id(
            "resources/list".to_string(),
            None,
            serde_json::Value::Number(serde_json::Number::from(request_id)),
        );

        let response = self.send_request(connection, mcp_request).await?;

        if !response.is_success() {
            return Err(response
                .error
                .unwrap_or_else(|| McpError::internal_error("Unknown error".to_string())));
        }

        let result: serde_json::Value = response.result.unwrap();
        let resources: Vec<ResourceCapability> = serde_json::from_value(
            result
                .get("resources")
                .unwrap_or(&serde_json::Value::Array(vec![]))
                .clone(),
        )
        .map_err(|e| McpError::parse_error(format!("Failed to parse resources list: {}", e)))?;

        Ok(resources)
    }

    /// List available prompts
    pub async fn list_prompts(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
    ) -> Result<Vec<PromptCapability>, McpError> {
        let request_id = self.next_request_id();
        let mcp_request = McpRequest::with_id(
            "prompts/list".to_string(),
            None,
            serde_json::Value::Number(serde_json::Number::from(request_id)),
        );

        let response = self.send_request(connection, mcp_request).await?;

        if !response.is_success() {
            return Err(response
                .error
                .unwrap_or_else(|| McpError::internal_error("Unknown error".to_string())));
        }

        let result: serde_json::Value = response.result.unwrap();
        let prompts: Vec<PromptCapability> = serde_json::from_value(
            result
                .get("prompts")
                .unwrap_or(&serde_json::Value::Array(vec![]))
                .clone(),
        )
        .map_err(|e| McpError::parse_error(format!("Failed to parse prompts list: {}", e)))?;

        Ok(prompts)
    }

    /// Read a resource
    pub async fn read_resource(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
        resource_uri: String,
    ) -> Result<ResourceReference, McpError> {
        let request_id = self.next_request_id();
        let params = serde_json::json!({ "uri": resource_uri });
        let mcp_request = McpRequest::with_id(
            "resources/read".to_string(),
            Some(params),
            serde_json::Value::Number(serde_json::Number::from(request_id)),
        );

        let response = self.send_request(connection, mcp_request).await?;

        if !response.is_success() {
            return Err(response
                .error
                .unwrap_or_else(|| McpError::internal_error("Unknown error".to_string())));
        }

        let result: serde_json::Value = response.result.unwrap();
        let resource: ResourceReference = serde_json::from_value(
            result
                .get("contents")
                .unwrap_or(&serde_json::Value::Null)
                .clone(),
        )
        .map_err(|e| McpError::parse_error(format!("Failed to parse resource contents: {}", e)))?;

        Ok(resource)
    }

    /// Get a prompt
    pub async fn get_prompt(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
        prompt_name: String,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String, McpError> {
        let request_id = self.next_request_id();
        let mut params = serde_json::Map::new();
        params.insert("name".to_string(), serde_json::Value::String(prompt_name));

        if let Some(args) = arguments {
            let args_value = serde_json::to_value(args).map_err(|e| {
                McpError::invalid_params(format!("Failed to serialize prompt arguments: {}", e))
            })?;
            params.insert("arguments".to_string(), args_value);
        }

        let mcp_request = McpRequest::with_id(
            "prompts/get".to_string(),
            Some(serde_json::Value::Object(params)),
            serde_json::Value::Number(serde_json::Number::from(request_id)),
        );

        let response = self.send_request(connection, mcp_request).await?;

        if !response.is_success() {
            return Err(response
                .error
                .unwrap_or_else(|| McpError::internal_error("Unknown error".to_string())));
        }

        let result: serde_json::Value = response.result.unwrap();
        let description = result
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(description.to_string())
    }

    /// Send a request and receive response
    async fn send_request(
        &self,
        connection: Arc<Mutex<dyn StreamTrait>>,
        request: McpRequest,
    ) -> Result<McpResponse, McpError> {
        let mut conn = connection.lock().await;

        // Serialize and send request
        let request_json = serde_json::to_string(&request)
            .map_err(|e| McpError::parse_error(format!("Failed to serialize request: {}", e)))?;

        // Send request with Content-Length header (HTTP-like)
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            request_json.len(),
            request_json
        );

        use tokio::io::AsyncWriteExt;
        conn.write_all(message.as_bytes())
            .await
            .map_err(|e| McpError::internal_error(format!("Failed to send request: {}", e)))?;

        // Read response
        let response = self.read_response(&mut *conn).await?;

        Ok(response)
    }

    /// Read response from connection
    async fn read_response(&self, conn: &mut dyn StreamTrait) -> Result<McpResponse, McpError> {
        use tokio::io::{AsyncReadExt, BufReader};

        let mut reader = BufReader::new(conn);
        let mut header_buffer = Vec::new();

        // Read headers
        loop {
            let mut byte = [0u8; 1];
            reader
                .read_exact(&mut byte)
                .await
                .map_err(|e| McpError::internal_error(format!("Failed to read header: {}", e)))?;

            header_buffer.push(byte[0]);

            // Check for header end (\r\n\r\n)
            if header_buffer.len() >= 4 {
                let last_four = &header_buffer[header_buffer.len() - 4..];
                if last_four == b"\r\n\r\n" {
                    break;
                }
            }
        }

        // Parse Content-Length header
        let header_str = String::from_utf8_lossy(&header_buffer);
        let content_length = self.parse_content_length(&header_str)?;

        // Read content
        let mut content_buffer = vec![0u8; content_length];
        reader
            .read_exact(&mut content_buffer)
            .await
            .map_err(|e| McpError::internal_error(format!("Failed to read content: {}", e)))?;

        let content_str = String::from_utf8(content_buffer)
            .map_err(|e| McpError::parse_error(format!("Invalid UTF-8 in response: {}", e)))?;

        // Parse JSON response
        let response: McpResponse = serde_json::from_str(&content_str)
            .map_err(|e| McpError::parse_error(format!("Failed to parse JSON response: {}", e)))?;

        Ok(response)
    }

    /// Parse Content-Length from headers
    fn parse_content_length(&self, headers: &str) -> Result<usize, McpError> {
        for line in headers.lines() {
            if line.starts_with("Content-Length:") {
                let length_str = line["Content-Length:".len()..].trim();
                return length_str
                    .parse()
                    .map_err(|e| McpError::parse_error(format!("Invalid Content-Length: {}", e)));
            }
        }
        Err(McpError::parse_error(
            "Content-Length header not found".to_string(),
        ))
    }
}

impl Default for McpProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_handler_creation() {
        let handler = McpProtocolHandler::new();
        assert_eq!(handler.next_request_id(), 1);
        assert_eq!(handler.next_request_id(), 2);
    }

    #[test]
    fn test_default_protocol_handler() {
        let handler = McpProtocolHandler::default();
        assert_eq!(handler.next_request_id(), 1);
    }

    #[test]
    fn test_content_length_parsing() {
        let handler = McpProtocolHandler::new();
        let headers = "Content-Length: 42\r\nContent-Type: application/json\r\n\r\n";
        let length = handler.parse_content_length(headers).unwrap();
        assert_eq!(length, 42);
    }

    #[test]
    fn test_content_length_parsing_missing() {
        let handler = McpProtocolHandler::new();
        let headers = "Content-Type: application/json\r\n\r\n";
        let result = handler.parse_content_length(headers);
        assert!(result.is_err());
    }

    #[test]
    fn test_content_length_parsing_invalid() {
        let handler = McpProtocolHandler::new();
        let headers = "Content-Length: invalid\r\n\r\n";
        let result = handler.parse_content_length(headers);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mcp_request_creation() {
        let request = McpRequest::new("test.method".to_string(), None);
        assert_eq!(request.jsonrpc, "2.0");
        assert_eq!(request.method, "test.method");
        assert!(request.params.is_none());
    }

    #[tokio::test]
    async fn test_mcp_response_success() {
        let response = McpResponse::success(
            serde_json::Value::Number(serde_json::Number::from(1)),
            serde_json::json!({"result": "success"}),
        );
        assert!(response.is_success());
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_mcp_response_error() {
        let response = McpResponse::error(
            serde_json::Value::Number(serde_json::Number::from(1)),
            -32601,
            "Method not found".to_string(),
            None,
        );
        assert!(!response.is_success());
        assert!(response.error.is_some());
        assert!(response.result.is_none());
    }
}
