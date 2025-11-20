//! LTMC Feature Detection Module
//!
//! This module provides feature detection and graceful fallback capabilities
//! for the LTMC system, allowing it to work with different compilation options
//! and runtime configurations.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tracing::{debug, info, warn};

/// Feature information structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureInfo {
    /// Feature name
    pub name: String,

    /// Feature description
    pub description: String,

    /// Whether the feature is available (compiled in)
    pub available: bool,

    /// Whether the feature is enabled (runtime configuration)
    pub enabled: bool,

    /// Feature dependencies
    pub dependencies: Vec<String>,

    /// Feature category
    pub category: FeatureCategory,
}

/// Feature categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeatureCategory {
    /// Database features
    Database,

    /// AI/ML features
    AIML,

    /// Performance features
    Performance,

    /// Security features
    Security,

    /// Utility features
    Utility,
}

/// Feature detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureDetection {
    /// All detected features
    pub features: Vec<FeatureInfo>,

    /// Available databases
    pub available_databases: HashSet<String>,

    /// Enabled features
    pub enabled_features: HashSet<String>,

    /// Missing dependencies
    pub missing_dependencies: HashSet<String>,

    /// System capabilities
    pub capabilities: SystemCapabilities,
}

/// System capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemCapabilities {
    /// Maximum concurrent connections
    pub max_connections: usize,

    /// Supported vector dimensions
    pub vector_dimensions: Option<usize>,

    /// Graph operations supported
    pub graph_operations: bool,

    /// Caching supported
    pub caching: bool,

    /// Async processing supported
    pub async_processing: bool,
}

/// Feature detector
pub struct FeatureDetector {
    features: Vec<FeatureInfo>,
    capabilities: SystemCapabilities,
}

impl FeatureDetector {
    /// Create a new feature detector
    pub fn new() -> Self {
        let features = vec![
            // Database features
            FeatureInfo {
                name: "sqlite".to_string(),
                description: "SQLite database for persistent storage".to_string(),
                available: true, // Always available
                enabled: true,   // Always enabled by default
                dependencies: vec![],
                category: FeatureCategory::Database,
            },
            FeatureInfo {
                name: "redis".to_string(),
                description: "Redis for caching and real-time operations".to_string(),
                available: cfg!(feature = "ltmc-redis"),
                enabled: false, // Will be set by runtime config
                dependencies: vec!["sqlite".to_string()],
                category: FeatureCategory::Database,
            },
            FeatureInfo {
                name: "neo4j".to_string(),
                description: "Neo4j for graph-based knowledge storage".to_string(),
                available: cfg!(feature = "ltmc-neo4j"),
                enabled: false, // Will be set by runtime config
                dependencies: vec!["sqlite".to_string()],
                category: FeatureCategory::Database,
            },
            FeatureInfo {
                name: "faiss".to_string(),
                description: "FAISS for vector similarity search".to_string(),
                available: cfg!(feature = "ltmc-faiss"),
                enabled: false, // Will be set by runtime config
                dependencies: vec!["sqlite".to_string()],
                category: FeatureCategory::Database,
            },
            // AI/ML features
            FeatureInfo {
                name: "pattern_learning".to_string(),
                description: "Machine learning for code pattern recognition".to_string(),
                available: true, // Always available with basic functionality
                enabled: true,   // Enabled by default
                dependencies: vec!["sqlite".to_string()],
                category: FeatureCategory::AIML,
            },
            FeatureInfo {
                name: "sequential_thinking".to_string(),
                description: "Sequential thinking and reasoning capabilities".to_string(),
                available: true, // Always available
                enabled: true,   // Enabled by default
                dependencies: vec!["sqlite".to_string()],
                category: FeatureCategory::AIML,
            },
            // Performance features
            FeatureInfo {
                name: "caching".to_string(),
                description: "In-memory caching for performance optimization".to_string(),
                available: true, // Always available
                enabled: true,   // Enabled by default
                dependencies: vec![],
                category: FeatureCategory::Performance,
            },
            FeatureInfo {
                name: "async_processing".to_string(),
                description: "Asynchronous processing for better performance".to_string(),
                available: true, // Always available with tokio
                enabled: true,   // Enabled by default
                dependencies: vec![],
                category: FeatureCategory::Performance,
            },
            // Security features
            FeatureInfo {
                name: "user_tracking".to_string(),
                description: "User interaction tracking for learning".to_string(),
                available: true, // Always available
                enabled: true,   // Enabled by default
                dependencies: vec!["sqlite".to_string()],
                category: FeatureCategory::Security,
            },
            // Utility features
            FeatureInfo {
                name: "cross_database".to_string(),
                description: "Cross-database coordination and synchronization".to_string(),
                available: true, // Always available
                enabled: false,  // Disabled by default (requires multiple databases)
                dependencies: vec!["sqlite".to_string()],
                category: FeatureCategory::Utility,
            },
        ];

        let capabilities = SystemCapabilities {
            max_connections: 100, // Default value
            vector_dimensions: if cfg!(feature = "ltmc-faiss") {
                Some(1536)
            } else {
                None
            },
            graph_operations: cfg!(feature = "ltmc-neo4j"),
            caching: true,
            async_processing: true,
        };

        Self {
            features,
            capabilities,
        }
    }

