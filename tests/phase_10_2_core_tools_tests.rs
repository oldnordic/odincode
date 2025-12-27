//! Phase 10.2: Core tool definitions tests
//!
//! TDD approach: Tests for core tool metadata.

use odincode::tools::{core_tools, ToolCategory};

#[test]
fn test_core_tools_returns_five_tools() {
    let tools = core_tools();
    assert_eq!(tools.len(), 5, "Should have exactly 5 core tools");
}

#[test]
fn test_core_tools_all_have_core_category() {
    let tools = core_tools();
    for tool in &tools {
        assert_eq!(tool.category, ToolCategory::Core, "{} should be Core", tool.name);
    }
}

#[test]
fn test_core_tools_all_valid() {
    let tools = core_tools();
    for tool in &tools {
        assert!(tool.is_valid(), "{} should be valid (has examples)", tool.name);
    }
}

#[test]
fn test_core_tools_all_visible_to_llm() {
    let tools = core_tools();
    for tool in &tools {
        assert!(tool.visible_to_llm(), "{} should be visible to LLM", tool.name);
    }
}

#[test]
fn test_file_read_in_core_tools() {
    let tools = core_tools();
    let file_read = tools.iter().find(|t| t.name == "file_read");
    assert!(file_read.is_some(), "file_read should be in core tools");

    let metadata = file_read.unwrap();
    assert_eq!(metadata.category, ToolCategory::Core);
    assert!(!metadata.examples.is_empty(), "file_read should have examples");
    assert!(!metadata.not_examples.is_empty(), "file_read should have not_examples");
    assert!(!metadata.gated, "file_read should not be gated");
}

#[test]
fn test_file_search_in_core_tools() {
    let tools = core_tools();
    let file_search = tools.iter().find(|t| t.name == "file_search");
    assert!(file_search.is_some(), "file_search should be in core tools");

    let metadata = file_search.unwrap();
    assert_eq!(metadata.category, ToolCategory::Core);
    assert!(!metadata.examples.is_empty(), "file_search should have examples");
    assert!(!metadata.gated, "file_search should not be gated");
}

#[test]
fn test_splice_patch_in_core_tools() {
    let tools = core_tools();
    let splice_patch = tools.iter().find(|t| t.name == "splice_patch");
    assert!(splice_patch.is_some(), "splice_patch should be in core tools");

    let metadata = splice_patch.unwrap();
    assert_eq!(metadata.category, ToolCategory::Core);
    assert!(!metadata.examples.is_empty(), "splice_patch should have examples");
    assert!(!metadata.gated, "splice_patch should not be gated");
}

#[test]
fn test_bash_exec_in_core_tools() {
    let tools = core_tools();
    let bash_exec = tools.iter().find(|t| t.name == "bash_exec");
    assert!(bash_exec.is_some(), "bash_exec should be in core tools");

    let metadata = bash_exec.unwrap();
    assert_eq!(metadata.category, ToolCategory::Core);
    assert!(!metadata.examples.is_empty(), "bash_exec should have examples");
    assert!(!metadata.gated, "bash_exec should not be gated");
}

#[test]
fn test_display_text_in_core_tools() {
    let tools = core_tools();
    let display_text = tools.iter().find(|t| t.name == "display_text");
    assert!(display_text.is_some(), "display_text should be in core tools");

    let metadata = display_text.unwrap();
    assert_eq!(metadata.category, ToolCategory::Core);
    assert!(!metadata.examples.is_empty(), "display_text should have examples");
    assert!(!metadata.gated, "display_text should not be gated");
}

#[test]
fn test_file_read_examples_mention_file_read() {
    let tools = core_tools();
    let file_read = tools.iter().find(|t| t.name == "file_read").unwrap();

    for example in &file_read.examples {
        assert!(
            example.command.contains("file_read"),
            "Example command should use file_read: {}",
            example.command
        );
    }
}

#[test]
fn test_file_search_examples_mention_file_search() {
    let tools = core_tools();
    let file_search = tools.iter().find(|t| t.name == "file_search").unwrap();

    for example in &file_search.examples {
        assert!(
            example.command.contains("file_search"),
            "Example command should use file_search: {}",
            example.command
        );
    }
}

#[test]
fn test_splice_patch_examples_mention_splice_patch() {
    let tools = core_tools();
    let splice_patch = tools.iter().find(|t| t.name == "splice_patch").unwrap();

    for example in &splice_patch.examples {
        assert!(
            example.command.contains("splice_patch"),
            "Example command should use splice_patch: {}",
            example.command
        );
    }
}

#[test]
fn test_bash_exec_examples_mention_bash_exec() {
    let tools = core_tools();
    let bash_exec = tools.iter().find(|t| t.name == "bash_exec").unwrap();

    for example in &bash_exec.examples {
        assert!(
            example.command.contains("bash_exec"),
            "Example command should use bash_exec: {}",
            example.command
        );
    }
}

#[test]
fn test_display_text_examples_mention_display_text() {
    let tools = core_tools();
    let display_text = tools.iter().find(|t| t.name == "display_text").unwrap();

    for example in &display_text.examples {
        assert!(
            example.command.contains("display_text"),
            "Example command should use display_text: {}",
            example.command
        );
    }
}

#[test]
fn test_core_tools_total_token_cost_reasonable() {
    let tools = core_tools();
    let total_cost: usize = tools.iter().map(|t| t.estimate_token_cost()).sum();

    // Core tools should be under 1500 tokens total
    assert!(
        total_cost < 1500,
        "Core tools should total < 1500 tokens, got {}",
        total_cost
    );

    // But should be at least 500 tokens (we have meaningful descriptions)
    assert!(
        total_cost > 500,
        "Core tools should total > 500 tokens, got {}",
        total_cost
    );
}

#[test]
fn test_all_examples_have_reasoning() {
    let tools = core_tools();

    for tool in &tools {
        for example in &tool.examples {
            assert!(
                !example.reasoning.is_empty(),
                "{} example '{}' should have reasoning",
                tool.name,
                example.scenario
            );
        }

        for example in &tool.not_examples {
            assert!(
                !example.reasoning.is_empty(),
                "{} not_example '{}' should have reasoning",
                tool.name,
                example.scenario
            );
        }
    }
}

#[test]
fn test_all_examples_have_scenario_and_command() {
    let tools = core_tools();

    for tool in &tools {
        for example in &tool.examples {
            assert!(
                !example.scenario.is_empty(),
                "{} example should have scenario",
                tool.name
            );
            assert!(
                !example.command.is_empty(),
                "{} example should have command",
                tool.name
            );
        }

        for example in &tool.not_examples {
            assert!(
                !example.scenario.is_empty(),
                "{} not_example should have scenario",
                tool.name
            );
            assert!(
                !example.command.is_empty(),
                "{} not_example should have command",
                tool.name
            );
        }
    }
}
