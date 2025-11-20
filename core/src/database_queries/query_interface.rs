//! Database query interface for unified access to all databases
//!
//! Provides a single entry point for querying metadata, graph, and RAG databases
//! with consistent API and result types.

use crate::database::DatabaseManager;

/// Unified query interface for all databases
pub struct DatabaseQueryInterface {
    pub(crate) db_manager: DatabaseManager,
}

impl DatabaseQueryInterface {
    /// Create a new query interface
    pub fn new(db_manager: DatabaseManager) -> Self {
        Self { db_manager }
    }
}
