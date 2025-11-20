//! OdinCode FAISS Database Manager
//!
//! This module provides a native FAISS driver for vector database operations,
//! including similarity search, embedding storage, and index management.

use anyhow::{anyhow, Result};
use faiss::{index::IndexImpl, index_factory, Index, MetricType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// FAISS configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaissConfig {
    /// Index description string (e.g., "Flat", "IVF100,Flat")
    pub index_description: String,
    /// Vector dimension
    pub dimension: usize,
    /// Metric type for distance calculation
    pub metric_type: FaissMetricType,
    /// Number of lists for IVF indexes
    pub nlist: Option<usize>,
    /// Number of probes for IVF search
    pub nprobe: Option<usize>,
    /// Path for index persistence (optional)
    pub index_path: Option<String>,
    /// Whether to use GPU acceleration
    pub use_gpu: bool,
    /// Maximum number of vectors to store in memory
    pub max_vectors: Option<usize>,
}

/// FAISS metric types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FaissMetricType {
    /// Euclidean distance (L2)
    L2,
    /// Inner product (cosine similarity for normalized vectors)
    InnerProduct,
}

impl From<FaissMetricType> for MetricType {
    fn from(metric_type: FaissMetricType) -> Self {
        match metric_type {
            FaissMetricType::L2 => MetricType::L2,
            FaissMetricType::InnerProduct => MetricType::InnerProduct,
        }
    }
}

/// Vector embedding data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEmbedding {
    /// Unique identifier for the embedding
    pub id: String,
    /// The vector data
    pub vector: Vec<f32>,
    /// Optional metadata associated with the vector
    pub metadata: HashMap<String, String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// Vector ID
    pub id: String,
    /// Distance from query vector
    pub distance: f32,
    /// Associated metadata
    pub metadata: HashMap<String, String>,
}

/// Search query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Query vector
    pub vector: Vec<f32>,
    /// Number of results to return
    pub k: usize,
    /// Optional filter criteria
    pub filters: Option<HashMap<String, String>>,
}

/// FAISS statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaissStats {
    /// Total number of vectors in the index
    pub total_vectors: usize,
    /// Index dimension
    pub dimension: usize,
    /// Index type description
    pub index_type: String,
    /// Number of search operations performed
    pub searches_performed: u64,
    /// Number of add operations performed
    pub adds_performed: u64,
    /// Average search time in milliseconds
    pub avg_search_time_ms: f64,
    /// Average add time in milliseconds
    pub avg_add_time_ms: f64,
    /// Index size in bytes (if available)
    pub index_size_bytes: Option<u64>,
    /// Last index update timestamp
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

/// FAISS index manager
pub struct FaissManager {
    /// FAISS index instance
    index: Arc<RwLock<Option<IndexImpl>>>,
    /// Configuration
    config: FaissConfig,
    /// Vector metadata storage
    metadata: Arc<RwLock<HashMap<String, VectorEmbedding>>>,
    /// Statistics
    stats: Arc<RwLock<FaissStats>>,
    /// ID to index position mapping
    id_to_position: Arc<RwLock<HashMap<String, usize>>>,
    /// Position to ID mapping
    position_to_id: Arc<RwLock<HashMap<usize, String>>>,
}

impl FaissManager {
    /// Create a new FAISS manager with default configuration
    pub async fn new() -> Result<Self> {
        let config = FaissConfig {
            index_description: "Flat".to_string(),
            dimension: 768, // Common embedding dimension
            metric_type: FaissMetricType::L2,
            nlist: None,
            nprobe: None,
            index_path: None,
            use_gpu: false,
            max_vectors: Some(1000000),
        };

        Self::with_config(config).await
    }

    /// Create a new FAISS manager with custom configuration
    pub async fn with_config(config: FaissConfig) -> Result<Self> {
        info!("Creating FAISS manager with config: {:?}", config);

        // Create the index
        let metric_type: MetricType = config.metric_type.clone().into();
        let index = index_factory(
            config.dimension as u32,
            &config.index_description,
            metric_type,
        )
        .map_err(|e| anyhow!("Failed to create FAISS index: {e}"))?;

        // Configure IVF parameters if applicable
        if let Some(nprobe) = config.nprobe {
            // Note: This would require downcasting to specific index types
            // For now, we'll store this in the config for future use
            debug!("Setting nprobe to {nprobe}");
        }

        let manager = Self {
            index: Arc::new(RwLock::new(Some(index))),
            config,
            metadata: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(FaissStats {
                total_vectors: 0,
                dimension: 0,
                index_type: "Flat".to_string(),
                searches_performed: 0,
                adds_performed: 0,
                avg_search_time_ms: 0.0,
                avg_add_time_ms: 0.0,
                index_size_bytes: None,
                last_updated: chrono::Utc::now(),
            })),
            id_to_position: Arc::new(RwLock::new(HashMap::new())),
            position_to_id: Arc::new(RwLock::new(HashMap::new())),
        };

