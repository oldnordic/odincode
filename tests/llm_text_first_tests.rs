//! LLM Text-First Contract Tests (Phase 7.3 → Phase 8)
//!
//! Regression tests for text-first LLM contract alignment.

use odincode::llm::planner::parse_plan;
use odincode::llm::types::Intent;

#[test]
fn test_plain_text_response_does_not_error() {
    let plain_text = "Hello! I can help you with that code.";
    let result = parse_plan(plain_text);

    assert!(
        result.is_ok(),
        "Plain text should be accepted, not rejected"
    );

    let plan = result.unwrap();
    assert_eq!(plan.intent, Intent::Explain);
}

#[test]
fn test_json_response_still_works() {
    let json_plan = r#"{
        "plan_id": "plan_123",
        "intent": "READ",
        "steps": [{
            "step_id": "step_1",
            "tool": "file_read",
            "arguments": {"path": "src/lib.rs"},
            "precondition": "file exists"
        }],
        "evidence_referenced": []
    }"#;

    let result = parse_plan(json_plan);
    assert!(result.is_ok());

    let plan = result.unwrap();
    assert_eq!(plan.plan_id, "plan_123");
    assert_eq!(plan.intent, Intent::Read);
    assert_eq!(plan.steps.len(), 1);
}

#[test]
fn test_json_guard_only_parses_if_starts_with_brace() {
    let text_with_json_later = "Here's my response:\n{\"plan_id\": \"test\"}";
    let result = parse_plan(text_with_json_later);

    assert!(result.is_ok());

    let plan = result.unwrap();
    assert_eq!(plan.intent, Intent::Explain);
    assert_eq!(plan.steps[0].tool, "display_text");
}

#[test]
fn test_malformed_json_does_not_crash() {
    let malformed = "{this is not valid json but it's a reasonable response";
    let result = parse_plan(malformed);

    assert!(result.is_ok());

    let plan = result.unwrap();
    assert_eq!(plan.intent, Intent::Explain);
}

#[test]
fn test_empty_response_handled_gracefully() {
    let empty = "";
    let result = parse_plan(empty);

    assert!(result.is_ok());

    let plan = result.unwrap();
    assert_eq!(plan.intent, Intent::Read);
}

#[test]
fn test_markdown_code_block_accepted() {
    let markdown_json = "```json\n{\"plan_id\": \"test\", \"intent\": \"READ\", \"steps\": [], \"evidence_referenced\": []}\n```";
    let result = parse_plan(markdown_json);

    assert!(result.is_ok());
}

#[test]
fn test_slash_commands_work() {
    // Phase 8: "/" prefix commands, NOT ":"
    use odincode::ui::input::{parse_command, Command};

    let cmd = parse_command("/quit");
    assert!(matches!(cmd, Command::Quit));

    let cmd = parse_command("/q");
    assert!(matches!(cmd, Command::Quit));

    let cmd = parse_command("/exit");
    assert!(matches!(cmd, Command::Quit));

    let cmd = parse_command("/help");
    assert!(matches!(cmd, Command::Help));

    let cmd = parse_command("/open src/lib.rs");
    assert!(matches!(cmd, Command::Open(_)));
}

#[test]
fn test_colon_is_chat_not_command() {
    // Phase 8: ":" is plain text, NOT a command prefix
    use odincode::ui::input::{parse_command, Command};

    let cmd = parse_command(":q");
    assert!(matches!(cmd, Command::Chat(_)));

    let cmd = parse_command(":help");
    assert!(matches!(cmd, Command::Chat(_)));

    let cmd = parse_command(":quit");
    assert!(matches!(cmd, Command::Chat(_)));
}

#[test]
fn test_natural_language_is_chat() {
    // Phase 8: No "/" prefix = chat
    use odincode::ui::input::{parse_command, Command};

    let cmd = parse_command("hello world");
    assert!(matches!(cmd, Command::Chat(_)));

    let cmd = parse_command("read src/lib.rs");
    assert!(matches!(cmd, Command::Chat(_)));
}

#[test]
fn test_llm_response_with_leading_whitespace() {
    let json_with_leading_space = "   \n  {\"plan_id\": \"test\", \"intent\": \"READ\", \"steps\": [], \"evidence_referenced\": []}";
    let result = parse_plan(json_with_leading_space);

    assert!(result.is_ok());

    let plan = result.unwrap();
    assert_eq!(plan.plan_id, "test");
}
