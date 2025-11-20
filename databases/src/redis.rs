//! OdinCode Redis Database Manager
//!
//! This module provides Redis database operations for the LTMC system,
//! including caching, session management, and real-time data operations.

use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Redis-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    /// Connection pool size
    pub pool_size: usize,
    /// Default TTL for keys in seconds
    pub default_ttl: u64,
    /// Maximum retry attempts for failed operations
    pub max_retries: usize,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Command timeout in seconds
    pub command_timeout: u64,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: 5,
            default_ttl: 3600, // 1 hour
            max_retries: 3,
            connection_timeout: 10,
            command_timeout: 5,
        }
    }
}

/// Redis connection manager with connection pooling
#[derive(Clone)]
pub struct RedisManager {
    /// Redis client
    client: Arc<Client>,
    /// Connection manager for async operations
    connection_manager: Arc<RwLock<Option<ConnectionManager>>>,
    /// Configuration
    config: RedisConfig,
    /// Connection status
    is_connected: Arc<RwLock<bool>>,
    /// Statistics
    stats: Arc<RwLock<RedisStats>>,
}

/// Redis operation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisStats {
    /// Total operations performed
    pub total_operations: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
    /// Last operation timestamp
    pub last_operation: Option<DateTime<Utc>>,
}

impl Default for RedisStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            cache_hits: 0,
            cache_misses: 0,
            failed_operations: 0,
            avg_response_time_ms: 0.0,
            last_operation: None,
        }
    }
}

/// Redis key patterns for LTMC data
pub struct RedisKeyPatterns;

impl RedisKeyPatterns {
    /// Pattern for learning patterns: `ltmc:pattern:{id}`
    pub fn learning_pattern(id: &str) -> String {
        format!("ltmc:pattern:{id}")
    }

    /// Pattern for sequential thinking sessions: `ltmc:session:{id}`
    pub fn sequential_session(id: &str) -> String {
        format!("ltmc:session:{id}")
    }

    /// Pattern for session thoughts: `ltmc:session:{id}:thoughts`
    pub fn session_thoughts(id: &str) -> String {
        format!("ltmc:session:{id}:thoughts")
    }

    /// Pattern for user interactions: `ltmc:interaction:{id}`
    pub fn user_interaction(id: &str) -> String {
        format!("ltmc:interaction:{id}")
    }

    /// Pattern for pattern type indexes: `ltmc:index:pattern:{type}`
    pub fn pattern_type_index(pattern_type: &str) -> String {
        format!("ltmc:index:pattern:{pattern_type}")
    }

    /// Pattern for cache keys: `ltmc:cache:{category}:{key}`
    pub fn cache_key(category: &str, key: &str) -> String {
        format!("ltmc:cache:{category}:{key}")
    }

    /// Pattern for session locks: `ltmc:lock:session:{id}`
    pub fn session_lock(id: &str) -> String {
        format!("ltmc:lock:session:{id}")
    }
}

impl RedisManager {
    /// Create a new Redis manager
    pub fn new(config: RedisConfig) -> Result<Self> {
        let client = Client::open(config.url.clone())
            .map_err(|e| anyhow::anyhow!("Failed to create Redis client: {e}"))?;

        Ok(Self {
            client: Arc::new(client),
            connection_manager: Arc::new(RwLock::new(None)),
            config,
            is_connected: Arc::new(RwLock::new(false)),
            stats: Arc::new(RwLock::new(RedisStats::default())),
        })
    }

    /// Create a Redis manager from a connection string
    pub fn from_connection_string(connection_string: &str) -> Result<Self> {
        let config = RedisConfig {
            url: connection_string.to_string(),
            ..Default::default()
        };
        Self::new(config)
    }

    /// Initialize the Redis connection
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing Redis connection to {}", self.config.url);

        let start_time = std::time::Instant::now();

        // Create connection manager
        let manager = ConnectionManager::new((*self.client).clone())
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Redis connection manager: {e}"))?;

