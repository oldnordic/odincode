//! SQLite storage layer for the Simple LTMC system
//!
//! Implements database operations for tasks, patterns, and relationships using
//! SQLite with JSON functions for graph-like relationships.

use super::models::{EntityType, Pattern, ProductRequirement, Relationship, Task};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Manager for database operations
pub struct StorageManager {
    db: SqlitePool,
}

impl StorageManager {
    /// Create a new StorageManager instance
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Initialize the database schema
    pub async fn init_db(&self) -> Result<()> {
        // Create tasks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                parent_task_id TEXT,
                prd_id TEXT,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                estimated_time INTEGER,
                actual_time INTEGER,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                dependencies TEXT,        -- JSON array of UUIDs
                related_files TEXT,       -- JSON array of file paths
                metadata TEXT             -- JSON object for additional data
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Create patterns table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS patterns (
                id TEXT PRIMARY KEY,
                pattern_type TEXT NOT NULL,
                title TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_accessed TEXT NOT NULL,
                access_count INTEGER DEFAULT 0,
                confidence REAL DEFAULT 1.0,
                embedding BLOB,           -- Serialized embedding vector
                context TEXT,             -- JSON object for context
                related_patterns TEXT     -- JSON array of related patterns
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Create relationships table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS relationships (
                id TEXT PRIMARY KEY,
                from_id TEXT NOT NULL,
                from_type TEXT NOT NULL,
                to_id TEXT NOT NULL,
                to_type TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                metadata TEXT,            -- JSON object for relationship metadata
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Create PRDs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS prds (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                overview TEXT,
                goals TEXT,               -- JSON array of goal strings
                user_stories TEXT,        -- JSON array of user stories
                functional_requirements TEXT,  -- JSON array of requirements
                non_goals TEXT,           -- JSON array of non-goals
                design_considerations TEXT,    -- JSON array of design considerations
                technical_considerations TEXT, -- JSON array of technical considerations
                success_metrics TEXT,     -- JSON array of success metrics
                open_questions TEXT,      -- JSON array of open questions
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                status TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Create indexes for better performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_task_id)")
            .execute(&self.db)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_prd ON tasks(prd_id)")
            .execute(&self.db)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)")
            .execute(&self.db)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_relationships_from ON relationships(from_id)")
            .execute(&self.db)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_relationships_to ON relationships(to_id)")
            .execute(&self.db)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_patterns_type ON patterns(pattern_type)")
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Create a new task in the database
    pub async fn create_task(&self, task: &Task) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tasks (
                id, parent_task_id, prd_id, title, description, status, priority,
                estimated_time, actual_time, created_at, completed_at,
                dependencies, related_files, metadata
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(task.id.to_string())
        .bind(task.parent_task_id.map(|id| id.to_string()))
        .bind(task.prd_id.map(|id| id.to_string()))
        .bind(&task.title)
        .bind(&task.description)
        .bind(serde_json::to_string(&task.status)?)
        .bind(serde_json::to_string(&task.priority)?)
        .bind(task.estimated_time)
        .bind(task.actual_time)
        .bind(task.created_at.to_rfc3339())
        .bind(task.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(serde_json::to_string(&task.dependencies)?)
        .bind(serde_json::to_string(&task.related_files)?)
        .bind(serde_json::to_string(&task.metadata)?)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get a task by its ID
    pub async fn get_task(&self, id: Uuid) -> Result<Option<Task>> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, parent_task_id, prd_id, title, description, status, priority,
                estimated_time, actual_time, created_at, completed_at,
                dependencies, related_files, metadata
            FROM tasks WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = row {
            let task = Task {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                parent_task_id: row
                    .get::<Option<String>, _>("parent_task_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                prd_id: row
                    .get::<Option<String>, _>("prd_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                title: row.get("title"),
                description: row.get("description"),
                status: serde_json::from_str(row.get::<&str, _>("status"))?,
                priority: serde_json::from_str(row.get::<&str, _>("priority"))?,
                estimated_time: row.get("estimated_time"),
                actual_time: row.get("actual_time"),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                completed_at: row
                    .get::<Option<&str>, _>("completed_at")
                    .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()?,
                dependencies: serde_json::from_str(row.get::<&str, _>("dependencies"))
                    .unwrap_or_default(),
                related_files: serde_json::from_str(row.get::<&str, _>("related_files"))
                    .unwrap_or_default(),
                metadata: serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default(),
            };

            Ok(Some(task))
        } else {
            Ok(None)
        }
    }

    /// Update an existing task
    pub async fn update_task(&self, task: &Task) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE tasks SET
                parent_task_id = ?, prd_id = ?, title = ?, description = ?, status = ?,
                priority = ?, estimated_time = ?, actual_time = ?, completed_at = ?,
                dependencies = ?, related_files = ?, metadata = ?
            WHERE id = ?
            "#,
        )
        .bind(task.parent_task_id.map(|id| id.to_string()))
        .bind(task.prd_id.map(|id| id.to_string()))
        .bind(&task.title)
        .bind(&task.description)
        .bind(serde_json::to_string(&task.status)?)
        .bind(serde_json::to_string(&task.priority)?)
        .bind(task.estimated_time)
        .bind(task.actual_time)
        .bind(task.completed_at.map(|dt| dt.to_rfc3339()))
        .bind(serde_json::to_string(&task.dependencies)?)
        .bind(serde_json::to_string(&task.related_files)?)
        .bind(serde_json::to_string(&task.metadata)?)
        .bind(task.id.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Delete a task by its ID
    pub async fn delete_task(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tasks WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get all child tasks of a parent task
    pub async fn get_child_tasks(&self, parent_id: Uuid) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, parent_task_id, prd_id, title, description, status, priority,
                estimated_time, actual_time, created_at, completed_at,
                dependencies, related_files, metadata
            FROM tasks WHERE parent_task_id = ?
            "#,
        )
        .bind(parent_id.to_string())
        .fetch_all(&self.db)
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                parent_task_id: row
                    .get::<Option<String>, _>("parent_task_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                prd_id: row
                    .get::<Option<String>, _>("prd_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                title: row.get("title"),
                description: row.get("description"),
                status: serde_json::from_str(row.get::<&str, _>("status"))?,
                priority: serde_json::from_str(row.get::<&str, _>("priority"))?,
                estimated_time: row.get("estimated_time"),
                actual_time: row.get("actual_time"),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                completed_at: row
                    .get::<Option<&str>, _>("completed_at")
                    .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()?,
                dependencies: serde_json::from_str(row.get::<&str, _>("dependencies"))
                    .unwrap_or_default(),
                related_files: serde_json::from_str(row.get::<&str, _>("related_files"))
                    .unwrap_or_default(),
                metadata: serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default(),
            };
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Create a new pattern in the database
    pub async fn create_pattern(&self, pattern: &Pattern) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO patterns (
                id, pattern_type, title, content, created_at, last_accessed,
                access_count, confidence, embedding, context, related_patterns
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(pattern.id.to_string())
        .bind(serde_json::to_string(&pattern.pattern_type)?)
        .bind(&pattern.title)
        .bind(&pattern.content)
        .bind(pattern.created_at.to_rfc3339())
        .bind(pattern.last_accessed.to_rfc3339())
        .bind(pattern.access_count as i32)
        .bind(pattern.confidence)
        .bind(&pattern.embedding) // This will be serialized differently
        .bind(serde_json::to_string(&pattern.context)?)
        .bind(serde_json::to_string(&pattern.related_patterns)?)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get a pattern by its ID
    pub async fn get_pattern(&self, id: Uuid) -> Result<Option<Pattern>> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, pattern_type, title, content, created_at, last_accessed,
                access_count, confidence, context, related_patterns
            FROM patterns WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = row {
            let pattern = Pattern {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                pattern_type: serde_json::from_str(row.get::<&str, _>("pattern_type"))?,
                title: row.get("title"),
                content: row.get("content"),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                last_accessed: DateTime::parse_from_rfc3339(row.get::<&str, _>("last_accessed"))?
                    .with_timezone(&Utc),
                access_count: row.get::<i32, _>("access_count") as u32,
                confidence: row.get("confidence"),
                embedding: Vec::new(), // Will be populated by the search module
                context: serde_json::from_str(row.get::<&str, _>("context")).unwrap_or_default(),
                related_patterns: serde_json::from_str(row.get::<&str, _>("related_patterns"))
                    .unwrap_or_default(),
            };

            Ok(Some(pattern))
        } else {
            Ok(None)
        }
    }

    /// Create a new relationship in the database
    pub async fn create_relationship(&self, relationship: &Relationship) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO relationships (
                id, from_id, from_type, to_id, to_type, relationship_type, metadata, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(relationship.id.to_string())
        .bind(relationship.from_id.to_string())
        .bind(serde_json::to_string(&relationship.from_type)?)
        .bind(relationship.to_id.to_string())
        .bind(serde_json::to_string(&relationship.to_type)?)
        .bind(serde_json::to_string(&relationship.relationship_type)?)
        .bind(serde_json::to_string(&relationship.metadata)?)
        .bind(relationship.created_at.to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get relationships for a specific entity
    pub async fn get_relationships(
        &self,
        entity_id: Uuid,
        entity_type: EntityType,
    ) -> Result<Vec<Relationship>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, from_id, from_type, to_id, to_type, relationship_type, metadata, created_at
            FROM relationships 
            WHERE (from_id = ? AND from_type = ?) OR (to_id = ? AND to_type = ?)
            "#,
        )
        .bind(entity_id.to_string())
        .bind(serde_json::to_string(&entity_type)?)
        .bind(entity_id.to_string())
        .bind(serde_json::to_string(&entity_type)?)
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
                metadata: serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default(),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
            };
            relationships.push(relationship);
        }

        Ok(relationships)
    }

    /// Create a new PRD in the database
    pub async fn create_prd(&self, prd: &ProductRequirement) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO prds (
                id, title, overview, goals, user_stories, functional_requirements,
                non_goals, design_considerations, technical_considerations,
                success_metrics, open_questions, created_at, updated_at, status
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(prd.id.to_string())
        .bind(&prd.title)
        .bind(&prd.overview)
        .bind(serde_json::to_string(&prd.goals)?)
        .bind(serde_json::to_string(&prd.user_stories)?)
        .bind(serde_json::to_string(&prd.functional_requirements)?)
        .bind(serde_json::to_string(&prd.non_goals)?)
        .bind(serde_json::to_string(&prd.design_considerations)?)
        .bind(serde_json::to_string(&prd.technical_considerations)?)
        .bind(serde_json::to_string(&prd.success_metrics)?)
        .bind(serde_json::to_string(&prd.open_questions)?)
        .bind(prd.created_at.to_rfc3339())
        .bind(prd.updated_at.to_rfc3339())
        .bind(serde_json::to_string(&prd.status)?)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get a PRD by its ID
    pub async fn get_prd(&self, id: Uuid) -> Result<Option<ProductRequirement>> {
        let row = sqlx::query(
            r#"
            SELECT 
                id, title, overview, goals, user_stories, functional_requirements,
                non_goals, design_considerations, technical_considerations,
                success_metrics, open_questions, created_at, updated_at, status
            FROM prds WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = row {
            let prd = ProductRequirement {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                title: row.get("title"),
                overview: row.get("overview"),
                goals: serde_json::from_str(row.get::<&str, _>("goals")).unwrap_or_default(),
                user_stories: serde_json::from_str(row.get::<&str, _>("user_stories"))
                    .unwrap_or_default(),
                functional_requirements: serde_json::from_str(
                    row.get::<&str, _>("functional_requirements"),
                )
                .unwrap_or_default(),
                non_goals: serde_json::from_str(row.get::<&str, _>("non_goals"))
                    .unwrap_or_default(),
                design_considerations: serde_json::from_str(
                    row.get::<&str, _>("design_considerations"),
                )
                .unwrap_or_default(),
                technical_considerations: serde_json::from_str(
                    row.get::<&str, _>("technical_considerations"),
                )
                .unwrap_or_default(),
                success_metrics: serde_json::from_str(row.get::<&str, _>("success_metrics"))
                    .unwrap_or_default(),
                open_questions: serde_json::from_str(row.get::<&str, _>("open_questions"))
                    .unwrap_or_default(),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("updated_at"))?
                    .with_timezone(&Utc),
                status: serde_json::from_str(row.get::<&str, _>("status"))?,
            };

            Ok(Some(prd))
        } else {
            Ok(None)
        }
    }

    /// Get tasks by PRD ID
    pub async fn get_tasks_by_prd(&self, prd_id: Uuid) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id, parent_task_id, prd_id, title, description, status, priority,
                estimated_time, actual_time, created_at, completed_at,
                dependencies, related_files, metadata
            FROM tasks WHERE prd_id = ?
            "#,
        )
        .bind(prd_id.to_string())
        .fetch_all(&self.db)
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                parent_task_id: row
                    .get::<Option<String>, _>("parent_task_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                prd_id: Some(prd_id), // Already know this from the query
                title: row.get("title"),
                description: row.get("description"),
                status: serde_json::from_str(row.get::<&str, _>("status"))?,
                priority: serde_json::from_str(row.get::<&str, _>("priority"))?,
                estimated_time: row.get("estimated_time"),
                actual_time: row.get("actual_time"),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                completed_at: row
                    .get::<Option<&str>, _>("completed_at")
                    .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()?,
                dependencies: serde_json::from_str(row.get::<&str, _>("dependencies"))
                    .unwrap_or_default(),
                related_files: serde_json::from_str(row.get::<&str, _>("related_files"))
                    .unwrap_or_default(),
                metadata: serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default(),
            };
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Search for patterns based on content similarity
    pub async fn search_patterns(&self, query: &str, limit: usize) -> Result<Vec<Pattern>> {
        // For now, this will do a simple text search
        // In a real implementation with FAISS integration, this would use vector similarity
        let rows = sqlx::query(
            r#"
            SELECT
                id, pattern_type, title, content, created_at, last_accessed,
                access_count, confidence, context, related_patterns
            FROM patterns
            WHERE content LIKE ?
            ORDER BY access_count DESC
            LIMIT ?
            "#,
        )
        .bind(format!("%{}%", query))
        .bind(limit as i32)
        .fetch_all(&self.db)
        .await?;

        let mut patterns = Vec::new();
        for row in rows {
            let pattern = Pattern {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                pattern_type: serde_json::from_str(row.get::<&str, _>("pattern_type"))?,
                title: row.get("title"),
                content: row.get("content"),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                last_accessed: DateTime::parse_from_rfc3339(row.get::<&str, _>("last_accessed"))?
                    .with_timezone(&Utc),
                access_count: row.get::<i32, _>("access_count") as u32,
                confidence: row.get("confidence"),
                embedding: Vec::new(), // Will be populated by the search module
                context: serde_json::from_str(row.get::<&str, _>("context")).unwrap_or_default(),
                related_patterns: serde_json::from_str(row.get::<&str, _>("related_patterns"))
                    .unwrap_or_default(),
            };
            patterns.push(pattern);
        }

        Ok(patterns)
    }

    /// Get all tasks from database
    pub async fn list_tasks(&self) -> Result<Vec<Task>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, parent_task_id, prd_id, title, description, status, priority,
                estimated_time, actual_time, created_at, completed_at,
                dependencies, related_files, metadata
            FROM tasks
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut tasks = Vec::new();
        for row in rows {
            let task = Task {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                parent_task_id: row
                    .get::<Option<String>, _>("parent_task_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                prd_id: row
                    .get::<Option<String>, _>("prd_id")
                    .map(|s| Uuid::parse_str(&s).ok())
                    .flatten(),
                title: row.get("title"),
                description: row.get("description"),
                status: serde_json::from_str(row.get::<&str, _>("status"))?,
                priority: serde_json::from_str(row.get::<&str, _>("priority"))?,
                estimated_time: row.get("estimated_time"),
                actual_time: row.get("actual_time"),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                completed_at: row
                    .get::<Option<&str>, _>("completed_at")
                    .map(|s| DateTime::parse_from_rfc3339(s).map(|dt| dt.with_timezone(&Utc)))
                    .transpose()?,
                dependencies: serde_json::from_str(row.get::<&str, _>("dependencies"))
                    .unwrap_or_default(),
                related_files: serde_json::from_str(row.get::<&str, _>("related_files"))
                    .unwrap_or_default(),
                metadata: serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default(),
            };
            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Update an existing pattern
    pub async fn update_pattern(&self, pattern: &Pattern) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE patterns SET
                pattern_type = ?, title = ?, content = ?, last_accessed = ?,
                access_count = ?, confidence = ?, embedding = ?,
                context = ?, related_patterns = ?
            WHERE id = ?
            "#,
        )
        .bind(serde_json::to_string(&pattern.pattern_type)?)
        .bind(&pattern.title)
        .bind(&pattern.content)
        .bind(pattern.last_accessed.to_rfc3339())
        .bind(pattern.access_count as i32)
        .bind(pattern.confidence)
        .bind(&pattern.embedding)
        .bind(serde_json::to_string(&pattern.context)?)
        .bind(serde_json::to_string(&pattern.related_patterns)?)
        .bind(pattern.id.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Delete a pattern by its ID
    pub async fn delete_pattern(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM patterns WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get all patterns from database
    pub async fn list_patterns(&self) -> Result<Vec<Pattern>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, pattern_type, title, content, created_at, last_accessed,
                access_count, confidence, context, related_patterns
            FROM patterns
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut patterns = Vec::new();
        for row in rows {
            let pattern = Pattern {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                pattern_type: serde_json::from_str(row.get::<&str, _>("pattern_type"))?,
                title: row.get("title"),
                content: row.get("content"),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                last_accessed: DateTime::parse_from_rfc3339(row.get::<&str, _>("last_accessed"))?
                    .with_timezone(&Utc),
                access_count: row.get::<i32, _>("access_count") as u32,
                confidence: row.get("confidence"),
                embedding: Vec::new(), // Will be populated by the search module
                context: serde_json::from_str(row.get::<&str, _>("context")).unwrap_or_default(),
                related_patterns: serde_json::from_str(row.get::<&str, _>("related_patterns"))
                    .unwrap_or_default(),
            };
            patterns.push(pattern);
        }

        Ok(patterns)
    }

    /// Update an existing PRD
    pub async fn update_prd(&self, prd: &ProductRequirement) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE prds SET
                title = ?, overview = ?, goals = ?, user_stories = ?,
                functional_requirements = ?, non_goals = ?, design_considerations = ?,
                technical_considerations = ?, success_metrics = ?, open_questions = ?,
                updated_at = ?, status = ?
            WHERE id = ?
            "#,
        )
        .bind(&prd.title)
        .bind(&prd.overview)
        .bind(serde_json::to_string(&prd.goals)?)
        .bind(serde_json::to_string(&prd.user_stories)?)
        .bind(serde_json::to_string(&prd.functional_requirements)?)
        .bind(serde_json::to_string(&prd.non_goals)?)
        .bind(serde_json::to_string(&prd.design_considerations)?)
        .bind(serde_json::to_string(&prd.technical_considerations)?)
        .bind(serde_json::to_string(&prd.success_metrics)?)
        .bind(serde_json::to_string(&prd.open_questions)?)
        .bind(prd.updated_at.to_rfc3339())
        .bind(serde_json::to_string(&prd.status)?)
        .bind(prd.id.to_string())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get all PRDs from database
    pub async fn list_prds(&self) -> Result<Vec<ProductRequirement>> {
        let rows = sqlx::query(
            r#"
            SELECT
                id, title, overview, goals, user_stories, functional_requirements,
                non_goals, design_considerations, technical_considerations,
                success_metrics, open_questions, created_at, updated_at, status
            FROM prds
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut prds = Vec::new();
        for row in rows {
            let prd = ProductRequirement {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                title: row.get("title"),
                overview: row.get("overview"),
                goals: serde_json::from_str(row.get::<&str, _>("goals")).unwrap_or_default(),
                user_stories: serde_json::from_str(row.get::<&str, _>("user_stories"))
                    .unwrap_or_default(),
                functional_requirements: serde_json::from_str(
                    row.get::<&str, _>("functional_requirements"),
                )
                .unwrap_or_default(),
                non_goals: serde_json::from_str(row.get::<&str, _>("non_goals"))
                    .unwrap_or_default(),
                design_considerations: serde_json::from_str(
                    row.get::<&str, _>("design_considerations"),
                )
                .unwrap_or_default(),
                technical_considerations: serde_json::from_str(
                    row.get::<&str, _>("technical_considerations"),
                )
                .unwrap_or_default(),
                success_metrics: serde_json::from_str(row.get::<&str, _>("success_metrics"))
                    .unwrap_or_default(),
                open_questions: serde_json::from_str(row.get::<&str, _>("open_questions"))
                    .unwrap_or_default(),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("updated_at"))?
                    .with_timezone(&Utc),
                status: serde_json::from_str(row.get::<&str, _>("status"))?,
            };
            prds.push(prd);
        }

        Ok(prds)
    }

    /// Get a relationship by its ID
    pub async fn get_relationship(&self, id: Uuid) -> Result<Option<Relationship>> {
        let row = sqlx::query(
            r#"
            SELECT
                id, from_id, from_type, to_id, to_type, relationship_type, metadata, created_at
            FROM relationships
            WHERE id = ?
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = row {
            let relationship = Relationship {
                id: Uuid::parse_str(row.get::<&str, _>("id"))?,
                from_id: Uuid::parse_str(row.get::<&str, _>("from_id"))?,
                from_type: serde_json::from_str(row.get::<&str, _>("from_type"))?,
                to_id: Uuid::parse_str(row.get::<&str, _>("to_id"))?,
                to_type: serde_json::from_str(row.get::<&str, _>("to_type"))?,
                relationship_type: serde_json::from_str(row.get::<&str, _>("relationship_type"))?,
                metadata: serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default(),
                created_at: DateTime::parse_from_rfc3339(row.get::<&str, _>("created_at"))?
                    .with_timezone(&Utc),
            };

            Ok(Some(relationship))
        } else {
            Ok(None)
        }
    }

    /// Delete a relationship by its ID
    pub async fn delete_relationship(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM relationships WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.db)
            .await?;

        Ok(())
    }
}

/// Initialize the database connection and schema
pub async fn init_db(db_path: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(db_path).await?;

    // Create our tables
    let storage = StorageManager::new(pool.clone());
    storage.init_db().await?;

    Ok(pool)
}
