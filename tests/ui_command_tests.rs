//! UI command tests — Phase 1 command parsing
//!
//! These tests verify deterministic command parsing.
//!
//! Phase 1 Command Model:
//! - Commands start with ':' to distinguish from text
//! - Supported: :open, :read, :lsp, :evidence, :quit
//! - Deterministic parsing — no ambiguity

/// Test: Command parser handles empty input
#[test]
fn test_parse_empty() {
    let input = "";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::None);
}

/// Test: Command parser handles whitespace-only input
#[test]
fn test_parse_whitespace() {
    let input = "   \t  ";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::None);
}

/// Test: Command parser handles text without colon prefix
#[test]
fn test_parse_no_colon_prefix() {
    let input = "some text without colon";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::None);
}

/// Test: Command parser handles colon-only input
#[test]
fn test_parse_colon_only() {
    let input = ":";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::None);
}

/// Test: Command parser handles :quit (short form)
#[test]
fn test_parse_quit_short() {
    let input = ":q";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Quit);
}

/// Test: Command parser handles :quit (long form)
#[test]
fn test_parse_quit_long() {
    let input = ":quit";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Quit);
}

/// Test: Command parser handles :quit with extra whitespace
#[test]
fn test_parse_quit_with_whitespace() {
    let input = ":quit   ";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Quit);
}

/// Test: Command parser handles :open (short form)
#[test]
fn test_parse_open_short() {
    let input = ":o src/lib.rs";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Open("src/lib.rs".to_string()));
}

/// Test: Command parser handles :open (long form)
#[test]
fn test_parse_open_long() {
    let input = ":open src/lib.rs";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Open("src/lib.rs".to_string()));
}

/// Test: Command parser handles :open with quoted path (containing spaces)
#[test]
fn test_parse_open_quoted() {
    let input = r#":open "path with spaces/lib.rs""#;
    let parsed = parse_command(input);
    assert_eq!(
        parsed,
        ParsedCommand::Open("path with spaces/lib.rs".to_string())
    );
}

/// Test: Command parser handles :open without arguments
#[test]
fn test_parse_open_no_args() {
    let input = ":open";
    let parsed = parse_command(input);
    assert_eq!(
        parsed,
        ParsedCommand::Error("Missing argument for :open".to_string())
    );
}

/// Test: Command parser handles :read (short form)
#[test]
fn test_parse_read_short() {
    let input = ":r src/main.rs";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Read("src/main.rs".to_string()));
}

/// Test: Command parser handles :read (long form)
#[test]
fn test_parse_read_long() {
    let input = ":read src/main.rs";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Read("src/main.rs".to_string()));
}

/// Test: Command parser handles :lsp with explicit path
#[test]
fn test_parse_lsp_with_path() {
    let input = ":lsp /path/to/project";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Lsp("/path/to/project".to_string()));
}

/// Test: Command parser handles :lsp without path (default to current dir)
#[test]
fn test_parse_lsp_no_path() {
    let input = ":lsp";
    let parsed = parse_command(input);
    assert_eq!(parsed, ParsedCommand::Lsp(".".to_string()));
}

/// Test: Command parser handles :evidence list command
#[test]
fn test_parse_evidence_list() {
    let input = ":evidence list splice_patch";
    let parsed = parse_command(input);
    assert_eq!(
        parsed,
        ParsedCommand::Evidence {
            query: "list".to_string(),
            args: vec!["splice_patch".to_string()],
        }
    );
}

/// Test: Command parser handles :evidence failures command
#[test]
fn test_parse_evidence_failures() {
    let input = ":evidence failures splice_patch";
    let parsed = parse_command(input);
    assert_eq!(
        parsed,
        ParsedCommand::Evidence {
            query: "failures".to_string(),
            args: vec!["splice_patch".to_string()],
        }
    );
}