        // Test the connection
        let mut conn = manager.clone();
        let ping_result: Result<String> = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| anyhow::anyhow!("Redis PING failed: {e}"));

        match ping_result {
            Ok(response) => {
                if response == "PONG" {
                    info!("Redis connection successful: {response}");

                    // Store connection manager
                    let mut conn_manager = self.connection_manager.write().await;
                    *conn_manager = Some(manager);
                    drop(conn_manager);

                    // Update connection status
                    let mut connected = self.is_connected.write().await;
                    *connected = true;
                    drop(connected);

                    // Update statistics
                    let mut stats = self.stats.write().await;
                    stats.total_operations += 1;
                    stats.last_operation = Some(Utc::now());
                    let response_time = start_time.elapsed().as_millis() as f64;
                    stats.avg_response_time_ms = (stats.avg_response_time_ms
                        * (stats.total_operations - 1) as f64
                        + response_time)
                        / stats.total_operations as f64;
                    drop(stats);

                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "Redis PING returned unexpected response: {response}"
                    ))
                }
            }
            Err(e) => {
                error!("Redis connection failed: {e}");
                Err(e)
            }
        }
    }

    /// Test the Redis connection
    pub async fn test_connection(&self) -> Result<bool> {
        let start_time = std::time::Instant::now();

        let conn_manager = self.connection_manager.read().await;
        if conn_manager.is_none() {
            return Ok(false);
        }

        let mut manager = conn_manager.as_ref().unwrap().clone();
        drop(conn_manager);

        let result: Result<String> = redis::cmd("PING")
            .query_async(&mut manager)
            .await
            .map_err(|e| anyhow::anyhow!("Redis PING failed: {e}"));

        let success = result.is_ok();

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_operations += 1;
        stats.last_operation = Some(Utc::now());
        let response_time = start_time.elapsed().as_millis() as f64;
        stats.avg_response_time_ms =
            (stats.avg_response_time_ms * (stats.total_operations - 1) as f64 + response_time)
                / stats.total_operations as f64;

        if !success {
            stats.failed_operations += 1;
        }
        drop(stats);

        Ok(success)
    }

    /// Store a learning pattern in Redis
    pub async fn store_learning_pattern(&self, pattern: &str, ttl: Option<u64>) -> Result<()> {
        let key = RedisKeyPatterns::learning_pattern(pattern);
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let pattern = pattern.to_string();

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                let pattern = pattern.clone();
                async move {
                    let _: () = conn
                        .set_ex(&key, &pattern, ttl)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(())
                }
            })
        })
        .await
    }

    /// Retrieve a learning pattern from Redis
    pub async fn get_learning_pattern(&self, pattern_id: &str) -> Result<Option<String>> {
        let key = RedisKeyPatterns::learning_pattern(pattern_id);

        let start_time = std::time::Instant::now();

        let result: Option<String> = self
            .execute_with_retry(|mut conn| {
                Box::pin({
                    let key = key.clone();
                    async move {
                        conn.get(&key)
                            .await
                            .map_err(|e| anyhow::anyhow!("Redis error: {e}"))
                    }
                })
            })
            .await?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_operations += 1;
        stats.last_operation = Some(Utc::now());
        let response_time = start_time.elapsed().as_millis() as f64;
        stats.avg_response_time_ms =
            (stats.avg_response_time_ms * (stats.total_operations - 1) as f64 + response_time)
                / stats.total_operations as f64;

        if result.is_some() {
            stats.cache_hits += 1;
        } else {
            stats.cache_misses += 1;
        }
        drop(stats);

        Ok(result)
    }

    /// Store a sequential thinking session in Redis
    pub async fn store_sequential_session(
        &self,
        session_id: &str,
        session_data: &str,
        ttl: Option<u64>,
    ) -> Result<()> {
        let key = RedisKeyPatterns::sequential_session(session_id);
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let session_data = session_data.to_string();

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                let session_data = session_data.clone();
                async move {
                    let _: () = conn
                        .set_ex(&key, &session_data, ttl)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(())
                }
            })
        })
        .await
    }

    /// Retrieve a sequential thinking session from Redis
    pub async fn get_sequential_session(&self, session_id: &str) -> Result<Option<String>> {
        let key = RedisKeyPatterns::sequential_session(session_id);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                async move {
                    conn.get(&key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))
                }
            })
        })
        .await
    }

    /// Add a thought to a session's thought list
    pub async fn add_thought_to_session(&self, session_id: &str, thought_data: &str) -> Result<()> {
        let key = RedisKeyPatterns::session_thoughts(session_id);
        let ttl = self.config.default_ttl;
        let thought_data = thought_data.to_string();

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                let thought_data = thought_data.clone();
                async move {
                    let _: () = conn
                        .rpush(&key, &thought_data)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    // Set TTL on the list if it doesn't exist
                    let _: () = conn
                        .expire(&key, ttl as i64)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(())
                }
            })
        })
        .await
    }

    /// Get all thoughts for a session
    pub async fn get_session_thoughts(&self, session_id: &str) -> Result<Vec<String>> {
        let key = RedisKeyPatterns::session_thoughts(session_id);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                async move {
                    conn.lrange(&key, 0, -1)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))
                }
            })
        })
        .await
    }

    /// Store a user interaction in Redis
    pub async fn store_user_interaction(
        &self,
        interaction_id: &str,
        interaction_data: &str,
        ttl: Option<u64>,
    ) -> Result<()> {
        let key = RedisKeyPatterns::user_interaction(interaction_id);
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let interaction_data = interaction_data.to_string();

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                let interaction_data = interaction_data.clone();
                async move {
                    let _: () = conn
                        .set_ex(&key, &interaction_data, ttl)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(())
                }
            })
        })
        .await
    }

    /// Retrieve a user interaction from Redis
    pub async fn get_user_interaction(&self, interaction_id: &str) -> Result<Option<String>> {
        let key = RedisKeyPatterns::user_interaction(interaction_id);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                async move {
                    conn.get(&key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))
                }
            })
        })
        .await
    }

    /// Add a pattern ID to a type index
    pub async fn add_pattern_to_type_index(
        &self,
        pattern_type: &str,
        pattern_id: &str,
    ) -> Result<()> {
        let key = RedisKeyPatterns::pattern_type_index(pattern_type);
        let pattern_id = pattern_id.to_string();

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                let pattern_id = pattern_id.clone();
                async move {
                    let _: () = conn
                        .sadd(&key, &pattern_id)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(())
                }
            })
        })
        .await
    }

    /// Get all pattern IDs for a specific type
    pub async fn get_patterns_by_type(&self, pattern_type: &str) -> Result<Vec<String>> {
        let key = RedisKeyPatterns::pattern_type_index(pattern_type);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                async move {
                    conn.smembers(&key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))
                }
            })
        })
        .await
    }

    /// Store a cached value
    pub async fn cache_set(
        &self,
        category: &str,
        key: &str,
        value: &str,
        ttl: Option<u64>,
    ) -> Result<()> {
        let cache_key = RedisKeyPatterns::cache_key(category, key);
        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let value = value.to_string();

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let cache_key = cache_key.clone();
                let value = value.clone();
                async move {
                    let _: () = conn
                        .set_ex(&cache_key, &value, ttl)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(())
                }
            })
        })
        .await
    }

    /// Retrieve a cached value
    pub async fn cache_get(&self, category: &str, key: &str) -> Result<Option<String>> {
        let cache_key = RedisKeyPatterns::cache_key(category, key);

        let start_time = std::time::Instant::now();

        let result: Option<String> = self
            .execute_with_retry(|mut conn| {
                Box::pin({
                    let cache_key = cache_key.clone();
                    async move {
                        conn.get(&cache_key)
                            .await
                            .map_err(|e| anyhow::anyhow!("Redis error: {e}"))
                    }
                })
            })
            .await?;

        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_operations += 1;
        stats.last_operation = Some(Utc::now());
        let response_time = start_time.elapsed().as_millis() as f64;
        stats.avg_response_time_ms =
            (stats.avg_response_time_ms * (stats.total_operations - 1) as f64 + response_time)
                / stats.total_operations as f64;

        if result.is_some() {
            stats.cache_hits += 1;
        } else {
            stats.cache_misses += 1;
        }
        drop(stats);

        Ok(result)
    }

    /// Acquire a session lock (for exclusive access)
    pub async fn acquire_session_lock(
        &self,
        session_id: &str,
        timeout_seconds: u64,
    ) -> Result<bool> {
        let key = RedisKeyPatterns::session_lock(session_id);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                async move {
                    let result: bool = conn
                        .set_nx(&key, "locked")
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    if result {
                        let _: () = conn
                            .expire(&key, timeout_seconds as i64)
                            .await
                            .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    }
                    Ok(result)
                }
            })
        })
        .await
    }

    /// Release a session lock
    pub async fn release_session_lock(&self, session_id: &str) -> Result<bool> {
        let key = RedisKeyPatterns::session_lock(session_id);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                async move {
                    let result: i32 = conn
                        .del(&key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(result > 0)
                }
            })
        })
        .await
    }

    /// Delete a learning pattern
    pub async fn delete_learning_pattern(&self, pattern_id: &str) -> Result<bool> {
        let key = RedisKeyPatterns::learning_pattern(pattern_id);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let key = key.clone();
                async move {
                    let result: i32 = conn
                        .del(&key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(result > 0)
                }
            })
        })
        .await
    }

    /// Delete a sequential thinking session
    pub async fn delete_sequential_session(&self, session_id: &str) -> Result<bool> {
        let session_key = RedisKeyPatterns::sequential_session(session_id);
        let thoughts_key = RedisKeyPatterns::session_thoughts(session_id);
        let lock_key = RedisKeyPatterns::session_lock(session_id);

        self.execute_with_retry(|mut conn| {
            Box::pin({
                let session_key = session_key.clone();
                let thoughts_key = thoughts_key.clone();
                let lock_key = lock_key.clone();
                async move {
                    let _: () = conn
                        .del(&session_key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    let _: () = conn
                        .del(&thoughts_key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    let _: () = conn
                        .del(&lock_key)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                    Ok(true)
                }
            })
        })
        .await
    }

    /// Get Redis statistics
    pub async fn get_stats(&self) -> Result<RedisStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Clear all LTMC data (use with caution!)
    pub async fn clear_all_data(&self) -> Result<()> {
        warn!("Clearing all LTMC data from Redis");

        self.execute_with_retry(|mut conn| {
            Box::pin(async move {
                // Get all keys with LTMC prefix
                let keys: Vec<String> = conn
                    .keys("ltmc:*")
                    .await
                    .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;

                if !keys.is_empty() {
                    let _: () = conn
                        .del(&keys)
                        .await
                        .map_err(|e| anyhow::anyhow!("Redis error: {e}"))?;
                }

                Ok(())
            })
        })
        .await
    }

    /// Execute a Redis command with retry logic
    async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn(ConnectionManager) -> futures::future::BoxFuture<'static, Result<T>>,
        T: Send + 'static,
    {
        let mut last_error = None;

        for attempt in 0..self.config.max_retries {
            let conn_manager = self.connection_manager.read().await;
            if conn_manager.is_none() {
                return Err(anyhow::anyhow!("Redis connection not initialized"));
            }

            let manager = conn_manager.as_ref().unwrap().clone();
            drop(conn_manager);

            match operation(manager).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries - 1 {
                        warn!(
                            "Redis operation failed (attempt {}/{}), retrying...",
                            attempt + 1,
                            self.config.max_retries
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(
                            100 * (attempt + 1) as u64,
                        ))
                        .await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!(
                "Redis operation failed after {} retries",
                self.config.max_retries
            )
        }))
    }

    /// Check if connected to Redis
    pub async fn is_connected(&self) -> bool {
        let connected = self.is_connected.read().await;
        *connected
    }

    /// Get the Redis configuration
    pub fn get_config(&self) -> &RedisConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_redis_manager_creation() {
        let config = RedisConfig::default();
        let manager = RedisManager::new(config).unwrap();
        assert!(!manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_redis_from_connection_string() {
        let manager = RedisManager::from_connection_string("redis://localhost:6379").unwrap();
        assert_eq!(manager.get_config().url, "redis://localhost:6379");
    }

    #[tokio::test]
    async fn test_redis_key_patterns() {
        assert_eq!(
            RedisKeyPatterns::learning_pattern("test123"),
            "ltmc:pattern:test123"
        );
        assert_eq!(
            RedisKeyPatterns::sequential_session("session456"),
            "ltmc:session:session456"
        );
        assert_eq!(
            RedisKeyPatterns::session_thoughts("session456"),
            "ltmc:session:session456:thoughts"
        );
        assert_eq!(
            RedisKeyPatterns::user_interaction("interaction789"),
            "ltmc:interaction:interaction789"
        );
        assert_eq!(
            RedisKeyPatterns::pattern_type_index("code"),
            "ltmc:index:pattern:code"
        );
        assert_eq!(
            RedisKeyPatterns::cache_key("patterns", "key123"),
            "ltmc:cache:patterns:key123"
        );
        assert_eq!(
            RedisKeyPatterns::session_lock("session456"),
            "ltmc:lock:session:session456"
        );
    }

    #[tokio::test]
    async fn test_redis_stats() {
        let config = RedisConfig::default();
        let manager = RedisManager::new(config).unwrap();
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_operations, 0);
        assert_eq!(stats.cache_hits, 0);
        assert_eq!(stats.cache_misses, 0);
        assert_eq!(stats.failed_operations, 0);
    }

    // Integration tests require a running Redis server
    // These are marked as ignored so they don't fail in CI/CD
    #[tokio::test]
    #[ignore]
    async fn test_redis_integration() {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 1,
            default_ttl: 60,
            max_retries: 1,
            connection_timeout: 5,
            command_timeout: 3,
        };

        let manager = RedisManager::new(config).unwrap();

        // Test connection
        let init_result = manager.initialize().await;
        assert!(init_result.is_ok());
        assert!(manager.is_connected().await);

        // Test basic operations
        let test_pattern = r#"{"id": "test123", "type": "code", "content": "test pattern"}"#;
        manager
            .store_learning_pattern("test123", Some(60))
            .await
            .unwrap();

        let retrieved = manager.get_learning_pattern("test123").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_pattern);

        // Test session operations
        let session_data = r#"{"id": "session456", "context": "test session"}"#;
        manager
            .store_sequential_session("session456", session_data, Some(60))
            .await
            .unwrap();

        let retrieved_session = manager.get_sequential_session("session456").await.unwrap();
        assert!(retrieved_session.is_some());
        assert_eq!(retrieved_session.unwrap(), session_data);

        // Test thought operations
        let thought_data = r#"{"id": "thought789", "content": "test thought"}"#;
        manager
            .add_thought_to_session("session456", thought_data)
            .await
            .unwrap();

        let thoughts = manager.get_session_thoughts("session456").await.unwrap();
        assert_eq!(thoughts.len(), 1);
        assert_eq!(thoughts[0], thought_data);

        // Test cache operations
        manager
            .cache_set("test", "cache_key", "cache_value", Some(30))
            .await
            .unwrap();

        let cached_value = manager.cache_get("test", "cache_key").await.unwrap();
        assert!(cached_value.is_some());
        assert_eq!(cached_value.unwrap(), "cache_value");

        // Test type indexing
        manager
            .add_pattern_to_type_index("code", "test123")
            .await
            .unwrap();

        let code_patterns = manager.get_patterns_by_type("code").await.unwrap();
        assert!(code_patterns.contains(&"test123".to_string()));

        // Test statistics
        let stats = manager.get_stats().await.unwrap();
        assert!(stats.total_operations > 0);
        assert!(stats.cache_hits > 0);
        assert!(stats.last_operation.is_some());

        // Test cleanup
        manager.delete_learning_pattern("test123").await.unwrap();
        manager
            .delete_sequential_session("session456")
            .await
            .unwrap();

        // Verify deletion
        let deleted_pattern = manager.get_learning_pattern("test123").await.unwrap();
        assert!(deleted_pattern.is_none());

        let deleted_session = manager.get_sequential_session("session456").await.unwrap();
        assert!(deleted_session.is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_redis_session_locking() {
        let config = RedisConfig {
            url: "redis://localhost:6379".to_string(),
            pool_size: 1,
            default_ttl: 60,
            max_retries: 1,
            connection_timeout: 5,
            command_timeout: 3,
        };

        let manager = RedisManager::new(config).unwrap();
        manager.initialize().await.unwrap();

        let session_id = "lock_test_session";

        // Acquire lock
        let lock_acquired = manager.acquire_session_lock(session_id, 10).await.unwrap();
        assert!(lock_acquired);

        // Try to acquire lock again (should fail)
        let lock_acquired_again = manager.acquire_session_lock(session_id, 10).await.unwrap();
        assert!(!lock_acquired_again);

        // Release lock
        let lock_released = manager.release_session_lock(session_id).await.unwrap();
        assert!(lock_released);

        // Now should be able to acquire lock again
        let lock_acquired_final = manager.acquire_session_lock(session_id, 10).await.unwrap();
        assert!(lock_acquired_final);

        // Clean up
        manager.release_session_lock(session_id).await.unwrap();
    }
}
