//! Query execution logic for OdinCode databases.
//!
//! This module contains the functions that execute queries against the metadata,
//! graph, and RAG databases.

use crate::database::{DatabaseManager, FileMetadata, SymbolInfo};
use crate::database_queries::converters;
use crate::database_queries::types::{
    DatabaseStats, QueryParams, QueryResult, SearchQuery,
};
use crate::graph_database::{GraphNode, GraphRelationship};
use crate::rag_database::CodeChunk;
use anyhow::Result;
use sqlx::Row;
use std::collections::HashMap;

/// Query files with filters
pub async fn query_files(db_manager: &DatabaseManager, params: QueryParams) -> Result<QueryResult> {
    let mut query = "SELECT id, path, language, content, created_at, modified_at, size FROM files WHERE 1=1".to_string();
    let mut bind_params = Vec::new();

    for (key, value) in &params.filters {
        match key.as_str() {
            "language" => {
                query.push_str(" AND language = ?");
                bind_params.push(value.clone());
            }
            "path_contains" => {
                query.push_str(" AND path LIKE ?");
                bind_params.push(format!("%{}%", value));
            }
            "extension" => {
                query.push_str(" AND path LIKE ?");
                bind_params.push(format!("%.{}", value));
            }
            _ => {}
        }
    }

    if let Some(order_by) = &params.order_by {
        query.push_str(&format!(" ORDER BY {}", order_by));
    }
    if let Some(limit) = params.limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }
    if let Some(offset) = params.offset {
        query.push_str(&format!(" OFFSET {}", offset));
    }

    let mut query_builder = sqlx::query(&query);
    for param in &bind_params {
        query_builder = query_builder.bind(param);
    }

    let rows = query_builder.fetch_all(db_manager.metadata_pool()).await?;
    let mut files = Vec::new();
    for row in rows {
        files.push(FileMetadata {
            id: row.get("id"),
            path: row.get("path"),
            language: row.get("language"),
            content: row.get("content"),
            created_at: row.get("created_at"),
            modified_at: row.get("modified_at"),
            size: row.get("size"),
        });
    }
    Ok(QueryResult::Files(files))
}

/// Query symbols with filters
pub async fn query_symbols(db_manager: &DatabaseManager, params: QueryParams) -> Result<QueryResult> {
    let mut query = "SELECT id, name, kind, file_path, line, column, scope, visibility, language FROM symbols WHERE 1=1".to_string();
    let mut bind_params = Vec::new();

    for (key, value) in &params.filters {
        match key.as_str() {
            "name" => {
                query.push_str(" AND name = ?");
                bind_params.push(value.clone());
            }
            "kind" => {
                query.push_str(" AND kind = ?");
                bind_params.push(value.clone());
            }
            "file_path" => {
                query.push_str(" AND file_path = ?");
                bind_params.push(value.clone());
            }
            "language" => {
                query.push_str(" AND language = ?");
                bind_params.push(value.clone());
            }
            "name_contains" => {
                query.push_str(" AND name LIKE ?");
                bind_params.push(format!("%{}%", value));
            }
            _ => {}
        }
    }

    if let Some(order_by) = &params.order_by {
        query.push_str(&format!(" ORDER BY {}", order_by));
    }
    if let Some(limit) = params.limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }
    if let Some(offset) = params.offset {
        query.push_str(&format!(" OFFSET {}", offset));
    }

    let mut query_builder = sqlx::query(&query);
    for param in &bind_params {
        query_builder = query_builder.bind(param);
    }

    let rows = query_builder.fetch_all(db_manager.metadata_pool()).await?;
    let mut symbols = Vec::new();
    for row in rows {
        symbols.push(SymbolInfo {
            id: row.get("id"),
            name: row.get("name"),
            kind: row.get("kind"),
            file_path: row.get("file_path"),
            line: row.get::<i64, _>("line") as u32,
            column: row.get::<i64, _>("column") as u32,
            scope: row.get("scope"),
            visibility: row.get("visibility"),
            language: row.get("language"),
        });
    }
    Ok(QueryResult::Symbols(symbols))
}

