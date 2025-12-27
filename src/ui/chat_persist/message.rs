//! Chat message persistence and queries

use crate::execution_tools::ExecutionDb;
use anyhow::Context;
use rusqlite::params;

use super::{create_chat_message_graph_entity, ChatMessageRecord, ChatRole};

/// Persist user message to SQLite
pub fn persist_user_message(
    this: &ExecutionDb,
    session_id: &str,
    content: &str,
) -> anyhow::Result<i64> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    this.conn()
        .execute(
            "INSERT INTO chat_messages (session_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, "user", content, timestamp],
        )
        .context("Failed to persist user message")?;

    let msg_id = this.conn().last_insert_rowid();

    // Update message count
    this.conn()
        .execute(
            "UPDATE chat_sessions SET message_count = message_count + 1
             WHERE session_id = ?1",
            params![session_id],
        )
        .context("Failed to update message count")?;

    // Create graph entity for message
    let _ = create_chat_message_graph_entity(this, session_id, msg_id, "user", timestamp);

    Ok(msg_id)
}

/// Persist assistant message to SQLite
pub fn persist_assistant_message(
    this: &ExecutionDb,
    session_id: &str,
    content: &str,
) -> anyhow::Result<i64> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    this.conn()
        .execute(
            "INSERT INTO chat_messages (session_id, role, content, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![session_id, "assistant", content, timestamp],
        )
        .context("Failed to persist assistant message")?;

    let msg_id = this.conn().last_insert_rowid();

    // Update message count
    this.conn()
        .execute(
            "UPDATE chat_sessions SET message_count = message_count + 1
             WHERE session_id = ?1",
            params![session_id],
        )
        .context("Failed to update message count")?;

    // Create graph entity for message
    let _ = create_chat_message_graph_entity(this, session_id, msg_id, "assistant", timestamp);

    Ok(msg_id)
}

/// Load all chat messages for a session
pub fn load_chat_session(
    this: &ExecutionDb,
    session_id: &str,
) -> anyhow::Result<Vec<ChatMessageRecord>> {
    let mut stmt = this
        .conn()
        .prepare(
            "SELECT id, session_id, role, content, timestamp
             FROM chat_messages
             WHERE session_id = ?1
             ORDER BY timestamp ASC",
        )
        .context("Failed to prepare message query")?;

    let messages = stmt
        .query_map(params![session_id], |row| {
            let role_str: String = row.get(2)?;
            let role = match role_str.as_str() {
                "user" => ChatRole::User,
                "assistant" => ChatRole::Assistant,
                _ => ChatRole::Assistant,
            };
            Ok(ChatMessageRecord {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role,
                content: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })
        .context("Failed to load chat messages")?
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to collect chat messages")?;

    Ok(messages)
}
