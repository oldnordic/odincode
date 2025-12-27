//! Core tools — always loaded into LLM context
//!
//! # Core Tool Criteria
//!
//! A tool is "core" if:
//! - Used in >50% of typical sessions
//! - Required for basic workflow (read → edit → test)
//! - Not discoverable from context alone
//!
//! # Current Core Tools (5)
//!
//! 1. file_read — Reading file contents
//! 2. file_search — Finding files by pattern
//! 3. splice_patch — Single symbol replacement
//! 4. bash_exec — Terminal commands
//! 5. display_text — Show LLM text responses

use crate::tools::metadata::{ToolCategory, ToolExample, ToolMetadata};

/// All core tools — always loaded into LLM context
pub fn core_tools() -> Vec<ToolMetadata> {
    vec![
        file_read_metadata(),
        file_search_metadata(),
        splice_patch_metadata(),
        bash_exec_metadata(),
        display_text_metadata(),
    ]
}

/// file_read metadata
///
/// Source: src/llm/router.rs:13 (TOOL_WHITELIST)
/// Source: src/file_tools/file_read.rs
fn file_read_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "file_read".to_string(),
        category: ToolCategory::Core,
        description: "Read the complete contents of a file. \
            Use for reading specific known files. \
            Supports line range with file:path:line_start:line_end format.".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Read a specific file".to_string(),
                command: "file_read src/lib.rs".to_string(),
                reasoning: "Direct file access is fastest for known files".to_string(),
            },
            ToolExample {
                scenario: "Read specific lines where error occurs".to_string(),
                command: "file_read src/main.rs:100:120".to_string(),
                reasoning: "Line range focuses on relevant code".to_string(),
            },
            ToolExample {
                scenario: "View configuration file".to_string(),
                command: "file_read Cargo.toml".to_string(),
                reasoning: "Configuration files need complete reading".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Finding files matching pattern".to_string(),
                command: "file_search \"**/*.test.rs\"".to_string(),
                reasoning: "file_search finds files by pattern, file_read reads them".to_string(),
            },
            ToolExample {
                scenario: "Getting directory listing".to_string(),
                command: "ls src/".to_string(),
                reasoning: "Directory listing is faster than reading files".to_string(),
            },
        ],
        token_cost: 100,
        gated: false,
    }
}

/// file_search metadata
///
/// Source: src/llm/router.rs:16 (TOOL_WHITELIST)
/// Source: src/file_tools/file_search.rs
fn file_search_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "file_search".to_string(),
        category: ToolCategory::Core,
        description: "Search for files by pattern using ripgrep. \
            Returns matching files with line numbers and context. \
            Use when exploring codebase or finding specific content.".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Find all test files".to_string(),
                command: "file_search \"test.*fn\\(\\)\"".to_string(),
                reasoning: "Pattern matching finds all test functions".to_string(),
            },
            ToolExample {
                scenario: "Search for specific function usage".to_string(),
                command: "file_search \"splice_patch\"".to_string(),
                reasoning: "Find where a function is called across codebase".to_string(),
            },
            ToolExample {
                scenario: "Find error messages".to_string(),
                command: "file_search \"Error::\"".to_string(),
                reasoning: "Locate error definitions in code".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Reading a known file".to_string(),
                command: "file_read src/main.rs".to_string(),
                reasoning: "Use file_read for specific known files".to_string(),
            },
            ToolExample {
                scenario: "Getting symbol definitions".to_string(),
                command: "symbols_in_file src/lib.rs".to_string(),
                reasoning: "Use Magellan tools for code structure queries".to_string(),
            },
        ],
        token_cost: 100,
        gated: false,
    }
}

