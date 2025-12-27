//! First-run bootstrap (Phase 6)
//!
//! Ensures OdinCode infrastructure exists before normal operation.
//!
//! ## Behavior
//!
//! - Interactive mode: Creates db_root directory, prompts for config
//! - Non-interactive mode: Fails with clear error if config missing
//! - With --no-bootstrap: Skips all checks, strict errors only
//!
//! ## What Bootstrap Does
//!
//! - Creates db_root directory if missing (interactive only)
//! - Detects missing config.toml
//! - Calls LLM preflight for first-time setup (interactive only)
//! - Prints guidance for codegraph.db (external dependency)
//!
//! ## What Bootstrap Does NOT Do
//!
//! - Create codegraph.db (external tool responsibility)
//! - Write execution evidence entries
//! - Log execution artifacts
//! - Run Magellan automatically

use crate::cli::preflight::{run_llm_preflight, PreflightOutcome};
use crate::cli::{Error, Result};
use std::io::{self, Write};
use std::path::Path;

/// Bootstrap status result
#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapStatus {
    /// Bootstrap complete, proceed to normal operation
    Ready,
    /// Config was written, user should restart
    NeedsRestart,
}

/// Config file location: <db_root>/config.toml
const CONFIG_FILE: &str = "config.toml";

/// Code graph database file
const CODEGRAPH_DB: &str = "codegraph.db";

/// Ensure infrastructure exists at given db_root
///
/// # Arguments
/// * `db_root` - Resolved database root path
/// * `interactive` - Whether this is an interactive session (TUI vs CLI mode)
/// * `allow_prompt` - Whether to allow prompting for config (true if stdin available)
/// * `no_bootstrap` - Skip bootstrap entirely (expert mode / CI)
///
/// # Behavior
/// - If `no_bootstrap`: Skip all checks, return Ready immediately
/// - If `interactive`: Create directories, run LLM preflight if needed
/// - If `allow_prompt`: Run preflight even if not interactive (for CLI with piped stdin)
/// - If non-interactive without allow_prompt: Fail with clear error if config missing
///
/// # Returns
/// * `Ok(BootstrapStatus::Ready)` - Infrastructure ready, proceed
/// * `Ok(BootstrapStatus::NeedsRestart)` - Config written, restart required
/// * `Err(Error)` - Fatal error (non-interactive without config, I/O failure)
pub fn ensure_infrastructure(
    db_root: &Path,
    interactive: bool,
    allow_prompt: bool,
    no_bootstrap: bool,
) -> Result<BootstrapStatus> {
    // Expert mode: skip all bootstrap logic
    if no_bootstrap {
        return Ok(BootstrapStatus::Ready);
    }

    // Create db_root directory if missing (interactive only)
    if !db_root.exists() {
        if interactive {
            create_db_root(db_root)?;
        } else {
            return Err(Error::InvalidArgs(format!(
                "OdinCode is not initialized.\n\
                 db_root '{}' does not exist.\n\
                 Run `odincode` interactively or create the directory.",
                db_root.display()
            )));
        }
    }

    // Check for config file
    let config_path = db_root.join(CONFIG_FILE);
    if !config_path.exists() {
        if interactive || allow_prompt {
            // TUI mode: run preflight wizard
            // CLI mode with allow_prompt: run preflight if config missing
            match run_llm_preflight(db_root)? {
                PreflightOutcome::Exit => {
                    return Ok(BootstrapStatus::NeedsRestart);
                }
                PreflightOutcome::Proceed => {
                    // Config created, continue
                }
            }
        } else {
            // Non-interactive mode without prompt: fail with clear error
            return Err(Error::InvalidArgs(format!(
                "OdinCode is not initialized.\n\
                 Config file not found: {}\n\
                 Run `odincode` interactively or create a config file.",
                config_path.display()
            )));
        }
    }

    // Check for codegraph.db (external dependency, NOT created by bootstrap)
    let codegraph_path = db_root.join(CODEGRAPH_DB);
    if !codegraph_path.exists() {
        // Print guidance but don't fail
        // degraded mode is acceptable (no symbol queries)
        if interactive {
            println!();
            println!("Symbol navigation is unavailable.");
            println!("To enable code search, run:");
            println!(
                "  magellan watch --root . --db {}",
                codegraph_path.display()
            );
            println!();
            io::stdout().flush()?;
        }
        // Non-interactive modes continue without codegraph.db
        // (evidence queries will have limited functionality)
    }

    // Print first-run completion message if this is a fresh setup
    if interactive {
        print_completion_message();
    }

    Ok(BootstrapStatus::Ready)
}

