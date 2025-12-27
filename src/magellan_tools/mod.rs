//! Magellan tools: query SQLiteGraph database directly
//!
//! Wraps SQLiteGraph DB access for querying code structure.
//! Read-only access — no mutations to DB.

mod db;

// Re-export public API
pub use db::{MagellanDb, ReferenceRow, StatusCounts, SymbolRow};
