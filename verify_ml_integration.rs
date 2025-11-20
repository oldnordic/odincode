//! Comprehensive verification script for ML Integration system
//! Ensures all ML models return actual predictions, not zeros

use anyhow::Result;
use ndarray::{Array1, Array2, array};
use odincode_core::ml_integration::{
    models::{
        LinearRegression, LogisticRegression, DecisionTree, KMeans, DBSCAN, 
        GaussianMixture, NaiveBayes, PCA, SVM, ElasticNet, PLSRegression, 
        FTRL, RandomForest, HierarchicalClustering
    },
    trainer::{SupervisedTrainer, UnsupervisedTrainer},
    hyperparameters::{
        LinearRegressionParams, LogisticRegressionParams, DecisionTreeParams,
        KMeansParams, DBSCANParams, GaussianMixtureParams, NaiveBayesParams,
        PCAParams, SVMParams, ElasticNetParams, PLSRegressionParams,
        FTRLParams, RandomForestParams, HierarchicalClusteringParams
    }
};

fn main() -> Result<()> {
    println!("🔍 Starting ML Integration Verification");
    println!("=====================================\n");

    // Create test data
    let (features, targets) = create_test_data();
    
    // Test supervised models
    test_supervised_models(&features, &targets)?;
    
    // Test unsupervised models
    test_unsupervised_models(&features)?;
    
    println!("✅ All ML Integration verification tests passed!");
    Ok(())
}

