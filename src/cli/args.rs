//! CLI argument parsing
//!
//! Parses command-line arguments according to Phase 4 specification:
//! - Modes: tui, plan, execute, evidence
//! - Options: --db-root, --plan-file, --json, --version, --help

use crate::cli::{Error, Result};

/// Parsed CLI arguments
#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    /// CLI mode (or None for default TUI)
    pub mode: Option<Mode>,

    /// Database root path (explicitly set or None for resolution)
    pub db_root: Option<String>,

    /// Plan file path (for execute mode)
    pub plan_file: Option<String>,

    /// JSON output flag
    pub json_output: bool,

    /// Skip bootstrap (expert mode / CI)
    pub no_bootstrap: bool,

    /// Show version and exit
    pub show_version: bool,

    /// Show help and exit
    pub show_help: bool,
}

/// CLI modes
#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    /// TUI mode (interactive terminal UI)
    Tui,

    /// Plan mode: generate plan from goal
    Plan { goal: String },

    /// Execute mode: execute a stored plan
    Execute { plan_file: String },

    /// Evidence mode: query evidence database
    Evidence {
        query: String,
        query_args: Vec<String>,
    },
}

/// Parse CLI arguments from std::env::args()
///
/// Grammar:
/// ```text
/// odincode [options] <mode> [mode-args]
///
/// MODES:
///   (no mode)    → TUI mode
///   tui           → TUI mode
///   plan <goal>   → Plan mode
///   execute       → Execute mode (requires --plan-file)
///   evidence <query> [args...] → Evidence mode
///
/// OPTIONS:
///   --db-root <path>     Database root
///   --plan-file <file>   Plan file (for execute mode)
///   --json              Output JSON
///   --no-bootstrap      Skip first-run setup (expert mode)
///   --version           Show version
///   --help              Show help
/// ```
pub fn parse_args<I: IntoIterator<Item = String>>(args: I) -> Result<Args> {
    let mut iter = args.into_iter();
    let _program = iter.next(); // Skip program name

    let mut args_out = Args {
        mode: None,
        db_root: None,
        plan_file: None,
        json_output: false,
        no_bootstrap: false,
        show_version: false,
        show_help: false,
    };

    let mut positional = Vec::new();

    // First pass: collect flags and positional args
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--version" | "-v" => {
                args_out.show_version = true;
            }
            "--help" | "-h" => {
                args_out.show_help = true;
            }
            "--json" => {
                args_out.json_output = true;
            }
            "--no-bootstrap" => {
                args_out.no_bootstrap = true;
            }
            "--db-root" => {
                let path = iter.next().ok_or_else(|| {
                    Error::MissingArgument("--db-root requires a path".to_string())
                })?;
                args_out.db_root = Some(path);
            }
            "--plan-file" => {
                let path = iter.next().ok_or_else(|| {
                    Error::MissingArgument("--plan-file requires a path".to_string())
                })?;
                args_out.plan_file = Some(path);
            }
            arg if arg.starts_with("--") => {
                return Err(Error::InvalidArgs(format!("Unknown option: {}", arg)));
            }
            other => {
                positional.push(other.to_string());
            }
        }
    }

    // Second pass: parse mode from positional args
    if !positional.is_empty() {
        args_out.mode = Some(parse_mode(&mut positional.into_iter(), &mut args_out)?);
    }

    Ok(args_out)
}

/// Parse mode from positional arguments
fn parse_mode<I: Iterator<Item = String>>(iter: &mut I, args: &mut Args) -> Result<Mode> {
    let first = iter
        .next()
        .ok_or_else(|| Error::InvalidArgs("Expected mode argument".to_string()))?;

    match first.as_str() {
        "tui" => Ok(Mode::Tui),
        "plan" => {
            // plan mode requires a goal string
            // If there are remaining args, join them as the goal
            // Otherwise, error
            let goal_parts: Vec<_> = iter.collect();
            if goal_parts.is_empty() {
                return Err(Error::MissingArgument(
                    "plan mode requires a goal".to_string(),
                ));
            }
            Ok(Mode::Plan {
                goal: goal_parts.join(" "),
            })
        }
        "execute" => {
            // execute mode: use plan_file from --plan-file flag if provided
            // Remaining args after "execute" are ignored (options should come before mode)
            let _: Vec<_> = iter.collect(); // Consume remaining args

            // Use plan_file from --plan-file flag, or empty string if not provided
            let plan_file = args.plan_file.take().unwrap_or_default();

            Ok(Mode::Execute { plan_file })
        }
        "evidence" => {
            // evidence mode: Q1-Q8 queries
            let query = iter.next().ok_or_else(|| {
                Error::MissingArgument("evidence mode requires a query (Q1-Q8)".to_string())
            })?;

            let query_args: Vec<_> = iter.collect();
            Ok(Mode::Evidence { query, query_args })
        }
        other => Err(Error::UnknownMode(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_args() {
        let args = parse_args(vec!["odincode".to_string()]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert!(parsed.mode.is_none());
        assert!(!parsed.show_version);
        assert!(!parsed.show_help);
    }

    #[test]
    fn test_parse_version_flag() {
        let args = parse_args(vec!["odincode".to_string(), "--version".to_string()]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert!(parsed.show_version);
    }

    #[test]
    fn test_parse_help_flag() {
        let args = parse_args(vec!["odincode".to_string(), "--help".to_string()]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert!(parsed.show_help);
    }

    #[test]
    fn test_parse_tui_mode() {
        let args = parse_args(vec!["odincode".to_string(), "tui".to_string()]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert_eq!(parsed.mode, Some(Mode::Tui));
    }

    #[test]
    fn test_parse_plan_mode() {
        let args = parse_args(vec![
            "odincode".to_string(),
            "plan".to_string(),
            "read".to_string(),
            "src/lib.rs".to_string(),
        ]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert_eq!(
            parsed.mode,
            Some(Mode::Plan {
                goal: "read src/lib.rs".to_string()
            })
        );
    }

    #[test]
    fn test_parse_evidence_mode() {
        let args = parse_args(vec![
            "odincode".to_string(),
            "evidence".to_string(),
            "Q1".to_string(),
            "file_read".to_string(),
        ]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert_eq!(
            parsed.mode,
            Some(Mode::Evidence {
                query: "Q1".to_string(),
                query_args: vec!["file_read".to_string()],
            })
        );
    }

    #[test]
    fn test_parse_db_root_option() {
        let args = parse_args(vec![
            "odincode".to_string(),
            "--db-root".to_string(),
            "/tmp/test".to_string(),
        ]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert_eq!(parsed.db_root, Some("/tmp/test".to_string()));
    }

    #[test]
    fn test_parse_json_flag() {
        let args = parse_args(vec![
            "odincode".to_string(),
            "--json".to_string(),
            "evidence".to_string(),
            "Q1".to_string(),
            "file_read".to_string(),
        ]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert!(parsed.json_output);
    }

    #[test]
    fn test_parse_unknown_mode() {
        let args = parse_args(vec!["odincode".to_string(), "unknown_mode".to_string()]);
        assert!(args.is_err());
    }

    #[test]
    fn test_parse_no_bootstrap_flag() {
        let args = parse_args(vec!["odincode".to_string(), "--no-bootstrap".to_string()]);
        assert!(args.is_ok());
        let parsed = args.unwrap();
        assert!(parsed.no_bootstrap);
    }
}
