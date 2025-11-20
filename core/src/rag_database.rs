//! RAG (Retrieval-Augmented Generation) database module for OdinCode
//!
//! This module provides semantic search capabilities using vector storage
//! to enable intelligent code retrieval and context building for AI models.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

/// Code chunk representing a semantically meaningful piece of code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub id: String,
    pub file_path: String,
    pub chunk_type: ChunkType,
    pub content: String,
    pub start_line: u32,
    pub end_line: u32,
    pub embedding: Option<Vec<f32>>,   // Optional vector embedding
    pub semantic_hash: Option<String>, // Hash for semantic similarity
    pub metadata: HashMap<String, String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Types of code chunks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChunkType {
    Function,
    Class,
    Method,
    Module,
    Block,
    Statement,
    Expression,
    Comment,
    Documentation,
    Test,
}

impl ChunkType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChunkType::Function => "function",
            ChunkType::Class => "class",
            ChunkType::Method => "method",
            ChunkType::Module => "module",
            ChunkType::Block => "block",
            ChunkType::Statement => "statement",
            ChunkType::Expression => "expression",
            ChunkType::Comment => "comment",
            ChunkType::Documentation => "documentation",
            ChunkType::Test => "test",
        }
    }
}

/// Search result from RAG queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub chunk: CodeChunk,
    pub similarity_score: f32,
    pub rank: u32,
}

/// RAG database manager for semantic code search
pub struct RagDatabase {
    pool: SqlitePool,
}

impl RagDatabase {
    /// Create a new RAG database instance
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Initialize the RAG database with required tables and indexes
    pub async fn init(&self) -> Result<()> {
        // Create code chunks table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS code_chunks (
                id TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                chunk_type TEXT NOT NULL,
                content TEXT NOT NULL,
                start_line INTEGER NOT NULL,
                end_line INTEGER NOT NULL,
                embedding BLOB,  -- Serialized vector embedding
                semantic_hash TEXT,  -- For exact semantic matching
                metadata TEXT,  -- JSON serialized metadata
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes for performance
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chunks_file ON code_chunks(file_path)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chunks_type ON code_chunks(chunk_type)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_chunks_hash ON code_chunks(semantic_hash)")
            .execute(&self.pool)
            .await?;

        // Full-text search index for content
        sqlx::query(
            "CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(content, chunk_id, tokenize='porter')"
        ).execute(&self.pool).await?;

        Ok(())
    }

