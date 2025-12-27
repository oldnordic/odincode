//! Action returned by chat loop processing
//!
//! # LoopAction
//!
//! Represents the next action to take after processing a chat event.
//! Returned by `ChatLoop::process_event()` and related methods.

use crate::execution_engine::ToolResult;

/// Action returned by chat loop processing
///
/// Each variant represents a different outcome from event processing:
/// - Continue the loop with a tool call
/// - Request user approval for a gated tool
/// - Inject an error message
/// - Complete the loop (normal or error termination)
#[derive(Debug)]
pub enum LoopAction {
    /// No action needed
    None,

    /// Execute an AUTO tool (main thread should call execute_tool_and_continue)
    ExecuteTool(String, std::collections::HashMap<String, String>),

    /// Tool was executed successfully
    ToolExecuted(ToolResult),

    /// Tool execution failed
    ToolFailed(ToolResult),

    /// Request user approval for GATED tool
    RequestApproval(String, std::collections::HashMap<String, String>),

    /// User approved GATED tool
    ToolApproved,

    /// User denied GATED tool
    ToolDenied,

    /// Inject error message and continue
    InjectError(String),

    /// Loop complete (final response)
    LoopComplete(String),

    /// Loop terminated due to error
    LoopError,
}
