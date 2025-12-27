//! Specialized tools — discovered on-demand based on context
//!
//! # Discovery Criteria
//!
//! A tool is "specialized" if:
//! - Used in <20% of typical sessions
//! - Triggered by specific keywords in user query
//! - Discoverable from user intent

use crate::tools::metadata::{DiscoveryTrigger, SpecializedTool};

/// All specialized tools with discovery triggers
pub fn specialized_tools() -> Vec<SpecializedTool> {
    vec![
        file_write_tool(),
        file_create_tool(),
        file_glob_tool(),
        file_edit_tool(),
        splice_plan_tool(),
        symbols_in_file_tool(),
        references_to_symbol_name_tool(),
        references_from_file_tool_tool(),
        lsp_check_tool(),
        memory_query_tool(),
        execution_summary_tool(),
        git_status_tool(),
        git_diff_tool(),
        git_log_tool(),
        wc_tool(),
    ]
}

// File operation tools

fn file_write_tool() -> SpecializedTool {
    SpecializedTool::new("file_write", "Atomically write contents to a file", 100)
        .with_example("Write new file contents", "file_write src/main.rs 'content'", "Atomic write with fsync")
        .with_example("Overwrite existing file", "file_write config.json '{\"key\": \"value\"}'", "Replaces entire file atomically")
        .with_not_example("Create new file only", "file_create new.rs 'content'", "Use file_create to avoid overwriting")
        .with_trigger(DiscoveryTrigger::Keyword("write".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("save".to_string()))
}

fn file_create_tool() -> SpecializedTool {
    SpecializedTool::new("file_create", "Create a new file only if it doesn't exist", 80)
        .with_example("Create new source file", "file_create src/new_module.rs", "Fails if file already exists")
        .with_not_example("Overwrite existing file", "file_write existing.rs 'content'", "Use file_write to overwrite")
        .with_trigger(DiscoveryTrigger::Keyword("create new file".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("new file".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("add file".to_string()))
}

fn file_glob_tool() -> SpecializedTool {
    SpecializedTool::new("file_glob", "Find files matching glob patterns", 80)
        .with_example("Find all Rust files", "file_glob \"**/*.rs\"", "Pattern matching for file discovery")
        .with_example("Find test files", "file_glob \"**/*test*.rs\"", "Wildcard expansion")
        .with_not_example("Search file contents", "file_search \"pattern\"", "Use file_search for content search")
        .with_trigger(DiscoveryTrigger::Keyword("glob".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("pattern".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("all files".to_string()))
}

fn file_edit_tool() -> SpecializedTool {
    SpecializedTool::new("file_edit", "Edit specific line ranges in a file", 100)
        .with_example("Replace specific lines", "file_edit --file src/lib.rs --line 10:20 --new 'new code'", "Targeted line replacement")
        .with_not_example("Refactor symbol safely", "splice_patch --file src/lib.rs --symbol foo --with bar", "Use splice_patch for symbol refactoring")
        .with_trigger(DiscoveryTrigger::Keyword("edit line".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("change line".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("replace lines".to_string()))
        .with_trigger(DiscoveryTrigger::ToolPattern(vec!["file_read".to_string()]))
}

// Splice tools

fn splice_plan_tool() -> SpecializedTool {
    SpecializedTool::new("splice_plan", "Execute multi-step refactoring plan", 120)
        .with_example("Multi-step refactoring", "splice_plan --file plan.json", "Execute complex refactoring across multiple symbols")
        .with_not_example("Single symbol change", "splice_patch --file src/lib.rs --symbol foo --with bar", "Use splice_patch for single changes")
        .with_trigger(DiscoveryTrigger::Keyword("multi-step".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("refactor plan".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("plan".to_string()))
}

// Magellan tools

fn symbols_in_file_tool() -> SpecializedTool {
    SpecializedTool::new("symbols_in_file", "List all symbols (functions, types, etc.) in a file", 100)
        .with_example("List functions in file", "symbols_in_file src/lib.rs", "Shows all defined symbols")
        .with_example("Find types defined", "symbols_in_file src/types.rs", "Shows structs, enums, types")
        .with_not_example("Find where symbol is used", "references_to_symbol_name MyStruct", "Use references tools for usage lookup")
        .with_trigger(DiscoveryTrigger::Keyword("functions".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("types".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("symbols".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("definitions".to_string()))
}

fn references_to_symbol_name_tool() -> SpecializedTool {
    SpecializedTool::new("references_to_symbol_name", "Find all references to a symbol", 80)
        .with_example("Find function callers", "references_to_symbol_name MyFunction", "Shows where function is called")
        .with_example("Find type usage", "references_to_symbol_name MyStruct", "Shows where type is used")
        .with_not_example("List symbols in file", "symbols_in_file src/lib.rs", "Use symbols_in_file for definitions")
        .with_trigger(DiscoveryTrigger::Keyword("where used".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("callers".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("references".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("usages".to_string()))
}

fn references_from_file_tool_tool() -> SpecializedTool {
    SpecializedTool::new("references_from_file_to_symbol_name", "Find what symbols a file imports/uses", 80)
        .with_example("Show file dependencies", "references_from_file_to_symbol_name src/main.rs MyStruct", "Shows if file uses the symbol")
        .with_not_example("Find all symbol references", "references_to_symbol_name MyStruct", "Use references_to_symbol_name for global lookup")
        .with_trigger(DiscoveryTrigger::Keyword("imports".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("dependencies".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("uses".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("depends on".to_string()))
}

// LSP tools

fn lsp_check_tool() -> SpecializedTool {
    SpecializedTool::new("lsp_check", "Run cargo check and capture diagnostics", 80)
        .with_example("Check for errors", "lsp_check", "Runs cargo check and returns diagnostics")
        .with_example("Check specific file", "lsp_check src/lib.rs", "Checks specific file for errors")
        .with_not_example("Understand code behavior", "file_read src/lib.rs", "Use file_read to understand code")
        .with_trigger(DiscoveryTrigger::Keyword("error".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("diagnostic".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("check".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("compile".to_string()))
        .with_trigger(DiscoveryTrigger::InOutput("error".to_string()))
}

// Evidence query tools

fn memory_query_tool() -> SpecializedTool {
    SpecializedTool::new("memory_query", "Query execution memory for past operations", 80)
        .with_example("Query past executions", "memory_query \"tool=file_read\"", "Searches execution history")
        .with_not_example("Current file contents", "file_read src/lib.rs", "Use file_read for current state")
        .with_trigger(DiscoveryTrigger::Keyword("previous".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("before".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("history".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("past".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("earlier".to_string()))
}

fn execution_summary_tool() -> SpecializedTool {
    SpecializedTool::new("execution_summary", "Get summary of execution statistics", 80)
        .with_example("Summarize recent activity", "execution_summary", "Shows execution stats")
        .with_not_example("Query specific executions", "memory_query \"tool=file_read\"", "Use memory_query for specific queries")
        .with_trigger(DiscoveryTrigger::Keyword("summary".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("statistics".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("stats".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("what happened".to_string()))
}

// Git tools

fn git_status_tool() -> SpecializedTool {
    SpecializedTool::new("git_status", "Show git working tree status", 70)
        .with_example("Check git status", "git_status", "Shows modified/added/deleted files")
        .with_not_example("See what changed in file", "git_diff src/lib.rs", "Use git_diff for file changes")
        .with_trigger(DiscoveryTrigger::Keyword("git status".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("git".to_string()))
}

fn git_diff_tool() -> SpecializedTool {
    SpecializedTool::new("git_diff", "Show git diff of changes", 70)
        .with_example("Show file changes", "git_diff", "Shows all changes")
        .with_example("Show specific file diff", "git_diff src/lib.rs", "Shows changes to one file")
        .with_not_example("See commit history", "git_log", "Use git_log for history")
        .with_trigger(DiscoveryTrigger::Keyword("diff".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("what changed".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("changes".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("git".to_string()))
}

fn git_log_tool() -> SpecializedTool {
    SpecializedTool::new("git_log", "Show git commit history", 70)
        .with_example("Show commit history", "git_log", "Shows recent commits")
        .with_not_example("See current changes", "git_diff", "Use git_diff for uncommitted changes")
        .with_trigger(DiscoveryTrigger::Keyword("history".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("commits".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("log".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("git".to_string()))
}

// OS tools

fn wc_tool() -> SpecializedTool {
    SpecializedTool::new("wc", "Count lines, words, characters in files", 50)
        .with_example("Count lines in file", "wc src/lib.rs", "Returns line, word, char counts")
        .with_not_example("Find files", "file_glob \"**/*.rs\"", "Use file_glob to find files")
        .with_trigger(DiscoveryTrigger::Keyword("count".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("lines".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("size".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("loc".to_string()))
}
