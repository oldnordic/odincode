//! Event processing and tool execution handlers
//!
//! # Event Handlers
//!
//! Contains the heavy logic for processing ChatEvents and executing tools.
//! These methods operate on ChatLoop but are separated for modularity.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::collections::HashMap;

use crate::execution_engine::{ChatToolCategory, ToolResult};
use crate::llm::chat_events::{ChatEvent, ChatSender};
use crate::llm::chat_thread::spawn_chat_thread_with_frame_stack;
use crate::llm::tool_call::{extract_tool_call, has_tool_call};

use super::loop_action::LoopAction;
use super::loop_state_mode::LoopStateModeExt;
use super::loop_state_types::LoopState;
use super::gated_execution::execute_gated_tool;

/// Write to debug log file
fn debug_log(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    if let Ok(mut file) = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("/tmp/odincode_debug.log")
    {
        let _ = writeln!(file, "{}", msg);
        let _ = file.flush();
    }
}

/// Process a ChatEvent and return the appropriate LoopAction
///
/// Called from ChatLoop::process_event(), this handles:
/// - ChatEvent::Complete: Parse for TOOL_CALL, classify tool, return action
/// - ChatEvent::Error: Terminate loop
/// - Other events: No action
pub fn process_event(
    state: &mut LoopState,
    event: &ChatEvent,
    tool_runner: &crate::execution_engine::ChatToolRunner,
    tx: &Option<ChatSender>,
) -> LoopAction {
    // Only process events for our session
    if event.session_id() != state.session_id {
        debug_log(&format!(
            "[CHAT_LOOP] Session mismatch: expected={}, got={} - dropping event",
            state.session_id,
            event.session_id()
        ));
        return LoopAction::None;
    }

    match event {
        ChatEvent::Complete {
            session_id: _,
            full_response,
        } => {
            // Phase 9.10: Log the full response for diagnostics
            debug_log(&format!(
                "[CHAT_LOOP] Complete event: response_len={}, has_tool_call={}, response_preview={}",
                full_response.len(),
                has_tool_call(full_response),
                if full_response.len() > 200 {
                    format!("{}...", &full_response[..200])
                } else {
                    full_response.clone()
                }
            ));

            // Phase 9.7: Add assistant response to frame stack
            state.add_assistant_response(full_response);

            // Store last response
            state.last_response = Some(full_response.clone());

            // Check for TOOL_CALL
            if !has_tool_call(full_response) {
                // No tool call, loop complete
                debug_log(&format!("[CHAT_LOOP] No tool call detected, completing loop. Full response: {}", full_response));
                state.complete();
                return LoopAction::LoopComplete(full_response.clone());
            }

            // Parse TOOL_CALL
            let (tool_call, _remaining) = match extract_tool_call(full_response) {
                Some(tc) => {
                    debug_log(&format!("[CHAT_LOOP] Tool call parsed: tool={}, args={:?}", tc.0.tool, tc.0.args));
                    tc
                },
                None => {
                    // Failed to parse, loop complete with error
                    debug_log(&format!("[CHAT_LOOP] Failed to parse TOOL_CALL from response: {}", full_response));
                    state.complete();
                    return LoopAction::LoopComplete(format!(
                        "{}\n[System: Failed to parse TOOL_CALL]",
                        full_response
                    ));
                }
            };

            // Classify tool
            let category = tool_runner.classify_tool(&tool_call.tool);
            debug_log(&format!("[CHAT_LOOP] Tool '{}' classified as {:?}", tool_call.tool, category));

            match category {
                ChatToolCategory::Auto => {
                    debug_log(&format!("[CHAT_LOOP] Auto branch: tool_allowed={}, exceeded_max_calls={}",
                        state.tool_allowed_in_mode(&tool_call.tool),
                        state.has_exceeded_max_calls()));

                    // Phase 9.9: Check mode constraints
                    // 1. Check if tool is allowed in current mode
                    if !state.tool_allowed_in_mode(&tool_call.tool) {
                        debug_log(&format!("[CHAT_LOOP] Tool '{}' NOT allowed in mode {:?}", tool_call.tool, state.prompt_mode()));
                        let error_msg = format!(
                            "[SYSTEM ERROR] Tool '{}' is NOT ALLOWED in {} mode. \
                            Allowed tools: {:?}. \
                            Please use only allowed tools for this mode.",
                            tool_call.tool,
                            state.prompt_mode().to_string(),
                            state.prompt_mode().allowed_tools()
                        );
                        state.add_hidden_result(&ToolResult {
                            tool: tool_call.tool.clone(),
                            success: false,
                            output_full: error_msg.clone(),
                            output_preview: error_msg.clone(),
                            error_message: Some("Tool not allowed in current mode".to_string()),
                            affected_path: None,
                            kind: crate::execution_engine::ToolOutputKind::Error,
                            structured_data: None,
                            execution_id: format!("mode-error-{}", uuid::Uuid::new_v4()),
                        });
                        return LoopAction::InjectError(error_msg);
                    }

                    // 2. Check if mode has exceeded max tool calls
                    if state.has_exceeded_max_calls() {
                        debug_log(&format!("[CHAT_LOOP] Max calls exceeded for mode {:?}", state.prompt_mode()));
                        // Capture original mode info before switching
                        let old_mode = state.prompt_mode();
                        let old_max_calls = old_mode.max_tool_calls();

                        // Switch to Presentation mode
                        state.switch_to_presentation_mode();
                        let error_msg = format!(
                            "[SYSTEM] Maximum tool calls ({}) reached for {} mode. \
                            Switching to PRESENTATION mode. Please provide your final answer.",
                            old_max_calls,
                            old_mode.to_string()
                        );
                        state.add_hidden_result(&ToolResult {
                            tool: "mode_switch".to_string(),
                            success: true,
                            output_full: error_msg.clone(),
                            output_preview: error_msg.clone(),
                            error_message: None,
                            affected_path: None,
                            kind: crate::execution_engine::ToolOutputKind::Textual,
                            structured_data: None,
                            execution_id: format!("mode-switch-{}", uuid::Uuid::new_v4()),
                        });
                        return LoopAction::InjectError(error_msg);
                    }

                    // Execute AUTO tool immediately
                    debug_log(&format!("[CHAT_LOOP] Returning ExecuteTool for tool={}", tool_call.tool));
                    LoopAction::ExecuteTool(tool_call.tool, tool_call.args)
                }
                ChatToolCategory::Gated => {
                    // Phase 9.2: Pause loop, emit ApprovalRequired event
                    let pending = super::loop_state_types::PendingGatedTool {
                        tool: tool_call.tool.clone(),
                        args: tool_call.args.clone(),
                        step: state.step,
                    };
                    state.pause(pending);

                    // Extract affected path for UI
                    let affected_path = tool_runner
                        .extract_affected_path(&tool_call.tool, &tool_call.args);

                    // Emit ApprovalRequired event (Phase 9.2)
                    if let Some(tx) = tx {
                        let args_json =
                            serde_json::to_string(&tool_call.args).unwrap_or_default();
                        let _ = tx.send(ChatEvent::ApprovalRequired {
                            session_id: state.session_id.clone(),
                            tool: tool_call.tool.clone(),
                            args: args_json,
                            affected_path,
                        });
                    }

                    LoopAction::RequestApproval(tool_call.tool, tool_call.args)
                }
                ChatToolCategory::Forbidden => {
                    // Inject error and continue
                    let error_msg = format!(
                        "[SYSTEM ERROR] Tool '{}' is not allowed in chat mode",
                        tool_call.tool
                    );
                    state.add_hidden_result(&ToolResult {
                        tool: tool_call.tool.clone(),
                        success: false,
                        output_full: error_msg.clone(),
                        output_preview: error_msg.clone(),
                        error_message: Some("Tool not allowed".to_string()),
                        affected_path: None,
                        kind: crate::execution_engine::ToolOutputKind::Error,
                        structured_data: None,
                        execution_id: format!("forbidden-{}", uuid::Uuid::new_v4()),
                    });
                    LoopAction::InjectError(error_msg)
                }
            }
        }
        ChatEvent::Error { .. } => {
            // LLM error, terminate loop
            state.complete();
            LoopAction::LoopError
        }
        _ => LoopAction::None,
    }
}

