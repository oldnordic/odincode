//! Tool output classification (Task A)
//!
//! Semantic classification of tool outputs for proper routing:
//! - Textual → Chat panel
//! - Structural → Code View / Explorer state
//! - FileContent → Code View
//! - NumericSummary → Chat (concise) + structured value

use serde::{Deserialize, Serialize};

/// Semantic kind of tool output
///
/// Determines routing behavior:
/// - Textual outputs go to chat as-is
/// - Structural outputs are routed to UI components, summarized for chat
/// - FileContent goes to Code View
/// - NumericSummary gets concise chat treatment + stored value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolOutputKind {
    /// Textual/explanatory content → Chat panel
    Textual,

    /// Structured data (lists, paths, symbols) → UI state + chat summary
    Structural,

    /// File contents → Code View
    FileContent,

    /// Numeric counts/stats → Chat (concise) + stored value
    NumericSummary,

    /// Error output → Chat panel
    Error,

    /// Empty/void output (e.g., successful write) → No chat injection
    Void,
}

impl ToolOutputKind {
    /// Get the default kind for a given tool name
    pub fn for_tool(tool_name: &str) -> Self {
        match tool_name {
            // File operations
            "file_read" => ToolOutputKind::FileContent,
            "file_write" | "file_create" => ToolOutputKind::Void,

            // Search/list operations → Structural (file lists, matches)
            "file_search" | "file_glob" => ToolOutputKind::Structural,

            // Symbol queries → Structural (symbol lists, references)
            "symbols_in_file" | "references_to_symbol_name" | "references_from_file_to_symbol_name" => {
                ToolOutputKind::Structural
            }

            // LSP diagnostics → Structural (diagnostic list)
            "lsp_check" => ToolOutputKind::Structural,

            // Splice operations → Textual (stdout/stderr)
            "splice_patch" | "splice_plan" => ToolOutputKind::Textual,

            // Stats tools → NumericSummary
            "count_files" | "count_lines" | "fs_stats" => ToolOutputKind::NumericSummary,

            // Default → Textual
            _ => ToolOutputKind::Textual,
        }
    }

    /// Whether this kind should be injected into chat context
    pub fn should_inject_into_chat(&self) -> bool {
        matches!(
            self,
            ToolOutputKind::Textual
                | ToolOutputKind::FileContent
                | ToolOutputKind::NumericSummary
                | ToolOutputKind::Error
        )
    }

    /// Whether this kind has structured data for UI
    pub fn has_structured_data(&self) -> bool {
        matches!(
            self,
            ToolOutputKind::Structural | ToolOutputKind::NumericSummary
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_for_tool_file_operations() {
        assert_eq!(ToolOutputKind::for_tool("file_read"), ToolOutputKind::FileContent);
        assert_eq!(ToolOutputKind::for_tool("file_write"), ToolOutputKind::Void);
        assert_eq!(ToolOutputKind::for_tool("file_create"), ToolOutputKind::Void);
    }

    #[test]
    fn test_for_tool_search_operations() {
        assert_eq!(
            ToolOutputKind::for_tool("file_search"),
            ToolOutputKind::Structural
        );
        assert_eq!(ToolOutputKind::for_tool("file_glob"), ToolOutputKind::Structural);
    }

    #[test]
    fn test_for_tool_symbol_operations() {
        assert_eq!(
            ToolOutputKind::for_tool("symbols_in_file"),
            ToolOutputKind::Structural
        );
        assert_eq!(
            ToolOutputKind::for_tool("references_to_symbol_name"),
            ToolOutputKind::Structural
        );
        assert_eq!(
            ToolOutputKind::for_tool("references_from_file_to_symbol_name"),
            ToolOutputKind::Structural
        );
    }

    #[test]
    fn test_for_tool_stats() {
        assert_eq!(
            ToolOutputKind::for_tool("count_files"),
            ToolOutputKind::NumericSummary
        );
        assert_eq!(
            ToolOutputKind::for_tool("count_lines"),
            ToolOutputKind::NumericSummary
        );
        assert_eq!(ToolOutputKind::for_tool("fs_stats"), ToolOutputKind::NumericSummary);
    }

    #[test]
    fn test_should_inject_into_chat() {
        assert!(ToolOutputKind::Textual.should_inject_into_chat());
        assert!(ToolOutputKind::FileContent.should_inject_into_chat());
        assert!(ToolOutputKind::NumericSummary.should_inject_into_chat());
        assert!(ToolOutputKind::Error.should_inject_into_chat());
        assert!(!ToolOutputKind::Structural.should_inject_into_chat());
        assert!(!ToolOutputKind::Void.should_inject_into_chat());
    }

    #[test]
    fn test_has_structured_data() {
        assert!(ToolOutputKind::Structural.has_structured_data());
        assert!(ToolOutputKind::NumericSummary.has_structured_data());
        assert!(!ToolOutputKind::Textual.has_structured_data());
        assert!(!ToolOutputKind::FileContent.has_structured_data());
        assert!(!ToolOutputKind::Error.has_structured_data());
        assert!(!ToolOutputKind::Void.has_structured_data());
    }
}
