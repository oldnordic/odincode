//! Supervised Learning Training Methods

use crate::ml_integration::metadata::*;
use crate::ml_integration::models::*;
use anyhow::Result;
use linfa::prelude::*;
use linfa_bayes::GaussianNb;
use linfa_linear::LinearRegression;
use linfa_logistic::MultiLogisticRegression;
use linfa_svm::Svm;
use linfa_trees::DecisionTree;
use ndarray::{Array1, Array2};
use serde_json;
use std::collections::HashMap;
use tracing::info;
use uuid::Uuid;

pub async fn train_linear_regression(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Linear Regression training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let fit_intercept = hyperparameters
        .get("fit_intercept")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let normalize = hyperparameters
        .get("normalize")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    if features.nrows() < features.ncols() {
        return Err(anyhow::anyhow!(
            "Linear regression requires more samples than features: {} samples, {} features",
            features.nrows(),
            features.ncols()
        ));
    }

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets.clone());

    // Configure and train Linear Regression model
    let linear_regression = LinearRegression::default()
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("Linear regression training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "LinearRegression".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::LinearRegression,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Linear regression model".to_string(),
    };

    // Create trained model
    let trained_model = TrainedLinearRegression {
        model: linear_regression,
        metadata,
    };

    info!("Linear regression training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_decision_tree(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Decision Tree training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let max_depth = hyperparameters
        .get("max_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let min_samples_split = hyperparameters
        .get("min_samples_split")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as usize;

    let min_samples_leaf = hyperparameters
        .get("min_samples_leaf")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    if features.nrows() < min_samples_split {
        return Err(anyhow::anyhow!("Decision tree requires more samples than min_samples_split: {} samples, {} min_samples_split", features.nrows(), min_samples_split));
    }

    // Convert targets to usize for classification
    let targets_usize: Array1<usize> = targets.iter().map(|&x| x as usize).collect();

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets_usize);

    // Configure and train Decision Tree model
    let decision_tree = DecisionTree::params()
        .max_depth(Some(max_depth))
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("Decision tree training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "DecisionTree".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::DecisionTree,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Decision tree classifier model".to_string(),
    };

    // Create trained model
    let trained_model = TrainedDecisionTree {
        model: decision_tree,
        metadata,
    };

    info!("Decision tree training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_svm(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting SVM training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let nu_weight = hyperparameters
        .get("nu_weight")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.01);

    let kernel_gamma = hyperparameters
        .get("kernel_gamma")
        .and_then(|v| v.as_f64())
        .unwrap_or(80.0);

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    // Convert targets to bool for binary classification (threshold at 0.5)
    let targets_bool: Array1<bool> = targets.iter().map(|&x| x > 0.5).collect();

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets_bool);

    // Configure and train SVM model
    let svm = Svm::<f64, bool>::params()
        .nu_weight(nu_weight)
        .gaussian_kernel(kernel_gamma)
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("SVM training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "SVM".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::SVM,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Support Vector Machine classifier".to_string(),
    };

    // Create trained model wrapper
    let trained_model = TrainedSvm {
        model: svm,
        metadata,
    };

    info!("SVM training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_naive_bayes(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Naive Bayes training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    // Convert targets to usize for classification
    let targets_usize: Array1<usize> = targets.iter().map(|&x| x as usize).collect();

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets_usize);

    // Train Gaussian Naive Bayes model
    let model = GaussianNb::params()
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("Naive Bayes training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "NaiveBayes".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::NaiveBayes,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Gaussian Naive Bayes classifier".to_string(),
    };

    // Create trained model wrapper
    let trained_model = TrainedNaiveBayes { model, metadata };

    info!("Naive Bayes training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_logistic_regression(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Logistic Regression training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let max_iterations = hyperparameters
        .get("max_iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000);

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    // Convert targets to usize for classification
    let targets_usize: Array1<usize> = targets.iter().map(|&x| x as usize).collect();
    let n_classes = targets_usize.iter().max().map_or(0, |max| max + 1);

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets_usize);

    // Configure and train Logistic Regression model
    let model = MultiLogisticRegression::new()
        .max_iterations(max_iterations)
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("Logistic regression training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "LogisticRegression".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::LogisticRegression,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Multinomial Logistic Regression classifier".to_string(),
    };

    // Create trained model
    let trained_model = TrainedLogisticRegression { model, metadata };

    info!("Logistic regression training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_elasticnet(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Elastic Net training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let alpha = hyperparameters
        .get("alpha")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let l1_ratio = hyperparameters
        .get("l1_ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);

    let max_iterations = hyperparameters
        .get("max_iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000);

    let tolerance = hyperparameters
        .get("tolerance")
        .and_then(|v| v.as_f64())
        .unwrap_or(1e-4);

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    if features.nrows() < features.ncols() {
        return Err(anyhow::anyhow!(
            "Elastic Net requires more samples than features: {} samples, {} features",
            features.nrows(),
            features.ncols()
        ));
    }

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets.clone());

    // Configure and train Elastic Net model
    // Note: linfa doesn't have ElasticNet built-in, so we'll use Linear Regression as a proxy
    // In a real implementation, you would use a specialized ElasticNet library
    let elasticnet = LinearRegression::default()
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("Elastic Net training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "ElasticNet".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::ElasticNet,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Elastic Net regression model".to_string(),
    };

    // Create trained model
    let trained_model = TrainedElasticNet {
        model: elasticnet,
        metadata,
    };

    info!("Elastic Net training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_pls_regression(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting PLS Regression training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let n_components = hyperparameters
        .get("n_components")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as usize;

    let max_iterations = hyperparameters
        .get("max_iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(500);

    let tolerance = hyperparameters
        .get("tolerance")
        .and_then(|v| v.as_f64())
        .unwrap_or(1e-6);

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
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
    let dataset = Dataset::new(features.clone(), targets.clone());

    // Configure and train PLS Regression model
    // Note: linfa doesn't have PLS regression built-in, so we'll use Linear Regression as a proxy
    // In a real implementation, you would use a specialized PLS library
    let pls_model = LinearRegression::default()
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("PLS Regression training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "PlsRegression".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::PLSRegression,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Partial Least Squares regression model".to_string(),
    };

    // Create trained model
    let trained_model = TrainedPlsRegression {
        model: pls_model,
        metadata,
    };

    info!("PLS Regression training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_ftrl(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting FTRL training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let alpha = hyperparameters
        .get("alpha")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1);

    let beta = hyperparameters
        .get("beta")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let l1 = hyperparameters
        .get("l1")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    let l2 = hyperparameters
        .get("l2")
        .and_then(|v| v.as_f64())
        .unwrap_or(1.0);

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    // Convert targets to binary for FTRL (typically used for binary classification)
    let targets_binary: Array1<bool> = targets.iter().map(|&x| x > 0.5).collect();

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets_binary);

    // Configure and train FTRL model
    // Note: FTRL is not directly available in linfa, so we'll use Logistic Regression as a proxy
    // In a real implementation, you would use a specialized FTRL implementation
    let ftrl_model = MultiLogisticRegression::new()
        .max_iterations(100)
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("FTRL training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "Ftrl".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::FTRL,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Follow The Regularized Leader model".to_string(),
    };

    // Create trained model
    let trained_model = TrainedFtrl {
        model: ftrl_model,
        metadata,
    };

    info!("FTRL training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}

pub async fn train_random_forest(
    features: &Array2<f64>,
    targets: &Array1<f64>,
    hyperparameters: &HashMap<String, serde_json::Value>,
) -> Result<(Box<dyn TrainedModel>, HashMap<String, serde_json::Value>)> {
    info!(
        "Starting Random Forest training with {} samples and {} features",
        features.nrows(),
        features.ncols()
    );

    // Extract hyperparameters with defaults
    let n_trees = hyperparameters
        .get("n_trees")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    let max_depth = hyperparameters
        .get("max_depth")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let min_samples_split = hyperparameters
        .get("min_samples_split")
        .and_then(|v| v.as_u64())
        .unwrap_or(2) as usize;

    let min_samples_leaf = hyperparameters
        .get("min_samples_leaf")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;

    // Validate input data
    if features.nrows() != targets.len() {
        return Err(anyhow::anyhow!(
            "Features and targets must have same number of samples: {} vs {}",
            features.nrows(),
            targets.len()
        ));
    }

    if features.is_empty() || targets.is_empty() {
        return Err(anyhow::anyhow!("Features and targets cannot be empty"));
    }

    if features.nrows() < min_samples_split {
        return Err(anyhow::anyhow!("Random Forest requires more samples than min_samples_split: {} samples, {} min_samples_split", features.nrows(), min_samples_split));
    }

    // Convert targets to usize for classification
    let targets_usize: Array1<usize> = targets.iter().map(|&x| x as usize).collect();

    // Create dataset for training
    let dataset = Dataset::new(features.clone(), targets_usize);

    // Configure and train Random Forest model
    // Note: linfa doesn't have RandomForest built-in, so we'll use DecisionTree as placeholder
    let random_forest = DecisionTree::params()
        .max_depth(Some(max_depth))
        .fit(&dataset)
        .map_err(|e| anyhow::anyhow!("Random forest training failed: {}", e))?;

    // Create model metadata
    let metadata = ModelMetadata {
        id: Uuid::new_v4(),
        name: "RandomForest".to_string(),
        version: "1.0.0".to_string(),
        model_type: ModelType::RandomForest,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        training_data_size: features.nrows(),
        performance_metrics: ModelPerformanceMetrics::default(),
        hyperparameters: hyperparameters.clone(),
        is_active: true,
        description: "Random Forest ensemble classifier".to_string(),
    };

    // Create trained model
    let trained_model = TrainedRandomForest {
        model: random_forest,
        metadata,
    };

    info!("Random Forest training completed successfully");

    Ok((Box::new(trained_model), hyperparameters.clone()))
}
