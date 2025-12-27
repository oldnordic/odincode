//! Conversation frame stack (Phase 9.7)
//!
//! Fixes LLM amnesia by preserving full conversation history across tool calls.
//!
//! Frame types:
//! - User: User input
//! - Assistant: LLM response (accumulated during streaming)
//! - ToolResult: Tool execution result (injected between assistant turns)
//!
//! Every LLM call receives the full ordered frame history.

use crate::llm::adapters::{LlmMessage, LlmRole};
use crate::llm::types::{PromptMode, TimelinePosition};
use std::collections::VecDeque;

/// Maximum frames to retain (prevents unbounded growth)
const MAX_FRAMES: usize = 50;

/// A single conversation frame
#[derive(Debug, Clone)]
pub enum Frame {
    /// User input
    User(String),
    /// Assistant response (may be accumulated during streaming)
    Assistant(String),
    /// Tool execution result
    ToolResult {
        tool: String,
        success: bool,
        output: String,
        /// Compaction flag: if true, output is suppressed (use memory_query to retrieve)
        compacted: bool,
        /// Execution ID for memory_query reference (if available)
        execution_id: Option<String>,
    },
}

impl Frame {
    /// Get display name for the frame type
    pub fn type_name(&self) -> &'static str {
        match self {
            Frame::User(_) => "User",
            Frame::Assistant(_) => "Assistant",
            Frame::ToolResult { .. } => "ToolResult",
        }
    }

    /// Get the frame content as a string
    pub fn content(&self) -> String {
        match self {
            Frame::User(s) => s.clone(),
            Frame::Assistant(s) => s.clone(),
            Frame::ToolResult {
                tool,
                success,
                output,
                compacted,
                execution_id: _,
            } => {
                if *compacted {
                    format!(
                        "[TOOL RESULT: {} {} - compacted, use memory_query to retrieve]",
                        tool,
                        if *success { "OK" } else { "FAILED" }
                    )
                } else {
                    format!(
                        "[TOOL RESULT: {} {}]\n{}",
                        tool,
                        if *success { "OK" } else { "FAILED" },
                        output
                    )
                }
            }
        }
    }

    /// Estimate token count (rough approximation: 1 token ≈ 4 characters)
    pub fn estimated_tokens(&self) -> usize {
        match self {
            Frame::User(s) => s.len() / 4,
            Frame::Assistant(s) => s.len() / 4,
            Frame::ToolResult { tool, output, compacted, .. } => {
                if *compacted {
                    // Compacted results use minimal tokens
                    (tool.len() + 50) / 4
                } else {
                    (tool.len() + output.len()) / 4
                }
            }
        }
    }
}

/// Conversation frame stack
///
/// Maintains ordered history of conversation turns.
/// User → Assistant → ToolResult → User → Assistant → ...
#[derive(Debug, Clone)]
pub struct FrameStack {
    /// Ordered frames (oldest first)
    frames: VecDeque<Frame>,
    /// Total estimated tokens
    total_tokens: usize,
}

impl FrameStack {
    /// Create new empty frame stack
    pub fn new() -> Self {
        Self {
            frames: VecDeque::with_capacity(16),
            total_tokens: 0,
        }
    }

    /// Add a user frame
    pub fn add_user(&mut self, message: String) {
        self.push_frame(Frame::User(message));
    }

    /// Add or append to an assistant frame
    ///
    /// If the last frame is Assistant, appends to it (streaming).
    /// Otherwise, creates a new Assistant frame.
    pub fn add_assistant(&mut self, chunk: &str) {
        // Check if last frame is Assistant (streaming continuation)
        if let Some(Frame::Assistant(existing)) = self.frames.back_mut() {
            // Append to existing assistant frame
            self.total_tokens -= existing.len() / 4; // Remove old token count
            existing.push_str(chunk);
            self.total_tokens += existing.len() / 4; // Add new token count
        } else {
            // Create new assistant frame
            self.push_frame(Frame::Assistant(chunk.to_string()));
        }
    }

