//! SQLiteGraph entity creation for chat persistence

use crate::execution_tools::ExecutionDb;
use anyhow::Context;
use rusqlite::params;
use serde_json::json;

/// Create chat session graph entity (best-effort)
pub fn create_chat_session_graph_entity(
    this: &ExecutionDb,
    session_id: &str,
    timestamp: i64,
) -> anyhow::Result<i64> {
    this.graph_conn()
        .execute(
            "INSERT INTO graph_entities (kind, name, file_path, data)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                "chat_session",
                session_id,
                None::<&str>,
                json!({
                    "session_id": session_id,
                    "timestamp": timestamp,
                })
                .to_string(),
            ],
        )
        .context("Failed to create chat session graph entity")?;
    Ok(this.graph_conn().last_insert_rowid())
}

/// Create chat message graph entity (best-effort)
pub fn create_chat_message_graph_entity(
    this: &ExecutionDb,
    session_id: &str,
    msg_id: i64,
    role: &str,
    timestamp: i64,
) -> anyhow::Result<i64> {
    this.graph_conn()
        .execute(
            "INSERT INTO graph_entities (kind, name, file_path, data)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                "chat_message",
                format!("{}:{}", session_id, msg_id),
                None::<&str>,
                json!({
                    "session_id": session_id,
                    "message_id": msg_id,
                    "role": role,
                    "timestamp": timestamp,
                })
                .to_string(),
            ],
        )
        .context("Failed to create chat message graph entity")?;
    Ok(this.graph_conn().last_insert_rowid())
}