/// Test: Command parser handles :evidence with multiple args
#[test]
fn test_parse_evidence_with_multiple_args() {
    let input = ":evidence list splice_patch --limit 10";
    let parsed = parse_command(input);
    assert_eq!(
        parsed,
        ParsedCommand::Evidence {
            query: "list".to_string(),
            args: vec![
                "splice_patch".to_string(),
                "--limit".to_string(),
                "10".to_string()
            ],
        }
    );
}

/// Test: Command parser handles :evidence short form
#[test]
fn test_parse_evidence_short() {
    let input = ":ev list splice_patch";
    let parsed = parse_command(input);
    assert_eq!(
        parsed,
        ParsedCommand::Evidence {
            query: "list".to_string(),
            args: vec!["splice_patch".to_string()],
        }
    );
}

/// Test: Command parser handles unknown command
#[test]
fn test_parse_unknown_command() {
    let input = ":unknown_command args";
    let parsed = parse_command(input);
    assert_eq!(
        parsed,
        ParsedCommand::Error("Unknown command: unknown_command".to_string())
    );
}

/// Test: Command parser is deterministic (same input → same output)
#[test]
fn test_parse_deterministic() {
    let input = ":open src/lib.rs";

    let parsed1 = parse_command(input);
    let parsed2 = parse_command(input);

    assert_eq!(parsed1, parsed2, "Same input must produce same output");
}

/// Test: Multiple parses of different commands are independent
#[test]
fn test_parse_independence() {
    let cmd1 = parse_command(":open src/lib.rs");
    let cmd2 = parse_command(":read src/main.rs");
    let cmd3 = parse_command(":quit");

    assert_eq!(cmd1, ParsedCommand::Open("src/lib.rs".to_string()));
    assert_eq!(cmd2, ParsedCommand::Read("src/main.rs".to_string()));
    assert_eq!(cmd3, ParsedCommand::Quit);
}

//
// Command representation and parser (will be replaced by ui::input module)
//

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedCommand {
    None,
    Quit,
    Open(String),
    Read(String),
    Lsp(String),
    Evidence { query: String, args: Vec<String> },
    Error(String),
}

fn parse_command(input: &str) -> ParsedCommand {
    let input = input.trim();
    if input.is_empty() {
        return ParsedCommand::None;
    }

    if !input.starts_with(':') {
        return ParsedCommand::None;
    }

    let rest = &input[1..];
    if rest.is_empty() {
        return ParsedCommand::None;
    }

    // Parse with support for quoted arguments
    let parts = parse_arguments(rest);

    if parts.is_empty() {
        return ParsedCommand::None;
    }

    match parts[0].as_str() {
        "quit" | "q" => ParsedCommand::Quit,
        "open" | "o" => {
            if parts.len() > 1 {
                ParsedCommand::Open(parts[1].clone())
            } else {
                ParsedCommand::Error("Missing argument for :open".to_string())
            }
        }
        "read" | "r" => {
            if parts.len() > 1 {
                ParsedCommand::Read(parts[1].clone())
            } else {
                ParsedCommand::Error("Missing argument for :read".to_string())
            }
        }
        "lsp" => {
            if parts.len() > 1 {
                ParsedCommand::Lsp(parts[1].clone())
            } else {
                ParsedCommand::Lsp(".".to_string())
            }
        }
        "evidence" | "ev" => {
            if parts.len() > 1 {
                let query = parts[1].clone();
                let args: Vec<String> = parts[2..].to_vec();
                ParsedCommand::Evidence { query, args }
            } else {
                ParsedCommand::Error("Missing query for :evidence".to_string())
            }
        }
        unknown => ParsedCommand::Error(format!("Unknown command: {}", unknown)),
    }
}

/// Parse command arguments with support for quoted strings
fn parse_arguments(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let chars = input.chars().peekable();

    for c in chars {
        match c {
            '"' => {
                in_quotes = !in_quotes;
            }
            ' ' | '\t' if !in_quotes => {
                if !current.is_empty() {
                    args.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        args.push(current);
    }

    args
}