/// Execute an AUTO tool and trigger next LLM call
///
/// Called after LoopAction::ExecuteTool is returned.
/// Phase 9.7: Uses FrameStack for full conversation history.
pub fn execute_tool_and_continue(
    state: &mut LoopState,
    tool: String,
    args: HashMap<String, String>,
    db_root: &Path,
    tool_runner: &mut crate::execution_engine::ChatToolRunner,
    tx: &ChatSender,
) -> Result<LoopAction, String> {
    debug_log(&format!("[CHAT_LOOP] Executing tool '{}' (session_id={})", tool, state.session_id));

    // Advance step
    state.advance_step();

    // Phase 9.9: Track tool calls in current mode
    state.tool_calls_in_mode += 1;

    // Execute tool
    let result = tool_runner.execute_auto_tool(&tool, &args)?;
    debug_log(&format!("[CHAT_LOOP] Tool '{}' result: success={}, output_len={}", tool, result.success, result.output_full.len()));

    // Emit ToolResult event for UI (Phase 9.1: includes path)
    let _ = tx.send(ChatEvent::ToolResult {
        session_id: state.session_id.clone(),
        tool: result.tool.clone(),
        success: result.success,
        preview: result.output_preview.clone(),
        path: result.affected_path.clone(),
    });

    // Add tool result to frame stack
    state.add_hidden_result(&result);

    // Phase 9.7: Spawn next LLM call with full conversation history
    let session_id = state.session_id.clone();
    spawn_chat_thread_with_frame_stack(db_root, state.frame_stack_mut(), tx.clone(), Some(session_id));

    // Return action for UI
    if result.success {
        Ok(LoopAction::ToolExecuted(result))
    } else {
        Ok(LoopAction::ToolFailed(result))
    }
}

