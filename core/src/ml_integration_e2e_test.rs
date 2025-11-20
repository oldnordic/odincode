use std::sync::Arc;
use odincode_core::{MLIntegrationManager, CodeFile, LearningPattern, PatternType, CodeSuggestion, SuggestionType};
use odincode_databases::{DatabaseManager, DatabaseType};
use odincode_ltmc::LTMManager;
use ndarray::{Array1, Array2};
use linfa::Dataset;
use linfa::traits::{Fit, Predict};

#[tokio::test]
async fn test_ml_integration_end_to_end() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database manager
    let mut db_manager = DatabaseManager::new()?;
    
    // Configure all databases from LTMC2 config
    db_manager.add_database(DatabaseType::SQLite, "/home/feanor/Projects/Data/ltmc.db")?;
    db_manager.add_database(DatabaseType::FAISS, "/home/feanor/Projects/Data/faiss_index")?;
    db_manager.add_database(DatabaseType::Redis, "redis://localhost:6379")?;
    db_manager.add_database(DatabaseType::Neo4j, "bolt://localhost:7689")?;
    
    // Initialize LTMC manager
    let ltmc_manager = Arc::new(LTMManager::new("/home/feanor/Projects/ltmc2/ltmc_config.json").await?);
    
    // Initialize ML Integration Manager
    let ml_manager = MLIntegrationManager::new(db_manager, ltmc_manager.clone()).await?;
    
    // Create test code files with various patterns
    let test_files = create_test_code_files();
    
    // Test 1: SVM Classification
    println!("Testing SVM Classification...");
    test_svm_classification(&ml_manager, &test_files).await?;
    
    // Test 2: DBSCAN Clustering
    println!("Testing DBSCAN Clustering...");
    test_dbscan_clustering(&ml_manager, &test_files).await?;
    
    // Test 3: FTRL Online Learning
    println!("Testing FTRL Online Learning...");
    test_ftrl_online_learning(&ml_manager, &test_files).await?;
    
    // Test 4: Pattern Persistence to Database
    println!("Testing Pattern Persistence...");
    test_pattern_persistence(&ml_manager, &test_files).await?;
    
    // Test 5: Cross-Database Integration
    println!("Testing Cross-Database Integration...");
    test_cross_database_integration(&ml_manager, &test_files).await?;
    
    println!("✅ All ML Integration End-to-End Tests Passed!");
    Ok(())
}

fn create_test_code_files() -> Vec<CodeFile> {
    vec![
        CodeFile {
            path: "test_files/rust_function.rs".to_string(),
            content: r#"
pub fn calculate_fibonacci(n: u32) -> u64 {
    match n {
        0 => 0,
        1 => 1,
        _ => calculate_fibonacci(n - 1) + calculate_fibonacci(n - 2),
    }
}

pub fn bubble_sort<T: Ord>(arr: &mut [T]) {
    let n = arr.len();
    for i in 0..n {
        for j in 0..n - i - 1 {
            if arr[j] > arr[j + 1] {
                arr.swap(j, j + 1);
            }
        }
    }
}
"#.to_string(),
            language: "rust".to_string(),
            complexity_score: 0.7,
            dependencies: vec![],
            functions: vec!["calculate_fibonacci".to_string(), "bubble_sort".to_string()],
            classes: vec![],
            imports: vec![],
        },
        CodeFile {
            path: "test_files/python_script.py".to_string(),
            content: r#"
import numpy as np
import pandas as pd
from typing import List, Dict

def process_data(data: List[float]) -> Dict[str, float]:
    """Process numerical data and return statistics."""
    return {
        'mean': np.mean(data),
        'std': np.std(data),
        'min': np.min(data),
        'max': np.max(data)
    }

class DataProcessor:
    def __init__(self, config: Dict):
        self.config = config
        
    def analyze(self, data: pd.DataFrame) -> pd.DataFrame:
        return data.describe()
"#.to_string(),
            language: "python".to_string(),
            complexity_score: 0.8,
            dependencies: vec!["numpy".to_string(), "pandas".to_string()],
            functions: vec!["process_data".to_string()],
            classes: vec!["DataProcessor".to_string()],
            imports: vec!["numpy".to_string(), "pandas".to_string(), "typing".to_string()],
        },
        CodeFile {
            path: "test_files/javascript_module.js".to_string(),
            content: r#"
const express = require('express');
const bodyParser = require('body-parser');

function createServer(config = {}) {
    const app = express();
    
    app.use(bodyParser.json());
    
    app.get('/api/data', async (req, res) => {
        try {
            const data = await fetchData();
            res.json(data);
        } catch (error) {
            res.status(500).json({ error: error.message });
        }
    });
    
    return app;
}

class DatabaseManager {
    constructor(connection) {
        this.connection = connection;
    }
    
    async query(sql, params = []) {
        return this.connection.query(sql, params);
    }
}
"#.to_string(),
            language: "javascript".to_string(),
            complexity_score: 0.9,
            dependencies: vec!["express".to_string(), "body-parser".to_string()],
            functions: vec!["createServer".to_string()],
            classes: vec!["DatabaseManager".to_string()],
            imports: vec!["express".to_string(), "body-parser".to_string()],
        },
    ]
}

