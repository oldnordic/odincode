//! MCP Server Discovery System
//!
//! This module provides dynamic discovery of MCP servers from various sources.

use crate::mcp::models::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

/// MCP Server Discovery System
pub struct McpServerDiscovery {
    /// Configuration file paths
    config_paths: Vec<PathBuf>,
    /// Environment variable prefix
    env_prefix: String,
    /// Cache timeout in seconds
    cache_timeout: u64,
    /// Last discovery time
    last_discovery: std::sync::Mutex<Option<DateTime<Utc>>>,
    /// Cached servers
    cached_servers: std::sync::Mutex<Option<Vec<McpServerInfo>>>,
}

impl McpServerDiscovery {
    /// Create a new MCP server discovery instance
    pub fn new() -> Self {
        let config_paths = vec![
            PathBuf::from("/etc/odincode/mcp_servers.json"),
            PathBuf::from("~/.config/odincode/mcp_servers.json"),
            PathBuf::from("./mcp_servers.json"),
        ];

        Self {
            config_paths,
            env_prefix: "MCP_SERVER_".to_string(),
            cache_timeout: 300, // 5 minutes
            last_discovery: std::sync::Mutex::new(None),
            cached_servers: std::sync::Mutex::new(None),
        }
    }

    /// Discover available MCP servers
    pub async fn discover_servers(&self) -> Result<Vec<McpServerInfo>, McpError> {
        // Check cache first
        if self.is_cache_valid() {
            if let Ok(cached) = self.cached_servers.lock() {
                if let Some(servers) = &*cached {
                    return Ok(servers.clone());
                }
            }
        }

        // Discover servers from all sources
        let mut servers = Vec::new();

        // Discover from configuration files
        servers.extend(self.discover_from_config_files().await?);

        // Discover from environment variables
        servers.extend(self.discover_from_env_vars().await?);

        // Discover from system locations
        servers.extend(self.discover_from_system().await?);

        // Discover from running processes
        servers.extend(self.discover_from_processes().await?);

        // Update cache
        let mut cached = self.cached_servers.lock().unwrap();
        *cached = Some(servers.clone());

        let mut last_discovery = self.last_discovery.lock().unwrap();
        *last_discovery = Some(Utc::now());

        Ok(servers)
    }

    /// Discover servers from configuration files
    async fn discover_from_config_files(&self) -> Result<Vec<McpServerInfo>, McpError> {
        let mut servers = Vec::new();

        for config_path in &self.config_paths {
            if let Ok(config_servers) = self.load_config_file(config_path).await {
                servers.extend(config_servers);
            }
        }

        Ok(servers)
    }

    /// Load servers from a configuration file
    async fn load_config_file(&self, path: &PathBuf) -> Result<Vec<McpServerInfo>, McpError> {
        // Expand home directory
        let expanded_path = if path.starts_with("~") {
            if let Some(home) = dirs::home_dir() {
                home.join(path.strip_prefix("~").unwrap())
            } else {
                path.clone()
            }
        } else {
            path.clone()
        };

        // Check if file exists
        if !expanded_path.exists() {
            return Ok(Vec::new());
        }

        // Read and parse file
        let content = fs::read_to_string(&expanded_path).await.map_err(|e| {
            McpError::internal_error(format!(
                "Failed to read config file {}: {}",
                expanded_path.display(),
                e
            ))
        })?;

        let config: McpServerConfig = serde_json::from_str(&content).map_err(|e| {
            McpError::parse_error(format!(
                "Failed to parse config file {}: {}",
                expanded_path.display(),
                e
            ))
        })?;

        Ok(config.servers)
    }

    /// Discover servers from environment variables
    async fn discover_from_env_vars(&self) -> Result<Vec<McpServerInfo>, McpError> {
        let mut servers = Vec::new();

        for (key, value) in std::env::vars() {
            if key.starts_with(&self.env_prefix) {
                if let Ok(server_info) = self.parse_env_server(&key, &value) {
                    servers.push(server_info);
                }
            }
        }

        Ok(servers)
    }