    /// Detect features and their availability
    pub fn detect(&self) -> FeatureDetection {
        let mut available_databases = HashSet::new();
        let mut enabled_features = HashSet::new();
        let mut missing_dependencies = HashSet::new();

        // Check each feature
        for feature in &self.features {
            if feature.available {
                // Add to available databases if it's a database feature
                if matches!(feature.category, FeatureCategory::Database) {
                    available_databases.insert(feature.name.clone());
                }

                // Check if feature should be enabled (basic logic for now)
                if feature.enabled {
                    enabled_features.insert(feature.name.clone());
                }

                // Check dependencies
                for dep in &feature.dependencies {
                    if !self.features.iter().any(|f| f.name == *dep && f.available) {
                        missing_dependencies.insert(dep.clone());
                    }
                }
            }
        }

        FeatureDetection {
            features: self.features.clone(),
            available_databases,
            enabled_features,
            missing_dependencies,
            capabilities: self.capabilities.clone(),
        }
    }

    /// Update feature states based on runtime configuration
    pub fn update_from_config(
        &mut self,
        enabled_databases: &[String],
        enabled_features: &[String],
    ) {
        for feature in &mut self.features {
            match feature.category {
                FeatureCategory::Database => {
                    feature.enabled =
                        enabled_databases.contains(&feature.name) && feature.available;
                }
                _ => {
                    feature.enabled = enabled_features.contains(&feature.name) && feature.available;
                }
            }
        }
    }

    /// Check if a specific feature is available
    pub fn is_feature_available(&self, feature_name: &str) -> bool {
        self.features
            .iter()
            .find(|f| f.name == feature_name)
            .map(|f| f.available)
            .unwrap_or(false)
    }

    /// Check if a specific feature is enabled
    pub fn is_feature_enabled(&self, feature_name: &str) -> bool {
        self.features
            .iter()
            .find(|f| f.name == feature_name)
            .map(|f| f.enabled)
            .unwrap_or(false)
    }

    /// Get feature information
    pub fn get_feature_info(&self, feature_name: &str) -> Option<&FeatureInfo> {
        self.features.iter().find(|f| f.name == feature_name)
    }

    /// Get all available features
    pub fn get_available_features(&self) -> Vec<&FeatureInfo> {
        self.features.iter().filter(|f| f.available).collect()
    }

    /// Get all enabled features
    pub fn get_enabled_features(&self) -> Vec<&FeatureInfo> {
        self.features.iter().filter(|f| f.enabled).collect()
    }

    /// Get system capabilities
    pub fn get_capabilities(&self) -> &SystemCapabilities {
        &self.capabilities
    }

