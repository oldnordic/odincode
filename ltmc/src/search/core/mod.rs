//! LTMC Search Core Utilities
//!
//! This module contains the core search utilities for the LTMC system.

use anyhow::Result;
use tracing::{debug, info};
use uuid::Uuid;

use crate::{LTMManager, LearningPattern, PatternType};

/// Core search utilities for the LTMC system
pub struct CoreSearchUtils;

impl CoreSearchUtils {
    /// Search for patterns related to a specific topic
    ///
    /// This method uses the memory search bridge to perform a hybrid search
    /// across all databases (SQLite, FAISS, Neo4j, Redis) for patterns related
    /// to the specified topic.
    ///
    /// # Arguments
    ///
    /// * `ltm_manager` - Reference to the LTMC manager
    /// * `topic` - The topic to search for
    /// * `pattern_type` - Optional pattern type to filter results
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of learning patterns related to the topic
    pub async fn search_for_topic(
        ltm_manager: &LTMManager,
        topic: &str,
        pattern_type: Option<PatternType>,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        debug!("Searching for topic: {}", topic);

        // Try to use the memory search bridge if available
        if let Some(bridge) = &ltm_manager.memory_search_bridge {
            if bridge.is_initialized() {
                match bridge
                    .search_patterns_hybrid(pattern_type.clone(), topic, limit)
                    .await
                {
                    Ok(results) => {
                        info!("Found {} patterns using hybrid search", results.len());
                        return Ok(results);
                    }
                    Err(e) => {
                        debug!("Hybrid search failed, falling back to cache search: {}", e);
                    }
                }
            }
        }

        // Fall back to cache search
        let cache = ltm_manager.pattern_cache.read().await;
        let mut results = Vec::new();

        for pattern in cache.values() {
            let matches_type = match &pattern_type {
                Some(t) => &pattern.pattern_type == t,
                None => true,
            };

            let matches_content = pattern
                .content
                .to_lowercase()
                .contains(&topic.to_lowercase());

            if matches_type && matches_content {
                results.push(pattern.clone());

                // Respect the limit
                if results.len() >= limit {
                    break;
                }
            }
        }

        info!("Found {} patterns using cache search", results.len());
        Ok(results)
    }

    /// Get pattern by ID
    ///
    /// This method retrieves a specific pattern by its ID, first checking the cache
    /// and then using the memory search bridge if available.
    ///
    /// # Arguments
    ///
    /// * `ltm_manager` - Reference to the LTMC manager
    /// * `id` - The UUID of the pattern to retrieve
    ///
    /// # Returns
    ///
    /// The learning pattern if found, or None if not found
    pub async fn get_pattern_by_id(
        ltm_manager: &LTMManager,
        id: Uuid,
    ) -> Result<Option<LearningPattern>> {
        debug!("Getting pattern by ID: {}", id);

        // Check cache first
        {
            let cache = ltm_manager.pattern_cache.read().await;
            if let Some(pattern) = cache.get(&id) {
                return Ok(Some(pattern.clone()));
            }
        }

        // Try to use the memory search bridge if available
        if let Some(bridge) = &ltm_manager.memory_search_bridge {
            if bridge.is_initialized() {
                match bridge.get_pattern_from_cache(id).await {
                    Ok(Some(pattern)) => {
                        // Store in local cache
                        let mut cache = ltm_manager.pattern_cache.write().await;
                        cache.insert(id, pattern.clone());
                        drop(cache);

                        return Ok(Some(pattern));
                    }
                    Ok(None) => {
                        // Pattern not found in bridge cache
                    }
                    Err(e) => {
                        debug!("Failed to get pattern from bridge cache: {}", e);
                    }
                }
            }
        }

        // Pattern not found
        Ok(None)
    }
}
