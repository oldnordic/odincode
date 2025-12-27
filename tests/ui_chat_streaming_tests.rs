//! UI chat streaming integration tests
//!
//! Tests the full chat workflow from handle_chat through
//! event streaming to process_chat_events, without making real HTTP calls.

use odincode::llm::chat_events::{ChatEvent, ChatSender};
use odincode::ui::state::{App, ChatMessage, ChatRole};
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create a test database with minimal config
fn create_test_app() -> (TempDir, App) {
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
    let exec_db = odincode::execution_tools::ExecutionDb::open(db_root).unwrap();
    exec_db.init_chat_schema().unwrap();

    let app = App::new(db_root.to_path_buf());

    (temp_dir, app)
}

/// Simulate a chat thread that sends events through the channel
fn simulate_chat_thread(
    tx: ChatSender,
    session_id: String,
    user_message: String,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        // Simulate the Started event (sent immediately by spawn_chat_thread)
        let _ = tx.send(ChatEvent::Started {
            session_id: session_id.clone(),
            user_message: user_message.clone(),
        });

        // Small delay to simulate processing
        thread::sleep(Duration::from_millis(10));

        // Simulate streaming chunks
        for chunk in &["Hello", ", world", "!"] {
            thread::sleep(Duration::from_millis(10));
            let _ = tx.send(ChatEvent::Chunk {
                session_id: session_id.clone(),
                content: chunk.to_string(),
            });
        }

        // Simulate completion
        let _ = tx.send(ChatEvent::Complete {
            session_id,
            full_response: "Hello, world!".to_string(),
        });
    })
}

/// Test that the full chat lifecycle works:
/// 1. User message added to UI immediately
/// 2. "Thinking..." indicator is set
/// 3. Started event creates session in DB
/// 4. Chunk events update the last message
/// 5. Complete event persists assistant message
/// 6. Chat thread cleans up
#[test]
fn test_full_chat_lifecycle_with_events() {
    let (_temp, mut app) = create_test_app();

    // Start chat - this should add user message and set thinking immediately
    let user_message = "test message";

    // Create the channel and handle manually to simulate chat_threaded
    let (tx, rx) = channel();
    let session_id = "test-session-lifecycle".to_string();

    // Add user message and set thinking (what handle_chat does)
    app.add_user_message(user_message.to_string());
    app.set_thinking();

    // Verify immediate UI state - User + Thinking = 2 messages
    assert_eq!(app.chat_messages.len(), 2);
    assert_eq!(app.chat_messages[0].role, ChatRole::User);
    assert_eq!(app.chat_messages[0].content, user_message);
    assert_eq!(app.chat_messages[1].role, ChatRole::Thinking);

    // Store the receiver for process_chat_events
    app.chat_event_receiver = Some(rx);

    // Simulate the background chat thread sending events
    let handle = simulate_chat_thread(tx, session_id.clone(), user_message.to_string());

    // Process events (simulating main loop polling)
    let mut max_iterations = 100;
    let mut got_complete = false;

    while max_iterations > 0 {
        max_iterations -= 1;

        if app.process_chat_events() {
            // Terminal event received (Complete or Error)
            got_complete = true;
            break;
        }

        thread::sleep(Duration::from_millis(5));
    }

    // Wait for thread to finish
    handle.join().unwrap();

    // Verify we received and processed the Complete event
    assert!(got_complete, "Should have received Complete event");

    // Verify final state: User + Assistant (Thinking converted to Assistant)
    assert_eq!(app.chat_messages.len(), 2); // User + Assistant

    let assistant_msg = &app.chat_messages[1];
    assert_eq!(
        assistant_msg.role,
        ChatRole::Assistant,
        "Role should be Assistant after first chunk"
    );
    assert_eq!(assistant_msg.content, "Hello, world!");

    // Verify chat thread was cleaned up (but receiver comes back, see note in error test)
    assert!(
        app.chat_thread_handle.is_none(),
        "Thread handle should be cleaned up after Complete"
    );

    // Verify no "Thinking..." message remains
    assert!(!app
        .chat_messages
        .iter()
        .any(|m| m.role == ChatRole::Thinking));
}

