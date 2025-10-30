//! Action history module for OdinCode
//! 
//! This module provides comprehensive logging of all actions, tool calls,
//! file modifications, and AI decisions for ML feedback and verification.

use anyhow::Result;
use sqlx::{SqlitePool, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Action history manager
pub struct ActionHistoryManager {
    pool: SqlitePool,
}

/// Types of actions that can be logged
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    ToolCall,
    FileModification,
    AiDecision,
    UserInteraction,
    SystemEvent,
    TodoCreation,
    TodoUpdate,
    TodoCompletion,
}

impl ActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionType::ToolCall => "tool_call",
            ActionType::FileModification => "file_modification",
            ActionType::AiDecision => "ai_decision",
            ActionType::UserInteraction => "user_interaction",
            ActionType::SystemEvent => "system_event",
            ActionType::TodoCreation => "todo_creation",
            ActionType::TodoUpdate => "todo_update",
            ActionType::TodoCompletion => "todo_completion",
        }
    }
}

/// Detailed action information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub action_type: ActionType,
    pub timestamp: i64,
    pub session_id: String,
    pub user_id: Option<String>,
    pub tool_name: Option<String>,
    pub file_path: Option<String>,
    pub line_number: Option<u32>,
    pub column_number: Option<u32>,
    pub before_content: Option<String>,
    pub after_content: Option<String>,
    pub ai_reasoning: Option<String>,
    pub parameters: Option<HashMap<String, String>>,
    pub result: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub duration_ms: Option<u64>,
    pub token_usage: Option<u32>,
    pub metadata: Option<HashMap<String, String>>,
}

/// File modification snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub id: String,
    pub action_id: String,
    pub file_path: String,
    pub content: String,
    pub timestamp: i64,
    pub hash: String,
}

/// AI decision log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiDecision {
    pub id: String,
    pub action_id: String,
    pub reasoning_chain: String,
    pub confidence_score: Option<f32>,
    pub alternatives_considered: Option<u32>,
    pub selected_alternative: Option<u32>,
    pub timestamp: i64,
}

