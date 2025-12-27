//! Phase 10.1: Tool metadata tests
//!
//! TDD approach: Tests for tool metadata types.

use odincode::tools::metadata::{
    DiscoveryResult, DiscoveryTrigger, SpecializedTool,
    ToolCategory, ToolExample, ToolMetadata,
};
use serde_json;

#[test]
fn test_tool_example_serialization() {
    let example = ToolExample {
        scenario: "Read a specific file".to_string(),
        command: "file_read src/lib.rs".to_string(),
        reasoning: "Direct file access is fastest".to_string(),
    };

    let json = serde_json::to_string(&example).unwrap();
    let parsed: ToolExample = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.scenario, example.scenario);
    assert_eq!(parsed.command, example.command);
    assert_eq!(parsed.reasoning, example.reasoning);
}

#[test]
fn test_tool_metadata_serialization() {
    let metadata = ToolMetadata {
        name: "file_read".to_string(),
        category: ToolCategory::Core,
        description: "Read file contents".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Read known file".to_string(),
                command: "file_read src/lib.rs".to_string(),
                reasoning: "Direct access".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Find files by pattern".to_string(),
                command: "file_search \"**/*.rs\"".to_string(),
                reasoning: "Use file_search instead".to_string(),
            },
        ],
        token_cost: 150,
        gated: false,
    };

    // Verify serializable
    let json = serde_json::to_string(&metadata).unwrap();
    let parsed: ToolMetadata = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, "file_read");
    assert_eq!(parsed.category, ToolCategory::Core);
    assert_eq!(parsed.examples.len(), 1);
    assert_eq!(parsed.not_examples.len(), 1);
    assert_eq!(parsed.token_cost, 150);
    assert_eq!(parsed.gated, false);
}

#[test]
fn test_tool_category_core_serialization() {
    let category = ToolCategory::Core;

    let json = serde_json::to_string(&category).unwrap();
    assert_eq!(json, "\"Core\"");

    let parsed: ToolCategory = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, ToolCategory::Core);
}

#[test]
fn test_tool_category_specialized_serialization() {
    let category = ToolCategory::Specialized;

    let json = serde_json::to_string(&category).unwrap();
    assert_eq!(json, "\"Specialized\"");

    let parsed: ToolCategory = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, ToolCategory::Specialized);
}

#[test]
fn test_tool_category_internal_serialization() {
    let category = ToolCategory::Internal;

    let json = serde_json::to_string(&category).unwrap();
    assert_eq!(json, "\"Internal\"");

    let parsed: ToolCategory = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, ToolCategory::Internal);
}

#[test]
fn test_tool_metadata_validation() {
    // Core tool with no examples should be invalid
    let metadata = ToolMetadata {
        name: "file_read".to_string(),
        category: ToolCategory::Core,
        description: "Read file contents".to_string(),
        examples: vec![],
        not_examples: vec![],
        token_cost: 150,
        gated: false,
    };

    // Validation: Core tools MUST have examples
    assert!(!metadata.is_valid(), "Core tool with no examples should be invalid");
}

#[test]
fn test_tool_metadata_valid_with_examples() {
    let metadata = ToolMetadata {
        name: "file_read".to_string(),
        category: ToolCategory::Core,
        description: "Read file contents".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Read known file".to_string(),
                command: "file_read src/lib.rs".to_string(),
                reasoning: "Direct access".to_string(),
            },
        ],
        not_examples: vec![],
        token_cost: 150,
        gated: false,
    };

    assert!(metadata.is_valid());
}

#[test]
fn test_tool_token_cost_calculation() {
    let metadata = ToolMetadata {
        name: "test_tool".to_string(),
        category: ToolCategory::Core,
        description: "A test tool".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Scenario 1".to_string(),
                command: "command 1".to_string(),
                reasoning: "Reasoning 1".to_string(),
            },
            ToolExample {
                scenario: "Scenario 2".to_string(),
                command: "command 2".to_string(),
                reasoning: "Reasoning 2".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Don't use for X".to_string(),
                command: "avoid".to_string(),
                reasoning: "Why not".to_string(),
            },
        ],
        token_cost: 100,
        gated: false,
    };

    // Token cost includes description + examples + not_examples
    let estimated = metadata.estimate_token_cost();
    assert!(estimated > 100, "Base cost should be included");
}

