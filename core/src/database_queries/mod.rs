//! Database query interfaces for OdinCode
//!
//! This module provides unified query interfaces for accessing all three databases
//! (Metadata, Graph, RAG) with optimized query planning and execution.
//!
//! ## Architecture
//!
//! The module is organized following smart modularization principles:
//! - **query_types**: Core type definitions for queries and results
//! - **query_interface**: Main interface struct and constructor
//! - **execute_metadata_queries**: Query files and symbols from metadata database
//! - **execute_graph_queries**: Query nodes and relationships from graph database
//! - **execute_rag_queries**: Query code chunks and perform semantic search
//! - **execute_stats**: Collect statistics across all databases
//! - **find_related_entities**: Discover cross-database entity relationships
//! - **convert_types**: String-to-enum conversion helpers
//!
//! ## Example
//!
//! ```rust
//! use odincode_core::database_queries::{DatabaseQueryInterface, QueryParams};
//!
//! let interface = DatabaseQueryInterface::new(db_manager);
//! let params = QueryParams::default();
//! let results = interface.query_files(params).await?;
//! ```

// Public modules
pub mod convert_types;
pub mod query_interface;
pub mod query_types;

// Implementation modules (not directly public, accessed via trait implementations)
mod execute_graph_queries;
mod execute_metadata_queries;
mod execute_rag_queries;
mod execute_stats;
mod find_related_entities;

// Re-export main types for convenience
pub use query_interface::DatabaseQueryInterface;
pub use query_types::{DatabaseStats, QueryParams, QueryResult, SearchQuery};

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_query_interface_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::database::DatabaseConfig {
            metadata_db_path: temp_dir
                .path()
                .join("metadata.db")
                .to_string_lossy()
                .to_string(),
            graph_db_path: temp_dir
                .path()
                .join("graph.db")
                .to_string_lossy()
                .to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = crate::database::DatabaseManager::new(config).await.unwrap();

        // Initialize database schemas
        let graph_db = crate::graph_database::GraphDatabase::new(db_manager.graph_pool().clone());
        graph_db.init().await.unwrap();

        let file_manager =
            crate::file_metadata::FileMetadataManager::new(db_manager.metadata_pool().clone());
        file_manager.init().await.unwrap();

        let rag_db = crate::rag_database::RagDatabase::new(db_manager.rag_pool().clone());
        rag_db.init().await.unwrap();

        let query_interface = DatabaseQueryInterface::new(db_manager);

        // Test that we can get stats (even if they're empty)
        let stats_result = query_interface.get_stats().await.unwrap();
        match stats_result {
            QueryResult::Stats(stats) => {
                assert_eq!(stats.total_files, 0);
                assert_eq!(stats.total_symbols, 0);
                assert_eq!(stats.total_nodes, 0);
                assert_eq!(stats.total_relationships, 0);
                assert_eq!(stats.total_chunks, 0);
            }
            _ => panic!("Expected stats result"),
        }
    }

    #[tokio::test]
    async fn test_file_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::database::DatabaseConfig {
            metadata_db_path: temp_dir
                .path()
                .join("metadata.db")
                .to_string_lossy()
                .to_string(),
            graph_db_path: temp_dir
                .path()
                .join("graph.db")
                .to_string_lossy()
                .to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = crate::database::DatabaseManager::new(config).await.unwrap();

        // Initialize database schemas
        let file_manager =
            crate::file_metadata::FileMetadataManager::new(db_manager.metadata_pool().clone());
        file_manager.init().await.unwrap();

        let query_interface = DatabaseQueryInterface::new(db_manager);

        // Test querying files with empty database
        let params = QueryParams::default();
        let result = query_interface.query_files(params).await.unwrap();
        match result {
            QueryResult::Files(files) => {
                assert_eq!(files.len(), 0);
            }
            _ => panic!("Expected files result"),
        }
    }

    #[tokio::test]
    async fn test_symbol_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::database::DatabaseConfig {
            metadata_db_path: temp_dir
                .path()
                .join("metadata.db")
                .to_string_lossy()
                .to_string(),
            graph_db_path: temp_dir
                .path()
                .join("graph.db")
                .to_string_lossy()
                .to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = crate::database::DatabaseManager::new(config).await.unwrap();
        let query_interface = DatabaseQueryInterface::new(db_manager);

        // Test querying symbols with empty database
        let params = QueryParams::default();
        let result = query_interface.query_symbols(params).await.unwrap();
        match result {
            QueryResult::Symbols(symbols) => {
                assert_eq!(symbols.len(), 0);
            }
            _ => panic!("Expected symbols result"),
        }
    }

    #[tokio::test]
    async fn test_node_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::database::DatabaseConfig {
            metadata_db_path: temp_dir
                .path()
                .join("metadata.db")
                .to_string_lossy()
                .to_string(),
            graph_db_path: temp_dir
                .path()
                .join("graph.db")
                .to_string_lossy()
                .to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = crate::database::DatabaseManager::new(config).await.unwrap();

        // Initialize database schemas
        let graph_db = crate::graph_database::GraphDatabase::new(db_manager.graph_pool().clone());
        graph_db.init().await.unwrap();

        let query_interface = DatabaseQueryInterface::new(db_manager);

        // Test querying nodes with empty database
        let params = QueryParams::default();
        let result = query_interface.query_nodes(params).await.unwrap();
        match result {
            QueryResult::Nodes(nodes) => {
                assert_eq!(nodes.len(), 0);
            }
            _ => panic!("Expected nodes result"),
        }
    }

    #[tokio::test]
    async fn test_relationship_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::database::DatabaseConfig {
            metadata_db_path: temp_dir
                .path()
                .join("metadata.db")
                .to_string_lossy()
                .to_string(),
            graph_db_path: temp_dir
                .path()
                .join("graph.db")
                .to_string_lossy()
                .to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = crate::database::DatabaseManager::new(config).await.unwrap();

        // Initialize database schemas
        let graph_db = crate::graph_database::GraphDatabase::new(db_manager.graph_pool().clone());
        graph_db.init().await.unwrap();

        let query_interface = DatabaseQueryInterface::new(db_manager);

        // Test querying relationships with empty database
        let params = QueryParams::default();
        let result = query_interface.query_relationships(params).await.unwrap();
        match result {
            QueryResult::Relationships(relationships) => {
                assert_eq!(relationships.len(), 0);
            }
            _ => panic!("Expected relationships result"),
        }
    }

    #[tokio::test]
    async fn test_chunk_query() {
        let temp_dir = TempDir::new().unwrap();
        let config = crate::database::DatabaseConfig {
            metadata_db_path: temp_dir
                .path()
                .join("metadata.db")
                .to_string_lossy()
                .to_string(),
            graph_db_path: temp_dir
                .path()
                .join("graph.db")
                .to_string_lossy()
                .to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = crate::database::DatabaseManager::new(config).await.unwrap();

        // Initialize database schemas
        let rag_db = crate::rag_database::RagDatabase::new(db_manager.rag_pool().clone());
        rag_db.init().await.unwrap();

        let query_interface = DatabaseQueryInterface::new(db_manager);

        // Test querying chunks with empty database
        let params = QueryParams::default();
        let result = query_interface.query_chunks(params).await.unwrap();
        match result {
            QueryResult::Chunks(chunks) => {
                assert_eq!(chunks.len(), 0);
            }
            _ => panic!("Expected chunks result"),
        }
    }
}
