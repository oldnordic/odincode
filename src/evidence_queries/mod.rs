//! Evidence queries (Phase 0.6)
//!
//! Read-only query interface over execution memory.
//!
//! ## Architecture
//!
//! - `db.rs` — EvidenceDb, read-only connection management
//! - `queries.rs` — Q1–Q8 implementations
//! - `types.rs` — Result types
//!
//! ## Evidence Only
//!
//! All queries return facts, not inferences.
//! - "X occurred at T" ✓
//! - "X caused Y" ✗
//! - "X likely fixed Y" ✗

pub mod db;
pub mod queries;
pub mod types;

// Re-export main types
pub use db::EvidenceDb;
pub use types::*;

/// Evidence query errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("execution_log.db not found at {0}")]
    ExecutionLogNotFound(String),

    #[error("execution not found: {0}")]
    ExecutionNotFound(String),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
