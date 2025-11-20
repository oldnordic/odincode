//! Simple LTMC (Learning Through Meta-Cognition) System
//!
//! This module implements a simplified version of the LTMC system using only SQLite and FAISS,
//! without requiring an external MCP server. It provides core functionality for pattern storage,
//! task management, and semantic search while maintaining a simple, single-binary distribution.

#[cfg(feature = "simple-ltmc")]
pub mod api;
#[cfg(feature = "simple-ltmc")]
pub mod graph;
#[cfg(feature = "simple-ltmc")]
pub mod models;
#[cfg(feature = "simple-ltmc")]
pub mod search;
#[cfg(feature = "simple-ltmc")]
pub mod storage;

#[cfg(feature = "simple-ltmc")]
use anyhow::Result;
#[cfg(feature = "simple-ltmc")]
use faiss::index::IndexImpl;
#[cfg(feature = "simple-ltmc")]
use sqlx::sqlite::SqlitePool;
#[cfg(feature = "simple-ltmc")]
use std::sync::Arc;
#[cfg(feature = "simple-ltmc")]
use tokio::sync::RwLock;

/// Main manager for the simple LTMC system
#[cfg(feature = "simple-ltmc")]
pub struct SimpleLTMCManager {
    /// SQLite database connection pool
    pub db: SqlitePool,
    /// FAISS index for similarity search
    pub faiss_index: Arc<RwLock<IndexImpl>>,
    /// Storage layer for database operations
    pub storage: storage::StorageManager,
    /// Search layer for FAISS operations
    pub search: search::SearchManager,
    /// Graph operations layer
    pub graph: graph::GraphManager,
}

#[cfg(feature = "simple-ltmc")]
impl SimpleLTMCManager {
    /// Create a new SimpleLTMCManager instance
    pub async fn new(db_path: &str) -> Result<Self> {
        // Initialize SQLite database
        let db = storage::init_db(db_path).await?;

        // Initialize FAISS index
        let faiss_index = Arc::new(RwLock::new(search::SearchManager::create_faiss_index()?));

        // Create component managers
        let storage = storage::StorageManager::new(db.clone());
        let search = search::SearchManager::new(Arc::clone(&faiss_index));
        let graph = graph::GraphManager::new(db.clone());

        Ok(Self {
            db,
            faiss_index,
            storage,
            search,
            graph,
        })
    }

    /// Create a new SimpleLTMCManager instance (blocking version for use in constructors)
    pub fn new_blocking(db_path: &str) -> Result<Self> {
        let rt = tokio::runtime::Runtime::new()?;
        rt.block_on(async { Self::new(db_path).await })
    }
}

#[cfg(not(feature = "simple-ltmc"))]
pub struct SimpleLTMCManager;

#[cfg(not(feature = "simple-ltmc"))]
impl SimpleLTMCManager {
    pub async fn new(_db_path: &str) -> Result<Self> {
        anyhow::bail!("Simple LTMC feature is not enabled");
    }
}
