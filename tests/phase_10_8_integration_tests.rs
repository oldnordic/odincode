//! Phase 10.8: End-to-End Integration Tests
//!
//! Tests the complete progressive tool discovery workflow:
//! - User query → DiscoveryEngine → ToolDiscoveryContext
//! - Discovery → Prompt generation → LLM integration
//! - Discovery → Logging → Query validation

use std::fs::File;
use std::path::PathBuf;
use tempfile::TempDir;

use odincode::execution_tools::{
    log_discovery_event, query_discovery_events, ExecutionDb,
};
use odincode::llm::discovery::{
    discover_tools_for_chat, discover_tools_for_plan, ToolDiscoveryContext,
};
use odincode::tools::{
    core_tools, format_tool, DiscoveryEngine,
};

// Helper to set up test database
fn setup_test_db() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_root = temp_dir.path().to_path_buf();

    // Create codegraph.db (required by ExecutionDb)
    let codegraph_path = db_root.join("codegraph.db");
    File::create(&codegraph_path).expect("Failed to create codegraph.db");

    (temp_dir, db_root)
}

// ===== End-to-End Workflow Tests =====

#[test]
fn test_end_to_end_discovery_workflow() {
    // Given: A user query about file operations
    let engine = DiscoveryEngine::new();
    let query = "I need to write a new file with some content";

    // When: Running discovery
    let result = engine.discover(query, &[]);

    // Then: Should get core tools + relevant specialized tools
    assert!(!result.core.is_empty(), "Core tools should always be present");
    assert!(result.total_token_cost > 0, "Token cost should be calculated");

    // file_write should be discovered (keyword: "write")
    let specialized_names: Vec<_> = result.specialized.iter().map(|t| &t.name).collect();
    assert!(specialized_names.iter().any(|&n| n == "file_write"),
            "file_write should be discovered for 'write' keyword");
}

#[test]
fn test_end_to_end_chat_mode_workflow() {
    // Given: A user query for LLM chat mode
    let context = ToolDiscoveryContext::new("check the git status and then commit changes");

    // When: Discovering tools for chat
    let tools = discover_tools_for_chat(&context);

    // Then: Should include core tools + git tools
    assert!(!tools.is_empty(), "Should discover at least core tools");
    assert!(tools.contains(&String::from("bash_exec")), "bash_exec should be present");
    assert!(tools.contains(&String::from("display_text")), "display_text should be present");

    // git_status/git_commit should be discovered (keyword: "git")
    assert!(tools.contains(&String::from("git_status")),
            "git_status should be discovered for 'git' keyword");
}

#[test]
fn test_end_to_end_plan_mode_workflow() {
    // Given: A user query for plan mode
    let context = ToolDiscoveryContext::new("help me search for files and then edit them");

    // When: Discovering tools for plan
    let (tools, prompt) = discover_tools_for_plan(&context);

    // Then: Should get tools + formatted system prompt
    assert!(!tools.is_empty(), "Should discover at least core tools");
    assert!(!prompt.is_empty(), "System prompt should not be empty");

    // Prompt should include tool descriptions
    assert!(prompt.contains("file_search"), "Prompt should mention file_search");
    assert!(prompt.contains("Core Tools"), "Prompt should have Core Tools section");
}

#[test]
fn test_progressive_discovery_workflow() {
    // Given: A generic query (no specialized tools needed)
    let engine = DiscoveryEngine::new();
    let generic_query = "hello world";
    let generic_result = engine.discover(generic_query, &[]);

    // When: Query becomes more specific
    let specific_query = "I need to write code and search for files";
    let specific_result = engine.discover(specific_query, &[]);

    // Then: Specific query should discover more tools
    assert!(specific_result.specialized.len() >= generic_result.specialized.len(),
            "More specific query should discover equal or more specialized tools");

    // Token cost should be higher for specific query (more tools loaded)
    assert!(specific_result.total_token_cost > generic_result.total_token_cost,
            "More specific query should have higher token cost");
}

#[test]
fn test_token_cost_accuracy_across_workflow() {
    // Given: Core tools with known token costs
    let core = core_tools();
    let mut expected_core_cost = 0;
    for tool in &core {
        expected_core_cost += tool.token_cost;
    }

    // When: Running discovery with empty specialized set
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);  // Empty query = no specialized tools

    // Then: Total cost should match core tools cost
    assert_eq!(result.total_token_cost, expected_core_cost,
               "Token cost should exactly match core tools cost when no specialized tools discovered");
}

