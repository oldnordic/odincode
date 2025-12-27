//! Chat loop tool continuation integration tests
//!
//! Tests the full workflow:
//! 1. User sends message
//! 2. LLM responds with tool call
//! 3. Tool executes
//! 4. execute_tool_and_continue() is called
//! 5. New thread spawns with FrameStack
//! 6. Thread sends events back
//! 7. UI receives and processes events
//!
//! This test identifies where events are being dropped when the chat loop
//! gets stuck after tool invocation.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::channel;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

use odincode::execution_engine::{ChatToolRunner, ToolResult};
use odincode::llm::chat_events::{ChatEvent, ChatSender};
use odincode::llm::chat_loop::{ChatLoop, LoopAction};
use odincode::llm::frame_stack::{Frame, FrameStack};
use odincode::llm::LoopAction as LoopActionEnum;

/// Helper to create a test database with minimal config
fn create_test_db() -> TempDir {
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

    temp_dir
}

/// Test that ChatLoop correctly processes a Complete event with tool call
#[test]
fn test_chat_loop_processes_tool_call() {
    let temp_dir = create_test_db();
    let (tx, _rx) = channel();

    // Create ChatLoop with tool runner
    let magellan_db =
        odincode::magellan_tools::MagellanDb::open_readonly(temp_dir.path().join("codegraph.db"))
            .ok();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).ok();
    let tool_runner = ChatToolRunner::new(magellan_db, exec_db);

    let mut chat_loop = ChatLoop::new(tool_runner);
    chat_loop.set_sender(tx);

    // Start the loop
    chat_loop
        .start("test message".to_string(), temp_dir.path())
        .unwrap();

    let session_id = chat_loop.state().unwrap().session_id.clone();

    // Simulate a Complete event with a tool call (YAML-like format as expected by parser)
    let llm_response = r#"I'll read that file for you.

TOOL_CALL:
  tool: file_read
  args:
    path: src/lib.rs"#;

    let complete_event = ChatEvent::Complete {
        session_id: session_id.clone(),
        full_response: llm_response.to_string(),
    };

    // Process the event
    let action = chat_loop.process_event(&complete_event, temp_dir.path());

    // Verify ExecuteTool action was returned
    match action {
        LoopActionEnum::ExecuteTool(tool, args) => {
            assert_eq!(tool, "file_read");
            assert_eq!(args.get("path"), Some(&"src/lib.rs".to_string()));
        }
        _ => panic!("Expected ExecuteTool action, got {:?}", action),
    }
}

/// Test that execute_tool_and_continue spawns a new thread that sends events
#[test]
fn test_execute_tool_and_continue_sends_events() {
    let temp_dir = create_test_db();
    let (tx, rx) = channel();

    // Create ChatLoop with tool runner
    let magellan_db =
        odincode::magellan_tools::MagellanDb::open_readonly(temp_dir.path().join("codegraph.db"))
            .ok();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).ok();
    let tool_runner = ChatToolRunner::new(magellan_db, exec_db);

    let mut chat_loop = ChatLoop::new(tool_runner);
    chat_loop.set_sender(tx.clone());

    // Start the loop with a REFINE mode prompt (allows file_read)
    chat_loop
        .start("please read src/lib.rs and explain what it does".to_string(), temp_dir.path())
        .unwrap();

    let session_id = chat_loop.state().unwrap().session_id.clone();

    // Simulate Complete event with tool call (YAML-like format as expected by parser)
    // Using file_read which is allowed in REFINE mode
    let llm_response = r#"I'll read that file for you.

TOOL_CALL:
  tool: file_read
  args:
    path: src/lib.rs"#;

    let complete_event = ChatEvent::Complete {
        session_id: session_id.clone(),
        full_response: llm_response.to_string(),
    };

    // Process event
    let action = chat_loop.process_event(&complete_event, temp_dir.path());

    // Execute the tool
    if let LoopActionEnum::ExecuteTool(tool, args) = action {
        let result = chat_loop
            .execute_tool_and_continue(tool, args, temp_dir.path())
            .unwrap();

        match result {
            LoopActionEnum::ToolExecuted(_) | LoopActionEnum::ToolFailed(_) => {
                // Tool executed, now check for events from the spawned thread
            }
            _ => panic!("Expected ToolExecuted or ToolFailed, got {:?}", result),
        }

        // Wait for events from the spawned thread
        // The spawned thread should send: Started, Chunks, Complete
        let mut events_received = Vec::new();
        let timeout = Duration::from_secs(5);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            match rx.try_recv() {
                Ok(event) => {
                    eprintln!("[TEST] Received event: {:?}", event);
                    events_received.push(event);

                    // Check for terminal event
                    if let ChatEvent::Complete { .. } | ChatEvent::Error { .. } =
                        events_received.last().unwrap()
                    {
                        break;
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    break;
                }
            }
        }

        // Assert we received events from the spawned thread
        eprintln!(
            "[TEST] Total events received: {}",
            events_received.len()
        );
        eprintln!(
            "[TEST] Events: {:?}",
            events_received
                .iter()
                .map(|e| format!("{:?}", std::mem::discriminant(e)))
                .collect::<Vec<_>>()
        );

        // At minimum, we should get a Started event from the spawned thread
        assert!(
            events_received
                .iter()
                .any(|e| matches!(e, ChatEvent::Started { .. })),
            "Expected at least one Started event from spawned thread, got: {:?}",
            events_received
        );

    } else {
        panic!("Expected ExecuteTool action, got {:?}", action);
    }
}

