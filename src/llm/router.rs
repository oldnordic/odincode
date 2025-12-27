//! Tool router — Intent to tool mapping, whitelist, preconditions
//!
//! Pure data structures for tool routing.
//! No IO, no randomness, no side effects.

use crate::tools::{core_tools, specialized_tools, ToolCategory};
use std::collections::HashSet;
use std::path::Path;

use crate::llm::types::{Intent, PromptMode};

/// Tool whitelist (Phase 0 tools only)
///
/// This is a static list for security — tools must be explicitly added.
/// Use `verify_whitelist_sync()` test to ensure this matches registered tools.
pub const TOOL_WHITELIST: &[&str] = &[
    "bash_exec", // Phase 4: Bash command execution
    "display_text", // Phase 7.3: For displaying plain text LLM responses
    "execution_summary", // Phase 1.2: Aggregate execution statistics
    "file_create",
    "file_edit", // Phase 2: Patch-based text editing
    "file_glob",
    "file_read",
    "file_search",
    "file_write",
    "git_diff", // Phase 3: Git diff
    "git_log", // Phase 3: Git log
    "git_status", // Phase 3: Git status
    "lsp_check",
    "memory_query", // Phase 1.1: Query execution log
    "references_from_file_to_symbol_name",
    "references_to_symbol_name",
    "splice_patch",
    "splice_plan",
    "symbols_in_file",
    "wc", // Phase 4: Word count
];

/// Generate whitelist from registered tools (core + specialized, excluding internal)
///
/// Returns a sorted list of tool names that should be in TOOL_WHITELIST.
/// This is used in tests to verify the static whitelist stays in sync.
pub fn generate_tool_whitelist() -> Vec<String> {
    let mut whitelist = Vec::new();

    // Add core tools
    for tool in core_tools() {
        whitelist.push(tool.name.clone());
    }

    // Add specialized tools (excluding internal)
    for tool in specialized_tools() {
        if tool.metadata.category != ToolCategory::Internal {
            whitelist.push(tool.metadata.name.clone());
        }
    }

    whitelist.sort();
    whitelist
}

/// Check if tool is in whitelist
pub fn tool_is_allowed(tool: &str) -> bool {
    TOOL_WHITELIST.contains(&tool)
}

/// Tool router — maps intents to allowed tools
///
/// Pure data structure. Deterministic results.
#[derive(Debug, Clone)]
pub struct ToolRouter;

impl ToolRouter {
    pub fn new() -> Self {
        ToolRouter
    }

    /// Classify user input into PromptMode based on keyword patterns
    ///
    /// This is a HARD RULE that determines which internal prompt to inject.
    /// Classification happens BEFORE any tool call.
    ///
    /// Priority order (first match wins):
    /// 1. Explore → location/discovery keywords (checked first to avoid "files in" ambiguity)
    /// 2. Mutation → edit/change keywords
    /// 3. Query → counting/statistics keywords
    /// 4. Presentation → explanation keywords (after tools complete)
    pub fn classify_prompt_mode(user_input: &str) -> PromptMode {
        let input = user_input.to_lowercase();

        // EXPLORE MODE keywords — location/discovery (checked first for "list files in" case)
        const EXPLORE_KEYWORDS: &[&str] = &[
            "where is", "find", "locate", "which file", "show me", "list",
            "search for", "look for", "symbol", "reference", "defined in",
            "used in", "called from", "imports", "exports",
        ];

        // MUTATION MODE keywords — edit/change
        const MUTATION_KEYWORDS: &[&str] = &[
            "edit", "fix", "change", "modify", "refactor", "rename",
            "replace", "update", "delete", "add", "remove", "move",
            "extract", "inline", "rewrite", "transform",
        ];

        // QUERY MODE keywords — counting/statistics (checked after Explore)
        const QUERY_KEYWORDS: &[&str] = &[
            "how many", "how much", "count", "total", "sum", "number of",
            "lines of", "loc", "size of", "statistics", "stats",
            "frequency", "occurrences", "average", "median",
        ];

        // Check each category in priority order
        // Phase 9.11 FIX: Check Explore before Query to handle "list files in" correctly
        // "list" indicates Explore intent, even if "files in" substring exists
        for keyword in EXPLORE_KEYWORDS {
            if input.contains(keyword) {
                return PromptMode::Explore;
            }
        }

        for keyword in MUTATION_KEYWORDS {
            if input.contains(keyword) {
                return PromptMode::Mutation;
            }
        }

        for keyword in QUERY_KEYWORDS {
            if input.contains(keyword) {
                return PromptMode::Query;
            }
        }

        // Default to Explore mode for ambiguous input
        PromptMode::Explore
    }

    /// Check if a tool is allowed in the given mode
    pub fn tool_allowed_in_mode(tool: &str, mode: PromptMode) -> bool {
        let allowed = mode.allowed_tools();
        let forbidden = mode.forbidden_tools();

        // Explicitly forbidden takes precedence
        if forbidden.contains(&tool) {
            return false;
        }

        allowed.contains(&tool)
    }

    /// Get all allowed tools
    pub fn allowed_tools() -> HashSet<String> {
        TOOL_WHITELIST.iter().map(|s| s.to_string()).collect()
    }