    /// Complete the current assistant frame
    ///
    /// Ensures no more content will be appended to this frame.
    pub fn complete_assistant(&mut self) {
        // Assistant frames are naturally complete when we add a non-assistant frame
        // This is a no-op but provides semantic clarity
    }

    /// Add a tool result frame
    pub fn add_tool_result(&mut self, tool: String, success: bool, output: String, execution_id: Option<String>) {
        self.push_frame(Frame::ToolResult {
            tool,
            success,
            output,
            compacted: false,
            execution_id,
        });
    }

    /// Push a frame, maintaining max size
    fn push_frame(&mut self, frame: Frame) {
        let tokens = frame.estimated_tokens();
        self.frames.push_back(frame);
        self.total_tokens += tokens;

        // Evict oldest frames if over limit
        while self.frames.len() > MAX_FRAMES {
            if let Some(old) = self.frames.pop_front() {
                self.total_tokens -= old.estimated_tokens();
            }
        }
    }

    /// Build the full prompt for LLM
    ///
    /// Includes system prompt and all frames in order.
    pub fn build_prompt(&self) -> String {
        use crate::llm::contracts::chat_system_prompt;

        let mut prompt = String::new();

        // Add system prompt
        prompt.push_str(&chat_system_prompt());
        prompt.push_str("\n\n--- CONVERSATION HISTORY ---\n");

        // Add all frames
        for frame in &self.frames {
            match frame {
                Frame::User(msg) => {
                    prompt.push_str(&format!("User: {}\n", msg));
                }
                Frame::Assistant(msg) => {
                    prompt.push_str(&format!("Assistant: {}\n", msg));
                }
                Frame::ToolResult {
                    tool,
                    success,
                    output,
                    compacted,
                    execution_id,
                } => {
                    if *compacted {
                        let exec_ref = execution_id.as_ref().map(|id| format!(" (execution_id: {})", id)).unwrap_or_default();
                        prompt.push_str(&format!(
                            "[Tool {}]: {} - [Old tool result content cleared{}. Use memory_query tool to retrieve full details]\n",
                            tool,
                            if *success { "OK" } else { "FAILED" },
                            exec_ref
                        ));
                    } else {
                        prompt.push_str(&format!(
                            "[Tool {}]: {}\nResult: {}\n",
                            tool,
                            if *success { "OK" } else { "FAILED" },
                            output
                        ));
                    }
                }
            }
        }

        prompt
            .push_str("\n[Please continue the conversation, considering all prior context above.]");

        prompt
    }

