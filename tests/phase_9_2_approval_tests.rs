//! Phase 9.2 — Gated Tools & Approvals: Integration Tests (TDD)
//!
//! Tests for session-scoped GATED tool approvals in multi-step chat loop.

use std::collections::HashMap;
use std::fs::File;

use odincode::execution_engine::{ChatToolCategory, ChatToolRunner};
use odincode::execution_tools::ExecutionDb;
use odincode::llm::chat_loop::{LoopAction, LoopState, PendingGatedTool};
use odincode::ui::approval::{ApprovalScope, ApprovalState, PendingApproval};

// === TEST UTILITIES ===

/// Create temporary directory with codegraph.db
fn setup_temp_dir() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create codegraph.db (required by ExecutionDb::open)
    let graph_db_path = temp_dir.path().join("codegraph.db");
    File::create(&graph_db_path).expect("Failed to create codegraph.db");

    temp_dir
}

/// Create ExecutionDb for testing
fn setup_exec_db(temp_dir: &tempfile::TempDir) -> ExecutionDb {
    ExecutionDb::open(temp_dir.path()).expect("Failed to open ExecutionDb")
}

// === TEST 1: GATED tool prompts for approval ===

#[test]
fn test_1_gated_tool_prompts_once() {
    use odincode::llm::chat_loop::ChatLoop;

    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);

    // Create tool runner
    let runner = ChatToolRunner::new(None, Some(exec_db));

    // Verify file_write is classified as GATED
    assert_eq!(runner.classify_tool("file_write"), ChatToolCategory::Gated);
    assert!(runner.is_gated_tool("file_write"));
    assert!(!runner.is_auto_tool("file_write"));

    // Create ChatLoop with an active loop state
    let mut loop_driver = ChatLoop::new(runner);

    // Manually set up active loop state (simulating a started chat loop)
    let loop_state = LoopState::new("test-session".to_string(), "test message".to_string());
    loop_driver.loop_state = Some(loop_state);

    // Process Complete event with GATED tool call
    // Format: TOOL_CALL: <tool: name> <args: key: value>
    let action = loop_driver.process_event(
        &odincode::llm::chat_events::ChatEvent::Complete {
            session_id: "test-session".to_string(),
            full_response:
                "TOOL_CALL:\n  tool: file_write\n  args:\n    path: test.txt\n    content: hello"
                    .to_string(),
        },
        temp_dir.path(),
    );

    // Verify RequestApproval action returned
    match action {
        LoopAction::RequestApproval(tool, _) => {
            assert_eq!(tool, "file_write");
        }
        _ => {
            panic!("Expected RequestApproval, got {:?}", action);
        }
    }

    // Also verify loop is now paused with pending tool
    assert!(loop_driver.loop_state.as_ref().unwrap().is_paused());
    let pending = loop_driver
        .loop_state
        .as_ref()
        .unwrap()
        .pending_tool()
        .unwrap();
    assert_eq!(pending.tool, "file_write");
}

// === TEST 2: Session-scoped approval allows multiple writes ===

#[test]
fn test_2_approve_all_allows_multiple_writes() {
    let mut approval_state = ApprovalState::new();

    // Initially, no tools approved
    assert!(!approval_state.is_approved("file_write"));
    assert!(!approval_state.is_approved("file_create"));

    // Grant session-scoped approval
    approval_state.grant(ApprovalScope::SessionAllGated);

    // Now all GATED tools are approved
    assert!(approval_state.is_approved("file_write"));
    assert!(approval_state.is_approved("file_create"));

    // Verify SessionAllGated scope covers all tools
    let scope = ApprovalScope::SessionAllGated;
    assert!(scope.covers("file_write"));
    assert!(scope.covers("file_create"));
}

// === TEST 3: Single-tool approval scope ===

#[test]
fn test_3_single_tool_approval_scope() {
    let mut approval_state = ApprovalState::new();

    // Grant single-tool approval for file_write only
    approval_state.grant(ApprovalScope::Once {
        tool: "file_write".to_string(),
    });

    // file_write is approved
    assert!(approval_state.is_approved("file_write"));

    // file_create is NOT approved (different tool)
    assert!(!approval_state.is_approved("file_create"));

    // Verify Once scope only covers its specific tool
    let scope = ApprovalScope::Once {
        tool: "file_write".to_string(),
    };
    assert!(scope.covers("file_write"));
    assert!(!scope.covers("file_create"));
}