async fn test_svm_classification(ml_manager: &MLIntegrationManager, test_files: &[CodeFile]) -> Result<(), Box<dyn std::error::Error>> {
    // Create training data for SVM
    let features = Array2::from_shape_vec((6, 4), vec![
        0.7, 0.8, 2.0, 3.0,  // Rust file - high complexity, multiple functions
        0.8, 0.9, 3.0, 2.0,  // Python file - high complexity, classes
        0.9, 0.7, 4.0, 1.0,  // JavaScript file - highest complexity
        0.3, 0.4, 1.0, 1.0,  // Simple file
        0.4, 0.3, 1.0, 2.0,  // Medium file
        0.5, 0.6, 2.0, 1.0,  // Medium file
    ])?;
    
    let targets = Array1::from_vec(vec![
        true,   // Complex files need refactoring
        true,   // Complex files need refactoring
        true,   // Complex files need refactoring
        false,  // Simple files are okay
        false,  // Medium files are okay
        false,  // Medium files are okay
    ]);
    
    let dataset = Dataset::new(features, targets);
    
    // Test SVM training and prediction
    let suggestions = ml_manager.analyze_code_patterns_svm(&dataset, &test_files[0]).await?;
    
    // Verify SVM produces suggestions
    assert!(!suggestions.is_empty(), "SVM should produce refactoring suggestions");
    
    // Verify suggestion types are appropriate
    for suggestion in &suggestions {
        assert!(matches!(suggestion.suggestion_type, SuggestionType::Refactoring | SuggestionType::Optimization));
    }
    
    println!("✅ SVM Classification Test Passed - {} suggestions generated", suggestions.len());
    Ok(())
}

async fn test_dbscan_clustering(ml_manager: &MLIntegrationManager, test_files: &[CodeFile]) -> Result<(), Box<dyn std::error::Error>> {
    // Create feature data for DBSCAN clustering
    let features = Array2::from_shape_vec((3, 5), vec![
        0.7, 0.8, 2.0, 3.0, 0.9,  // Rust file features
        0.8, 0.9, 3.0, 2.0, 0.8,  // Python file features
        0.9, 0.7, 4.0, 1.0, 0.7,  // JavaScript file features
    ])?;
    
    // Test DBSCAN clustering
    let clusters = ml_manager.cluster_code_patterns_dbscan(&features, &test_files).await?;
    
    // Verify DBSCAN produces clusters
    assert!(!clusters.is_empty(), "DBSCAN should produce code clusters");
    
    // Verify cluster assignments are valid
    for cluster in &clusters {
        assert!(cluster.cluster_id >= 0, "Cluster ID should be non-negative");
        assert!(!cluster.patterns.is_empty(), "Cluster should contain patterns");
    }
    
    println!("✅ DBSCAN Clustering Test Passed - {} clusters identified", clusters.len());
    Ok(())
}

