//! Chat persistence schema initialization

use crate::execution_tools::ExecutionDb;
use anyhow::Context;

/// Initialize chat persistence schema (chat_sessions, chat_messages tables)
///
/// Called during database initialization if tables don't exist.
pub fn init_chat_schema(this: &ExecutionDb) -> anyhow::Result<()> {
    let conn = this.conn();

    // Create chat_sessions table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_sessions (
            session_id TEXT PRIMARY KEY NOT NULL,
            start_time INTEGER NOT NULL,
            end_time INTEGER,
            message_count INTEGER NOT NULL DEFAULT 0,
            compacted INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )
    .context("Failed to create chat_sessions table")?;

    // Create chat_messages table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chat_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            FOREIGN KEY (session_id) REFERENCES chat_sessions(session_id) ON DELETE CASCADE
        )",
        [],
    )
    .context("Failed to create chat_messages table")?;

    // Create indexes
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_session
         ON chat_messages(session_id)",
        [],
    )
    .context("Failed to create idx_chat_messages_session")?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chat_messages_timestamp
         ON chat_messages(timestamp)",
        [],
    )
    .context("Failed to create idx_chat_messages_timestamp")?;

    Ok(())
}
