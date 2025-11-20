//! ML Integration Module
//!
//! This module provides comprehensive machine learning integration for OdinCode,
//! including model training, prediction, and advanced numerical operations.
//!
//! # Architecture
//!
//! The module is organized into several submodules:
//! - `metadata`: Core types for model metadata, metrics, and results
//! - `models`: Trained model implementations (KMeans, LinearRegression, SVM, etc.)
//! - `manager`: Model lifecycle management and training orchestration
//! - `trainer`: Training implementations for supervised and unsupervised learning
//! - `operations`: Numerical array operations and preprocessing utilities
//! - `integration`: Main entry point (MLIntegrationManager) for ML operations
//!
//! # Usage
//!
//! ```rust,no_run
//! use odincode_core::ml_integration::{MLIntegrationManager, MLIntegrationConfig};
//!
//! // Create ML integration manager
//! let config = MLIntegrationConfig::default();
//! let manager = MLIntegrationManager::new(engine, ltmc_manager, config);
//!
//! // Train and use models through the manager
//! ```

use candle_core::{Result as CandleResult, Tensor};
use candle_nn::{Linear, Module};

// Submodule declarations
pub mod integration;
pub mod manager;
pub mod metadata;
pub mod models;
pub mod operations;
pub mod trainer;

// New modular structure
pub mod analysis;
pub mod config;
pub mod core;
pub mod llm;
pub mod model_management;
pub mod training;

// New facade replacing massive integration.rs
pub mod facade;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod manager_test;

// Re-export key types for convenience
pub use metadata::{
    HyperparameterSearchConfig, HyperparameterSearchType, ModelMetadata, ModelPerformanceMetrics,
    ModelType, PredictionResult, TrainingConfig, TrainingResult,
};

pub use models::{
    TrainedDbscan, TrainedDecisionTree, TrainedGaussianMixture, TrainedKMeans,
    TrainedLinearRegression, TrainedLogisticRegression, TrainedModel, TrainedNaiveBayes,
    TrainedSvm,
};

pub use integration::{MLIntegrationConfig, MLIntegrationManager};
pub use manager::ModelManager;
pub use operations::NDArrayOperations;

// New modular exports
pub use analysis::{MLPredictor, QualityAnalyzer};
pub use config::{
    LLMConfig, MLIntegrationConfig as NewMLIntegrationConfig, ModelRegistryConfig,
    PerformanceTrackingConfig,
};
pub use core::MLIntegrationCore;
pub use llm::LLMIntegration;
pub use model_management::{ModelExporter, ModelImporter, ModelPerformanceTracker, ModelRegistry};
pub use training::TrainingOrchestrator;

// Facade exports (replaces integration.rs)
pub use facade::{MLIntegrationFacade, MLIntegrationStats};

/// Simple neural network for code analysis using Candle
///
/// This is a basic feedforward neural network with three layers,
/// used for code embedding and analysis tasks.
#[derive(Debug)]
pub struct SimpleNN {
    layer1: Linear,
    layer2: Linear,
    layer3: Linear,
}

impl Module for SimpleNN {
    fn forward(&self, xs: &Tensor) -> CandleResult<Tensor> {
        let xs = self.layer1.forward(xs)?;
        let xs = xs.relu()?;
        let xs = self.layer2.forward(&xs)?;
        let xs = xs.relu()?;
        self.layer3.forward(&xs)
    }
}

/// Helper function to generate combinations with replacement
///
/// Used for hyperparameter grid search generation.
fn combinations_with_replacement(n: usize, k: usize) -> Vec<Vec<usize>> {
    if k == 0 {
        return vec![vec![]];
    }

    let mut result = Vec::new();
    for i in 0..n {
        for mut combo in combinations_with_replacement(n, k - 1) {
            combo.push(i);
            combo.sort();
            result.push(combo);
        }
    }

    result.sort();
    result.dedup();
    result
}
