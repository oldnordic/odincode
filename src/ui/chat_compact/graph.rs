//! SQLiteGraph operations for chat compaction

use crate::execution_tools::ExecutionDb;
use anyhow::Context;
use rusqlite::params;
use serde_json::json;

use super::logic::chrono_timestamp;

/// Create COMPACTED_TO edge (best-effort)
pub fn create_compacted_edge(
    this: &ExecutionDb,
    _session_id: &str,
    message_id: i64,
    summary_id: i64,
) -> anyhow::Result<()> {
    // Find message entity ID
    let from_entity: Option<i64> = this
        .graph_conn()
        .query_row(
            "SELECT id FROM graph_entities
             WHERE kind = 'chat_message'
             AND name LIKE ?1",
            [format!("%:{}", message_id)],
            |row| row.get(0),
        )
        .ok();

    let Some(from_id) = from_entity else {
        return Ok(()); // Skip if no entity found
    };

    // Create summary entity (if not exists)
    let to_id = this
        .graph_conn()
        .execute(
            "INSERT OR IGNORE INTO graph_entities (kind, name, file_path, data)
             VALUES ('chat_summary', ?1, ?2, ?3)",
            params![
                format!("summary-{}", summary_id),
                None::<&str>,
                json!({ "summary_id": summary_id }).to_string(),
            ],
        )
        .context("Failed to create summary entity")?;

    let to_id = if to_id > 0 {
        to_id
    } else {
        this.graph_conn()
            .query_row(
                "SELECT id FROM graph_entities WHERE kind = 'chat_summary' AND name = ?1",
                [format!("summary-{}", summary_id)],
                |row| row.get(0),
            )
            .context("Failed to query summary entity")?
    };

    // Create edge
    this.graph_conn()
        .execute(
            "INSERT INTO graph_edges (from_id, to_id, edge_type, data)
             VALUES (?1, ?2, 'COMPACTED_TO', ?3)",
            params![
                from_id,
                to_id,
                json!({ "compacted_at": chrono_timestamp() }).to_string(),
            ],
        )
        .context("Failed to create compacted edge")?;

    Ok(())
}

/// Load compacted view of chat (summaries + recent messages)
pub fn load_compacted_chat(
    this: &ExecutionDb,
    session_id: &str,
    recent_count: usize,
) -> anyhow::Result<Vec<String>> {
    // Load recent messages (not compacted)
    let mut stmt = this
        .conn()
        .prepare(
            "SELECT content FROM chat_messages
             WHERE session_id = ?1
             ORDER BY timestamp DESC
             LIMIT ?2",
        )
        .context("Failed to prepare compacted chat query")?;

    let messages: Vec<String> = stmt
        .query_map(params![session_id, recent_count as i64], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect compacted chat messages")?;

    // Reverse to get chronological order
    let messages: Vec<String> = messages.into_iter().rev().collect();

    Ok(messages)
}
