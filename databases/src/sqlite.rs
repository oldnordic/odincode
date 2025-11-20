//! SQLite Database Implementation for LTMC
//!
//! This module provides real SQLite database operations for the LTMC system,
//! including connection management, schema creation, and CRUD operations.

use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// SQLite connection manager
pub struct SQLiteManager {
    /// Database connection
    connection: Arc<Mutex<Connection>>,
    /// Database path
    db_path: String,
    /// Connection status
    is_connected: Arc<RwLock<bool>>,
}

/// Learning pattern data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPattern {
    /// Unique identifier
    pub id: String,
    /// Pattern type
    pub pattern_type: String,
    /// Pattern data (JSON)
    pub pattern_data: String,
    /// Source file or context
    pub source: String,
    /// Confidence score
    pub confidence: f64,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Sequential thinking step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialThinkingStep {
    /// Unique identifier
    pub id: String,
    /// Session identifier
    pub session_id: String,
    /// Step number
    pub step_number: i32,
    /// Step description
    pub description: String,
    /// Step data (JSON)
    pub step_data: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

/// User interaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInteraction {
    /// Unique identifier
    pub id: String,
    /// User identifier
    pub user_id: String,
    /// Interaction type
    pub interaction_type: String,
    /// Interaction data (JSON)
    pub interaction_data: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl SQLiteManager {
    /// Create a new SQLite manager
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let path = db_path.as_ref().to_string_lossy().to_string();

        info!("Creating SQLite manager with database path: {path}");

        // Create or open the database connection
        let conn = Connection::open(&path)
            .map_err(|e| anyhow::anyhow!("Failed to open SQLite database: {e}"))?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| anyhow::anyhow!("Failed to enable foreign keys: {e}"))?;

        // Set busy timeout
        conn.busy_timeout(std::time::Duration::from_secs(30))
            .map_err(|e| anyhow::anyhow!("Failed to set busy timeout: {e}"))?;

        Ok(Self {
            connection: Arc::new(Mutex::new(conn)),
            db_path: path,
            is_connected: Arc::new(RwLock::new(true)),
        })
    }

    /// Initialize database schema
    pub async fn initialize_schema(&self) -> Result<()> {
        info!("Initializing SQLite database schema");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        // Create learning patterns table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS learning_patterns (
                id TEXT PRIMARY KEY,
                pattern_type TEXT NOT NULL,
                pattern_data TEXT NOT NULL,
                source TEXT NOT NULL,
                confidence REAL NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                tags TEXT
            );",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create learning_patterns table: {e}"))?;

        // Create sequential thinking table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sequential_thinking (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                step_number INTEGER NOT NULL,
                description TEXT NOT NULL,
                step_data TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES thinking_sessions (id)
            );",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create sequential_thinking table: {e}"))?;

        // Create thinking sessions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS thinking_sessions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                session_type TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                status TEXT NOT NULL
            );",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create thinking_sessions table: {e}"))?;

        // Create user interactions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_interactions (
                id TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                interaction_type TEXT NOT NULL,
                interaction_data TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create user_interactions table: {e}"))?;

        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_patterns_type ON learning_patterns(pattern_type);",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create patterns_type index: {e}"))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_patterns_source ON learning_patterns(source);",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create patterns_source index: {e}"))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_thinking_session ON sequential_thinking(session_id);",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create thinking_session index: {e}"))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_interactions_user ON user_interactions(user_id);",
            [],
        )
        .map_err(|e| anyhow::anyhow!("Failed to create interactions_user index: {e}"))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_interactions_type ON user_interactions(interaction_type);",
            [],
        ).map_err(|e| anyhow::anyhow!("Failed to create interactions_type index: {e}"))?;

        drop(conn);

        info!("SQLite database schema initialized successfully");
        Ok(())
    }

    /// Test database connection
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing SQLite connection");

        let result = {
            let conn = self
                .connection
                .lock()
                .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

            // Execute a simple query to test connection
            conn.query_row("SELECT 1;", [], |_| Ok(()))
                .map(|_| true)
                .unwrap_or(false)
        };

        // Update connection status
        let mut status = self.is_connected.write().await;
        *status = result;
        drop(status);

        if result {
            debug!("SQLite connection test successful");
        } else {
            warn!("SQLite connection test failed");
        }

        Ok(result)
    }

    /// Get connection status
    pub async fn is_connected(&self) -> bool {
        let status = self.is_connected.read().await;
        *status
    }

    /// Get database path
    pub fn get_database_path(&self) -> &str {
        &self.db_path
    }

    /// Create a new learning pattern
    pub async fn create_learning_pattern(&self, pattern: &LearningPattern) -> Result<()> {
        debug!("Creating learning pattern: {}", pattern.id);

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let tags_json = serde_json::to_string(&pattern.tags)
            .map_err(|e| anyhow::anyhow!("Failed to serialize tags: {e}"))?;

        conn.execute(
            "INSERT INTO learning_patterns (id, pattern_type, pattern_data, source, confidence, created_at, updated_at, tags) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8);",
            params![
                pattern.id,
                pattern.pattern_type,
                pattern.pattern_data,
                pattern.source,
                pattern.confidence,
                pattern.created_at.to_rfc3339(),
                pattern.updated_at.to_rfc3339(),
                tags_json
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to insert learning pattern: {e}"))?;

        drop(conn);

        debug!("Learning pattern created successfully: {}", pattern.id);
        Ok(())
    }

    /// Get a learning pattern by ID
    pub async fn get_learning_pattern(&self, id: &str) -> Result<Option<LearningPattern>> {
        debug!("Getting learning pattern: {id}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let pattern = conn.query_row(
            "SELECT id, pattern_type, pattern_data, source, confidence, created_at, updated_at, tags 
             FROM learning_patterns WHERE id = ?1;",
            params![id],
            |row| {
                let tags_json: String = row.get(7)?;
                let tags: Vec<String> = serde_json::from_str(&tags_json)
                    .unwrap_or_else(|_| Vec::new());
                
                Ok(LearningPattern {
                    id: row.get(0)?,
                    pattern_type: row.get(1)?,
                    pattern_data: row.get(2)?,
                    source: row.get(3)?,
                    confidence: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    tags,
                })
            }
        ).optional()
        .map_err(|e| anyhow::anyhow!("Failed to query learning pattern: {e}"))?;

        drop(conn);

        match &pattern {
            Some(p) => debug!("Learning pattern found: {}", p.id),
            None => debug!("Learning pattern not found: {id}"),
        }

        Ok(pattern)
    }

    /// Update a learning pattern
    pub async fn update_learning_pattern(&self, pattern: &LearningPattern) -> Result<bool> {
        debug!("Updating learning pattern: {}", pattern.id);

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let tags_json = serde_json::to_string(&pattern.tags)
            .map_err(|e| anyhow::anyhow!("Failed to serialize tags: {e}"))?;

        let result = conn
            .execute(
                "UPDATE learning_patterns 
             SET pattern_type = ?2, pattern_data = ?3, source = ?4, confidence = ?5, 
                 updated_at = ?6, tags = ?7 
             WHERE id = ?1;",
                params![
                    pattern.id,
                    pattern.pattern_type,
                    pattern.pattern_data,
                    pattern.source,
                    pattern.confidence,
                    pattern.updated_at.to_rfc3339(),
                    tags_json
                ],
            )
            .map_err(|e| anyhow::anyhow!("Failed to update learning pattern: {e}"))?;

        drop(conn);

        let success = result > 0;
        if success {
            debug!("Learning pattern updated successfully: {}", pattern.id);
        } else {
            warn!("Learning pattern not found for update: {}", pattern.id);
        }

        Ok(success)
    }

    /// Delete a learning pattern
    pub async fn delete_learning_pattern(&self, id: &str) -> Result<bool> {
        debug!("Deleting learning pattern: {id}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let result = conn
            .execute("DELETE FROM learning_patterns WHERE id = ?1;", params![id])
            .map_err(|e| anyhow::anyhow!("Failed to delete learning pattern: {e}"))?;

        drop(conn);

        let success = result > 0;
        if success {
            debug!("Learning pattern deleted successfully: {id}");
        } else {
            warn!("Learning pattern not found for deletion: {id}");
        }

        Ok(success)
    }

    /// List learning patterns by type
    pub async fn list_learning_patterns_by_type(
        &self,
        pattern_type: &str,
    ) -> Result<Vec<LearningPattern>> {
        debug!("Listing learning patterns by type: {pattern_type}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let mut stmt = conn.prepare(
            "SELECT id, pattern_type, pattern_data, source, confidence, created_at, updated_at, tags 
             FROM learning_patterns WHERE pattern_type = ?1 ORDER BY created_at DESC;"
        ).map_err(|e| anyhow::anyhow!("Failed to prepare statement: {e}"))?;

        let patterns: Vec<LearningPattern> = stmt
            .query_map(params![pattern_type], |row| {
                let tags_json: String = row.get(7)?;
                let tags: Vec<String> =
                    serde_json::from_str(&tags_json).unwrap_or_else(|_| Vec::new());

                Ok(LearningPattern {
                    id: row.get(0)?,
                    pattern_type: row.get(1)?,
                    pattern_data: row.get(2)?,
                    source: row.get(3)?,
                    confidence: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    tags,
                })
            })
            .map_err(|e| anyhow::anyhow!("Failed to query learning patterns: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Failed to collect learning patterns: {e}"))?;

        drop(stmt);

        drop(conn);

        debug!(
            "Found {} learning patterns of type {pattern_type}",
            patterns.len()
        );
        Ok(patterns)
    }

    /// Search learning patterns by keyword
    pub async fn search_learning_patterns(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        debug!("Searching learning patterns with query: {query}, limit: {limit}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        // Use LIKE for simple keyword search (in production, would use FTS)
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT id, pattern_type, pattern_data, source, confidence, created_at, updated_at, tags 
             FROM learning_patterns 
             WHERE pattern_data LIKE ?1 OR source LIKE ?1
             ORDER BY confidence DESC, created_at DESC
             LIMIT ?2;"
        )?;

        let patterns = stmt
            .query_map(params![search_pattern, limit as i64], |row| {
                let tags_str: String = row.get(7)?;
                let tags: Vec<String> = if tags_str.is_empty() {
                    Vec::new()
                } else {
                    tags_str.split(',').map(|s| s.trim().to_string()).collect()
                };

                Ok(LearningPattern {
                    id: row.get(0)?,
                    pattern_type: row.get(1)?,
                    pattern_data: row.get(2)?,
                    source: row.get(3)?,
                    confidence: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                        .unwrap()
                        .with_timezone(&Utc),
                    tags,
                })
            })
            .map_err(|e| anyhow::anyhow!("Failed to search learning patterns: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Failed to collect search results: {e}"))?;

        drop(stmt);
        drop(conn);

        debug!(
            "Found {} learning patterns matching query: {query}",
            patterns.len()
        );
        Ok(patterns)
    }

    /// Create a new thinking session
    pub async fn create_thinking_session(
        &self,
        session_id: &str,
        user_id: &str,
        session_type: &str,
    ) -> Result<()> {
        debug!("Creating thinking session: {session_id}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let now = Utc::now();

        conn.execute(
            "INSERT INTO thinking_sessions (id, user_id, session_type, created_at, updated_at, status) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![
                session_id,
                user_id,
                session_type,
                now.to_rfc3339(),
                now.to_rfc3339(),
                "active"
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to insert thinking session: {e}"))?;

        drop(conn);

        debug!("Thinking session created successfully: {session_id}");
        Ok(())
    }

    /// Create a new sequential thinking step
    pub async fn create_sequential_thinking_step(
        &self,
        step: &SequentialThinkingStep,
    ) -> Result<()> {
        debug!("Creating sequential thinking step: {}", step.id);

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        conn.execute(
            "INSERT INTO sequential_thinking (id, session_id, step_number, description, step_data, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
            params![
                step.id,
                step.session_id,
                step.step_number,
                step.description,
                step.step_data,
                step.created_at.to_rfc3339()
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to insert sequential thinking step: {e}"))?;

        drop(conn);

        debug!("Sequential thinking step created successfully: {}", step.id);
        Ok(())
    }

    /// Get sequential thinking steps by session ID
    pub async fn get_sequential_thinking_steps(
        &self,
        session_id: &str,
    ) -> Result<Vec<SequentialThinkingStep>> {
        debug!("Getting sequential thinking steps for session: {session_id}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, session_id, step_number, description, step_data, created_at 
             FROM sequential_thinking WHERE session_id = ?1 ORDER BY step_number ASC;",
            )
            .map_err(|e| anyhow::anyhow!("Failed to prepare statement: {e}"))?;

        let steps: Vec<SequentialThinkingStep> = stmt
            .query_map(params![session_id], |row| {
                Ok(SequentialThinkingStep {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    step_number: row.get(2)?,
                    description: row.get(3)?,
                    step_data: row.get(4)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                        .unwrap()
                        .with_timezone(&Utc),
                })
            })
            .map_err(|e| anyhow::anyhow!("Failed to query sequential thinking steps: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow::anyhow!("Failed to collect sequential thinking steps: {e}"))?;

        drop(stmt);
        drop(conn);

        debug!(
            "Found {} sequential thinking steps for session {session_id}",
            steps.len()
        );
        Ok(steps)
    }

    /// Get a sequential thinking step by ID
    pub async fn get_sequential_thinking_step(
        &self,
        id: &str,
    ) -> Result<Option<SequentialThinkingStep>> {
        debug!("Getting sequential thinking step: {id}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let step = conn
            .query_row(
                "SELECT id, session_id, step_number, description, step_data, created_at 
             FROM sequential_thinking WHERE id = ?1;",
                params![id],
                |row| {
                    Ok(SequentialThinkingStep {
                        id: row.get(0)?,
                        session_id: row.get(1)?,
                        step_number: row.get(2)?,
                        description: row.get(3)?,
                        step_data: row.get(4)?,
                        created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                            .unwrap()
                            .with_timezone(&Utc),
                    })
                },
            )
            .optional()
            .map_err(|e| anyhow::anyhow!("Failed to query sequential thinking step: {e}"))?;

        drop(conn);

        match &step {
            Some(s) => debug!("Sequential thinking step found: {}", s.id),
            None => debug!("Sequential thinking step not found: {id}"),
        }

        Ok(step)
    }

    /// Update a sequential thinking step
    pub async fn update_sequential_thinking_step(
        &self,
        step: &SequentialThinkingStep,
    ) -> Result<bool> {
        debug!("Updating sequential thinking step: {}", step.id);

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let result = conn
            .execute(
                "UPDATE sequential_thinking 
             SET session_id = ?2, step_number = ?3, description = ?4, step_data = ?5 
             WHERE id = ?1;",
                params![
                    step.id,
                    step.session_id,
                    step.step_number,
                    step.description,
                    step.step_data,
                ],
            )
            .map_err(|e| anyhow::anyhow!("Failed to update sequential thinking step: {e}"))?;

        drop(conn);

        let success = result > 0;
        if success {
            debug!("Sequential thinking step updated successfully: {}", step.id);
        } else {
            warn!("Sequential thinking step not found for update: {}", step.id);
        }

        Ok(success)
    }

    /// Delete a sequential thinking step
    pub async fn delete_sequential_thinking_step(&self, id: &str) -> Result<bool> {
        debug!("Deleting sequential thinking step: {id}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let result = conn
            .execute(
                "DELETE FROM sequential_thinking WHERE id = ?1;",
                params![id],
            )
            .map_err(|e| anyhow::anyhow!("Failed to delete sequential thinking step: {e}"))?;

        drop(conn);

        let success = result > 0;
        if success {
            debug!("Sequential thinking step deleted successfully: {id}");
        } else {
            warn!("Sequential thinking step not found for deletion: {id}");
        }

        Ok(success)
    }

    /// Update thinking session status
    pub async fn update_thinking_session_status(
        &self,
        session_id: &str,
        status: &str,
    ) -> Result<bool> {
        debug!("Updating thinking session status: {session_id} -> {status}");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let result = conn
            .execute(
                "UPDATE thinking_sessions SET status = ?2, updated_at = ?3 WHERE id = ?1;",
                params![session_id, status, Utc::now().to_rfc3339()],
            )
            .map_err(|e| anyhow::anyhow!("Failed to update thinking session status: {e}"))?;

        drop(conn);

        let success = result > 0;
        if success {
            debug!("Thinking session status updated successfully: {session_id}");
        } else {
            warn!("Thinking session not found for status update: {session_id}");
        }

        Ok(success)
    }

    /// Create a new user interaction
    pub async fn create_user_interaction(&self, interaction: &UserInteraction) -> Result<()> {
        debug!("Creating user interaction: {}", interaction.id);

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        conn.execute(
            "INSERT INTO user_interactions (id, user_id, interaction_type, interaction_data, created_at) 
             VALUES (?1, ?2, ?3, ?4, ?5);",
            params![
                interaction.id,
                interaction.user_id,
                interaction.interaction_type,
                interaction.interaction_data,
                interaction.created_at.to_rfc3339()
            ],
        ).map_err(|e| anyhow::anyhow!("Failed to insert user interaction: {e}"))?;

        drop(conn);

        debug!("User interaction created successfully: {}", interaction.id);
        Ok(())
    }

    /// Get database statistics
    pub async fn get_database_stats(&self) -> Result<DatabaseStats> {
        debug!("Getting database statistics");

        let conn = self
            .connection
            .lock()
            .map_err(|e| anyhow::anyhow!("Failed to acquire connection lock: {e}"))?;

        let learning_patterns_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM learning_patterns;", [], |row| {
                row.get(0)
            })
            .map_err(|e| anyhow::anyhow!("Failed to count learning patterns: {e}"))?;

        let thinking_sessions_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM thinking_sessions;", [], |row| {
                row.get(0)
            })
            .map_err(|e| anyhow::anyhow!("Failed to count thinking sessions: {e}"))?;

        let sequential_thinking_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sequential_thinking;", [], |row| {
                row.get(0)
            })
            .map_err(|e| anyhow::anyhow!("Failed to count sequential thinking steps: {e}"))?;

        let user_interactions_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM user_interactions;", [], |row| {
                row.get(0)
            })
            .map_err(|e| anyhow::anyhow!("Failed to count user interactions: {e}"))?;

        drop(conn);

        let stats = DatabaseStats {
            learning_patterns_count,
            thinking_sessions_count,
            sequential_thinking_count,
            user_interactions_count,
        };

        debug!("Database stats: {:?}", stats);
        Ok(stats)
    }
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub learning_patterns_count: i64,
    pub thinking_sessions_count: i64,
    pub sequential_thinking_count: i64,
    pub user_interactions_count: i64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_sqlite_manager_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();

        // Initialize schema first
        manager.initialize_schema().await.unwrap();

        assert!(manager.test_connection().await.unwrap());
        assert!(manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_schema_initialization() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();

        manager.initialize_schema().await.unwrap();

        // Test that tables were created
        let stats = manager.get_database_stats().await.unwrap();
        assert_eq!(stats.learning_patterns_count, 0);
        assert_eq!(stats.thinking_sessions_count, 0);
        assert_eq!(stats.sequential_thinking_count, 0);
        assert_eq!(stats.user_interactions_count, 0);
    }

    #[tokio::test]
    async fn test_learning_pattern_crud() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();
        manager.initialize_schema().await.unwrap();

        let pattern = LearningPattern {
            id: Uuid::new_v4().to_string(),
            pattern_type: "code_pattern".to_string(),
            pattern_data: r#"{"language": "rust", "pattern": "async_function"}"#.to_string(),
            source: "src/main.rs".to_string(),
            confidence: 0.95,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["rust".to_string(), "async".to_string()],
        };

        // Create
        manager.create_learning_pattern(&pattern).await.unwrap();

        // Read
        let retrieved = manager.get_learning_pattern(&pattern.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, pattern.id);
        assert_eq!(retrieved.pattern_type, pattern.pattern_type);
        assert_eq!(retrieved.tags, pattern.tags);

        // Update
        let mut updated_pattern = pattern.clone();
        updated_pattern.confidence = 0.98;
        updated_pattern.updated_at = Utc::now();
        manager
            .update_learning_pattern(&updated_pattern)
            .await
            .unwrap();

        let updated = manager
            .get_learning_pattern(&pattern.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(updated.confidence, 0.98);

        // Delete
        let deleted = manager.delete_learning_pattern(&pattern.id).await.unwrap();
        assert!(deleted);

        let not_found = manager.get_learning_pattern(&pattern.id).await.unwrap();
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_learning_patterns_by_type() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();
        manager.initialize_schema().await.unwrap();

        // Create multiple patterns of the same type
        let pattern1 = LearningPattern {
            id: Uuid::new_v4().to_string(),
            pattern_type: "code_pattern".to_string(),
            pattern_data: r#"{"language": "rust", "pattern": "async_function"}"#.to_string(),
            source: "src/main.rs".to_string(),
            confidence: 0.95,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["rust".to_string()],
        };

        let pattern2 = LearningPattern {
            id: Uuid::new_v4().to_string(),
            pattern_type: "code_pattern".to_string(),
            pattern_data: r#"{"language": "python", "pattern": "decorator"}"#.to_string(),
            source: "src/main.py".to_string(),
            confidence: 0.90,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["python".to_string()],
        };

        let pattern3 = LearningPattern {
            id: Uuid::new_v4().to_string(),
            pattern_type: "architecture_pattern".to_string(),
            pattern_data: r#"{"pattern": "microservices"}"#.to_string(),
            source: "design.md".to_string(),
            confidence: 0.85,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["architecture".to_string()],
        };

        manager.create_learning_pattern(&pattern1).await.unwrap();
        manager.create_learning_pattern(&pattern2).await.unwrap();
        manager.create_learning_pattern(&pattern3).await.unwrap();

        // Test listing by type
        let code_patterns = manager
            .list_learning_patterns_by_type("code_pattern")
            .await
            .unwrap();
        assert_eq!(code_patterns.len(), 2);

        let arch_patterns = manager
            .list_learning_patterns_by_type("architecture_pattern")
            .await
            .unwrap();
        assert_eq!(arch_patterns.len(), 1);

        let non_existent = manager
            .list_learning_patterns_by_type("non_existent")
            .await
            .unwrap();
        assert_eq!(non_existent.len(), 0);
    }

    #[tokio::test]
    async fn test_thinking_session_management() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();
        manager.initialize_schema().await.unwrap();

        let session_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        let session_type = "sequential_analysis".to_string();

        // Create thinking session
        manager
            .create_thinking_session(&session_id, &user_id, &session_type)
            .await
            .unwrap();

        // Verify session was created by checking stats
        let stats = manager.get_database_stats().await.unwrap();
        assert_eq!(stats.thinking_sessions_count, 1);

        // Update session status
        let updated = manager
            .update_thinking_session_status(&session_id, "completed")
            .await
            .unwrap();
        assert!(updated);

        // Test updating non-existent session
        let not_updated = manager
            .update_thinking_session_status("non_existent", "active")
            .await
            .unwrap();
        assert!(!not_updated);
    }

    #[tokio::test]
    async fn test_sequential_thinking_crud() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();
        manager.initialize_schema().await.unwrap();

        // Create a thinking session first
        let session_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        manager
            .create_thinking_session(&session_id, &user_id, "sequential_analysis")
            .await
            .unwrap();

        // Create sequential thinking steps
        let step1 = SequentialThinkingStep {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.clone(),
            step_number: 1,
            description: "Analyze code structure".to_string(),
            step_data: r#"{"analysis": "function_identification"}"#.to_string(),
            created_at: Utc::now(),
        };

        let step2 = SequentialThinkingStep {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.clone(),
            step_number: 2,
            description: "Identify patterns".to_string(),
            step_data: r#"{"patterns": ["async_functions", "error_handling"]}"#.to_string(),
            created_at: Utc::now(),
        };

        // Create steps
        manager
            .create_sequential_thinking_step(&step1)
            .await
            .unwrap();
        manager
            .create_sequential_thinking_step(&step2)
            .await
            .unwrap();

        // Test getting individual step
        let retrieved = manager
            .get_sequential_thinking_step(&step1.id)
            .await
            .unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, step1.id);
        assert_eq!(retrieved.description, step1.description);

        // Test getting all steps for session
        let steps = manager
            .get_sequential_thinking_steps(&session_id)
            .await
            .unwrap();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].step_number, 1);
        assert_eq!(steps[1].step_number, 2);

        // Update step
        let mut updated_step = step1.clone();
        updated_step.description = "Analyze code structure in detail".to_string();
        updated_step.step_data = r#"{"analysis": "detailed_function_identification"}"#.to_string();

        let updated = manager
            .update_sequential_thinking_step(&updated_step)
            .await
            .unwrap();
        assert!(updated);

        let retrieved_updated = manager
            .get_sequential_thinking_step(&step1.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            retrieved_updated.description,
            "Analyze code structure in detail"
        );

        // Delete step
        let deleted = manager
            .delete_sequential_thinking_step(&step2.id)
            .await
            .unwrap();
        assert!(deleted);

        let steps_after_delete = manager
            .get_sequential_thinking_steps(&session_id)
            .await
            .unwrap();
        assert_eq!(steps_after_delete.len(), 1);

        // Test deleting non-existent step
        let not_deleted = manager
            .delete_sequential_thinking_step("non_existent")
            .await
            .unwrap();
        assert!(!not_deleted);
    }

    #[tokio::test]
    async fn test_user_interaction_crud() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();
        manager.initialize_schema().await.unwrap();

        let interaction = UserInteraction {
            id: Uuid::new_v4().to_string(),
            user_id: Uuid::new_v4().to_string(),
            interaction_type: "code_generation".to_string(),
            interaction_data: r#"{"prompt": "create async function", "result": "success"}"#
                .to_string(),
            created_at: Utc::now(),
        };

        // Create interaction
        manager.create_user_interaction(&interaction).await.unwrap();

        // Verify interaction was created by checking stats
        let stats = manager.get_database_stats().await.unwrap();
        assert_eq!(stats.user_interactions_count, 1);
    }

    #[tokio::test]
    async fn test_database_statistics() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();
        manager.initialize_schema().await.unwrap();

        // Initially all counts should be zero
        let stats = manager.get_database_stats().await.unwrap();
        assert_eq!(stats.learning_patterns_count, 0);
        assert_eq!(stats.thinking_sessions_count, 0);
        assert_eq!(stats.sequential_thinking_count, 0);
        assert_eq!(stats.user_interactions_count, 0);

        // Create one of each type
        let pattern = LearningPattern {
            id: Uuid::new_v4().to_string(),
            pattern_type: "test_pattern".to_string(),
            pattern_data: r#"{"test": "data"}"#.to_string(),
            source: "test.rs".to_string(),
            confidence: 1.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec!["test".to_string()],
        };
        manager.create_learning_pattern(&pattern).await.unwrap();

        let session_id = Uuid::new_v4().to_string();
        let user_id = Uuid::new_v4().to_string();
        manager
            .create_thinking_session(&session_id, &user_id, "test_session")
            .await
            .unwrap();

        let step = SequentialThinkingStep {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.clone(),
            step_number: 1,
            description: "Test step".to_string(),
            step_data: r#"{"test": "step"}"#.to_string(),
            created_at: Utc::now(),
        };
        manager
            .create_sequential_thinking_step(&step)
            .await
            .unwrap();

        let interaction = UserInteraction {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            interaction_type: "test_interaction".to_string(),
            interaction_data: r#"{"test": "interaction"}"#.to_string(),
            created_at: Utc::now(),
        };
        manager.create_user_interaction(&interaction).await.unwrap();

        // Check that all counts are now 1
        let stats = manager.get_database_stats().await.unwrap();
        assert_eq!(stats.learning_patterns_count, 1);
        assert_eq!(stats.thinking_sessions_count, 1);
        assert_eq!(stats.sequential_thinking_count, 1);
        assert_eq!(stats.user_interactions_count, 1);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = SQLiteManager::new(temp_file.path()).unwrap();
        manager.initialize_schema().await.unwrap();

        // Test getting non-existent learning pattern
        let not_found = manager.get_learning_pattern("non_existent").await.unwrap();
        assert!(not_found.is_none());

        // Test updating non-existent learning pattern
        let pattern = LearningPattern {
            id: "non_existent".to_string(),
            pattern_type: "test".to_string(),
            pattern_data: "test".to_string(),
            source: "test".to_string(),
            confidence: 1.0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            tags: vec![],
        };
        let not_updated = manager.update_learning_pattern(&pattern).await.unwrap();
        assert!(!not_updated);

        // Test deleting non-existent learning pattern
        let not_deleted = manager
            .delete_learning_pattern("non_existent")
            .await
            .unwrap();
        assert!(!not_deleted);

        // Test getting non-existent sequential thinking step
        let not_found_step = manager
            .get_sequential_thinking_step("non_existent")
            .await
            .unwrap();
        assert!(not_found_step.is_none());

        // Test getting steps for non-existent session
        let no_steps = manager
            .get_sequential_thinking_steps("non_existent")
            .await
            .unwrap();
        assert_eq!(no_steps.len(), 0);
    }
}
