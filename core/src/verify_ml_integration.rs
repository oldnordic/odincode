//! Comprehensive verification script for ML Integration system
//! Ensures all ML models return actual predictions, not zeros

use anyhow::Result;
use ndarray::{Array1, Array2};
use std::collections::HashMap;

use crate::ml_integration::{
    trainer::{
        train_dbscan, train_decision_tree, train_elasticnet, train_ftrl, train_gaussian_mixture,
        train_hierarchical_clustering, train_kmeans, train_linear_regression,
        train_logistic_regression, train_naive_bayes, train_pca, train_pls_regression,
        train_random_forest, train_svm,
    },
    ModelType,
};

/// Test data generator for ML verification
struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate sample features for regression (n_samples x n_features)
    fn generate_regression_features(n_samples: usize, n_features: usize) -> Array2<f64> {
        let mut data = Array2::zeros((n_samples, n_features));
        for ((i, j), val) in data.indexed_iter_mut() {
            *val = (i as f64 * 0.1 + j as f64 * 0.05 + (i + j) as f64 * 0.01).sin();
        }
        data
    }

    /// Generate sample targets for regression
    fn generate_regression_targets(n_samples: usize) -> Array1<f64> {
        let mut targets = Array1::zeros(n_samples);
        for (i, val) in targets.iter_mut().enumerate() {
            *val = (i as f64 * 0.2).sin() + 0.5;
        }
        targets
    }

    /// Generate sample features for clustering (n_samples x n_features)
    /// Creates distinct clusters for better DBSCAN performance
    fn generate_clustering_features(n_samples: usize, n_features: usize) -> Array2<f64> {
        let mut data = Array2::zeros((n_samples, n_features));
        let cluster_size = n_samples / 3;

        for ((i, j), val) in data.indexed_iter_mut() {
            let cluster = i / cluster_size;
            let base_offset = cluster as f64 * 3.0; // Separate clusters by distance
            let noise = (i as f64 * 0.1 + j as f64 * 0.05) * 0.1; // Small noise within cluster
            *val = base_offset + noise + (cluster as f64 * 0.5); // Add cluster-specific offset
        }
        data
    }

    /// Generate default hyperparameters for each model type
    fn get_hyperparameters(model_type: &ModelType) -> HashMap<String, serde_json::Value> {
        let mut params = HashMap::new();

        match model_type {
            ModelType::LinearRegression => {
                params.insert("fit_intercept".to_string(), serde_json::Value::Bool(true));
                params.insert("normalize".to_string(), serde_json::Value::Bool(false));
            }
            ModelType::LogisticRegression => {
                params.insert(
                    "max_iterations".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(100)),
                );
                params.insert(
                    "tolerance".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1e-4).unwrap()),
                );
            }
            ModelType::DecisionTree => {
                params.insert(
                    "max_depth".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(5)),
                );
                params.insert(
                    "min_samples_split".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(2)),
                );
            }
            ModelType::RandomForest => {
                params.insert(
                    "n_trees".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(10)),
                );
                params.insert(
                    "max_depth".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(5)),
                );
            }
            ModelType::SVM => {
                params.insert(
                    "c".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap()),
                );
                params.insert(
                    "kernel".to_string(),
                    serde_json::Value::String("rbf".to_string()),
                );
            }
            ModelType::ElasticNet => {
                params.insert(
                    "alpha".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap()),
                );
                params.insert(
                    "l1_ratio".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(0.5).unwrap()),
                );
            }
            ModelType::PLSRegression => {
                params.insert(
                    "n_components".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(2)),
                );
                params.insert(
                    "max_iterations".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(100)),
                );
            }
            ModelType::FTRL => {
                params.insert(
                    "alpha".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(0.1).unwrap()),
                );
                params.insert(
                    "beta".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap()),
                );
                params.insert(
                    "l1".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(0.01).unwrap()),
                );
                params.insert(
                    "l2".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap()),
                );
            }
            ModelType::KMeans => {
                params.insert(
                    "n_clusters".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(3)),
                );
                params.insert(
                    "max_iterations".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(100)),
                );
                params.insert(
                    "tolerance".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1e-4).unwrap()),
                );
            }
            ModelType::DBSCAN => {
                params.insert(
                    "epsilon".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(0.5).unwrap()),
                );
                params.insert(
                    "min_points".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(5)),
                );
            }
            ModelType::GaussianMixture => {
                params.insert(
                    "n_components".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(3)),
                );
                params.insert(
                    "max_iterations".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(100)),
                );
                params.insert(
                    "tolerance".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1e-4).unwrap()),
                );
            }
            ModelType::NaiveBayes => {
                params.insert(
                    "alpha".to_string(),
                    serde_json::Value::Number(serde_json::Number::from_f64(1.0).unwrap()),
                );
            }
            ModelType::PCA => {
                params.insert(
                    "n_components".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(2)),
                );
            }
            ModelType::HierarchicalClustering => {
                params.insert(
                    "n_clusters".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(3)),
                );
                params.insert(
                    "linkage".to_string(),
                    serde_json::Value::String("ward".to_string()),
                );
            }
            ModelType::NeuralNetwork | ModelType::Custom(_) => {
                // Not implemented in verification test
            }
        }

        params
    }
}