fn create_test_data() -> (Array2<f64>, Array1<f64>) {
    println!("📊 Creating test dataset...");
    
    // Create synthetic features: 100 samples, 5 features
    let features = Array2::from_shape_fn((100, 5), |(i, j)| {
        (i as f64 * 0.1 + j as f64 * 0.05 + (i * j) as f64 * 0.01).sin()
    });
    
    // Create synthetic targets based on features
    let targets = Array1::from_shape_fn(100, |i| {
        let row_sum = features.row(i).sum();
        (row_sum * 0.5 + (i as f64 * 0.1).cos()).abs()
    });
    
    println!("   Features shape: {:?}", features.shape());
    println!("   Targets shape: {:?}", targets.shape());
    println!("   Feature range: [{:.3}, {:.3}]", features.iter().fold(f64::INFINITY, |a, &b| a.min(b)), 
             features.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
    println!("   Target range: [{:.3}, {:.3}]", targets.iter().fold(f64::INFINITY, |a, &b| a.min(b)), 
             targets.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
    
    (features, targets)
}

fn test_supervised_models(features: &Array2<f64>, targets: &Array1<f64>) -> Result<()> {
    println!("\n🤖 Testing Supervised Learning Models");
    println!("=====================================");
    
    // Test Linear Regression
    test_model(
        "Linear Regression",
        || {
            let params = LinearRegressionParams::default();
            let model = LinearRegression::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_linear_regression(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test Logistic Regression
    test_model(
        "Logistic Regression",
        || {
            let params = LogisticRegressionParams::default();
            let model = LogisticRegression::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_logistic_regression(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test Decision Tree
    test_model(
        "Decision Tree",
        || {
            let params = DecisionTreeParams::default();
            let model = DecisionTree::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_decision_tree(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test SVM
    test_model(
        "SVM",
        || {
            let params = SVMParams::default();
            let model = SVM::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_svm(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test ElasticNet
    test_model(
        "ElasticNet",
        || {
            let params = ElasticNetParams::default();
            let model = ElasticNet::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_elasticnet(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test PLS Regression
    test_model(
        "PLS Regression",
        || {
            let params = PLSRegressionParams::default();
            let model = PLSRegression::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_pls_regression(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test FTRL
    test_model(
        "FTRL",
        || {
            let params = FTRLParams::default();
            let model = FTRL::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_ftrl(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test Random Forest
    test_model(
        "Random Forest",
        || {
            let params = RandomForestParams::default();
            let model = RandomForest::new(params);
            let trainer = SupervisedTrainer::new();
            trainer.train_random_forest(model, features.view(), targets.view())
        },
        |model| model.predict(features.view())
    )?;
    
    Ok(())
}

fn test_unsupervised_models(features: &Array2<f64>) -> Result<()> {
    println!("\n🔍 Testing Unsupervised Learning Models");
    println!("=======================================");
    
    // Test KMeans
    test_model(
        "KMeans",
        || {
            let params = KMeansParams::default();
            let model = KMeans::new(params);
            let trainer = UnsupervisedTrainer::new();
            trainer.train_kmeans(model, features.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test DBSCAN
    test_model(
        "DBSCAN",
        || {
            let params = DBSCANParams::default();
            let model = DBSCAN::new(params);
            let trainer = UnsupervisedTrainer::new();
            trainer.train_dbscan(model, features.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test Gaussian Mixture
    test_model(
        "Gaussian Mixture",
        || {
            let params = GaussianMixtureParams::default();
            let model = GaussianMixture::new(params);
            let trainer = UnsupervisedTrainer::new();
            trainer.train_gaussian_mixture(model, features.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test Naive Bayes
    test_model(
        "Naive Bayes",
        || {
            let params = NaiveBayesParams::default();
            let model = NaiveBayes::new(params);
            let trainer = UnsupervisedTrainer::new();
            trainer.train_naive_bayes(model, features.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test PCA
    test_model(
        "PCA",
        || {
            let params = PCAParams::default();
            let model = PCA::new(params);
            let trainer = UnsupervisedTrainer::new();
            trainer.train_pca(model, features.view())
        },
        |model| model.predict(features.view())
    )?;
    
    // Test Hierarchical Clustering
    test_model(
        "Hierarchical Clustering",
        || {
            let params = HierarchicalClusteringParams::default();
            let model = HierarchicalClustering::new(params);
            let trainer = UnsupervisedTrainer::new();
            trainer.train_hierarchical_clustering(model, features.view())
        },
        |model| model.predict(features.view())
    )?;
    
    Ok(())
}

fn test_model<F, T, P>(
    name: &str,
    train_fn: F,
    predict_fn: P
) -> Result<()> 
where
    F: FnOnce() -> Result<T>,
    P: FnOnce(&T) -> Array1<f64>,
{
    print!("📈 Testing {}... ", name);
    
    // Train the model
    let model = match train_fn() {
        Ok(model) => model,
        Err(e) => {
            println!("❌ Training failed: {}", e);
            return Err(e);
        }
    };
    
    // Make predictions
    let predictions = predict_fn(&model);
    
    // Verify predictions are not all zeros
    let all_zeros = predictions.iter().all(|&x| x == 0.0);
    let all_same = predictions.iter().all(|&x| x == predictions[0]);
    
    if all_zeros {
        println!("❌ FAILED: All predictions are zeros!");
        println!("   Predictions: {:?}", predictions.slice(s![..5.min(predictions.len())]));
        return Err(anyhow::anyhow!("{} predictions are all zeros", name));
    }
    
    if all_same && predictions.len() > 1 {
        println!("❌ FAILED: All predictions are identical!");
        println!("   Predictions: {:?}", predictions.slice(s![..5.min(predictions.len())]));
        return Err(anyhow::anyhow!("{} predictions are all identical", name));
    }
    
    // Check prediction statistics
    let pred_min = predictions.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let pred_max = predictions.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let pred_mean = predictions.iter().sum::<f64>() / predictions.len() as f64;
    let pred_std = (predictions.iter().map(|&x| (x - pred_mean).powi(2)).sum::<f64>() / predictions.len() as f64).sqrt();
    
    println!("✅ SUCCESS");
    println!("   Range: [{:.3}, {:.3}]", pred_min, pred_max);
    println!("   Mean: {:.3}, Std: {:.3}", pred_mean, pred_std);
    println!("   Sample: {:?}", predictions.slice(s![..5.min(predictions.len())]));
    
    Ok(())
}