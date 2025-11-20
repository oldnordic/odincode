#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_ml_integration_manager_creation() {
        // Test basic creation of ML integration manager
        let config = MLIntegrationConfig::default();
        
        // Create a mock code engine for testing
        let code_engine = Arc::new(CodeEngine::new());
        
        // Create a mock LTMC manager for testing
        let ltmc_manager = Arc::new(LTMManager::new().unwrap());
        
        // This should not panic
        let result = MLIntegrationManager::new(config, code_engine, ltmc_manager);
        assert!(result.is_ok());
        
        let manager = result.unwrap();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_ml_integration_config() {
        let config = MLIntegrationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.model_path, "models/");
        assert_eq!(config.cache_size, 1000);
    }

    #[test]
    fn test_ml_integration_enable_disable() {
        let config = MLIntegrationConfig::default();
        let code_engine = Arc::new(CodeEngine::new());
        let ltmc_manager = Arc::new(LTMManager::new().unwrap());
        
        let mut manager = MLIntegrationManager::new(config, code_engine, ltmc_manager).unwrap();
        
        // Test initial state
        assert!(!manager.is_enabled());
        
        // Test enabling
        manager.enable();
        assert!(manager.is_enabled());
        
        // Test disabling
        manager.disable();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_ml_suggestion_types() {
        // Test that ML suggestion types are properly defined
        assert_eq!(MLSuggestionType::Refactoring.to_string(), "refactoring");
        assert_eq!(MLSuggestionType::Optimization.to_string(), "optimization");
        assert_eq!(MLSuggestionType::Documentation.to_string(), "documentation");
        assert_eq!(MLSuggestionType::Testing.to_string(), "testing");
        assert_eq!(MLSuggestionType::Security.to_string(), "security");
    }

    #[test]
    fn test_ml_model_info() {
        let model_info = MLModelInfo {
            name: "test_model".to_string(),
            version: "1.0.0".to_string(),
            model_type: "neural_network".to_string(),
            accuracy: 0.95,
            last_trained: chrono::Utc::now().naive_utc(),
        };
        
        assert_eq!(model_info.name, "test_model");
        assert_eq!(model_info.version, "1.0.0");
        assert_eq!(model_info.model_type, "neural_network");
        assert_eq!(model_info.accuracy, 0.95);
    }
}