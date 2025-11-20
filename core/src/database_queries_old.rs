//! Database query interfaces for OdinCode
//! 
//! This module provides unified query interfaces for accessing all three databases
//! (Metadata, Graph, RAG) with optimized query planning and execution.

use anyhow::Result;
use sqlx::Row;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import types from other modules
use crate::database::{DatabaseManager, FileMetadata, SymbolInfo};
use crate::graph_database::{GraphNode, GraphRelationship, NodeType, RelationshipType};
use crate::rag_database::{CodeChunk, ChunkType, SearchHit};

/// Unified query interface for all databases
pub struct DatabaseQueryInterface {
    db_manager: DatabaseManager,
}

/// Query result that can contain different types of data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryResult {
    Files(Vec<FileMetadata>),
    Symbols(Vec<SymbolInfo>),
    Nodes(Vec<GraphNode>),
    Relationships(Vec<GraphRelationship>),
    Chunks(Vec<CodeChunk>),
    SearchHits(Vec<SearchHit>),
    Stats(DatabaseStats),
    Custom(serde_json::Value),
}

/// Statistics from all databases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_files: u32,
    pub total_symbols: u32,
    pub total_nodes: u32,
    pub total_relationships: u32,
    pub total_chunks: u32,
    pub metadata_db_size_mb: u32,
    pub graph_db_size_mb: u32,
    pub rag_db_size_mb: u32,
}

/// Query parameters for flexible querying
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryParams {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub filters: HashMap<String, String>,
    pub include_related: bool,
}

impl Default for QueryParams {
    fn default() -> Self {
        Self {
            limit: Some(100),
            offset: Some(0),
            order_by: None,
            filters: HashMap::new(),
            include_related: false,
        }
    }
}

/// Cross-database search query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: Option<String>,
    pub semantic_vector: Option<Vec<f32>>,
    pub file_filters: Option<Vec<String>>,
    pub type_filters: Option<Vec<String>>,
    pub limit: u32,
}

