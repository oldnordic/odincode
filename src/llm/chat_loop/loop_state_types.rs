//! Loop state type definitions
//!
//! # Types
//!
//! - PendingGatedTool: Tool awaiting user approval
//! - LoopState: Main state structure for chat loop

use std::collections::HashMap;

use crate::llm::frame_stack::FrameStack;
use crate::llm::types::PromptMode;

/// Pending GATED tool (awaiting user approval)
#[derive(Debug, Clone)]
pub struct PendingGatedTool {
    /// Tool name
    pub tool: String,
    /// Tool arguments
    pub args: HashMap<String, String>,
    /// Step number when tool was requested
    pub step: usize,
}

/// Loop state for multi-step tool execution (Phase 9.7: FrameStack-based, Phase 9.9: Mode enforcement)
pub struct LoopState {
    /// Session ID for this loop
    pub session_id: String,
    /// Current step number (1-indexed)
    pub step: usize,
    /// Phase 9.7: Full conversation frame stack
    pub frame_stack: FrameStack,
    /// Original user message that started the loop
    pub original_user_message: String,
    /// Last LLM response (for TOOL_CALL parsing)
    pub last_response: Option<String>,
    /// Whether loop is active
    pub active: bool,
    /// Whether loop is paused (waiting for approval on GATED tool)
    pub paused: bool,
    /// Pending GATED tool (when paused)
    pub pending_gated_tool: Option<PendingGatedTool>,
    /// Phase 9.9: Current prompt mode (classified from user message)
    pub current_prompt_mode: PromptMode,
    /// Phase 9.9: Number of tool calls made in current mode
    pub tool_calls_in_mode: usize,
}