// ===== Logging Integration Tests =====

#[test]
fn test_discovery_logging_workflow() {
    // Given: Test database and discovery context
    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    let context = ToolDiscoveryContext::new("write a file");
    let tools = discover_tools_for_chat(&context);

    // When: Logging a discovery event
    log_discovery_event(&db, "test_session", &context, &tools, "keyword: write")
        .expect("Failed to log discovery event");

    // Then: Should be queryable
    let events = query_discovery_events(&db, "test_session")
        .expect("Failed to query events");

    assert_eq!(events.len(), 1, "Should have exactly one event");
    assert_eq!(events[0].session_id, "test_session");
    assert_eq!(events[0].reason, "keyword: write");
    assert!(events[0].tools_discovered.contains(&String::from("file_write")));
}

#[test]
fn test_multi_query_session_workflow() {
    // Given: A session with multiple queries
    let (_temp, db_root) = setup_test_db();
    let db = ExecutionDb::open(&db_root).expect("Failed to open DB");

    let session_id = "multi_query_session";

    // When: Running multiple queries in sequence
    let queries = vec![
        ("write a file", "keyword: write"),
        ("read some files", "keyword: read"),
        ("check git status", "keyword: git"),
    ];

    for (query, reason) in &queries {
        let context = ToolDiscoveryContext::new(*query);
        let tools = discover_tools_for_chat(&context);
        log_discovery_event(&db, session_id, &context, &tools, reason)
            .expect("Failed to log");
    }

    // Then: All events should be logged in order
    let events = query_discovery_events(&db, session_id)
        .expect("Failed to query events");

    assert_eq!(events.len(), 3, "Should have three events");

    // Events should be sorted by timestamp
    for i in 0..events.len() - 1 {
        assert!(events[i].timestamp <= events[i + 1].timestamp,
                "Events should be sorted by timestamp");
    }

    // Each event should have the correct reason
    assert_eq!(events[0].reason, "keyword: write");
    assert_eq!(events[1].reason, "keyword: read");
    assert_eq!(events[2].reason, "keyword: git");
}

// ===== Whitelist Validation Tests =====

#[test]
fn test_all_discovered_tools_in_whitelist() {
    // Given: Various user queries that might trigger discovery
    let engine = DiscoveryEngine::new();
    let queries = vec![
        "write a file",
        "search for files",
        "check git status",
        "run a bash command",
        "show text output",
        "edit with splice",
        "read lsp diagnostics",
    ];

    // Expected whitelist from router.rs
    let expected_whitelist = vec![
        "bash_exec", "display_text", "execution_summary", "file_create",
        "file_edit", "file_glob", "file_read", "file_search",
        "file_write", "git_diff", "git_log", "git_status", "lsp_check",
        "memory_query",
        "references_from_file_to_symbol_name", "references_to_symbol_name",
        "splice_patch", "splice_plan",
        "symbols_in_file",
        "wc",
    ];
    // Note: Internal tools (approval_granted, approval_denied) are NOT in whitelist

    // When: Running discovery for each query
    for query in queries {
        let result = engine.discover(query, &[]);

        // Then: ALL discovered tools should be in whitelist
        for tool in &result.core {
            assert!(expected_whitelist.contains(&tool.name.as_str()),
                    "Core tool '{}' should be in whitelist", tool.name);
        }

        for tool in &result.specialized {
            assert!(expected_whitelist.contains(&tool.name.as_str()),
                    "Specialized tool '{}' should be in whitelist", tool.name);
        }
    }
}

#[test]
fn test_chat_mode_validates_whitelist() {
    // Given: Discovery context
    let context = ToolDiscoveryContext::new("comprehensive query for all tools");

    // When: Discovering tools for chat
    let tools = discover_tools_for_chat(&context);

    // Then: All tools should be in expected whitelist
    let expected_whitelist = vec![
        "bash_exec", "display_text", "execution_summary", "file_create",
        "file_edit", "file_glob", "file_read", "file_search",
        "file_write", "git_diff", "git_log", "git_status", "lsp_check",
        "memory_query",
        "references_from_file_to_symbol_name", "references_to_symbol_name",
        "splice_patch", "splice_plan",
        "symbols_in_file",
        "wc",
    ];
    // Note: Internal tools (approval_granted, approval_denied) are NOT returned by discovery

    for tool in &tools {
        assert!(expected_whitelist.contains(&tool.as_str()),
                "Tool '{}' should be in whitelist", tool);
    }
}