/// Handle user approval for GATED tool
///
/// Called when user approves a GATED tool.
/// Executes tool and continues loop.
/// Phase 9.7: Uses FrameStack for full conversation history.
pub fn handle_approval(
    state: &mut LoopState,
    db_root: &Path,
    tool_runner: &mut crate::execution_engine::ChatToolRunner,
    tx: &ChatSender,
) -> Result<LoopAction, String> {
    // Clone pending tool data before resuming
    let (tool_name, tool_args) = {
        let pending = state.pending_tool().ok_or("No pending tool")?;
        (pending.tool.clone(), pending.args.clone())
    };

    // Resume loop and track tool call
    state.resume();
    // Phase 9.9: Track GATED tool calls in current mode
    state.tool_calls_in_mode += 1;

    // Execute tool (bypassing AUTO check since user approved)
    let result = execute_gated_tool(&tool_name, &tool_args, tool_runner)?;

    // Add tool result to frame stack
    state.add_hidden_result(&result);

    // Phase 9.7: Spawn next LLM call with full conversation history
    let session_id = state.session_id.clone();
    spawn_chat_thread_with_frame_stack(db_root, state.frame_stack_mut(), tx.clone(), Some(session_id));

    Ok(LoopAction::ToolExecuted(result))
}

/// Handle user denial for GATED tool
///
/// Called when user denies a GATED tool.
/// Injects denial message and continues loop.
/// Phase 9.7: Uses FrameStack for full conversation history.
pub fn handle_denial(
    state: &mut LoopState,
    db_root: &Path,
    tx: &ChatSender,
) -> Result<LoopAction, String> {
    // Clone pending tool name before resuming
    let tool_name = {
        let pending = state.pending_tool().ok_or("No pending tool")?;
        pending.tool.clone()
    };

    // Inject denial message
    let denial_msg = format!(
        "[SYSTEM] Tool '{}' was denied by user. Please try a different approach.",
        tool_name
    );
    state.add_hidden_result(&ToolResult {
        tool: tool_name.clone(),
        success: false,
        output_full: denial_msg.clone(),
        output_preview: denial_msg.clone(),
        error_message: Some("User denied".to_string()),
        affected_path: None,
        kind: crate::execution_engine::ToolOutputKind::Error,
        structured_data: None,
        execution_id: format!("denied-{}", uuid::Uuid::new_v4()),
    });
    state.resume();

    // Phase 9.7: Spawn next LLM call with full conversation history
    let session_id = state.session_id.clone();
    spawn_chat_thread_with_frame_stack(db_root, state.frame_stack_mut(), tx.clone(), Some(session_id));

    Ok(LoopAction::ToolDenied)
}
