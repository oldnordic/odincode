//! ML Model Metadata Types
//!
//! This module contains all metadata structures for ML models including
//! model types, performance metrics, training configurations, and results.

use ndarray::Array1;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Model metadata for tracking model versions and performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub model_type: ModelType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub training_data_size: usize,
    pub performance_metrics: ModelPerformanceMetrics,
    pub hyperparameters: HashMap<String, serde_json::Value>,
    pub is_active: bool,
    pub description: String,
}

/// Performance metrics for model evaluation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelPerformanceMetrics {
    pub accuracy: Option<f64>,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1_score: Option<f64>,
    pub mse: Option<f64>,
    pub mae: Option<f64>,
    pub r2_score: Option<f64>,
    pub training_time_ms: u64,
    pub prediction_time_ms: u64,
    pub model_size_bytes: u64,
}

/// Types of ML models supported by the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModelType {
    KMeans,
    LinearRegression,
    DecisionTree,
    SVM,
    GaussianMixture,
    DBSCAN,
    NaiveBayes,
    LogisticRegression,
    ElasticNet,
    PCA,
    PLSRegression,
    FTRL,
    HierarchicalClustering,
    RandomForest,
    NeuralNetwork,
    Custom(String),
}

/// Training configuration for model training
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub model_types: Vec<String>,
    pub test_size: f64,
    pub validation_size: f64,
    pub random_seed: Option<u64>,
    pub cross_validation_folds: Option<usize>,
    pub early_stopping: bool,
    pub max_iterations: usize,
    pub tolerance: f64,
    pub hyperparameter_search: HyperparameterSearchConfig,
}

impl Default for TrainingConfig {
    fn default() -> Self {
        Self {
            model_types: vec!["random_forest".to_string(), "linear_regression".to_string()],
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
                max_trials: 100,
                scoring_metric: "accuracy".to_string(),
            },
        }
    }
}

/// Hyperparameter search configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperparameterSearchConfig {
    pub enabled: bool,
    pub search_type: HyperparameterSearchType,
    pub max_trials: usize,
    pub scoring_metric: String,
}

/// Types of hyperparameter search
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HyperparameterSearchType {
    GridSearch,
    RandomSearch,
    BayesianOptimization,
}

/// Model training result
#[derive(Clone, Serialize, Deserialize)]
pub struct TrainingResult {
    pub model_id: Uuid,
    pub training_time_ms: u64,
    pub validation_metrics: ModelPerformanceMetrics,
    pub test_metrics: ModelPerformanceMetrics,
    pub best_hyperparameters: HashMap<String, serde_json::Value>,
    pub convergence_history: Vec<f64>,
    #[serde(skip)]
    pub trained_model: Option<Box<dyn crate::ml_integration::models::TrainedModel>>,
    pub metadata: Option<ModelMetadata>,
}

impl std::fmt::Debug for TrainingResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrainingResult")
            .field("model_id", &self.model_id)
            .field("training_time_ms", &self.training_time_ms)
            .field("validation_metrics", &self.validation_metrics)
            .field("test_metrics", &self.test_metrics)
            .field("best_hyperparameters", &self.best_hyperparameters)
            .field("convergence_history", &self.convergence_history)
            .field("trained_model", &self.trained_model.is_some())
            .field("metadata", &self.metadata)
            .finish()
    }
}

/// Training data structure for ML models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingData {
    pub features: Array1<f64>,
    pub target: Array1<f64>,
    pub metadata: Option<serde_json::Value>,
}

/// Model prediction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    pub predictions: Array1<f64>,
    pub confidence_scores: Option<Array1<f64>>,
    pub prediction_time_ms: u64,
    pub model_version: String,
    pub confidence: Option<f32>,
}
