//! Execute queries against RAG database (code chunks and semantic search)
//!
//! Handles querying code chunks with flexible filtering and performing
//! cross-database semantic searches.

use anyhow::Result;
use sqlx::Row;
use std::collections::HashMap;

use super::convert_types::str_to_chunk_type;
use super::query_interface::DatabaseQueryInterface;
use super::query_types::{QueryParams, QueryResult, SearchQuery};
use crate::rag_database::CodeChunk;

impl DatabaseQueryInterface {
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
                chunk_type: str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
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
        if let Some(_vector) = &search_query.semantic_vector {
            // This would typically call the RAG database's semantic search
            // For now, we'll return an empty result
            return Ok(QueryResult::SearchHits(Vec::new()));
        }

        // If we have text, perform text search
        if let Some(_text) = &search_query.text {
            // This would typically call the RAG database's text search
            // For now, we'll return an empty result
            return Ok(QueryResult::SearchHits(Vec::new()));
        }

        // Default to empty result
        Ok(QueryResult::SearchHits(Vec::new()))
    }
}
