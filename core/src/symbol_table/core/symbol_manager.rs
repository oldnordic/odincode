//! Main symbol table manager

use anyhow::Result;
use sqlx::{Row, SqlitePool};
use tree_sitter::Tree;
use uuid::Uuid;

use crate::symbol_table::analysis::{
    ComprehensiveStats, DuplicateDetector, DuplicateGroup, StatisticsCollector, UsageAnalysis,
    UsageAnalyzer,
};
use crate::symbol_table::core::{Symbol, SymbolFilter, TableManager};
use crate::symbol_table::extraction::ASTExtractor;
use crate::symbol_table::references::{HierarchyAnalyzer, ReferenceManager, RelationshipManager};

/// Main symbol table manager
pub struct SymbolTableManager {
    pool: SqlitePool,
    table_manager: TableManager,
    reference_manager: ReferenceManager,
    relationship_manager: RelationshipManager,
    hierarchy_analyzer: HierarchyAnalyzer,
    ast_extractor: ASTExtractor,
    usage_analyzer: UsageAnalyzer,
    duplicate_detector: DuplicateDetector,
    statistics_collector: StatisticsCollector,
}

impl SymbolTableManager {
    /// Create a new symbol table manager
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            table_manager: TableManager::new(pool.clone()),
            reference_manager: ReferenceManager::new(pool.clone()),
            relationship_manager: RelationshipManager::new(pool.clone()),
            hierarchy_analyzer: HierarchyAnalyzer::new(pool.clone()),
            ast_extractor: ASTExtractor::new(pool.clone()),
            usage_analyzer: UsageAnalyzer::new(pool.clone()),
            duplicate_detector: DuplicateDetector::new(pool.clone()),
            statistics_collector: StatisticsCollector::new(pool.clone()),
            pool,
        }
    }

    /// Initialize the symbol table database
    pub async fn init(&self) -> Result<()> {
        self.table_manager.init().await
    }

    /// Create a new symbol
    pub async fn create_symbol(&self, symbol: Symbol) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO symbols (
                id, name, kind, file_path, line, column, scope, visibility, 
                language, signature, documentation, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&symbol.id)
        .bind(&symbol.name)
        .bind(symbol.kind.as_str())
        .bind(&symbol.file_path)
        .bind(symbol.line)
        .bind(symbol.column)
        .bind(&symbol.scope)
        .bind(symbol.visibility.as_str())
        .bind(&symbol.language)
        .bind(&symbol.signature)
        .bind(&symbol.documentation)
        .bind(symbol.created_at)
        .bind(symbol.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get symbol by ID
    pub async fn get_symbol_by_id(&self, id: &str) -> Result<Option<Symbol>> {
        let row = sqlx::query(
            r#"
            SELECT *
            FROM symbols
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_symbol(row)?))
        } else {
            Ok(None)
        }
    }

    /// Get symbols by name
    pub async fn get_symbols_by_name(&self, name: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM symbols
            WHERE name = ?
            ORDER BY file_path, line
            "#,
        )
        .bind(name)
        .fetch_all(&self.pool)
        .await?;

        let mut symbols = Vec::new();
        for row in rows {
            symbols.push(self.row_to_symbol(row)?);
        }

        Ok(symbols)
    }

    /// Get symbols by file
    pub async fn get_symbols_by_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        let rows = sqlx::query(
            r#"
            SELECT *
            FROM symbols
            WHERE file_path = ?
            ORDER BY line, column
            "#,
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await?;

        let mut symbols = Vec::new();
        for row in rows {
            symbols.push(self.row_to_symbol(row)?);
        }

        Ok(symbols)
    }

    /// Update a symbol
    pub async fn update_symbol(&self, symbol: Symbol) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE symbols SET
                name = ?, kind = ?, file_path = ?, line = ?, column = ?, scope = ?,
                visibility = ?, language = ?, signature = ?, documentation = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&symbol.name)
        .bind(symbol.kind.as_str())
        .bind(&symbol.file_path)
        .bind(symbol.line)
        .bind(symbol.column)
        .bind(&symbol.scope)
        .bind(symbol.visibility.as_str())
        .bind(&symbol.language)
        .bind(&symbol.signature)
        .bind(&symbol.documentation)
        .bind(symbol.updated_at)
        .bind(&symbol.id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete a symbol
    pub async fn delete_symbol(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM symbols WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List symbols with optional filter
    pub async fn list_symbols(&self, filter: Option<SymbolFilter>) -> Result<Vec<Symbol>> {
        let mut query = "SELECT * FROM symbols".to_string();
        let mut bindings = Vec::new();

        if let Some(f) = filter {
            let mut conditions = Vec::new();

            if let Some(name_pattern) = f.name_pattern {
                conditions.push("name LIKE ?");
                bindings.push(format!("%{}%", name_pattern));
            }

            if let Some(kind) = f.kind {
                conditions.push("kind = ?");
                bindings.push(kind.as_str().to_string());
            }

            if let Some(file_path) = f.file_path {
                conditions.push("file_path = ?");
                bindings.push(file_path);
            }

            if let Some(language) = f.language {
                conditions.push("language = ?");
                bindings.push(language);
            }

            if let Some(visibility) = f.visibility {
                conditions.push("visibility = ?");
                bindings.push(visibility.as_str().to_string());
            }

            if let Some(scope) = f.scope {
                conditions.push("scope = ?");
                bindings.push(scope);
            }

            if !conditions.is_empty() {
                query.push_str(" WHERE ");
                query.push_str(&conditions.join(" AND "));
            }
        }

        query.push_str(" ORDER BY file_path, line");

        let mut sqlx_query = sqlx::query(&query);
        for binding in bindings {
            sqlx_query = sqlx_query.bind(binding);
        }

        let rows = sqlx_query.fetch_all(&self.pool).await?;

        let mut symbols = Vec::new();
        for row in rows {
            symbols.push(self.row_to_symbol(row)?);
        }

        Ok(symbols)
    }

    /// Extract symbols from AST
    pub async fn extract_symbols_from_ast(
        &self,
        tree: &Tree,
        file_content: &str,
        file_path: &str,
        language: &str,
    ) -> Result<Vec<Symbol>> {
        self.ast_extractor
            .extract_symbols_from_ast(tree, file_content, file_path, language)
            .await
    }

    /// Get symbol references
    pub async fn get_symbol_references(
        &self,
        symbol_id: &str,
    ) -> Result<Vec<crate::symbol_table::core::SymbolReference>> {
        self.reference_manager
            .get_symbol_references(symbol_id)
            .await
    }

    /// Get references by name
    pub async fn get_references_by_name(
        &self,
        name: &str,
    ) -> Result<Vec<crate::symbol_table::core::SymbolReference>> {
        self.reference_manager.get_references_by_name(name).await
    }

    /// Create symbol relationship
    pub async fn create_relationship(
        &self,
        relationship: crate::symbol_table::core::SymbolRelationship,
    ) -> Result<()> {
        self.relationship_manager
            .create_relationship(relationship)
            .await
    }

    /// Get symbol relationships
    pub async fn get_symbol_relationships(
        &self,
        symbol_id: &str,
    ) -> Result<Vec<crate::symbol_table::core::SymbolRelationship>> {
        self.relationship_manager
            .get_symbol_relationships(symbol_id)
            .await
    }

    /// Find callers of a function
    pub async fn find_callers(&self, function_name: &str) -> Result<Vec<Symbol>> {
        self.relationship_manager.find_callers(function_name).await
    }

    /// Find callees of a symbol
    pub async fn find_callees(&self, symbol_id: &str) -> Result<Vec<Symbol>> {
        self.relationship_manager.find_callees(symbol_id).await
    }

    /// Get inheritance hierarchy
    pub async fn get_inheritance_hierarchy(&self, symbol_id: &str) -> Result<Vec<Symbol>> {
        self.relationship_manager
            .get_inheritance_hierarchy(symbol_id)
            .await
    }

    /// Get subclasses
    pub async fn get_subclasses(&self, class_name: &str) -> Result<Vec<Symbol>> {
        self.relationship_manager.get_subclasses(class_name).await
    }

    /// Get usage statistics
    pub async fn get_usage_statistics(&self, symbol_id: &str) -> Result<u32> {
        self.hierarchy_analyzer
            .get_usage_statistics(symbol_id)
            .await
    }

    /// Get call statistics
    pub async fn get_call_statistics(&self, symbol_id: &str) -> Result<u32> {
        self.hierarchy_analyzer.get_call_statistics(symbol_id).await
    }

    /// Find unused symbols
    pub async fn find_unused_symbols(&self) -> Result<Vec<Symbol>> {
        self.hierarchy_analyzer.find_unused_symbols().await
    }

    /// Get symbols in scope
    pub async fn get_symbols_in_scope(&self, scope: &str) -> Result<Vec<Symbol>> {
        self.hierarchy_analyzer.get_symbols_in_scope(scope).await
    }

    /// Get exported symbols
    pub async fn get_exported_symbols(&self, file_path: &str) -> Result<Vec<Symbol>> {
        self.hierarchy_analyzer
            .get_exported_symbols(file_path)
            .await
    }

    /// Find duplicate symbols
    pub async fn find_duplicate_symbols(&self) -> Result<Vec<(String, Vec<Symbol>)>> {
        self.hierarchy_analyzer.find_duplicate_symbols().await
    }

    /// Get symbol count by kind
    pub async fn get_symbol_count_by_kind(&self) -> Result<std::collections::HashMap<String, u32>> {
        self.hierarchy_analyzer.get_symbol_count_by_kind().await
    }

    /// Get symbol count by language
    pub async fn get_symbol_count_by_language(
        &self,
    ) -> Result<std::collections::HashMap<String, u32>> {
        self.hierarchy_analyzer.get_symbol_count_by_language().await
    }

    /// Get total symbol count
    pub async fn get_total_symbol_count(&self) -> Result<u32> {
        self.hierarchy_analyzer.get_total_symbol_count().await
    }

    /// Batch update timestamps
    pub async fn batch_update_timestamps(&self, symbol_ids: &[String]) -> Result<u32> {
        let now = chrono::Utc::now().timestamp();

        let placeholders = symbol_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "UPDATE symbols SET updated_at = {} WHERE id IN ({})",
            now, placeholders
        );

        let mut sqlx_query = sqlx::query(&query);
        for id in symbol_ids {
            sqlx_query = sqlx_query.bind(id);
        }

        let result = sqlx_query.execute(&self.pool).await?;
        Ok(result.rows_affected() as u32)
    }

    /// Get recently updated symbols
    pub async fn get_recently_updated_symbols(&self, limit: u32) -> Result<Vec<Symbol>> {
        self.hierarchy_analyzer
            .get_recently_updated_symbols(limit)
            .await
    }

    /// Analyze symbol usage
    pub async fn analyze_symbol_usage(&self, symbol_id: &str) -> Result<UsageAnalysis> {
        self.usage_analyzer.analyze_symbol_usage(symbol_id).await
    }

    /// Find all duplicate symbols
    pub async fn find_all_duplicates(&self) -> Result<Vec<DuplicateGroup>> {
        self.duplicate_detector.find_all_duplicates().await
    }

    /// Get comprehensive statistics
    pub async fn get_comprehensive_stats(&self) -> Result<ComprehensiveStats> {
        self.statistics_collector.get_comprehensive_stats().await
    }

    /// Generate unique symbol ID
    pub fn generate_symbol_id() -> String {
        Uuid::new_v4().to_string()
    }

    /// Convert database row to Symbol
    fn row_to_symbol(&self, row: sqlx::sqlite::SqliteRow) -> Result<Symbol> {
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

        Ok(Symbol {
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
        })
    }
}
