//! Execute queries against graph database (nodes and relationships)
//!
//! Handles querying graph nodes and relationships with flexible filtering,
//! ordering, and pagination support.

use anyhow::Result;
use sqlx::Row;
use std::collections::HashMap;

use super::convert_types::{str_to_node_type, str_to_relationship_type};
use super::query_interface::DatabaseQueryInterface;
use super::query_types::{QueryParams, QueryResult};
use crate::graph_database::{GraphNode, GraphRelationship};

impl DatabaseQueryInterface {
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

        let rows = query_builder
            .fetch_all(self.db_manager.graph_pool())
            .await?;

        let mut nodes = Vec::new();
        for row in rows {
            let properties: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();

            nodes.push(GraphNode {
                id: row.get("id"),
                name: row.get("name"),
                node_type: str_to_node_type(row.get::<&str, _>("node_type"))?,
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

        let rows = query_builder
            .fetch_all(self.db_manager.graph_pool())
            .await?;

        let mut relationships = Vec::new();
        for row in rows {
            let properties: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();

            relationships.push(GraphRelationship {
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

        Ok(QueryResult::Relationships(relationships))
    }
}