/// Query graph nodes with filters
pub async fn query_nodes(db_manager: &DatabaseManager, params: QueryParams) -> Result<QueryResult> {
    let mut query = "SELECT id, name, node_type, file_path, properties, created_at, updated_at FROM graph_nodes WHERE 1=1".to_string();
    let mut bind_params = Vec::new();

    for (key, value) in &params.filters {
        match key.as_str() {
            "name" => {
                query.push_str(" AND name = ?");
                bind_params.push(value.clone());
            }
            "node_type" => {
                query.push_str(" AND node_type = ?");
                bind_params.push(value.clone());
            }
            "file_path" => {
                query.push_str(" AND file_path = ?");
                bind_params.push(value.clone());
            }
            "name_contains" => {
                query.push_str(" AND name LIKE ?");
                bind_params.push(format!("%{}%", value));
            }
            _ => {}
        }
    }

    if let Some(order_by) = &params.order_by {
        query.push_str(&format!(" ORDER BY {}", order_by));
    }
    if let Some(limit) = params.limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }
    if let Some(offset) = params.offset {
        query.push_str(&format!(" OFFSET {}", offset));
    }

    let mut query_builder = sqlx::query(&query);
    for param in &bind_params {
        query_builder = query_builder.bind(param);
    }

    let rows = query_builder.fetch_all(db_manager.graph_pool()).await?;
    let mut nodes = Vec::new();
    for row in rows {
        let properties: HashMap<String, String> =
            serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();
        nodes.push(GraphNode {
            id: row.get("id"),
            name: row.get("name"),
            node_type: converters::str_to_node_type(row.get::<&str, _>("node_type"))?,
            file_path: row.get("file_path"),
            properties,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }
    Ok(QueryResult::Nodes(nodes))
}

/// Query graph relationships with filters
pub async fn query_relationships(db_manager: &DatabaseManager, params: QueryParams) -> Result<QueryResult> {
    let mut query = "SELECT id, from_node_id, to_node_id, relationship_type, properties, created_at FROM graph_relationships WHERE 1=1".to_string();
    let mut bind_params = Vec::new();

    for (key, value) in &params.filters {
        match key.as_str() {
            "from_node_id" => {
                query.push_str(" AND from_node_id = ?");
                bind_params.push(value.clone());
            }
            "to_node_id" => {
                query.push_str(" AND to_node_id = ?");
                bind_params.push(value.clone());
            }
            "relationship_type" => {
                query.push_str(" AND relationship_type = ?");
                bind_params.push(value.clone());
            }
            _ => {}
        }
    }

    if let Some(order_by) = &params.order_by {
        query.push_str(&format!(" ORDER BY {}", order_by));
    }
    if let Some(limit) = params.limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }
    if let Some(offset) = params.offset {
        query.push_str(&format!(" OFFSET {}", offset));
    }

    let mut query_builder = sqlx::query(&query);
    for param in &bind_params {
        query_builder = query_builder.bind(param);
    }

    let rows = query_builder.fetch_all(db_manager.graph_pool()).await?;
    let mut relationships = Vec::new();
    for row in rows {
        let properties: HashMap<String, String> =
            serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();
        relationships.push(GraphRelationship {
            id: row.get("id"),
            from_node_id: row.get("from_node_id"),
            to_node_id: row.get("to_node_id"),
            relationship_type: converters::str_to_relationship_type(
                row.get::<&str, _>("relationship_type"),
            )?,
            properties,
            created_at: row.get("created_at"),
        });
    }
    Ok(QueryResult::Relationships(relationships))
}

/// Query code chunks with filters
pub async fn query_chunks(db_manager: &DatabaseManager, params: QueryParams) -> Result<QueryResult> {
    let mut query = "SELECT id, file_path, chunk_type, content, start_line, end_line, embedding, semantic_hash, metadata, created_at, updated_at FROM code_chunks WHERE 1=1".to_string();
    let mut bind_params = Vec::new();

    for (key, value) in &params.filters {
        match key.as_str() {
            "file_path" => {
                query.push_str(" AND file_path = ?");
                bind_params.push(value.clone());
            }
            "chunk_type" => {
                query.push_str(" AND chunk_type = ?");
                bind_params.push(value.clone());
            }
            "content_contains" => {
                query.push_str(" AND content LIKE ?");
                bind_params.push(format!("%{}%", value));
            }
            _ => {}
        }
    }

    if let Some(order_by) = &params.order_by {
        query.push_str(&format!(" ORDER BY {}", order_by));
    }
    if let Some(limit) = params.limit {
        query.push_str(&format!(" LIMIT {}", limit));
    }
    if let Some(offset) = params.offset {
        query.push_str(&format!(" OFFSET {}", offset));
    }

    let mut query_builder = sqlx::query(&query);
    for param in &bind_params {
        query_builder = query_builder.bind(param);
    }

    let rows = query_builder.fetch_all(db_manager.rag_pool()).await?;
    let mut chunks = Vec::new();
    for row in rows {
        let embedding: Option<Vec<u8>> = row.get("embedding");
        let embedding = if let Some(embedding_bytes) = embedding {
            Some(bincode::deserialize(&embedding_bytes)?)
        } else {
            None
        };
        let metadata: HashMap<String, String> =
            serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();
        chunks.push(CodeChunk {
            id: row.get("id"),
            file_path: row.get("file_path"),
            chunk_type: converters::str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
            content: row.get("content"),
            start_line: row.get::<i64, _>("start_line") as u32,
            end_line: row.get::<i64, _>("end_line") as u32,
            embedding,
            semantic_hash: row.get("semantic_hash"),
            metadata,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        });
    }
    Ok(QueryResult::Chunks(chunks))
}

