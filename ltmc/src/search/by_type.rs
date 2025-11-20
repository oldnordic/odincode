//! LTMC Search By Type Utilities
//!
//! This module contains utilities for searching patterns by type in the LTMC system.

use anyhow::Result;
use tracing::{debug, info};

use crate::{LTMManager, LearningPattern, PatternType};

/// Search by type utilities for the LTMC system
pub struct SearchByTypeUtils;

impl SearchByTypeUtils {
    /// Search for patterns by type
    ///
    /// This method retrieves all patterns of a specific type.
    ///
    /// # Arguments
    ///
    /// * `ltm_manager` - Reference to the LTMC manager
    /// * `pattern_type` - The type of patterns to retrieve
    ///
    /// # Returns
    ///
    /// A vector of learning patterns of the specified type
    pub async fn search_by_type(
        ltm_manager: &LTMManager,
        pattern_type: &PatternType,
    ) -> Result<Vec<LearningPattern>> {
        debug!("Searching for patterns of type: {:?}", pattern_type);

        // Try to use the memory search bridge if available
        if let Some(bridge) = &ltm_manager.memory_search_bridge {
            if bridge.is_initialized() {
                // In a real implementation, we would query the databases for patterns of this type
                // For now, we'll fall back to cache search
            }
        }

        // Fall back to cache search
        let cache = ltm_manager.pattern_cache.read().await;
        let results: Vec<LearningPattern> = cache
            .values()
            .filter(|pattern| &pattern.pattern_type == pattern_type)
            .cloned()
            .collect();

        info!(
            "Found {} patterns of type {:?}",
            results.len(),
            pattern_type
        );
        Ok(results)
    }
}
