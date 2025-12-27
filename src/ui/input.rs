//! Command parsing for TUI — 3-way router architecture
//!
//! INPUT ROUTING (strict 3-way):
//! A) COMMAND: Input starts with "/"
//!    - Executes immediately, bypasses planner
//!    - Examples: /quit, /help, /open <path>
//!
//! B) CHAT: Default (no "/" prefix)
//!    - Sent to LLM as conversational text
//!    - NO approval for display_text responses
//!    - Context recorded in memory
//!
//! C) PLAN: Explicit plan commands only
//!    - /plan, /apply — converts chat context to structured plan
//!    - Requires approval (y/n) before execution
//!
//! EXIT HANDLING (hard requirement):
//! - /quit, /q, /exit work from ANY state
//! - Bypass planner, LLM, memory, approval
//! - Ctrl+C exits immediately

/// Parsed command result
///
/// Phase 8 routing: Commands use "/" prefix, chat is default.
/// Phase 9.5: Added TraceOn, TraceOff, Cancel, Status, Continue commands
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    None,
    Quit,         // /quit, /q, /exit — exits immediately from any state
    Open(String), // /open <path>
    Read(String), // /read <path>
    Lsp(String),  // /lsp [path]
    Help,         // /help
    Find(String), // /find <pattern>
    Plan,         // /plan — explicit plan trigger (requires approval)
    Apply,        // /apply — execute pending plan (requires approval)
    TraceOn,      // /trace — show trace viewer (Phase 9.5)
    TraceOff,     // /trace off — hide trace viewer (Phase 9.5)
    Cancel,       // /cancel — cancel current tool (Phase 9.5)
    Status,       // /status — show current tool state (Phase 9.5)
    Continue,     // /continue — resume after approval (Phase 9.5)
    Chat(String), // Default: conversational text (LLM responds directly)
}

/// Parse command input string into Command
///
/// Phase 8 routing (strict 3-way):
/// 1. Starts with "/" → Parse as command (execute immediately)
/// 2. "/" is not present → Treat as Chat (conversational)
///
/// # Examples
/// ```
/// use odincode::ui::input::{parse_command, Command};
///
/// // Commands (with "/")
/// assert_eq!(parse_command("/quit"), Command::Quit);
/// assert_eq!(parse_command("/q"), Command::Quit);
/// assert_eq!(parse_command("/exit"), Command::Quit);
/// assert_eq!(parse_command("/help"), Command::Help);
///
/// // Plan commands (explicit triggers)
/// assert_eq!(parse_command("/plan"), Command::Plan);
/// assert_eq!(parse_command("/apply"), Command::Apply);
///
/// // Chat (default, no "/")
/// assert!(matches!(parse_command("read src/lib.rs"), Command::Chat(_)));
/// assert!(matches!(parse_command("fix the error"), Command::Chat(_)));
/// assert!(matches!(parse_command(":help"), Command::Chat(_))); // ":" is chat
/// ```
pub fn parse_command(input: &str) -> Command {
    let input = input.trim();
    if input.is_empty() {
        return Command::None;
    }

    // Phase 8: Commands start with "/" ONLY
    // ":" is treated as plain text (chat)
    if !input.starts_with('/') {
        return Command::Chat(input.to_string());
    }

    // After "/" comes command name
    let rest = &input[1..];

    // "/" alone is not a command
    if rest.is_empty() {
        return Command::None;
    }

    // Leading space after "/" means invalid syntax
    if rest.starts_with(' ') || rest.starts_with('\t') {
        return Command::None;
    }

    // Split into parts (first word is command, rest are args)
    let parts: Vec<&str> = rest.splitn(2, |c: char| [' ', '\t'].contains(&c)).collect();

    match parts[0] {
        // Exit commands (work from any state, no args allowed)
        "quit" | "q" | "exit" => {
            if parts.len() == 1 {
                Command::Quit
            } else {
                Command::None
            }
        }
        "open" | "o" => {
            if parts.len() > 1 {
                Command::Open(parts[1].to_string())
            } else {
                Command::None
            }
        }
        "read" | "r" => {
            if parts.len() > 1 {
                Command::Read(parts[1].to_string())
            } else {
                Command::None
            }
        }
        "lsp" => {
            if parts.len() > 1 {
                Command::Lsp(parts[1].to_string())
            } else {
                Command::Lsp(".".to_string())
            }
        }
        "help" | "h" => Command::Help,
        "find" | "f" => {
            if parts.len() > 1 {
                Command::Find(parts[1].to_string())
            } else {
                Command::None
            }
        }
        "plan" | "p" => Command::Plan,
        "apply" => Command::Apply,
        // Phase 9.5: Tool execution and trace commands
        "trace" => {
            // Check for "off" argument
            if parts.len() > 1 && parts[1] == "off" {
                Command::TraceOff
            } else {
                Command::TraceOn
            }
        }
        "cancel" => Command::Cancel,
        "status" => Command::Status,
        "continue" => Command::Continue,
        _ => Command::None,
    }
}

