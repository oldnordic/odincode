//! Chat loop driver — MAIN-thread only
//!
//! # ChatLoop
//!
//! Main driver struct for multi-step tool execution.
//! Coordinates tool execution, approval flow, and LLM interaction.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::execution_engine::ChatToolRunner;
use crate::llm::chat_events::ChatSender;
use crate::llm::chat_thread::spawn_chat_thread;

use super::event_handler::{execute_tool_and_continue, handle_approval, handle_denial, process_event};
use super::loop_action::LoopAction;
use super::loop_state_types::LoopState;

/// Chat loop driver — MAIN-thread only
pub struct ChatLoop {
    /// Tool runner for AUTO tool execution
    pub tool_runner: ChatToolRunner,
    /// Current loop state (if active)
    pub loop_state: Option<LoopState>,
    /// Chat event sender (for spawning new LLM calls)
    tx: Option<ChatSender>,
}

impl ChatLoop {
    /// Create new chat loop driver
    pub fn new(tool_runner: ChatToolRunner) -> Self {
        Self {
            tool_runner,
            loop_state: None,
            tx: None,
        }
    }

    /// Set chat event sender (for triggering next LLM call)
    pub fn set_sender(&mut self, tx: ChatSender) {
        self.tx = Some(tx);
    }

    /// Start a new loop from user message
    ///
    /// Spawns initial chat thread for user message.
    pub fn start(&mut self, user_message: String, db_root: &Path) -> Result<(), String> {
        let tx = self.tx.as_ref().ok_or("Chat sender not set")?;

        // Generate session ID
        let session_id = self.generate_session_id();

        // Create loop state
        self.loop_state = Some(LoopState::new(session_id.clone(), user_message.clone()));

        // Spawn chat thread for initial user message (reuse loop session_id)
        spawn_chat_thread(db_root, user_message, tx.clone(), Some(session_id));

        Ok(())
    }

    /// Process a ChatEvent and advance loop if needed
    ///
    /// Called from main event loop after receiving ChatEvent.
    /// Returns LoopAction indicating what should happen next.
    ///
    /// Phase 9.7: Accumulates assistant responses in FrameStack
    pub fn process_event(&mut self, event: &crate::llm::chat_events::ChatEvent, _db_root: &Path) -> LoopAction {
        // Only process if loop is active
        let state = match &mut self.loop_state {
            Some(s) if s.active => s,
            _ => return LoopAction::None,
        };

        process_event(state, event, &self.tool_runner, &self.tx)
    }

    /// Execute an AUTO tool and trigger next LLM call
    ///
    /// Called after LoopAction::ExecuteTool is returned.
    /// Phase 9.7: Uses FrameStack for full conversation history.
    pub fn execute_tool_and_continue(
        &mut self,
        tool: String,
        args: std::collections::HashMap<String, String>,
        db_root: &Path,
    ) -> Result<LoopAction, String> {
        let state = self.loop_state.as_mut().ok_or("No active loop")?;
        let tx = self.tx.as_ref().ok_or("Chat sender not set")?;

        execute_tool_and_continue(state, tool, args, db_root, &mut self.tool_runner, tx)
    }

    /// Handle user approval for GATED tool
    ///
    /// Called when user approves a GATED tool.
    /// Executes tool and continues loop.
    /// Phase 9.7: Uses FrameStack for full conversation history.
    pub fn handle_approval(&mut self, db_root: &Path) -> Result<LoopAction, String> {
        let tx = self.tx.as_ref().ok_or("Chat sender not set")?;
        let state = self.loop_state.as_mut().ok_or("No active loop")?;

        handle_approval(state, db_root, &mut self.tool_runner, tx)
    }

    /// Handle user denial for GATED tool
    ///
    /// Called when user denies a GATED tool.
    /// Injects denial message and continues loop.
    /// Phase 9.7: Uses FrameStack for full conversation history.
    pub fn handle_denial(&mut self, db_root: &Path) -> Result<LoopAction, String> {
        let tx = self.tx.as_ref().ok_or("Chat sender not set")?;
        let state = self.loop_state.as_mut().ok_or("No active loop")?;

        handle_denial(state, db_root, tx)
    }

    /// Generate session ID
    fn generate_session_id(&self) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("loop-{:x}", nanos)
    }

    /// Get current loop state reference
    pub fn state(&self) -> Option<&LoopState> {
        self.loop_state.as_ref()
    }

    /// End current loop
    pub fn end(&mut self) {
        self.loop_state = None;
        self.tool_runner.reset_rate_limits();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;

    #[test]
    fn test_chat_loop_new() {
        let tool_runner = ChatToolRunner::new_no_db();
        let loop_driver = ChatLoop::new(tool_runner);

        assert!(loop_driver.loop_state.is_none());
        assert!(loop_driver.state().is_none());
    }

    #[test]
    fn test_chat_loop_set_sender() {
        let tool_runner = ChatToolRunner::new_no_db();
        let mut loop_driver = ChatLoop::new(tool_runner);

        let (tx, _rx) = channel();
        loop_driver.set_sender(tx);

        // Should be able to call start now
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = loop_driver.start("test".to_string(), temp_dir.path());
        assert!(result.is_ok());
        assert!(loop_driver.loop_state.is_some());
    }

    #[test]
    fn test_chat_loop_end() {
        let tool_runner = ChatToolRunner::new_no_db();
        let mut loop_driver = ChatLoop::new(tool_runner);

        // Start a loop
        let (tx, _rx) = channel();
        loop_driver.set_sender(tx);
        let temp_dir = tempfile::TempDir::new().unwrap();
        loop_driver
            .start("test".to_string(), temp_dir.path())
            .unwrap();

        assert!(loop_driver.loop_state.is_some());

        // End loop
        loop_driver.end();
        assert!(loop_driver.loop_state.is_none());
    }
}
