//! File metadata database with CRUD operations for OdinCode
//!
//! This module provides comprehensive CRUD operations for file metadata,
//! including create, read, update, delete, and advanced querying capabilities.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

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

/// File metadata with additional computed fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedFileMetadata {
    pub metadata: FileMetadata,
    pub line_count: u32,
    pub word_count: u32,
    pub character_count: u32,
    pub hash: Option<String>,
    pub tags: Vec<String>,
}

/// Filter criteria for file queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFilter {
    pub path_pattern: Option<String>,
    pub language: Option<String>,
    pub min_size: Option<i64>,
    pub max_size: Option<i64>,
    pub min_modified_time: Option<i64>,
    pub max_modified_time: Option<i64>,
    pub tags: Option<Vec<String>>,
    pub content_contains: Option<String>,
}

/// Sort criteria for file queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileSort {
    Path,
    Language,
    Size,
    ModifiedAt,
    CreatedAt,
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub limit: u32,
    pub offset: u32,
}

/// File metadata manager
pub struct FileMetadataManager {
    pool: SqlitePool,
}

impl FileMetadataManager {
    /// Create a new file metadata manager
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the file metadata database with required tables and indexes
    pub async fn init(&self) -> Result<()> {
        // Create files table
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
        .execute(&self.pool)
        .await?;

        // Create file tags table for additional metadata
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS file_tags (
                file_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                PRIMARY KEY (file_id, tag),
                FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create file hashes table for content hashing
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS file_hashes (
                file_id TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create computed statistics table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS file_statistics (
                file_id TEXT PRIMARY KEY,
                line_count INTEGER NOT NULL DEFAULT 0,
                word_count INTEGER NOT NULL DEFAULT 0,
                character_count INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (file_id) REFERENCES files (id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_path ON files(path)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_language ON files(language)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_size ON files(size)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_modified ON files(modified_at)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_created ON files(created_at)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_file_tags_tag ON file_tags(tag)")
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Create a new file metadata entry
    pub async fn create_file(&self, metadata: FileMetadata) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO files (id, path, language, content, created_at, modified_at, size)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&metadata.id)
        .bind(&metadata.path)
        .bind(&metadata.language)
        .bind(&metadata.content)
        .bind(metadata.created_at)
        .bind(metadata.modified_at)
        .bind(metadata.size)
        .execute(&self.pool)
        .await?;

        // Initialize statistics with default values
        sqlx::query(
            r#"
            INSERT INTO file_statistics (file_id, line_count, word_count, character_count)
            VALUES (?, 0, 0, 0)
            "#,
        )
        .bind(&metadata.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get file metadata by ID
    pub async fn get_file_by_id(&self, id: &str) -> Result<Option<FileMetadata>> {
        let row = sqlx::query(
            r#"
            SELECT id, path, language, content, created_at, modified_at, size
            FROM files
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(FileMetadata {
                id: row.get("id"),
                path: row.get("path"),
                language: row.get("language"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                modified_at: row.get("modified_at"),
                size: row.get("size"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Get file metadata by path
    pub async fn get_file_by_path(&self, path: &str) -> Result<Option<FileMetadata>> {
        let row = sqlx::query(
            r#"
            SELECT id, path, language, content, created_at, modified_at, size
            FROM files
            WHERE path = ?
            "#,
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(Some(FileMetadata {
                id: row.get("id"),
                path: row.get("path"),
                language: row.get("language"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                modified_at: row.get("modified_at"),
                size: row.get("size"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Update file metadata
    pub async fn update_file(&self, metadata: FileMetadata) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE files
            SET path = ?, language = ?, content = ?, modified_at = ?, size = ?
            WHERE id = ?
            "#,
        )
        .bind(&metadata.path)
        .bind(&metadata.language)
        .bind(&metadata.content)
        .bind(metadata.modified_at)
        .bind(metadata.size)
        .bind(&metadata.id)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete file metadata by ID
    pub async fn delete_file(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM files WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Delete file metadata by path
    pub async fn delete_file_by_path(&self, path: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM files WHERE path = ?")
            .bind(path)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// List files with filtering, sorting, and pagination
    pub async fn list_files(
        &self,
        filter: Option<FileFilter>,
        sort: Option<FileSort>,
        pagination: Option<Pagination>,
    ) -> Result<Vec<FileMetadata>> {
        let mut query = "SELECT id, path, language, content, created_at, modified_at, size FROM files WHERE 1=1".to_string();
        let mut bind_params = Vec::new();

        // Apply filters
        if let Some(filter) = filter {
            if let Some(pattern) = filter.path_pattern {
                query.push_str(" AND path LIKE ?");
                bind_params.push(format!("%{}%", pattern));
            }
            if let Some(language) = filter.language {
                query.push_str(" AND language = ?");
                bind_params.push(language);
            }
            if let Some(min_size) = filter.min_size {
                query.push_str(" AND size >= ?");
                bind_params.push(format!("{}", min_size));
            }
            if let Some(max_size) = filter.max_size {
                query.push_str(" AND size <= ?");
                bind_params.push(format!("{}", max_size));
            }
            if let Some(min_time) = filter.min_modified_time {
                query.push_str(" AND modified_at >= ?");
                bind_params.push(format!("{}", min_time));
            }
            if let Some(max_time) = filter.max_modified_time {
                query.push_str(" AND modified_at <= ?");
                bind_params.push(format!("{}", max_time));
            }
            if let Some(content) = filter.content_contains {
                query.push_str(" AND content LIKE ?");
                bind_params.push(format!("%{}%", content));
            }
        }

        // Apply sorting
        if let Some(sort) = sort {
            let order_by = match sort {
                FileSort::Path => "path",
                FileSort::Language => "language",
                FileSort::Size => "size",
                FileSort::ModifiedAt => "modified_at",
                FileSort::CreatedAt => "created_at",
            };
            query.push_str(&format!(" ORDER BY {}", order_by));
        }

        // Apply pagination
        if let Some(pagination) = pagination {
            query.push_str(&format!(
                " LIMIT {} OFFSET {}",
                pagination.limit, pagination.offset
            ));
        }

        // Execute query
        let mut query_builder = sqlx::query(&query);
        for param in &bind_params {
            query_builder = query_builder.bind(param);
        }

        let rows = query_builder.fetch_all(&self.pool).await?;

        let mut files = Vec::new();
        for row in rows {
            files.push(FileMetadata {
                id: row.get("id"),
                path: row.get("path"),
                language: row.get("language"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                modified_at: row.get("modified_at"),
                size: row.get("size"),
            });
        }

        Ok(files)
    }

    /// Get extended file metadata with computed statistics
    pub async fn get_extended_metadata(&self, id: &str) -> Result<Option<ExtendedFileMetadata>> {
        let row = sqlx::query(
            r#"
            SELECT 
                f.id, f.path, f.language, f.content, f.created_at, f.modified_at, f.size,
                fs.line_count, fs.word_count, fs.character_count,
                fh.hash
            FROM files f
            LEFT JOIN file_statistics fs ON f.id = fs.file_id
            LEFT JOIN file_hashes fh ON f.id = fh.file_id
            WHERE f.id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            // Get tags for this file
            let tag_rows = sqlx::query("SELECT tag FROM file_tags WHERE file_id = ?")
                .bind(id)
                .fetch_all(&self.pool)
                .await?;

            let tags: Vec<String> = tag_rows.into_iter().map(|row| row.get("tag")).collect();

            let metadata = FileMetadata {
                id: row.get("id"),
                path: row.get("path"),
                language: row.get("language"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                modified_at: row.get("modified_at"),
                size: row.get("size"),
            };

            let extended = ExtendedFileMetadata {
                metadata,
                line_count: row.get::<i64, _>("line_count") as u32,
                word_count: row.get::<i64, _>("word_count") as u32,
                character_count: row.get::<i64, _>("character_count") as u32,
                hash: row.get("hash"),
                tags,
            };

            Ok(Some(extended))
        } else {
            Ok(None)
        }
    }

    /// Update file statistics (line count, word count, etc.)
    pub async fn update_statistics(
        &self,
        file_id: &str,
        line_count: u32,
        word_count: u32,
        character_count: u32,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO file_statistics (file_id, line_count, word_count, character_count)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(file_id) DO UPDATE SET
                line_count = excluded.line_count,
                word_count = excluded.word_count,
                character_count = excluded.character_count
            "#,
        )
        .bind(file_id)
        .bind(line_count as i64)
        .bind(word_count as i64)
        .bind(character_count as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Add tags to a file
    pub async fn add_tags(&self, file_id: &str, tags: &[String]) -> Result<()> {
        for tag in tags {
            sqlx::query("INSERT OR IGNORE INTO file_tags (file_id, tag) VALUES (?, ?)")
                .bind(file_id)
                .bind(tag)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Remove tags from a file
    pub async fn remove_tags(&self, file_id: &str, tags: &[String]) -> Result<()> {
        for tag in tags {
            sqlx::query("DELETE FROM file_tags WHERE file_id = ? AND tag = ?")
                .bind(file_id)
                .bind(tag)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Get all tags for a file
    pub async fn get_tags(&self, file_id: &str) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT tag FROM file_tags WHERE file_id = ?")
            .bind(file_id)
            .fetch_all(&self.pool)
            .await?;

        let tags: Vec<String> = rows.into_iter().map(|row| row.get("tag")).collect();
        Ok(tags)
    }

    /// Set file hash
    pub async fn set_hash(&self, file_id: &str, hash: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO file_hashes (file_id, hash)
            VALUES (?, ?)
            ON CONFLICT(file_id) DO UPDATE SET hash = excluded.hash
            "#,
        )
        .bind(file_id)
        .bind(hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get file hash
    pub async fn get_hash(&self, file_id: &str) -> Result<Option<String>> {
        let row = sqlx::query("SELECT hash FROM file_hashes WHERE file_id = ?")
            .bind(file_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = row {
            Ok(Some(row.get("hash")))
        } else {
            Ok(None)
        }
    }

    /// Search for files by content
    pub async fn search_content(&self, query: &str, limit: u32) -> Result<Vec<FileMetadata>> {
        let rows = sqlx::query(
            r#"
            SELECT id, path, language, content, created_at, modified_at, size
            FROM files
            WHERE content LIKE ?
            ORDER BY modified_at DESC
            LIMIT ?
            "#,
        )
        .bind(format!("%{}%", query))
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut files = Vec::new();
        for row in rows {
            files.push(FileMetadata {
                id: row.get("id"),
                path: row.get("path"),
                language: row.get("language"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                modified_at: row.get("modified_at"),
                size: row.get("size"),
            });
        }

        Ok(files)
    }

    /// Get files by language
    pub async fn get_files_by_language(&self, language: &str) -> Result<Vec<FileMetadata>> {
        let rows = sqlx::query(
            r#"
            SELECT id, path, language, content, created_at, modified_at, size
            FROM files
            WHERE language = ?
            ORDER BY path
            "#,
        )
        .bind(language)
        .fetch_all(&self.pool)
        .await?;

        let mut files = Vec::new();
        for row in rows {
            files.push(FileMetadata {
                id: row.get("id"),
                path: row.get("path"),
                language: row.get("language"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                modified_at: row.get("modified_at"),
                size: row.get("size"),
            });
        }

        Ok(files)
    }

    /// Get file count by language
    pub async fn get_file_count_by_language(&self) -> Result<HashMap<String, u32>> {
        let rows = sqlx::query("SELECT language, COUNT(*) as count FROM files GROUP BY language")
            .fetch_all(&self.pool)
            .await?;

        let mut counts = HashMap::new();
        for row in rows {
            let language: String = row.get("language");
            let count: i64 = row.get("count");
            counts.insert(language, count as u32);
        }

        Ok(counts)
    }

    /// Get total file count
    pub async fn get_total_file_count(&self) -> Result<u32> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
            .fetch_one(&self.pool)
            .await?;

        Ok(count as u32)
    }

    /// Get total size of all files
    pub async fn get_total_size(&self) -> Result<u64> {
        let size: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(size), 0) FROM files")
            .fetch_one(&self.pool)
            .await?;

        Ok(size as u64)
    }

    /// Batch update file modification times
    pub async fn batch_update_modification_times(
        &self,
        file_updates: &[(&str, i64)],
    ) -> Result<()> {
        for (file_id, modified_at) in file_updates {
            sqlx::query("UPDATE files SET modified_at = ? WHERE id = ?")
                .bind(modified_at)
                .bind(file_id)
                .execute(&self.pool)
                .await?;
        }

        Ok(())
    }

    /// Get recently modified files
    pub async fn get_recently_modified_files(&self, limit: u32) -> Result<Vec<FileMetadata>> {
        let rows = sqlx::query(
            r#"
            SELECT id, path, language, content, created_at, modified_at, size
            FROM files
            ORDER BY modified_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut files = Vec::new();
        for row in rows {
            files.push(FileMetadata {
                id: row.get("id"),
                path: row.get("path"),
                language: row.get("language"),
                content: row.get("content"),
                created_at: row.get("created_at"),
                modified_at: row.get("modified_at"),
                size: row.get("size"),
            });
        }

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use tempfile::TempDir;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_file_metadata_creation() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        let metadata = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/test/file.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() { println!(\"Hello\"); }".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 35,
        };

        manager.create_file(metadata.clone()).await.unwrap();

        let retrieved = manager.get_file_by_id(&metadata.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.path, "/test/file.rs");
        assert_eq!(retrieved.language, "rust");
        assert_eq!(retrieved.size, 35);
    }

    #[tokio::test]
    async fn test_file_metadata_update() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        let mut metadata = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/test/file.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() { println!(\"Hello\"); }".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 35,
        };

        manager.create_file(metadata.clone()).await.unwrap();

        // Update the metadata
        metadata.content = Some("fn main() { println!(\"Hello, world!\"); }".to_string());
        metadata.modified_at = 1234567891;
        metadata.size = 42;

        let updated = manager.update_file(metadata.clone()).await.unwrap();
        assert!(updated);

        let retrieved = manager.get_file_by_id(&metadata.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(
            retrieved.content,
            Some("fn main() { println!(\"Hello, world!\"); }".to_string())
        );
        assert_eq!(retrieved.size, 42);
    }

    #[tokio::test]
    async fn test_file_metadata_deletion() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        let metadata = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/test/file.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() { println!(\"Hello\"); }".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 35,
        };

        manager.create_file(metadata.clone()).await.unwrap();

        // Delete the file
        let deleted = manager.delete_file(&metadata.id).await.unwrap();
        assert!(deleted);

        // Verify it's gone
        let retrieved = manager.get_file_by_id(&metadata.id).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_file_listing_with_filters() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        // Create test files
        let metadata1 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/main.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() { println!(\"Hello\"); }".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 35,
        };

        let metadata2 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/lib.rs".to_string(),
            language: "rust".to_string(),
            content: Some("pub fn hello() {}".to_string()),
            created_at: 1234567891,
            modified_at: 1234567891,
            size: 18,
        };

        let metadata3 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/test/test.js".to_string(),
            language: "javascript".to_string(),
            content: Some("console.log('Hello');".to_string()),
            created_at: 1234567892,
            modified_at: 1234567892,
            size: 23,
        };

        manager.create_file(metadata1).await.unwrap();
        manager.create_file(metadata2).await.unwrap();
        manager.create_file(metadata3).await.unwrap();

        // List all files
        let all_files = manager.list_files(None, None, None).await.unwrap();
        assert_eq!(all_files.len(), 3);

        // Filter by language
        let filter = FileFilter {
            language: Some("rust".to_string()),
            ..Default::default()
        };
        let rust_files = manager.list_files(Some(filter), None, None).await.unwrap();
        assert_eq!(rust_files.len(), 2);

        // Filter by path pattern
        let filter = FileFilter {
            path_pattern: Some("main".to_string()),
            ..Default::default()
        };
        let main_files = manager.list_files(Some(filter), None, None).await.unwrap();
        assert_eq!(main_files.len(), 1);
    }

    #[tokio::test]
    async fn test_file_statistics() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        let metadata = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/test/file.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() {\n    println!(\"Hello\");\n}".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 37,
        };

        manager.create_file(metadata.clone()).await.unwrap();

        // Update statistics
        manager
            .update_statistics(&metadata.id, 3, 7, 37)
            .await
            .unwrap();

        // Get extended metadata
        let extended = manager.get_extended_metadata(&metadata.id).await.unwrap();
        assert!(extended.is_some());
        let extended = extended.unwrap();
        assert_eq!(extended.line_count, 3);
        assert_eq!(extended.word_count, 7);
        assert_eq!(extended.character_count, 37);
    }

    #[tokio::test]
    async fn test_file_tags() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        let metadata = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/test/file.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() { println!(\"Hello\"); }".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 35,
        };

        manager.create_file(metadata.clone()).await.unwrap();

        // Add tags
        let tags = vec!["important".to_string(), "review".to_string()];
        manager.add_tags(&metadata.id, &tags).await.unwrap();

        // Get tags
        let retrieved_tags = manager.get_tags(&metadata.id).await.unwrap();
        assert_eq!(retrieved_tags.len(), 2);
        assert!(retrieved_tags.contains(&"important".to_string()));
        assert!(retrieved_tags.contains(&"review".to_string()));

        // Remove a tag
        manager
            .remove_tags(&metadata.id, &["review".to_string()])
            .await
            .unwrap();

        // Verify tag was removed
        let retrieved_tags = manager.get_tags(&metadata.id).await.unwrap();
        assert_eq!(retrieved_tags.len(), 1);
        assert!(retrieved_tags.contains(&"important".to_string()));
    }

    #[tokio::test]
    async fn test_file_hashing() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        let metadata = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/test/file.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() { println!(\"Hello\"); }".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 35,
        };

        manager.create_file(metadata.clone()).await.unwrap();

        // Set hash
        let hash = "abc123def456";
        manager.set_hash(&metadata.id, hash).await.unwrap();

        // Get hash
        let retrieved_hash = manager.get_hash(&metadata.id).await.unwrap();
        assert!(retrieved_hash.is_some());
        assert_eq!(retrieved_hash.unwrap(), hash);
    }

    #[tokio::test]
    async fn test_content_search() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        let metadata1 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/main.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() { println!(\"Hello, world!\"); }".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 42,
        };

        let metadata2 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/lib.rs".to_string(),
            language: "rust".to_string(),
            content: Some("pub fn hello() { println!(\"Greetings\"); }".to_string()),
            created_at: 1234567891,
            modified_at: 1234567891,
            size: 45,
        };

        manager.create_file(metadata1).await.unwrap();
        manager.create_file(metadata2).await.unwrap();

        // Search for content
        let results = manager.search_content("println", 10).await.unwrap();
        assert_eq!(results.len(), 2);

        let results = manager.search_content("Greetings", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_language_filtering() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        // Create files in different languages
        let rust_file = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/main.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() {}".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 13,
        };

        let js_file = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/script.js".to_string(),
            language: "javascript".to_string(),
            content: Some("console.log('Hello');".to_string()),
            created_at: 1234567891,
            modified_at: 1234567891,
            size: 23,
        };

        manager.create_file(rust_file).await.unwrap();
        manager.create_file(js_file).await.unwrap();

        // Get files by language
        let rust_files = manager.get_files_by_language("rust").await.unwrap();
        assert_eq!(rust_files.len(), 1);
        assert_eq!(rust_files[0].path, "/src/main.rs");

        let js_files = manager.get_files_by_language("javascript").await.unwrap();
        assert_eq!(js_files.len(), 1);
        assert_eq!(js_files[0].path, "/src/script.js");
    }

    #[tokio::test]
    async fn test_file_counts_and_sizes() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        // Create test files
        let metadata1 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/main.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() {}".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 13,
        };

        let metadata2 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/lib.rs".to_string(),
            language: "rust".to_string(),
            content: Some("pub fn hello() {}".to_string()),
            created_at: 1234567891,
            modified_at: 1234567891,
            size: 18,
        };

        let metadata3 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/script.js".to_string(),
            language: "javascript".to_string(),
            content: Some("console.log('Hello');".to_string()),
            created_at: 1234567892,
            modified_at: 1234567892,
            size: 23,
        };

        manager.create_file(metadata1).await.unwrap();
        manager.create_file(metadata2).await.unwrap();
        manager.create_file(metadata3).await.unwrap();

        // Get total file count
        let total_count = manager.get_total_file_count().await.unwrap();
        assert_eq!(total_count, 3);

        // Get total size
        let total_size = manager.get_total_size().await.unwrap();
        assert_eq!(total_size, 13 + 18 + 23);

        // Get file count by language
        let counts = manager.get_file_count_by_language().await.unwrap();
        assert_eq!(counts.get("rust"), Some(&2));
        assert_eq!(counts.get("javascript"), Some(&1));
    }

    #[tokio::test]
    async fn test_batch_operations() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        // Create test files
        let metadata1 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/main.rs".to_string(),
            language: "rust".to_string(),
            content: Some("fn main() {}".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 13,
        };

        let metadata2 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/lib.rs".to_string(),
            language: "rust".to_string(),
            content: Some("pub fn hello() {}".to_string()),
            created_at: 1234567891,
            modified_at: 1234567891,
            size: 18,
        };

        manager.create_file(metadata1.clone()).await.unwrap();
        manager.create_file(metadata2.clone()).await.unwrap();

        // Batch update modification times
        let updates = vec![
            (metadata1.id.as_str(), 1234567900),
            (metadata2.id.as_str(), 1234567901),
        ];
        manager
            .batch_update_modification_times(&updates)
            .await
            .unwrap();

        // Verify updates
        let file1 = manager
            .get_file_by_id(&metadata1.id)
            .await
            .unwrap()
            .unwrap();
        let file2 = manager
            .get_file_by_id(&metadata2.id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(file1.modified_at, 1234567900);
        assert_eq!(file2.modified_at, 1234567901);
    }

    #[tokio::test]
    async fn test_recent_files() {
        // Use in-memory database for tests - faster and more reliable
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        let manager = FileMetadataManager::new(pool.clone());
        manager.init().await.unwrap();

        // Create test files with different modification times
        let metadata1 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/oldest.rs".to_string(),
            language: "rust".to_string(),
            content: Some("// oldest".to_string()),
            created_at: 1234567890,
            modified_at: 1234567890,
            size: 10,
        };

        let metadata2 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/newer.rs".to_string(),
            language: "rust".to_string(),
            content: Some("// newer".to_string()),
            created_at: 1234567891,
            modified_at: 1234567895,
            size: 10,
        };

        let metadata3 = FileMetadata {
            id: Uuid::new_v4().to_string(),
            path: "/src/newest.rs".to_string(),
            language: "rust".to_string(),
            content: Some("// newest".to_string()),
            created_at: 1234567892,
            modified_at: 1234567900,
            size: 11,
        };

        manager.create_file(metadata1).await.unwrap();
        manager.create_file(metadata2).await.unwrap();
        manager.create_file(metadata3).await.unwrap();

        // Get recently modified files
        let recent_files = manager.get_recently_modified_files(2).await.unwrap();
        assert_eq!(recent_files.len(), 2);
        // Should be ordered by modification time, descending
        assert_eq!(recent_files[0].path, "/src/newest.rs");
        assert_eq!(recent_files[1].path, "/src/newer.rs");
    }
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            path_pattern: None,
            language: None,
            min_size: None,
            max_size: None,
            min_modified_time: None,
            max_modified_time: None,
            tags: None,
            content_contains: None,
        }
    }
}
