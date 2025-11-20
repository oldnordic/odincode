//! Graph database module for OdinCode
//!
//! This module provides graph database functionality using SQLite with graph extensions
//! to represent code relationships, dependencies, and architecture patterns.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

/// Graph node representing an entity in the codebase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub name: String,
    pub node_type: NodeType,
    pub file_path: Option<String>,
    pub properties: HashMap<String, String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Types of graph nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeType {
    Function,
    Class,
    Module,
    Variable,
    Interface,
    Enum,
    Struct,
    Trait,
    File,
    Package,
    Import,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeType::Function => "function",
            NodeType::Class => "class",
            NodeType::Module => "module",
            NodeType::Variable => "variable",
            NodeType::Interface => "interface",
            NodeType::Enum => "enum",
            NodeType::Struct => "struct",
            NodeType::Trait => "trait",
            NodeType::File => "file",
            NodeType::Package => "package",
            NodeType::Import => "import",
        }
    }
}

/// Graph relationship between two nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRelationship {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub relationship_type: RelationshipType,
    pub properties: HashMap<String, String>,
    pub created_at: i64,
}

/// Types of relationships between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Contains,   // Module contains function
    Imports,    // File imports module
    Calls,      // Function calls function
    Extends,    // Class extends class
    Implements, // Class implements interface
    Uses,       // Function uses variable
    Parameter,  // Function parameter
    Return,     // Function return type
    Field,      // Class field
    Dependency, // Dependency relationship
    Reference,  // General reference
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::Contains => "contains",
            RelationshipType::Imports => "imports",
            RelationshipType::Calls => "calls",
            RelationshipType::Extends => "extends",
            RelationshipType::Implements => "implements",
            RelationshipType::Uses => "uses",
            RelationshipType::Parameter => "parameter",
            RelationshipType::Return => "return",
            RelationshipType::Field => "field",
            RelationshipType::Dependency => "dependency",
            RelationshipType::Reference => "reference",
        }
    }
}

/// Graph database manager for code relationships
pub struct GraphDatabase {
    pool: SqlitePool,
}