/// Create db_root directory
fn create_db_root(db_root: &Path) -> Result<()> {
    println!();
    println!("OdinCode — First-time setup");
    println!();
    println!("Creating database directory: {}", db_root.display());

    std::fs::create_dir_all(db_root).map_err(Error::Io)?;

    println!("Database directory created.");
    println!();

    Ok(())
}

/// Print first-run completion message
fn print_completion_message() {
    println!("Setup complete.");
    println!("You can now:");
    println!("  • Ask questions about your code");
    println!("  • Request refactoring plans");
    println!("  • Run diagnostics");
    println!("  • Search symbols (if Magellan is running)");
    println!();
    println!("Type your request or :help to begin.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_no_bootstrap_skips_all_checks() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");

        // --no-bootstrap should skip everything, even if dir doesn't exist
        let result = ensure_infrastructure(&nonexistent, true, true, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BootstrapStatus::Ready);
    }

    #[test]
    fn test_interactive_creates_db_root() {
        let temp_dir = TempDir::new().unwrap();
        let db_root = temp_dir.path().join("new_db");

        assert!(!db_root.exists());

        // Interactive mode should create the directory
        let _result = ensure_infrastructure(&db_root, true, true, false);
        // Will fail at LLM preflight (can't prompt in test), but dir should be created
        // Actually, it might create the dir before hitting preflight
        // Let's just check the result
        assert!(db_root.exists());
    }

    #[test]
    fn test_noninteractive_fails_without_config() {
        let temp_dir = TempDir::new().unwrap();
        let db_root = temp_dir.path().join("no_config_db");

        fs::create_dir(&db_root).unwrap();

        // Non-interactive without allow_prompt should fail if config missing
        let result = ensure_infrastructure(&db_root, false, false, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_existing_config_ready() {
        let temp_dir = TempDir::new().unwrap();
        let db_root = temp_dir.path();
        let config_path = db_root.join(CONFIG_FILE);

        // Create a valid (disabled) config
        fs::write(&config_path, "[llm]\nprovider = \"disabled\"\n").unwrap();

        // Should return Ready immediately
        let result = ensure_infrastructure(db_root, true, true, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BootstrapStatus::Ready);
    }

    #[test]
    fn test_missing_codegraph_db_prints_guidance() {
        let temp_dir = TempDir::new().unwrap();
        let db_root = temp_dir.path();
        let config_path = db_root.join(CONFIG_FILE);

        // Create config but no codegraph.db
        fs::write(&config_path, "[llm]\nprovider = \"disabled\"\n").unwrap();

        // Should succeed (degraded mode) even without codegraph.db
        let result = ensure_infrastructure(db_root, true, true, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BootstrapStatus::Ready);
    }

    #[test]
    fn test_idempotent_rerun() {
        let temp_dir = TempDir::new().unwrap();
        let db_root = temp_dir.path();
        let config_path = db_root.join(CONFIG_FILE);

        // Create config
        fs::write(&config_path, "[llm]\nprovider = \"disabled\"\n").unwrap();

        // First run
        let result1 = ensure_infrastructure(db_root, true, true, false);
        assert!(result1.is_ok());
        assert_eq!(result1.unwrap(), BootstrapStatus::Ready);

        // Second run should be identical
        let result2 = ensure_infrastructure(db_root, true, true, false);
        assert!(result2.is_ok());
        assert_eq!(result2.unwrap(), BootstrapStatus::Ready);
    }
}
