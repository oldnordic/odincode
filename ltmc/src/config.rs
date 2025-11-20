//! LTMC Configuration Module
//!
//! This module provides runtime configuration management for the LTMC system,
//! allowing dynamic enable/disable of LTMC components and graceful fallbacks.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

/// LTMC Configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LTMCConfig {
    /// Whether LTMC is enabled at runtime
    pub enabled: bool,

    /// Database-specific configurations
    pub databases: DatabaseConfig,

    /// Feature flags for runtime behavior
    pub features: FeatureConfig,

    /// Performance settings
    pub performance: PerformanceConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DatabaseConfig {
    /// SQLite configuration
    pub sqlite: SQLiteConfig,

    /// Redis configuration (optional)
    pub redis: Option<RedisConfig>,

    /// Neo4j configuration (optional)
    pub neo4j: Option<Neo4jConfig>,

    /// FAISS configuration (optional)
    pub faiss: Option<FAISSConfig>,
}

/// SQLite configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SQLiteConfig {
    /// Database file path
    pub path: PathBuf,

    /// Connection pool size
    pub pool_size: u32,

    /// Enable WAL mode
    pub wal_mode: bool,
}

/// Redis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,

    /// Connection pool size
    pub pool_size: u32,

    /// Default TTL for cached items (seconds)
    pub default_ttl: u64,
}

/// Neo4j configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Neo4jConfig {
    /// Neo4j connection URI
    pub uri: String,

    /// Username
    pub username: String,

    /// Password
    pub password: String,

    /// Connection pool size
    pub pool_size: u32,
}

/// FAISS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FAISSConfig {
    /// Index file path
    pub index_path: PathBuf,

    /// Vector dimension
    pub dimension: usize,

    /// Index type
    pub index_type: String,
}

/// Feature configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureConfig {
    /// Enable pattern learning
    pub pattern_learning: bool,

    /// Enable sequential thinking
    pub sequential_thinking: bool,

    /// Enable user interaction tracking
    pub user_tracking: bool,

    /// Enable cross-database coordination
    pub cross_database: bool,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Enable caching
    pub caching: bool,

    /// Cache size limit
    pub cache_size: usize,

    /// Enable async processing
    pub async_processing: bool,

    /// Batch size for operations
    pub batch_size: usize,
}

impl Default for LTMCConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            databases: DatabaseConfig::default(),
            features: FeatureConfig::default(),
            performance: PerformanceConfig::default(),
        }
    }
}

impl Default for SQLiteConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("odincode.db"),
            pool_size: 10,
            wal_mode: true,
        }
    }
}

impl Default for FeatureConfig {
    fn default() -> Self {
        Self {
            pattern_learning: true,
            sequential_thinking: true,
            user_tracking: true,
            cross_database: false,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            caching: true,
            cache_size: 1000,
            async_processing: true,
            batch_size: 100,
        }
    }
}

/// LTMC Configuration Manager
pub struct LTMCConfigManager {
    config: LTMCConfig,
    config_path: Option<PathBuf>,
}

impl Default for LTMCConfigManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LTMCConfigManager {
    /// Create a new configuration manager
    pub fn new() -> Self {
        Self {
            config: LTMCConfig::default(),
            config_path: None,
        }
    }

    /// Create a configuration manager with a specific config file
    pub fn with_config_file<P: AsRef<std::path::Path>>(path: P) -> Self {
        Self {
            config: LTMCConfig::default(),
            config_path: Some(path.as_ref().to_path_buf()),
        }
    }

    /// Load configuration from file
    pub async fn load_config(&mut self) -> Result<()> {
        if let Some(ref path) = self.config_path {
            if path.exists() {
                let content = tokio::fs::read_to_string(path)
                    .await
                    .map_err(|e| anyhow!("Failed to read config file: {e}"))?;

                self.config = serde_json::from_str(&content)
                    .map_err(|e| anyhow!("Failed to parse config file: {e}"))?;

                info!("Loaded LTMC configuration from {:?}", path);
            } else {
                warn!("Config file {:?} not found, using defaults", path);
                self.save_config().await?;
            }
        }
        Ok(())
    }

    /// Save configuration to file
    pub async fn save_config(&self) -> Result<()> {
        if let Some(ref path) = self.config_path {
            let content = serde_json::to_string_pretty(&self.config)
                .map_err(|e| anyhow!("Failed to serialize config: {e}"))?;

            tokio::fs::write(path, content)
                .await
                .map_err(|e| anyhow!("Failed to write config file: {e}"))?;

            info!("Saved LTMC configuration to {:?}", path);
        }
        Ok(())
    }

    /// Get current configuration
    pub fn get_config(&self) -> &LTMCConfig {
        &self.config
    }

    /// Get mutable configuration
    pub fn get_config_mut(&mut self) -> &mut LTMCConfig {
        &mut self.config
    }