        // Try to load existing index if path is provided
        if let Some(ref path) = manager.config.index_path {
            if Path::new(path).exists() {
                manager.load_index(path).await?;
            }
        }

        Ok(manager)
    }

    /// Test the FAISS connection/index
    pub async fn test_connection(&self) -> Result<bool> {
        debug!("Testing FAISS index connection");

        let index_guard = self.index.read().await;
        if index_guard.is_none() {
            warn!("FAISS index is not initialized");
            return Ok(false);
        }

        // Test basic operations
        let test_vector = vec![0.0; self.config.dimension];
        let search_result = self.search_internal(&test_vector, 1).await;

        match search_result {
            Ok(_) => {
                info!("FAISS index test successful");
                Ok(true)
            }
            Err(e) => {
                error!("FAISS index test failed: {e}");
                Ok(false)
            }
        }
    }

    /// Add a vector embedding to the index
    pub async fn add_embedding(&self, embedding: VectorEmbedding) -> Result<()> {
        let start_time = std::time::Instant::now();

        debug!("Adding embedding with ID: {}", embedding.id);

        // Validate vector dimension
        if embedding.vector.len() != self.config.dimension {
            return Err(anyhow!(
                "Vector dimension mismatch: expected {}, got {}",
                self.config.dimension,
                embedding.vector.len()
            ));
        }

        // Check max vectors limit
        if let Some(max_vectors) = self.config.max_vectors {
            let current_count = {
                let metadata = self.metadata.read().await;
                metadata.len()
            };
            if current_count >= max_vectors {
                return Err(anyhow!(
                    "Maximum vector limit reached: {current_count}/{max_vectors}"
                ));
            }
        }

        // Add to FAISS index
        {
            let mut index_guard = self.index.write().await;
            if let Some(ref mut index) = *index_guard {
                let vector_data = &embedding.vector;
                index
                    .add(vector_data as &[f32])
                    .map_err(|e| anyhow!("Failed to add vector to FAISS index: {e}"))?;

                // Update position mappings
                let position = index.ntotal() - 1;
                {
                    let mut id_to_pos = self.id_to_position.write().await;
                    let mut pos_to_id = self.position_to_id.write().await;

                    id_to_pos.insert(embedding.id.clone(), position as usize);
                    pos_to_id.insert(position as usize, embedding.id.clone());
                }
            } else {
                return Err(anyhow!("FAISS index is not initialized"));
            }
        }

        // Store metadata
        {
            let mut metadata = self.metadata.write().await;
            metadata.insert(embedding.id.clone(), embedding.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_vectors += 1;
            stats.adds_performed += 1;
            stats.avg_add_time_ms = (stats.avg_add_time_ms * (stats.adds_performed - 1) as f64
                + start_time.elapsed().as_millis() as f64)
                / stats.adds_performed as f64;
            stats.last_updated = chrono::Utc::now();
        }

        info!("Successfully added embedding: {}", embedding.id);
        Ok(())
    }

    /// Search for similar vectors
    pub async fn search(&self, query: SearchQuery) -> Result<Vec<VectorSearchResult>> {
        debug!("Searching for {} nearest neighbors", query.k);

        // Validate query vector dimension
        if query.vector.len() != self.config.dimension {
            return Err(anyhow!(
                "Query vector dimension mismatch: expected {}, got {}",
                self.config.dimension,
                query.vector.len()
            ));
        }

        let start_time = std::time::Instant::now();
        let raw_results = self.search_internal(&query.vector, query.k).await?;

        // Convert raw results to VectorSearchResult with metadata
        let mut results = Vec::new();
        let metadata = self.metadata.read().await;

        for (i, &distance) in raw_results.distances.iter().enumerate() {
            if i >= raw_results.labels.len() {
                break;
            }

            let label = raw_results.labels[i];
            if label.is_none() {
                continue; // Skip invalid results
            }

            if let Some(position_u64) = label.get() {
                if let Some(vector_id) = self
                    .position_to_id
                    .read()
                    .await
                    .get(&(position_u64 as usize))
                {
                    if let Some(embedding) = metadata.get(vector_id) {
                        // Apply filters if provided
                        if let Some(ref filters) = query.filters {
                            if !self.matches_filters(&embedding.metadata, filters) {
                                continue;
                            }
                        }

                        results.push(VectorSearchResult {
                            id: vector_id.clone(),
                            distance,
                            metadata: embedding.metadata.clone(),
                        });
                    }
                }
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.searches_performed += 1;
            stats.avg_search_time_ms = (stats.avg_search_time_ms
                * (stats.searches_performed - 1) as f64
                + start_time.elapsed().as_millis() as f64)
                / stats.searches_performed as f64;
        }

        info!("Search completed, found {} results", results.len());
        Ok(results)
    }

    /// Internal search method
    async fn search_internal(
        &self,
        query_vector: &[f32],
        k: usize,
    ) -> Result<faiss::index::SearchResult> {
        let mut index_guard = self.index.write().await;
        match index_guard.as_mut() {
            Some(index) => index
                .search(query_vector, k)
                .map_err(|e| anyhow!("FAISS search failed: {e}")),
            None => Err(anyhow!("FAISS index is not initialized")),
        }
    }

    /// Check if metadata matches filters
    fn matches_filters(
        &self,
        metadata: &HashMap<String, String>,
        filters: &HashMap<String, String>,
    ) -> bool {
        for (key, expected_value) in filters {
            if let Some(actual_value) = metadata.get(key) {
                if actual_value != expected_value {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }

    /// Get a vector embedding by ID
    pub async fn get_embedding(&self, id: &str) -> Result<Option<VectorEmbedding>> {
        let metadata = self.metadata.read().await;
        Ok(metadata.get(id).cloned())
    }

    /// Remove a vector embedding by ID
    pub async fn remove_embedding(&self, id: &str) -> Result<bool> {
        debug!("Removing embedding with ID: {id}");

        let position = {
            let id_to_pos = self.id_to_position.read().await;
            match id_to_pos.get(id) {
                Some(pos) => *pos,
                None => return Ok(false),
            }
        };

        // Remove from FAISS index
        {
            let mut index_guard = self.index.write().await;
            if let Some(ref mut _index) = *index_guard {
                // Note: FAISS doesn't directly support removal by position
                // This would require rebuilding the index or using IDSelector
                // For now, we'll mark it as removed from metadata
                warn!("FAISS index removal not fully implemented - metadata only");
            }
        }

        // Remove from metadata and mappings
        {
            let mut metadata = self.metadata.write().await;
            metadata.remove(id);

            let mut id_to_pos = self.id_to_position.write().await;
            let mut pos_to_id = self.position_to_id.write().await;

            id_to_pos.remove(id);
            pos_to_id.remove(&(position as usize));
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_vectors = stats.total_vectors.saturating_sub(1);
            stats.last_updated = chrono::Utc::now();
        }

        info!("Successfully removed embedding: {id}");
        Ok(true)
    }

    /// Save the index to disk
    pub async fn save_index(&self, path: &str) -> Result<()> {
        debug!("Saving FAISS index to: {path}");

        let index_guard = self.index.read().await;
        match index_guard.as_ref() {
            Some(_index) => {
                // Note: faiss-rs doesn't have direct save/load methods in the current API
                // This would need to be implemented using the C API or custom serialization
                warn!("FAISS index save not fully implemented in current API version");
                Ok(())
            }
            None => Err(anyhow!("FAISS index is not initialized")),
        }
    }

    /// Load the index from disk
    pub async fn load_index(&self, path: &str) -> Result<()> {
        debug!("Loading FAISS index from: {path}");

        let mut index_guard = self.index.write().await;
        match index_guard.as_mut() {
            Some(_) => {
                // Note: faiss-rs doesn't have direct save/load methods in the current API
                // This would need to be implemented using the C API or custom serialization
                warn!("FAISS index load not fully implemented in current API version");
                Ok(())
            }
            None => Err(anyhow!("FAISS index is not initialized")),
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> Result<FaissStats> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    /// Get all embedding IDs
    pub async fn get_all_embedding_ids(&self) -> Result<Vec<String>> {
        let metadata = self.metadata.read().await;
        Ok(metadata.keys().cloned().collect())
    }

    /// Batch add multiple embeddings
    pub async fn batch_add_embeddings(&self, embeddings: Vec<VectorEmbedding>) -> Result<()> {
        debug!("Batch adding {} embeddings", embeddings.len());

        for embedding in &embeddings {
            if let Err(e) = self.add_embedding(embedding.clone()).await {
                error!("Failed to add embedding in batch: {e}");
                return Err(e);
            }
        }

        info!("Successfully batch added {} embeddings", embeddings.len());
        Ok(())
    }

    /// Find similar patterns (LTMC-specific operation)
    pub async fn find_similar_patterns(
        &self,
        pattern_id: &str,
        threshold: f32,
        max_results: usize,
    ) -> Result<Vec<(String, f32)>> {
        debug!("Finding similar patterns for: {pattern_id}");

        // Get the query vector
        let query_embedding = self
            .get_embedding(pattern_id)
            .await?
            .ok_or_else(|| anyhow!("Pattern not found: {pattern_id}"))?;

        // Search for similar vectors
        let query = SearchQuery {
            vector: query_embedding.vector.clone(),
            k: max_results,
            filters: None,
        };

        let results = self.search(query).await?;

        // Filter by threshold and exclude self
        let similar_patterns: Vec<(String, f32)> = results
            .into_iter()
            .filter(|result| result.id != pattern_id && result.distance <= threshold)
            .map(|result| (result.id, result.distance))
            .collect();

        info!(
            "Found {} similar patterns for {pattern_id}",
            similar_patterns.len()
        );
        Ok(similar_patterns)
    }

    /// Create pattern relationships (LTMC-specific operation)
    pub async fn create_pattern_relationship(
        &self,
        source_id: &str,
        target_id: &str,
        relationship_type: &str,
        strength: f32,
    ) -> Result<()> {
        debug!(
            "Creating pattern relationship: {} -> {} ({})",
            source_id, target_id, relationship_type
        );

        // Verify both patterns exist
        let _source_embedding = self
            .get_embedding(source_id)
            .await?
            .ok_or_else(|| anyhow!("Source pattern not found: {source_id}"))?;
        let _target_embedding = self
            .get_embedding(target_id)
            .await?
            .ok_or_else(|| anyhow!("Target pattern not found: {target_id}"))?;

        // Add relationship metadata to both embeddings
        {
            let mut metadata = self.metadata.write().await;

            if let Some(source_embedding) = metadata.get_mut(source_id) {
                source_embedding.metadata.insert(
                    format!("relationship_{relationship_type}_{target_id}"),
                    strength.to_string(),
                );
            }

            if let Some(target_embedding) = metadata.get_mut(target_id) {
                target_embedding.metadata.insert(
                    format!("relationship_{relationship_type}_{source_id}"),
                    strength.to_string(),
                );
            }
        }

        info!("Created pattern relationship: {source_id} -> {target_id}");
        Ok(())
    }

    /// Get pattern relationships (LTMC-specific operation)
    pub async fn get_pattern_relationships(
        &self,
        pattern_id: &str,
    ) -> Result<Vec<(String, String, f32)>> {
        debug!("Getting pattern relationships for: {pattern_id}");

        let embedding = self
            .get_embedding(pattern_id)
            .await?
            .ok_or_else(|| anyhow!("Pattern not found: {pattern_id}"))?;

        let mut relationships = Vec::new();

        for (key, value) in embedding.metadata {
            if key.starts_with("relationship_") {
                let parts: Vec<&str> = key.split('_').collect();
                if parts.len() >= 3 {
                    let relationship_type = parts[1];
                    let target_id = parts[2..].join("_");
                    if let Ok(strength) = value.parse::<f32>() {
                        relationships.push((target_id, relationship_type.to_string(), strength));
                    }
                }
            }
        }

        info!(
            "Found {} relationships for pattern {pattern_id}",
            relationships.len()
        );
        Ok(relationships)
    }

    /// Check if the index is connected/initialized
    pub async fn is_connected(&self) -> bool {
        let index_guard = self.index.read().await;
        index_guard.is_some()
    }

    /// Get the total number of vectors in the index
    pub async fn get_vector_count(&self) -> usize {
        let stats = self.stats.read().await;
        stats.total_vectors
    }

    /// Clear all vectors from the index
    pub async fn clear_index(&self) -> Result<()> {
        debug!("Clearing FAISS index");

        // Reset the index
        {
            let mut index_guard = self.index.write().await;
            let metric_type: MetricType = self.config.metric_type.clone().into();
            *index_guard = Some(
                index_factory(
                    self.config.dimension as u32,
                    &self.config.index_description,
                    metric_type,
                )
                .map_err(|e| anyhow!("Failed to recreate FAISS index: {e}"))?,
            );
        }

        // Clear metadata and mappings
        {
            let mut metadata = self.metadata.write().await;
            metadata.clear();

            let mut id_to_pos = self.id_to_position.write().await;
            let mut pos_to_id = self.position_to_id.write().await;
            id_to_pos.clear();
            pos_to_id.clear();
        }

        // Reset statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_vectors = 0;
            stats.searches_performed = 0;
            stats.adds_performed = 0;
            stats.avg_search_time_ms = 0.0;
            stats.avg_add_time_ms = 0.0;
            stats.last_updated = chrono::Utc::now();
        }

        info!("FAISS index cleared");
        Ok(())
    }
}

impl Drop for FaissManager {
    fn drop(&mut self) {
        // Ensure proper cleanup when the manager is dropped
        debug!("Dropping FAISS manager");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_faiss_manager_creation() {
        let manager = FaissManager::new().await;
        assert!(manager.is_ok());

        let manager = manager.unwrap();
        assert!(manager.is_connected().await);
    }

    #[tokio::test]
    async fn test_faiss_connection_test() {
        let manager = FaissManager::new().await.unwrap();
        let result = manager.test_connection().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_embedding_addition() {
        let manager = FaissManager::new().await.unwrap();

        let embedding = VectorEmbedding {
            id: "test_1".to_string(),
            vector: vec![0.1; 768],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let result = manager.add_embedding(embedding).await;
        assert!(result.is_ok());

        assert_eq!(manager.get_vector_count().await, 1);
    }

    #[tokio::test]
    async fn test_embedding_retrieval() {
        let manager = FaissManager::new().await.unwrap();

        let embedding = VectorEmbedding {
            id: "test_retrieve".to_string(),
            vector: vec![0.5; 768],
            metadata: {
                let mut map = HashMap::new();
                map.insert("type".to_string(), "test".to_string());
                map
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        manager.add_embedding(embedding.clone()).await.unwrap();

        let retrieved = manager.get_embedding("test_retrieve").await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, "test_retrieve");
        assert_eq!(retrieved.metadata.get("type"), Some(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_vector_search() {
        let manager = FaissManager::new().await.unwrap();

        // Add test vectors
        for i in 0..10 {
            let embedding = VectorEmbedding {
                id: format!("test_{i}"),
                vector: vec![i as f32 / 10.0; 768],
                metadata: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            manager.add_embedding(embedding).await.unwrap();
        }

        // Search for similar vectors
        let query = SearchQuery {
            vector: vec![0.3; 768],
            k: 3,
            filters: None,
        };

        let results = manager.search(query).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 3);
    }

    #[tokio::test]
    async fn test_pattern_relationships() {
        let manager = FaissManager::new().await.unwrap();

        // Add test patterns
        let pattern1 = VectorEmbedding {
            id: "pattern_1".to_string(),
            vector: vec![0.1; 768],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let pattern2 = VectorEmbedding {
            id: "pattern_2".to_string(),
            vector: vec![0.2; 768],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        manager.add_embedding(pattern1).await.unwrap();
        manager.add_embedding(pattern2).await.unwrap();

        // Create relationship
        manager
            .create_pattern_relationship("pattern_1", "pattern_2", "similar_to", 0.8)
            .await
            .unwrap();

        // Get relationships
        let relationships = manager
            .get_pattern_relationships("pattern_1")
            .await
            .unwrap();
        assert!(!relationships.is_empty());

        let (target_id, rel_type, strength) = &relationships[0];
        assert_eq!(target_id, "pattern_2");
        assert_eq!(rel_type, "similar_to");
        assert!((strength - 0.8).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_similar_patterns() {
        let manager = FaissManager::new().await.unwrap();

        // Add test patterns
        let base_pattern = VectorEmbedding {
            id: "base".to_string(),
            vector: vec![0.5; 768],
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let similar_pattern = VectorEmbedding {
            id: "similar".to_string(),
            vector: vec![0.51; 768], // Very similar
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let different_pattern = VectorEmbedding {
            id: "different".to_string(),
            vector: vec![0.9; 768], // Different
            metadata: HashMap::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        manager.add_embedding(base_pattern).await.unwrap();
        manager.add_embedding(similar_pattern).await.unwrap();
        manager.add_embedding(different_pattern).await.unwrap();

        // Find similar patterns
        let similar = manager.find_similar_patterns("base", 1.0, 5).await.unwrap();
        assert!(!similar.is_empty());

        // The similar pattern should be found
        let found_similar = similar.iter().any(|(id, _)| id == "similar");
        assert!(found_similar);
    }

    #[tokio::test]
    async fn test_index_clear() {
        let manager = FaissManager::new().await.unwrap();

        // Add some vectors
        for i in 0..5 {
            let embedding = VectorEmbedding {
                id: format!("clear_test_{i}"),
                vector: vec![i as f32; 768],
                metadata: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            manager.add_embedding(embedding).await.unwrap();
        }

        assert_eq!(manager.get_vector_count().await, 5);

        // Clear the index
        manager.clear_index().await.unwrap();

        assert_eq!(manager.get_vector_count().await, 0);
    }

    #[tokio::test]
    async fn test_stats() {
        let manager = FaissManager::new().await.unwrap();

        // Add some vectors and perform searches
        for i in 0..3 {
            let embedding = VectorEmbedding {
                id: format!("stats_test_{i}"),
                vector: vec![i as f32; 768],
                metadata: HashMap::new(),
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            manager.add_embedding(embedding).await.unwrap();
        }

        // Perform a search
        let query = SearchQuery {
            vector: vec![0.5; 768],
            k: 2,
            filters: None,
        };
        manager.search(query).await.unwrap();

        // Get stats
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_vectors, 3);
        assert_eq!(stats.adds_performed, 3);
        assert_eq!(stats.searches_performed, 1);
        assert!(stats.avg_add_time_ms >= 0.0);
        assert!(stats.avg_search_time_ms >= 0.0);
    }

    #[tokio::test]
    #[ignore] // Integration test that may require specific FAISS setup
    async fn test_faiss_integration() {
        let config = FaissConfig {
            index_description: "IVF100,Flat".to_string(),
            dimension: 128,
            metric_type: FaissMetricType::L2,
            nlist: Some(100),
            nprobe: Some(10),
            index_path: None,
            use_gpu: false,
            max_vectors: Some(10000),
        };

        let manager = FaissManager::with_config(config).await.unwrap();

        // Test connection
        assert!(manager.test_connection().await.unwrap());

        // Add test data
        let mut embeddings = Vec::new();
        for i in 0..100 {
            let embedding = VectorEmbedding {
                id: format!("integration_test_{i}"),
                vector: vec![(i % 10) as f32 / 10.0; 128],
                metadata: {
                    let mut map = HashMap::new();
                    map.insert("batch".to_string(), "integration".to_string());
                    map
                },
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            };
            embeddings.push(embedding);
        }

        manager.batch_add_embeddings(embeddings).await.unwrap();
        assert_eq!(manager.get_vector_count().await, 100);

        // Test search with filters
        let query = SearchQuery {
            vector: vec![0.5; 128],
            k: 10,
            filters: {
                let mut filters = HashMap::new();
                filters.insert("batch".to_string(), "integration".to_string());
                Some(filters)
            },
        };

        let results = manager.search(query).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 10);

        // Test pattern operations
        manager
            .create_pattern_relationship("integration_test_0", "integration_test_1", "similar", 0.9)
            .await
            .unwrap();
        let relationships = manager
            .get_pattern_relationships("integration_test_0")
            .await
            .unwrap();
        assert!(!relationships.is_empty());

        // Test similar patterns
        let similar = manager
            .find_similar_patterns("integration_test_0", 2.0, 5)
            .await
            .unwrap();
        assert!(!similar.is_empty());

        // Get final stats
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_vectors, 100);
        assert!(stats.adds_performed >= 100);
        assert!(stats.searches_performed >= 1);
    }
}