// === TEST 4: LoopState pause/resume behavior ===

#[test]
fn test_4_loopstate_pause_resume() {
    // Create loop state
    let mut loop_state = LoopState::new("test".to_string(), "hello".to_string());

    // Initially not paused
    assert!(!loop_state.is_paused());
    assert!(loop_state.active);

    // Create pending GATED tool
    let mut args = HashMap::new();
    args.insert("path".to_string(), "test.txt".to_string());
    args.insert("content".to_string(), "test".to_string());

    let pending = PendingGatedTool {
        tool: "file_write".to_string(),
        args: args.clone(),
        step: 1,
    };

    // Pause the loop
    loop_state.pause(pending.clone());

    // Verify paused state
    assert!(loop_state.is_paused());
    assert!(loop_state.pending_tool().is_some());
    assert_eq!(loop_state.pending_tool().unwrap().tool, "file_write");

    // Resume the loop
    loop_state.resume();

    // Verify resumed state
    assert!(!loop_state.is_paused());
    assert!(loop_state.pending_tool().is_none());
}

// === TEST 5: ChatLoop handle_denial returns ToolDenied ===

#[test]
fn test_5_chatloop_handle_denial() {
    use odincode::llm::chat_loop::ChatLoop;
    use std::sync::mpsc::channel;

    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);
    let runner = ChatToolRunner::new(None, Some(exec_db));

    // Create ChatLoop
    let mut chat_loop = ChatLoop::new(runner);

    // Set up a channel for ChatEvents (required by handle_denial)
    let (tx, _rx) = channel::<odincode::llm::chat_events::ChatEvent>();
    chat_loop.set_sender(tx);

    // Create a loop state directly and pause it
    let mut args = HashMap::new();
    args.insert("path".to_string(), "test.txt".to_string());
    args.insert("content".to_string(), "test".to_string());

    let pending = PendingGatedTool {
        tool: "file_write".to_string(),
        args: args.clone(),
        step: 1,
    };

    // Manually set up the paused loop state
    let mut loop_state = LoopState::new("test-session".to_string(), "hello".to_string());
    loop_state.pause(pending);
    chat_loop.loop_state = Some(loop_state);

    // Handle denial via ChatLoop
    let result = chat_loop.handle_denial(temp_dir.path());

    // Verify ToolDenied action returned
    assert!(result.is_ok());
    match result.unwrap() {
        LoopAction::ToolDenied => {
            // Expected - denial was processed
        }
        other => {
            panic!("Expected ToolDenied, got {:?}", other);
        }
    }

    // Verify loop is resumed (no longer paused)
    assert!(!chat_loop.loop_state.as_ref().unwrap().is_paused());
}

// === TEST 6: Approval state cleared on new session ===

#[test]
fn test_6_approval_cleared_on_new_session() {
    let mut approval_state = ApprovalState::new();

    // Grant some approvals in first session
    approval_state.grant(ApprovalScope::Once {
        tool: "file_write".to_string(),
    });

    assert!(approval_state.is_approved("file_write"));

    // Set a pending approval
    let pending = PendingApproval::new(
        "session-1".to_string(),
        "file_create".to_string(),
        HashMap::new(),
        1,
        Some("new_file.txt".to_string()),
    );
    approval_state.set_pending(pending);

    // Verify state has pending and approved tools
    assert!(approval_state.pending.is_some());
    assert!(approval_state.approved_once.contains("file_write"));

    // Reset state (simulating new chat session)
    approval_state.reset();

    // Verify all state cleared
    assert!(!approval_state.approved_all_gated);
    assert!(approval_state.approved_once.is_empty());
    assert!(approval_state.pending.is_none());
    assert!(!approval_state.is_approved("file_write"));
}

// === TEST 7: GATED tool logged with scope ===