/// Verify that predictions are not all zeros
fn verify_non_zero_predictions(predictions: &Array1<f64>, model_name: &str) -> Result<()> {
    if predictions.is_empty() {
        return Err(anyhow::anyhow!("{}: Empty predictions", model_name));
    }

    let all_zeros = predictions.iter().all(|&x| x == 0.0);
    if all_zeros {
        return Err(anyhow::anyhow!("{}: All predictions are zeros", model_name));
    }

    let all_same = predictions.iter().all(|&x| x == predictions[0]);
    if all_same {
        return Err(anyhow::anyhow!(
            "{}: All predictions are identical ({})",
            model_name,
            predictions[0]
        ));
    }

    println!(
        "✅ {}: Predictions vary (range: {:.4} to {:.4})",
        model_name,
        predictions.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        predictions.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
    );

    Ok(())
}

/// Verify that clustering predictions are not all the same cluster
fn verify_non_zero_clustering(predictions: &Array1<usize>, model_name: &str) -> Result<()> {
    if predictions.is_empty() {
        return Err(anyhow::anyhow!("{}: Empty predictions", model_name));
    }

    let all_same = predictions.iter().all(|&x| x == predictions[0]);
    if all_same {
        return Err(anyhow::anyhow!(
            "{}: All predictions are identical cluster {}",
            model_name,
            predictions[0]
        ));
    }

    let unique_clusters: std::collections::HashSet<_> = predictions.iter().collect();
    println!(
        "✅ {}: Found {} unique clusters",
        model_name,
        unique_clusters.len()
    );

    Ok(())
}

