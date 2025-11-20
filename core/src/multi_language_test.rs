//! Multi-language testing for OdinCode
//! 
//! This module tests the multi-language support functionality of the OdinCode system.

use crate::{CodeEngine, CodeFile};
use crate::language_parsing::{SupportedLanguage, LanguageParser};
use uuid::Uuid;

#[tokio::test]
async fn test_rust_language_support() {
    let engine = CodeEngine::new().unwrap();
    
    let rust_code = r#"
        fn main() {
            let mut vec = Vec::new();
            vec.push(1);
            vec.push(2);
            // TODO: Add more elements
            let length = vec.iter().collect::<Vec<_>>().len();
            println!("Length: {}", length);
        }
    "#;
    
    let file_id = engine.load_file("test.rs".to_string(), rust_code.to_string(), "rust".to_string()).await.unwrap();
    
    // Analyze the file
    let analysis_result = engine.analyze_file(file_id).await.unwrap();
    assert!(analysis_result.is_some());
    let result = analysis_result.unwrap();
    
    // Should find issues
    assert!(result.issues.len() > 0);
    
    // Should find TODO comment
    let todo_issues: Vec<_> = result.issues.iter()
        .filter(|issue| issue.description.contains("TODO"))
        .collect();
    assert!(todo_issues.len() > 0);
    
    // Should find performance issue with collect
    let perf_issues: Vec<_> = result.issues.iter()
        .filter(|issue| issue.description.contains("inefficient"))
        .collect();
    assert!(perf_issues.len() > 0);
    
    // Should generate suggestions
    assert!(result.suggestions.len() > 0);
}

#[tokio::test]
async fn test_javascript_language_support() {
    let engine = CodeEngine::new().unwrap();
    
    let js_code = r#"
        function calculateSum(arr) {
            var sum = 0;
            for (var i = 0; i < arr.length; i++) {
                sum += arr[i];
            }
            return sum;
        }

        // Using == instead of ===
        if (sum == 10) {
            console.log("Sum is 10");
        }
    "#;
    
    let file_id = engine.load_file("test.js".to_string(), js_code.to_string(), "javascript".to_string()).await.unwrap();
    
    // Analyze the file
    let analysis_result = engine.analyze_file(file_id).await.unwrap();
    assert!(analysis_result.is_some());
    let result = analysis_result.unwrap();
    
    // Should find issues
    assert!(result.issues.len() > 0);
    
    // Should find == vs === issues
    let equality_issues: Vec<_> = result.issues.iter()
        .filter(|issue| issue.description.contains("=="))
        .collect();
    assert!(equality_issues.len() > 0);
    
    // Should find var usage
    let var_issues: Vec<_> = result.issues.iter()
        .filter(|issue| issue.description.contains("var"))
        .collect();
    assert!(var_issues.len() > 0);
    
    // Should generate suggestions
    assert!(result.suggestions.len() > 0);
}

#[tokio::test]
async fn test_python_language_support() {
    let engine = CodeEngine::new().unwrap();
    
    let python_code = r#"
        from module import *
        
        def test_function():
            # TODO: Implement this function
            if True:
                print("Hello")
            else:
                print("World")
    "#;
    
    let file_id = engine.load_file("test.py".to_string(), python_code.to_string(), "python".to_string()).await.unwrap();
    
    // Analyze the file
    let analysis_result = engine.analyze_file(file_id).await.unwrap();
    assert!(analysis_result.is_some());
    let result = analysis_result.unwrap();
    
    // Should find issues
    assert!(result.issues.len() > 0);
    
    // Should find TODO comment
    let todo_issues: Vec<_> = result.issues.iter()
        .filter(|issue| issue.description.contains("TODO"))
        .collect();
    assert!(todo_issues.len() > 0);
    
    // Should find wildcard import
    let wildcard_issues: Vec<_> = result.issues.iter()
        .filter(|issue| issue.description.contains("wildcard"))
        .collect();
    assert!(wildcard_issues.len() > 0);
    
    // Should generate suggestions
    assert!(result.suggestions.len() > 0);
}

#[tokio::test]
async fn test_language_detection_from_path() {
    let engine = CodeEngine::new().unwrap();
    
    // Test Rust file detection
    let rust_id = engine.load_file_with_detection("example.rs".to_string(), "fn main() {}".to_string()).await.unwrap();
    let rust_file = engine.get_file(rust_id).await.unwrap().unwrap();
    assert_eq!(rust_file.language, "rust");
    
    // Test JavaScript file detection
    let js_id = engine.load_file_with_detection("example.js".to_string(), "function test() {}".to_string()).await.unwrap();
    let js_file = engine.get_file(js_id).await.unwrap().unwrap();
    assert_eq!(js_file.language, "javascript");
    
    // Test Python file detection
    let py_id = engine.load_file_with_detection("example.py".to_string(), "def test(): pass".to_string()).await.unwrap();
    let py_file = engine.get_file(py_id).await.unwrap().unwrap();
    assert_eq!(py_file.language, "python");
    
    // Test TypeScript file detection
    let ts_id = engine.load_file_with_detection("example.ts".to_string(), "function test(): void {}".to_string()).await.unwrap();
    let ts_file = engine.get_file(ts_id).await.unwrap().unwrap();
    assert_eq!(ts_file.language, "typescript");
}

#[tokio::test]
async fn test_tree_sitter_parsing() {
    let mut parser = LanguageParser::new().unwrap();
    
    // Test Rust parsing
    let rust_code = r#"
        fn main() {
            println!("Hello, world!");
        }
    "#;
    let rust_tree = parser.parse(rust_code, &SupportedLanguage::Rust).unwrap();
    assert!(!rust_tree.root_node().has_error());
    
    // Test JavaScript parsing
    let js_code = r#"
        function hello() {
            console.log("Hello, world!");
        }
    "#;
    let js_tree = parser.parse(js_code, &SupportedLanguage::JavaScript).unwrap();
    assert!(!js_tree.root_node().has_error());
    
    // Test Python parsing
    let py_code = r#"
        def hello():
            print("Hello, world!")
    "#;
    let py_tree = parser.parse(py_code, &SupportedLanguage::Python).unwrap();
    assert!(!py_tree.root_node().has_error());
}

#[tokio::test]
async fn test_language_analyzer_manager() {
    use crate::language_analyzers::LanguageAnalyzerManager;
    
    let manager = LanguageAnalyzerManager::new().unwrap();
    
    // Test that analyzers exist for all supported languages
    assert!(manager.get_analyzer(&SupportedLanguage::Rust).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::JavaScript).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Python).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Java).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::C).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Cpp).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::CSharp).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Go).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Ruby).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::PHP).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::TypeScript).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Swift).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Kotlin).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Scala).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::R).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Dart).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Lua).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Perl).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::ObjectiveC).is_some());
    assert!(manager.get_analyzer(&SupportedLanguage::Shell).is_some());
}