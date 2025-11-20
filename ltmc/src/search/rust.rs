//! LTMC Rust Search Utilities
//!
//! This module contains utilities for searching Rust-related patterns in the LTMC system.

use anyhow::Result;
use tracing::{debug, info};

use crate::{search::core::CoreSearchUtils, LTMManager, LearningPattern};

/// Rust search utilities for the LTMC system
pub struct RustSearchUtils;

impl RustSearchUtils {
    /// Search for Rust-related patterns
    ///
    /// This method searches specifically for patterns related to Rust programming,
    /// including Rust ML libraries, Rust best practices, etc.
    ///
    /// # Arguments
    ///
    /// * `ltm_manager` - Reference to the LTMC manager
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of learning patterns related to Rust
    pub async fn search_rust_patterns(
        ltm_manager: &LTMManager,
        limit: usize,
    ) -> Result<Vec<LearningPattern>> {
        debug!("Searching for Rust-related patterns");

        // Search for various Rust-related topics
        let rust_topics = vec![
            "rust",
            "rust programming",
            "rust ml",
            "rust machine learning",
            "linfa",
            "rust ml libraries",
            "rust ai",
            "rust neural networks",
        ];

        let mut all_results = Vec::new();

        for topic in &rust_topics {
            let results = CoreSearchUtils::search_for_topic(
                ltm_manager,
                topic,
                None,
                limit / rust_topics.len(),
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
            "Found {} unique Rust-related patterns",
            unique_results.len()
        );
        Ok(unique_results)
    }
}