#[test]
fn test_7_gated_tool_logged_with_scope() {
    let temp_dir = setup_temp_dir();
    let exec_db = setup_exec_db(&temp_dir);

    // Record approval granted
    let session_id = "test-logging-session";
    let tool = "file_write";
    let scope = "SessionAllGated"; // Using string representation

    let mut args = HashMap::new();
    args.insert("path".to_string(), "test.txt".to_string());

    // Convert to serde_json::Value
    let args_value = serde_json::to_value(&args).unwrap();

    // Use the ExecutionDb method to record approval
    let result = exec_db.record_approval_granted(session_id, tool, scope, &args_value);

    assert!(result.is_ok(), "Approval recording should succeed");

    // Query execution_log.db to verify record
    let conn = exec_db.conn();

    // Check execution exists
    let exec_id = format!("approval_granted_{}_{}", session_id, tool);
    let tool_name: String = conn
        .query_row(
            "SELECT tool_name FROM executions WHERE id = ?1",
            [&exec_id],
            |row| row.get(0),
        )
        .expect("Should find approval execution");

    assert_eq!(tool_name, "approval_granted");

    // Check artifact exists with scope data
    let artifact_json: String = conn
        .query_row(
            "SELECT content_json FROM execution_artifacts
             WHERE execution_id = ?1 AND artifact_type = 'approval_granted'",
            [&exec_id],
            |row| row.get(0),
        )
        .expect("Should find approval artifact");

    // Verify artifact contains scope info
    assert!(artifact_json.contains("SessionAllGated"));
    assert!(artifact_json.contains(session_id));
    assert!(artifact_json.contains(tool));

    // Record denial and verify
    let deny_result =
        exec_db.record_approval_denied(session_id, "file_create", &args_value, "User denied");

    assert!(deny_result.is_ok());

    // Check denial artifact
    let deny_exec_id = format!("approval_denied_{}_{}", session_id, "file_create");
    let deny_artifact: String = conn
        .query_row(
            "SELECT content_json FROM execution_artifacts
             WHERE execution_id = ?1 AND artifact_type = 'approval_denied'",
            [&deny_exec_id],
            |row| row.get(0),
        )
        .expect("Should find denial artifact");

    assert!(deny_artifact.contains("User denied"));
}

// === TEST 8: /quit during approval exits immediately ===

#[test]
fn test_8_quit_during_approval_exits_immediately() {
    // Create pending approval
    let mut args = HashMap::new();
    args.insert("path".to_string(), "test.txt".to_string());
    args.insert("content".to_string(), "test".to_string());

    let pending = PendingGatedTool {
        tool: "file_write".to_string(),
        args: args.clone(),
        step: 1,
    };

    // Create loop state and pause
    let mut loop_state = LoopState::new("test".to_string(), "hello".to_string());
    loop_state.pause(pending);

    assert!(loop_state.is_paused());

    // End the loop (simulating /quit)
    loop_state.complete();

    // Verify loop state is cleared
    assert!(!loop_state.active);
}

// === TEST 9: PendingApproval formatting ===

#[test]
fn test_9_pending_approval_formatting() {
    let mut args = HashMap::new();
    args.insert("path".to_string(), "/path/to/file.txt".to_string());
    args.insert("content".to_string(), "hello world".to_string());

    let pending = PendingApproval::new(
        "session-123".to_string(),
        "file_write".to_string(),
        args,
        5,
        Some("/path/to/file.txt".to_string()),
    );

    // Format prompt for UI
    let prompt = pending.format_prompt();

    assert!(prompt.contains("GATED Tool"));
    assert!(prompt.contains("file_write"));
    assert!(prompt.contains("/path/to/file.txt"));
    assert!(prompt.contains("[y=once, a=session, n=deny, q=quit]"));
}

// === TEST 10: ApprovalResponse equality ===

#[test]
fn test_10_approval_response_equality() {
    use odincode::ui::ApprovalResponse;

    assert_eq!(
        ApprovalResponse::ApproveOnce("file_write".to_string()),
        ApprovalResponse::ApproveOnce("file_write".to_string())
    );
    assert_eq!(
        ApprovalResponse::ApproveSessionAllGated,
        ApprovalResponse::ApproveSessionAllGated
    );
    assert_eq!(
        ApprovalResponse::Deny("file_write".to_string()),
        ApprovalResponse::Deny("file_write".to_string())
    );
    assert_eq!(ApprovalResponse::Quit, ApprovalResponse::Quit);

    assert_ne!(
        ApprovalResponse::ApproveOnce("file_write".to_string()),
        ApprovalResponse::ApproveOnce("file_create".to_string())
    );
    assert_ne!(
        ApprovalResponse::ApproveSessionAllGated,
        ApprovalResponse::Quit
    );
}
