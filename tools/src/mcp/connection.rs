//! MCP Connection Management
//!
//! This module provides connection management for MCP servers with various transport types.

use crate::mcp::models::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::process::{Child, Command};
use tokio::sync::{Mutex, RwLock};

/// Trait combining AsyncRead and AsyncWrite for stream trait objects
pub trait StreamTrait: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> StreamTrait for T {}
use std::process::Stdio;
use tokio::net::{TcpStream, UnixStream};
use uuid::Uuid;

/// MCP Connection Manager
pub struct McpConnectionManager {
    /// Active connections
    connections: Arc<RwLock<HashMap<Uuid, Arc<McpConnection>>>>,
    /// Connection timeouts
    connection_timeout: u64,
    /// Keep-alive interval
    keep_alive_interval: u64,
}

impl McpConnectionManager {
    /// Create a new MCP connection manager
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_timeout: 30,  // 30 seconds
            keep_alive_interval: 60, // 1 minute
        }
    }

    /// Establish connection to MCP server
    pub async fn establish_connection(
        &self,
        server_info: &McpServerInfo,
    ) -> Result<Arc<McpConnection>, McpError> {
        let connection = match server_info.endpoint.as_str() {
            endpoint if endpoint.starts_with("stdio://") => {
                self.create_stdio_connection(endpoint).await?
            }
            endpoint if endpoint.starts_with("tcp://") => {
                self.create_tcp_connection(endpoint).await?
            }
            endpoint if endpoint.starts_with("unix://") => {
                self.create_unix_connection(endpoint).await?
            }
            endpoint if endpoint.starts_with("ws://") || endpoint.starts_with("wss://") => {
                self.create_websocket_connection(endpoint).await?
            }
            _ => {
                // Try to detect connection type
                if server_info.endpoint.contains('/') {
                    // Assume it's a file path for stdio
                    self.create_stdio_connection(&format!("stdio://{}", server_info.endpoint))
                        .await?
                } else if server_info.endpoint.contains(':') {
                    // Assume it's a TCP address
                    self.create_tcp_connection(&format!("tcp://{}", server_info.endpoint))
                        .await?
                } else {
                    // Assume it's a command for stdio
                    self.create_stdio_connection(&format!("stdio://{}", server_info.endpoint))
                        .await?
                }
            }
        };

        // Store connection
        let mut connections = self.connections.write().await;
        connections.insert(server_info.id, connection.clone());

        Ok(connection)
    }

    /// Create stdio connection
    async fn create_stdio_connection(
        &self,
        endpoint: &str,
    ) -> Result<Arc<McpConnection>, McpError> {
        let command = endpoint["stdio://".len()..].to_string();
        let parts: Vec<&str> = command.split_whitespace().collect();

        if parts.is_empty() {
            return Err(McpError::invalid_params("Empty stdio command".to_string()));
        }

        let mut cmd = Command::new(parts[0]);

        // Add arguments
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        // Configure process
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        // Start process
        let child = cmd.spawn().map_err(|e| {
            McpError::internal_error(format!("Failed to start process '{}': {}", command, e))
        })?;

        let connection = McpConnection {
            id: Uuid::new_v4(),
            server_id: Uuid::new_v4(), // Will be set by caller
            connection_type: ConnectionType::Stdio,
            process: Some(Arc::new(Mutex::new(child))),
            tcp_stream: None,
            unix_stream: None,
            websocket_stream: None,
            last_activity: std::time::Instant::now(),
        };

        Ok(Arc::new(connection))
    }

    /// Create TCP connection
    async fn create_tcp_connection(&self, endpoint: &str) -> Result<Arc<McpConnection>, McpError> {
        let address = endpoint["tcp://".len()..].to_string();

        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.connection_timeout),
            TcpStream::connect(&address),
        )
        .await
        .map_err(|_| McpError::internal_error("Connection timeout".to_string()))?
        .map_err(|e| {
            McpError::internal_error(format!("Failed to connect to TCP {}: {}", address, e))
        })?;

        let connection = McpConnection {
            id: Uuid::new_v4(),
            server_id: Uuid::new_v4(), // Will be set by caller
            connection_type: ConnectionType::Tcp,
            process: None,
            tcp_stream: Some(Arc::new(Mutex::new(stream))),
            unix_stream: None,
            websocket_stream: None,
            last_activity: std::time::Instant::now(),
        };

        Ok(Arc::new(connection))
    }

    /// Create Unix socket connection
    async fn create_unix_connection(&self, endpoint: &str) -> Result<Arc<McpConnection>, McpError> {
        let path = endpoint["unix://".len()..].to_string();

        let stream = tokio::time::timeout(
            std::time::Duration::from_secs(self.connection_timeout),
            UnixStream::connect(&path),
        )
        .await
        .map_err(|_| McpError::internal_error("Connection timeout".to_string()))?
        .map_err(|e| {
            McpError::internal_error(format!("Failed to connect to Unix socket {}: {}", path, e))
        })?;

        let connection = McpConnection {
            id: Uuid::new_v4(),
            server_id: Uuid::new_v4(), // Will be set by caller
            connection_type: ConnectionType::Unix,
            process: None,
            tcp_stream: None,
            unix_stream: Some(Arc::new(Mutex::new(stream))),
            websocket_stream: None,
            last_activity: std::time::Instant::now(),
        };

        Ok(Arc::new(connection))
    }

    /// Create WebSocket connection
    async fn create_websocket_connection(
        &self,
        _endpoint: &str,
    ) -> Result<Arc<McpConnection>, McpError> {
        // This would require a WebSocket client library
        // For now, return an error indicating WebSocket is not yet implemented
        Err(McpError::internal_error(
            "WebSocket connections not yet implemented".to_string(),
        ))
    }

    /// Get connection for server
    pub async fn get_connection(
        &self,
        server_info: &McpServerInfo,
    ) -> Result<Arc<McpConnection>, McpError> {
        let connections = self.connections.read().await;

        connections.get(&server_info.id).cloned().ok_or_else(|| {
            McpError::internal_error(format!("No connection found for server {}", server_info.id))
        })
    }

    /// Close connection to server
    pub async fn close_connection(&self, server_info: &McpServerInfo) -> Result<(), McpError> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.remove(&server_info.id) {
            connection.close().await?;
        }

        Ok(())
    }

    /// Close all connections
    pub async fn close_all_connections(&self) -> Result<(), McpError> {
        let mut connections = self.connections.write().await;

        for (_, connection) in connections.drain() {
            if let Err(e) = connection.close().await {
                eprintln!("Error closing connection: {}", e);
            }
        }

        Ok(())
    }

    /// Get active connections count
    pub async fn get_active_connections_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Check if connection is active
    pub async fn is_connection_active(&self, server_id: Uuid) -> bool {
        let connections = self.connections.read().await;
        connections.contains_key(&server_id)
    }

    /// Set connection timeout
    pub fn set_connection_timeout(&mut self, timeout_seconds: u64) {
        self.connection_timeout = timeout_seconds;
    }

    /// Set keep-alive interval
    pub fn set_keep_alive_interval(&mut self, interval_seconds: u64) {
        self.keep_alive_interval = interval_seconds;
    }

    /// Cleanup inactive connections
    pub async fn cleanup_inactive_connections(&self) -> Result<(), McpError> {
        let mut connections = self.connections.write().await;
        let now = std::time::Instant::now();

        let inactive_connections: Vec<Uuid> = connections
            .iter()
            .filter(|(_, conn)| {
                now.duration_since(conn.last_activity).as_secs() > self.keep_alive_interval
            })
            .map(|(id, _)| *id)
            .collect();

        for server_id in inactive_connections {
            if let Some(connection) = connections.remove(&server_id) {
                connection.close().await?;
            }
        }

        Ok(())
    }
}