impl GraphDatabase {
    /// Create a new graph database instance
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the graph database with required tables and indexes
    pub async fn init(&self) -> Result<()> {
        // Create nodes table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS graph_nodes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                node_type TEXT NOT NULL,
                file_path TEXT,
                properties TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create relationships table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS graph_relationships (
                id TEXT PRIMARY KEY,
                from_node_id TEXT NOT NULL,
                to_node_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                properties TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (from_node_id) REFERENCES graph_nodes (id),
                FOREIGN KEY (to_node_id) REFERENCES graph_nodes (id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_nodes_name ON graph_nodes(name)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_nodes_type ON graph_nodes(node_type)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_nodes_file ON graph_nodes(file_path)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_relationships_from ON graph_relationships(from_node_id)").execute(&self.pool).await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_relationships_to ON graph_relationships(to_node_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_relationships_type ON graph_relationships(relationship_type)").execute(&self.pool).await?;

        Ok(())
    }

    /// Create a new node in the graph
    pub async fn create_node(&self, node: GraphNode) -> Result<()> {
        let properties_json = serde_json::to_string(&node.properties)?;

        sqlx::query(
            r#"
            INSERT INTO graph_nodes (id, name, node_type, file_path, properties, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&node.id)
        .bind(&node.name)
        .bind(node.node_type.as_str())
        .bind(&node.file_path)
        .bind(&properties_json)
        .bind(node.created_at)
        .bind(node.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a node by ID
    pub async fn get_node(&self, node_id: &str) -> Result<Option<GraphNode>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, node_type, file_path, properties, created_at, updated_at
            FROM graph_nodes
            WHERE id = ?
            "#,
        )
        .bind(node_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let properties: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();

            Ok(Some(GraphNode {
                id: row.get("id"),
                name: row.get("name"),
                node_type: self.str_to_node_type(row.get::<&str, _>("node_type"))?,
                file_path: row.get("file_path"),
                properties,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Create a relationship between two nodes
    pub async fn create_relationship(&self, relationship: GraphRelationship) -> Result<()> {
        let properties_json = serde_json::to_string(&relationship.properties)?;

        sqlx::query(
            r#"
            INSERT INTO graph_relationships (id, from_node_id, to_node_id, relationship_type, properties, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&relationship.id)
        .bind(&relationship.from_node_id)
        .bind(&relationship.to_node_id)
        .bind(relationship.relationship_type.as_str())
        .bind(&properties_json)
        .bind(relationship.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get relationships for a node
    pub async fn get_node_relationships(&self, node_id: &str) -> Result<Vec<GraphRelationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_node_id, to_node_id, relationship_type, properties, created_at
            FROM graph_relationships
            WHERE from_node_id = ? OR to_node_id = ?
            "#,
        )
        .bind(node_id)
        .bind(node_id)
        .fetch_all(&self.pool)
        .await?;

        let mut relationships = Vec::new();
        for row in rows {
            let properties: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();

            relationships.push(GraphRelationship {
                id: row.get("id"),
                from_node_id: row.get("from_node_id"),
                to_node_id: row.get("to_node_id"),
                relationship_type: self
                    .str_to_relationship_type(row.get::<&str, _>("relationship_type"))?,
                properties,
                created_at: row.get("created_at"),
            });
        }

        Ok(relationships)
    }

    /// Find nodes by type and name pattern
    pub async fn find_nodes(
        &self,
        node_type: Option<NodeType>,
        name_pattern: Option<&str>,
    ) -> Result<Vec<GraphNode>> {
        let mut query = "SELECT id, name, node_type, file_path, properties, created_at, updated_at FROM graph_nodes WHERE 1=1".to_string();
        let mut params = Vec::new();

        if let Some(node_type) = node_type {
            query.push_str(" AND node_type = ?");
            params.push(node_type.as_str().to_string());
        }

        if let Some(pattern) = name_pattern {
            query.push_str(" AND name LIKE ?");
            params.push(format!("%{}%", pattern));
        }

        let mut query_builder = sqlx::query(&query);
        for param in &params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

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

        Ok(nodes)
    }

    /// Get the dependency graph for a specific file
    pub async fn get_file_dependency_graph(&self, file_path: &str) -> Result<DependencyGraph> {
        // Find all nodes in the file
        let file_nodes = self
            .find_nodes(None, None)
            .await? // This would need to be more specific in a real implementation
            .into_iter()
            .filter(|node| node.file_path.as_deref() == Some(file_path))
            .collect::<Vec<_>>();

        // Get all relationships for these nodes
        let mut all_relationships = Vec::new();
        for node in &file_nodes {
            let node_rels = self.get_node_relationships(&node.id).await?;
            all_relationships.extend(node_rels);
        }

        Ok(DependencyGraph {
            nodes: file_nodes,
            relationships: all_relationships,
        })
    }

    /// Find all nodes that depend on a specific node (reverse dependencies)
    pub async fn find_reverse_dependencies(&self, node_id: &str) -> Result<Vec<GraphRelationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_node_id, to_node_id, relationship_type, properties, created_at
            FROM graph_relationships
            WHERE to_node_id = ?
            "#,
        )
        .bind(node_id)
        .fetch_all(&self.pool)
        .await?;

        let mut relationships = Vec::new();
        for row in rows {
            let properties: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("properties")).unwrap_or_default();

            relationships.push(GraphRelationship {
                id: row.get("id"),
                from_node_id: row.get("from_node_id"),
                to_node_id: row.get("to_node_id"),
                relationship_type: self
                    .str_to_relationship_type(row.get::<&str, _>("relationship_type"))?,
                properties,
                created_at: row.get("created_at"),
            });
        }

        Ok(relationships)
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
}

/// Represents a dependency graph for analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyGraph {
    pub nodes: Vec<GraphNode>,
    pub relationships: Vec<GraphRelationship>,
}

impl DependencyGraph {
    /// Find circular dependencies in the graph
    pub fn find_circular_dependencies(&self) -> Vec<Vec<String>> {
        let mut circular_deps = Vec::new();

        // Build adjacency list
        let mut adj_list = std::collections::HashMap::new();
        for rel in &self.relationships {
            if matches!(
                rel.relationship_type,
                RelationshipType::Dependency | RelationshipType::Imports
            ) {
                adj_list
                    .entry(rel.from_node_id.clone())
                    .or_insert_with(Vec::new)
                    .push(rel.to_node_id.clone());
            }
        }

        // Detect cycles using DFS
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();
        let mut path = Vec::new();

        for node in &self.nodes {
            if !visited.contains(&node.id) {
                self.dfs_detect_cycle(
                    &node.id,
                    &adj_list,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                    &mut circular_deps,
                );
            }
        }

        circular_deps
    }

    /// Helper function for cycle detection using DFS
    fn dfs_detect_cycle(
        &self,
        node_id: &str,
        adj_list: &std::collections::HashMap<String, Vec<String>>,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
        path: &mut Vec<String>,
        circular_deps: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());
        path.push(node_id.to_string());

        if let Some(neighbors) = adj_list.get(node_id) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    self.dfs_detect_cycle(
                        neighbor,
                        adj_list,
                        visited,
                        rec_stack,
                        path,
                        circular_deps,
                    );
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle
                    let cycle_start_idx = path.iter().position(|x| x == neighbor).unwrap();
                    let cycle = path[cycle_start_idx..].to_vec();
                    circular_deps.push(cycle);
                }
            }
        }

        rec_stack.remove(node_id);
        path.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_graph_database_creation() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let graph_db = GraphDatabase::new(pool);
        graph_db.init().await.unwrap();

        // Verify tables were created by inserting and retrieving a node
        let node = GraphNode {
            id: "test_node".to_string(),
            name: "test_function".to_string(),
            node_type: NodeType::Function,
            file_path: Some("test.rs".to_string()),
            properties: HashMap::new(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        graph_db.create_node(node.clone()).await.unwrap();
        let retrieved = graph_db.get_node("test_node").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test_function");
    }

    #[tokio::test]
    async fn test_relationship_creation() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let graph_db = GraphDatabase::new(pool);
        graph_db.init().await.unwrap();

        // Create two nodes
        let node1 = GraphNode {
            id: "node1".to_string(),
            name: "function1".to_string(),
            node_type: NodeType::Function,
            file_path: Some("test1.rs".to_string()),
            properties: HashMap::new(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let node2 = GraphNode {
            id: "node2".to_string(),
            name: "function2".to_string(),
            node_type: NodeType::Function,
            file_path: Some("test2.rs".to_string()),
            properties: HashMap::new(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        graph_db.create_node(node1).await.unwrap();
        graph_db.create_node(node2).await.unwrap();

        // Create a relationship
        let relationship = GraphRelationship {
            id: "rel1".to_string(),
            from_node_id: "node1".to_string(),
            to_node_id: "node2".to_string(),
            relationship_type: RelationshipType::Calls,
            properties: HashMap::new(),
            created_at: 1234567890,
        };

        graph_db.create_relationship(relationship).await.unwrap();

        // Verify the relationship exists
        let rels = graph_db.get_node_relationships("node1").await.unwrap();
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].to_node_id, "node2");
    }

    #[tokio::test]
    async fn test_node_finding() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let graph_db = GraphDatabase::new(pool);
        graph_db.init().await.unwrap();

        // Create test nodes
        let node1 = GraphNode {
            id: Uuid::new_v4().to_string(),
            name: "authenticate_user".to_string(),
            node_type: NodeType::Function,
            file_path: Some("auth.rs".to_string()),
            properties: HashMap::new(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let node2 = GraphNode {
            id: Uuid::new_v4().to_string(),
            name: "validate_auth_token".to_string(),
            node_type: NodeType::Function,
            file_path: Some("auth.rs".to_string()),
            properties: HashMap::new(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        graph_db.create_node(node1.clone()).await.unwrap();
        graph_db.create_node(node2.clone()).await.unwrap();

        // Find nodes by name pattern
        let found_nodes = graph_db.find_nodes(None, Some("auth")).await.unwrap();
        assert_eq!(found_nodes.len(), 2);

        // Find nodes by type
        let function_nodes = graph_db
            .find_nodes(Some(NodeType::Function), None)
            .await
            .unwrap();
        assert_eq!(function_nodes.len(), 2);
    }
}
