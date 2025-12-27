//! Chat compaction (Phase 8.6)
//!
//! Compacts old chat messages into summaries for context folding.
//! Original messages are NEVER deleted — compaction is additive only.

mod graph;
mod logic;

// Re-export all public types and functions
pub use graph::{create_compacted_edge, load_compacted_chat};
pub use logic::{compact_session, mark_messages_compacted, should_compact_session};

use crate::execution_tools::ExecutionDb;

/// Compaction trigger conditions
#[derive(Debug, Clone)]
pub struct CompactionTrigger {
    /// Minimum message count to trigger compaction
    pub min_messages: usize,
    /// Minimum total token count (estimate)
    pub min_tokens: usize,
}

impl Default for CompactionTrigger {
    fn default() -> Self {
        Self {
            min_messages: 50,
            min_tokens: 4000,
        }
    }
}

/// Compaction result
#[derive(Debug, Clone)]
pub struct CompactionResult {
    /// Messages that were compacted
    pub compacted_message_ids: Vec<i64>,
    /// Summary message ID
    pub summary_id: i64,
    /// Session ID
    pub session_id: String,
}

// ExecutionDb impl blocks to preserve original API
impl ExecutionDb {
    /// Check if session should be compacted
    pub fn should_compact_session(
        &self,
        session_id: &str,
        trigger: &CompactionTrigger,
    ) -> anyhow::Result<bool> {
        should_compact_session(self, session_id, trigger)
    }

    /// Compact chat session (placeholder for LLM summarization)
    pub fn compact_session(
        &self,
        session_id: &str,
        num_messages: usize,
    ) -> anyhow::Result<CompactionResult> {
        compact_session(self, session_id, num_messages)
    }

    /// Mark a range of messages as compacted to a summary
    pub fn mark_messages_compacted(
        &self,
        session_id: &str,
        message_ids: &[i64],
        summary_id: i64,
    ) -> anyhow::Result<()> {
        mark_messages_compacted(self, session_id, message_ids, summary_id)
    }

    /// Load compacted view of chat
    pub fn load_compacted_chat(
        &self,
        session_id: &str,
        recent_count: usize,
    ) -> anyhow::Result<Vec<String>> {
        load_compacted_chat(self, session_id, recent_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_db() -> (TempDir, ExecutionDb) {
        let temp_dir = TempDir::new().unwrap();
        let db_root = temp_dir.path();

        // Create codegraph.db with required schema
        let codegraph_path = db_root.join("codegraph.db");
        let conn = rusqlite::Connection::open(&codegraph_path).unwrap();

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

        conn.execute(
            "CREATE TABLE graph_edges (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                from_id INTEGER NOT NULL,
                to_id INTEGER NOT NULL,
                edge_type TEXT NOT NULL,
                data TEXT
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

        let exec_db = ExecutionDb::open(db_root).unwrap();
        exec_db.init_chat_schema().unwrap();

        (temp_dir, exec_db)
    }

    #[test]
    fn test_should_compact_session_false() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-compact-1";
        exec_db.create_chat_session(session_id).unwrap();

        // Only 1 message, should not compact
        assert!(!exec_db
            .should_compact_session(session_id, &CompactionTrigger::default())
            .unwrap());
    }

    #[test]
    fn test_should_compact_session_true() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-compact-2";
        exec_db.create_chat_session(session_id).unwrap();

        // Add 51 messages (over threshold)
        for i in 0..51 {
            exec_db
                .persist_user_message(session_id, &format!("msg {}", i))
                .unwrap();
        }

        // Should compact now
        assert!(exec_db
            .should_compact_session(session_id, &CompactionTrigger::default())
            .unwrap());
    }

    #[test]
    fn test_compact_returns_error_without_llm() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-compact-3";
        let result = exec_db.compact_session(session_id, 10);

        // Should fail without LLM
        assert!(result.is_err());
    }

    #[test]
    fn test_load_compacted_chat() {
        let (_temp, exec_db) = create_test_db();

        let session_id = "test-compact-load";
        exec_db.create_chat_session(session_id).unwrap();

        exec_db.persist_user_message(session_id, "msg 1").unwrap();
        exec_db
            .persist_assistant_message(session_id, "resp 1")
            .unwrap();
        exec_db.persist_user_message(session_id, "msg 2").unwrap();

        let messages = exec_db.load_compacted_chat(session_id, 10).unwrap();

        // Should have all 3 messages (order may vary if timestamps are same)
        assert_eq!(messages.len(), 3);
        assert!(messages.contains(&"msg 1".to_string()));
        assert!(messages.contains(&"resp 1".to_string()));
        assert!(messages.contains(&"msg 2".to_string()));
    }

    #[test]
    fn test_compaction_trigger_default() {
        let trigger = CompactionTrigger::default();
        assert_eq!(trigger.min_messages, 50);
        assert_eq!(trigger.min_tokens, 4000);
    }
}