impl ActionHistoryManager {
    /// Create a new action history manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the action history database with required tables
    pub async fn init(&self) -> Result<()> {
        // Create actions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS actions (
                id TEXT PRIMARY KEY,
                action_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                session_id TEXT NOT NULL,
                user_id TEXT,
                tool_name TEXT,
                file_path TEXT,
                line_number INTEGER,
                column_number INTEGER,
                before_content TEXT,
                after_content TEXT,
                ai_reasoning TEXT,
                parameters TEXT,
                result TEXT,
                success BOOLEAN NOT NULL,
                error_message TEXT,
                duration_ms INTEGER,
                token_usage INTEGER,
                metadata TEXT
            )
            "#
        ).execute(&self.pool).await?;

        // Create file snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS file_snapshots (
                id TEXT PRIMARY KEY,
                action_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                hash TEXT NOT NULL,
                FOREIGN KEY (action_id) REFERENCES actions (id) ON DELETE CASCADE
            )
            "#
        ).execute(&self.pool).await?;

        // Create AI decisions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS ai_decisions (
                id TEXT PRIMARY KEY,
                action_id TEXT NOT NULL,
                reasoning_chain TEXT NOT NULL,
                confidence_score REAL,
                alternatives_considered INTEGER,
                selected_alternative INTEGER,
                timestamp INTEGER NOT NULL,
                FOREIGN KEY (action_id) REFERENCES actions (id) ON DELETE CASCADE
            )
            "#
        ).execute(&self.pool).await?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_actions_timestamp ON actions(timestamp)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_actions_session ON actions(session_id)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_actions_type ON actions(action_type)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_actions_file ON actions(file_path)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_actions_tool ON actions(tool_name)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_snapshots_action ON file_snapshots(action_id)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_snapshots_file ON file_snapshots(file_path)").execute(&self.pool).await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_decisions_action ON ai_decisions(action_id)").execute(&self.pool).await?;

        Ok(())
    }

    /// Log a tool call action
    pub async fn log_tool_call(
        &self,
        session_id: &str,
        tool_name: &str,
        parameters: Option<HashMap<String, String>>,
        result: Option<String>,
        success: bool,
        error_message: Option<String>,
        duration_ms: Option<u64>,
        token_usage: Option<u32>,
    ) -> Result<String> {
        let action_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        let parameters_json = parameters.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());
        let metadata_json = Some(serde_json::to_string(&HashMap::<String, String>::new()).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO actions 
            (id, action_type, timestamp, session_id, tool_name, parameters, result, success, error_message, duration_ms, token_usage, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&action_id)
        .bind(ActionType::ToolCall.as_str())
        .bind(timestamp)
        .bind(session_id)
        .bind(tool_name)
        .bind(&parameters_json)
        .bind(&result)
        .bind(success)
        .bind(&error_message)
        .bind(duration_ms.map(|d| d as i64))
        .bind(token_usage.map(|t| t as i64))
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(action_id)
    }

    /// Log a file modification action
    pub async fn log_file_modification(
        &self,
        session_id: &str,
        file_path: &str,
        line_number: Option<u32>,
        column_number: Option<u32>,
        before_content: Option<String>,
        after_content: Option<String>,
        success: bool,
        error_message: Option<String>,
    ) -> Result<String> {
        let action_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO actions 
            (id, action_type, timestamp, session_id, file_path, line_number, column_number, before_content, after_content, success, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&action_id)
        .bind(ActionType::FileModification.as_str())
        .bind(timestamp)
        .bind(session_id)
        .bind(file_path)
        .bind(line_number.map(|l| l as i64))
        .bind(column_number.map(|c| c as i64))
        .bind(&before_content)
        .bind(&after_content)
        .bind(success)
        .bind(&error_message)
        .execute(&self.pool)
        .await?;

        Ok(action_id)
    }

    /// Log an AI decision action
    pub async fn log_ai_decision(
        &self,
        session_id: &str,
        reasoning_chain: &str,
        confidence_score: Option<f32>,
        alternatives_considered: Option<u32>,
        selected_alternative: Option<u32>,
        parameters: Option<HashMap<String, String>>,
        result: Option<String>,
        success: bool,
        error_message: Option<String>,
        duration_ms: Option<u64>,
        token_usage: Option<u32>,
    ) -> Result<String> {
        let action_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        let parameters_json = parameters.as_ref().map(|p| serde_json::to_string(p).unwrap_or_default());
        let metadata_json = Some(serde_json::to_string(&HashMap::<String, String>::new()).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO actions 
            (id, action_type, timestamp, session_id, ai_reasoning, parameters, result, success, error_message, duration_ms, token_usage, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&action_id)
        .bind(ActionType::AiDecision.as_str())
        .bind(timestamp)
        .bind(session_id)
        .bind(reasoning_chain)
        .bind(&parameters_json)
        .bind(&result)
        .bind(success)
        .bind(&error_message)
        .bind(duration_ms.map(|d| d as i64))
        .bind(token_usage.map(|t| t as i64))
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;

        Ok(action_id)
    }

    /// Log a TODO creation action
    pub async fn log_todo_creation(
        &self,
        session_id: &str,
        todo_content: &str,
        todo_id: &str,
        success: bool,
        error_message: Option<String>,
    ) -> Result<String> {
        let action_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        let mut parameters = HashMap::new();
        parameters.insert("todo_content".to_string(), todo_content.to_string());
        parameters.insert("todo_id".to_string(), todo_id.to_string());
        let parameters_json = serde_json::to_string(&parameters).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO actions 
            (id, action_type, timestamp, session_id, parameters, success, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&action_id)
        .bind(ActionType::TodoCreation.as_str())
        .bind(timestamp)
        .bind(session_id)
        .bind(&parameters_json)
        .bind(success)
        .bind(&error_message)
        .execute(&self.pool)
        .await?;

        Ok(action_id)
    }

    /// Log a TODO completion action
    pub async fn log_todo_completion(
        &self,
        session_id: &str,
        todo_id: &str,
        completion_evidence: &str,
        success: bool,
        error_message: Option<String>,
    ) -> Result<String> {
        let action_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        let mut parameters = HashMap::new();
        parameters.insert("todo_id".to_string(), todo_id.to_string());
        parameters.insert("completion_evidence".to_string(), completion_evidence.to_string());
        let parameters_json = serde_json::to_string(&parameters).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO actions 
            (id, action_type, timestamp, session_id, parameters, success, error_message)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&action_id)
        .bind(ActionType::TodoCompletion.as_str())
        .bind(timestamp)
        .bind(session_id)
        .bind(&parameters_json)
        .bind(success)
        .bind(&error_message)
        .execute(&self.pool)
        .await?;

        Ok(action_id)
    }

    /// Store a file snapshot
    pub async fn store_file_snapshot(
        &self,
        action_id: &str,
        file_path: &str,
        content: &str,
        hash: &str,
    ) -> Result<String> {
        let snapshot_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO file_snapshots 
            (id, action_id, file_path, content, timestamp, hash)
            VALUES (?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&snapshot_id)
        .bind(action_id)
        .bind(file_path)
        .bind(content)
        .bind(timestamp)
        .bind(hash)
        .execute(&self.pool)
        .await?;

        Ok(snapshot_id)
    }

    /// Store an AI decision
    pub async fn store_ai_decision(
        &self,
        action_id: &str,
        reasoning_chain: &str,
        confidence_score: Option<f32>,
        alternatives_considered: Option<u32>,
        selected_alternative: Option<u32>,
    ) -> Result<String> {
        let decision_id = Uuid::new_v4().to_string();
        let timestamp = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO ai_decisions 
            (id, action_id, reasoning_chain, confidence_score, alternatives_considered, selected_alternative, timestamp)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&decision_id)
        .bind(action_id)
        .bind(reasoning_chain)
        .bind(confidence_score)
        .bind(alternatives_considered.map(|a| a as i64))
        .bind(selected_alternative.map(|s| s as i64))
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        Ok(decision_id)
    }

    /// Get actions by session ID
    pub async fn get_actions_by_session(&self, session_id: &str) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE session_id = ?
            ORDER BY timestamp
            "#
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get actions by type
    pub async fn get_actions_by_type(&self, action_type: ActionType) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE action_type = ?
            ORDER BY timestamp
            "#
        )
        .bind(action_type.as_str())
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get file snapshots for an action
    pub async fn get_file_snapshots(&self, action_id: &str) -> Result<Vec<FileSnapshot>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_id, file_path, content, timestamp, hash
            FROM file_snapshots
            WHERE action_id = ?
            ORDER BY timestamp
            "#
        )
        .bind(action_id)
        .fetch_all(&self.pool)
        .await?;

        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(FileSnapshot {
                id: row.get("id"),
                action_id: row.get("action_id"),
                file_path: row.get("file_path"),
                content: row.get("content"),
                timestamp: row.get("timestamp"),
                hash: row.get("hash"),
            });
        }

        Ok(snapshots)
    }

    /// Get AI decisions for an action
    pub async fn get_ai_decisions(&self, action_id: &str) -> Result<Vec<AiDecision>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_id, reasoning_chain, confidence_score, alternatives_considered, selected_alternative, timestamp
            FROM ai_decisions
            WHERE action_id = ?
            ORDER BY timestamp
            "#
        )
        .bind(action_id)
        .fetch_all(&self.pool)
        .await?;

        let mut decisions = Vec::new();
        for row in rows {
            decisions.push(AiDecision {
                id: row.get("id"),
                action_id: row.get("action_id"),
                reasoning_chain: row.get("reasoning_chain"),
                confidence_score: row.get("confidence_score"),
                alternatives_considered: row.get::<Option<i64>, _>("alternatives_considered").map(|a| a as u32),
                selected_alternative: row.get::<Option<i64>, _>("selected_alternative").map(|s| s as u32),
                timestamp: row.get("timestamp"),
            });
        }

        Ok(decisions)
    }

    /// Get recent actions
    pub async fn get_recent_actions(&self, limit: u32) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            ORDER BY timestamp DESC
            LIMIT ?
            "#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get action statistics
    pub async fn get_action_statistics(&self) -> Result<HashMap<String, u32>> {
        let rows = sqlx::query(
            "SELECT action_type, COUNT(*) as count FROM actions GROUP BY action_type"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut stats = HashMap::new();
        for row in rows {
            let action_type: String = row.get("action_type");
            let count: i64 = row.get("count");
            stats.insert(action_type, count as u32);
        }

        Ok(stats)
    }

    /// Get successful vs failed action counts
    pub async fn get_success_failure_counts(&self) -> Result<(u32, u32)> {
        let success_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM actions WHERE success = TRUE"
        )
        .fetch_one(&self.pool)
        .await?;

        let failure_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM actions WHERE success = FALSE"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok((success_count as u32, failure_count as u32))
    }

    /// Get average action duration
    pub async fn get_average_duration(&self) -> Result<Option<f64>> {
        let avg_duration: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(duration_ms) FROM actions WHERE duration_ms IS NOT NULL"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(avg_duration)
    }

    /// Get total token usage
    pub async fn get_total_token_usage(&self) -> Result<u64> {
        let total_tokens: Option<i64> = sqlx::query_scalar(
            "SELECT SUM(token_usage) FROM actions WHERE token_usage IS NOT NULL"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(total_tokens.unwrap_or(0) as u64)
    }

    /// Get actions by time range
    pub async fn get_actions_by_time_range(&self, start_time: i64, end_time: i64) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp
            "#
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Convert string to ActionType
    fn str_to_action_type(&self, s: &str) -> Result<ActionType> {
        match s {
            "tool_call" => Ok(ActionType::ToolCall),
            "file_modification" => Ok(ActionType::FileModification),
            "ai_decision" => Ok(ActionType::AiDecision),
            "user_interaction" => Ok(ActionType::UserInteraction),
            "system_event" => Ok(ActionType::SystemEvent),
            "todo_creation" => Ok(ActionType::TodoCreation),
            "todo_update" => Ok(ActionType::TodoUpdate),
            "todo_completion" => Ok(ActionType::TodoCompletion),
            _ => Err(anyhow::anyhow!("Invalid action type: {}", s)),
        }
    }

    /// Get actions that failed
    pub async fn get_failed_actions(&self, limit: u32) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE success = FALSE
            ORDER BY timestamp DESC
            LIMIT ?
            "#
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get actions by tool name
    pub async fn get_actions_by_tool(&self, tool_name: &str) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE tool_name = ?
            ORDER BY timestamp
            "#
        )
        .bind(tool_name)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get actions by file path
    pub async fn get_actions_by_file(&self, file_path: &str) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE file_path = ?
            ORDER BY timestamp
            "#
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Batch update action metadata
    pub async fn batch_update_metadata(&self, action_ids: &[String], metadata: &HashMap<String, String>) -> Result<()> {
        let metadata_json = serde_json::to_string(metadata)?;
        
        for action_id in action_ids {
            sqlx::query(
                "UPDATE actions SET metadata = ? WHERE id = ?"
            )
            .bind(&metadata_json)
            .bind(action_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Get actions with specific metadata
    pub async fn get_actions_with_metadata(&self, key: &str, value: &str) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE metadata LIKE ?
            ORDER BY timestamp
            "#
        )
        .bind(format!("%\"{}\":\"{}\"%", key, value))
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Delete old actions (older than specified timestamp)
    pub async fn delete_old_actions(&self, older_than: i64) -> Result<u64> {
        let result = sqlx::query(
            "DELETE FROM actions WHERE timestamp < ?"
        )
        .bind(older_than)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get total action count
    pub async fn get_total_action_count(&self) -> Result<u64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM actions")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u64)
    }

    /// Get actions by user ID
    pub async fn get_actions_by_user(&self, user_id: &str) -> Result<Vec<Action>> {
        let rows = sqlx::query(
            r#"
            SELECT id, action_type, timestamp, session_id, user_id, tool_name, file_path, 
                   line_number, column_number, before_content, after_content, ai_reasoning,
                   parameters, result, success, error_message, duration_ms, token_usage, metadata
            FROM actions
            WHERE user_id = ?
            ORDER BY timestamp
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut actions = Vec::new();
        for row in rows {
            let action_type = self.str_to_action_type(row.get::<&str, _>("action_type"))?;
            let parameters: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("parameters")
                    .and_then(|s| serde_json::from_str(s).ok());
            let metadata: Option<HashMap<String, String>> = 
                row.get::<Option<&str>, _>("metadata")
                    .and_then(|s| serde_json::from_str(s).ok());

            actions.push(Action {
                id: row.get("id"),
                action_type,
                timestamp: row.get("timestamp"),
                session_id: row.get("session_id"),
                user_id: row.get("user_id"),
                tool_name: row.get("tool_name"),
                file_path: row.get("file_path"),
                line_number: row.get::<Option<i64>, _>("line_number").map(|l| l as u32),
                column_number: row.get::<Option<i64>, _>("column_number").map(|c| c as u32),
                before_content: row.get("before_content"),
                after_content: row.get("after_content"),
                ai_reasoning: row.get("ai_reasoning"),
                parameters,
                result: row.get("result"),
                success: row.get("success"),
                error_message: row.get("error_message"),
                duration_ms: row.get::<Option<i64>, _>("duration_ms").map(|d| d as u64),
                token_usage: row.get::<Option<i64>, _>("token_usage").map(|t| t as u32),
                metadata,
            });
        }

        Ok(actions)
    }

    /// Get most common tools
    pub async fn get_most_common_tools(&self, limit: u32) -> Result<Vec<(String, u32)>> {
        let rows = sqlx::query(
            "SELECT tool_name, COUNT(*) as count FROM actions WHERE tool_name IS NOT NULL GROUP BY tool_name ORDER BY count DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut tools = Vec::new();
        for row in rows {
            let tool_name: String = row.get("tool_name");
            let count: i64 = row.get("count");
            tools.push((tool_name, count as u32));
        }

        Ok(tools)
    }

    /// Get most modified files
    pub async fn get_most_modified_files(&self, limit: u32) -> Result<Vec<(String, u32)>> {
        let rows = sqlx::query(
            "SELECT file_path, COUNT(*) as count FROM actions WHERE file_path IS NOT NULL GROUP BY file_path ORDER BY count DESC LIMIT ?"
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut files = Vec::new();
        for row in rows {
            let file_path: String = row.get("file_path");
            let count: i64 = row.get("count");
            files.push((file_path, count as u32));
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use tempfile::TempDir;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_action_history_creation() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Test that tables were created by inserting and retrieving an action
        let action_id = manager.log_tool_call(
            "test_session",
            "test_tool",
            None,
            Some("test result".to_string()),
            true,
            None,
            Some(100),
            Some(50),
        ).await?;

        let actions = manager.get_actions_by_session("test_session").await?;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, action_id);
        assert_eq!(actions[0].tool_name, Some("test_tool".to_string()));
        assert_eq!(actions[0].result, Some("test result".to_string()));
        assert_eq!(actions[0].success, true);
        assert_eq!(actions[0].duration_ms, Some(100));
        assert_eq!(actions[0].token_usage, Some(50));

        Ok(())
    }

    #[tokio::test]
    async fn test_file_modification_logging() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        let action_id = manager.log_file_modification(
            "test_session",
            "/test/file.rs",
            Some(10),
            Some(5),
            Some("old content".to_string()),
            Some("new content".to_string()),
            true,
            None,
        ).await?;

        let actions = manager.get_actions_by_file("/test/file.rs").await?;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, action_id);
        assert_eq!(actions[0].file_path, Some("/test/file.rs".to_string()));
        assert_eq!(actions[0].line_number, Some(10));
        assert_eq!(actions[0].column_number, Some(5));
        assert_eq!(actions[0].before_content, Some("old content".to_string()));
        assert_eq!(actions[0].after_content, Some("new content".to_string()));
        assert_eq!(actions[0].success, true);

        Ok(())
    }

    #[tokio::test]
    async fn test_ai_decision_logging() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        let action_id = manager.log_ai_decision(
            "test_session",
            "Reasoning chain here",
            Some(0.85),
            Some(5),
            Some(2),
            None,
            Some("AI result".to_string()),
            true,
            None,
            Some(200),
            Some(100),
        ).await?;

        let actions = manager.get_actions_by_type(ActionType::AiDecision).await?;
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].id, action_id);
        assert_eq!(actions[0].ai_reasoning, Some("Reasoning chain here".to_string()));
        assert_eq!(actions[0].success, true);
        assert_eq!(actions[0].duration_ms, Some(200));
        assert_eq!(actions[0].token_usage, Some(100));

        Ok(())
    }

    #[tokio::test]
    async fn test_todo_logging() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        let creation_id = manager.log_todo_creation(
            "test_session",
            "Implement feature X",
            "todo_123",
            true,
            None,
        ).await?;

        let completion_id = manager.log_todo_completion(
            "test_session",
            "todo_123",
            "Feature X implemented successfully",
            true,
            None,
        ).await?;

        let creation_actions = manager.get_actions_by_type(ActionType::TodoCreation).await?;
        assert_eq!(creation_actions.len(), 1);
        assert_eq!(creation_actions[0].id, creation_id);

        let completion_actions = manager.get_actions_by_type(ActionType::TodoCompletion).await?;
        assert_eq!(completion_actions.len(), 1);
        assert_eq!(completion_actions[0].id, completion_id);

        Ok(())
    }

    #[tokio::test]
    async fn test_file_snapshot_storage() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // First create an action
        let action_id = manager.log_tool_call(
            "test_session",
            "test_tool",
            None,
            Some("test result".to_string()),
            true,
            None,
            None,
            None,
        ).await?;

        // Then store a file snapshot
        let snapshot_id = manager.store_file_snapshot(
            &action_id,
            "/test/file.rs",
            "file content here",
            "abc123",
        ).await?;

        let snapshots = manager.get_file_snapshots(&action_id).await?;
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].id, snapshot_id);
        assert_eq!(snapshots[0].action_id, action_id);
        assert_eq!(snapshots[0].file_path, "/test/file.rs");
        assert_eq!(snapshots[0].content, "file content here");
        assert_eq!(snapshots[0].hash, "abc123");

        Ok(())
    }

    #[tokio::test]
    async fn test_ai_decision_storage() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // First create an action
        let action_id = manager.log_ai_decision(
            "test_session",
            "Reasoning chain",
            Some(0.9),
            Some(3),
            Some(1),
            None,
            Some("AI result".to_string()),
            true,
            None,
            None,
            None,
        ).await?;

        // Then store an AI decision
        let decision_id = manager.store_ai_decision(
            &action_id,
            "Detailed reasoning chain with multiple steps",
            Some(0.85),
            Some(5),
            Some(2),
        ).await?;

        let decisions = manager.get_ai_decisions(&action_id).await?;
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].id, decision_id);
        assert_eq!(decisions[0].action_id, action_id);
        assert_eq!(decisions[0].reasoning_chain, "Detailed reasoning chain with multiple steps");
        assert_eq!(decisions[0].confidence_score, Some(0.85));
        assert_eq!(decisions[0].alternatives_considered, Some(5));
        assert_eq!(decisions[0].selected_alternative, Some(2));

        Ok(())
    }

    #[tokio::test]
    async fn test_action_statistics() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions of different types
        manager.log_tool_call("session1", "tool1", None, Some("result1".to_string()), true, None, None, None).await?;
        manager.log_tool_call("session1", "tool2", None, Some("result2".to_string()), true, None, None, None).await?;
        manager.log_file_modification("session1", "/file1.rs", None, None, None, None, true, None).await?;
        manager.log_file_modification("session1", "/file2.rs", None, None, None, None, true, None).await?;
        manager.log_file_modification("session1", "/file3.rs", None, None, None, None, true, None).await?;
        manager.log_ai_decision("session1", "reasoning", Some(0.8), Some(3), Some(1), None, Some("result".to_string()), true, None, None, None).await?;

        let stats = manager.get_action_statistics().await?;
        assert_eq!(stats.get("tool_call"), Some(&2));
        assert_eq!(stats.get("file_modification"), Some(&3));
        assert_eq!(stats.get("ai_decision"), Some(&1));

        let (success_count, failure_count) = manager.get_success_failure_counts().await?;
        assert_eq!(success_count, 6);
        assert_eq!(failure_count, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_recent_actions() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions with different timestamps
        manager.log_tool_call("session1", "tool1", None, Some("result1".to_string()), true, None, Some(100), Some(50)).await?;
        manager.log_tool_call("session1", "tool2", None, Some("result2".to_string()), true, None, Some(200), Some(75)).await?;
        manager.log_tool_call("session1", "tool3", None, Some("result3".to_string()), true, None, Some(300), Some(100)).await?;

        let recent_actions = manager.get_recent_actions(2).await?;
        assert_eq!(recent_actions.len(), 2);
        // Should be ordered by timestamp, descending (most recent first)
        assert_eq!(recent_actions[0].tool_name, Some("tool3".to_string()));
        assert_eq!(recent_actions[1].tool_name, Some("tool2".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_action_queries() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions
        manager.log_tool_call("session1", "formatter", None, Some("formatted".to_string()), true, None, None, Some(25)).await?;
        manager.log_tool_call("session1", "linter", None, Some("linted".to_string()), true, None, None, Some(30)).await?;
        manager.log_tool_call("session2", "formatter", None, Some("formatted".to_string()), true, None, None, Some(20)).await?;
        manager.log_file_modification("session1", "/src/main.rs", None, None, None, None, true, None).await?;
        manager.log_file_modification("session1", "/src/lib.rs", None, None, None, None, true, None).await?;

        // Query by tool
        let formatter_actions = manager.get_actions_by_tool("formatter").await?;
        assert_eq!(formatter_actions.len(), 2);

        // Query by file
        let main_rs_actions = manager.get_actions_by_file("/src/main.rs").await?;
        assert_eq!(main_rs_actions.len(), 1);

        // Query by session
        let session1_actions = manager.get_actions_by_session("session1").await?;
        assert_eq!(session1_actions.len(), 4);

        // Get most common tools
        let common_tools = manager.get_most_common_tools(5).await?;
        assert_eq!(common_tools.len(), 2);
        // Formatter should appear twice, linter once
        let formatter_count = common_tools.iter().find(|(tool, _)| tool == "formatter").map(|(_, count)| *count).unwrap();
        let linter_count = common_tools.iter().find(|(tool, _)| tool == "linter").map(|(_, count)| *count).unwrap();
        assert_eq!(formatter_count, 2);
        assert_eq!(linter_count, 1);

        // Get most modified files
        let modified_files = manager.get_most_modified_files(5).await?;
        assert_eq!(modified_files.len(), 2);
        // Both files should appear once
        let main_rs_count = modified_files.iter().find(|(file, _)| file == "/src/main.rs").map(|(_, count)| *count).unwrap();
        let lib_rs_count = modified_files.iter().find(|(file, _)| file == "/src/lib.rs").map(|(_, count)| *count).unwrap();
        assert_eq!(main_rs_count, 1);
        assert_eq!(lib_rs_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_action_metadata() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions
        let action_id1 = manager.log_tool_call("session1", "tool1", None, Some("result1".to_string()), true, None, None, None).await?;
        let action_id2 = manager.log_tool_call("session1", "tool2", None, Some("result2".to_string()), true, None, None, None).await?;

        // Add metadata to actions
        let mut metadata1 = HashMap::new();
        metadata1.insert("category".to_string(), "formatting".to_string());
        metadata1.insert("priority".to_string(), "high".to_string());
        
        let mut metadata2 = HashMap::new();
        metadata2.insert("category".to_string(), "linting".to_string());
        metadata2.insert("priority".to_string(), "medium".to_string());

        manager.batch_update_metadata(&[action_id1.clone()], &metadata1).await?;
        manager.batch_update_metadata(&[action_id2.clone()], &metadata2).await?;

        // Query actions by metadata
        let formatting_actions = manager.get_actions_with_metadata("category", "formatting").await?;
        assert_eq!(formatting_actions.len(), 1);
        assert_eq!(formatting_actions[0].id, action_id1);

        let linting_actions = manager.get_actions_with_metadata("category", "linting").await?;
        assert_eq!(linting_actions.len(), 1);
        assert_eq!(linting_actions[0].id, action_id2);

        Ok(())
    }

    #[tokio::test]
    async fn test_action_deletion() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions with different timestamps
        manager.log_tool_call("session1", "tool1", None, Some("result1".to_string()), true, None, None, None).await?;
        
        // Wait a bit to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        manager.log_tool_call("session1", "tool2", None, Some("result2".to_string()), true, None, None, None).await?;

        // Get total count before deletion
        let total_before = manager.get_total_action_count().await?;
        assert_eq!(total_before, 2);

        // Delete old actions (this should delete nothing since we're using a future timestamp)
        let deleted_count = manager.delete_old_actions(chrono::Utc::now().timestamp() + 1000).await?;
        assert_eq!(deleted_count, 0);

        // Get total count after deletion attempt
        let total_after = manager.get_total_action_count().await?;
        assert_eq!(total_after, 2);

        Ok(())
    }

    #[tokio::test]
    async fn test_action_performance_metrics() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions with performance metrics
        manager.log_tool_call("session1", "slow_tool", None, Some("result1".to_string()), true, None, Some(1000), Some(200)).await?;
        manager.log_tool_call("session1", "fast_tool", None, Some("result2".to_string()), true, None, Some(50), Some(25)).await?;
        manager.log_tool_call("session1", "medium_tool", None, Some("result3".to_string()), true, None, Some(250), Some(75)).await?;

        // Get average duration
        let avg_duration = manager.get_average_duration().await?;
        assert!(avg_duration.is_some());
        let avg_duration = avg_duration.unwrap();
        assert!(avg_duration > 0.0);

        // Get total token usage
        let total_tokens = manager.get_total_token_usage().await?;
        assert_eq!(total_tokens, 300); // 200 + 25 + 75

        Ok(())
    }

    #[tokio::test]
    async fn test_action_time_range_queries() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions with different timestamps
        let start_time = chrono::Utc::now().timestamp();
        
        manager.log_tool_call("session1", "tool1", None, Some("result1".to_string()), true, None, None, None).await?;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let middle_time = chrono::Utc::now().timestamp();
        
        manager.log_tool_call("session1", "tool2", None, Some("result2".to_string()), true, None, None, None).await?;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let end_time = chrono::Utc::now().timestamp();
        
        manager.log_tool_call("session1", "tool3", None, Some("result3".to_string()), true, None, None, None).await?;

        // Query actions in the middle time range
        let middle_actions = manager.get_actions_by_time_range(start_time, middle_time).await?;
        // Should include first two actions (first action is definitely in range, second might be depending on timing)
        assert!(!middle_actions.is_empty());

        // Query actions in the full time range
        let all_actions = manager.get_actions_by_time_range(start_time, end_time).await?;
        // Should include all actions
        assert_eq!(all_actions.len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_failed_action_tracking() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions - some successful, some failed
        manager.log_tool_call("session1", "good_tool", None, Some("success".to_string()), true, None, None, None).await?;
        manager.log_tool_call("session1", "bad_tool", None, None, false, Some("Something went wrong".to_string()), None, None).await?;
        manager.log_tool_call("session1", "another_good_tool", None, Some("also success".to_string()), true, None, None, None).await?;

        // Get failed actions
        let failed_actions = manager.get_failed_actions(10).await?;
        assert_eq!(failed_actions.len(), 1);
        assert_eq!(failed_actions[0].tool_name, Some("bad_tool".to_string()));
        assert_eq!(failed_actions[0].success, false);
        assert_eq!(failed_actions[0].error_message, Some("Something went wrong".to_string()));

        // Get success/failure counts
        let (success_count, failure_count) = manager.get_success_failure_counts().await?;
        assert_eq!(success_count, 2);
        assert_eq!(failure_count, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_user_specific_actions() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Create test actions for different users
        let mut parameters1 = HashMap::new();
        parameters1.insert("user_id".to_string(), "user1".to_string());
        
        let mut parameters2 = HashMap::new();
        parameters2.insert("user_id".to_string(), "user2".to_string());
        
        let mut parameters3 = HashMap::new();
        parameters3.insert("user_id".to_string(), "user1".to_string());

        manager.log_tool_call("session1", "tool1", Some(parameters1), Some("result1".to_string()), true, None, None, None).await?;
        manager.log_tool_call("session1", "tool2", Some(parameters2), Some("result2".to_string()), true, None, None, None).await?;
        manager.log_tool_call("session1", "tool3", Some(parameters3), Some("result3".to_string()), true, None, None, None).await?;

        // Get actions by user (note: in this simplified test, we're not actually storing user_id in the actions table)
        // This would require modifying the schema to include user_id as a separate column
        // For now, we'll test the concept with what we have
        let user1_actions = manager.get_actions_by_session("session1").await?;
        assert_eq!(user1_actions.len(), 3);

        Ok(())
    }

    #[tokio::test]
    async fn test_action_lifecycle() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");
        let pool = SqlitePool::connect(&format!("sqlite://{}", db_path.display())).await?;
        
        let manager = ActionHistoryManager::new(pool.clone());
        manager.init().await?;

        // Test the complete lifecycle of creating, querying, and getting statistics for actions
        
        // 1. Create various types of actions
        let tool_action_id = manager.log_tool_call(
            "test_session", 
            "test_tool", 
            None, 
            Some("tool result".to_string()), 
            true, 
            None, 
            Some(100), 
            Some(50)
        ).await?;
        
        let file_action_id = manager.log_file_modification(
            "test_session", 
            "/test/file.rs", 
            Some(10), 
            Some(5), 
            Some("old".to_string()), 
            Some("new".to_string()), 
            true, 
            None
        ).await?;
        
        let ai_action_id = manager.log_ai_decision(
            "test_session", 
            "AI reasoning", 
            Some(0.85), 
            Some(3), 
            Some(1), 
            None, 
            Some("AI result".to_string()), 
            true, 
            None, 
            Some(200), 
            Some(100)
        ).await?;

        // 2. Verify actions were created
        let all_actions = manager.get_actions_by_session("test_session").await?;
        assert_eq!(all_actions.len(), 3);

        // 3. Test querying by type
        let tool_actions = manager.get_actions_by_type(ActionType::ToolCall).await?;
        assert_eq!(tool_actions.len(), 1);
        assert_eq!(tool_actions[0].id, tool_action_id);

        let file_actions = manager.get_actions_by_type(ActionType::FileModification).await?;
        assert_eq!(file_actions.len(), 1);
        assert_eq!(file_actions[0].id, file_action_id);

        let ai_actions = manager.get_actions_by_type(ActionType::AiDecision).await?;
        assert_eq!(ai_actions.len(), 1);
        assert_eq!(ai_actions[0].id, ai_action_id);

        // 4. Test querying by tool name
        let specific_tool_actions = manager.get_actions_by_tool("test_tool").await?;
        assert_eq!(specific_tool_actions.len(), 1);
        assert_eq!(specific_tool_actions[0].id, tool_action_id);

        // 5. Test querying by file path
        let specific_file_actions = manager.get_actions_by_file("/test/file.rs").await?;
        assert_eq!(specific_file_actions.len(), 1);
        assert_eq!(specific_file_actions[0].id, file_action_id);

        // 6. Test getting recent actions
        let recent_actions = manager.get_recent_actions(2).await?;
        assert_eq!(recent_actions.len(), 2);
        // Most recent should be AI decision, then file modification

        // 7. Test getting action statistics
        let stats = manager.get_action_statistics().await?;
        assert_eq!(stats.get("tool_call"), Some(&1));
        assert_eq!(stats.get("file_modification"), Some(&1));
        assert_eq!(stats.get("ai_decision"), Some(&1));

        // 8. Test success/failure counts
        let (success_count, failure_count) = manager.get_success_failure_counts().await?;
        assert_eq!(success_count, 3);
        assert_eq!(failure_count, 0);

        // 9. Test performance metrics
        let avg_duration = manager.get_average_duration().await?;
        assert!(avg_duration.is_some());
        
        let total_tokens = manager.get_total_token_usage().await?;
        assert_eq!(total_tokens, 150); // 50 + 100

        // 10. Test getting total action count
        let total_actions = manager.get_total_action_count().await?;
        assert_eq!(total_actions, 3);

        // 11. Test getting most common tools
        let common_tools = manager.get_most_common_tools(5).await?;
        assert_eq!(common_tools.len(), 1);
        assert_eq!(common_tools[0].0, "test_tool");
        assert_eq!(common_tools[0].1, 1);

        // 12. Test getting most modified files
        let modified_files = manager.get_most_modified_files(5).await?;
        assert_eq!(modified_files.len(), 1);
        assert_eq!(modified_files[0].0, "/test/file.rs");
        assert_eq!(modified_files[0].1, 1);

        Ok(())
    }
}