#[test]
fn test_internal_tools_hidden_from_llm() {
    let metadata = ToolMetadata {
        name: "approval_granted".to_string(),
        category: ToolCategory::Internal,
        description: "Internal approval tracking".to_string(),
        examples: vec![],
        not_examples: vec![],
        token_cost: 0,
        gated: false,
    };

    assert!(!metadata.visible_to_llm(), "Internal tools should not be visible to LLM");
}

#[test]
fn test_core_tools_visible_to_llm() {
    let metadata = ToolMetadata {
        name: "file_read".to_string(),
        category: ToolCategory::Core,
        description: "Read file contents".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Read file".to_string(),
                command: "file_read x.rs".to_string(),
                reasoning: "Direct".to_string(),
            },
        ],
        not_examples: vec![],
        token_cost: 150,
        gated: false,
    };

    assert!(metadata.visible_to_llm(), "Core tools should be visible to LLM");
}

#[test]
fn test_gated_tool_flag() {
    let metadata = ToolMetadata {
        name: "file_write".to_string(),
        category: ToolCategory::Core,
        description: "Write to file".to_string(),
        examples: vec![],
        not_examples: vec![],
        token_cost: 200,
        gated: true,  // GATED tool
    };

    assert!(metadata.gated, "file_write should be gated");
}

#[test]
fn test_specialized_tool_builder() {
    let tool = SpecializedTool::new("test_tool", "A test tool", 100)
        .with_example("Test scenario", "test command", "test reasoning")
        .with_not_example("Don't use for X", "avoid", "why not")
        .with_trigger(DiscoveryTrigger::Keyword("test".to_string()));

    assert_eq!(tool.metadata.name, "test_tool");
    assert_eq!(tool.metadata.examples.len(), 1);
    assert_eq!(tool.metadata.not_examples.len(), 1);
    assert_eq!(tool.triggers.len(), 1);
}

#[test]
fn test_specialized_tool_keyword_discovery() {
    let tool = SpecializedTool::new("git_log", "Show git history", 150)
        .with_trigger(DiscoveryTrigger::Keyword("history".to_string()))
        .with_trigger(DiscoveryTrigger::Keyword("commits".to_string()));

    assert!(tool.should_discover("show me the git history", &[]));
    assert!(tool.should_discover("list all commits", &[]));
    assert!(!tool.should_discover("read a file", &[]));
}

#[test]
fn test_discovery_result_visible_tools() {
    let result = DiscoveryResult {
        core: vec![
            ToolMetadata {
                name: "file_read".to_string(),
                category: ToolCategory::Core,
                description: "Read file".to_string(),
                examples: vec![],
                not_examples: vec![],
                token_cost: 150,
                gated: false,
            },
        ],
        specialized: vec![
            ToolMetadata {
                name: "approval_granted".to_string(),
                category: ToolCategory::Internal,
                description: "Internal".to_string(),
                examples: vec![],
                not_examples: vec![],
                token_cost: 0,
                gated: false,
            },
        ],
        total_token_cost: 150,
    };

    let visible = result.visible_tools();
    assert_eq!(visible.len(), 1);  // Only file_read, not internal tool
    assert_eq!(visible[0].name, "file_read");
}

#[test]
fn test_discovery_result_tool_names() {
    let result = DiscoveryResult {
        core: vec![
            ToolMetadata {
                name: "file_read".to_string(),
                category: ToolCategory::Core,
                description: "Read".to_string(),
                examples: vec![],
                not_examples: vec![],
                token_cost: 150,
                gated: false,
            },
            ToolMetadata {
                name: "bash_exec".to_string(),
                category: ToolCategory::Core,
                description: "Run commands".to_string(),
                examples: vec![],
                not_examples: vec![],
                token_cost: 250,
                gated: false,
            },
        ],
        specialized: vec![],
        total_token_cost: 400,
    };

    let names = result.tool_names();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"file_read".to_string()));
    assert!(names.contains(&"bash_exec".to_string()));
}
