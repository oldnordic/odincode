//! Database statistics collection
//!
//! Provides aggregated statistics across all three databases
//! (metadata, graph, and RAG).

use anyhow::Result;

use super::query_interface::DatabaseQueryInterface;
use super::query_types::{DatabaseStats, QueryResult};

impl DatabaseQueryInterface {
    /// Get database statistics
    pub async fn get_stats(&self) -> Result<QueryResult> {
        // Count files
        let files_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files")
            .fetch_one(self.db_manager.metadata_pool())
            .await?;

        // Count symbols
        let symbols_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM symbols")
            .fetch_one(self.db_manager.metadata_pool())
            .await?;

        // Count graph nodes
        let nodes_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM graph_nodes")
            .fetch_one(self.db_manager.graph_pool())
            .await?;

        // Count graph relationships
        let relationships_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM graph_relationships")
                .fetch_one(self.db_manager.graph_pool())
                .await?;

        // Count code chunks
        let chunks_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM code_chunks")
            .fetch_one(self.db_manager.rag_pool())
            .await?;

        // Get database sizes (simplified)
        let metadata_db_size = 0; // Would need to query PRAGMA for actual size
        let graph_db_size = 0; // Would need to query PRAGMA for actual size
        let rag_db_size = 0; // Would need to query PRAGMA for actual size

        let stats = DatabaseStats {
            total_files: files_count as u32,
            total_symbols: symbols_count as u32,
            total_nodes: nodes_count as u32,
            total_relationships: relationships_count as u32,
            total_chunks: chunks_count as u32,
            metadata_db_size_mb: metadata_db_size,
            graph_db_size_mb: graph_db_size,
            rag_db_size_mb: rag_db_size,
        };

        Ok(QueryResult::Stats(stats))
    }
}