// ===== Prompt Quality Tests =====

#[test]
fn test_prompt_includes_all_required_sections() {
    // Given: Discovery result with specialized tools
    let engine = DiscoveryEngine::new();
    let result = engine.discover("write and search files", &[]);

    // When: Generating system prompt
    let context = ToolDiscoveryContext::new("write and search files");
    let (_tools, prompt) = discover_tools_for_plan(&context);

    // Then: Prompt should have required sections
    assert!(prompt.contains("Core Tools"), "Prompt should have Core Tools section");
    assert!(prompt.contains("Available Tools"), "Prompt should list available tools");
    assert!(prompt.contains("Guidelines"), "Prompt should have usage guidelines");

    // If specialized tools were discovered, should have that section
    if !result.specialized.is_empty() {
        assert!(prompt.contains("Specialized Tools"),
                "Prompt should have Specialized Tools section when discovered");
    }
}

#[test]
fn test_prompt_formatting_consistency() {
    // Given: Multiple discovery results
    let engine = DiscoveryEngine::new();
    let queries = vec!["write file", "read file", "search files"];

    // When: Generating prompts for each
    for query in queries {
        let result = engine.discover(query, &[]);

        // Then: Each tool description should have consistent format
        for tool in &result.core {
            let formatted = format_tool(tool);
            // Format: name - description
            assert!(formatted.contains(&tool.name),
                    "Formatted tool should contain name");
            assert!(formatted.contains(&tool.description),
                    "Formatted tool should contain description");
        }
    }
}

// ===== Output-Based Discovery Tests =====

#[test]
fn test_output_based_discovery_workflow() {
    // Given: A context with recent outputs
    let engine = DiscoveryEngine::new();

    // When: Recent output contains error keyword
    let error_output = "error: something went wrong";
    let result = engine.discover("help me", &[error_output.to_string()]);

    // Then: lsp_check should be discovered (InOutput trigger: "error")
    let specialized_names: Vec<_> = result.specialized.iter().map(|t| &t.name).collect();
    assert!(specialized_names.iter().any(|&n| n == "lsp_check"),
            "lsp_check should be discovered when output contains 'error'");
}

// ===== Tool Pattern Tests =====

#[test]
fn test_tool_pattern_triggers_after_tool_usage() {
    // Given: A context with tool usage in output
    let engine = DiscoveryEngine::new();

    // When: Recent output mentions file_read (via ToolPattern trigger)
    let tool_usage = "file_read completed: read 150 lines from src/main.rs";
    let result = engine.discover("continue working", &[tool_usage.to_string()]);

    // Then: file_edit should be discovered (ToolPattern: ["file_read"])
    let specialized_names: Vec<_> = result.specialized.iter().map(|t| &t.name).collect();
    assert!(specialized_names.iter().any(|&n| n == "file_edit"),
            "Should discover file_edit when file_read was recently used");
}

// ===== Token Cost Tracking Tests =====

#[test]
fn test_discovery_context_tracks_token_cost() {
    // Given: A discovery context
    let context = ToolDiscoveryContext::new("write a file");

    // When: Discovering tools
    let _tools = discover_tools_for_chat(&context);

    // Then: Token cost should be trackable via DiscoveryEngine
    let engine = DiscoveryEngine::new();
    let result = engine.discover("write a file", &[]);

    assert!(result.total_token_cost > 0, "Token cost should be calculated");

    // Verify: Token cost should be reasonable (not explode)
    assert!(result.total_token_cost < 10000,
            "Token cost should be reasonable (< 10k tokens)");
}

#[test]
fn test_core_tools_token_cost_is_minimum() {
    // Given: Empty query (no specialized tools)
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    // When: Calculating minimum token cost
    let core = core_tools();
    let mut min_expected_cost = 0;
    for tool in &core {
        min_expected_cost += tool.token_cost;
    }

    // Then: Result should match minimum
    assert_eq!(result.total_token_cost, min_expected_cost,
               "Empty query should have minimum token cost (core tools only)");
}