    /// Parse server from environment variable
    fn parse_env_server(&self, key: &str, value: &str) -> Result<McpServerInfo, McpError> {
        let server_name = key[self.env_prefix.len()..]
            .to_lowercase()
            .replace('_', "-");

        // Parse value as JSON or simple endpoint
        let server_info = if value.trim().starts_with('{') {
            // JSON format
            serde_json::from_str(value).map_err(|e| {
                McpError::parse_error(format!(
                    "Failed to parse server config from env var {}: {}",
                    key, e
                ))
            })?
        } else {
            // Simple endpoint format
            McpServerInfo {
                id: Uuid::new_v4(),
                name: server_name.clone(),
                description: format!("MCP server from environment variable {}", key),
                version: "1.0.0".to_string(),
                endpoint: value.to_string(),
                capabilities: ServerCapabilities {
                    tools: Vec::new(),
                    resources: Vec::new(),
                    prompts: Vec::new(),
                    logging: false,
                    sampling: false,
                },
                status: ConnectionStatus::Disconnected,
                last_connected: None,
                metadata: HashMap::new(),
            }
        };

        Ok(server_info)
    }

    /// Discover servers from system locations
    async fn discover_from_system(&self) -> Result<Vec<McpServerInfo>, McpError> {
        let mut servers = Vec::new();

        // Check common system locations for MCP servers
        let system_paths = vec![
            "/usr/local/bin/mcp-*",
            "/usr/bin/mcp-*",
            "/opt/mcp/bin/*",
            "~/.local/bin/mcp-*",
        ];

        for pattern in system_paths {
            if let Ok(found_servers) = self.discover_from_glob(pattern).await {
                servers.extend(found_servers);
            }
        }

        Ok(servers)
    }

    /// Discover servers from glob pattern
    async fn discover_from_glob(&self, pattern: &str) -> Result<Vec<McpServerInfo>, McpError> {
        let mut servers = Vec::new();

        // Use glob to find matching files
        if let Ok(paths) = glob::glob(pattern) {
            for path in paths.flatten() {
                if path.is_file() {
                    if let Some(server_info) = self.create_server_from_path(&path).await {
                        servers.push(server_info);
                    }
                }
            }
        }

        Ok(servers)
    }

    /// Create server info from file path
    async fn create_server_from_path(&self, path: &PathBuf) -> Option<McpServerInfo> {
        let file_name = path.file_name()?.to_string_lossy().to_string();
        let server_name = file_name.replace("mcp-", "").replace('_', "-");

        Some(McpServerInfo {
            id: Uuid::new_v4(),
            name: server_name.clone(),
            description: format!("MCP server from executable: {}", file_name),
            version: "1.0.0".to_string(),
            endpoint: path.to_string_lossy().to_string(),
            capabilities: ServerCapabilities {
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
                logging: false,
                sampling: false,
            },
            status: ConnectionStatus::Disconnected,
            last_connected: None,
            metadata: HashMap::new(),
        })
    }

    /// Discover servers from running processes
    async fn discover_from_processes(&self) -> Result<Vec<McpServerInfo>, McpError> {
        let mut servers = Vec::new();

        // Check for running MCP processes
        if let Ok(processes) = self.find_mcp_processes().await {
            for process in processes {
                servers.push(process);
            }
        }

        Ok(servers)
    }

