//! FAISS-based search functionality for the Simple LTMC system
//!
//! Implements semantic search and similarity matching for patterns using FAISS.

use anyhow::Result;
use faiss::{index::IndexImpl, Idx, Index};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Manager for FAISS-based search operations
pub struct SearchManager {
    /// FAISS index for similarity search
    index: Arc<RwLock<IndexImpl>>,
}

impl SearchManager {
    /// Create a new SearchManager instance
    pub fn new(index: Arc<RwLock<IndexImpl>>) -> Self {
        Self { index }
    }

    /// Create a new FAISS index
    pub fn create_faiss_index() -> Result<IndexImpl> {
        // Use L2 distance metric with 128 dimensions
        // In a real implementation, you might want to adjust dimensions based on your embedding model
        let d = 128; // dimension of the vectors
        let index = faiss::index::Index::new_l2(d)?;
        Ok(index)
    }

    /// Add a pattern to the FAISS index
    pub async fn add_pattern_to_index(&self, pattern_id: Uuid, embedding: Vec<f32>) -> Result<()> {
        let mut index = self.index.write().await;

        // Ensure the embedding has the right dimension
        if embedding.len() != index.d() as usize {
            return Err(anyhow::anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                index.d(),
                embedding.len()
            ));
        }

        // Convert UUID to FAISS Idx
        let id = pattern_id.as_u128() as Idx;

        // Add the vector to the index
        index.add_with_ids(&[embedding], &[id])?;

        Ok(())
    }

    /// Update an existing pattern in the FAISS index
    pub async fn update_pattern_in_index(
        &self,
        pattern_id: Uuid,
        new_embedding: Vec<f32>,
    ) -> Result<()> {
        let mut index = self.index.write().await;

        // Ensure the embedding has the right dimension
        if new_embedding.len() != index.d() as usize {
            return Err(anyhow::anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                index.d(),
                new_embedding.len()
            ));
        }

        // Convert UUID to FAISS Idx
        let id = pattern_id.as_u128() as Idx;

        // Remove the old vector
        index.remove_ids(&[id])?;

        // Add the updated vector
        index.add_with_ids(&[new_embedding], &[id])?;

        Ok(())
    }

    /// Remove a pattern from the FAISS index
    pub async fn remove_pattern_from_index(&self, pattern_id: Uuid) -> Result<()> {
        let mut index = self.index.write().await;

        // Convert UUID to FAISS Idx
        let id = pattern_id.as_u128() as Idx;

        // Remove the vector from the index
        index.remove_ids(&[id])?;

        Ok(())
    }

    /// Search for similar patterns to the given query
    pub async fn search_similar_patterns(
        &self,
        query_embedding: &[f32],
        k: usize,
    ) -> Result<Vec<(Uuid, f32)>> {
        let index = self.index.read().await;

        // Ensure the query embedding has the right dimension
        if query_embedding.len() != index.d() as usize {
            return Err(anyhow::anyhow!(
                "Query embedding dimension mismatch: expected {}, got {}",
                index.d(),
                query_embedding.len()
            ));
        }

        // Perform the search
        let (distances, ids) = index.search(query_embedding, k as i32)?;

        // Convert FAISS Idx back to UUIDs and pair with distances
        let mut results = Vec::new();
        for (distance, &faiss_id) in distances.iter().zip(ids.iter()) {
            if faiss_id != -1 {
                // -1 indicates no match found
                let uuid = Uuid::from_u128(faiss_id as u128);
                results.push((uuid, distance));
            }
        }

        Ok(results)
    }

    /// Search for similar patterns by content
    pub async fn search_patterns_by_content(
        &self,
        query: &str,
        k: usize,
    ) -> Result<Vec<(Uuid, f32)>> {
        // Create an embedding for the query
        let query_embedding = self.create_embedding(query).await?;

        // Search for similar patterns
        self.search_similar_patterns(&query_embedding, k).await
    }

    /// Create embedding for text content
    pub async fn create_embedding(&self, text: &str) -> Result<Vec<f32>> {
        // This is a simplified embedding creation
        // In a real implementation, you would use proper ML models like BERT, etc.

        // For this example, we'll create a simple embedding based on character frequencies
        // This is just for demonstration purposes
        let mut embedding = vec![0.0f32; 128]; // Assuming 128-dimensional space

        if !text.is_empty() {
            // Simple hashing approach to create an embedding
            // In a real system, you'd use a trained ML model
            for (i, ch) in text.chars().enumerate() {
                if i < embedding.len() {
                    embedding[i] = (ch as u32 % 1000) as f32 / 1000.0; // Normalize to [0, 1]
                } else {
                    // Add to existing values to handle longer texts
                    embedding[i % embedding.len()] += (ch as u32 % 1000) as f32 / 1000.0;
                }
            }

            // Normalize the embedding
            let magnitude: f32 = embedding.iter().map(|x| x * x).sum();
            if magnitude > 0.0 {
                let magnitude = magnitude.sqrt();
                for val in &mut embedding {
                    *val /= magnitude;
                }
            }
        }

        Ok(embedding)
    }

    /// Update a pattern in the index with new content
    pub async fn update_pattern_content(&self, pattern_id: Uuid, new_content: &str) -> Result<()> {
        let new_embedding = self.create_embedding(new_content).await?;
        self.update_pattern_in_index(pattern_id, new_embedding)
            .await
    }

    /// Batch add multiple patterns to the index
    pub async fn batch_add_patterns(&self, patterns: &[(Uuid, Vec<f32>)]) -> Result<()> {
        if patterns.is_empty() {
            return Ok(());
        }

        let mut index = self.index.write().await;

        // Verify all embeddings have the right dimension
        let expected_dim = index.d() as usize;
        for (_, ref embedding) in patterns {
            if embedding.len() != expected_dim {
                return Err(anyhow::anyhow!(
                    "Embedding dimension mismatch: expected {}, got {}",
                    expected_dim,
                    embedding.len()
                ));
            }
        }

        // Prepare arrays for batch operation
        let mut all_embeddings = Vec::new();
        let mut all_ids = Vec::new();

        for (uuid, embedding) in patterns {
            all_embeddings.extend_from_slice(embedding);
            all_ids.push(uuid.as_u128() as Idx);
        }

        // Add all vectors at once
        index.add_with_ids(&all_embeddings, &all_ids)?;

        Ok(())
    }

    /// Get the current number of vectors in the index
    pub async fn get_index_size(&self) -> Result<usize> {
        let index = self.index.read().await;
        Ok(index.ntotal() as usize)
    }

    /// Train the index if it's a trainable index type (not applicable to basic L2 index)
    pub async fn train_index(&self, training_data: &[Vec<f32>]) -> Result<()> {
        if training_data.is_empty() {
            return Ok(());
        }

        // Check if the index is trainable (our L2 index is not trainable)
        // This is just a placeholder implementation
        // For a trainable index, you would do something like:
        // index.train(training_data)?;

        Ok(())
    }
}

