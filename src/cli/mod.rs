//! CLI module — Phase 4: End-to-End CLI Wiring
//!
//! Provides:
//! - Argument parsing for CLI modes
//! - db_root resolution (flag → env → cwd)
//! - Mode dispatch (tui, plan, execute, evidence)
//! - LLM preflight configuration wizard
//! - First-run bootstrap

pub mod args;
pub mod bootstrap;
pub mod db_root;
pub mod dispatch;
pub mod preflight;

// Re-exports
pub use args::{parse_args, Args, Mode};
pub use bootstrap::{ensure_infrastructure, BootstrapStatus};
pub use db_root::resolve_db_root;
pub use dispatch::{run_cli_mode, ExitCode};
pub use preflight::{run_llm_preflight, PreflightOutcome};

/// CLI errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),

    #[error("Unknown mode: {0}")]
    UnknownMode(String),

    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Exit codes (deterministic)
pub const EXIT_SUCCESS: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_DB_ERROR: i32 = 2;

/// Result type for CLI operations
pub type Result<T> = std::result::Result<T, Error>;