    /// Enable or disable LTMC
    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        info!("LTMC {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if LTMC is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check if a specific feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        match feature {
            "pattern_learning" => self.config.features.pattern_learning,
            "sequential_thinking" => self.config.features.sequential_thinking,
            "user_tracking" => self.config.features.user_tracking,
            "cross_database" => self.config.features.cross_database,
            _ => false,
        }
    }

    /// Enable or disable a specific feature
    pub fn set_feature_enabled(&mut self, feature: &str, enabled: bool) -> Result<()> {
        match feature {
            "pattern_learning" => self.config.features.pattern_learning = enabled,
            "sequential_thinking" => self.config.features.sequential_thinking = enabled,
            "user_tracking" => self.config.features.user_tracking = enabled,
            "cross_database" => self.config.features.cross_database = enabled,
            _ => return Err(anyhow!("Unknown feature: {feature}")),
        }
        info!(
            "Feature '{}' {}",
            feature,
            if enabled { "enabled" } else { "disabled" }
        );
        Ok(())
    }

    /// Check if a database is configured
    pub fn is_database_configured(&self, database: &str) -> bool {
        match database {
            "sqlite" => true, // Always configured
            "redis" => self.config.databases.redis.is_some(),
            "neo4j" => self.config.databases.neo4j.is_some(),
            "faiss" => self.config.databases.faiss.is_some(),
            _ => false,
        }
    }

    /// Get available databases based on compilation features and runtime config
    #[allow(unused_mut)]
    pub fn get_available_databases(&self) -> Vec<String> {
        let mut databases = vec!["sqlite".to_string()];

        #[cfg(feature = "ltmc-redis")]
        if self.is_database_configured("redis") {
            databases.push("redis".to_string());
        }

        #[cfg(feature = "ltmc-neo4j")]
        if self.is_database_configured("neo4j") {
            databases.push("neo4j".to_string());
        }

        #[cfg(feature = "ltmc-faiss")]
        if self.is_database_configured("faiss") {
            databases.push("faiss".to_string());
        }

        databases
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // Validate SQLite configuration
        if self.config.databases.sqlite.pool_size == 0 {
            return Err(anyhow!("SQLite pool size must be greater than 0"));
        }

        // Validate Redis configuration if present
        if let Some(ref redis) = self.config.databases.redis {
            if redis.pool_size == 0 {
                return Err(anyhow!("Redis pool size must be greater than 0"));
            }
            if redis.default_ttl == 0 {
                return Err(anyhow!("Redis default TTL must be greater than 0"));
            }
        }

        // Validate Neo4j configuration if present
        if let Some(ref neo4j) = self.config.databases.neo4j {
            if neo4j.pool_size == 0 {
                return Err(anyhow!("Neo4j pool size must be greater than 0"));
            }
            if neo4j.username.is_empty() {
                return Err(anyhow!("Neo4j username cannot be empty"));
            }
            if neo4j.password.is_empty() {
                return Err(anyhow!("Neo4j password cannot be empty"));
            }
        }

        // Validate FAISS configuration if present
        if let Some(ref faiss) = self.config.databases.faiss {
            if faiss.dimension == 0 {
                return Err(anyhow!("FAISS dimension must be greater than 0"));
            }
            if faiss.index_type.is_empty() {
                return Err(anyhow!("FAISS index type cannot be empty"));
            }
        }

        // Validate performance configuration
        if self.config.performance.cache_size == 0 {
            return Err(anyhow!("Cache size must be greater than 0"));
        }

        if self.config.performance.batch_size == 0 {
            return Err(anyhow!("Batch size must be greater than 0"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_config_default() {
        let config = LTMCConfig::default();
        assert!(config.enabled);
        assert!(config.features.pattern_learning);
        assert!(config.performance.caching);
    }

    #[tokio::test]
    async fn test_config_manager() {
        let mut manager = LTMCConfigManager::new();
        assert!(manager.is_enabled());

        manager.set_enabled(false);
        assert!(!manager.is_enabled());

        assert!(manager.is_feature_enabled("pattern_learning"));
        manager
            .set_feature_enabled("pattern_learning", false)
            .unwrap();
        assert!(!manager.is_feature_enabled("pattern_learning"));
    }

    #[tokio::test]
    async fn test_config_file_operations() -> Result<()> {
        let temp_file = NamedTempFile::new()?;
        let temp_path = temp_file.path();

        let mut manager = LTMCConfigManager::with_config_file(temp_path);

        // Modify config
        manager.set_enabled(false);
        manager.set_feature_enabled("pattern_learning", false)?;

        // Save config
        manager.save_config().await?;

        // Create new manager and load config
        let mut manager2 = LTMCConfigManager::with_config_file(temp_path);
        manager2.load_config().await?;

        assert!(!manager2.is_enabled());
        assert!(!manager2.is_feature_enabled("pattern_learning"));

        Ok(())
    }

    #[tokio::test]
    async fn test_config_validation() -> Result<()> {
        let mut manager = LTMCConfigManager::new();

        // Valid config should pass
        assert!(manager.validate().is_ok());

        // Invalid pool size should fail
        manager.get_config_mut().databases.sqlite.pool_size = 0;
        assert!(manager.validate().is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_available_databases() {
        let manager = LTMCConfigManager::new();
        let databases = manager.get_available_databases();

        // SQLite should always be available
        assert!(databases.contains(&"sqlite".to_string()));

        // Other databases depend on compile-time features
        #[cfg(feature = "ltmc-redis")]
        assert!(!databases.contains(&"redis".to_string())); // Not configured by default

        #[cfg(not(feature = "ltmc-redis"))]
        assert!(!databases.contains(&"redis".to_string())); // Feature not enabled
    }
}
