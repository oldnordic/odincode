//! Chat compaction logic and operations

use crate::execution_tools::ExecutionDb;
use anyhow::Context;
use rusqlite::params;

use super::create_compacted_edge;
use super::{CompactionResult, CompactionTrigger};

/// Check if session should be compacted
pub fn should_compact_session(
    this: &ExecutionDb,
    session_id: &str,
    trigger: &CompactionTrigger,
) -> anyhow::Result<bool> {
    let message_count: i64 = this
        .conn()
        .query_row(
            "SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1",
            params![session_id],
            |row| row.get(0),
        )
        .context("Failed to count messages")?;

    // Simple heuristic: message count threshold
    Ok(message_count >= trigger.min_messages as i64)
}

/// Compact chat session (placeholder for LLM summarization)
///
/// NOTE: This is a simplified implementation that creates a summary marker.
/// Full implementation would:
/// 1. Call LLM to generate summary of oldest N messages
/// 2. Store summary as new chat_message with role="summary"
/// 3. Create COMPACTED_TO edges linking originals to summary
/// 4. Mark session as compacted
pub fn compact_session(
    _this: &ExecutionDb,
    _session_id: &str,
    _num_messages: usize,
) -> anyhow::Result<CompactionResult> {
    // Placeholder: Actual LLM call required for real summarization
    // For now, this creates the structure for compaction
    anyhow::bail!("Compaction requires LLM integration for summarization")
}

/// Mark a range of messages as compacted to a summary
pub fn mark_messages_compacted(
    this: &ExecutionDb,
    session_id: &str,
    message_ids: &[i64],
    summary_id: i64,
) -> anyhow::Result<()> {
    // Create COMPACTED_TO edges in graph (best-effort)
    for &msg_id in message_ids {
        let _ = create_compacted_edge(this, session_id, msg_id, summary_id);
    }
    Ok(())
}

/// Simple timestamp for graph edge data
pub(crate) fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
