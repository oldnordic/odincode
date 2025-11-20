//! Symbol relationship management

use crate::symbol_table::core::{RelationshipType, Symbol, SymbolRelationship};
use anyhow::Result;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Manager for symbol relationships
pub struct RelationshipManager {
    pool: SqlitePool,
}

impl RelationshipManager {
    /// Create a new relationship manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a new symbol relationship
    pub async fn create_relationship(&self, relationship: SymbolRelationship) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO symbol_relationships (
                id, from_symbol_id, to_symbol_id, relationship_type, created_at
            ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&relationship.id)
        .bind(&relationship.from_symbol_id)
        .bind(&relationship.to_symbol_id)
        .bind(relationship.relationship_type.as_str())
        .bind(relationship.created_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get relationships for a symbol
    pub async fn get_symbol_relationships(
        &self,
        symbol_id: &str,
    ) -> Result<Vec<SymbolRelationship>> {
        let rows = sqlx::query(
            r#"
            SELECT id, from_symbol_id, to_symbol_id, relationship_type, created_at
            FROM symbol_relationships
            WHERE from_symbol_id = ? OR to_symbol_id = ?
            ORDER BY created_at
            "#,
        )
        .bind(symbol_id)
        .bind(symbol_id)
        .fetch_all(&self.pool)
        .await?;

        let mut relationships = Vec::new();
        for row in rows {
            let relationship_type_str: String = row.get("relationship_type");
            let relationship_type = match relationship_type_str.as_str() {
                "calls" => RelationshipType::Calls,
                "inherits" => RelationshipType::Inherits,
                "implements" => RelationshipType::Implements,
                "uses" => RelationshipType::Uses,
                "contains" => RelationshipType::Contains,
                "depends_on" => RelationshipType::DependsOn,
                "overrides" => RelationshipType::Overrides,
                "extends" => RelationshipType::Extends,
                _ => RelationshipType::Uses,
            };

            relationships.push(SymbolRelationship {
                id: row.get("id"),
                from_symbol_id: row.get("from_symbol_id"),
                to_symbol_id: row.get("to_symbol_id"),
                relationship_type,
                created_at: row.get("created_at"),
            });
        }

        Ok(relationships)
    }

    /// Find symbols that call the given function
    pub async fn find_callers(&self, function_name: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT s.*
            FROM symbols s
            JOIN symbol_relationships sr ON s.id = sr.from_symbol_id
            JOIN symbols target ON sr.to_symbol_id = target.id
            WHERE sr.relationship_type = 'calls' AND target.name = ?
            ORDER BY s.file_path, s.line
            "#,
        )
        .bind(function_name)
        .fetch_all(&self.pool)
        .await?;

        self.rows_to_symbols(rows).await
    }

    /// Find symbols called by the given symbol
    pub async fn find_callees(&self, symbol_id: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT s.*
            FROM symbols s
            JOIN symbol_relationships sr ON s.id = sr.to_symbol_id
            WHERE sr.from_symbol_id = ? AND sr.relationship_type = 'calls'
            ORDER BY s.file_path, s.line
            "#,
        )
        .bind(symbol_id)
        .fetch_all(&self.pool)
        .await?;

        self.rows_to_symbols(rows).await
    }

    /// Get inheritance hierarchy for a symbol
    pub async fn get_inheritance_hierarchy(&self, symbol_id: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE inheritance_chain AS (
                SELECT s.*, 0 as level
                FROM symbols s
                WHERE s.id = ?
                
                UNION ALL
                
                SELECT s.*, ic.level + 1
                FROM symbols s
                JOIN symbol_relationships sr ON s.id = sr.to_symbol_id
                JOIN inheritance_chain ic ON sr.from_symbol_id = ic.id
                WHERE sr.relationship_type IN ('inherits', 'extends', 'implements')
            )
            SELECT * FROM inheritance_chain
            ORDER BY level
            "#,
        )
        .bind(symbol_id)
        .fetch_all(&self.pool)
        .await?;

        self.rows_to_symbols(rows).await
    }

    /// Get subclasses for a class
    pub async fn get_subclasses(&self, class_name: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT s.*
            FROM symbols s
            JOIN symbol_relationships sr ON s.id = sr.from_symbol_id
            JOIN symbols parent ON sr.to_symbol_id = parent.id
            WHERE sr.relationship_type IN ('inherits', 'extends') AND parent.name = ?
            ORDER BY s.file_path, s.line
            "#,
        )
        .bind(class_name)
        .fetch_all(&self.pool)
        .await?;

        self.rows_to_symbols(rows).await
    }

    /// Convert database rows to Symbol objects
    async fn rows_to_symbols(&self, rows: Vec<sqlx::sqlite::SqliteRow>) -> Result<Vec<Symbol>> {
        let mut symbols = Vec::new();

        for row in rows {
            let kind_str: String = row.get("kind");
            let kind = match kind_str.as_str() {
                "function" => crate::symbol_table::core::SymbolKind::Function,
                "method" => crate::symbol_table::core::SymbolKind::Method,
                "variable" => crate::symbol_table::core::SymbolKind::Variable,
                "constant" => crate::symbol_table::core::SymbolKind::Constant,
                "class" => crate::symbol_table::core::SymbolKind::Class,
                "struct" => crate::symbol_table::core::SymbolKind::Struct,
                "interface" => crate::symbol_table::core::SymbolKind::Interface,
                "enum" => crate::symbol_table::core::SymbolKind::Enum,
                "trait" => crate::symbol_table::core::SymbolKind::Trait,
                "module" => crate::symbol_table::core::SymbolKind::Module,
                "namespace" => crate::symbol_table::core::SymbolKind::Namespace,
                "package" => crate::symbol_table::core::SymbolKind::Package,
                "import" => crate::symbol_table::core::SymbolKind::Import,
                "parameter" => crate::symbol_table::core::SymbolKind::Parameter,
                "field" => crate::symbol_table::core::SymbolKind::Field,
                "property" => crate::symbol_table::core::SymbolKind::Property,
                "event" => crate::symbol_table::core::SymbolKind::Event,
                "macro" => crate::symbol_table::core::SymbolKind::Macro,
                "template" => crate::symbol_table::core::SymbolKind::Template,
                "type_alias" => crate::symbol_table::core::SymbolKind::TypeAlias,
                _ => crate::symbol_table::core::SymbolKind::Variable,
            };

            let visibility_str: String = row.get("visibility");
            let visibility = match visibility_str.as_str() {
                "public" => crate::symbol_table::core::Visibility::Public,
                "private" => crate::symbol_table::core::Visibility::Private,
                "protected" => crate::symbol_table::core::Visibility::Protected,
                "internal" => crate::symbol_table::core::Visibility::Internal,
                "package" => crate::symbol_table::core::Visibility::Package,
                _ => crate::symbol_table::core::Visibility::Private,
            };

            symbols.push(Symbol {
                id: row.get("id"),
                name: row.get("name"),
                kind,
                file_path: row.get("file_path"),
                line: row.get("line"),
                column: row.get("column"),
                scope: row.get("scope"),
                visibility,
                language: row.get("language"),
                signature: row.get("signature"),
                documentation: row.get("documentation"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(symbols)
    }

    /// Generate a unique relationship ID
    pub fn generate_relationship_id() -> String {
        Uuid::new_v4().to_string()
    }
}
