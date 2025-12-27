//! Execution database: SQLite schema initialization and connection management
//!
//! ## Architecture
//!
//! - `ExecutionDb::open()` — Initialize execution_log.db and open codegraph.db
//! - Auto-creates execution_log.db with full schema if missing
//! - Fails if codegraph.db missing (returns Error::CodegraphNotFound)
//! - Provides access to both connections for queries

use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Execution database handle
///
/// Manages dual connections:
/// - SQLite (execution_log.db) for temporal log
/// - SQLiteGraph (codegraph.db) for relationships
pub struct ExecutionDb {
    sqlite_conn: Connection,
    graph_conn: Connection,
}

/// Execution memory errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("codegraph.db not found at {0}")]
    CodegraphNotFound(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl ExecutionDb {
    /// Open execution memory at given DB root
    ///
    /// # Arguments
    /// * `db_root` - Directory containing execution_log.db and codegraph.db
    ///
    /// # Behavior
    /// * Creates execution_log.db if missing (with full schema)
    /// * Fails if codegraph.db missing (Error::CodegraphNotFound)
    ///
    /// # Returns
    /// * `Ok(ExecutionDb)` — Both connections open
    /// * `Err(Error::CodegraphNotFound)` — codegraph.db missing
    pub fn open<P: AsRef<Path>>(db_root: P) -> Result<Self> {
        let root = db_root.as_ref();

        // Path to codegraph.db (must exist)
        let codegraph_path = root.join("codegraph.db");
        if !codegraph_path.exists() {
            return Err(Error::CodegraphNotFound(
                codegraph_path.display().to_string(),
            ))
            .context("codegraph.db not found");
        }

        // Path to execution_log.db (auto-create if missing)
        let exec_log_path = root.join("execution_log.db");
        let exec_log_exists = exec_log_path.exists();

        // Open execution_log.db connection
        let sqlite_conn =
            Connection::open(&exec_log_path).context("Failed to open execution_log.db")?;

        // Initialize schema if this is a new database
        if !exec_log_exists {
            Self::init_schema(&sqlite_conn)?;
        }

        // Open codegraph.db connection
        let graph_conn =
            Connection::open(&codegraph_path).context("Failed to open codegraph.db")?;

        let db = ExecutionDb {
            sqlite_conn,
            graph_conn,
        };

        // Phase 8.6: Initialize chat schema (idempotent, safe to call on existing DB)
        let _ = db.init_chat_schema();

        Ok(db)
    }

    /// Get SQLite connection (execution_log.db)
    ///
    /// Exposed for direct queries in tests and query.rs
    pub fn conn(&self) -> &Connection {
        &self.sqlite_conn
    }

    /// Get SQLiteGraph connection (codegraph.db)
    ///
    /// Exposed for direct queries in tests and graph.rs
    pub fn graph_conn(&self) -> &Connection {
        &self.graph_conn
    }

    /// Initialize execution_log.db schema (tables, indexes, triggers)
    ///
    /// Called automatically on first open if database doesn't exist.
    fn init_schema(conn: &Connection) -> Result<()> {
        // Create executions table
        conn.execute(
            "CREATE TABLE executions (
                id TEXT PRIMARY KEY NOT NULL,
                tool_name TEXT NOT NULL,
                arguments_json TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                success BOOLEAN NOT NULL,
                exit_code INTEGER,
                duration_ms INTEGER,
                error_message TEXT
            )",
            [],
        )
        .context("Failed to create executions table")?;

        // Create execution_artifacts table
        conn.execute(
            "CREATE TABLE execution_artifacts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                execution_id TEXT NOT NULL,
                artifact_type TEXT NOT NULL,
                content_json TEXT NOT NULL,
                FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE RESTRICT
            )",
            [],
        )
        .context("Failed to create execution_artifacts table")?;

        // Phase 10.7: Create discovery_events table
        conn.execute(
            "CREATE TABLE discovery_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                user_query_hash TEXT NOT NULL,
                tools_discovered TEXT NOT NULL,
                reason TEXT NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )
        .context("Failed to create discovery_events table")?;

        // Create indexes
        conn.execute(
            "CREATE INDEX idx_executions_tool ON executions(tool_name)",
            [],
        )
        .context("Failed to create idx_executions_tool")?;

        conn.execute(
            "CREATE INDEX idx_executions_timestamp ON executions(timestamp)",
            [],
        )
        .context("Failed to create idx_executions_timestamp")?;

        conn.execute(
            "CREATE INDEX idx_executions_success ON executions(success)",
            [],
        )
        .context("Failed to create idx_executions_success")?;

        conn.execute(
            "CREATE INDEX idx_executions_tool_timestamp ON executions(tool_name, timestamp)",
            [],
        )
        .context("Failed to create idx_executions_tool_timestamp")?;

        conn.execute(
            "CREATE INDEX idx_artifacts_execution ON execution_artifacts(execution_id)",
            [],
        )
        .context("Failed to create idx_artifacts_execution")?;

        conn.execute(
            "CREATE INDEX idx_artifacts_type ON execution_artifacts(artifact_type)",
            [],
        )
        .context("Failed to create idx_artifacts_type")?;

        conn.execute(
            "CREATE INDEX idx_artifacts_execution_type ON execution_artifacts(execution_id, artifact_type)",
            [],
        ).context("Failed to create idx_artifacts_execution_type")?;

        // Phase 10.7: Create discovery_events indexes
        conn.execute(
            "CREATE INDEX idx_discovery_session ON discovery_events(session_id)",
            [],
        )
        .context("Failed to create idx_discovery_session")?;

        conn.execute(
            "CREATE INDEX idx_discovery_timestamp ON discovery_events(timestamp)",
            [],
        )
        .context("Failed to create idx_discovery_timestamp")?;

        conn.execute(
            "CREATE INDEX idx_discovery_query_hash ON discovery_events(user_query_hash)",
            [],
        )
        .context("Failed to create idx_discovery_query_hash")?;

        // Create triggers
        Self::init_triggers(conn)?;

        Ok(())
    }

    /// Initialize validation triggers
    fn init_triggers(conn: &Connection) -> Result<()> {
        // Validate tool_name (Phase 9.2: added approval_granted, approval_denied; Phase 1.1: added memory_query; Phase 1.2: added execution_summary; Phase 2: added file_edit; Phase 3: added git_status, git_diff, git_log; Phase 4: added wc, bash_exec)
        conn.execute(
            "CREATE TRIGGER validate_tool_name BEFORE INSERT ON executions
            BEGIN
                SELECT CASE
                    WHEN NEW.tool_name NOT IN (
                        'file_read', 'file_write', 'file_create',
                        'file_search', 'file_glob',
                        'file_edit',
                        'splice_patch', 'splice_plan',
                        'symbols_in_file', 'references_to_symbol_name', 'references_from_file_to_symbol_name',
                        'lsp_check',
                        'llm_plan', 'llm_explain',
                        'llm_preflight',
                        'memory_query',
                        'execution_summary',
                        'git_status', 'git_diff', 'git_log',
                        'wc', 'bash_exec',
                        'approval_granted', 'approval_denied'
                    ) THEN RAISE(ABORT, 'Invalid tool_name')
                END;
            END",
            [],
        ).context("Failed to create validate_tool_name trigger")?;

        // Validate timestamp
        conn.execute(
            "CREATE TRIGGER validate_timestamp BEFORE INSERT ON executions
            BEGIN
                SELECT CASE
                    WHEN NEW.timestamp < 1577836800000 THEN RAISE(ABORT, 'Timestamp too old')
                    WHEN NEW.timestamp > strftime('%s', 'now') * 1000 + 86400000 THEN RAISE(ABORT, 'Timestamp in future')
                END;
            END",
            [],
        ).context("Failed to create validate_timestamp trigger")?;

        // Validate artifact_type (Phase 4.5: added plan_edit; Phase 5: added adapter_* types; Phase 8.6: added chat_* types; Phase 9.2: added approval_* types)
        conn.execute(
            "CREATE TRIGGER validate_artifact_type BEFORE INSERT ON execution_artifacts
            BEGIN
                SELECT CASE
                    WHEN NEW.artifact_type NOT IN ('stdout', 'stderr', 'diagnostics', 'prompt', 'plan', 'validation_error', 'llm_preflight', 'llm_plan_stream', 'plan_edit', 'adapter_call', 'adapter_response', 'adapter_stream_chunk', 'adapter_error', 'chat_user_message', 'chat_assistant_message', 'chat_session', 'chat_summary', 'approval_granted', 'approval_denied') THEN
                        RAISE(ABORT, 'Invalid artifact_type')
                END;
            END",
            [],
        ).context("Failed to create validate_artifact_type trigger")?;

        // Validate JSON
        conn.execute(
            "CREATE TRIGGER validate_json BEFORE INSERT ON execution_artifacts
            WHEN json_valid(NEW.content_json) != 1
            BEGIN
                SELECT RAISE(ABORT, 'Invalid JSON in content_json');
            END",
            [],
        )
        .context("Failed to create validate_json trigger")?;

        Ok(())
    }
}