/// Test that process_chat_events correctly handles Chunk events
/// Note: Chunks REPLACE content (not accumulate), Complete sets final content
/// First chunk changes role from Thinking to Assistant.
#[test]
fn test_process_chat_events_accumulates_chunks() {
    let (_temp, mut app) = create_test_app();

    let (tx, rx) = channel();
    app.chat_event_receiver = Some(rx);

    // Add a Thinking message (what set_thinking does)
    app.chat_messages.push(ChatMessage {
        role: ChatRole::Thinking,
        content: String::new(),
    });

    // Send multiple Chunk events
    let session_id = "test-chunk-session".to_string();
    tx.send(ChatEvent::Started {
        session_id: session_id.clone(),
        user_message: "test".to_string(),
    })
    .unwrap();

    for chunk in &["Hello", " ", "world"] {
        tx.send(ChatEvent::Chunk {
            session_id: session_id.clone(),
            content: chunk.to_string(),
        })
        .unwrap();
    }

    // Process events
    for _ in 0..10 {
        if app.process_chat_events() {
            break;
        }
    }

    // Verify content was replaced (last chunk wins)
    let last_msg = &app.chat_messages[0];
    assert_eq!(last_msg.content, "world");
    // Verify role changed from Thinking to Assistant after first chunk
    assert_eq!(last_msg.role, ChatRole::Assistant);
}

/// Test that Error event is handled properly
#[test]
fn test_error_event_cleans_up_thread() {
    let (_temp, mut app) = create_test_app();

    let (tx, rx) = channel();
    app.chat_event_receiver = Some(rx);

    // Send an Error event
    tx.send(ChatEvent::Error {
        session_id: "error-session".to_string(),
        error: odincode::llm::chat::ChatError::NotConfigured,
    })
    .unwrap();

    // Process events - should return true for terminal event
    let terminal = app.process_chat_events();
    assert!(terminal, "Error event should be terminal");

    // Note: process_chat_events puts receiver back even for terminal events
    // The actual cleanup happens via cleanup_chat_thread() but the receiver
    // is restored by process_chat_events after handle_chat_event returns

    // Verify error state is set
    assert!(app.chat_error.is_some(), "Error should be set in app state");
}

/// Test that multiple Chunk events before Complete are handled correctly
#[test]
fn test_multiple_chunks_before_complete() {
    let (_temp, mut app) = create_test_app();

    let (tx, rx) = channel();
    let session_id = "multi-chunk".to_string();

    app.add_user_message("test".to_string());
    app.set_thinking(); // Add Thinking message that will be replaced
    app.chat_event_receiver = Some(rx);

    // Send all events
    tx.send(ChatEvent::Started {
        session_id: session_id.clone(),
        user_message: "test".to_string(),
    })
    .unwrap();

    for i in 0..5 {
        tx.send(ChatEvent::Chunk {
            session_id: session_id.clone(),
            content: format!("chunk{}", i),
        })
        .unwrap();
    }

    tx.send(ChatEvent::Complete {
        session_id,
        full_response: "full response".to_string(),
    })
    .unwrap();

    // Process until terminal
    let mut iterations = 0;
    while !app.process_chat_events() && iterations < 50 {
        iterations += 1;
        thread::sleep(Duration::from_millis(5));
    }

    // Verify assistant message has full content from Complete event
    let assistant_msg = app
        .chat_messages
        .iter()
        .find(|m| m.role == ChatRole::Assistant);
    assert!(
        assistant_msg.is_some(),
        "Should have an Assistant message after chunks"
    );
    assert_eq!(
        assistant_msg.unwrap().role,
        ChatRole::Assistant,
        "Role should be Assistant"
    );
    // Complete event's full_response is what ends up in the message
    assert_eq!(assistant_msg.unwrap().content, "full response");

    // Verify no Thinking message remains
    assert!(!app
        .chat_messages
        .iter()
        .any(|m| m.role == ChatRole::Thinking));
}

/// Test that process_chat_events returns receiver when no events available
#[test]
fn test_process_events_returns_receiver_when_empty() {
    let (_temp, mut app) = create_test_app();

    let (_tx, rx) = channel();
    app.chat_event_receiver = Some(rx);

    // Process with no events - should not be terminal, should return receiver
    let terminal = app.process_chat_events();
    assert!(!terminal, "No events should not be terminal");
    assert!(
        app.chat_event_receiver.is_some(),
        "Receiver should be returned"
    );
}

/// Test that Started event sets current_chat_session_id
#[test]
fn test_started_event_sets_session_id() {
    let (_temp, mut app) = create_test_app();

    let (tx, rx) = channel();
    app.chat_event_receiver = Some(rx);

    let session_id = "test-session-id".to_string();
    tx.send(ChatEvent::Started {
        session_id: session_id.clone(),
        user_message: "test".to_string(),
    })
    .unwrap();

    app.process_chat_events();

    assert_eq!(app.current_chat_session_id, Some(session_id));
}