    /// Build messages array for multi-turn LLM API (Phase 9.8 + 9.9)
    ///
    /// Returns Vec<LlmMessage> where each Frame becomes a separate message.
    /// - User frames → LlmRole::User
    /// - Assistant frames → LlmRole::Assistant
    /// - ToolResult frames → LlmRole::User with "[Tool {name}]: OK|FAILED\nResult: …" prefix
    /// - System prompt → LlmRole::System (first message)
    ///
    /// Phase 9.7: Timeline context injection - if timeline_position is provided,
    /// it is appended to the system prompt for temporal grounding.
    ///
    /// Phase 9.9: Internal prompt injection - if prompt_mode is provided,
    /// the appropriate internal prompt is injected into the system prompt.
    ///
    /// Tool result compaction: Old tool results (beyond 3 most recent) are
    /// automatically compacted to keep context concise.
    pub fn build_messages_with_timeline_and_mode(
        &mut self,
        timeline_position: Option<&TimelinePosition>,
        prompt_mode: Option<PromptMode>,
    ) -> Vec<LlmMessage> {
        // Auto-compact old tool results before building messages
        self.auto_compact_if_needed();
        use crate::llm::contracts::{chat_system_prompt, internal_prompt};

        let mut messages = Vec::with_capacity(self.frames.len() + 1);

        // Build system prompt with timeline context and internal mode prompt
        let system_prompt = if let Some(pos) = timeline_position {
            let timeline = format!(
                "\n\n=== EXECUTION TIMELINE (GROUND TRUTH) ===\n{}\n\
                 REQUIRED: Before editing, call memory_query to see recent history.\n\
                 Reference execution IDs, not memory.",
                format_timeline_position(pos)
            );
            format!("{}{}\n", chat_system_prompt(), timeline)
        } else {
            chat_system_prompt()
        };

        // Inject internal prompt if mode is specified
        let system_prompt = if let Some(mode) = prompt_mode {
            format!("{}\n\n{}", system_prompt, internal_prompt(mode))
        } else {
            system_prompt
        };

        // Add system prompt first
        messages.push(LlmMessage {
            role: LlmRole::System,
            content: system_prompt,
        });

        // Add all frames
        for frame in &self.frames {
            match frame {
                Frame::User(msg) => {
                    messages.push(LlmMessage {
                        role: LlmRole::User,
                        content: msg.clone(),
                    });
                }
                Frame::Assistant(msg) => {
                    messages.push(LlmMessage {
                        role: LlmRole::Assistant,
                        content: msg.clone(),
                    });
                }
                Frame::ToolResult {
                    tool,
                    success,
                    output,
                    compacted,
                    execution_id,
                } => {
                    // ToolResult becomes User message with prefix
                    let content = if *compacted {
                        let exec_ref = execution_id.as_ref().map(|id| format!(" (execution_id: {})", id)).unwrap_or_default();
                        format!(
                            "[Tool {}]: {} - [Old tool result content cleared{}. Use memory_query tool with session_id or execution_id to retrieve full details]",
                            tool,
                            if *success { "OK" } else { "FAILED" },
                            exec_ref
                        )
                    } else {
                        format!(
                            "[Tool {}]: {}\nResult: {}",
                            tool,
                            if *success { "OK" } else { "FAILED" },
                            output
                        )
                    };
                    messages.push(LlmMessage {
                        role: LlmRole::User,
                        content,
                    });
                }
            }
        }

        messages
    }

    /// Build messages array with timeline (backward-compatible wrapper)
    ///
    /// Calls build_messages_with_timeline_and_mode with no mode specified.
    pub fn build_messages_with_timeline(&mut self, timeline_position: Option<&TimelinePosition>) -> Vec<LlmMessage> {
        self.build_messages_with_timeline_and_mode(timeline_position, None)
    }

    /// Build messages array with mode (backward-compatible wrapper)
    ///
    /// Calls build_messages_with_timeline_and_mode with no timeline specified.
    pub fn build_messages_with_mode(&mut self, prompt_mode: PromptMode) -> Vec<LlmMessage> {
        self.build_messages_with_timeline_and_mode(None, Some(prompt_mode))
    }

    /// Build messages array (backward-compatible wrapper)
    ///
    /// Calls build_messages_with_timeline with no timeline position.
    pub fn build_messages(&mut self) -> Vec<LlmMessage> {
        self.build_messages_with_timeline(None)
    }

    /// Get the total estimated token count
    pub fn total_tokens(&self) -> usize {
        self.total_tokens
    }

    /// Get the number of frames
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Check if stack is empty
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Get the last assistant response (if any)
    pub fn last_assistant_response(&self) -> Option<String> {
        for frame in self.frames.iter().rev() {
            if let Frame::Assistant(msg) = frame {
                return Some(msg.clone());
            }
        }
        None
    }

    /// Get context usage as a percentage (for telemetry)
    ///
    /// Assumes context window of ~128K tokens (conservative estimate)
    pub fn context_usage_percent(&self) -> f32 {
        const MAX_CONTEXT: usize = 128_000;
        let percent = (self.total_tokens as f32 / MAX_CONTEXT as f32) * 100.0;
        percent.min(100.0)
    }

