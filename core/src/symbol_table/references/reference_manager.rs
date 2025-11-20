//! Symbol reference management

use crate::symbol_table::core::{ReferenceType, SymbolReference};
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Manager for symbol references
pub struct ReferenceManager {
    pool: SqlitePool,
}

impl ReferenceManager {
    /// Create a new reference manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new symbol reference
    pub async fn create_reference(&self, reference: SymbolReference) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO symbol_references (
                id, symbol_id, file_path, line, column, reference_type, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&reference.id)
        .bind(&reference.symbol_id)
        .bind(&reference.file_path)
        .bind(reference.line)
        .bind(reference.column)
        .bind(reference.reference_type.as_str())
        .bind(reference.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all references for a symbol
    pub async fn get_symbol_references(&self, symbol_id: &str) -> Result<Vec<SymbolReference>> {
        let rows = sqlx::query(
            r#"
            SELECT id, symbol_id, file_path, line, column, reference_type, created_at
            FROM symbol_references
            WHERE symbol_id = ?
            ORDER BY file_path, line
            "#,
        )
        .bind(symbol_id)
        .fetch_all(&self.pool)
        .await?;

        let mut references = Vec::new();
        for row in rows {
            let reference_type_str: String = row.get("reference_type");
            let reference_type = match reference_type_str.as_str() {
                "usage" => ReferenceType::Usage,
                "definition" => ReferenceType::Definition,
                "declaration" => ReferenceType::Declaration,
                "call" => ReferenceType::Call,
                "assignment" => ReferenceType::Assignment,
                "inheritance" => ReferenceType::Inheritance,
                "implementation" => ReferenceType::Implementation,
                "import" => ReferenceType::Import,
                "export" => ReferenceType::Export,
                _ => ReferenceType::Usage,
            };

            references.push(SymbolReference {
                id: row.get("id"),
                symbol_id: row.get("symbol_id"),
                file_path: row.get("file_path"),
                line: row.get("line"),
                column: row.get("column"),
                reference_type,
                created_at: row.get("created_at"),
            });
        }

        Ok(references)
    }

    /// Get references by symbol name
    pub async fn get_references_by_name(&self, name: &str) -> Result<Vec<SymbolReference>> {
        let rows = sqlx::query(
            r#"
            SELECT sr.id, sr.symbol_id, sr.file_path, sr.line, sr.column, sr.reference_type, sr.created_at
            FROM symbol_references sr
            JOIN symbols s ON sr.symbol_id = s.id
            WHERE s.name = ?
            ORDER BY sr.file_path, sr.line
            "#,
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await?;

        let mut references = Vec::new();
        for row in rows {
            let reference_type_str: String = row.get("reference_type");
            let reference_type = match reference_type_str.as_str() {
                "usage" => ReferenceType::Usage,
                "definition" => ReferenceType::Definition,
                "declaration" => ReferenceType::Declaration,
                "call" => ReferenceType::Call,
                "assignment" => ReferenceType::Assignment,
                "inheritance" => ReferenceType::Inheritance,
                "implementation" => ReferenceType::Implementation,
                "import" => ReferenceType::Import,
                "export" => ReferenceType::Export,
                _ => ReferenceType::Usage,
            };

            references.push(SymbolReference {
                id: row.get("id"),
                symbol_id: row.get("symbol_id"),
                file_path: row.get("file_path"),
                line: row.get("line"),
                column: row.get("column"),
                reference_type,
                created_at: row.get("created_at"),
            });
        }

        Ok(references)
    }

    /// Generate a unique reference ID
    pub fn generate_reference_id() -> String {
        Uuid::new_v4().to_string()
    }
}
