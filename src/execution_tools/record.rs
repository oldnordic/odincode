//! Execution recording: Write executions and artifacts to SQLite + SQLiteGraph
//!
//! ## Write Ordering (Non-Negotiable)
//!
//! 1. BEGIN TRANSACTION (execution_log.db)
//! 2. INSERT INTO executions
//! 3. INSERT INTO execution_artifacts (if any)
//! 4. COMMIT (execution_log.db)
//! 5. BEGIN TRANSACTION (codegraph.db)
//! 6. INSERT INTO graph_entities (execution node)
//! 7. INSERT INTO graph_edges (all edges)
//! 8. COMMIT (codegraph.db)
//!
//! ## Failure Semantics
//!
//! * SQLite failure → nothing written
//! * SQLiteGraph failure → execution_log.db persists, graph missing (detectable)
//! * No retries, no cross-DB rollback

use crate::execution_tools::{db::ExecutionDb, graph};
use anyhow::Result;
use rusqlite::params;
use serde_json::Value;

/// Execution record (returned by queries)
#[derive(Debug, Clone)]
pub struct Execution {
    pub id: String,
    pub tool_name: String,
    pub timestamp: i64,
    pub success: bool,
}

impl ExecutionDb {
    /// Record execution with dual-write (SQLite + SQLiteGraph)
    ///
    /// # Write Ordering
    /// 1. SQLite transaction (executions row)
    /// 2. SQLiteGraph transaction (execution entity + EXECUTED_ON edge to file)
    ///
    /// # Failure Semantics
    /// * If SQLite fails → nothing written
    /// * If SQLiteGraph fails → SQLite persists, graph missing (returns Ok)
    #[allow(clippy::too_many_arguments)]
    pub fn record_execution_on_file(
        &self,
        id: &str,
        tool_name: &str,
        arguments: &Value,
        timestamp: i64,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: Option<i64>,
        error_message: Option<&str>,
        file_path: &str,
    ) -> Result<()> {
        // Step 1-4: SQLite write (execution_log.db)
        self.write_execution_sqlite(
            id,
            tool_name,
            arguments,
            timestamp,
            success,
            exit_code,
            duration_ms,
            error_message,
        )?;

        // Step 5-8: SQLiteGraph write (codegraph.db)
        // Note: If this fails, SQLite data remains (best-effort dual-write)
        let _ =
            self.write_execution_graph(id, tool_name, timestamp, success, file_path, "EXECUTED_ON");

        Ok(())
    }

    /// Record execution (SQLite only, no graph edges)
    #[allow(clippy::too_many_arguments)]
    pub fn record_execution(
        &self,
        id: &str,
        tool_name: &str,
        arguments: &Value,
        timestamp: i64,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<()> {
        // Step 1-4: SQLite write only
        self.write_execution_sqlite(
            id,
            tool_name,
            arguments,
            timestamp,
            success,
            exit_code,
            duration_ms,
            error_message,
        )?;

        // Step 5-8: Create execution entity only (no edges)
        let _ = self.write_execution_graph_entity(id, tool_name, timestamp, success);

        Ok(())
    }

    /// Record execution with artifacts
    #[allow(clippy::too_many_arguments)]
    pub fn record_execution_with_artifacts(
        &self,
        id: &str,
        tool_name: &str,
        arguments: &Value,
        timestamp: i64,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: Option<i64>,
        error_message: Option<&str>,
        artifacts: &[(&str, &Value)], // (artifact_type, content)
    ) -> Result<()> {
        // Step 1: Begin transaction
        let tx = self.conn().unchecked_transaction()?;

        // Step 2: Insert execution
        tx.execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                tool_name,
                serde_json::to_string(arguments)?,
                timestamp,
                success,
                exit_code,
                duration_ms,
                error_message,
            ],
        )?;