/// Helper function to update pattern embeddings when patterns are created or modified
pub async fn update_pattern_embedding(
    search_manager: &SearchManager,
    storage_manager: &super::storage::StorageManager,
    pattern_id: Uuid,
) -> Result<()> {
    // Get the pattern from storage
    if let Some(mut pattern) = storage_manager.get_pattern(pattern_id).await? {
        // Create a new embedding for the pattern content
        let embedding = search_manager.create_embedding(&pattern.content).await?;

        // Update the pattern in the FAISS index
        search_manager
            .update_pattern_in_index(pattern_id, embedding)
            .await?;

        // Update the pattern's embedding in storage as well (in a real implementation)
        // This would involve updating the binary embedding data in the patterns table

        Ok(())
    } else {
        Err(anyhow::anyhow!("Pattern not found: {}", pattern_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_create_and_search_embedding() {
        // Create a fresh index for testing
        let index = SearchManager::create_faiss_index().unwrap();
        let search_manager = SearchManager::new(Arc::new(RwLock::new(index)));

        // Test creating embeddings
        let text1 = "This is a test pattern";
        let embedding1 = search_manager.create_embedding(text1).await.unwrap();
        assert_eq!(embedding1.len(), 128);

        let text2 = "This is another test pattern";
        let embedding2 = search_manager.create_embedding(text2).await.unwrap();
        assert_eq!(embedding2.len(), 128);

        // Add patterns to the index
        let pattern_id1 = Uuid::new_v4();
        let pattern_id2 = Uuid::new_v4();

        search_manager
            .add_pattern_to_index(pattern_id1, embedding1)
            .await
            .unwrap();
        search_manager
            .add_pattern_to_index(pattern_id2, embedding2)
            .await
            .unwrap();

        // Search for similar patterns
        let query_embedding = search_manager
            .create_embedding("Testing pattern search")
            .await
            .unwrap();
        let results = search_manager
            .search_similar_patterns(&query_embedding, 5)
            .await
            .unwrap();

        assert!(!results.is_empty());
        assert!(results.len() <= 5);

        // Check that returned UUIDs are valid
        for (uuid, _) in &results {
            assert!(*uuid == pattern_id1 || *uuid == pattern_id2);
        }
    }
}
