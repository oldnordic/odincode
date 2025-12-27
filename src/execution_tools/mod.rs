//! Execution memory tools (Phase 0.5.2)
//!
//! Provides audit-grade logging of tool invocations with:
//! - SQLite (execution_log.db) for temporal log
//! - SQLiteGraph (codegraph.db) for causal relationships
//!
//! ## Architecture
//!
//! - `db.rs` — ExecutionDb, schema initialization
//! - `record.rs` — Execution recording with dual-write
//! - `query.rs` — Read-only queries with deterministic ordering
//! - `graph.rs` — SQLiteGraph integration (entities + edges)
//! - `memory_query.rs` — Query execution log with filters (Phase 1.1)
//! - `execution_summary.rs` — Aggregate execution statistics (Phase 1.2)
//! - `discovery_log.rs` — Discovery event logging (Phase 10.7)

pub mod db;
pub mod discovery_log;
pub mod execution_summary;
pub mod graph;
pub mod memory_query;
pub mod query;
pub mod record;

// Re-export main types
pub use db::{Error, ExecutionDb};
pub use discovery_log::{log_discovery_event, query_discovery_events, DiscoveryEvent};
pub use execution_summary::{execution_summary, ExecutionSummary, ExecutionSummaryArgs};
pub use memory_query::{
    ExecutionRecord, MemoryQueryArgs, MemoryQueryResult, get_pending_failures,
    get_preceding_context, get_recent_timeline, get_timeline_position, memory_query,
};
pub use record::Execution;
