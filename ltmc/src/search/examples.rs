//! LTMC Search Examples
//!
//! This module provides examples of how to use the LTMC search functionality.

use anyhow::Result;
use tracing::info;
use uuid::Uuid;

use crate::search::{
    CoreSearchUtils, LtmcStructureSearchUtils, RustSearchUtils, SearchByTypeUtils,
};
use crate::{LTMManager, PatternType};

/// Example usage of LTMC search functionality
pub struct SearchExamples;

impl SearchExamples {
    /// Example of searching for Rust-related patterns
    pub async fn example_search_rust_patterns(ltm_manager: &LTMManager) -> Result<()> {
        info!("=== Example: Searching for Rust Patterns ===");

        let results = RustSearchUtils::search_rust_patterns(ltm_manager, 10).await?;

        if results.is_empty() {
            info!("No Rust patterns found in LTMC system");
        } else {
            info!("Found {} Rust patterns:", results.len());
            for (i, pattern) in results.iter().enumerate() {
                info!(
                    "  {}. {:?} - {}",
                    i + 1,
                    pattern.pattern_type,
                    pattern.content.lines().next().unwrap_or("")
                );
            }
        }

        Ok(())
    }

    /// Example of searching for LTMC structure information
    pub async fn example_search_ltmc_structure(ltm_manager: &LTMManager) -> Result<()> {
        info!("=== Example: Searching for LTMC Structure Information ===");

        let results = LtmcStructureSearchUtils::search_ltmc_structure(ltm_manager, 10).await?;

        if results.is_empty() {
            info!("No LTMC structure information found in LTMC system");
        } else {
            info!("Found {} LTMC structure patterns:", results.len());
            for (i, pattern) in results.iter().enumerate() {
                info!(
                    "  {}. {:?} - {}",
                    i + 1,
                    pattern.pattern_type,
                    pattern.content.lines().next().unwrap_or("")
                );
            }
        }

        Ok(())
    }

    /// Example of searching for a specific topic
    pub async fn example_search_topic(ltm_manager: &LTMManager, topic: &str) -> Result<()> {
        info!("=== Example: Searching for Topic '{}' ===", topic);

        let results = CoreSearchUtils::search_for_topic(ltm_manager, topic, None, 5).await?;

        if results.is_empty() {
            info!("No patterns found for topic '{}'", topic);
        } else {
            info!("Found {} patterns for topic '{}':", results.len(), topic);
            for (i, pattern) in results.iter().enumerate() {
                info!(
                    "  {}. {:?} - {}",
                    i + 1,
                    pattern.pattern_type,
                    pattern.content.lines().next().unwrap_or("")
                );
            }
        }

        Ok(())
    }

    /// Example of retrieving a pattern by ID
    pub async fn example_get_pattern_by_id(ltm_manager: &LTMManager, id: Uuid) -> Result<()> {
        info!("=== Example: Retrieving Pattern by ID ===");

        match CoreSearchUtils::get_pattern_by_id(ltm_manager, id).await? {
            Some(pattern) => {
                info!("Found pattern:");
                info!("  ID: {}", pattern.id);
                info!("  Type: {:?}", pattern.pattern_type);
                info!(
                    "  Content: {}",
                    pattern.content.lines().next().unwrap_or("")
                );
            }
            None => {
                info!("Pattern with ID {} not found", id);
            }
        }

        Ok(())
    }

    /// Example of searching by pattern type
    pub async fn example_search_by_type(
        ltm_manager: &LTMManager,
        pattern_type: PatternType,
    ) -> Result<()> {
        info!(
            "=== Example: Searching by Pattern Type '{:?}' ===",
            pattern_type
        );

        let results = SearchByTypeUtils::search_by_type(ltm_manager, &pattern_type).await?;

        if results.is_empty() {
            info!("No patterns found of type '{:?}'", pattern_type);
        } else {
            info!(
                "Found {} patterns of type '{:?}':",
                results.len(),
                pattern_type
            );
            for (i, pattern) in results.iter().enumerate() {
                info!(
                    "  {}. {}",
                    i + 1,
                    pattern.content.lines().next().unwrap_or("")
                );
            }
        }

        Ok(())
    }

    /// Run all search examples
    pub async fn run_all_examples(ltm_manager: &LTMManager) -> Result<()> {
        info!("=== Running All LTMC Search Examples ===");

        // Example 1: Search for Rust patterns
        Self::example_search_rust_patterns(ltm_manager).await?;

        // Example 2: Search for LTMC structure information
        Self::example_search_ltmc_structure(ltm_manager).await?;

        // Example 3: Search for a specific topic
        Self::example_search_topic(ltm_manager, "rust machine learning").await?;

        // Example 4: Search by pattern type
        Self::example_search_by_type(ltm_manager, PatternType::ArchitecturalDecision).await?;

        info!("=== All Examples Completed ===");
        Ok(())
    }
}