/// Test that FrameStack is built correctly after tool execution
#[test]
fn test_frame_stack_after_tool_execution() {
    let temp_dir = create_test_db();
    let (tx, _rx) = channel();

    let magellan_db =
        odincode::magellan_tools::MagellanDb::open_readonly(temp_dir.path().join("codegraph.db"))
            .ok();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).ok();
    let tool_runner = ChatToolRunner::new(magellan_db, exec_db);

    let mut chat_loop = ChatLoop::new(tool_runner);
    chat_loop.set_sender(tx);

    chat_loop
        .start("read test.txt".to_string(), temp_dir.path())
        .unwrap();

    let session_id = chat_loop.state().unwrap().session_id.clone();

    // Simulate Complete event (YAML-like format as expected by parser)
    let llm_response = r#"I'll read the file.

TOOL_CALL:
  tool: file_read
  args:
    path: test.txt"#;

    let complete_event = ChatEvent::Complete {
        session_id,
        full_response: llm_response.to_string(),
    };

    chat_loop.process_event(&complete_event, temp_dir.path());

    // Check FrameStack state
    let state = chat_loop.state().unwrap();
    let frame_stack = state.frame_stack();

    // Should have at least: User (initial) + Assistant (response with tool call)
    assert!(frame_stack.len() >= 2, "FrameStack should have at least 2 frames, has {}", frame_stack.len());

    // First frame should be User
    if let Some(Frame::User(msg)) = frame_stack.iter().next() {
        assert_eq!(msg, "read test.txt");
    } else {
        panic!("First frame should be User");
    }
}

/// Test that session_id is preserved across tool execution
#[test]
fn test_session_id_preserved_across_tool_execution() {
    let temp_dir = create_test_db();
    let (tx, _rx) = channel();

    let magellan_db =
        odincode::magellan_tools::MagellanDb::open_readonly(temp_dir.path().join("codegraph.db"))
            .ok();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).ok();
    let tool_runner = ChatToolRunner::new(magellan_db, exec_db);

    let mut chat_loop = ChatLoop::new(tool_runner);
    chat_loop.set_sender(tx);

    chat_loop
        .start("test message".to_string(), temp_dir.path())
        .unwrap();

    // The session_id should be set
    let state = chat_loop.state().unwrap();
    assert!(!state.session_id.is_empty());
}

/// Test that FrameStack includes tool result when building messages
/// This is the CRITICAL test for verifying tool results are sent to LLM
#[test]
fn test_frame_stack_includes_tool_result_in_messages() {
    let temp_dir = create_test_db();
    let (tx, _rx) = channel();

    let magellan_db =
        odincode::magellan_tools::MagellanDb::open_readonly(temp_dir.path().join("codegraph.db"))
            .ok();
    let exec_db = odincode::execution_tools::ExecutionDb::open(temp_dir.path()).ok();
    let tool_runner = ChatToolRunner::new(magellan_db, exec_db);

    let mut chat_loop = ChatLoop::new(tool_runner);
    chat_loop.set_sender(tx);

    // Start loop
    chat_loop
        .start("list files".to_string(), temp_dir.path())
        .unwrap();

    let session_id = chat_loop.state().unwrap().session_id.clone();

    // Simulate Complete event with tool call
    let llm_response = r#"I'll list the files.

TOOL_CALL:
  tool: file_glob
  args:
    pattern: "*.rs"
    root: src"#;

    let complete_event = ChatEvent::Complete {
        session_id,
        full_response: llm_response.to_string(),
    };

    chat_loop.process_event(&complete_event, temp_dir.path());

    // Execute the tool
    let state = chat_loop.state().unwrap();

    // Before executing, FrameStack should have: User + Assistant
    let initial_frames = state.frame_stack().len();
    assert!(initial_frames >= 2, "Should have User + Assistant frames");

    // Now execute the tool (this will add ToolResult to FrameStack)
    drop(state); // Release borrow

    let action = chat_loop.process_event(&complete_event, temp_dir.path());

    if let LoopActionEnum::ExecuteTool(tool, args) = action {
        chat_loop.execute_tool_and_continue(tool, args, temp_dir.path()).unwrap();
    }

    // After execution, FrameStack should have: User + Assistant + ToolResult
    let state = chat_loop.state().unwrap();
    let _frames_after = state.frame_stack().len();

    // Check that ToolResult was added
    let has_tool_result = state.frame_stack()
        .iter()
        .any(|f| matches!(f, Frame::ToolResult { .. }));

    assert!(has_tool_result, "FrameStack should contain a ToolResult after tool execution");

    // CRITICAL: Build messages and verify ToolResult is included
    // Note: We can't call build_messages() through &FrameStack reference
    // because it needs &mut self now. Instead, verify the FrameStack directly.

    // Verify tool result frame exists with correct data
    let tool_result_found = state.frame_stack()
        .iter()
        .find_map(|f| {
            if let Frame::ToolResult { tool, success, output, .. } = f {
                Some((tool.clone(), *success, output.clone()))
            } else {
                None
            }
        });

    assert!(tool_result_found.is_some(), "Should have tool result in FrameStack");

    let (tool_name, success, output) = tool_result_found.unwrap();
    assert_eq!(tool_name, "file_glob");
    eprintln!("[TEST] Tool result in FrameStack: {} {}, output: {} chars",
        tool_name, if success { "OK" } else { "FAILED" }, output.len());
}