/// splice_patch metadata
///
/// Source: src/llm/router.rs:19 (TOOL_WHITELIST)
/// Source: src/splice_tools/splice_patch.rs
fn splice_patch_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "splice_patch".to_string(),
        category: ToolCategory::Core,
        description: "Replace a single symbol definition using Splice. \
            Performs span-safe refactoring with cargo validation. \
            Use for focused single-symbol changes.".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Rename a function".to_string(),
                command: "splice_patch --file src/lib.rs --symbol old_name --kind function --with new_name".to_string(),
                reasoning: "Splice handles rename safely across references".to_string(),
            },
            ToolExample {
                scenario: "Replace function implementation".to_string(),
                command: "splice_patch --file src/main.rs --symbol process --kind fn --with impl.rs".to_string(),
                reasoning: "Replace single symbol with new implementation".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Multi-file refactoring".to_string(),
                command: "splice_plan --file plan.json".to_string(),
                reasoning: "Use splice_plan for complex multi-step changes".to_string(),
            },
            ToolExample {
                scenario: "Simple text replacement".to_string(),
                command: "file_edit --file src/lib.rs --old 'foo' --new 'bar'".to_string(),
                reasoning: "Use file_edit for simple text edits without refactoring".to_string(),
            },
        ],
        token_cost: 150,
        gated: false,
    }
}

/// bash_exec metadata
///
/// Source: src/llm/router.rs:31 (TOOL_WHITELIST)
/// Source: src/os_tools/bash_exec.rs
fn bash_exec_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "bash_exec".to_string(),
        category: ToolCategory::Core,
        description: "Execute bash commands safely with timeout. \
            Use for git operations, running tests, building projects. \
            Do NOT use for file operations (use file_* tools).".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Run tests".to_string(),
                command: "bash_exec \"cargo test\"".to_string(),
                reasoning: "Build and test require cargo commands".to_string(),
            },
            ToolExample {
                scenario: "Check git status".to_string(),
                command: "bash_exec \"git status\"".to_string(),
                reasoning: "Git operations require bash".to_string(),
            },
            ToolExample {
                scenario: "Build project".to_string(),
                command: "bash_exec \"cargo build --release\"".to_string(),
                reasoning: "Compilation requires cargo".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Reading file contents".to_string(),
                command: "bash_exec \"cat file.rs\"".to_string(),
                reasoning: "Use file_read instead of cat".to_string(),
            },
            ToolExample {
                scenario: "Writing files".to_string(),
                command: "bash_exec \"echo 'content' > file.rs\"".to_string(),
                reasoning: "Use file_write for atomic file writes".to_string(),
            },
            ToolExample {
                scenario: "Finding files".to_string(),
                command: "bash_exec \"find . -name '*.rs'\"".to_string(),
                reasoning: "Use file_search or file_glob for file operations".to_string(),
            },
        ],
        token_cost: 150,
        gated: false,
    }
}

/// display_text metadata
///
/// Source: src/llm/router.rs:32 (TOOL_WHITELIST)
/// Source: docs/PHASE_7_3_IMPLEMENTATION_REPORT.md
fn display_text_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "display_text".to_string(),
        category: ToolCategory::Core,
        description: "Display plain text responses to the user. \
            Used for showing explanations, summaries, or other non-tool output. \
            This is a pure UI tool with no side effects.".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Show explanation to user".to_string(),
                command: "display_text \"The function processes input data\"".to_string(),
                reasoning: "Natural language response needs text display".to_string(),
            },
            ToolExample {
                scenario: "Display summary of results".to_string(),
                command: "display_text \"Found 5 matches across 3 files\"".to_string(),
                reasoning: "Summarize search results for user".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Reading file contents".to_string(),
                command: "file_read src/lib.rs".to_string(),
                reasoning: "Use file_read for file operations, not display_text".to_string(),
            },
        ],
        token_cost: 50,
        gated: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_tools_count() {
        let tools = core_tools();
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn test_all_core_tools_have_examples() {
        let tools = core_tools();
        for tool in &tools {
            assert!(!tool.examples.is_empty(), "{} should have examples", tool.name);
        }
    }

    #[test]
    fn test_all_core_tools_have_not_examples() {
        let tools = core_tools();
        for tool in &tools {
            assert!(!tool.not_examples.is_empty(), "{} should have not_examples", tool.name);
        }
    }
}
