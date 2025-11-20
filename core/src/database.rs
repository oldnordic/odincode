//! Database module for OdinCode
//!
//! This module provides the triple-database architecture (SQLite + Graph + RAG)
//! with efficient connection management and query interfaces.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// Database configuration
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub metadata_db_path: String,
    pub graph_db_path: String,
    pub rag_db_path: String,
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            metadata_db_path: "odincode_metadata.db".to_string(),
            graph_db_path: "odincode_graph.db".to_string(),
            rag_db_path: "odincode_rag.db".to_string(),
            max_connections: 10,
        }
    }
}

/// Main database manager for the triple-database system
pub struct DatabaseManager {
    /// Metadata database (files, symbols, patterns)
    metadata_pool: SqlitePool,

    /// Graph database (relationships, dependencies)
    graph_pool: SqlitePool,

    /// RAG database (code chunks, embeddings, context)
    rag_pool: SqlitePool,

    /// Configuration
    config: DatabaseConfig,
}

impl DatabaseManager {
    /// Create a new database manager with the specified configuration
    pub async fn new(config: DatabaseConfig) -> Result<Self> {
        // Initialize metadata database
        let metadata_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&config.metadata_db_path)
                .create_if_missing(true),
        )
        .await?;

        // Initialize graph database
        let graph_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&config.graph_db_path)
                .create_if_missing(true),
        )
        .await?;

        // Initialize RAG database
        let rag_pool = SqlitePool::connect_with(
            sqlx::sqlite::SqliteConnectOptions::new()
                .filename(&config.rag_db_path)
                .create_if_missing(true),
        )
        .await?;

        // Run migrations for all databases
        Self::run_migrations(&metadata_pool, DatabaseType::Metadata).await?;
        Self::run_migrations(&graph_pool, DatabaseType::Graph).await?;
        Self::run_migrations(&rag_pool, DatabaseType::Rag).await?;

        Ok(Self {
            metadata_pool,
            graph_pool,
            rag_pool,
            config,
        })
    }

    /// Run database migrations for the specified database type
    async fn run_migrations(pool: &SqlitePool, db_type: DatabaseType) -> Result<()> {
        match db_type {
            DatabaseType::Metadata => {
                // File metadata tables
                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS files (
                        id TEXT PRIMARY KEY,
                        path TEXT NOT NULL UNIQUE,
                        language TEXT NOT NULL,
                        content TEXT,
                        created_at INTEGER NOT NULL,
                        modified_at INTEGER NOT NULL,
                        size INTEGER NOT NULL
                    )
                    "#,
                )
                .execute(pool)
                .await?;

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
                        visibility TEXT,
                        language TEXT NOT NULL,
                        FOREIGN KEY (file_path) REFERENCES files (path)
                    )
                    "#,
                )
                .execute(pool)
                .await?;

                // Create indexes for performance
                sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_path ON files(path)")
                    .execute(pool)
                    .await?;
                sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name)")
                    .execute(pool)
                    .await?;
                sqlx::query("CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_path)")
                    .execute(pool)
                    .await?;
            }
            DatabaseType::Graph => {
                // Graph relationship tables
                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS relationships (
                        id TEXT PRIMARY KEY,
                        from_node TEXT NOT NULL,
                        to_node TEXT NOT NULL,
                        relationship_type TEXT NOT NULL,
                        file_path TEXT,
                        created_at INTEGER NOT NULL
                    )
                    "#,
                )
                .execute(pool)
                .await?;

                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS nodes (
                        id TEXT PRIMARY KEY,
                        name TEXT NOT NULL,
                        node_type TEXT NOT NULL,
                        file_path TEXT,
                        metadata TEXT
                    )
                    "#,
                )
                .execute(pool)
                .await?;

                // Create indexes for performance
                sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_relationships_from ON relationships(from_node)",
                )
                .execute(pool)
                .await?;
                sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_relationships_to ON relationships(to_node)",
                )
                .execute(pool)
                .await?;
                sqlx::query("CREATE INDEX IF NOT EXISTS idx_relationships_type ON relationships(relationship_type)").execute(pool).await?;
            }
            DatabaseType::Rag => {
                // RAG tables for code chunks and embeddings
                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS code_chunks (
                        id TEXT PRIMARY KEY,
                        file_path TEXT NOT NULL,
                        chunk_type TEXT NOT NULL,
                        content TEXT NOT NULL,
                        start_line INTEGER NOT NULL,
                        end_line INTEGER NOT NULL,
                        embedding BLOB,
                        semantic_hash TEXT,
                        metadata TEXT,
                        created_at INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL
                    )
                    "#,
                )
                .execute(pool)
                .await?;

                sqlx::query(
                    r#"
                    CREATE TABLE IF NOT EXISTS embeddings (
                        id TEXT PRIMARY KEY,
                        chunk_id TEXT NOT NULL,
                        vector_data BLOB NOT NULL,
                        model_name TEXT NOT NULL,
                        created_at INTEGER NOT NULL,
                        FOREIGN KEY (chunk_id) REFERENCES code_chunks (id)
                    )
                    "#,
                )
                .execute(pool)
                .await?;

                // Create indexes for performance
                sqlx::query("CREATE INDEX IF NOT EXISTS idx_chunks_file ON code_chunks(file_path)")
                    .execute(pool)
                    .await?;
                sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_chunks_type ON code_chunks(chunk_type)",
                )
                .execute(pool)
                .await?;
                sqlx::query(
                    "CREATE INDEX IF NOT EXISTS idx_chunks_hash ON code_chunks(semantic_hash)",
                )
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }

    /// Get a reference to the metadata database pool
    pub fn metadata_pool(&self) -> &SqlitePool {
        &self.metadata_pool
    }

    /// Get a reference to the graph database pool
    pub fn graph_pool(&self) -> &SqlitePool {
        &self.graph_pool
    }

    /// Get a reference to the RAG database pool
    pub fn rag_pool(&self) -> &SqlitePool {
        &self.rag_pool
    }

    /// Get the database configuration
    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Close all database connections
    pub async fn close(&self) -> Result<()> {
        self.metadata_pool.close().await;
        self.graph_pool.close().await;
        self.rag_pool.close().await;
        Ok(())
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        let metadata_stats = self
            .get_database_stats(&self.metadata_pool, DatabaseType::Metadata)
            .await?;
        let graph_stats = self
            .get_database_stats(&self.graph_pool, DatabaseType::Graph)
            .await?;
        let rag_stats = self
            .get_database_stats(&self.rag_pool, DatabaseType::Rag)
            .await?;

        Ok(DatabaseStats {
            metadata: metadata_stats,
            graph: graph_stats,
            rag: rag_stats,
        })
    }

    /// Get statistics for a specific database
    async fn get_database_stats(
        &self,
        pool: &SqlitePool,
        db_type: DatabaseType,
    ) -> Result<DatabaseStat> {
        let table_count_query = match db_type {
            DatabaseType::Metadata => {
                "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table'"
            }
            DatabaseType::Graph => "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table'",
            DatabaseType::Rag => "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table'",
        };

        let row = sqlx::query(table_count_query).fetch_one(pool).await?;
        let table_count: i64 = row.get("count");

        // Get approximate row counts for main tables
        let (rows_count, size_mb) = match db_type {
            DatabaseType::Metadata => {
                let files_count: (i64, i64) =
                    sqlx::query_as("SELECT COUNT(*) as files, SUM(size) as total_size FROM files")
                        .fetch_one(pool)
                        .await?;
                (files_count.0, files_count.1 / (1024 * 1024)) // Convert to MB
            }
            DatabaseType::Graph => {
                let rel_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM relationships")
                    .fetch_one(pool)
                    .await?;
                (rel_count, 0) // Size calculation would require pragma
            }
            DatabaseType::Rag => {
                let chunks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM code_chunks")
                    .fetch_one(pool)
                    .await?;
                (chunks_count, 0) // Size calculation would require pragma
            }
        };

        Ok(DatabaseStat {
            table_count: table_count as u32,
            row_count: rows_count as u32,
            size_mb: size_mb as u32,
        })
    }
}