    /// Validate feature configuration
    pub fn validate_configuration(&self) -> Result<()> {
        let detection = self.detect();

        // Check for missing dependencies
        if !detection.missing_dependencies.is_empty() {
            warn!("Missing dependencies: {:?}", detection.missing_dependencies);
        }

        // Check if essential features are available
        if !detection.available_databases.contains("sqlite") {
            return Err(anyhow!("SQLite database is not available"));
        }

        // Check feature dependencies
        for feature in &detection.features {
            if feature.enabled {
                for dep in &feature.dependencies {
                    if !detection.enabled_features.contains(dep) {
                        return Err(anyhow!(
                            "Feature '{}' requires '{}' but it's not enabled",
                            feature.name,
                            dep
                        ));
                    }
                }
            }
        }

        info!("Feature configuration validated successfully");
        debug!("Available databases: {:?}", detection.available_databases);
        debug!("Enabled features: {:?}", detection.enabled_features);

        Ok(())
    }

    /// Create a minimal feature set for graceful fallback
    pub fn create_minimal_feature_set(&self) -> Vec<String> {
        vec!["sqlite".to_string(), "pattern_learning".to_string()]
    }

    /// Get recommended feature set based on available features
    pub fn get_recommended_feature_set(&self) -> Vec<String> {
        let mut recommended = vec!["sqlite".to_string()];

        if self.is_feature_available("redis") {
            recommended.push("redis".to_string());
        }

        if self.is_feature_available("neo4j") {
            recommended.push("neo4j".to_string());
        }

        if self.is_feature_available("faiss") {
            recommended.push("faiss".to_string());
        }

        // Always include essential AI/ML features
        recommended.extend(vec![
            "pattern_learning".to_string(),
            "sequential_thinking".to_string(),
            "user_tracking".to_string(),
        ]);

        // Performance features
        recommended.extend(vec!["caching".to_string(), "async_processing".to_string()]);

        recommended
    }
}

impl Default for FeatureDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_detector_creation() {
        let detector = FeatureDetector::new();
        assert!(!detector.features.is_empty());
        assert!(detector.is_feature_available("sqlite"));
    }

    #[test]
    fn test_feature_detection() {
        let detector = FeatureDetector::new();
        let detection = detector.detect();

        // SQLite should always be available
        assert!(detection.available_databases.contains("sqlite"));

        // Should have some features
        assert!(!detection.features.is_empty());
    }

    #[test]
    fn test_feature_availability() {
        let detector = FeatureDetector::new();

        // SQLite should always be available
        assert!(detector.is_feature_available("sqlite"));

        // Test non-existent feature
        assert!(!detector.is_feature_available("nonexistent"));
    }

    #[test]
    fn test_feature_info() {
        let detector = FeatureDetector::new();
        let sqlite_info = detector.get_feature_info("sqlite");

        assert!(sqlite_info.is_some());
        let info = sqlite_info.unwrap();
        assert_eq!(info.name, "sqlite");
        assert!(info.available);
    }

    #[test]
    fn test_minimal_feature_set() {
        let detector = FeatureDetector::new();
        let minimal = detector.create_minimal_feature_set();

        assert!(minimal.contains(&"sqlite".to_string()));
        assert!(minimal.contains(&"pattern_learning".to_string()));
    }

    #[test]
    fn test_recommended_feature_set() {
        let detector = FeatureDetector::new();
        let recommended = detector.get_recommended_feature_set();

        // Should always include SQLite
        assert!(recommended.contains(&"sqlite".to_string()));

        // Should include essential features
        assert!(recommended.contains(&"pattern_learning".to_string()));
    }

    #[test]
    fn test_configuration_validation() {
        let detector = FeatureDetector::new();

        // Should pass with default configuration
        assert!(detector.validate_configuration().is_ok());
    }

    #[test]
    fn test_feature_update_from_config() {
        let mut detector = FeatureDetector::new();

        // Update with specific configuration
        detector.update_from_config(
            &["sqlite".to_string(), "redis".to_string()],
            &["pattern_learning".to_string()],
        );

        // Check if features are updated correctly
        assert!(detector.is_feature_enabled("sqlite"));

        // Redis should only be enabled if available
        if detector.is_feature_available("redis") {
            assert!(detector.is_feature_enabled("redis"));
        }
    }
}
