//! Feature Detection Integration Tests
//!
//! This module tests the feature detection system with different compilation
//! features to ensure graceful fallbacks work correctly.

use odincode_ltmc::features::FeatureDetector;

#[test]
fn test_feature_detection_with_lite_features() {
    let detector = FeatureDetector::new();
    let detection = detector.detect();

    // Basic features should always be available
    assert!(detection.available_databases.contains("sqlite"));
    assert!(detector.is_feature_available("sqlite"));
    assert!(detector.is_feature_available("pattern_learning"));
    assert!(detector.is_feature_available("sequential_thinking"));

    // Check capabilities
    assert!(detection.capabilities.async_processing);
    assert!(detection.capabilities.caching);

    // Validate configuration should pass
    assert!(detector.validate_configuration().is_ok());
}

#[test]
fn test_feature_detection_with_redis_feature() {
    let detector = FeatureDetector::new();

    // Test Redis availability based on compilation feature
    let redis_available = cfg!(feature = "ltmc-redis");
    assert_eq!(detector.is_feature_available("redis"), redis_available);

    if redis_available {
        assert!(detector.detect().available_databases.contains("redis"));
    }
}

#[test]
fn test_feature_detection_with_neo4j_feature() {
    let detector = FeatureDetector::new();

    // Test Neo4j availability based on compilation feature
    let neo4j_available = cfg!(feature = "ltmc-neo4j");
    assert_eq!(detector.is_feature_available("neo4j"), neo4j_available);

    if neo4j_available {
        assert!(detector.detect().available_databases.contains("neo4j"));
        assert!(detector.get_capabilities().graph_operations);
    }
}

#[test]
fn test_feature_detection_with_faiss_feature() {
    let detector = FeatureDetector::new();

    // Test FAISS availability based on compilation feature
    let faiss_available = cfg!(feature = "ltmc-faiss");
    assert_eq!(detector.is_feature_available("faiss"), faiss_available);

    if faiss_available {
        assert!(detector.detect().available_databases.contains("faiss"));
        assert!(detector.get_capabilities().vector_dimensions.is_some());
    }
}

#[test]
fn test_minimal_feature_set() {
    let detector = FeatureDetector::new();
    let minimal = detector.create_minimal_feature_set();

    // Should always include essential features
    assert!(minimal.contains(&"sqlite".to_string()));
    assert!(minimal.contains(&"pattern_learning".to_string()));

    // Minimal set should work with any configuration
    for feature in &minimal {
        assert!(detector.is_feature_available(feature));
    }
}

#[test]
fn test_recommended_feature_set() {
    let detector = FeatureDetector::new();
    let recommended = detector.get_recommended_feature_set();

    // Should always include SQLite and essential features
    assert!(recommended.contains(&"sqlite".to_string()));
    assert!(recommended.contains(&"pattern_learning".to_string()));
    assert!(recommended.contains(&"sequential_thinking".to_string()));

    // Should include available optional features
    if detector.is_feature_available("redis") {
        assert!(recommended.contains(&"redis".to_string()));
    }

    if detector.is_feature_available("neo4j") {
        assert!(recommended.contains(&"neo4j".to_string()));
    }

    if detector.is_feature_available("faiss") {
        assert!(recommended.contains(&"faiss".to_string()));
    }
}

#[test]
fn test_feature_configuration_update() {
    let mut detector = FeatureDetector::new();

    // Update with specific configuration
    let enabled_databases = vec!["sqlite".to_string()];
    let enabled_features = vec!["pattern_learning".to_string(), "caching".to_string()];

    detector.update_from_config(&enabled_databases, &enabled_features);

    // Check if features are enabled correctly
    assert!(detector.is_feature_enabled("sqlite"));
    assert!(detector.is_feature_enabled("pattern_learning"));
    assert!(detector.is_feature_enabled("caching"));

    // Validate updated configuration
    assert!(detector.validate_configuration().is_ok());
}

#[test]
fn test_graceful_fallback() {
    let detector = FeatureDetector::new();

    // Test that system works even when optional features are not available
    let detection = detector.detect();

    // Essential features should be available
    assert!(detection.available_databases.contains("sqlite"));
    assert!(detection.enabled_features.contains("sqlite"));
    assert!(detection.enabled_features.contains("pattern_learning"));

    // System should be functional even without optional databases
    // Should work regardless of optional features
    assert!(detector.validate_configuration().is_ok());
}

#[test]
fn test_feature_dependencies() {
    let detector = FeatureDetector::new();

    // Test that feature dependencies are correctly identified
    let redis_info = detector.get_feature_info("redis");
    if let Some(info) = redis_info {
        if info.available {
            // Redis should depend on SQLite
            assert!(info.dependencies.contains(&"sqlite".to_string()));
        }
    }

    let neo4j_info = detector.get_feature_info("neo4j");
    if let Some(info) = neo4j_info {
        if info.available {
            // Neo4j should depend on SQLite
            assert!(info.dependencies.contains(&"sqlite".to_string()));
        }
    }

    let faiss_info = detector.get_feature_info("faiss");
    if let Some(info) = faiss_info {
        if info.available {
            // FAISS should depend on SQLite
            assert!(info.dependencies.contains(&"sqlite".to_string()));
        }
    }
}

#[test]
fn test_system_capabilities() {
    let detector = FeatureDetector::new();
    let capabilities = detector.get_capabilities();

    // Basic capabilities should always be available
    assert!(capabilities.async_processing);
    assert!(capabilities.caching);
    assert!(capabilities.max_connections > 0);

    // Vector dimensions should be available only if FAISS is compiled
    let faiss_available = cfg!(feature = "ltmc-faiss");
    assert_eq!(capabilities.vector_dimensions.is_some(), faiss_available);

    // Graph operations should be available only if Neo4j is compiled
    let neo4j_available = cfg!(feature = "ltmc-neo4j");
    assert_eq!(capabilities.graph_operations, neo4j_available);
}
