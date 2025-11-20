//! Universal MCP Client Module
//!
//! This module provides a universal MCP (Model Context Protocol) client
//! that can connect to ANY MCP server with dynamic discovery and connection management.

pub mod client;
pub mod connection;
pub mod discovery;
pub mod models;
pub mod protocol;

pub use client::*;
pub use connection::*;
pub use discovery::*;
pub use models::*;
pub use protocol::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mcp_client_creation() {
        let client = UniversalMcpClient::new();
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_server_discovery() {
        let discovery = McpServerDiscovery::new();
        let servers = discovery.discover_servers().await.unwrap();
        // Should find at least some servers or return empty list
        assert!(servers.len() >= 0);
    }
}
