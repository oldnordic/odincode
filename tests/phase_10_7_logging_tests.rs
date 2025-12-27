//! Phase 10.7: Discovery Event Logging tests
//!
//! TDD approach: Tests for logging tool discovery events to execution memory.

use std::path::PathBuf;
use std::fs::File;
use tempfile::TempDir;

use odincode::execution_tools::ExecutionDb;
use odincode::llm::discovery::{discover_tools_for_chat, ToolDiscoveryContext};

// Helper to set up test database
fn setup_test_db() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_root = temp_dir.path().to_path_buf();

    // Create codegraph.db first (required by ExecutionDb)
    let codegraph_path = db_root.join("codegraph.db");
    File::create(&codegraph_path).expect("Failed to create codegraph.db");

    (temp_dir, db_root)
}

// ===== Schema Tests =====

#[test]
fn test_discovery_events_table_exists() {
    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    // Check that discovery_events table exists
    let query = "SELECT name FROM sqlite_master WHERE type='table' AND name='discovery_events'";
    let result: Vec<String> = db.conn()
        .prepare(query)
        .expect("Failed to prepare query")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .collect::<Result<_, _>>()
        .expect("Failed to collect");

    assert!(result.contains(&"discovery_events".to_string()), "discovery_events table should exist");
}

#[test]
fn test_discovery_events_has_correct_columns() {
    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    // Get table info
    let query = "PRAGMA table_info(discovery_events)";
    let mut stmt = db.conn().prepare(query).expect("Failed to prepare");

    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .expect("Failed to query")
        .collect::<Result<_, _>>()
        .expect("Failed to collect");

    assert!(columns.contains(&"id".to_string()));
    assert!(columns.contains(&"session_id".to_string()));
    assert!(columns.contains(&"user_query_hash".to_string()));
    assert!(columns.contains(&"tools_discovered".to_string()));
    assert!(columns.contains(&"reason".to_string()));
    assert!(columns.contains(&"timestamp".to_string()));
}

// ===== Logging Tests =====

#[test]
fn test_log_discovery_event() {
    use odincode::execution_tools::log_discovery_event;

    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    let context = ToolDiscoveryContext::new("write a file");
    let tools = discover_tools_for_chat(&context);

    log_discovery_event(&db, "test_session", &context, &tools, "keyword: write")
        .expect("Failed to log discovery event");

    // Verify the event was logged
    let query = "SELECT * FROM discovery_events";
    let mut stmt = db.conn().prepare(query).expect("Failed to prepare");

    let events: Vec<(String, String, String)> = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>("session_id")?,
                row.get::<_, String>("user_query_hash")?,
                row.get::<_, String>("tools_discovered")?,
            ))
        })
        .expect("Failed to query")
        .collect::<Result<_, _>>()
        .expect("Failed to collect");

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].0, "test_session");
    assert!(events[0].2.contains("file_write"));
}

#[test]
fn test_log_discovery_event_includes_reason() {
    use odincode::execution_tools::log_discovery_event;

    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    let context = ToolDiscoveryContext::new("check git status");
    let tools = discover_tools_for_chat(&context);

    log_discovery_event(&db, "session_123", &context, &tools, "keyword: git")
        .expect("Failed to log discovery event");

    // Check reason field
    let query = "SELECT reason FROM discovery_events";
    let reason: String = db.conn()
        .prepare(query)
        .expect("Failed to prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .next()
        .expect("No events found")
        .expect("Failed to get row");

    assert_eq!(reason, "keyword: git");
}

#[test]
fn test_log_discovery_event_captures_query_hash() {
    use odincode::execution_tools::log_discovery_event;

    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    let context = ToolDiscoveryContext::new("help me write code");
    let tools = discover_tools_for_chat(&context);

    log_discovery_event(&db, "session_abc", &context, &tools, "keyword: write")
        .expect("Failed to log discovery event");

    // Get the query hash
    let query = "SELECT user_query_hash FROM discovery_events";
    let hash: String = db.conn()
        .prepare(query)
        .expect("Failed to prepare")
        .query_map([], |row| row.get::<_, String>(0))
        .expect("Failed to query")
        .next()
        .expect("No events found")
        .expect("Failed to get row");

    // Hash should be deterministic
    assert!(!hash.is_empty());
    assert!(hash.len() <= 64); // SHA-256 is 64 hex chars
}

#[test]
fn test_log_discovery_event_with_empty_discovery() {
    use odincode::execution_tools::log_discovery_event;

    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    let context = ToolDiscoveryContext::new("hello world"); // No specialized tools
    let tools = discover_tools_for_chat(&context);

    log_discovery_event(&db, "session_xyz", &context, &tools, "no triggers matched")
        .expect("Failed to log discovery event");

    // Should still log even with no specialized tools discovered
    let query = "SELECT COUNT(*) FROM discovery_events";
    let count: i64 = db.conn()
        .prepare(query)
        .expect("Failed to prepare")
        .query_map([], |row| row.get::<_, i64>(0))
        .expect("Failed to query")
        .next()
        .expect("No count")
        .expect("Failed to get row");

    assert_eq!(count, 1);
}

// ===== Query Tests =====

#[test]
fn test_query_discovery_events_by_session() {
    use odincode::execution_tools::{log_discovery_event, query_discovery_events};

    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    // Log multiple events for the same session
    for query in &["write file", "read file", "git status"] {
        let context = ToolDiscoveryContext::new(*query);
        let tools = discover_tools_for_chat(&context);
        log_discovery_event(&db, "session_multi", &context, &tools, "test")
            .expect("Failed to log");
    }

    // Query events for session
    let events = query_discovery_events(&db, "session_multi")
        .expect("Failed to query");

    assert_eq!(events.len(), 3);
}

#[test]
fn test_query_discovery_events_returns_sorted() {
    use odincode::execution_tools::{log_discovery_event, query_discovery_events};

    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    // Log events with slight delays to ensure different timestamps
    let contexts = [
        ToolDiscoveryContext::new("first"),
        ToolDiscoveryContext::new("second"),
        ToolDiscoveryContext::new("third"),
    ];

    for (i, context) in contexts.iter().enumerate() {
        let tools = discover_tools_for_chat(context);
        log_discovery_event(&db, "session_sorted", context, &tools, &format!("event_{}", i))
            .expect("Failed to log");
        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let events = query_discovery_events(&db, "session_sorted")
        .expect("Failed to query");

    // Should be sorted by timestamp (ascending)
    for i in 0..events.len() - 1 {
        assert!(events[i].timestamp <= events[i + 1].timestamp, "Events should be sorted by timestamp");
    }
}
