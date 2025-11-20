//! Universal MCP Client Implementation
//!
//! This module provides the core Universal MCP Client that can connect to ANY MCP server
//! with dynamic discovery and connection management.

use crate::mcp::connection::McpConnectionManager;
use crate::mcp::discovery::McpServerDiscovery;
use crate::mcp::models::*;
use crate::mcp::protocol::McpProtocolHandler;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Universal MCP Client that can connect to any MCP server
#[derive(Clone)]
pub struct UniversalMcpClient {
    /// Protocol handler for JSON-RPC 2.0 communication
    protocol: Arc<McpProtocolHandler>,
    /// Server discovery system
    discovery: Arc<McpServerDiscovery>,
    /// Connection manager
    connection_manager: Arc<McpConnectionManager>,
    /// Connected servers
    connected_servers: Arc<RwLock<HashMap<Uuid, McpServerInfo>>>,
    /// Client capabilities
    capabilities: ClientCapabilities,
    /// Client information
    client_info: ClientInfo,
}

impl UniversalMcpClient {
    /// Create a new Universal MCP Client
    pub fn new() -> Self {
        Self {
            protocol: Arc::new(McpProtocolHandler::new()),
            discovery: Arc::new(McpServerDiscovery::new()),
            connection_manager: Arc::new(McpConnectionManager::new()),
            connected_servers: Arc::new(RwLock::new(HashMap::new())),
            capabilities: ClientCapabilities {
                roots: Some(RootCapability { root_uris: vec![] }),
                sampling: Some(SamplingCapability {}),
            },
            client_info: ClientInfo {
                name: "OdinCode".to_string(),
                version: "0.1.0".to_string(),
            },
        }
    }

    /// Discover available MCP servers
    pub async fn discover_servers(&self) -> Result<Vec<McpServerInfo>, McpError> {
        self.discovery.discover_servers().await
    }

    /// Connect to a specific MCP server
    pub async fn connect_to_server(&self, server_id: Uuid) -> Result<(), McpError> {
        // Get server info from discovery
        let servers = self.discovery.discover_servers().await?;
        let server_info = servers
            .into_iter()
            .find(|s| s.id == server_id)
            .ok_or_else(|| {
                McpError::new(-32000, format!("Server with ID {} not found", server_id))
            })?;

        // Establish connection
        let connection = self
            .connection_manager
            .establish_connection(&server_info)
            .await?;

        // Initialize the server
        let init_request = InitializeRequest {
            protocol_version: "2024-11-05".to_string(),
            capabilities: self.capabilities.clone(),
            client_info: self.client_info.clone(),
        };

        let stream = connection.get_stream()?;
        let init_response = self.protocol.initialize(stream, init_request).await?;

        // Update server info with capabilities
        let mut updated_server_info = server_info;
        updated_server_info.capabilities = init_response.capabilities;
        updated_server_info.status = ConnectionStatus::Connected;
        updated_server_info.last_connected = Some(Utc::now());

        // Store connected server
        let mut servers = self.connected_servers.write().await;
        servers.insert(server_id, updated_server_info);

        Ok(())
    }

    /// Disconnect from a server
    pub async fn disconnect_from_server(&self, server_id: Uuid) -> Result<(), McpError> {
        let mut servers = self.connected_servers.write().await;

        if let Some(mut server_info) = servers.remove(&server_id) {
            self.connection_manager
                .close_connection(&server_info)
                .await?;
            server_info.status = ConnectionStatus::Disconnected;
            server_info.last_connected = None;
        }

        Ok(())
    }

    /// Check if connected to a specific server
    pub async fn is_connected_to_server(&self, server_id: Uuid) -> bool {
        let servers = self.connected_servers.read().await;
        servers
            .get(&server_id)
            .map(|s| matches!(s.status, ConnectionStatus::Connected))
            .unwrap_or(false)
    }

    /// Get list of connected servers
    pub async fn get_connected_servers(&self) -> Vec<McpServerInfo> {
        let servers = self.connected_servers.read().await;
        servers.values().cloned().collect()
    }

    /// Check if connected to any server
    pub async fn is_connected(&self) -> bool {
        let servers = self.connected_servers.read().await;
        !servers.is_empty()
    }

    /// Get server capabilities
    pub async fn get_server_capabilities(&self, server_id: Uuid) -> Option<ServerCapabilities> {
        let servers = self.connected_servers.read().await;
        servers.get(&server_id).map(|s| s.capabilities.clone())
    }

