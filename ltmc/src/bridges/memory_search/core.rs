//! LTMC Memory Search Bridge Core
//!
//! This module contains the core implementation of the memory search bridge.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use odincode_databases::{DatabaseManager, DatabaseType};

/// Memory search bridge for LTMC system
#[derive(Clone)]
pub struct MemorySearchBridge {
    /// Reference to the database manager
    pub database_manager: Arc<DatabaseManager>,
    /// Cache for database connection IDs
    pub connection_ids: Arc<RwLock<HashMap<DatabaseType, Uuid>>>,
    /// Flag indicating if the bridge is initialized
    pub initialized: bool,
}

impl MemorySearchBridge {
    /// Create a new memory search bridge
    pub fn new(database_manager: DatabaseManager) -> Self {
        Self {
            database_manager: Arc::new(database_manager),
            connection_ids: Arc::new(RwLock::new(HashMap::new())),
            initialized: false,
        }
    }

    /// Initialize the memory search bridge
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing LTMC Memory Search Bridge...");

        // Register database connections
        self.register_database_connections().await?;

        // Test all connections
        self.test_all_connections().await?;

        self.initialized = true;
        info!("LTMC Memory Search Bridge initialized successfully");
        Ok(())
    }

    /// Register database connections for the LTMC system
    async fn register_database_connections(&self) -> Result<()> {
        info!("Registering database connections for LTMC...");

        // Register SQLite connection
        let mut sqlite_props = HashMap::new();
        sqlite_props.insert("pool_size".to_string(), "10".to_string());
        sqlite_props.insert("journal_mode".to_string(), "WAL".to_string());
        sqlite_props.insert("synchronous".to_string(), "NORMAL".to_string());

        let sqlite_id = self
            .database_manager
            .register_connection(
                DatabaseType::SQLite,
                "LTMC_SQLite".to_string(),
                "sqlite:///tmp/ltmc.db".to_string(),
                sqlite_props,
            )
            .await?;

        // Register Neo4j connection
        let mut neo4j_props = HashMap::new();
        neo4j_props.insert("pool_size".to_string(), "5".to_string());
        neo4j_props.insert("encryption".to_string(), "false".to_string());

        let neo4j_id = self
            .database_manager
            .register_connection(
                DatabaseType::Neo4j,
                "LTMC_Neo4j".to_string(),
                "neo4j://localhost:7687".to_string(),
                neo4j_props,
            )
            .await?;

        // Register Redis connection
        let mut redis_props = HashMap::new();
        redis_props.insert("pool_size".to_string(), "5".to_string());
        redis_props.insert("ttl".to_string(), "3600".to_string());

        let redis_id = self
            .database_manager
            .register_connection(
                DatabaseType::Redis,
                "LTMC_Redis".to_string(),
                "redis://localhost:6379".to_string(),
                redis_props,
            )
            .await?;

        // Register FAISS connection
        let mut faiss_props = HashMap::new();
        faiss_props.insert("dimension".to_string(), "768".to_string());
        faiss_props.insert("index_type".to_string(), "IVFFlat".to_string());

        let faiss_id = self
            .database_manager
            .register_connection(
                DatabaseType::FAISS,
                "LTMC_FAISS".to_string(),
                "/tmp/ltmc_faiss.index".to_string(),
                faiss_props,
            )
            .await?;

        // Store connection IDs for quick access
        let mut ids = self.connection_ids.write().await;
        ids.insert(DatabaseType::SQLite, sqlite_id);
        ids.insert(DatabaseType::Neo4j, neo4j_id);
        ids.insert(DatabaseType::Redis, redis_id);
        ids.insert(DatabaseType::FAISS, faiss_id);
        drop(ids);

        info!("Registered all database connections for LTMC");
        Ok(())
    }

    /// Test all registered database connections
    async fn test_all_connections(&self) -> Result<()> {
        info!("Testing all database connections...");

        let connection_ids: Vec<Uuid> = {
            let ids = self.connection_ids.read().await;
            ids.values().cloned().collect()
        };

        for id in connection_ids {
            match self.database_manager.test_connection(id).await {
                Ok(success) => {
                    if success {
                        info!("Connection test succeeded for connection: {id}");
                    } else {
                        error!("Connection test failed for connection: {id}");
                        return Err(anyhow::anyhow!(
                            "Connection test failed for connection: {id}"
                        ));
                    }
                }
                Err(e) => {
                    error!("Failed to test connection {id}: {e}");
                    return Err(anyhow::anyhow!("Failed to test connection {id}: {e}"));
                }
            }
        }

        info!("All database connections tested successfully");
        Ok(())
    }

    /// Check if the bridge is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}