    /// Get context usage bar (for UI display)
    ///
    /// Returns a string like "[####....] 45%"
    pub fn context_usage_bar(&self, width: usize) -> String {
        let percent = self.context_usage_percent();
        let filled = (width as f32 * percent / 100.0).round() as usize;
        let empty = width.saturating_sub(filled);

        format!(
            "[{}{}] {:.0}%",
            "#".repeat(filled),
            ".".repeat(empty),
            percent
        )
    }

    /// Iterate over frames (Phase 9.7: public accessor)
    pub fn iter(&self) -> std::collections::vec_deque::Iter<'_, Frame> {
        self.frames.iter()
    }

    /// Clear all frames
    pub fn clear(&mut self) {
        self.frames.clear();
        self.total_tokens = 0;
    }

    /// Mark old tool results as compacted (keep N most recent)
    ///
    /// # Arguments
    /// * `keep_recent` - Number of recent tool results to keep un-compacted
    ///
    /// # Behavior
    /// - Iterates frames backwards (newest first)
    /// - Marks ToolResult frames as compacted=false for N most recent
    /// - Marks older ToolResult frames as compacted=true
    pub fn compact_old_tool_results(&mut self, keep_recent: usize) {
        let mut tool_result_count = 0;

        // Iterate in reverse (newest first)
        for frame in self.frames.iter_mut().rev() {
            if let Frame::ToolResult { compacted, .. } = frame {
                tool_result_count += 1;
                if tool_result_count > keep_recent {
                    *compacted = true;
                }
            }
        }
    }

    /// Auto-compact tool results before building messages
    ///
    /// This is called automatically by `build_messages_with_timeline_and_mode()`
    /// to keep only the most recent tool results in the LLM context.
    /// Older results are marked as compacted and the LLM is told to use memory_query.
    fn auto_compact_if_needed(&mut self) {
        const MAX_RECENT_TOOL_RESULTS: usize = 3;

        // Count tool results
        let tool_result_count = self
            .iter()
            .filter(|f| matches!(f, Frame::ToolResult { .. }))
            .count();

        // Compact if we have too many tool results
        if tool_result_count > MAX_RECENT_TOOL_RESULTS {
            self.compact_old_tool_results(MAX_RECENT_TOOL_RESULTS);
        }
    }
}

/// Format timeline position for prompt injection (Phase 9.7)
///
/// Returns a human-readable summary of the current timeline position.
fn format_timeline_position(pos: &TimelinePosition) -> String {
    format!(
        "Current Step: {} | Total Executions: {}\n\
         Last Execution: #{} ({}) {}\n\
         Time Since Last Query: {}ms ago\n\
         Pending Failures: {}",
        pos.current_step,
        pos.total_executions,
        pos.last_execution_id,
        pos.last_execution_tool,
        if pos.last_execution_success { "SUCCESS" } else { "FAILED" },
        pos.time_since_last_query_ms,
        pos.pending_failure_count
    )
}

