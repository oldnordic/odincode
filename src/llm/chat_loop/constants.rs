//! Chat loop constants
//!
//! # Constants
//!
//! - MAX_AUTO_STEPS: Safety limit for automatic tool execution
//! - MAX_CONTEXT_MESSAGES: DEPRECATED, use FrameStack::MAX_FRAMES instead

/// Maximum AUTO tool steps per loop (safety limit)
pub const MAX_AUTO_STEPS: usize = 10;

/// DEPRECATED (Phase 9.7): Replaced by FrameStack with MAX_FRAMES limit
#[deprecated(since = "0.9.7", note = "Use FrameStack::MAX_FRAMES instead")]
pub const MAX_CONTEXT_MESSAGES: usize = 10;

/// DEPRECATED (Phase 9.7): Use FrameStack::ToolResult instead
///
/// This was used to track tool results before FrameStack was introduced.
/// Now tool results are tracked as Frame::ToolResult in the frame stack.
#[derive(Debug, Clone)]
#[deprecated(since = "0.9.7", note = "Use FrameStack::ToolResult instead")]
pub struct HiddenToolResult {
    /// Tool that was executed
    pub tool: String,
    /// Formatted result for LLM context
    pub formatted: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_auto_steps_const() {
        assert_eq!(MAX_AUTO_STEPS, 10);
    }
}
