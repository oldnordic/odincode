//! Loop state mode-related methods
//!
//! # Mode Enforcement (Phase 9.9)
//!
//! Extension trait for LoopState providing prompt mode classification
//! and tool permission checking.

use crate::llm::router::ToolRouter;
use crate::llm::types::PromptMode;

use super::loop_state_types::LoopState;

/// Extension trait for mode-related LoopState methods
pub trait LoopStateModeExt {
    /// Phase 9.9: Check if tool is allowed in current prompt mode
    fn tool_allowed_in_mode(&self, tool: &str) -> bool;

    /// Phase 9.9: Check if mode has exceeded max tool calls
    fn has_exceeded_max_calls(&self) -> bool;

    /// Phase 9.9: Get current prompt mode
    fn prompt_mode(&self) -> PromptMode;

    /// Phase 9.9: Switch to Presentation mode (after tools complete)
    fn switch_to_presentation_mode(&mut self);
}

impl LoopStateModeExt for LoopState {
    /// Phase 9.9: Check if tool is allowed in current prompt mode
    fn tool_allowed_in_mode(&self, tool: &str) -> bool {
        ToolRouter::tool_allowed_in_mode(tool, self.current_prompt_mode)
    }

    /// Phase 9.9: Check if mode has exceeded max tool calls
    fn has_exceeded_max_calls(&self) -> bool {
        self.tool_calls_in_mode >= self.current_prompt_mode.max_tool_calls()
    }

    /// Phase 9.9: Get current prompt mode
    fn prompt_mode(&self) -> PromptMode {
        self.current_prompt_mode
    }

    /// Phase 9.9: Switch to Presentation mode (after tools complete)
    fn switch_to_presentation_mode(&mut self) {
        self.current_prompt_mode = PromptMode::Presentation;
        self.tool_calls_in_mode = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_mode_query_classification() {
        // "how many" → Query mode
        let state = LoopState::new("test".to_string(), "How many files are in src?".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Query);
    }

    #[test]
    fn test_prompt_mode_explore_classification() {
        // "where is" → Explore mode
        let state = LoopState::new("test".to_string(), "Where is the main function?".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Explore);

        // "find" → Explore mode
        let state = LoopState::new("test".to_string(), "Find all uses of Symbol".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Explore);
    }

    #[test]
    fn test_prompt_mode_mutation_classification() {
        // "edit" → Mutation mode
        let state = LoopState::new("test".to_string(), "Edit the main function".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Mutation);

        // "fix" → Mutation mode
        let state = LoopState::new("test".to_string(), "Fix this bug".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Mutation);
    }

    #[test]
    fn test_prompt_mode_default_to_explore() {
        // Ambiguous input → Explore mode (default)
        let state = LoopState::new("test".to_string(), "hello world".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Explore);
    }

    #[test]
    fn test_tool_allowed_in_query_mode() {
        let state = LoopState::new("test".to_string(), "How many files?".to_string());

        // These are allowed in Query mode
        assert!(state.tool_allowed_in_mode("wc"));
        assert!(state.tool_allowed_in_mode("memory_query"));

        // file_read is FORBIDDEN in Query mode
        assert!(!state.tool_allowed_in_mode("file_read"));
        assert!(!state.tool_allowed_in_mode("file_search"));
        assert!(!state.tool_allowed_in_mode("splice_patch"));
    }

    #[test]
    fn test_tool_allowed_in_explore_mode() {
        let state = LoopState::new("test".to_string(), "Where is the symbol?".to_string());

        // These are allowed in Explore mode
        assert!(state.tool_allowed_in_mode("file_read"));
        assert!(state.tool_allowed_in_mode("file_search"));
        assert!(state.tool_allowed_in_mode("symbols_in_file"));

        // These are FORBIDDEN in Explore mode
        assert!(!state.tool_allowed_in_mode("splice_patch"));
        assert!(!state.tool_allowed_in_mode("file_write"));
    }

    #[test]
    fn test_tool_allowed_in_mutation_mode() {
        let state = LoopState::new("test".to_string(), "Edit this function".to_string());

        // These are allowed in Mutation mode
        assert!(state.tool_allowed_in_mode("splice_patch"));
        assert!(state.tool_allowed_in_mode("file_edit"));
        assert!(state.tool_allowed_in_mode("lsp_check"));
    }

    #[test]
    fn test_presentation_mode_forbids_all_tools() {
        let mut state = LoopState::new("test".to_string(), "hello".to_string());
        state.switch_to_presentation_mode();

        // Presentation mode forbids ALL tools
        assert!(!state.tool_allowed_in_mode("file_read"));
        assert!(!state.tool_allowed_in_mode("file_search"));
        assert!(!state.tool_allowed_in_mode("splice_patch"));
    }

    #[test]
    fn test_max_tool_calls_per_mode() {
        // Query mode: 2 calls
        let state = LoopState::new("test".to_string(), "How many?".to_string());
        assert_eq!(state.prompt_mode().max_tool_calls(), 2);

        // Explore mode: 3 calls
        let state = LoopState::new("test".to_string(), "Where is it?".to_string());
        assert_eq!(state.prompt_mode().max_tool_calls(), 3);

        // Mutation mode: 5 calls
        let state = LoopState::new("test".to_string(), "Edit this".to_string());
        assert_eq!(state.prompt_mode().max_tool_calls(), 5);

        // Presentation mode: 0 calls
        let mut state = LoopState::new("test".to_string(), "hello".to_string());
        state.switch_to_presentation_mode();
        assert_eq!(state.prompt_mode().max_tool_calls(), 0);
    }

    #[test]
    fn test_exceeded_max_calls_detection() {
        let mut state = LoopState::new("test".to_string(), "How many?".to_string());
        assert!(!state.has_exceeded_max_calls());

        // Simulate tool calls up to limit
        state.tool_calls_in_mode = 2;
        assert!(state.has_exceeded_max_calls());
    }

    #[test]
    fn test_switch_to_presentation_mode() {
        let mut state = LoopState::new("test".to_string(), "hello".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Explore);

        state.switch_to_presentation_mode();
        assert_eq!(state.prompt_mode(), PromptMode::Presentation);
        assert_eq!(state.tool_calls_in_mode, 0); // Counter reset
    }

    #[test]
    fn test_priority_keyword_classification() {
        // Phase 9.11: Priority order is EXPLORE → MUTATION → QUERY
        // EXPLORE keywords have FIRST priority
        let state = LoopState::new("test".to_string(), "Where is the file to edit?".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Explore); // "where is" wins

        // MUTATION keywords have SECOND priority (checked before Query)
        let state = LoopState::new("test".to_string(), "How many files should I edit?".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Mutation); // "edit" wins over "how many"

        // QUERY keywords have THIRD priority (checked after Explore and Mutation)
        let state = LoopState::new("test".to_string(), "What is the total count of items?".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Query); // "total" wins (no explore/mutation keywords)

        // Simple mutation test
        let state = LoopState::new("test".to_string(), "Edit the main function".to_string());
        assert_eq!(state.prompt_mode(), PromptMode::Mutation); // "edit" wins
    }
}