    /// Get tools allowed for given intent
    pub fn tools_for_intent(&self, intent: &Intent) -> Vec<String> {
        match intent {
            Intent::Read => vec![
                "file_read".to_string(),
                "symbols_in_file".to_string(),
                "references_to_symbol_name".to_string(),
                "references_from_file_to_symbol_name".to_string(),
            ],
            Intent::Mutate => vec![
                "splice_patch".to_string(),
                "splice_plan".to_string(),
                "file_write".to_string(),
                "file_create".to_string(),
                "file_edit".to_string(),
            ],
            Intent::Query => vec![
                "file_search".to_string(),
                "file_glob".to_string(),
                "lsp_check".to_string(),
                "memory_query".to_string(),
                "execution_summary".to_string(),
                "git_status".to_string(),
                "git_diff".to_string(),
                "git_log".to_string(),
                "wc".to_string(),
                "bash_exec".to_string(),
            ],
            Intent::Explain => vec![
                // Explain uses evidence queries, not direct tools
                // But we allow lsp_check for diagnostic context
                "lsp_check".to_string(),
            ],
        }
    }

    /// Get preconditions for a tool
    pub fn preconditions_for_tool(tool: &str) -> Vec<String> {
        match tool {
            "file_read" | "file_write" | "file_create" => vec!["file exists".to_string()],
            "file_edit" => vec!["file exists".to_string()],
            "file_search" | "file_glob" => vec!["root exists".to_string()],
            "splice_patch" => vec![
                "file is in Cargo workspace".to_string(),
                "symbol exists in file".to_string(),
            ],
            "splice_plan" => vec![
                "plan file exists".to_string(),
                "file is in Cargo workspace".to_string(),
            ],
            "symbols_in_file"
            | "references_to_symbol_name"
            | "references_from_file_to_symbol_name" => {
                vec![
                    "codegraph.db exists".to_string(),
                    "magellan has indexed file".to_string(),
                ]
            }
            "lsp_check" => vec!["Cargo project exists".to_string()],
            "memory_query" => vec!["execution_log.db exists".to_string()],
            "execution_summary" => vec!["execution_log.db exists".to_string()],
            "git_status" => vec!["git repository exists".to_string()],
            "git_diff" => vec!["git repository exists".to_string()],
            "git_log" => vec!["git repository exists".to_string()],
            "wc" => vec!["file exists".to_string()],
            "bash_exec" => vec!["command is safe".to_string()],
            "display_text" => vec![],
            _ => vec![],
        }
    }
}

impl Default for ToolRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if file exists (precondition helper)
///
/// Used by validation layer. Pure function, no side effects.
pub fn check_file_exists(path: &Path) -> bool {
    path.exists()
}

/// Check if Cargo project exists (precondition helper)
pub fn check_cargo_project_exists(path: &Path) -> bool {
    let cargo_toml = path.join("Cargo.toml");
    cargo_toml.exists()
}

/// Check if codegraph.db exists (precondition helper)
pub fn check_codegraph_exists(db_root: &Path) -> bool {
    let codegraph_db = db_root.join("codegraph.db");
    codegraph_db.exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_whitelist_has_expected_count() {
        assert_eq!(TOOL_WHITELIST.len(), 20);
    }

    #[test]
    fn debug_show_generated_whitelist() {
        let generated = generate_tool_whitelist();
        println!("Generated whitelist ({} tools):", generated.len());
        for (i, tool) in generated.iter().enumerate() {
            println!("  {}: {}", i, tool);
        }
    }

    #[test]
    fn test_whitelist_matches_registered_tools() {
        // Verify static whitelist matches generated whitelist from registered tools
        let generated = generate_tool_whitelist();
        let static_whitelist: Vec<&str> = TOOL_WHITELIST.to_vec();

        // Both should be sorted
        let mut static_sorted = static_whitelist.clone();
        static_sorted.sort();

        assert_eq!(generated, static_sorted,
            "TOOL_WHITELIST should match tools registered in core_tools() + specialized_tools() (excluding Internal).\n\
             Run generate_tool_whitelist() to see expected list, then update TOOL_WHITELIST in router.rs");
    }

    #[test]
    fn test_tool_is_allowed() {
        assert!(tool_is_allowed("file_read"));
        assert!(tool_is_allowed("lsp_check"));
        assert!(!tool_is_allowed("unknown_tool"));
    }

    #[test]
    fn test_tools_for_intent() {
        let router = ToolRouter::new();

        let read_tools = router.tools_for_intent(&Intent::Read);
        assert!(read_tools.contains(&"file_read".to_string()));

        let mutate_tools = router.tools_for_intent(&Intent::Mutate);
        assert!(mutate_tools.contains(&"splice_patch".to_string()));

        let query_tools = router.tools_for_intent(&Intent::Query);
        assert!(query_tools.contains(&"file_search".to_string()));
    }

    #[test]
    fn test_preconditions_defined() {
        for tool in TOOL_WHITELIST {
            let preconditions = ToolRouter::preconditions_for_tool(tool);
            // Phase 7.3: display_text is allowed to have empty preconditions
            // (it's a pure UI tool that doesn't interact with external systems)
            if *tool == "display_text" {
                assert!(
                    preconditions.is_empty(),
                    "Tool display_text should have no preconditions"
                );
            } else {
                assert!(
                    !preconditions.is_empty(),
                    "Tool {} must have preconditions",
                    tool
                );
            }
        }
    }

    #[test]
    fn test_check_file_exists() {
        assert!(check_file_exists(Path::new(".")));
        assert!(!check_file_exists(Path::new(
            "/nonexistent/path/that/does/not/exist"
        )));
    }

    #[test]
    fn test_router_is_deterministic() {
        let router1 = ToolRouter::new();
        let router2 = ToolRouter::new();

        assert_eq!(
            router1.tools_for_intent(&Intent::Mutate),
            router2.tools_for_intent(&Intent::Mutate)
        );
    }
}