    /// Find running MCP processes
    async fn find_mcp_processes(&self) -> Result<Vec<McpServerInfo>, McpError> {
        let mut servers = Vec::new();

        // Use ps command to find running processes
        let output = tokio::process::Command::new("ps")
            .args(&["aux"])
            .output()
            .await
            .map_err(|e| McpError::internal_error(format!("Failed to run ps command: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout);

        for line in output_str.lines() {
            if line.contains("mcp-") || line.contains("mcp_") {
                if let Some(server_info) = self.parse_process_line(line) {
                    servers.push(server_info);
                }
            }
        }

        Ok(servers)
    }

    /// Parse process line to extract server info
    fn parse_process_line(&self, line: &str) -> Option<McpServerInfo> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 11 {
            return None;
        }

        let command = parts[10..].join(" ");

        // Extract server name from command
        let server_name = if command.contains("mcp-") {
            command.split("mcp-").nth(1)?.split_whitespace().next()?
        } else if command.contains("mcp_") {
            command.split("mcp_").nth(1)?.split_whitespace().next()?
        } else {
            return None;
        };

        Some(McpServerInfo {
            id: Uuid::new_v4(),
            name: server_name.replace('_', "-"),
            description: format!("Running MCP process: {}", command),
            version: "1.0.0".to_string(),
            endpoint: command,
            capabilities: ServerCapabilities {
                tools: Vec::new(),
                resources: Vec::new(),
                prompts: Vec::new(),
                logging: false,
                sampling: false,
            },
            status: ConnectionStatus::Disconnected,
            last_connected: None,
            metadata: HashMap::new(),
        })
    }

    /// Check if cache is still valid
    fn is_cache_valid(&self) -> bool {
        let last_discovery = self.last_discovery.lock().unwrap();
        if let Some(last) = *last_discovery {
            let now = Utc::now();
            let duration = now.signed_duration_since(last);
            duration.num_seconds() < self.cache_timeout as i64
        } else {
            false
        }
    }

    /// Clear the discovery cache
    pub fn clear_cache(&self) {
        let mut cached = self.cached_servers.lock().unwrap();
        *cached = None;

        let mut last_discovery = self.last_discovery.lock().unwrap();
        *last_discovery = None;
    }

    /// Set cache timeout
    pub fn set_cache_timeout(&mut self, timeout_seconds: u64) {
        self.cache_timeout = timeout_seconds;
    }

    /// Add custom config path
    pub fn add_config_path(&mut self, path: PathBuf) {
        self.config_paths.push(path);
    }

    /// Set environment variable prefix
    pub fn set_env_prefix(&mut self, prefix: String) {
        self.env_prefix = prefix;
    }
}

impl Default for McpServerDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// MCP Server Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// List of servers
    pub servers: Vec<McpServerInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_discovery_creation() {
        let discovery = McpServerDiscovery::new();
        assert_eq!(discovery.env_prefix, "MCP_SERVER_");
        assert_eq!(discovery.cache_timeout, 300);
    }

    #[tokio::test]
    async fn test_default_discovery() {
        let discovery = McpServerDiscovery::default();
        assert_eq!(discovery.env_prefix, "MCP_SERVER_");
    }

    #[tokio::test]
    async fn test_cache_timeout() {
        let mut discovery = McpServerDiscovery::new();
        discovery.set_cache_timeout(60);
        assert_eq!(discovery.cache_timeout, 60);
    }

    #[tokio::test]
    async fn test_env_prefix() {
        let mut discovery = McpServerDiscovery::new();
        discovery.set_env_prefix("TEST_".to_string());
        assert_eq!(discovery.env_prefix, "TEST_");
    }

    #[tokio::test]
    async fn test_config_file_loading() {
        let discovery = McpServerDiscovery::new();

        // Create temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        let config_content = r#"
        {
            "servers": [
                {
                    "id": "00000000-0000-0000-0000-000000000001",
                    "name": "test-server",
                    "description": "Test server",
                    "version": "1.0.0",
                    "endpoint": "stdio://test",
                    "capabilities": {
                        "tools": [],
                        "resources": [],
                        "prompts": [],
                        "logging": false,
                        "sampling": false
                    },
                    "status": "Disconnected",
                    "last_connected": null,
                    "metadata": {}
                }
            ]
        }
        "#;

        write!(temp_file, "{}", config_content).unwrap();

        let servers = discovery
            .load_config_file(&temp_file.path().to_path_buf())
            .await
            .unwrap();
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].name, "test-server");
    }

    #[tokio::test]
    async fn test_env_server_parsing() {
        let discovery = McpServerDiscovery::new();

        // Test simple endpoint format
        let server = discovery
            .parse_env_server("MCP_SERVER_TEST", "stdio://test-server")
            .unwrap();
        assert_eq!(server.name, "test");
        assert_eq!(server.endpoint, "stdio://test-server");

        // Test JSON format
        let json_config = r#"
        {
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "json-server",
            "description": "JSON config server",
            "version": "1.0.0",
            "endpoint": "stdio://json-server",
            "capabilities": {
                "tools": [],
                "resources": [],
                "prompts": [],
                "logging": false,
                "sampling": false
            },
            "status": "Disconnected",
            "last_connected": null,
            "metadata": {}
        }
        "#;

        let server = discovery
            .parse_env_server("MCP_SERVER_JSON", json_config)
            .unwrap();
        assert_eq!(server.name, "json-server");
        assert_eq!(server.endpoint, "stdio://json-server");
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let discovery = McpServerDiscovery::new();

        // Initially cache should be invalid
        assert!(!discovery.is_cache_valid());

        // Clear cache should work
        discovery.clear_cache();
        assert!(!discovery.is_cache_valid());
    }
}
