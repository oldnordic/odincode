//! Graph operations for the Simple LTMC system using SQLite JSON functions
//!
//! Implements graph-like operations using SQLite's JSON capabilities to store
//! and query relationships between tasks and patterns.

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use super::models::{EntityType, Pattern, Relationship, RelationshipType, Task};

/// Manager for graph operations using SQLite JSON
pub struct GraphManager {
    db: SqlitePool,
}

impl GraphManager {
    /// Create a new GraphManager instance
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Create a relationship between two entities
    pub async fn create_relationship(
        &self,
        from_id: Uuid,
        to_id: Uuid,
        rel_type: RelationshipType,
    ) -> Result<()> {
        // Determine entity types (for simplicity, we'll assume they're both patterns for this example)
        // In a real implementation, you'd want to determine the actual types
        let from_type = EntityType::Pattern; // Could be determined by checking which table contains the ID
        let to_type = EntityType::Pattern;

        // Insert the relationship into the relationships table
        sqlx::query(
            r#"
            INSERT INTO relationships (id, from_id, from_type, to_id, to_type, relationship_type, created_at)
            VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(from_id.to_string())
        .bind(serde_json::to_string(&from_type)?)
        .bind(to_id.to_string())
        .bind(serde_json::to_string(&to_type)?)
        .bind(serde_json::to_string(&rel_type)?)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get all relationships for a specific entity
    pub async fn get_relationships(&self, entity_id: Uuid) -> Result<Vec<Relationship>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, from_id, from_type, to_id, to_type, relationship_type, created_at
            FROM relationships
            WHERE from_id = ? OR to_id = ?
            "#,
        )
        .bind(entity_id.to_string())
        .bind(entity_id.to_string())
        .fetch_all(&self.db)
        .await?;

        let mut relationships = Vec::new();
        for row in rows {
            let relationship = Relationship {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                from_id: Uuid::parse_str(row.get::<&str, _>("from_id"))?,
                from_type: serde_json::from_str(row.get::<&str, _>("from_type"))?,
                to_id: Uuid::parse_str(row.get::<&str, _>("to_id"))?,
                to_type: serde_json::from_str(row.get::<&str, _>("to_type"))?,
                relationship_type: serde_json::from_str(row.get::<&str, _>("relationship_type"))?,
                metadata: std::collections::HashMap::new(), // For now, empty metadata
                created_at: chrono::DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&chrono::Utc),
            };
            relationships.push(relationship);
        }

