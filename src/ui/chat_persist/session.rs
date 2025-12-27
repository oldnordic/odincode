//! Chat session lifecycle operations

use crate::execution_tools::ExecutionDb;
use anyhow::Context;
use rusqlite::params;

use super::{create_chat_session_graph_entity, ChatSession};

/// Create a new chat session
pub fn create_chat_session(this: &ExecutionDb, session_id: &str) -> anyhow::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    this.conn()
        .execute(
            "INSERT INTO chat_sessions (session_id, start_time, message_count, compacted)
             VALUES (?1, ?2, 0, 0)",
            params![session_id, timestamp],
        )
        .context("Failed to create chat session")?;

    // Create graph entity for session
    create_chat_session_graph_entity(this, session_id, timestamp)?;

    Ok(())
}

/// Mark chat session as complete
pub fn complete_chat_session(this: &ExecutionDb, session_id: &str) -> anyhow::Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    this.conn()
        .execute(
            "UPDATE chat_sessions SET end_time = ?1 WHERE session_id = ?2",
            params![timestamp, session_id],
        )
        .context("Failed to complete chat session")?;

    Ok(())
}

/// Load all chat sessions
pub fn load_chat_sessions(this: &ExecutionDb) -> anyhow::Result<Vec<ChatSession>> {
    let mut stmt = this
        .conn()
        .prepare(
            "SELECT session_id, start_time, end_time, message_count, compacted
             FROM chat_sessions
             ORDER BY start_time DESC",
        )
        .context("Failed to prepare sessions query")?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(ChatSession {
                session_id: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
                message_count: row.get(3)?,
                compacted: row.get(4)?,
            })
        })
        .context("Failed to load chat sessions")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect chat sessions")?;

    Ok(sessions)
}