    /// Store a code chunk in the database
    pub async fn store_chunk(&self, chunk: CodeChunk) -> Result<()> {
        let metadata_json = serde_json::to_string(&chunk.metadata)?;
        let embedding_blob = chunk.embedding.as_ref().map(|v| {
            // Serialize vector as binary
            bincode::serialize(v).unwrap_or_default()
        });

        // Insert the chunk
        sqlx::query(
            r#"
            INSERT INTO code_chunks 
            (id, file_path, chunk_type, content, start_line, end_line, embedding, semantic_hash, metadata, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&chunk.id)
        .bind(&chunk.file_path)
        .bind(chunk.chunk_type.as_str())
        .bind(&chunk.content)
        .bind(chunk.start_line as i64)
        .bind(chunk.end_line as i64)
        .bind(embedding_blob.as_deref())
        .bind(&chunk.semantic_hash)
        .bind(&metadata_json)
        .bind(chunk.created_at)
        .bind(chunk.updated_at)
        .execute(&self.pool)
        .await?;

        // Also insert into FTS table for text search
        sqlx::query("INSERT INTO chunks_fts (content, chunk_id) VALUES (?, ?)")
            .bind(&chunk.content)
            .bind(&chunk.id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Get a code chunk by ID
    pub async fn get_chunk(&self, chunk_id: &str) -> Result<Option<CodeChunk>> {
        let row = sqlx::query(
            r#"
            SELECT id, file_path, chunk_type, content, start_line, end_line, 
                   embedding, semantic_hash, metadata, created_at, updated_at
            FROM code_chunks
            WHERE id = ?
            "#,
        )
        .bind(chunk_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            let embedding: Option<Vec<u8>> = row.get("embedding");
            let embedding = if let Some(embedding_bytes) = embedding {
                Some(bincode::deserialize(&embedding_bytes)?)
            } else {
                None
            };

            let metadata: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();

            Ok(Some(CodeChunk {
                id: row.get("id"),
                file_path: row.get("file_path"),
                chunk_type: self.str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
                content: row.get("content"),
                start_line: row.get::<i64, _>("start_line") as u32,
                end_line: row.get::<i64, _>("end_line") as u32,
                embedding,
                semantic_hash: row.get("semantic_hash"),
                metadata,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Search for code chunks using semantic similarity
    pub async fn semantic_search(
        &self,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<SearchHit>> {
        // In a real implementation, we would use a proper vector similarity search
        // For now, we'll return chunks with a dummy similarity calculation
        // This is a simplified version - in practice, you'd use a vector database
        // or implement cosine similarity in SQL

        let rows = sqlx::query(
            r#"
            SELECT id, file_path, chunk_type, content, start_line, end_line, 
                   embedding, semantic_hash, metadata, created_at, updated_at
            FROM code_chunks
            WHERE embedding IS NOT NULL
            ORDER BY id  -- Placeholder ordering
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut hits = Vec::new();
        for (rank, row) in rows.iter().enumerate() {
            let embedding: Option<Vec<u8>> = row.get("embedding");
            let embedding = if let Some(embedding_bytes) = embedding {
                Some(bincode::deserialize(&embedding_bytes)?)
            } else {
                None
            };

            // Calculate dummy similarity (in real implementation, this would be proper vector similarity)
            let similarity = embedding
                .as_ref()
                .map(|emb: &Vec<f32>| self.calculate_similarity(query_embedding, emb))
                .unwrap_or(0.0);

            let metadata: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();

            hits.push(SearchHit {
                chunk: CodeChunk {
                    id: row.get("id"),
                    file_path: row.get("file_path"),
                    chunk_type: self.str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
                    content: row.get("content"),
                    start_line: row.get::<i64, _>("start_line") as u32,
                    end_line: row.get::<i64, _>("end_line") as u32,
                    embedding,
                    semantic_hash: row.get("semantic_hash"),
                    metadata,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                },
                similarity_score: similarity,
                rank: rank as u32 + 1,
            });
        }

        // Sort by similarity score in descending order
        hits.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Update ranks after sorting
        for (i, hit) in hits.iter_mut().enumerate() {
            hit.rank = i as u32 + 1;
        }

        Ok(hits)
    }

    /// Search for code chunks using text-based search (FTS)
    pub async fn text_search(&self, query: &str, limit: u32) -> Result<Vec<SearchHit>> {
        let rows = sqlx::query(
            r#"
            SELECT c.id, c.file_path, c.chunk_type, c.content, c.start_line, c.end_line, 
                   c.embedding, c.semantic_hash, c.metadata, c.created_at, c.updated_at
            FROM code_chunks c
            JOIN chunks_fts f ON c.id = f.chunk_id
            WHERE f.content MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )
        .bind(query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut hits = Vec::new();
        for (rank, row) in rows.iter().enumerate() {
            let embedding: Option<Vec<u8>> = row.get("embedding");
            let embedding = if let Some(embedding_bytes) = embedding {
                Some(bincode::deserialize(&embedding_bytes)?)
            } else {
                None
            };

            let metadata: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();

            hits.push(SearchHit {
                chunk: CodeChunk {
                    id: row.get("id"),
                    file_path: row.get("file_path"),
                    chunk_type: self.str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
                    content: row.get("content"),
                    start_line: row.get::<i64, _>("start_line") as u32,
                    end_line: row.get::<i64, _>("end_line") as u32,
                    embedding,
                    semantic_hash: row.get("semantic_hash"),
                    metadata,
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                },
                similarity_score: 1.0, // FTS doesn't provide similarity scores
                rank: rank as u32 + 1,
            });
        }

        Ok(hits)
    }

    /// Search using both semantic and text-based approaches
    pub async fn hybrid_search(
        &self,
        query: &str,
        query_embedding: &[f32],
        limit: u32,
    ) -> Result<Vec<SearchHit>> {
        // Get semantic search results
        let semantic_results = self.semantic_search(query_embedding, limit).await?;

        // Get text search results
        let text_results = self.text_search(query, limit).await?;

        // Combine and rank results (simplified approach)
        let mut combined_results = HashMap::new();

        // Add semantic results with weights
        for hit in semantic_results {
            combined_results.insert(hit.chunk.id.clone(), (hit.similarity_score * 0.7, hit));
        }

        // Add text results with weights
        for hit in text_results {
            let score = hit.similarity_score * 0.3; // Lower weight for text results
            if let Some((existing_score, _)) = combined_results.get(&hit.chunk.id) {
                // Update with combined score if it exists
                combined_results.insert(hit.chunk.id.clone(), (existing_score + score, hit));
            } else {
                combined_results.insert(hit.chunk.id.clone(), (score, hit));
            }
        }

        // Convert to vector and sort
        let mut results: Vec<SearchHit> =
            combined_results.into_values().map(|(_, hit)| hit).collect();

        results.sort_by(|a, b| {
            b.similarity_score
                .partial_cmp(&a.similarity_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Update ranks
        for (i, hit) in results.iter_mut().enumerate() {
            hit.rank = i as u32 + 1;
        }

        // Limit results
        results.truncate(limit as usize);

        Ok(results)
    }

    /// Get all chunks for a specific file
    pub async fn get_chunks_for_file(&self, file_path: &str) -> Result<Vec<CodeChunk>> {
        let rows = sqlx::query(
            r#"
            SELECT id, file_path, chunk_type, content, start_line, end_line, 
                   embedding, semantic_hash, metadata, created_at, updated_at
            FROM code_chunks
            WHERE file_path = ?
            ORDER BY start_line
            "#,
        )
        .bind(file_path)
        .fetch_all(&self.pool)
        .await?;

        let mut chunks = Vec::new();
        for row in rows {
            let embedding: Option<Vec<u8>> = row.get("embedding");
            let embedding = if let Some(embedding_bytes) = embedding {
                Some(bincode::deserialize(&embedding_bytes)?)
            } else {
                None
            };

            let metadata: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();

            chunks.push(CodeChunk {
                id: row.get("id"),
                file_path: row.get("file_path"),
                chunk_type: self.str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
                content: row.get("content"),
                start_line: row.get::<i64, _>("start_line") as u32,
                end_line: row.get::<i64, _>("end_line") as u32,
                embedding,
                semantic_hash: row.get("semantic_hash"),
                metadata,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(chunks)
    }

    /// Calculate cosine similarity between two vectors
    fn calculate_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Convert string to ChunkType
    fn str_to_chunk_type(&self, s: &str) -> Result<ChunkType> {
        match s {
            "function" => Ok(ChunkType::Function),
            "class" => Ok(ChunkType::Class),
            "method" => Ok(ChunkType::Method),
            "module" => Ok(ChunkType::Module),
            "block" => Ok(ChunkType::Block),
            "statement" => Ok(ChunkType::Statement),
            "expression" => Ok(ChunkType::Expression),
            "comment" => Ok(ChunkType::Comment),
            "documentation" => Ok(ChunkType::Documentation),
            "test" => Ok(ChunkType::Test),
            _ => Err(anyhow::anyhow!("Invalid chunk type: {}", s)),
        }
    }

    /// Find similar chunks based on semantic hash (exact semantic matching)
    pub async fn find_similar_chunks(
        &self,
        semantic_hash: &str,
        limit: u32,
    ) -> Result<Vec<CodeChunk>> {
        let rows = sqlx::query(
            r#"
            SELECT id, file_path, chunk_type, content, start_line, end_line, 
                   embedding, semantic_hash, metadata, created_at, updated_at
            FROM code_chunks
            WHERE semantic_hash = ?
            LIMIT ?
            "#,
        )
        .bind(semantic_hash)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        let mut chunks = Vec::new();
        for row in rows {
            let embedding: Option<Vec<u8>> = row.get("embedding");
            let embedding = if let Some(embedding_bytes) = embedding {
                Some(bincode::deserialize(&embedding_bytes)?)
            } else {
                None
            };

            let metadata: HashMap<String, String> =
                serde_json::from_str(row.get::<&str, _>("metadata")).unwrap_or_default();

            chunks.push(CodeChunk {
                id: row.get("id"),
                file_path: row.get("file_path"),
                chunk_type: self.str_to_chunk_type(row.get::<&str, _>("chunk_type"))?,
                content: row.get("content"),
                start_line: row.get::<i64, _>("start_line") as u32,
                end_line: row.get::<i64, _>("end_line") as u32,
                embedding,
                semantic_hash: row.get("semantic_hash"),
                metadata,
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            });
        }

        Ok(chunks)
    }

    /// Update a chunk's embedding
    pub async fn update_chunk_embedding(&self, chunk_id: &str, embedding: &[f32]) -> Result<()> {
        let embedding_blob = bincode::serialize(embedding)?;

        sqlx::query("UPDATE code_chunks SET embedding = ?, updated_at = ? WHERE id = ?")
            .bind(&embedding_blob)
            .bind(chrono::Utc::now().timestamp())
            .bind(chunk_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_rag_database_creation() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let rag_db = RagDatabase::new(pool);
        rag_db.init().await.unwrap();

        // Test creating and storing a chunk
        let mut metadata = HashMap::new();
        metadata.insert("language".to_string(), "rust".to_string());
        metadata.insert("complexity".to_string(), "medium".to_string());

        let chunk = CodeChunk {
            id: "test_chunk".to_string(),
            file_path: "test.rs".to_string(),
            chunk_type: ChunkType::Function,
            content: "fn hello() { println!(\"Hello, world!\"); }".to_string(),
            start_line: 1,
            end_line: 3,
            embedding: Some(vec![0.1, 0.2, 0.3, 0.4]),
            semantic_hash: Some("hash123".to_string()),
            metadata,
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        rag_db.store_chunk(chunk.clone()).await.unwrap();
        let retrieved = rag_db.get_chunk("test_chunk").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().content,
            "fn hello() { println!(\"Hello, world!\"); }"
        );
    }

    #[tokio::test]
    async fn test_chunk_retrieval_by_file() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let rag_db = RagDatabase::new(pool);
        rag_db.init().await.unwrap();

        // Create test chunks
        let chunk1 = CodeChunk {
            id: Uuid::new_v4().to_string(),
            file_path: "test.rs".to_string(),
            chunk_type: ChunkType::Function,
            content: "fn func1() {}".to_string(),
            start_line: 1,
            end_line: 1,
            embedding: None,
            semantic_hash: None,
            metadata: HashMap::new(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        let chunk2 = CodeChunk {
            id: Uuid::new_v4().to_string(),
            file_path: "test.rs".to_string(),
            chunk_type: ChunkType::Function,
            content: "fn func2() {}".to_string(),
            start_line: 2,
            end_line: 2,
            embedding: None,
            semantic_hash: None,
            metadata: HashMap::new(),
            created_at: 1234567890,
            updated_at: 1234567890,
        };

        rag_db.store_chunk(chunk1).await.unwrap();
        rag_db.store_chunk(chunk2).await.unwrap();

        let chunks = rag_db.get_chunks_for_file("test.rs").await.unwrap();
        assert_eq!(chunks.len(), 2);

        // Verify they're ordered by line number
        assert_eq!(chunks[0].start_line, 1);
        assert_eq!(chunks[1].start_line, 2);
    }

    #[tokio::test]
    async fn test_similarity_calculation() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let rag_db = RagDatabase::new(pool);
        rag_db.init().await.unwrap();

        // Test similarity calculation
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0]; // Same vector, should be 1.0 similarity
        let similarity = rag_db.calculate_similarity(&vec1, &vec2);
        assert!((similarity - 1.0).abs() < 0.001);

        let vec3 = vec![0.0, 1.0, 0.0]; // Orthogonal vector, should be 0.0 similarity
        let similarity2 = rag_db.calculate_similarity(&vec1, &vec3);
        assert!(similarity2 < 0.001);
    }
}