/// Perform a cross-database search
pub async fn search(_db_manager: &DatabaseManager, search_query: SearchQuery) -> Result<QueryResult> {
    if let Some(_vector) = &search_query.semantic_vector {
        return Ok(QueryResult::SearchHits(Vec::new()));
    }
    if let Some(_text) = &search_query.text {
        return Ok(QueryResult::SearchHits(Vec::new()));
    }
    Ok(QueryResult::SearchHits(Vec::new()))
}

/// Get database statistics
pub async fn get_stats(db_manager: &DatabaseManager) -> Result<QueryResult> {
    let files_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
        .fetch_one(db_manager.metadata_pool())
        .await?;
    let symbols_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM symbols")
        .fetch_one(db_manager.metadata_pool())
        .await?;
    let nodes_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_nodes")
        .fetch_one(db_manager.graph_pool())
        .await?;
    let relationships_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_relationships")
        .fetch_one(db_manager.graph_pool())
        .await?;
    let chunks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM code_chunks")
        .fetch_one(db_manager.rag_pool())
        .await?;

    let stats = DatabaseStats {
        total_files: files_count as u32,
        total_symbols: symbols_count as u32,
        total_nodes: nodes_count as u32,
        total_relationships: relationships_count as u32,
        total_chunks: chunks_count as u32,
        metadata_db_size_mb: 0,
        graph_db_size_mb: 0,
        rag_db_size_mb: 0,
    };
    Ok(QueryResult::Stats(stats))
}

/// Find related entities across databases
pub async fn find_related(db_manager: &DatabaseManager, entity_id: &str, entity_type: &str) -> Result<QueryResult> {
    match entity_type {
        "file" => {
            let symbols = sqlx::query(
                "SELECT id, name, kind, file_path, line, column, scope, visibility, language FROM symbols WHERE file_path = ?",
            )
            .bind(entity_id)
            .fetch_all(db_manager.metadata_pool())
            .await?;
            let mut symbol_infos = Vec::new();
            for row in symbols {
                symbol_infos.push(SymbolInfo {
                    id: row.get("id"),
                    name: row.get("name"),
                    kind: row.get("kind"),
                    file_path: row.get("file_path"),
                    line: row.get::<i64, _>("line") as u32,
                    column: row.get::<i64, _>("column") as u32,
                    scope: row.get("scope"),
                    visibility: row.get("visibility"),
                    language: row.get("language"),
                });
            }
            Ok(QueryResult::Symbols(symbol_infos))
        }
        "symbol" => {
            let symbol_row = sqlx::query("SELECT file_path FROM symbols WHERE id = ?")
                .bind(entity_id)
                .fetch_optional(db_manager.metadata_pool())
                .await?;
            if let Some(row) = symbol_row {
                let file_path: String = row.get("file_path");
                let chunks = query_chunks(
                    db_manager,
                    QueryParams {
                        filters: [("file_path".to_string(), file_path)].iter().cloned().collect(),
                        ..Default::default()
                    },
                )
                .await?;
                Ok(chunks)
            } else {
                Ok(QueryResult::Chunks(Vec::new()))
            }
        }
        "node" => {
            let relationships = sqlx::query(
                "SELECT id, from_node_id, to_node_id, relationship_type, properties, created_at FROM graph_relationships WHERE from_node_id = ? OR to_node_id = ?",
            )
            .bind(entity_id)
            .bind(entity_id)
            .fetch_all(db_manager.graph_pool())
            .await?;
            let mut rels = Vec::new();
            for row in relationships {
                let properties: HashMap<String, String> =
                    serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();
                rels.push(GraphRelationship {
                    id: row.get("id"),
                    from_node_id: row.get("from_node_id"),
                    to_node_id: row.get("to_node_id"),
                    relationship_type: converters::str_to_relationship_type(
                        row.get::<&str, _>("relationship_type"),
                    )?,
                    properties,
                    created_at: row.get("created_at"),
                });
            }
            Ok(QueryResult::Relationships(rels))
        }
        _ => Ok(QueryResult::Custom(serde_json::Value::Null)),
    }
}

/// Execute a custom SQL query (for advanced use cases)
pub async fn execute_custom_query(
    _db_manager: &DatabaseManager,
    _query: &str,
    _params: Vec<String>,
) -> Result<QueryResult> {
    Ok(QueryResult::Custom(serde_json::Value::Null))
}
