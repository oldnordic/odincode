//! Execute queries against metadata database (files and symbols)
//!
//! Handles querying files and symbols with flexible filtering,
//! ordering, and pagination support.

use anyhow::Result;
use sqlx::Row;

use super::query_interface::DatabaseQueryInterface;
use super::query_types::{QueryParams, QueryResult};
use crate::database::{FileMetadata, SymbolInfo};

impl DatabaseQueryInterface {
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

        let rows = query_builder
            .fetch_all(self.db_manager.metadata_pool())
            .await?;

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

        let rows = query_builder
            .fetch_all(self.db_manager.metadata_pool())
            .await?;

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
}
