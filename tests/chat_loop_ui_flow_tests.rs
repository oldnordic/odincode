//! Chat loop UI flow integration tests
//!
//! Tests the complete flow from UI through chat loop to find where events are lost.

use std::fs::File;
use std::io::Write;
use std::sync::mpsc::channel;
use std::time::Duration;
use tempfile::TempDir;

use odincode::execution_engine::ChatToolRunner;
use odincode::llm::chat_events::{ChatEvent, ChatSender};
use odincode::llm::chat_loop::ChatLoop;
use odincode::ui::state::App;

/// Helper to create a test database with minimal config
/// Returns (temp_dir, app, tx, db_root) for use in tests
fn create_test_app_with_channel(user_message: String) -> (TempDir, App, ChatSender, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let db_root = temp_dir.path().to_path_buf();

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

    // Create minimal config with stub provider
    let config_path = db_root.join("config.toml");
    let mut config_file = File::create(&config_path).unwrap();
    writeln!(
        config_file,
        r#"[llm]
mode = "external"
provider = "stub"
model = "test"
"#
    )
    .unwrap();

    // Initialize execution DB with chat schema
    let exec_db = odincode::execution_tools::ExecutionDb::open(&db_root).unwrap();
    exec_db.init_chat_schema().unwrap();

    // Create app with mpsc channel (following production pattern from handlers.rs:80-105)
    let (tx, rx) = channel();
    let mut app = App::new(db_root.clone());

    // Add user message to chat transcript
    app.add_user_message(user_message.clone());

    // Show "Thinking..." immediately
    app.set_thinking();

    // Set up chat loop (following production pattern)
    let magellan_db = odincode::magellan_tools::MagellanDb::open_readonly(
        db_root.join("codegraph.db")
    ).ok();
    let exec_db = odincode::execution_tools::ExecutionDb::open(&db_root).ok();
    let tool_runner = ChatToolRunner::new(magellan_db, exec_db);

    let mut chat_loop = ChatLoop::new(tool_runner);
    chat_loop.set_sender(tx.clone());

    // Start the loop (spawns initial chat thread) - BEFORE setting on app
    // This is the critical part that was missing!
    match chat_loop.start(user_message, &db_root) {
        Ok(()) => {
            // Store loop, receiver, and sender in app state
            app.set_chat_loop(chat_loop);
            app.chat_event_receiver = Some(rx);
            app.chat_event_sender = Some(tx.clone());
        }
        Err(e) => {
            panic!("Failed to start chat loop in test: {}", e);
        }
    }

    (temp_dir, app, tx, db_root)
}

/// Test that process_chat_events can handle events from spawned thread
#[test]
fn test_process_chat_events_handles_spawned_thread_events() {
    let user_message = "please read src/lib.rs".to_string();
    let (_temp, mut app, tx, _db_root) = create_test_app_with_channel(user_message.clone());

    // The ChatLoop was already started, let's get the session_id
    let loop_session_id = app.chat_loop_mut()
        .and_then(|loop_state| loop_state.state())
        .map(|state| state.session_id.clone())
        .unwrap();

    eprintln!("[TEST] ChatLoop session_id: {:?}", loop_session_id);

    // Send Complete event with tool call using the SAME session_id
    let complete_event = ChatEvent::Complete {
        session_id: loop_session_id.clone(),
        full_response: r#"I'll read that.

TOOL_CALL:
  tool: file_read
  args:
    path: src/lib.rs"#
        .to_string(),
    };

    let _ = tx.send(complete_event);

    // Process events - this should trigger tool execution
    eprintln!("[TEST] Processing Complete event");
    let terminal = app.process_chat_events();
    eprintln!("[TEST] process_chat_events returned: {}", terminal);

    // After tool execution, the spawned thread should send events
    // Let's process more events to catch them
    for i in 0..20 {
        std::thread::sleep(Duration::from_millis(100));
        let result = app.process_chat_events();
        eprintln!("[TEST] Iteration {}: process_chat_events returned: {}", i, result);
        if result {
            eprintln!("[TEST] Terminal event detected");
            break;
        }
    }

    // Check chat messages
    eprintln!(
        "[TEST] Chat messages count: {}",
        app.chat_messages.len()
    );
    for (i, msg) in app.chat_messages.iter().enumerate() {
        eprintln!(
            "[TEST] Message {}: role={:?}, content={}",
            i,
            msg.role,
            msg.content.chars().take(50).collect::<String>()
        );
    }
}

/// Test the complete flow: tool execution spawns thread, events are received
#[test]
fn test_tool_execution_flow() {
    let user_message = "please explain src/lib.rs".to_string();
    let (_temp, mut app, tx, _db_root) = create_test_app_with_channel(user_message.clone());

    // The ChatLoop was already started, let's get the session_id
    let loop_session_id = app.chat_loop_mut()
        .and_then(|loop_state| loop_state.state())
        .map(|state| state.session_id.clone())
        .unwrap();

    eprintln!("[TEST] ChatLoop session_id: {:?}", loop_session_id);

    // Send Complete event with tool call using the SAME session_id
    let complete_event = ChatEvent::Complete {
        session_id: loop_session_id.clone(),
        full_response: r#"I'll read that file.

TOOL_CALL:
  tool: file_read
  args:
    path: src/lib.rs"#
            .to_string(),
    };

    let _ = tx.send(complete_event);

    // Process the event (should trigger tool execution and spawn new thread)
    eprintln!("[TEST] Processing Complete event with tool call");
    app.process_chat_events();

    // Now keep processing to catch events from spawned thread
    eprintln!("[TEST] Processing events from spawned thread...");
    for i in 0..30 {
        std::thread::sleep(Duration::from_millis(100));
        let result = app.process_chat_events();
        eprintln!("[TEST] Iteration {}: process_chat_events returned: {}", i, result);
        if result {
            eprintln!("[TEST] Terminal event detected at iteration {}", i);
            break;
        }
    }

    // Check final state
    eprintln!(
        "[TEST] Final chat messages count: {}",
        app.chat_messages.len()
    );
    for (i, msg) in app.chat_messages.iter().enumerate() {
        eprintln!(
            "[TEST] Message {}: role={:?}, content={}",
            i,
            msg.role,
            msg.content.chars().take(80).collect::<String>()
        );
    }

    // We should have at least 2 messages: original user + the spawned thread's response
    // (The original Thinking message should have been converted to Assistant)
    assert!(
        app.chat_messages.len() >= 2,
        "Expected at least 2 messages, got {}",
        app.chat_messages.len()
    );
}
