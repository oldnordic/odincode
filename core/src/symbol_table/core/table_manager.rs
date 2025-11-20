//! Database table management for symbol table

use anyhow::Result;
use sqlx::SqlitePool;

/// Table manager for symbol table database
pub struct TableManager {
    pool: SqlitePool,
}

impl TableManager {
    /// Create a new table manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize symbol table database with required tables and indexes
    pub async fn init(&self) -> Result<()> {
        self.create_symbols_table().await?;
        self.create_symbol_references_table().await?;
        self.create_symbol_relationships_table().await?;
        self.create_indexes().await?;
        Ok(())
    }

    /// Create symbols table
    async fn create_symbols_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS symbols (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL,
                file_path TEXT NOT NULL,
                line INTEGER NOT NULL,
                column INTEGER NOT NULL,
                scope TEXT,
                visibility TEXT NOT NULL,
                language TEXT NOT NULL,
                signature TEXT,
                documentation TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create symbol references table
    async fn create_symbol_references_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS symbol_references (
                id TEXT PRIMARY KEY,
                symbol_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                line INTEGER NOT NULL,
                column INTEGER NOT NULL,
                reference_type TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (symbol_id) REFERENCES symbols (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create symbol relationships table
    async fn create_symbol_relationships_table(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS symbol_relationships (
                id TEXT PRIMARY KEY,
                from_symbol_id TEXT NOT NULL,
                to_symbol_id TEXT NOT NULL,
                relationship_type TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (from_symbol_id) REFERENCES symbols (id) ON DELETE CASCADE,
                FOREIGN KEY (to_symbol_id) REFERENCES symbols (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Create indexes for performance
    async fn create_indexes(&self) -> Result<()> {
        // Symbols table indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_kind ON symbols(kind)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_path)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_language ON symbols(language)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_scope ON symbols(scope)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_visibility ON symbols(visibility)")
            .execute(&self.pool)
            .await?;

        // Symbol references table indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_references_symbol ON symbol_references(symbol_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_references_file ON symbol_references(file_path)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_references_type ON symbol_references(reference_type)",
        )
        .execute(&self.pool)
        .await?;

        // Symbol relationships table indexes
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_relationships_from ON symbol_relationships(from_symbol_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_relationships_to ON symbol_relationships(to_symbol_id)",
        )
        .execute(&self.pool)
        .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_relationships_type ON symbol_relationships(relationship_type)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