    /// Call a tool on a specific server
    pub async fn call_tool(
        &self,
        server_id: Uuid,
        tool_name: String,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Result<ToolCallResponse, McpError> {
        let servers = self.connected_servers.read().await;
        let server_info = servers.get(&server_id).ok_or_else(|| {
            McpError::new(-32001, format!("Not connected to server {}", server_id))
        })?;

        let connection = self.connection_manager.get_connection(server_info).await?;
        let tool_request = ToolCallRequest {
            name: tool_name,
            arguments,
        };

        let stream = connection.get_stream()?;
        self.protocol.call_tool(stream, tool_request).await
    }

    /// Get available tools from a server
    pub async fn get_server_tools(&self, server_id: Uuid) -> Result<Vec<ToolCapability>, McpError> {
        let servers = self.connected_servers.read().await;
        let server_info = servers.get(&server_id).ok_or_else(|| {
            McpError::new(-32001, format!("Not connected to server {}", server_id))
        })?;

        let connection = self.connection_manager.get_connection(server_info).await?;
        let stream = connection.get_stream()?;
        self.protocol.list_tools(stream).await
    }

    /// Get available resources from a server
    pub async fn get_server_resources(
        &self,
        server_id: Uuid,
    ) -> Result<Vec<ResourceCapability>, McpError> {
        let servers = self.connected_servers.read().await;
        let server_info = servers.get(&server_id).ok_or_else(|| {
            McpError::new(-32001, format!("Not connected to server {}", server_id))
        })?;

        let connection = self.connection_manager.get_connection(server_info).await?;
        let stream = connection.get_stream()?;
        self.protocol.list_resources(stream).await
    }

    /// Get available prompts from a server
    pub async fn get_server_prompts(
        &self,
        server_id: Uuid,
    ) -> Result<Vec<PromptCapability>, McpError> {
        let servers = self.connected_servers.read().await;
        let server_info = servers.get(&server_id).ok_or_else(|| {
            McpError::new(-32001, format!("Not connected to server {}", server_id))
        })?;

        let connection = self.connection_manager.get_connection(server_info).await?;
        let stream = connection.get_stream()?;
        self.protocol.list_prompts(stream).await
    }

    /// Read a resource from a server
    pub async fn read_resource(
        &self,
        server_id: Uuid,
        resource_uri: String,
    ) -> Result<ResourceReference, McpError> {
        let servers = self.connected_servers.read().await;
        let server_info = servers.get(&server_id).ok_or_else(|| {
            McpError::new(-32001, format!("Not connected to server {}", server_id))
        })?;

        let connection = self.connection_manager.get_connection(server_info).await?;
        let stream = connection.get_stream()?;
        self.protocol.read_resource(stream, resource_uri).await
    }

    /// Get a prompt from a server
    pub async fn get_prompt(
        &self,
        server_id: Uuid,
        prompt_name: String,
        arguments: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<String, McpError> {
        let servers = self.connected_servers.read().await;
        let server_info = servers.get(&server_id).ok_or_else(|| {
            McpError::new(-32001, format!("Not connected to server {}", server_id))
        })?;

        let connection = self.connection_manager.get_connection(server_info).await?;
        let stream = connection.get_stream()?;
        self.protocol
            .get_prompt(stream, prompt_name, arguments)
            .await
    }

    /// Set client capabilities
    pub fn set_capabilities(&mut self, capabilities: ClientCapabilities) {
        self.capabilities = capabilities;
    }

    /// Set client information
    pub fn set_client_info(&mut self, client_info: ClientInfo) {
        self.client_info = client_info;
    }

    /// Get client capabilities
    pub fn get_capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    /// Get client information
    pub fn get_client_info(&self) -> &ClientInfo {
        &self.client_info
    }
}

impl Default for UniversalMcpClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = UniversalMcpClient::new();
        assert!(!client.is_connected().await);
        assert_eq!(client.get_client_info().name, "OdinCode");
        assert_eq!(client.get_client_info().version, "0.1.0");
    }

    #[tokio::test]
    async fn test_default_client() {
        let client = UniversalMcpClient::default();
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_capabilities() {
        let mut client = UniversalMcpClient::new();
        let new_caps = ClientCapabilities {
            roots: Some(RootCapability {
                root_uris: vec!["file:///test".to_string()],
            }),
            sampling: None,
        };

        client.set_capabilities(new_caps.clone());
        assert_eq!(
            client.get_capabilities().roots.as_ref().unwrap().root_uris,
            vec!["file:///test"]
        );
    }

    #[tokio::test]
    async fn test_client_info() {
        let mut client = UniversalMcpClient::new();
        let new_info = ClientInfo {
            name: "TestClient".to_string(),
            version: "1.0.0".to_string(),
        };

        client.set_client_info(new_info.clone());
        assert_eq!(client.get_client_info().name, "TestClient");
        assert_eq!(client.get_client_info().version, "1.0.0");
    }
}
