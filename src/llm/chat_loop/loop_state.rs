//! Loop state for multi-step tool execution
//!
//! # LoopState
//!
//! Tracks the state of a chat loop iteration including:
//! - FrameStack with full conversation history
//! - Pause/resume state for GATED tools
//! - Step counter and active status

use crate::execution_engine::ToolResult;
use crate::llm::frame_stack::{Frame, FrameStack};
use crate::llm::router::ToolRouter;

pub use super::loop_state_types::{LoopState, PendingGatedTool};

impl LoopState {
    /// Create new loop state
    pub fn new(session_id: String, original_user_message: String) -> Self {
        let mut frame_stack = FrameStack::new();
        // Add initial user message to frame stack
        frame_stack.add_user(original_user_message.clone());

        // Phase 9.9: Classify prompt mode from user message
        let current_prompt_mode = ToolRouter::classify_prompt_mode(&original_user_message);

        Self {
            session_id,
            step: 0,
            frame_stack,
            original_user_message,
            last_response: None,
            active: true,
            paused: false,
            pending_gated_tool: None,
            current_prompt_mode,
            tool_calls_in_mode: 0,
        }
    }

    /// Check if loop should continue
    pub fn should_continue(&self) -> bool {
        self.active && !self.paused && self.step < super::constants::MAX_AUTO_STEPS
    }

    /// Check if loop is paused (waiting for approval)
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Add hidden tool result to context (Phase 9.7: adds to FrameStack)
    pub fn add_hidden_result(&mut self, result: &ToolResult) {
        self.frame_stack.add_tool_result(
            result.tool.clone(),
            result.success,
            result.output_full.clone(),
            Some(result.execution_id.clone()),
        );
    }

    /// Add assistant response to frame stack (Phase 9.7)
    pub fn add_assistant_response(&mut self, response: &str) {
        self.frame_stack.add_assistant(response);
    }

    /// Complete the current assistant frame (Phase 9.7)
    pub fn complete_assistant_frame(&mut self) {
        self.frame_stack.complete_assistant();
    }

    /// Get frame stack reference (Phase 9.7)
    pub fn frame_stack(&self) -> &FrameStack {
        &self.frame_stack
    }

    /// Get mutable frame stack reference (Phase 9.7)
    pub fn frame_stack_mut(&mut self) -> &mut FrameStack {
        &mut self.frame_stack
    }

    /// Get context usage bar (Phase 9.7)
    pub fn context_usage_bar(&self, width: usize) -> String {
        self.frame_stack.context_usage_bar(width)
    }

    /// DEPRECATED: Use frame_stack instead
    pub fn hidden_context_string(&self) -> String {
        // For backward compatibility, extract tool results from frame stack
        self.frame_stack
            .iter()
            .filter_map(|f| {
                if let Frame::ToolResult { tool, output, .. } = f {
                    Some(format!("[TOOL RESULT: {}]\n{}", tool, output))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Increment step counter
    pub fn advance_step(&mut self) {
        self.step += 1;
    }

    /// Complete the loop (normal termination)
    pub fn complete(&mut self) {
        self.active = false;
        self.paused = false;
    }

    /// Pause the loop (waiting for GATED tool approval)
    pub fn pause(&mut self, pending: PendingGatedTool) {
        self.paused = true;
        self.pending_gated_tool = Some(pending);
    }

    /// Resume the loop (after user approval/denial)
    pub fn resume(&mut self) {
        self.paused = false;
        self.pending_gated_tool = None;
    }

    /// Get pending GATED tool (if paused)
    pub fn pending_tool(&self) -> Option<&PendingGatedTool> {
        self.pending_gated_tool.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::chat_loop::constants::MAX_AUTO_STEPS;
    use std::collections::HashMap;

    #[test]
    fn test_loop_state_new() {
        let state = LoopState::new("test-session".to_string(), "hello".to_string());
        assert_eq!(state.session_id, "test-session");
        assert_eq!(state.step, 0);
        assert!(state.active);
        assert!(!state.paused);
        // Phase 9.7: FrameStack contains initial user message
        assert_eq!(state.frame_stack().len(), 1);
    }

    #[test]
    fn test_loop_state_should_continue() {
        let mut state = LoopState::new("test".to_string(), "hello".to_string());

        // Initially should continue
        assert!(state.should_continue());

        // After MAX steps, should not continue
        for _ in 0..MAX_AUTO_STEPS {
            state.advance_step();
        }
        assert!(!state.should_continue());
    }

    #[test]
    fn test_loop_state_pause_resume() {
        let mut state = LoopState::new("test".to_string(), "hello".to_string());

        let pending = PendingGatedTool {
            tool: "file_write".to_string(),
            args: HashMap::new(),
            step: 1,
        };

        state.pause(pending.clone());
        assert!(state.is_paused());
        assert!(!state.should_continue()); // Paused loops don't continue

        state.resume();
        assert!(!state.is_paused());
        assert!(state.should_continue()); // Can continue again
    }

    #[test]
    fn test_loop_state_complete() {
        let mut state = LoopState::new("test".to_string(), "hello".to_string());

        state.complete();
        assert!(!state.active);
        assert!(!state.should_continue());
    }

    #[test]
    fn test_loop_state_add_hidden_result() {
        let mut state = LoopState::new("test".to_string(), "hello".to_string());

        let result = ToolResult {
            tool: "file_read".to_string(),
            success: true,
            output_full: "content".to_string(),
            output_preview: "content".to_string(),
            error_message: None,
            affected_path: Some("src/lib.rs".to_string()),
            kind: crate::execution_engine::ToolOutputKind::FileContent,
            structured_data: None,
            execution_id: "test-exec-1".to_string(),
        };

        // Phase 9.7: Adding tool results to FrameStack
        for _ in 0..10 {
            state.add_hidden_result(&result);
        }
        // FrameStack tracks tool results as frames
        assert!(state.frame_stack().len() >= 10);
    }

    #[test]
    fn test_hidden_context_string() {
        let mut state = LoopState::new("test".to_string(), "hello".to_string());

        let result = ToolResult {
            tool: "file_read".to_string(),
            success: true,
            output_full: "content".to_string(),
            output_preview: "content".to_string(),
            error_message: None,
            affected_path: None,
            kind: crate::execution_engine::ToolOutputKind::FileContent,
            structured_data: None,
            execution_id: "test-exec-2".to_string(),
        };

        state.add_hidden_result(&result);

        // Phase 9.7: hidden_context_string extracts from FrameStack
        let ctx = state.hidden_context_string();
        assert!(ctx.contains("[TOOL RESULT"));
        assert!(ctx.contains("file_read"));
    }

    #[test]
    fn test_pending_gated_tool() {
        let mut args = HashMap::new();
        args.insert("path".to_string(), "test.txt".to_string());

        let pending = PendingGatedTool {
            tool: "file_write".to_string(),
            args,
            step: 1,
        };

        assert_eq!(pending.tool, "file_write");
        assert_eq!(pending.step, 1);
        assert_eq!(pending.args.get("path"), Some(&"test.txt".to_string()));
    }
}