        Ok(relationships)
    }

    /// Get all outgoing relationships from a specific entity (entity -> related entities)
    pub async fn get_outgoing_relationships(&self, entity_id: Uuid) -> Result<Vec<Relationship>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, from_id, from_type, to_id, to_type, relationship_type, created_at
            FROM relationships
            WHERE from_id = ?
            "#,
        )
        .bind(entity_id.to_string())
        .fetch_all(&self.db)
        .await?;

        let mut relationships = Vec::new();
        for row in rows {
            let relationship = Relationship {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                from_id: Uuid::parse_str(row.get::<&str, _>("from_id"))?,
                from_type: serde_json::from_str(row.get::<&str, _>("from_type"))?,
                to_id: Uuid::parse_str(row.get::<&str, _>("to_id"))?,
                to_type: serde_json::from_str(row.get::<&str, _>("to_type"))?,
                relationship_type: serde_json::from_str(row.get::<&str, _>("relationship_type"))?,
                metadata: std::collections::HashMap::new(),
                created_at: chrono::DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&chrono::Utc),
            };
            relationships.push(relationship);
        }

        Ok(relationships)
    }

    /// Get all incoming relationships to a specific entity (related entities -> entity)
    pub async fn get_incoming_relationships(&self, entity_id: Uuid) -> Result<Vec<Relationship>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, from_id, from_type, to_id, to_type, relationship_type, created_at
            FROM relationships
            WHERE to_id = ?
            "#,
        )
        .bind(entity_id.to_string())
        .fetch_all(&self.db)
        .await?;

        let mut relationships = Vec::new();
        for row in rows {
            let relationship = Relationship {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                from_id: Uuid::parse_str(row.get::<&str, _>("from_id"))?,
                from_type: serde_json::from_str(row.get::<&str, _>("from_type"))?,
                to_id: Uuid::parse_str(row.get::<&str, _>("to_id"))?,
                to_type: serde_json::from_str(row.get::<&str, _>("to_type"))?,
                relationship_type: serde_json::from_str(row.get::<&str, _>("relationship_type"))?,
                metadata: std::collections::HashMap::new(),
                created_at: chrono::DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&chrono::Utc),
            };
            relationships.push(relationship);
        }

        Ok(relationships)
    }

    /// Find connected entities within a certain depth (graph traversal)
    pub async fn find_connected_entities(
        &self,
        start_id: Uuid,
        max_depth: u32,
        entity_type: Option<EntityType>,
        rel_type_filter: Option<RelationshipType>,
    ) -> Result<Vec<(Uuid, EntityType, u32)>> {
        // (entity_id, type, depth)
        let mut visited = std::collections::HashSet::new();
        let mut result = Vec::new();
        let mut current_level = vec![(start_id, 0u32)]; // (id, depth)

        while !current_level.is_empty() && current_level[0].1 < max_depth {
            let mut next_level = Vec::new();

            for (entity_id, depth) in current_level {
                if visited.contains(&entity_id) {
                    continue;
                }

                visited.insert(entity_id);

                // Get outgoing relationships
                let outgoing = self.get_outgoing_relationships(entity_id).await?;
                for rel in outgoing {
                    if let Some(ref filter) = rel_type_filter {
                        if &rel.relationship_type != filter {
                            continue;
                        }
                    }

                    let target_type = rel.to_type.clone();
                    if let Some(ref filter_type) = entity_type {
                        if &target_type != filter_type {
                            continue;
                        }
                    }

                    let next_depth = depth + 1;
                    result.push((rel.to_id, target_type, next_depth));
                    next_level.push((rel.to_id, next_depth));
                }

                // Get incoming relationships
                let incoming = self.get_incoming_relationships(entity_id).await?;
                for rel in incoming {
                    if let Some(ref filter) = rel_type_filter {
                        if &rel.relationship_type != filter {
                            continue;
                        }
                    }

                    let source_type = rel.from_type.clone();
                    if let Some(ref filter_type) = entity_type {
                        if &source_type != filter_type {
                            continue;
                        }
                    }

                    let next_depth = depth + 1;
                    result.push((rel.from_id, source_type, next_depth));
                    next_level.push((rel.from_id, next_depth));
                }
            }

            current_level = next_level;
        }

        Ok(result)
    }

    /// Get task hierarchy (parent-child relationships using JSON in SQLite)
    pub async fn get_task_hierarchy(&self, root_task_id: Uuid) -> Result<Vec<Task>> {
        // Use recursive query to get all child tasks
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE task_tree AS (
                -- Base case: start with the root task
                SELECT id, parent_task_id, title, description, status, priority, 
                       created_at, completed_at, 0 as level
                FROM tasks 
                WHERE id = ?
                
                UNION ALL
                
                -- Recursive case: get all child tasks
                SELECT t.id, t.parent_task_id, t.title, t.description, t.status, t.priority,
                       t.created_at, t.completed_at, tt.level + 1
                FROM tasks t
                JOIN task_tree tt ON t.parent_task_id = tt.id
                WHERE tt.level < 10  -- Prevent infinite recursion
            )
            SELECT id, parent_task_id, title, description, status, priority, 
                   created_at, completed_at
            FROM task_tree
            ORDER BY level, created_at
            "#,
        )
        .bind(root_task_id.to_string())
        .fetch_all(&self.db)
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            // Note: We're only pulling basic fields here for simplicity
            // A full implementation would map all Task fields
            let task = Task {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                parent_task_id: row
                    .get::<Option<String>, _>("parent_task_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                prd_id: None, // Not in the query, but would be available in the real table
                title: row.get("title"),
                description: row.get("description"),
                status: serde_json::from_str(r#""Todo""#)?, // Placeholder - need to store as JSON in DB
                priority: serde_json::from_str(r#""Normal""#)?, // Placeholder
                estimated_time: None,
                actual_time: None,
                created_at: chrono::DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&chrono::Utc),
                completed_at: row
                    .get::<Option<&str>, _>("completed_at")
                    .map(|s| {
                        chrono::DateTime::parse_from_rfc3339(s)
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    })
                    .transpose()?,
                dependencies: Vec::new(),  // Would come from JSON in the DB
                related_files: Vec::new(), // Would come from JSON in the DB
                metadata: std::collections::HashMap::new(), // Would come from JSON in the DB
            };
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Find related patterns using semantic similarity and relationships
    pub async fn find_related_patterns(
        &self,
        pattern_id: Uuid,
        search_manager: &super::search::SearchManager,
        limit: usize,
    ) -> Result<Vec<(Pattern, f32)>> {
        // (pattern, similarity_score)
        // First, get the pattern to get its content
        let storage_manager = super::storage::StorageManager::new(self.db.clone());
        let pattern = storage_manager.get_pattern(pattern_id).await?;

        if let Some(ref pattern) = pattern {
            // Get similar patterns using FAISS
            let embedding = search_manager.create_embedding(&pattern.content).await?;
            let similar_pattern_ids = search_manager
                .search_similar_patterns(&embedding, limit)
                .await?;

            // Fetch the pattern details for each similar pattern
            let mut related_patterns = Vec::new();
            for (similar_id, similarity) in similar_pattern_ids {
                if similar_id != pattern_id {
                    // Don't include the pattern itself
                    if let Some(similar_pattern) = storage_manager.get_pattern(similar_id).await? {
                        related_patterns.push((similar_pattern, similarity));
                    }
                }
            }

            // Also get patterns connected via relationships
            let relationships = self.get_relationships(pattern_id).await?;
            for relationship in &relationships {
                if relationship.relationship_type == RelationshipType::Similar {
                    // Get the related pattern
                    let rel_pattern_id = if relationship.from_id == pattern_id {
                        relationship.to_id
                    } else {
                        relationship.from_id
                    };

                    if let Some(rel_pattern) = storage_manager.get_pattern(rel_pattern_id).await? {
                        // Check if this pattern is already in our results
                        if !related_patterns.iter().any(|(p, _)| p.id == rel_pattern_id) {
                            related_patterns.push((rel_pattern, 0.9)); // High confidence for explicit relationships
                        }
                    }
                }
            }

            // Sort by similarity score (descending)
            related_patterns
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            // Limit the results
            related_patterns.truncate(limit);

            Ok(related_patterns)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get dependency chains for a task
    pub async fn get_task_dependencies(&self, task_id: Uuid) -> Result<Vec<Task>> {
        // Get the task's dependencies from its metadata (stored as JSON in the database)
        let row = sqlx::query("SELECT dependencies FROM tasks WHERE id = ?")
            .bind(task_id.to_string())
            .fetch_optional(&self.db)
            .await?;

        if let Some(row) = row {
            let dependencies_json: &str = row.get("dependencies");
            let dependencies: Vec<String> =
                serde_json::from_str(dependencies_json).unwrap_or_default();

            let mut dependency_tasks = Vec::new();
            for dep_id_str in dependencies {
                if let Ok(dep_id) = Uuid::parse_str(&dep_id_str) {
                    // Get the dependent task
                    let storage_manager = super::storage::StorageManager::new(self.db.clone());
                    if let Some(task) = storage_manager.get_task(dep_id).await? {
                        dependency_tasks.push(task);
                    }
                }
            }

            Ok(dependency_tasks)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get tasks that depend on a specific task (reverse dependencies)
    pub async fn get_dependent_tasks(&self, task_id: Uuid) -> Result<Vec<Task>> {
        // Find tasks that have this task as a dependency
        let rows = sqlx::query(
            r#"
            SELECT id, dependencies FROM tasks
            WHERE dependencies LIKE ?
            "#,
        )
        .bind(format!("%{}%", task_id)) // Simple approach, in practice would use JSON functions
        .fetch_all(&self.db)
        .await?;

        let mut dependent_tasks = Vec::new();
        for row in rows {
            let task_id_str: String = row.get("id");
            if let Ok(task_uuid) = Uuid::parse_str(&task_id_str) {
                let storage_manager = super::storage::StorageManager::new(self.db.clone());
                if let Some(task) = storage_manager.get_task(task_uuid).await? {
                    dependent_tasks.push(task);
                }
            }
        }

        Ok(dependent_tasks)
    }

    /// Complex query: Find all patterns related to a task through its connected entities
    pub async fn get_task_related_patterns(
        &self,
        task_id: Uuid,
        search_manager: &super::search::SearchManager,
        max_depth: u32,
    ) -> Result<Vec<(Pattern, u32)>> {
        // (pattern, relationship_depth)
        // First get the task
        let storage_manager = super::storage::StorageManager::new(self.db.clone());
        let task = storage_manager.get_task(task_id).await?;

        match task {
            Some(_) => {
                // Find connected entities starting from the task
                let connected_entities = self
                    .find_connected_entities(task_id, max_depth, Some(EntityType::Pattern), None)
                    .await?;

                let mut related_patterns = Vec::new();
                for (entity_id, entity_type, depth) in connected_entities {
                    if entity_type == EntityType::Pattern {
                        if let Some(pattern) = storage_manager.get_pattern(entity_id).await? {
                            related_patterns.push((pattern, depth));
                        }
                    }
                }

                Ok(related_patterns)
            }
            None => Ok(Vec::new()),
        }
    }
}
