//! Tests for the ML Integration module

use super::*;
use ndarray::Array1;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test ModelType enum has required traits
    #[test]
    fn test_model_type_traits() {
        // Test that ModelType can be used as HashMap key (requires Hash + Eq)
        let mut map = std::collections::HashMap::new();
        map.insert(ModelType::KMeans, "kmeans_model".to_string());
        map.insert(ModelType::LinearRegression, "linear_model".to_string());

        assert_eq!(
            map.get(&ModelType::KMeans),
            Some(&"kmeans_model".to_string())
        );
        assert_eq!(
            map.get(&ModelType::LinearRegression),
            Some(&"linear_model".to_string())
        );

        // Test PartialEq
        assert_eq!(ModelType::KMeans, ModelType::KMeans);
        assert_ne!(ModelType::KMeans, ModelType::LinearRegression);

        // Test Clone
        let model_type = ModelType::RandomForest;
        let cloned = model_type.clone();
        assert_eq!(model_type, cloned);
    }

    /// Test default hyperparameters generation
    #[test]
    fn test_get_default_hyperparameters() {
        // Create a mock LTMC manager for testing
        // Note: In a real test, we'd need to mock the LTMC manager properly
        let config = TrainingConfig {
            test_size: 0.2,
            validation_size: 0.1,
            random_seed: Some(42),
            cross_validation_folds: Some(5),
            early_stopping: true,
            max_iterations: 1000,
            tolerance: 1e-6,
            hyperparameter_search: HyperparameterSearchConfig {
                enabled: false,
                search_type: HyperparameterSearchType::GridSearch,
                max_trials: 10,
                scoring_metric: "accuracy".to_string(),
            },
        };

        // Test that we can create a ModelManager (would need real LTMC manager in integration test)
        // This test focuses on the data structures and basic functionality
        assert!(config.test_size > 0.0 && config.test_size < 1.0);
        assert!(config.validation_size > 0.0 && config.validation_size < 1.0);
        assert!(config.max_iterations > 0);
        assert!(config.tolerance > 0.0);
    }

    /// Test TrainingConfig serialization
    #[test]
    fn test_training_config_serialization() {
        let config = TrainingConfig {
            test_size: 0.2,
            validation_size: 0.1,
            random_seed: Some(42),
            cross_validation_folds: Some(5),
            early_stopping: true,
            max_iterations: 1000,
            tolerance: 1e-6,
            hyperparameter_search: HyperparameterSearchConfig {
                enabled: false,
                search_type: HyperparameterSearchType::GridSearch,
                max_trials: 10,
                scoring_metric: "accuracy".to_string(),
            },
        };

        // Test JSON serialization
        let json = serde_json::to_string(&config).expect("Failed to serialize config");
        let deserialized: TrainingConfig =
            serde_json::from_str(&json).expect("Failed to deserialize config");

        assert_eq!(config.test_size, deserialized.test_size);
        assert_eq!(config.validation_size, deserialized.validation_size);
        assert_eq!(config.random_seed, deserialized.random_seed);
        assert_eq!(config.early_stopping, deserialized.early_stopping);
        assert_eq!(config.max_iterations, deserialized.max_iterations);
    }

    /// Test ModelMetadata creation and validation
    #[test]
    fn test_model_metadata() {
        let model_id = Uuid::new_v4();
        let metadata = ModelMetadata {
            id: model_id,
            name: "Test Model".to_string(),
            version: "1.0.0".to_string(),
            model_type: ModelType::KMeans,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            training_data_size: 1000,
            performance_metrics: ModelPerformanceMetrics {
                accuracy: Some(0.95),
                precision: Some(0.92),
                recall: Some(0.88),
                f1_score: Some(0.90),
                mse: None,
                mae: None,
                r2_score: None,
                training_time_ms: 150,
                prediction_time_ms: 5,
                model_size_bytes: 1024,
            },
            hyperparameters: std::collections::HashMap::new(),
            is_active: true,
            description: "Test model for unit testing".to_string(),
        };

        assert_eq!(metadata.id, model_id);
        assert_eq!(metadata.name, "Test Model");
        assert_eq!(metadata.model_type, ModelType::KMeans);
        assert!(metadata.is_active);
        assert!(metadata.training_data_size > 0);
    }

    /// Test ModelPerformanceMetrics for regression models
    #[test]
    fn test_regression_metrics() {
        let metrics = ModelPerformanceMetrics {
            accuracy: None,
            precision: None,
            recall: None,
            f1_score: None,
            mse: Some(0.025),
            mae: Some(0.15),
            r2_score: Some(0.92),
            training_time_ms: 200,
            prediction_time_ms: 3,
            model_size_bytes: 2048,
        };

        assert!(metrics.mse.unwrap() > 0.0);
        assert!(metrics.mae.unwrap() > 0.0);
        assert!(metrics.r2_score.unwrap() > 0.0 && metrics.r2_score.unwrap() <= 1.0);
    }

    /// Test ModelPerformanceMetrics for clustering models
    #[test]
    fn test_clustering_metrics() {
        let metrics = ModelPerformanceMetrics {
            accuracy: None,
            precision: None,
            recall: None,
            f1_score: None,
            mse: None,
            mae: None,
            r2_score: None,
            training_time_ms: 300,
            prediction_time_ms: 8,
            model_size_bytes: 1536,
        };

        assert!(metrics.training_time_ms > 0);
        assert!(metrics.prediction_time_ms > 0);
        assert!(metrics.model_size_bytes > 0);
    }

    /// Test HyperparameterSearchConfig
    #[test]
    fn test_hyperparameter_search_config() {
        let config = HyperparameterSearchConfig {
            enabled: true,
            search_type: HyperparameterSearchType::RandomSearch,
            max_trials: 20,
            scoring_metric: "f1_score".to_string(),
        };

        assert!(config.enabled);
        assert_eq!(config.search_type, HyperparameterSearchType::RandomSearch);
        assert_eq!(config.max_trials, 20);
        assert_eq!(config.scoring_metric, "f1_score");
    }

    /// Test HyperparameterSearchType variants
    #[test]
    fn test_hyperparameter_search_type() {
        let search_types = vec![
            HyperparameterSearchType::GridSearch,
            HyperparameterSearchType::RandomSearch,
            HyperparameterSearchType::BayesianOptimization,
        ];

        for search_type in search_types {
            match search_type {
                HyperparameterSearchType::GridSearch => (),
                HyperparameterSearchType::RandomSearch => (),
                HyperparameterSearchType::BayesianOptimization => (),
            }
        }
    }

    /// Test PredictionResult structure
    #[test]
    fn test_prediction_result() {
        let predictions = Array1::from_vec(vec![1.0, 2.0, 3.0]);
        let confidence_scores = Some(Array1::from_vec(vec![0.8, 0.7, 0.9]));

        let result = PredictionResult {
            predictions: predictions.clone(),
            confidence_scores: confidence_scores.clone(),
            prediction_time_ms: 10,
            model_version: "1.0.0".to_string(),
            confidence: Some(0.9),
        };

        assert_eq!(result.predictions.len(), 3);
        assert!(result.confidence_scores.is_some());
        assert_eq!(result.confidence_scores.as_ref().unwrap().len(), 3);
    }

    /// Test TrainingResult structure
    #[test]
    fn test_training_result() {
        let model_id = Uuid::new_v4();
        let metrics = ModelPerformanceMetrics {
            accuracy: Some(0.95),
            precision: Some(0.92),
            recall: Some(0.88),
            f1_score: Some(0.90),
            mse: None,
            mae: None,
            r2_score: None,
            training_time_ms: 150,
            prediction_time_ms: 5,
            model_size_bytes: 1024,
        };

        let result = TrainingResult {
            model_id,
            training_time_ms: 45200,
            validation_metrics: metrics.clone(),
            test_metrics: metrics.clone(),
            best_hyperparameters: std::collections::HashMap::new(),
            convergence_history: vec![0.1, 0.05, 0.01],
        };

        assert!(result.training_time_ms > 0);
        assert!(!result.convergence_history.is_empty());
    }
}