        // Step 3: Insert artifacts
        for (artifact_type, content) in artifacts {
            tx.execute(
                "INSERT INTO execution_artifacts (execution_id, artifact_type, content_json)
                 VALUES (?1, ?2, ?3)",
                params![id, artifact_type, serde_json::to_string(content)?],
            )?;
        }

        // Step 4: Commit
        tx.commit()?;

        // Step 5-8: Create execution entity only
        let _ = self.write_execution_graph_entity(id, tool_name, timestamp, success);

        Ok(())
    }

    /// Write execution to SQLite (execution_log.db)
    #[allow(clippy::too_many_arguments)]
    fn write_execution_sqlite(
        &self,
        id: &str,
        tool_name: &str,
        arguments: &Value,
        timestamp: i64,
        success: bool,
        exit_code: Option<i32>,
        duration_ms: Option<i64>,
        error_message: Option<&str>,
    ) -> Result<()> {
        self.conn().execute(
            "INSERT INTO executions (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                tool_name,
                serde_json::to_string(arguments)?,
                timestamp,
                success,
                exit_code,
                duration_ms,
                error_message,
            ],
        )?;
        Ok(())
    }

    /// Write execution entity + EXECUTED_ON edge to SQLiteGraph
    fn write_execution_graph(
        &self,
        id: &str,
        tool_name: &str,
        timestamp: i64,
        success: bool,
        file_path: &str,
        edge_type: &str,
    ) -> Result<()> {
        // Create execution entity
        let entity_id = self.write_execution_graph_entity(id, tool_name, timestamp, success)?;

        // Find file entity ID
        let file_id: i64 = self.graph_conn().query_row(
            "SELECT id FROM graph_entities WHERE kind = 'File' AND name = ?1",
            params![file_path],
            |row| row.get::<_, i64>(0),
        )?;

        // Create EXECUTED_ON edge
        graph::create_edge(
            self.graph_conn(),
            entity_id,
            file_id,
            edge_type,
            &serde_json::json!({
                "operation": "read",
                "execution_id": id
            }),
        )?;

        Ok(())
    }

    /// Write execution entity to SQLiteGraph
    fn write_execution_graph_entity(
        &self,
        id: &str,
        tool_name: &str,
        timestamp: i64,
        success: bool,
    ) -> Result<i64> {
        self.graph_conn().execute(
            "INSERT INTO graph_entities (kind, name, file_path, data) VALUES (?1, ?2, ?3, ?4)",
            params![
                "execution",
                format!("{}:{}", tool_name, id),
                None::<&str>,
                serde_json::to_string(&serde_json::json!({
                    "tool": tool_name,
                    "timestamp": timestamp,
                    "success": success,
                    "execution_id": id
                }))?,
            ],
        )?;
        Ok(self.graph_conn().last_insert_rowid())
    }

    // Phase 9.2: Approval event logging

    /// Record approval granted event (Phase 9.2)
    pub fn record_approval_granted(
        &self,
        session_id: &str,
        tool: &str,
        scope: &str,
        args: &Value,
    ) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let exec_id = format!("approval_granted_{}_{}", session_id, tool);

        let arguments = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "scope": scope,
            "args": args,
        });

        let artifact = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "scope": scope,
            "timestamp": timestamp,
        });

        self.record_execution_with_artifacts(
            &exec_id,
            "approval_granted",
            &arguments,
            timestamp,
            true,
            None,
            None,
            None,
            &[("approval_granted", &artifact)],
        )
    }

    /// Record approval denied event (Phase 9.2)
    pub fn record_approval_denied(
        &self,
        session_id: &str,
        tool: &str,
        args: &Value,
        reason: &str,
    ) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let exec_id = format!("approval_denied_{}_{}", session_id, tool);

        let arguments = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "args": args,
            "reason": reason,
        });

        let artifact = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "reason": reason,
            "timestamp": timestamp,
        });

        self.record_execution_with_artifacts(
            &exec_id,
            "approval_denied",
            &arguments,
            timestamp,
            true,
            None,
            None,
            None,
            &[("approval_denied", &artifact)],
        )
    }
}
