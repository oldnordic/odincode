//! Chat persistence (Phase 8.6)
//!
//! Persists chat sessions and messages to SQLite + SQLiteGraph.
//! All persistence happens on main thread, NOT in chat thread.

mod graph;
mod message;
mod schema;
mod session;

// Re-export all public types
pub use graph::{create_chat_message_graph_entity, create_chat_session_graph_entity};
pub use message::{load_chat_session, persist_assistant_message, persist_user_message};
pub use schema::init_chat_schema;
pub use session::{complete_chat_session, create_chat_session, load_chat_sessions};

use crate::execution_tools::ExecutionDb;

/// Chat message role for persistence
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
}

/// Chat session record
#[derive(Debug, Clone)]
pub struct ChatSession {
    pub session_id: String,
    pub start_time: i64,
    pub end_time: Option<i64>,
    pub message_count: i32,
    pub compacted: bool,
}

/// Chat message record
#[derive(Debug, Clone)]
pub struct ChatMessageRecord {
    pub id: i64,
    pub session_id: String,
    pub role: ChatRole,
    pub content: String,
    pub timestamp: i64,
}

// ExecutionDb impl blocks to preserve original API
impl ExecutionDb {
    /// Initialize chat persistence schema
    pub fn init_chat_schema(&self) -> anyhow::Result<()> {
        init_chat_schema(self)
    }

    /// Create a new chat session
    pub fn create_chat_session(&self, session_id: &str) -> anyhow::Result<()> {
        create_chat_session(self, session_id)
    }

    /// Mark chat session as complete
    pub fn complete_chat_session(&self, session_id: &str) -> anyhow::Result<()> {
        complete_chat_session(self, session_id)
    }

    /// Load all chat sessions
    pub fn load_chat_sessions(&self) -> anyhow::Result<Vec<ChatSession>> {
        load_chat_sessions(self)
    }

    /// Persist user message to SQLite
    pub fn persist_user_message(&self, session_id: &str, content: &str) -> anyhow::Result<i64> {
        persist_user_message(self, session_id, content)
    }

    /// Persist assistant message to SQLite
    pub fn persist_assistant_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> anyhow::Result<i64> {
        persist_assistant_message(self, session_id, content)
    }

    /// Load all chat messages for a session
    pub fn load_chat_session(&self, session_id: &str) -> anyhow::Result<Vec<ChatMessageRecord>> {
        load_chat_session(self, session_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_db() -> (TempDir, crate::execution_tools::ExecutionDb) {
        let temp_dir = TempDir::new().unwrap();
        let db_root = temp_dir.path();

        // Create codegraph.db with required schema
        let codegraph_path = db_root.join("codegraph.db");
        let conn = rusqlite::Connection::open(&codegraph_path).unwrap();

        // Create graph_entities table
        conn.execute(
            "CREATE TABLE graph_entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                name TEXT NOT NULL,
                file_path TEXT,
                data TEXT
            )",
            [],
        )
        .unwrap();

        // Create graph_edges table
        conn.execute(
            "CREATE TABLE graph_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_id INTEGER NOT NULL,
                to_id INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                data TEXT,
                FOREIGN KEY (from_id) REFERENCES graph_entities(id) ON DELETE CASCADE,
                FOREIGN KEY (to_id) REFERENCES graph_entities(id) ON DELETE CASCADE
            )",
            [],
        )
        .unwrap();

        // Create minimal config
        let config_path = db_root.join("config.toml");
        let mut config_file = File::create(&config_path).unwrap();
        writeln!(
            config_file,
            r#"[llm]
mode = "external"
provider = "stub"
"#
        )
        .unwrap();

        let exec_db = crate::execution_tools::ExecutionDb::open(db_root).unwrap();
        exec_db.init_chat_schema().unwrap();

        (temp_dir, exec_db)
    }

    #[test]
    fn test_init_chat_schema() {
        let (_temp, exec_db) = create_test_db();

        // Tables should exist
        let table_count: i64 = exec_db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type='table' AND name IN ('chat_sessions', 'chat_messages')",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(table_count, 2);
    }

    #[test]
    fn test_create_and_complete_chat_session() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-session-123";
        exec_db.create_chat_session(session_id).unwrap();

        // Verify session exists
        let count: i64 = exec_db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chat_sessions WHERE session_id = ?1",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);

        // Complete session
        exec_db.complete_chat_session(session_id).unwrap();

        // Verify end_time is set
        let end_time: Option<i64> = exec_db
            .conn()
            .query_row(
                "SELECT end_time FROM chat_sessions WHERE session_id = ?1",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(end_time.is_some());
    }

    #[test]
    fn test_persist_user_message() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-session-456";
        exec_db.create_chat_session(session_id).unwrap();

        let msg_id = exec_db.persist_user_message(session_id, "hello").unwrap();

        assert!(msg_id > 0);

        // Verify message exists
        let count: i64 = exec_db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1 AND role = 'user'",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);

        // Verify message count updated
        let msg_count: i32 = exec_db
            .conn()
            .query_row(
                "SELECT message_count FROM chat_sessions WHERE session_id = ?1",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(msg_count, 1);
    }

    #[test]
    fn test_persist_assistant_message() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-session-789";
        exec_db.create_chat_session(session_id).unwrap();

        let msg_id = exec_db
            .persist_assistant_message(session_id, "hi there")
            .unwrap();

        assert!(msg_id > 0);

        // Verify message exists
        let count: i64 = exec_db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM chat_messages WHERE session_id = ?1 AND role = 'assistant'",
                rusqlite::params![session_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_load_chat_session() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-session-load";
        exec_db.create_chat_session(session_id).unwrap();
        exec_db
            .persist_user_message(session_id, "user msg")
            .unwrap();
        exec_db
            .persist_assistant_message(session_id, "assistant msg")
            .unwrap();

        let messages = exec_db.load_chat_session(session_id).unwrap();

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, ChatRole::User);
        assert_eq!(messages[0].content, "user msg");
        assert_eq!(messages[1].role, ChatRole::Assistant);
        assert_eq!(messages[1].content, "assistant msg");
    }

    #[test]
    fn test_load_chat_sessions() {
        let (_temp, exec_db) = create_test_db();

        exec_db.create_chat_session("session-1").unwrap();
        exec_db.create_chat_session("session-2").unwrap();

        let sessions = exec_db.load_chat_sessions().unwrap();

        assert_eq!(sessions.len(), 2);
    }
}
