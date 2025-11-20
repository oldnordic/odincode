//! The main database query interface for OdinCode.
//!
//! This module defines the `DatabaseQueryInterface` struct, which serves as the
//! primary entry point for all database query operations.

use crate::database::DatabaseManager;
use crate::database_queries::executors;
use crate::database_queries::types::{QueryParams, QueryResult, SearchQuery};
use anyhow::Result;

/// Unified query interface for all databases
pub struct DatabaseQueryInterface {
    db_manager: DatabaseManager,
}

impl DatabaseQueryInterface {
    /// Create a new query interface
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self { db_manager }
    }

    /// Query files with filters
    pub async fn query_files(&self, params: QueryParams) -> Result<QueryResult> {
        executors::query_files(&self.db_manager, params).await
    }

    /// Query symbols with filters
    pub async fn query_symbols(&self, params: QueryParams) -> Result<QueryResult> {
        executors::query_symbols(&self.db_manager, params).await
    }

    /// Query graph nodes with filters
    pub async fn query_nodes(&self, params: QueryParams) -> Result<QueryResult> {
        executors::query_nodes(&self.db_manager, params).await
    }

    /// Query graph relationships with filters
    pub async fn query_relationships(&self, params: QueryParams) -> Result<QueryResult> {
        executors::query_relationships(&self.db_manager, params).await
    }

    /// Query code chunks with filters
    pub async fn query_chunks(&self, params: QueryParams) -> Result<QueryResult> {
        executors::query_chunks(&self.db_manager, params).await
    }

    /// Perform a cross-database search
    pub async fn search(&self, search_query: SearchQuery) -> Result<QueryResult> {
        executors::search(&self.db_manager, search_query).await
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<QueryResult> {
        executors::get_stats(&self.db_manager).await
    }

    /// Find related entities across databases
    pub async fn find_related(&self, entity_id: &str, entity_type: &str) -> Result<QueryResult> {
        executors::find_related(&self.db_manager, entity_id, entity_type).await
    }

    /// Execute a custom SQL query (for advanced use cases)
    pub async fn execute_custom_query(&self, query: &str, params: Vec<String>) -> Result<QueryResult> {
        executors::execute_custom_query(&self.db_manager, query, params).await
    }
}