impl Default for McpConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP Connection
pub struct McpConnection {
    /// Connection ID
    pub id: Uuid,
    /// Server ID
    pub server_id: Uuid,
    /// Connection type
    pub connection_type: ConnectionType,
    /// Process handle (for stdio connections)
    pub process: Option<Arc<Mutex<Child>>>,
    /// TCP stream (for TCP connections)
    pub tcp_stream: Option<Arc<Mutex<TcpStream>>>,
    /// Unix stream (for Unix socket connections)
    pub unix_stream: Option<Arc<Mutex<UnixStream>>>,
    /// WebSocket stream (for WebSocket connections)
    pub websocket_stream: Option<Arc<Mutex<dyn StreamTrait>>>,
    /// Last activity timestamp
    pub last_activity: std::time::Instant,
}

impl McpConnection {
    /// Close the connection
    pub async fn close(&self) -> Result<(), McpError> {
        match self.connection_type {
            ConnectionType::Stdio => {
                if let Some(process) = &self.process {
                    let mut process = process.lock().await;
                    process.kill().await.map_err(|e| {
                        McpError::internal_error(format!("Failed to kill process: {}", e))
                    })?;
                }
            }
            ConnectionType::Tcp => {
                if let Some(stream) = &self.tcp_stream {
                    let mut stream = stream.lock().await;
                    stream.shutdown().await.map_err(|e| {
                        McpError::internal_error(format!("Failed to shutdown TCP stream: {}", e))
                    })?;
                }
            }
            ConnectionType::Unix => {
                if let Some(stream) = &self.unix_stream {
                    let mut stream = stream.lock().await;
                    stream.shutdown().await.map_err(|e| {
                        McpError::internal_error(format!("Failed to shutdown Unix stream: {}", e))
                    })?;
                }
            }
            ConnectionType::WebSocket => {
                // WebSocket cleanup would go here
                return Err(McpError::internal_error(
                    "WebSocket cleanup not yet implemented".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = std::time::Instant::now();
    }

    /// Get connection type as string
    pub fn connection_type_str(&self) -> &'static str {
        match self.connection_type {
            ConnectionType::Stdio => "stdio",
            ConnectionType::Tcp => "tcp",
            ConnectionType::Unix => "unix",
            ConnectionType::WebSocket => "websocket",
        }
    }

    /// Get the stream as a trait object for protocol operations
    pub fn get_stream(&self) -> Result<Arc<Mutex<dyn StreamTrait>>, McpError> {
        match self.connection_type {
            ConnectionType::Stdio => {
                if let Some(_process) = &self.process {
                    // For stdio, we need to get the child's stdin/stdout
                    // This is more complex and requires a different approach
                    return Err(McpError::internal_error(
                        "Stdio stream extraction not yet implemented".to_string(),
                    ));
                }
                Err(McpError::internal_error(
                    "No stdio process available".to_string(),
                ))
            }
            ConnectionType::Tcp => {
                if let Some(stream) = &self.tcp_stream {
                    Ok(stream.clone() as Arc<Mutex<dyn StreamTrait>>)
                } else {
                    Err(McpError::internal_error(
                        "No TCP stream available".to_string(),
                    ))
                }
            }
            ConnectionType::Unix => {
                if let Some(stream) = &self.unix_stream {
                    Ok(stream.clone() as Arc<Mutex<dyn StreamTrait>>)
                } else {
                    Err(McpError::internal_error(
                        "No Unix stream available".to_string(),
                    ))
                }
            }
            ConnectionType::WebSocket => {
                if let Some(stream) = &self.websocket_stream {
                    Ok(stream.clone() as Arc<Mutex<dyn StreamTrait>>)
                } else {
                    Err(McpError::internal_error(
                        "No WebSocket stream available".to_string(),
                    ))
                }
            }
        }
    }

    /// Check if connection is still alive
    pub async fn is_alive(&self) -> bool {
        match self.connection_type {
            ConnectionType::Stdio => {
                if let Some(process) = &self.process {
                    let mut process = process.lock().await;
                    process.try_wait().map(|w| w.is_none()).unwrap_or(false)
                } else {
                    false
                }
            }
            ConnectionType::Tcp => {
                if let Some(stream) = &self.tcp_stream {
                    let stream = stream.lock().await;
                    // Try to peek to see if connection is still alive
                    stream.peek(&mut [0u8; 1]).await.is_ok()
                } else {
                    false
                }
            }
            ConnectionType::Unix => {
                if let Some(_stream) = &self.unix_stream {
                    // For Unix streams, we'll assume they're alive if they exist
                    // A more sophisticated check would involve trying to read/write
                    true
                } else {
                    false
                }
            }
            ConnectionType::WebSocket => {
                // WebSocket health check would go here
                false
            }
        }
    }
}

/// Connection type enum
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// Stdio connection
    Stdio,
    /// TCP connection
    Tcp,
    /// Unix socket connection
    Unix,
    /// WebSocket connection
    WebSocket,
}

/// WebSocket stream trait (placeholder for future implementation)
pub trait WebSocketStream: AsyncRead + AsyncWrite + Unpin + Send + Sync {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let manager = McpConnectionManager::new();
        assert_eq!(manager.connection_timeout, 30);
        assert_eq!(manager.keep_alive_interval, 60);
    }

    #[tokio::test]
    async fn test_default_connection_manager() {
        let manager = McpConnectionManager::default();
        assert_eq!(manager.connection_timeout, 30);
    }

    #[tokio::test]
    async fn test_timeout_configuration() {
        let mut manager = McpConnectionManager::new();
        manager.set_connection_timeout(60);
        manager.set_keep_alive_interval(120);

        assert_eq!(manager.connection_timeout, 60);
        assert_eq!(manager.keep_alive_interval, 120);
    }

    #[tokio::test]
    async fn test_active_connections_count() {
        let manager = McpConnectionManager::new();
        assert_eq!(manager.get_active_connections_count().await, 0);
    }

    #[tokio::test]
    async fn test_connection_type_strings() {
        let connection = McpConnection {
            id: Uuid::new_v4(),
            server_id: Uuid::new_v4(),
            connection_type: ConnectionType::Stdio,
            process: None,
            tcp_stream: None,
            unix_stream: None,
            websocket_stream: None,
            last_activity: std::time::Instant::now(),
        };

        assert_eq!(connection.connection_type_str(), "stdio");
    }

    #[tokio::test]
    async fn test_activity_update() {
        let mut connection = McpConnection {
            id: Uuid::new_v4(),
            server_id: Uuid::new_v4(),
            connection_type: ConnectionType::Stdio,
            process: None,
            tcp_stream: None,
            unix_stream: None,
            websocket_stream: None,
            last_activity: std::time::Instant::now(),
        };

        let old_activity = connection.last_activity;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        connection.update_activity();
        assert!(connection.last_activity > old_activity);
    }
}