impl DatabaseQueryInterface {
    /// Create a new query interface
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self { db_manager }
    }

    /// Query files with filters
    pub async fn query_files(&self, params: QueryParams) -> Result<QueryResult> {
        let mut query = "SELECT id, path, language, content, created_at, modified_at, size FROM files WHERE 1=1".to_string();
        let mut bind_params = Vec::new();

        // Apply filters
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

        // Apply ordering
        if let Some(order_by) = &params.order_by {
            query.push_str(&format!(" ORDER BY {}", order_by));
        }

        // Apply limit and offset
        if let Some(limit) = params.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = params.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute query
        let mut query_builder = sqlx::query(&query);
        for param in &bind_params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(self.db_manager.metadata_pool()).await?;

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
    pub async fn query_symbols(&self, params: QueryParams) -> Result<QueryResult> {
        let mut query = "SELECT id, name, kind, file_path, line, column, scope, visibility, language FROM symbols WHERE 1=1".to_string();
        let mut bind_params = Vec::new();

        // Apply filters
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

        // Apply ordering
        if let Some(order_by) = &params.order_by {
            query.push_str(&format!(" ORDER BY {}", order_by));
        }

        // Apply limit and offset
        if let Some(limit) = params.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = params.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute query
        let mut query_builder = sqlx::query(&query);
        for param in &bind_params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(self.db_manager.metadata_pool()).await?;

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
    pub async fn query_nodes(&self, params: QueryParams) -> Result<QueryResult> {
        let mut query = "SELECT id, name, node_type, file_path, properties, created_at, updated_at FROM graph_nodes WHERE 1=1".to_string();
        let mut bind_params = Vec::new();

        // Apply filters
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

        // Apply ordering
        if let Some(order_by) = &params.order_by {
            query.push_str(&format!(" ORDER BY {}", order_by));
        }

        // Apply limit and offset
        if let Some(limit) = params.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = params.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute query
        let mut query_builder = sqlx::query(&query);
        for param in &bind_params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(self.db_manager.graph_pool()).await?;

        let mut nodes = Vec::new();
        for row in rows {
            let properties: HashMap<String, String> = 
                serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();
            
            nodes.push(GraphNode {
                id: row.get("id"),
                name: row.get("name"),
                node_type: self.str_to_node_type(row.get::<&str, _>("node_type"))?,
                file_path: row.get("file_path"),
                properties,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(QueryResult::Nodes(nodes))
    }

    /// Query graph relationships with filters
    pub async fn query_relationships(&self, params: QueryParams) -> Result<QueryResult> {
        let mut query = "SELECT id, from_node_id, to_node_id, relationship_type, properties, created_at FROM graph_relationships WHERE 1=1".to_string();
        let mut bind_params = Vec::new();

        // Apply filters
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

        // Apply ordering
        if let Some(order_by) = &params.order_by {
            query.push_str(&format!(" ORDER BY {}", order_by));
        }

        // Apply limit and offset
        if let Some(limit) = params.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = params.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute query
        let mut query_builder = sqlx::query(&query);
        for param in &bind_params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(self.db_manager.graph_pool()).await?;

        let mut relationships = Vec::new();
        for row in rows {
            let properties: HashMap<String, String> = 
                serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();
            
            relationships.push(GraphRelationship {
                id: row.get("id"),
                from_node_id: row.get("from_node_id"),
                to_node_id: row.get("to_node_id"),
                relationship_type: self.str_to_relationship_type(row.get::<&str, _>("relationship_type"))?,
                properties,
                created_at: row.get("created_at"),
            });
        }

        Ok(QueryResult::Relationships(relationships))
    }

    /// Query code chunks with filters
    pub async fn query_chunks(&self, params: QueryParams) -> Result<QueryResult> {
        let mut query = "SELECT id, file_path, chunk_type, content, start_line, end_line, embedding, semantic_hash, metadata, created_at, updated_at FROM code_chunks WHERE 1=1".to_string();
        let mut bind_params = Vec::new();

        // Apply filters
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

        // Apply ordering
        if let Some(order_by) = &params.order_by {
            query.push_str(&format!(" ORDER BY {}", order_by));
        }

        // Apply limit and offset
        if let Some(limit) = params.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = params.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute query
        let mut query_builder = sqlx::query(&query);
        for param in &bind_params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(self.db_manager.rag_pool()).await?;

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
                chunk_type: self.str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
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
    pub async fn search(&self, search_query: SearchQuery) -> Result<QueryResult> {
        // If we have a semantic vector, perform semantic search
        if let Some(vector) = &search_query.semantic_vector {
            // This would typically call the RAG database's semantic search
            // For now, we'll return an empty result
            return Ok(QueryResult::SearchHits(Vec::new()));
        }

        // If we have text, perform text search
        if let Some(text) = &search_query.text {
            // This would typically call the RAG database's text search
            // For now, we'll return an empty result
            return Ok(QueryResult::SearchHits(Vec::new()));
        }

        // Default to empty result
        Ok(QueryResult::SearchHits(Vec::new()))
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<QueryResult> {
        // Count files
        let files_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
            .fetch_one(self.db_manager.metadata_pool())
            .await?;

        // Count symbols
        let symbols_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM symbols")
            .fetch_one(self.db_manager.metadata_pool())
            .await?;

        // Count graph nodes
        let nodes_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_nodes")
            .fetch_one(self.db_manager.graph_pool())
            .await?;

        // Count graph relationships
        let relationships_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_relationships")
            .fetch_one(self.db_manager.graph_pool())
            .await?;

        // Count code chunks
        let chunks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM code_chunks")
            .fetch_one(self.db_manager.rag_pool())
            .await?;

        // Get database sizes (simplified)
        let metadata_db_size = 0; // Would need to query PRAGMA for actual size
        let graph_db_size = 0;    // Would need to query PRAGMA for actual size
        let rag_db_size = 0;      // Would need to query PRAGMA for actual size

        let stats = DatabaseStats {
            total_files: files_count as u32,
            total_symbols: symbols_count as u32,
            total_nodes: nodes_count as u32,
            total_relationships: relationships_count as u32,
            total_chunks: chunks_count as u32,
            metadata_db_size_mb: metadata_db_size,
            graph_db_size_mb: graph_db_size,
            rag_db_size_mb: rag_db_size,
        };

        Ok(QueryResult::Stats(stats))
    }

    /// Find related entities across databases
    pub async fn find_related(&self, entity_id: &str, entity_type: &str) -> Result<QueryResult> {
        match entity_type {
            "file" => {
                // Find symbols in this file
                let symbols = sqlx::query(
                    "SELECT id, name, kind, file_path, line, column, scope, visibility, language FROM symbols WHERE file_path = ?"
                )
                .bind(entity_id)
                .fetch_all(self.db_manager.metadata_pool())
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
            },
            "symbol" => {
                // Find the file this symbol belongs to
                let symbol_row = sqlx::query(
                    "SELECT file_path FROM symbols WHERE id = ?"
                )
                .bind(entity_id)
                .fetch_optional(self.db_manager.metadata_pool())
                .await?;

                if let Some(row) = symbol_row {
                    let file_path: String = row.get("file_path");
                    // Find chunks in this file
                    let chunks = self.query_chunks(QueryParams {
                        limit: Some(100),
                        offset: Some(0),
                        order_by: None,
                        filters: [("file_path".to_string(), file_path)].iter().cloned().collect(),
                        include_related: false,
                    }).await?;
                    Ok(chunks)
                } else {
                    Ok(QueryResult::Chunks(Vec::new()))
                }
            },
            "node" => {
                // Find relationships for this node
                let relationships = sqlx::query(
                    "SELECT id, from_node_id, to_node_id, relationship_type, properties, created_at FROM graph_relationships WHERE from_node_id = ? OR to_node_id = ?"
                )
                .bind(entity_id)
                .bind(entity_id)
                .fetch_all(self.db_manager.graph_pool())
                .await?;

                let mut rels = Vec::new();
                for row in relationships {
                    let properties: HashMap<String, String> = 
                        serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();
                    
                    rels.push(GraphRelationship {
                        id: row.get("id"),
                        from_node_id: row.get("from_node_id"),
                        to_node_id: row.get("to_node_id"),
                        relationship_type: self.str_to_relationship_type(row.get::<&str, _>("relationship_type"))?,
                        properties,
                        created_at: row.get("created_at"),
                    });
                }

                Ok(QueryResult::Relationships(rels))
            },
            _ => Ok(QueryResult::Custom(serde_json::Value::Null)),
        }
    }

    /// Execute a custom SQL query (for advanced use cases)
    pub async fn execute_custom_query(&self, query: &str, params: Vec<String>) -> Result<QueryResult> {
        // This is a simplified implementation - in practice, you'd want to be more careful
        // about SQL injection and would probably want to specify which database to query
        
        // For now, we'll just return an empty result
        Ok(QueryResult::Custom(serde_json::Value::Null))
    }

    /// Convert string to NodeType
    fn str_to_node_type(&self, s: &str) -> Result<NodeType> {
        match s {
            "function" => Ok(NodeType::Function),
            "class" => Ok(NodeType::Class),
            "module" => Ok(NodeType::Module),
            "variable" => Ok(NodeType::Variable),
            "interface" => Ok(NodeType::Interface),
            "enum" => Ok(NodeType::Enum),
            "struct" => Ok(NodeType::Struct),
            "trait" => Ok(NodeType::Trait),
            "file" => Ok(NodeType::File),
            "package" => Ok(NodeType::Package),
            "import" => Ok(NodeType::Import),
            _ => Err(anyhow::anyhow!("Invalid node type: {}", s)),
        }
    }

    /// Convert string to RelationshipType
    fn str_to_relationship_type(&self, s: &str) -> Result<RelationshipType> {
        match s {
            "contains" => Ok(RelationshipType::Contains),
            "imports" => Ok(RelationshipType::Imports),
            "calls" => Ok(RelationshipType::Calls),
            "extends" => Ok(RelationshipType::Extends),
            "implements" => Ok(RelationshipType::Implements),
            "uses" => Ok(RelationshipType::Uses),
            "parameter" => Ok(RelationshipType::Parameter),
            "return" => Ok(RelationshipType::Return),
            "field" => Ok(RelationshipType::Field),
            "dependency" => Ok(RelationshipType::Dependency),
            "reference" => Ok(RelationshipType::Reference),
            _ => Err(anyhow::anyhow!("Invalid relationship type: {}", s)),
        }
    }

    /// Convert string to ChunkType
    fn str_to_chunk_type(&self, s: &str) -> Result<ChunkType> {
        match s {
            "function" => Ok(ChunkType::Function),
            "class" => Ok(ChunkType::Class),
            "method" => Ok(ChunkType::Method),
            "module" => Ok(ChunkType::Module),
            "block" => Ok(ChunkType::Block),
            "statement" => Ok(ChunkType::Statement),
            "expression" => Ok(ChunkType::Expression),
            "comment" => Ok(ChunkType::Comment),
            "documentation" => Ok(ChunkType::Documentation),
            "test" => Ok(ChunkType::Test),
            _ => Err(anyhow::anyhow!("Invalid chunk type: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
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

        // Initialize database schemas
        let graph_manager = crate::graph_database::GraphDatabaseManager::new(db_manager.graph_pool().clone());
        graph_manager.init().await.unwrap();

        let file_manager = crate::file_metadata::FileMetadataManager::new(db_manager.metadata_pool().clone());
        file_manager.init().await.unwrap();

        let rag_manager = crate::rag_database::RAGDatabaseManager::new(db_manager.rag_pool().clone());
        rag_manager.init().await.unwrap();

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
            metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
            graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = DatabaseManager::new(config).await.unwrap();

        // Initialize database schemas
        let file_manager = crate::file_metadata::FileMetadataManager::new(db_manager.metadata_pool().clone());
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
            metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
            graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = DatabaseManager::new(config).await.unwrap();
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
            metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
            graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = DatabaseManager::new(config).await.unwrap();
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
            metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
            graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = DatabaseManager::new(config).await.unwrap();
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
            metadata_db_path: temp_dir.path().join("metadata.db").to_string_lossy().to_string(),
            graph_db_path: temp_dir.path().join("graph.db").to_string_lossy().to_string(),
            rag_db_path: temp_dir.path().join("rag.db").to_string_lossy().to_string(),
            max_connections: 5,
        };

        let db_manager = DatabaseManager::new(config).await.unwrap();
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