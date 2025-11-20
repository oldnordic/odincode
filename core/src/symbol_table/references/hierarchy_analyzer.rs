//! Hierarchy analysis for symbol relationships

use crate::symbol_table::core::{Symbol, SymbolKind};
use anyhow::Result;
use sqlx::{Row, SqlitePool};

/// Analyzer for symbol hierarchies and relationships
pub struct HierarchyAnalyzer {
    pool: SqlitePool,
}

impl HierarchyAnalyzer {
    /// Create a new hierarchy analyzer
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get usage statistics for a symbol
    pub async fn get_usage_statistics(&self, symbol_id: &str) -> Result<u32> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM symbol_references
            WHERE symbol_id = ? AND reference_type = 'usage'
            "#,
        )
        .bind(symbol_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }

    /// Get call statistics for a function/method
    pub async fn get_call_statistics(&self, symbol_id: &str) -> Result<u32> {
        let row = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM symbol_relationships
            WHERE to_symbol_id = ? AND relationship_type = 'calls'
            "#,
        )
        .bind(symbol_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.get("count"))
    }

    /// Find unused symbols (no references)
    pub async fn find_unused_symbols(&self) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT s.*
            FROM symbols s
            LEFT JOIN symbol_references sr ON s.id = sr.symbol_id
            LEFT JOIN symbol_relationships sr_rel ON s.id = sr_rel.to_symbol_id
            WHERE sr.symbol_id IS NULL AND sr_rel.to_symbol_id IS NULL
            AND s.kind NOT IN ('import', 'module', 'namespace', 'package')
            ORDER BY s.file_path, s.line
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        self.rows_to_symbols(rows).await
    }

    /// Get symbols in a specific scope
    pub async fn get_symbols_in_scope(&self, scope: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM symbols
            WHERE scope = ? OR scope LIKE ?
            ORDER BY kind, name
            "#,
        )
        .bind(scope)
        .bind(format!("{}.%", scope))
        .fetch_all(&self.pool)
        .await?;

        self.rows_to_symbols(rows).await
    }

    /// Get exported symbols from a file
    pub async fn get_exported_symbols(&self, file_path: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM symbols
            WHERE file_path = ? AND visibility = 'public'
            ORDER BY kind, name
            "#,
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await?;

        self.rows_to_symbols(rows).await
    }

    /// Find duplicate symbols across files
    pub async fn find_duplicate_symbols(&self) -> Result<Vec<(String, Vec<Symbol>)>> {
        let rows = sqlx::query(
            r#"
            SELECT name, COUNT(*) as count
            FROM symbols
            GROUP BY name
            HAVING count > 1
            ORDER BY count DESC, name
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut duplicates = Vec::new();
        for row in rows {
            let name: String = row.get("name");
            let symbol_rows = sqlx::query(
                r#"
                SELECT *
                FROM symbols
                WHERE name = ?
                ORDER BY file_path, line
                "#,
            )
            .bind(&name)
            .fetch_all(&self.pool)
            .await?;

            let symbols = self.rows_to_symbols(symbol_rows).await?;
            duplicates.push((name, symbols));
        }

        Ok(duplicates)
    }

    /// Get symbol count by kind
    pub async fn get_symbol_count_by_kind(&self) -> Result<std::collections::HashMap<String, u32>> {
        let rows = sqlx::query(
            r#"
            SELECT kind, COUNT(*) as count
            FROM symbols
            GROUP BY kind
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut counts = std::collections::HashMap::new();
        for row in rows {
            let kind: String = row.get("kind");
            let count: u32 = row.get("count");
            counts.insert(kind, count);
        }

        Ok(counts)
    }

    /// Get symbol count by language
    pub async fn get_symbol_count_by_language(
        &self,
    ) -> Result<std::collections::HashMap<String, u32>> {
        let rows = sqlx::query(
            r#"
            SELECT language, COUNT(*) as count
            FROM symbols
            GROUP BY language
            ORDER BY count DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut counts = std::collections::HashMap::new();
        for row in rows {
            let language: String = row.get("language");
            let count: u32 = row.get("count");
            counts.insert(language, count);
        }

        Ok(counts)
    }

    /// Get total symbol count
    pub async fn get_total_symbol_count(&self) -> Result<u32> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM symbols")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    /// Get recently updated symbols
    pub async fn get_recently_updated_symbols(&self, limit: u32) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM symbols
            ORDER BY updated_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
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
                "function" => SymbolKind::Function,
                "method" => SymbolKind::Method,
                "variable" => SymbolKind::Variable,
                "constant" => SymbolKind::Constant,
                "class" => SymbolKind::Class,
                "struct" => SymbolKind::Struct,
                "interface" => SymbolKind::Interface,
                "enum" => SymbolKind::Enum,
                "trait" => SymbolKind::Trait,
                "module" => SymbolKind::Module,
                "namespace" => SymbolKind::Namespace,
                "package" => SymbolKind::Package,
                "import" => SymbolKind::Import,
                "parameter" => SymbolKind::Parameter,
                "field" => SymbolKind::Field,
                "property" => SymbolKind::Property,
                "event" => SymbolKind::Event,
                "macro" => SymbolKind::Macro,
                "template" => SymbolKind::Template,
                "type_alias" => SymbolKind::TypeAlias,
                _ => SymbolKind::Variable,
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
}
