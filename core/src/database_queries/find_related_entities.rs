//! Find related entities across databases
//!
//! Provides cross-database entity relationship discovery,
//! linking files, symbols, nodes, and chunks.

use anyhow::Result;
use sqlx::Row;
use std::collections::HashMap;

use super::convert_types::str_to_relationship_type;
use super::query_interface::DatabaseQueryInterface;
use super::query_types::{QueryParams, QueryResult};
use crate::database::SymbolInfo;
use crate::graph_database::GraphRelationship;

impl DatabaseQueryInterface {
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
            }
            "symbol" => {
                // Find the file this symbol belongs to
                let symbol_row = sqlx::query("SELECT file_path FROM symbols WHERE id = ?")
                    .bind(entity_id)
                    .fetch_optional(self.db_manager.metadata_pool())
                    .await?;

                if let Some(row) = symbol_row {
                    let file_path: String = row.get("file_path");
                    // Find chunks in this file
                    let chunks = self
                        .query_chunks(QueryParams {
                            limit: Some(100),
                            offset: Some(0),
                            order_by: None,
                            filters: [("file_path".to_string(), file_path)]
                                .iter()
                                .cloned()
                                .collect(),
                            include_related: false,
                        })
                        .await?;
                    Ok(chunks)
                } else {
                    Ok(QueryResult::Chunks(Vec::new()))
                }
            }
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
                        relationship_type: str_to_relationship_type(
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
        &self,
        _query: &str,
        _params: Vec<String>,
    ) -> Result<QueryResult> {
        // This is a simplified implementation - in practice, you'd want to be more careful
        // about SQL injection and would probably want to specify which database to query

        // For now, we'll just return an empty result
        Ok(QueryResult::Custom(serde_json::Value::Null))
    }
}
