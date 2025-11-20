//! LTMC Structure Search Utilities
//!
//! This module contains utilities for searching LTMC structure information.

use anyhow::Result;
use tracing::{debug, info};

use crate::{search::core::CoreSearchUtils, LTMManager, LearningPattern};

/// LTMC structure search utilities
pub struct LtmcStructureSearchUtils;

impl LtmcStructureSearchUtils {
    /// Search for LTMC structure information
    ///
    /// This method searches for patterns that describe the LTMC system structure,
    /// architecture, and implementation details.
    ///
    /// # Arguments
    ///
    /// * `ltm_manager` - Reference to the LTMC manager
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of learning patterns related to LTMC structure
    pub async fn search_ltmc_structure(
        ltm_manager: &LTMManager,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        debug!("Searching for LTMC structure information");

        // Search for LTMC-related topics
        let ltmc_topics = vec![
            "ltmc",
            "ltmc architecture",
            "ltmc structure",
            "learning through meta cognition",
            "persistent memory",
            "four database system",
            "sqlite neo4j redis faiss",
            "ltmc implementation",
        ];

        let mut all_results = Vec::new();

        for topic in &ltmc_topics {
            let results = CoreSearchUtils::search_for_topic(
                ltm_manager,
                topic,
                None,
                limit / ltmc_topics.len(),
            )
            .await?;

            all_results.extend(results);
        }

        // Deduplicate results by ID
        let mut unique_results = Vec::new();
        let mut seen_ids = std::collections::HashSet::new();

        for pattern in all_results {
            if seen_ids.insert(pattern.id) {
                unique_results.push(pattern);
            }

            // Respect the limit
            if unique_results.len() >= limit {
                break;
            }
        }

        info!(
            "Found {} unique LTMC structure patterns",
            unique_results.len()
        );
        Ok(unique_results)
    }
}
