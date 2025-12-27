//! Tool catalog: metadata definitions for all available tools
//!
//! Contains the `default_tools()` function that returns the complete tool inventory.

use std::collections::HashMap;

use super::tool_registry::{ArgumentType, ToolArgument, ToolClassification, ToolCapability, ToolExamples, ToolMetadata, ResourceRequirement, SideEffectLevel};

/// Build default tool inventory
pub fn default_tools() -> HashMap<String, ToolMetadata> {
    let mut tools = HashMap::new();

    // === FILE OPERATIONS (AUTO) ===
    tools.insert(
        "file_read".to_string(),
        ToolMetadata::new("file_read", "Read complete file contents as text", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("path", ArgumentType::Path, true, "Absolute or relative path to file"),
            ])
            .with_examples(ToolExamples::single(
                "file_read(path=\"src/main.rs\")",
                "File contents as string",
            )),
    );

    tools.insert(
        "file_create".to_string(),
        ToolMetadata::new("file_create", "Create new file only if it does not exist", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Write, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::Local)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("path", ArgumentType::Path, true, "Path for new file"),
                ToolArgument::new("contents", ArgumentType::String, false, "Initial file content (default: empty)"),
            ])
            .with_examples(ToolExamples::single(
                "file_create(path=\"src/utils.rs\", contents=\"pub fn helper() {}\")",
                "File created: src/utils.rs",
            )),
    );

    tools.insert(
        "file_glob".to_string(),
        ToolMetadata::new("file_glob", "Find files matching glob pattern (sorted results)", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("pattern", ArgumentType::Pattern, true, "Glob pattern (e.g., '**/*.rs', 'src/**/*.rs')"),
                ToolArgument::new("root", ArgumentType::Path, false, "Root directory for search (default: '.')"),
            ])
            .with_examples(ToolExamples::single(
                "file_glob(pattern=\"**/*.rs\", root=\"src\")",
                "file_glob: 42 files matched\nExamples:\n  - src/main.rs\n  - src/lib.rs\n  - src/utils.rs\n(Full results in Explorer)",
            )),
    );

    tools.insert(
        "file_search".to_string(),
        ToolMetadata::new("file_search", "Search file contents with ripgrep regex", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Medium)
            .with_timeout(30000)
            .with_arguments(vec![
                ToolArgument::new("pattern", ArgumentType::String, true, "Ripgrep regex pattern to search for"),
                ToolArgument::new("root", ArgumentType::Path, false, "Root directory (default: '.')"),
            ])
            .with_examples(ToolExamples::single(
                "file_search(pattern=\"fn execute\", root=\"src\")",
                "file_search: 3 matches found\nExamples:\n  - src/executor.rs:42\n  - src/runner.rs:15\n  - src/main.rs:89\n(Full results in Explorer)",
            )),
    );

    tools.insert(
        "file_write".to_string(),
        ToolMetadata::new("file_write", "Atomically overwrite file with new contents", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Write, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::Local)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("path", ArgumentType::Path, true, "File to overwrite"),
                ToolArgument::new("contents", ArgumentType::String, true, "New file contents"),
            ])
            .with_examples(ToolExamples::single(
                "file_write(path=\"src/main.rs\", contents=\"fn main() { println!(\\\"Hello\\\"); }\")",
                "File written: src/main.rs",
            )),
    );

    tools.insert(
        "file_edit".to_string(),
        ToolMetadata::new("file_edit", "Edit file by line number or pattern (non-structural)", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Write, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::Local)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("path", ArgumentType::Path, true, "File to edit"),
                ToolArgument::new("line_number", ArgumentType::Integer, false, "Replace specific line"),
                ToolArgument::new("new_content", ArgumentType::String, false, "New line content"),
                ToolArgument::new("pattern", ArgumentType::String, false, "Regex pattern to find and replace"),
                ToolArgument::new("replace_all", ArgumentType::Boolean, false, "Replace all pattern matches (default: false)"),
                ToolArgument::new("insert_after", ArgumentType::Integer, false, "Insert new line after this line number"),
                ToolArgument::new("content", ArgumentType::String, false, "Content to insert"),
                ToolArgument::new("delete_line", ArgumentType::Integer, false, "Delete specific line"),
            ])
            .with_examples(ToolExamples::new(vec![
                "file_edit(path=\"src/main.rs\", line_number=10, new_content=\"    println!(\\\"Updated\\\");\")",
                "file_edit(path=\"src/main.rs\", pattern=\"TODO\", new_content=\"FIXME\", replace_all=true)",
                "file_edit(path=\"src/main.rs\", insert_after=5, content=\"    // New helper function\")",
            ], "file_edit: src/main.rs modified (1 lines changed, 0 inserted, 0 deleted)")),
    );

    // === SPAN-SAFE REFACTORING (GATED) ===
    tools.insert(
        "splice_patch".to_string(),
        ToolMetadata::new(
            "splice_patch",
            "Replace single symbol definition with span-safe validation via tree-sitter",
            ToolClassification::Gated,
        )
        .with_capabilities([ToolCapability::Write, ToolCapability::Execute, ToolCapability::Filesystem])
        .with_side_effect(SideEffectLevel::Mutating)
        .with_resource(ResourceRequirement::Heavy)
        .with_timeout(60000)
        .with_arguments(vec![
            ToolArgument::new("file", ArgumentType::Path, true, "File containing symbol to replace"),
            ToolArgument::new("symbol", ArgumentType::String, true, "Symbol name to replace"),
            ToolArgument::new("with", ArgumentType::Path, true, "File containing new symbol definition"),
            ToolArgument::new("kind", ArgumentType::String, false, "Symbol kind hint (e.g., 'function', 'struct')"),
        ])
        .with_examples(ToolExamples::single(
            "splice_patch(file=\"src/utils.rs\", symbol=\"helper\", with=\"/tmp/new_helper.rs\")",
            "Patched: replaced 'helper' in src/utils.rs\nChanged files: src/utils.rs",
        )),
    );

    tools.insert(
        "splice_plan".to_string(),
        ToolMetadata::new(
            "splice_plan",
            "Execute multi-step refactoring plan from JSON file",
            ToolClassification::Gated,
        )
        .with_capabilities([ToolCapability::Write, ToolCapability::Execute, ToolCapability::Filesystem])
        .with_side_effect(SideEffectLevel::Mutating)
        .with_resource(ResourceRequirement::Intensive)
        .with_timeout(300000)
        .with_arguments(vec![
            ToolArgument::new("plan_file", ArgumentType::Path, true, "JSON file with refactoring plan"),
        ])
        .with_examples(ToolExamples::single(
            "splice_plan(plan_file=\"refactor_plan.json\")",
            "Plan executed: 5 patches applied\nChanged files: src/a.rs, src/b.rs, src/c.rs",
        )),
    );

    // === CODEBASE QUERIES (AUTO) ===
    tools.insert(
        "symbols_in_file".to_string(),
        ToolMetadata::new(
            "symbols_in_file",
            "List all symbols (functions, structs, etc.) defined in a file",
            ToolClassification::Auto,
        )
        .with_capabilities([ToolCapability::Read, ToolCapability::Database])
        .with_side_effect(SideEffectLevel::None)
        .with_resource(ResourceRequirement::Light)
        .with_timeout(10000)
        .with_arguments(vec![
            ToolArgument::new("file_path", ArgumentType::Path, true, "Path to file"),
        ])
        .with_examples(ToolExamples::single(
            "symbols_in_file(file_path=\"src/main.rs\")",
            "symbols_in_file: 5 symbols\nExamples:\n  - main (Function)\n  - Config (Struct)\n  - run (Function)\n(Full results in Explorer)",
        )),
    );

    tools.insert(
        "references_to_symbol_name".to_string(),
        ToolMetadata::new(
            "references_to_symbol_name",
            "Find all references to a symbol across the codebase",
            ToolClassification::Auto,
        )
        .with_capabilities([ToolCapability::Read, ToolCapability::Database])
        .with_side_effect(SideEffectLevel::None)
        .with_resource(ResourceRequirement::Light)
        .with_timeout(10000)
        .with_arguments(vec![
            ToolArgument::new("symbol", ArgumentType::String, true, "Symbol name to find references for"),
        ])
        .with_examples(ToolExamples::single(
            "references_to_symbol_name(symbol=\"execute_plan\")",
            "references: 12 found\nExamples:\n  - execute_plan → src/executor.rs:42\n  - execute_plan → src/main.rs:15\n(Full results in Explorer)",
        )),
    );

    tools.insert(
        "references_from_file_to_symbol_name".to_string(),
        ToolMetadata::new(
            "references_from_file_to_symbol_name",
            "Find references from a specific file to a symbol",
            ToolClassification::Auto,
        )
        .with_capabilities([ToolCapability::Read, ToolCapability::Database])
        .with_side_effect(SideEffectLevel::None)
        .with_resource(ResourceRequirement::Light)
        .with_timeout(10000)
        .with_arguments(vec![
            ToolArgument::new("file_path", ArgumentType::Path, true, "Source file path"),
            ToolArgument::new("symbol", ArgumentType::String, true, "Symbol name"),
        ])
        .with_examples(ToolExamples::single(
            "references_from_file_to_symbol_name(file_path=\"src/main.rs\", symbol=\"Config\")",
            "references: 3 found\nExamples:\n  - Config → src/main.rs:10\n  - Config → src/main.rs:25\n(Full results in Explorer)",
        )),
    );

    // === COMPILER DIAGNOSTICS (AUTO) ===
    tools.insert(
        "cargo_check".to_string(),
        ToolMetadata::new(
            "cargo_check",
            "Run cargo check and return compiler diagnostics",
            ToolClassification::Auto,
        )
        .with_capabilities([ToolCapability::Execute, ToolCapability::Analysis])
        .with_side_effect(SideEffectLevel::None)
        .with_resource(ResourceRequirement::Heavy)
        .with_timeout(120000)
        .with_arguments(vec![
            ToolArgument::new("path", ArgumentType::Path, true, "Path to check (directory or file)"),
        ])
        .with_examples(ToolExamples::new(vec![
            "cargo_check(path=\".\")",
            "cargo_check(path=\"src/main.rs\")",
        ], "cargo_check: no errors - all clean!")),
    );

    tools.insert(
        "lsp_check".to_string(),
        ToolMetadata::new(
            "lsp_check",
            "Alias for cargo_check",
            ToolClassification::Auto,
        )
        .with_capabilities([ToolCapability::Execute, ToolCapability::Analysis])
        .with_side_effect(SideEffectLevel::None)
        .with_resource(ResourceRequirement::Heavy)
        .with_timeout(120000)
        .with_arguments(vec![
            ToolArgument::new("path", ArgumentType::Path, true, "Path to check (directory or file)"),
        ])
        .with_examples(ToolExamples::new(vec![
            "lsp_check(path=\".\")",
            "lsp_check(path=\"src/main.rs\")",
        ], "lsp_check: no errors - all clean!")),
    );

    // === GIT OPERATIONS (GATED) ===
    tools.insert(
        "git_status".to_string(),
        ToolMetadata::new("git_status", "Show git working tree status", ToolClassification::Gated)
            .with_capabilities([ToolCapability::Read, ToolCapability::Execute])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("repo_root", ArgumentType::Path, false, "Git repository root (default: '.')"),
            ])
            .with_examples(ToolExamples::single(
                "git_status(repo_root=\".\")",
                "git_status: 3 files changed",
            )),
    );

    tools.insert(
        "git_diff".to_string(),
        ToolMetadata::new("git_diff", "Show git diff of changes", ToolClassification::Gated)
            .with_capabilities([ToolCapability::Read, ToolCapability::Execute])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Medium)
            .with_timeout(10000)
            .with_arguments(vec![
                ToolArgument::new("repo_root", ArgumentType::Path, false, "Git repository root (default: '.')"),
                ToolArgument::new("path", ArgumentType::Path, false, "Specific file to diff"),
            ])
            .with_examples(ToolExamples::new(vec![
                "git_diff(repo_root=\".\")",
                "git_diff(repo_root=\".\", path=\"src/main.rs\")",
            ], "git_diff: 2 files changed (15 additions, 5 deletions)")),
    );

    tools.insert(
        "git_log".to_string(),
        ToolMetadata::new("git_log", "Show commit history", ToolClassification::Gated)
            .with_capabilities([ToolCapability::Read, ToolCapability::Execute])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("repo_root", ArgumentType::Path, false, "Git repository root (default: '.')"),
                ToolArgument::new("limit", ArgumentType::Integer, false, "Max commits to show"),
            ])
            .with_examples(ToolExamples::single(
                "git_log(repo_root=\".\", limit=10)",
                "git_log: 10 commits",
            )),
    );

    tools.insert(
        "git_commit".to_string(),
        ToolMetadata::new("git_commit", "Create git commit with changes", ToolClassification::Gated)
            .with_capabilities([ToolCapability::Write, ToolCapability::Execute])
            .with_side_effect(SideEffectLevel::Mutating)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(10000)
            .with_arguments(vec![
                ToolArgument::new("repo_root", ArgumentType::Path, false, "Git repository root (default: '.')"),
                ToolArgument::new("message", ArgumentType::String, false, "Commit message"),
            ])
            .with_examples(ToolExamples::single(
                "git_commit(repo_root=\".\", message=\"Fix tool routing bug\")",
                "git commit created: abc123",
            )),
    );

    // === EXECUTION MEMORY (AUTO) ===
    tools.insert(
        "memory_query".to_string(),
        ToolMetadata::new(
            "memory_query",
            "Query execution memory for tool outcomes and patterns",
            ToolClassification::Auto,
        )
        .with_capabilities([ToolCapability::Read, ToolCapability::Database])
        .with_side_effect(SideEffectLevel::None)
        .with_resource(ResourceRequirement::Light)
        .with_timeout(5000)
        .with_arguments(vec![
            ToolArgument::new("tool", ArgumentType::String, false, "Filter by tool name"),
            ToolArgument::new("session_id", ArgumentType::String, false, "Filter by session"),
            ToolArgument::new("success_only", ArgumentType::Boolean, false, "Show only successful executions"),
            ToolArgument::new("limit", ArgumentType::Integer, false, "Max results"),
            ToolArgument::new("include_output", ArgumentType::Boolean, false, "Include output in results"),
            ToolArgument::new("since", ArgumentType::Integer, false, "Unix timestamp start"),
            ToolArgument::new("until", ArgumentType::Integer, false, "Unix timestamp end"),
        ])
        .with_examples(ToolExamples::new(vec![
            "memory_query(tool=\"file_search\", success_only=true, limit=5)",
            "memory_query(tool=\"splice_patch\", since=1735210800000)",
        ], "memory_query: 42 executions found (showing 5)")),
    );

    tools.insert(
        "execution_summary".to_string(),
        ToolMetadata::new(
            "execution_summary",
            "Get aggregated statistics about tool executions",
            ToolClassification::Auto,
        )
        .with_capabilities([ToolCapability::Read, ToolCapability::Analysis])
        .with_side_effect(SideEffectLevel::None)
        .with_resource(ResourceRequirement::Light)
        .with_timeout(5000)
        .with_arguments(vec![
            ToolArgument::new("tool", ArgumentType::String, false, "Filter by tool name"),
            ToolArgument::new("session_id", ArgumentType::String, false, "Filter by session"),
            ToolArgument::new("since", ArgumentType::Integer, false, "Unix timestamp start"),
            ToolArgument::new("until", ArgumentType::Integer, false, "Unix timestamp end"),
        ])
        .with_examples(ToolExamples::single(
            "execution_summary(tool=\"splice_patch\")",
            "execution_summary: 150 total executions (135 success, 15 failed, 90% rate)",
        )),
    );

    // === OS UTILITIES (AUTO) ===
    tools.insert(
        "wc".to_string(),
        ToolMetadata::new("wc", "Count lines, words, characters in files", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read, ToolCapability::Analysis])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("paths", ArgumentType::Array, true, "Array of file paths"),
                ToolArgument::new("lines", ArgumentType::Boolean, false, "Count lines (default: true)"),
                ToolArgument::new("words", ArgumentType::Boolean, false, "Count words (default: true)"),
                ToolArgument::new("chars", ArgumentType::Boolean, false, "Count characters (default: true)"),
                ToolArgument::new("bytes", ArgumentType::Boolean, false, "Count bytes (default: false)"),
            ])
            .with_examples(ToolExamples::single(
                "wc(paths=\"[\\\"src/main.rs\\\", \\\"src/lib.rs\\\"]\")",
                "wc: 2 files",
            )),
    );

    // === STATS TOOLS (AUTO) ===
    tools.insert(
        "count_files".to_string(),
        ToolMetadata::new("count_files", "Count files matching pattern", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(10000)
            .with_arguments(vec![
                ToolArgument::new("pattern", ArgumentType::Pattern, true, "Glob pattern to match"),
                ToolArgument::new("root", ArgumentType::Path, false, "Root directory (default: '.')"),
                ToolArgument::new("by_extension", ArgumentType::Boolean, false, "Group by file extension"),
            ])
            .with_examples(ToolExamples::single(
                "count_files(pattern=\"**/*.rs\", by_extension=true)",
                "count_files: 42 files total (rs: 42)",
            )),
    );

    tools.insert(
        "count_lines".to_string(),
        ToolMetadata::new("count_lines", "Count total lines in matching files", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Medium)
            .with_timeout(30000)
            .with_arguments(vec![
                ToolArgument::new("pattern", ArgumentType::Pattern, true, "Glob pattern to match"),
                ToolArgument::new("root", ArgumentType::Path, false, "Root directory (default: '.')"),
            ])
            .with_examples(ToolExamples::single(
                "count_lines(pattern=\"**/*.rs\")",
                "count_lines: 5234 total lines in 42 files",
            )),
    );

    tools.insert(
        "fs_stats".to_string(),
        ToolMetadata::new("fs_stats", "Get filesystem statistics", ToolClassification::Auto)
            .with_capabilities([ToolCapability::Read, ToolCapability::Filesystem])
            .with_side_effect(SideEffectLevel::None)
            .with_resource(ResourceRequirement::Light)
            .with_timeout(5000)
            .with_arguments(vec![
                ToolArgument::new("path", ArgumentType::Path, true, "Path to analyze"),
                ToolArgument::new("max_depth", ArgumentType::Integer, false, "Maximum recursion depth"),
            ])
            .with_examples(ToolExamples::single(
                "fs_stats(path=\"src\", max_depth=3)",
                "fs_stats: 42 files, 8 dirs, 102400 bytes",
            )),
    );

    // === SHELL COMMANDS (FORBIDDEN) ===
    tools.insert(
        "bash_execute".to_string(),
        ToolMetadata::new(
            "bash_execute",
            "Execute shell command (FORBIDDEN - requires explicit override)",
            ToolClassification::Forbidden,
        )
        .with_capabilities([ToolCapability::Execute, ToolCapability::System])
        .with_side_effect(SideEffectLevel::External)
        .with_resource(ResourceRequirement::Intensive)
        .with_available(false)
        .with_arguments(vec![
            ToolArgument::new("command", ArgumentType::String, true, "Shell command to execute"),
            ToolArgument::new("timeout_ms", ArgumentType::Integer, false, "Command timeout (default: 30000)"),
            ToolArgument::new("working_dir", ArgumentType::Path, false, "Working directory"),
        ])
        .with_examples(ToolExamples::single(
            "bash_execute(command=\"ls -la\", working_dir=\"src\")",
            "Exit code: 0\nstdout: ...",
        )),
    );

    // === NETWORK OPERATIONS (FORBIDDEN) ===
    tools.insert(
        "http_request".to_string(),
        ToolMetadata::new(
            "http_request",
            "Make arbitrary HTTP requests (FORBIDDEN)",
            ToolClassification::Forbidden,
        )
        .with_capabilities([ToolCapability::Network, ToolCapability::Execute])
        .with_side_effect(SideEffectLevel::External)
        .with_resource(ResourceRequirement::Medium)
        .with_available(false)
        .with_arguments(vec![
            ToolArgument::new("url", ArgumentType::String, true, "Target URL"),
            ToolArgument::new("method", ArgumentType::String, false, "HTTP method (default: GET)"),
            ToolArgument::new("headers", ArgumentType::Object, false, "Request headers"),
            ToolArgument::new("body", ArgumentType::String, false, "Request body"),
        ])
        .with_examples(ToolExamples::single(
            "http_request(url=\"https://api.example.com\", method=\"GET\")",
            "{ \"status\": 200, \"body\": \"...\" }",
        )),
    );

    tools.insert(
        "web_fetch".to_string(),
        ToolMetadata::new(
            "web_fetch",
            "Fetch and parse web content as markdown (FORBIDDEN)",
            ToolClassification::Forbidden,
        )
        .with_capabilities([ToolCapability::Network, ToolCapability::Read])
        .with_side_effect(SideEffectLevel::External)
        .with_resource(ResourceRequirement::Medium)
        .with_available(false)
        .with_arguments(vec![
            ToolArgument::new("url", ArgumentType::String, true, "URL to fetch"),
        ])
        .with_examples(ToolExamples::single(
            "web_fetch(url=\"https://example.com\")",
            "# Page Title\n\nPage content as markdown...",
        )),
    );

    tools
}
