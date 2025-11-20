//! Query types and parameter structures for database operations
//!
//! This module defines the core types used across all database query operations,
//! including query parameters, results, and statistics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::database::{FileMetadata, SymbolInfo};
use crate::graph_database::{GraphNode, GraphRelationship};
use crate::rag_database::{CodeChunk, SearchHit};

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
