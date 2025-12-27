//! Evidence database: Read-only access to execution memory
//!
//! ## Architecture
//!
//! - `EvidenceDb::open()` — Open read-only connections to execution_log.db and codegraph.db
//! - Codegraph is optional (queries degrade gracefully when missing)
//! - All queries are SELECT-only (no mutations)

use super::Error;
use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::Path;

/// Evidence database handle (read-only)
///
/// Manages dual connections for evidence queries:
/// - SQLite (execution_log.db) for temporal log
/// - SQLiteGraph (codegraph.db) for relationships
///
/// ## Best-Effort Graceful Degradation
///
/// - If codegraph.db missing, graph_conn is None
/// - Graph-dependent queries return errors or use SQLite fallback
/// - SQLite queries continue working without graph
pub struct EvidenceDb {
    /// SQLite connection (execution_log.db) — always present
    sqlite_conn: Connection,
    /// SQLiteGraph connection (codegraph.db) — optional
    graph_conn: Option<Connection>,
}

impl EvidenceDb {
    /// Open evidence databases at given DB root
    ///
    /// # Arguments
    /// * `db_root` - Directory containing execution_log.db and optionally codegraph.db
    ///
    /// # Behavior
    /// * execution_log.db must exist (created by Phase 0.5.2 execution recording)
    /// * codegraph.db is optional (graceful degradation for graph queries)
    ///
    /// # Returns
    /// * `Ok(EvidenceDb)` — At least SQLite connection open
    /// * `Err(Error::ExecutionLogNotFound)` — execution_log.db missing
    pub fn open<P: AsRef<Path>>(db_root: P) -> Result<Self> {
        let root = db_root.as_ref();

        // execution_log.db must exist (created by execution recording)
        let exec_log_path = root.join("execution_log.db");
        if !exec_log_path.exists() {
            return Err(Error::ExecutionLogNotFound(
                exec_log_path.display().to_string(),
            ))
            .context("execution_log.db not found");
        }

        // Open execution_log.db connection
        let sqlite_conn =
            Connection::open(&exec_log_path).context("Failed to open execution_log.db")?;

        // codegraph.db is optional (try to open, don't fail if missing)
        let codegraph_path = root.join("codegraph.db");
        let graph_conn = if codegraph_path.exists() {
            Some(Connection::open(&codegraph_path).context("Failed to open codegraph.db")?)
        } else {
            None
        };

        Ok(EvidenceDb {
            sqlite_conn,
            graph_conn,
        })
    }

    /// Get SQLite connection (execution_log.db)
    ///
    /// Used for all SQLite-only queries (Q1, Q2, Q3, Q5 parts, Q7, Q8)
    pub fn conn(&self) -> &Connection {
        &self.sqlite_conn
    }

    /// Get SQLiteGraph connection (codegraph.db)
    ///
    /// Returns None if codegraph.db missing (best-effort dual-write gap)
    ///
    /// Used for graph queries (Q4, Q5 graph parts, Q6)
    pub fn graph_conn(&self) -> Option<&Connection> {
        self.graph_conn.as_ref()
    }

    /// Check if graph database is available
    pub fn has_graph(&self) -> bool {
        self.graph_conn.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_fails_without_execution_log() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_root = temp_dir.path();

        // No databases created
        let result = EvidenceDb::open(db_root);
        assert!(result.is_err());
    }

    #[test]
    fn test_open_succeeds_with_execution_log_only() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_root = temp_dir.path();

        // Create execution_log.db only
        let exec_log_path = db_root.join("execution_log.db");
        {
            let conn = Connection::open(&exec_log_path).unwrap();
            conn.execute(
                "CREATE TABLE executions (id TEXT PRIMARY KEY, tool_name TEXT)",
                [],
            )
            .unwrap();
        } // Drop connection to release lock

        let ev_db = EvidenceDb::open(db_root).unwrap();
        // Use query_row for SELECT (execute is for statements without results)
        let count: i64 = ev_db
            .conn()
            .query_row("SELECT COUNT(*) FROM executions", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0); // Table exists, no rows yet
        assert!(!ev_db.has_graph());
        assert!(ev_db.graph_conn().is_none());
    }

    #[test]
    fn test_open_succeeds_with_both_databases() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_root = temp_dir.path();

        // Create execution_log.db
        let exec_log_path = db_root.join("execution_log.db");
        {
            let exec_conn = Connection::open(&exec_log_path).unwrap();
            exec_conn
                .execute(
                    "CREATE TABLE executions (id TEXT PRIMARY KEY, tool_name TEXT)",
                    [],
                )
                .unwrap();
        }

        // Create codegraph.db
        let codegraph_path = db_root.join("codegraph.db");
        {
            let graph_conn = Connection::open(&codegraph_path).unwrap();
            graph_conn
                .execute(
                    "CREATE TABLE graph_entities (id INTEGER PRIMARY KEY, kind TEXT)",
                    [],
                )
                .unwrap();
        }

        let ev_db = EvidenceDb::open(db_root).unwrap();
        assert!(ev_db.has_graph());
        assert!(ev_db.graph_conn().is_some());
    }
}