/// Render help text for TUI
///
/// Phase 9.5: Shows "/" commands, no implicit key bindings
pub fn render_help() -> String {
    r#"OdinCode v0.0.1 — AI-Powered Refactoring Assistant

INPUT MODES:
    Type anything to chat with LLM (no "/" prefix needed)
    Responses are shown immediately — no approval for chat

KEYBOARD SHORTCUTS:
    Up/Down             Scroll chat 1 line
    PageUp/PageDown     Scroll chat 10 lines
    Home/End            Jump to top/bottom
    Tab                 Cycle through panels
    Ctrl+C              Exit immediately

COMMANDS (start with "/"):
    /quit, /q, /exit    Quit immediately (works from any state)
    /open <path>        Open file in code view
    /read <path>        Read file contents
    /lsp [path]         Run cargo check (default: current dir)
    /find <pattern>     Find symbols or files
    /help               Show this help message

TOOL EXECUTION (Phase 9.5):
    /trace              Show execution trace viewer
    /trace off         Hide execution trace viewer
    /cancel             Cancel current tool execution
    /status             Show current tool state
    /continue           Resume after approval

PLAN MODE (requires approval):
    /plan               Convert chat context to structured plan
    /apply              Execute pending plan (press 'y' to approve)

CHAT EXAMPLES:
    "read src/lib.rs"
    "fix the type error in main"
    "find all usages of MyStruct"
    ":help"              <- This is CHAT, not a command!
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_command(""), Command::None);
    }

    #[test]
    fn test_parse_slash_alone() {
        assert_eq!(parse_command("/"), Command::None);
    }

    #[test]
    fn test_parse_quit() {
        assert_eq!(parse_command("/quit"), Command::Quit);
        assert_eq!(parse_command("/q"), Command::Quit);
        assert_eq!(parse_command("/exit"), Command::Quit);
    }

    #[test]
    fn test_parse_quit_rejects_args() {
        // Exit commands must not have arguments
        assert_eq!(parse_command("/quit now"), Command::None);
        assert_eq!(parse_command("/q please"), Command::None);
    }

    #[test]
    fn test_colon_is_chat_not_command() {
        // Phase 8: ":" is plain text, NOT a command
        assert!(matches!(parse_command(":help"), Command::Chat(_)));
        assert!(matches!(parse_command(":quit"), Command::Chat(_)));
        assert!(matches!(parse_command(":q"), Command::Chat(_)));
    }

    #[test]
    fn test_chat_default() {
        // Anything without "/" is chat
        assert!(matches!(parse_command("hello"), Command::Chat(_)));
        assert!(matches!(parse_command("read src/lib.rs"), Command::Chat(_)));
        assert!(matches!(parse_command(":help"), Command::Chat(_)));
    }

    #[test]
    fn test_parse_open() {
        assert_eq!(
            parse_command("/open src/lib.rs"),
            Command::Open("src/lib.rs".to_string())
        );
        assert_eq!(
            parse_command("/o src/lib.rs"),
            Command::Open("src/lib.rs".to_string())
        );
    }

    #[test]
    fn test_parse_lsp() {
        assert_eq!(parse_command("/lsp"), Command::Lsp(".".to_string()));
        assert_eq!(
            parse_command("/lsp /path/to/project"),
            Command::Lsp("/path/to/project".to_string())
        );
    }

    #[test]
    fn test_parse_help() {
        assert_eq!(parse_command("/help"), Command::Help);
        assert_eq!(parse_command("/h"), Command::Help);
    }

    #[test]
    fn test_parse_find() {
        assert_eq!(
            parse_command("/find main"),
            Command::Find("main".to_string())
        );
    }

    #[test]
    fn test_parse_plan() {
        assert_eq!(parse_command("/plan"), Command::Plan);
        assert_eq!(parse_command("/p"), Command::Plan);
    }

    #[test]
    fn test_parse_apply() {
        assert_eq!(parse_command("/apply"), Command::Apply);
    }

    #[test]
    fn test_leading_space_after_slash_is_invalid() {
        // "/ quit" should NOT match "/quit"
        assert_eq!(parse_command("/ quit"), Command::None);
        assert_eq!(parse_command("/\tquit"), Command::None);
    }

    #[test]
    fn test_unknown_command_is_none() {
        assert_eq!(parse_command("/unknown"), Command::None);
        assert_eq!(parse_command("/xyz"), Command::None);
    }
}
