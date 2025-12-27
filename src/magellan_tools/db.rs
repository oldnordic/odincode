//! Magellan DB: read-only queries to SQLiteGraph
//!
//! Opens Magellan's SQLiteGraph database and queries:
//! - Status counts (files, symbols, references)
//! - Symbols in a file
//! - References to a symbol
//! - References from a file to a symbol

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Magellan DB handle (read-only)
pub struct MagellanDb {
    conn: Connection,
}

/// Status counts from Magellan indexing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusCounts {
    pub files: u64,
    pub symbols: u64,
    pub references: u64,
}

/// Symbol row from graph_entities table
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymbolRow {
    pub symbol_id: i64,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub byte_start: u64,
    pub byte_end: u64,
}

/// Reference row from graph_edges query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReferenceRow {
    pub reference_id: i64,
    pub from_file_path: String,
    pub symbol_id: i64,
    pub symbol_name: String,
    pub byte_start: u64,
    pub byte_end: u64,
}

impl MagellanDb {
    /// Open Magellan DB read-only
    ///
    /// # Arguments
    /// * `db_path` - Path to SQLiteGraph database file
    ///
    /// # Returns
    /// * `Ok(MagellanDb)` - DB handle ready for queries
    /// * `Err` - If DB cannot be opened
    pub fn open_readonly<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let path = db_path.as_ref();

        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .context("Failed to open Magellan DB read-only")?;

        Ok(MagellanDb { conn })
    }

    /// Get status counts (files, symbols, references)
    ///
    /// Queries graph_entities and graph_edges tables.
    pub fn status_counts(&self) -> Result<StatusCounts> {
        // Count files (entities with kind='File')
        let files: u64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM graph_entities WHERE kind = 'File'",
                [],
                |row| row.get(0),
            )
            .context("Failed to count files")?;

        // Count symbols (entities with kind='Symbol')
        let symbols: u64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM graph_entities WHERE kind = 'Symbol'",
                [],
                |row| row.get(0),
            )
            .context("Failed to count symbols")?;

        // Count references (edges)
        let references: u64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM graph_edges", [], |row| row.get(0))
            .context("Failed to count references")?;

        Ok(StatusCounts {
            files,
            symbols,
            references,
        })
    }

    /// Query symbols in a file
    ///
    /// Uses LIKE pattern on file_path. Returns symbols sorted by name (deterministic).
    ///
    /// # Arguments
    /// * `file_path_like` - SQL LIKE pattern (e.g., "lib.rs", "%/src/lib.rs")
    pub fn symbols_in_file(&self, file_path_like: &str) -> Result<Vec<SymbolRow>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT
                    e.id as symbol_id,
                    e.name,
                    e.kind,
                    e.file_path,
                    json_extract(e.data, '$.byte_start') as byte_start,
                    json_extract(e.data, '$.byte_end') as byte_end
                FROM graph_entities e
                WHERE e.kind = 'Symbol'
                  AND e.file_path LIKE ?
                ORDER BY e.name ASC
                "#,
            )
            .context("Failed to prepare symbols query")?;

        let rows = stmt
            .query_map([file_path_like], |row| {
                Ok(SymbolRow {
                    symbol_id: row.get(0)?,
                    name: row.get(1)?,
                    kind: row.get(2)?,
                    file_path: row.get(3)?,
                    byte_start: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
                    byte_end: row.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
                })
            })
            .context("Failed to execute symbols query")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect symbol rows")?;

        Ok(rows)
    }

    /// Query references to a symbol by name
    ///
    /// Finds all edges pointing to symbols with matching name.
    /// Returns references sorted by from_file_path (deterministic).
    ///
    /// # Arguments
    /// * `symbol_name` - Exact symbol name to find references to
    pub fn references_to_symbol_name(&self, symbol_name: &str) -> Result<Vec<ReferenceRow>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT
                    edge.id as reference_id,
                    from_entity.file_path as from_file_path,
                    to_entity.id as symbol_id,
                    to_entity.name as symbol_name,
                    json_extract(edge.data, '$.byte_start') as byte_start,
                    json_extract(edge.data, '$.byte_end') as byte_end
                FROM graph_edges edge
                JOIN graph_entities from_entity ON edge.from_id = from_entity.id
                JOIN graph_entities to_entity ON edge.to_id = to_entity.id
                WHERE to_entity.name = ?
                  AND edge.edge_type = 'REFERENCES'
                ORDER BY from_entity.file_path ASC
                "#,
            )
            .context("Failed to prepare references query")?;

        let rows = stmt
            .query_map([symbol_name], |row| {
                Ok(ReferenceRow {
                    reference_id: row.get(0)?,
                    from_file_path: row.get(1)?,
                    symbol_id: row.get(2)?,
                    symbol_name: row.get(3)?,
                    byte_start: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
                    byte_end: row.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
                })
            })
            .context("Failed to execute references query")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect reference rows")?;

        Ok(rows)
    }

    /// Query references from a specific file to a symbol
    ///
    /// Finds REFERENCES edges from entities in a file to a specific symbol.
    /// Returns references sorted by from_entity.id (deterministic).
    ///
    /// # Arguments
    /// * `file_path_like` - SQL LIKE pattern for from file
    /// * `symbol_name` - Exact symbol name being referenced
    pub fn references_from_file_to_symbol_name(
        &self,
        file_path_like: &str,
        symbol_name: &str,
    ) -> Result<Vec<ReferenceRow>> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT
                    edge.id as reference_id,
                    from_entity.file_path as from_file_path,
                    to_entity.id as symbol_id,
                    to_entity.name as symbol_name,
                    json_extract(edge.data, '$.byte_start') as byte_start,
                    json_extract(edge.data, '$.byte_end') as byte_end
                FROM graph_edges edge
                JOIN graph_entities from_entity ON edge.from_id = from_entity.id
                JOIN graph_entities to_entity ON edge.to_id = to_entity.id
                WHERE from_entity.file_path LIKE ?
                  AND to_entity.name = ?
                  AND edge.edge_type = 'REFERENCES'
                ORDER BY from_entity.id ASC
                "#,
            )
            .context("Failed to prepare file references query")?;

        let rows = stmt
            .query_map([file_path_like, symbol_name], |row| {
                Ok(ReferenceRow {
                    reference_id: row.get(0)?,
                    from_file_path: row.get(1)?,
                    symbol_id: row.get(2)?,
                    symbol_name: row.get(3)?,
                    byte_start: row.get::<_, Option<i64>>(4)?.unwrap_or(0) as u64,
                    byte_end: row.get::<_, Option<i64>>(5)?.unwrap_or(0) as u64,
                })
            })
            .context("Failed to execute file references query")?
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to collect file reference rows")?;

        Ok(rows)
    }
}