impl Default for FrameStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_stack_new() {
        let stack = FrameStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
        assert_eq!(stack.total_tokens(), 0);
    }

    #[test]
    fn test_add_user() {
        let mut stack = FrameStack::new();
        stack.add_user("hello".to_string());
        assert_eq!(stack.len(), 1);
        assert!(!stack.is_empty());
    }

    #[test]
    fn test_add_assistant_single() {
        let mut stack = FrameStack::new();
        stack.add_assistant("hi there");
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_add_assistant_streaming() {
        let mut stack = FrameStack::new();
        stack.add_assistant("hello ");
        stack.add_assistant("world");
        assert_eq!(stack.len(), 1); // Still one frame

        if let Frame::Assistant(msg) = &stack.frames[0] {
            assert_eq!(msg, "hello world");
        } else {
            panic!("Expected Assistant frame");
        }
    }

    #[test]
    fn test_multiple_assistant_frames() {
        let mut stack = FrameStack::new();
        stack.add_assistant("first");
        stack.add_assistant(" second"); // Appends to "first"
        stack.complete_assistant();
        stack.add_user("question".to_string());
        stack.add_assistant("second answer"); // New frame

        assert_eq!(stack.len(), 3); // User, Assistant(first second), User, Assistant(second answer)
    }

    #[test]
    fn test_add_tool_result() {
        let mut stack = FrameStack::new();
        stack.add_tool_result("file_read".to_string(), true, "content".to_string(), None);
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn test_build_prompt() {
        let mut stack = FrameStack::new();
        stack.add_user("read file.txt".to_string());
        stack.add_assistant("I'll read that for you.");
        stack.add_tool_result("file_read".to_string(), true, "file contents".to_string(), None);

        let prompt = stack.build_prompt();
        assert!(prompt.contains("User: read file.txt"));
        assert!(prompt.contains("Assistant: I'll read that for you."));
        assert!(prompt.contains("[Tool file_read]: OK"));
        assert!(prompt.contains("Please continue the conversation"));
    }

    #[test]
    fn test_last_assistant_response() {
        let mut stack = FrameStack::new();
        stack.add_assistant("first response");
        stack.add_user("question".to_string());
        stack.add_assistant("second response");

        assert_eq!(
            stack.last_assistant_response(),
            Some("second response".to_string())
        );
    }

    #[test]
    fn test_max_frames_eviction() {
        let mut stack = FrameStack::new();

        // Add more than MAX_FRAMES
        for i in 0..(MAX_FRAMES + 10) {
            stack.add_user(format!("message {}", i));
        }

        // Should be at MAX_FRAMES
        assert_eq!(stack.len(), MAX_FRAMES);

        // Oldest messages should be evicted
        if let Frame::User(msg) = &stack.frames[0] {
            assert!(!msg.contains("message 0"));
        }
    }

    #[test]
    fn test_context_usage_percent() {
        let mut stack = FrameStack::new();

        // Empty stack
        assert_eq!(stack.context_usage_percent(), 0.0);

        // Add some content
        stack.add_user("a".repeat(400)); // ~100 tokens
        assert!(stack.context_usage_percent() < 1.0);
    }

    #[test]
    fn test_context_usage_bar() {
        let mut stack = FrameStack::new();
        stack.add_user("test".to_string());

        let bar = stack.context_usage_bar(10);
        assert!(bar.starts_with("["));
        assert!(bar.ends_with("%")); // Format: [......] 0%
        assert!(bar.contains("#") || bar.contains(".")); // At least some character
    }

    #[test]
    fn test_clear() {
        let mut stack = FrameStack::new();
        stack.add_user("test".to_string());
        stack.add_assistant("response");
        assert_eq!(stack.len(), 2);

        stack.clear();
        assert!(stack.is_empty());
        assert_eq!(stack.total_tokens(), 0);
    }

    #[test]
    fn test_frame_content() {
        let frame = Frame::User("hello".to_string());
        assert_eq!(frame.content(), "hello");
        assert_eq!(frame.type_name(), "User");
    }

    #[test]
    fn test_frame_estimated_tokens() {
        let frame = Frame::Assistant("hello world".to_string()); // ~11 chars / 4 = ~2-3 tokens
        assert!(frame.estimated_tokens() >= 2);
        assert!(frame.estimated_tokens() <= 4);
    }

    #[test]
    fn test_default() {
        let stack = FrameStack::default();
        assert!(stack.is_empty());
    }

    // Phase 9.7: Context continuity tests
    #[test]
    fn test_context_continuity_across_two_tool_calls() {
        let mut stack = FrameStack::new();

        // User: "read file.txt"
        stack.add_user("read file.txt".to_string());

        // Assistant: "I'll read that file for you."
        stack.add_assistant("I'll read that file for you.");
        stack.complete_assistant();

        // Tool result: file contents
        stack.add_tool_result("file_read".to_string(), true, "Hello, World!".to_string(), None);

        // User: "what did I just read?"
        stack.add_user("what did I just read?".to_string());

        // Build prompt should contain full history
        let prompt = stack.build_prompt();

        // Assert conversation history is preserved
        assert!(prompt.contains("read file.txt")); // Original request
        assert!(prompt.contains("I'll read that file")); // Assistant response
        assert!(prompt.contains("[Tool file_read]")); // Tool result
        assert!(prompt.contains("Hello, World!")); // Tool output
        assert!(prompt.contains("what did I just read?")); // Follow-up question
    }

    #[test]
    fn test_context_continuity_assistant_memory_preserved() {
        let mut stack = FrameStack::new();

        // Simulate conversation: user asks to read, assistant responds, tool executes
        stack.add_user("read src/main.rs".to_string());
        stack.add_assistant("Reading src/main.rs...");
        stack.complete_assistant();
        stack.add_tool_result("file_read".to_string(), true, "fn main() {}".to_string(), None);

        // Follow-up: user asks question about previous result
        stack.add_user("what function is in that file?".to_string());

        // Verify the stack maintains all frames
        assert_eq!(stack.len(), 4); // User, Assistant, ToolResult, User

        // Verify assistant can "remember" the tool result
        let prompt = stack.build_prompt();
        assert!(prompt.contains("fn main() {}"));
        assert!(prompt.contains("file_read"));
    }

    #[test]
    fn test_context_continuity_multiple_tool_results_preserved() {
        let mut stack = FrameStack::new();

        // First tool call
        stack.add_user("list files".to_string());
        stack.add_assistant("I'll list the files.");
        stack.complete_assistant();
        stack.add_tool_result(
            "file_glob".to_string(),
            true,
            "src/main.rs\nsrc/lib.rs".to_string(),
            None,
        );

        // Second tool call
        stack.add_user("read src/lib.rs".to_string());
        stack.add_assistant("Reading src/lib.rs...");
        stack.complete_assistant();
        stack.add_tool_result(
            "file_read".to_string(),
            true,
            "pub fn hello() {}".to_string(),
            None,
        );

        // Verify both tool results are in history
        let prompt = stack.build_prompt();
        assert!(prompt.contains("src/main.rs")); // First tool result
        assert!(prompt.contains("src/lib.rs")); // Referenced in second user message
        assert!(prompt.contains("pub fn hello() {}")); // Second tool result

        // Should have 6 frames: User, Assistant, ToolResult, User, Assistant, ToolResult
        assert_eq!(stack.len(), 6);
    }

    #[test]
    fn test_context_continuity_frame_ordering_preserved() {
        let mut stack = FrameStack::new();

        // Add frames in specific order
        stack.add_user("first".to_string());
        stack.add_assistant("response 1");
        stack.add_tool_result("tool_a".to_string(), true, "result A".to_string(), None);
        stack.add_user("second".to_string());
        stack.add_assistant("response 2");
        stack.add_tool_result("tool_b".to_string(), true, "result B".to_string(), None);

        // Verify frames are in correct order
        let frames: Vec<_> = stack.iter().collect();
        assert_eq!(frames.len(), 6);

        match &frames[0] {
            Frame::User(msg) => assert_eq!(msg, "first"),
            _ => panic!("Expected User frame"),
        }
        match &frames[1] {
            Frame::Assistant(msg) => assert_eq!(msg, "response 1"),
            _ => panic!("Expected Assistant frame"),
        }
        match &frames[2] {
            Frame::ToolResult { tool, .. } => assert_eq!(tool, "tool_a"),
            _ => panic!("Expected ToolResult frame"),
        }
        match &frames[3] {
            Frame::User(msg) => assert_eq!(msg, "second"),
            _ => panic!("Expected User frame"),
        }
        match &frames[4] {
            Frame::Assistant(msg) => assert_eq!(msg, "response 2"),
            _ => panic!("Expected Assistant frame"),
        }
        match &frames[5] {
            Frame::ToolResult { tool, .. } => assert_eq!(tool, "tool_b"),
            _ => panic!("Expected ToolResult frame"),
        }
    }
}