/// Type of database in the triple-database system
#[derive(Debug, Clone, Copy)]
pub enum DatabaseType {
    Metadata,
    Graph,
    Rag,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub metadata: DatabaseStat,
    pub graph: DatabaseStat,
    pub rag: DatabaseStat,
}

/// Individual database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStat {
    pub table_count: u32,
    pub row_count: u32,
    pub size_mb: u32,
}

/// File metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub id: String,
    pub path: String,
    pub language: String,
    pub content: Option<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub size: i64,
}

impl FileMetadata {
    pub fn new(path: String, language: String, content: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        let content_len = content.len() as i64;
        Self {
            id: Uuid::new_v4().to_string(),
            path,
            language,
            content: Some(content),
            created_at: now,
            modified_at: now,
            size: content_len,
        }
    }
}

/// Symbol information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolInfo {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub file_path: String,
    pub line: u32,
    pub column: u32,
    pub scope: Option<String>,
    pub visibility: String,
    pub language: String,
}

impl SymbolInfo {
    pub fn new(
        name: String,
        kind: String,
        file_path: String,
        line: u32,
        column: u32,
        scope: Option<String>,
        visibility: String,
        language: String,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            kind,
            file_path,
            line,
            column,
            scope,
            visibility,
            language,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_file_metadata_creation() {
        let metadata = FileMetadata::new(
            "test.rs".to_string(),
            "rust".to_string(),
            "fn main() { println!(\"Hello\"); }".to_string(),
        );

        assert_eq!(metadata.path, "test.rs");
        assert_eq!(metadata.language, "rust");
        assert!(metadata.size > 0);
        assert!(metadata.created_at > 0);
        assert!(metadata.modified_at > 0);
    }

    #[tokio::test]
    async fn test_symbol_info_creation() {
        let symbol = SymbolInfo::new(
            "test_function".to_string(),
            "function".to_string(),
            "test.rs".to_string(),
            10,
            5,
            Some("TestModule".to_string()),
            "public".to_string(),
            "rust".to_string(),
        );

        assert_eq!(symbol.name, "test_function");
        assert_eq!(symbol.kind, "function");
        assert_eq!(symbol.file_path, "test.rs");
        assert_eq!(symbol.line, 10);
        assert_eq!(symbol.column, 5);
        assert_eq!(symbol.scope, Some("TestModule".to_string()));
        assert_eq!(symbol.visibility, "public");
        assert_eq!(symbol.language, "rust");
    }
}