#[tokio::test]
async fn test_ml_integration_verification() -> Result<()> {
    println!("🔍 Starting ML Integration Verification Test");

    let generator = TestDataGenerator;
    let mut results = Vec::new();

    // Test supervised learning models
    let supervised_models = vec![
        (ModelType::LinearRegression, "LinearRegression"),
        (ModelType::LogisticRegression, "LogisticRegression"),
        (ModelType::DecisionTree, "DecisionTree"),
        (ModelType::RandomForest, "RandomForest"),
        (ModelType::SVM, "SVM"),
        (ModelType::ElasticNet, "ElasticNet"),
        (ModelType::PLSRegression, "PLSRegression"),
        (ModelType::FTRL, "FTRL"),
    ];

    for (model_type, model_name) in supervised_models {
        println!("\n📊 Testing {}", model_name);

        let features = TestDataGenerator::generate_regression_features(50, 5);
        let targets = TestDataGenerator::generate_regression_targets(50);
        let hyperparameters = TestDataGenerator::get_hyperparameters(&model_type);

        let result = async {
            match model_type {
                ModelType::LinearRegression => {
                    train_linear_regression(&features, &targets, &hyperparameters).await
                }
                ModelType::LogisticRegression => {
                    train_logistic_regression(&features, &targets, &hyperparameters).await
                }
                ModelType::DecisionTree => {
                    train_decision_tree(&features, &targets, &hyperparameters).await
                }
                ModelType::RandomForest => {
                    train_random_forest(&features, &targets, &hyperparameters).await
                }
                ModelType::SVM => train_svm(&features, &targets, &hyperparameters).await,
                ModelType::ElasticNet => {
                    train_elasticnet(&features, &targets, &hyperparameters).await
                }
                ModelType::PLSRegression => {
                    train_pls_regression(&features, &targets, &hyperparameters).await
                }
                ModelType::FTRL => train_ftrl(&features, &targets, &hyperparameters).await,
                _ => Err(anyhow::anyhow!("Model not implemented in test")),
            }
        }
        .await;

        match result {
            Ok((trained_model, _metadata)) => {
                let prediction_result = trained_model.predict(&features)?;
                // All models return Array1<f64> in the predictions field
                let preds = &prediction_result.predictions;
                if let Err(e) = verify_non_zero_predictions(preds, model_name) {
                    println!("❌ {}: {}", model_name, e);
                    results.push((model_name.to_string(), false));
                } else {
                    results.push((model_name.to_string(), true));
                }
            }
            Err(e) => {
                println!("❌ {}: Training failed: {}", model_name, e);
                results.push((model_name.to_string(), false));
            }
        }
    }

    // Test unsupervised learning models
    let unsupervised_models = vec![
        (ModelType::KMeans, "KMeans"),
        (ModelType::DBSCAN, "DBSCAN"),
        (ModelType::GaussianMixture, "GaussianMixture"),
        (ModelType::NaiveBayes, "NaiveBayes"),
        (ModelType::PCA, "PCA"),
        (ModelType::HierarchicalClustering, "HierarchicalClustering"),
    ];

    for (model_type, model_name) in unsupervised_models {
        println!("\n📊 Testing {}", model_name);

        let features = TestDataGenerator::generate_clustering_features(50, 5);
        let hyperparameters = TestDataGenerator::get_hyperparameters(&model_type);

        let result = async {
            match model_type {
                ModelType::KMeans => train_kmeans(&features, &hyperparameters).await,
                ModelType::DBSCAN => train_dbscan(&features, &hyperparameters).await,
                ModelType::GaussianMixture => {
                    train_gaussian_mixture(&features, &hyperparameters).await
                }
                ModelType::NaiveBayes => {
                    // NaiveBayes needs targets, generate dummy ones
                    let targets = TestDataGenerator::generate_regression_targets(50);
                    train_naive_bayes(&features, &targets, &hyperparameters).await
                }
                ModelType::PCA => train_pca(&features, &hyperparameters).await,
                ModelType::HierarchicalClustering => {
                    train_hierarchical_clustering(&features, &hyperparameters).await
                }
                _ => Err(anyhow::anyhow!("Model not implemented in test")),
            }
        }
        .await;

        match result {
            Ok((trained_model, _metadata)) => {
                let prediction_result = trained_model.predict(&features)?;
                // For clustering models, convert predictions to usize for cluster verification
                let preds = &prediction_result.predictions;
                let cluster_preds = preds.mapv(|x| x.round() as usize);
                if let Err(e) = verify_non_zero_clustering(&cluster_preds, model_name) {
                    println!("❌ {}: {}", model_name, e);
                    results.push((model_name.to_string(), false));
                } else {
                    results.push((model_name.to_string(), true));
                }
            }
            Err(e) => {
                println!("❌ {}: Training failed: {}", model_name, e);
                results.push((model_name.to_string(), false));
            }
        }
    }

    // Summary
    println!("\n📋 ML Integration Verification Summary");
    println!("{}", "=".repeat(50));

    let passed = results.iter().filter(|(_, success)| *success).count();
    let total = results.len();

    for (model, success) in &results {
        let status = if *success { "✅ PASS" } else { "❌ FAIL" };
        println!("{} {}", status, model);
    }

    println!(
        "\n📊 Results: {}/{} models passed ({:.1}%)",
        passed,
        total,
        (passed as f64 / total as f64) * 100.0
    );

    if passed == total {
        println!("🎉 All ML models are working correctly!");
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "{} models failed verification",
            total - passed
        ))
    }
}
