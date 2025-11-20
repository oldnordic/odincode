//! Tests for the database query interface.
//!
//! This module contains all the unit tests for the database query functionality.

use crate::database::DatabaseManager;
use crate::database_queries::interface::DatabaseQueryInterface;
use crate::database_queries::types::{QueryParams, QueryResult};
use tempfile::TempDir;

#[tokio::test]
async fn test_query_interface_creation() {
    let temp_dir = TempDir::new().unwrap();
    let config = crate::database::DatabaseConfig {
        metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
        graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
        rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
        max_connections: 5,
    };

    let db_manager = DatabaseManager::new(config).await.unwrap();

    // Mock schema initialization
    sqlx::query("CREATE TABLE files (id TEXT, path TEXT, language TEXT, content TEXT, created_at TEXT, modified_at TEXT, size INTEGER)").execute(db_manager.metadata_pool()).await.unwrap();
    sqlx::query("CREATE TABLE symbols (id TEXT, name TEXT, kind TEXT, file_path TEXT, line INTEGER, column INTEGER, scope TEXT, visibility TEXT, language TEXT)").execute(db_manager.metadata_pool()).await.unwrap();
    sqlx::query("CREATE TABLE graph_nodes (id TEXT, name TEXT, node_type TEXT, file_path TEXT, properties TEXT, created_at TEXT, updated_at TEXT)").execute(db_manager.graph_pool()).await.unwrap();
    sqlx::query("CREATE TABLE graph_relationships (id TEXT, from_node_id TEXT, to_node_id TEXT, relationship_type TEXT, properties TEXT, created_at TEXT)").execute(db_manager.graph_pool()).await.unwrap();
    sqlx::query("CREATE TABLE code_chunks (id TEXT, file_path TEXT, chunk_type TEXT, content TEXT, start_line INTEGER, end_line INTEGER, embedding BLOB, semantic_hash TEXT, metadata TEXT, created_at TEXT, updated_at TEXT)").execute(db_manager.rag_pool()).await.unwrap();

    let query_interface = DatabaseQueryInterface::new(db_manager);

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
        metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
        graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
        rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
        max_connections: 5,
    };
    let db_manager = DatabaseManager::new(config).await.unwrap();
    sqlx::query("CREATE TABLE files (id TEXT, path TEXT, language TEXT, content TEXT, created_at TEXT, modified_at TEXT, size INTEGER)").execute(db_manager.metadata_pool()).await.unwrap();
    let query_interface = DatabaseQueryInterface::new(db_manager);

    let params = QueryParams::default();
    let result = query_interface.query_files(params).await.unwrap();
    match result {
        QueryResult::Files(files) => assert_eq!(files.len(), 0),
        _ => panic!("Expected files result"),
    }
}

#[tokio::test]
async fn test_symbol_query() {
    let temp_dir = TempDir::new().unwrap();
    let config = crate::database::DatabaseConfig {
        metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
        graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
        rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
        max_connections: 5,
    };
    let db_manager = DatabaseManager::new(config).await.unwrap();
    sqlx::query("CREATE TABLE symbols (id TEXT, name TEXT, kind TEXT, file_path TEXT, line INTEGER, column INTEGER, scope TEXT, visibility TEXT, language TEXT)").execute(db_manager.metadata_pool()).await.unwrap();
    let query_interface = DatabaseQueryInterface::new(db_manager);

    let params = QueryParams::default();
    let result = query_interface.query_symbols(params).await.unwrap();
    match result {
        QueryResult::Symbols(symbols) => assert_eq!(symbols.len(), 0),
        _ => panic!("Expected symbols result"),
    }
}

#[tokio::test]
async fn test_node_query() {
    let temp_dir = TempDir::new().unwrap();
    let config = crate::database::DatabaseConfig {
        metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
        graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
        rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
        max_connections: 5,
    };
    let db_manager = DatabaseManager::new(config).await.unwrap();
    sqlx::query("CREATE TABLE graph_nodes (id TEXT, name TEXT, node_type TEXT, file_path TEXT, properties TEXT, created_at TEXT, updated_at TEXT)").execute(db_manager.graph_pool()).await.unwrap();
    let query_interface = DatabaseQueryInterface::new(db_manager);

    let params = QueryParams::default();
    let result = query_interface.query_nodes(params).await.unwrap();
    match result {
        QueryResult::Nodes(nodes) => assert_eq!(nodes.len(), 0),
        _ => panic!("Expected nodes result"),
    }
}

#[tokio::test]
async fn test_relationship_query() {
    let temp_dir = TempDir::new().unwrap();
    let config = crate::database::DatabaseConfig {
        metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
        graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
        rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
        max_connections: 5,
    };
    let db_manager = DatabaseManager::new(config).await.unwrap();
    sqlx::query("CREATE TABLE graph_relationships (id TEXT, from_node_id TEXT, to_node_id TEXT, relationship_type TEXT, properties TEXT, created_at TEXT)").execute(db_manager.graph_pool()).await.unwrap();
    let query_interface = DatabaseQueryInterface::new(db_manager);

    let params = QueryParams::default();
    let result = query_interface.query_relationships(params).await.unwrap();
    match result {
        QueryResult::Relationships(relationships) => assert_eq!(relationships.len(), 0),
        _ => panic!("Expected relationships result"),
    }
}

#[tokio::test]
async fn test_chunk_query() {
    let temp_dir = TempDir::new().unwrap();
    let config = crate::database::DatabaseConfig {
        metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
        graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
        rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
        max_connections: 5,
    };
    let db_manager = DatabaseManager::new(config).await.unwrap();
    sqlx::query("CREATE TABLE code_chunks (id TEXT, file_path TEXT, chunk_type TEXT, content TEXT, start_line INTEGER, end_line INTEGER, embedding BLOB, semantic_hash TEXT, metadata TEXT, created_at TEXT, updated_at TEXT)").execute(db_manager.rag_pool()).await.unwrap();
    let query_interface = DatabaseQueryInterface::new(db_manager);

    let params = QueryParams::default();
    let result = query_interface.query_chunks(params).await.unwrap();
    match result {
        QueryResult::Chunks(chunks) => assert_eq!(chunks.len(), 0),
        _ => panic!("Expected chunks result"),
    }
}