async fn test_ftrl_online_learning(ml_manager: &MLIntegrationManager, test_files: &[CodeFile]) -> Result<(), Box<dyn std::error::Error>> {
    // Create sequential data for FTRL online learning
    let features = Array2::from_shape_vec((3, 4), vec![
        0.7, 0.8, 2.0, 3.0,  // Initial code state
        0.8, 0.9, 3.0, 2.0,  // Updated code state
        0.9, 0.7, 4.0, 1.0,  // Final code state
    ])?;
    
    let targets = Array1::from_vec(vec![1.0, 0.8, 0.6]); // Decreasing complexity scores
    
    // Test FTRL online learning
    let predictions = ml_manager.train_ftrl_online(&features, &targets, &test_files).await?;
    
    // Verify FTRL produces predictions
    assert!(!predictions.is_empty(), "FTRL should produce predictions");
    
    // Verify predictions are reasonable (between 0 and 1)
    for prediction in &predictions {
        assert!(*prediction >= 0.0 && *prediction <= 1.0, "FTRL predictions should be between 0 and 1");
    }
    
    println!("✅ FTRL Online Learning Test Passed - {} predictions generated", predictions.len());
    Ok(())
}

async fn test_pattern_persistence(ml_manager: &MLIntegrationManager, test_files: &[CodeFile]) -> Result<(), Box<dyn std::error::Error>> {
    // Create learning patterns
    let patterns = vec![
        LearningPattern {
            id: uuid::Uuid::new_v4(),
            pattern_type: PatternType::CodeStructure,
            name: "Rust Function Pattern".to_string(),
            description: "Common Rust function structure with proper error handling".to_string(),
            confidence: 0.85,
            frequency: 10,
            last_seen: chrono::Utc::now(),
            metadata: serde_json::json!({"language": "rust", "complexity": "medium"}),
            examples: vec![test_files[0].content.clone()],
            related_patterns: vec![],
        },
        LearningPattern {
            id: uuid::Uuid::new_v4(),
            pattern_type: PatternType::Dependency,
            name: "Python Data Science Stack".to_string(),
            description: "Common Python data science dependencies pattern".to_string(),
            confidence: 0.92,
            frequency: 15,
            last_seen: chrono::Utc::now(),
            metadata: serde_json::json!({"language": "python", "domain": "data_science"}),
            examples: vec![test_files[1].content.clone()],
            related_patterns: vec![],
        },
    ];
    
    // Test pattern persistence
    ml_manager.store_learning_patterns(&patterns).await?;
    
    // Test pattern retrieval
    let retrieved_patterns = ml_manager.get_learning_patterns_by_type(PatternType::CodeStructure).await?;
    
    // Verify patterns were stored and retrieved correctly
    assert!(!retrieved_patterns.is_empty(), "Should retrieve stored patterns");
    
    let stored_pattern = retrieved_patterns.iter().find(|p| p.name == "Rust Function Pattern");
    assert!(stored_pattern.is_some(), "Should find the stored pattern by name");
    
    println!("✅ Pattern Persistence Test Passed - {} patterns stored and retrieved", patterns.len());
    Ok(())
}

async fn test_cross_database_integration(ml_manager: &MLIntegrationManager, test_files: &[CodeFile]) -> Result<(), std::io::Error> {
    // Test SQLite integration
    let sqlite_result = ml_manager.test_database_connection("sqlite").await;
    assert!(sqlite_result.is_ok(), "Should connect to SQLite");
    
    // Test Redis integration
    let redis_result = ml_manager.test_database_connection("redis").await;
    assert!(redis_result.is_ok(), "Should connect to Redis");
    
    // Test Neo4j integration
    let neo4j_result = ml_manager.test_database_connection("neo4j").await;
    assert!(neo4j_result.is_ok(), "Should connect to Neo4j");
    
    // Test FAISS integration
    let faiss_result = ml_manager.test_database_connection("faiss").await;
    assert!(faiss_result.is_ok(), "Should connect to FAISS");
    
    // Test cross-database query
    let cross_db_result = ml_manager.query_cross_databases("test_pattern").await;
    assert!(cross_db_result.is_ok(), "Should perform cross-database queries");
    
    println!("✅ Cross-Database Integration Test Passed - All 4 databases connected");
    Ok(())
}