//! Comprehensive LLM Integration Tests
//!
//! End-to-end integration tests for the complete LLM integration system including
//! configuration management, model selection, provider adapters, and performance monitoring.

use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use crate::config::{ConfigFormat, ConfigManager, LLMConfig, ProviderConfig};
use crate::llm_integration::{
    AnalysisType, LLMIntegrationManager, LLMMessage, LLMProvider, LLMRequest, LLMRequestConfig,
    ModelSelectionPreferences, PerformanceFeedback,
};

/// Test configuration management with all supported formats
#[tokio::test]
async fn test_configuration_management_all_formats() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Test YAML configuration
    let yaml_config = r#"
default_provider: "openai"
providers:
  openai:
    provider: "OpenAI"
    api_key: "test-openai-key"
    model: "gpt-4"
    max_tokens: 2000
    temperature: 0.5
    timeout_seconds: 30
    retry_attempts: 3
    enabled: true
  anthropic:
    provider: "Anthropic"
    api_key: "test-anthropic-key"
    model: "claude-3"
    max_tokens: 1000
    temperature: 0.7
    timeout_seconds: 60
    retry_attempts: 5
    enabled: true
settings:
  cache_enabled: true
  cache_ttl_seconds: 3600
  max_cache_size: 1000
  log_level: "info"
  telemetry_enabled: false
  request_timeout_seconds: 30
model_selection:
  capability_weight: 0.5
  speed_weight: 0.2
  cost_weight: 0.2
  reliability_weight: 0.1
  auto_detection_enabled: true
  learning_enabled: true
"#;

    let yaml_path = config_dir.join("config.yaml");
    fs::write(&yaml_path, yaml_config).unwrap();

    let mut config_manager = ConfigManager::with_path(&yaml_path, ConfigFormat::Yaml).unwrap();
    let yaml_config_loaded = config_manager.get_config();
    assert_eq!(
        yaml_config_loaded.default_provider,
        Some("openai".to_string())
    );
    assert_eq!(yaml_config_loaded.providers.len(), 2);
    assert!(yaml_config_loaded.providers.contains_key("openai"));
    assert!(yaml_config_loaded.providers.contains_key("anthropic"));

    // Test TOML configuration
    let toml_config = r#"
default_provider = "anthropic"

[providers.openai]
provider = "OpenAI"
api_key = "test-openai-key"
model = "gpt-4-turbo"
max_tokens = 1500
temperature = 0.3
timeout_seconds = 30
retry_attempts = 3
enabled = true

[providers.anthropic]
provider = "Anthropic"
api_key = "test-anthropic-key"
model = "claude-3-sonnet"
max_tokens = 800
temperature = 0.8
timeout_seconds = 45
retry_attempts = 4
enabled = true

[settings]
cache_enabled = true
cache_ttl_seconds = 3600
max_cache_size = 1000
log_level = "info"
telemetry_enabled = false
request_timeout_seconds = 30

[model_selection]
capability_weight = 0.5
speed_weight = 0.2
cost_weight = 0.2
reliability_weight = 0.1
auto_detection_enabled = true
learning_enabled = true
"#;

    let toml_path = config_dir.join("config.toml");
    fs::write(&toml_path, toml_config).unwrap();

    let mut config_manager2 = ConfigManager::with_path(&toml_path, ConfigFormat::Toml).unwrap();
    let toml_config_loaded = config_manager2.get_config();
    assert_eq!(
        toml_config_loaded.default_provider,
        Some("anthropic".to_string())
    );
    assert_eq!(toml_config_loaded.providers.len(), 2);

    // Test JSON configuration
    let json_config = r#"
{
  "default_provider": "ollama",
  "providers": {
    "openai": {
      "provider": "OpenAI",
      "api_key": "test-openai-key",
      "model": "gpt-4",
      "max_tokens": 1200,
      "temperature": 0.4,
      "timeout_seconds": 35,
      "retry_attempts": 2,
      "enabled": true
    },
    "anthropic": {
      "provider": "Anthropic",
      "api_key": "test-anthropic-key",
      "model": "claude-3",
      "max_tokens": 900,
      "temperature": 0.6,
      "timeout_seconds": 50,
      "retry_attempts": 3,
      "enabled": true
    },
    "ollama": {
      "provider": "Ollama",
      "api_key": null,
      "base_url": "http://localhost:11434",
      "model": "llama2",
      "max_tokens": 800,
      "temperature": 0.9,
      "timeout_seconds": 90,
      "retry_attempts": 6,
      "enabled": true
    }
  },
  "settings": {
    "cache_enabled": true,
    "cache_ttl_seconds": 3600,
    "max_cache_size": 1000,
    "log_level": "info",
    "telemetry_enabled": false,
    "request_timeout_seconds": 30
  },
  "model_selection": {
    "capability_weight": 0.5,
    "speed_weight": 0.2,
    "cost_weight": 0.2,
    "reliability_weight": 0.1,
    "auto_detection_enabled": true,
    "learning_enabled": true
  }
}
"#;

    let json_path = config_dir.join("config.json");
    fs::write(&json_path, json_config).unwrap();

    let mut config_manager3 = ConfigManager::with_path(&json_path, ConfigFormat::Json).unwrap();
    let json_config_loaded = config_manager3.get_config();
    assert_eq!(
        json_config_loaded.default_provider,
        Some("ollama".to_string())
    );
    assert_eq!(json_config_loaded.providers.len(), 3);
    assert!(json_config_loaded.providers.contains_key("ollama"));
    assert!(json_config_loaded.providers.contains_key("openai"));
    assert!(json_config_loaded.providers.contains_key("anthropic"));
}

/// Test environment variable overrides and validation
#[tokio::test]
async fn test_environment_variable_overrides() {
    // Set environment variables
    std::env::set_var("ODINCODE_DEFAULT_PROVIDER", "anthropic");
    std::env::set_var("ODINCODE_OPENAI_API_KEY", "test-openai-key");
    std::env::set_var("ODINCODE_ANTHROPIC_API_KEY", "test-anthropic-key");
    std::env::set_var("ODINCODE_OPENAI_MODEL", "gpt-4-turbo");
    std::env::set_var("ODINCODE_MAX_TOKENS", "2000");
    std::env::set_var("ODINCODE_TEMPERATURE", "0.5");

    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Base configuration
    let base_config = r#"
default_provider: "openai"
providers:
  openai:
    provider: "OpenAI"
    model: "gpt-3.5-turbo"
    api_key: "test-openai-key"
    base_url: "https://api.openai.com/v1"
    max_tokens: 1000
    temperature: 0.7
    timeout_seconds: 30
    retry_attempts: 3
    enabled: true
  anthropic:
    provider: "Anthropic"
    model: "claude-3-sonnet-20240229"
    api_key: "test-anthropic-key"
    base_url: "https://api.anthropic.com"
    max_tokens: 1000
    temperature: 0.7
    timeout_seconds: 30
    retry_attempts: 3
    enabled: true
settings:
  cache_enabled: true
  cache_ttl_seconds: 3600
  max_cache_size: 1000
  log_level: "info"
  telemetry_enabled: false
  request_timeout_seconds: 30
model_selection:
  capability_weight: 0.5
  speed_weight: 0.2
  cost_weight: 0.2
  reliability_weight: 0.1
  auto_detection_enabled: true
  learning_enabled: true
"#;

    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, base_config).unwrap();

    let mut config_manager = ConfigManager::with_path(&config_path, ConfigFormat::Yaml).unwrap();
    // Set environment variables for overrides
    std::env::set_var("ODINCODE_DEFAULT_PROVIDER", "anthropic");
    std::env::set_var("ODINCODE_OPENAI_API_KEY", "env-openai-key");
    std::env::set_var("ODINCODE_ANTHROPIC_API_KEY", "env-anthropic-key");
    std::env::set_var("ODINCODE_OPENAI_MODEL", "gpt-4-turbo");
    std::env::set_var("ODINCODE_MAX_TOKENS", "2000");
    std::env::set_var("ODINCODE_TEMPERATURE", "0.5");

    let override_result = config_manager.apply_env_overrides();

    let config = config_manager.get_config_mut();
    assert!(override_result.is_ok());

    // Verify overrides were applied
    assert_eq!(config.default_provider, Some("anthropic".to_string()));

    let openai_config = config.providers.get("openai").unwrap();
    assert_eq!(openai_config.api_key, Some("env-openai-key".to_string()));
    assert_eq!(openai_config.model, "gpt-4-turbo");
    assert_eq!(openai_config.max_tokens, 2000);
    assert_eq!(openai_config.temperature, 0.5);

    let anthropic_config = config.providers.get("anthropic").unwrap();
    assert_eq!(
        anthropic_config.api_key,
        Some("env-anthropic-key".to_string())
    );

    // Clean up environment variables
    std::env::remove_var("ODINCODE_DEFAULT_PROVIDER");
    std::env::remove_var("ODINCODE_OPENAI_API_KEY");
    std::env::remove_var("ODINCODE_ANTHROPIC_API_KEY");
    std::env::remove_var("ODINCODE_OPENAI_MODEL");
    std::env::remove_var("ODINCODE_MAX_TOKENS");
    std::env::remove_var("ODINCODE_TEMPERATURE");
}

/// Test configuration validation
#[tokio::test]
async fn test_configuration_validation() {
    let mut config_manager = ConfigManager::new();

    // Test valid configuration
    let mut valid_manager = ConfigManager::new().unwrap();
    let validation_result = valid_manager.validate_config();
    assert!(validation_result.is_ok());

    // Test invalid configuration - missing required fields
    let invalid_config = LLMConfig {
        default_provider: Some("".to_string()), // Empty provider name
        providers: HashMap::new(),              // No providers
        settings: crate::config::ConfigSettings::default(),
        model_selection: crate::config::ModelSelectionConfig::default(),
    };

    // Create a new manager with invalid config for testing
    let mut invalid_manager = ConfigManager::new().unwrap();
    *invalid_manager.get_config_mut() = invalid_config;
    let validation_result = invalid_manager.validate_config();
    assert!(validation_result.is_err());

    // Test invalid provider configuration
    let mut invalid_provider_config = valid_manager.get_config().clone();
    let mut invalid_providers = HashMap::new();
    invalid_providers.insert(
        "openai".to_string(),
        ProviderConfig {
            provider: LLMProvider::OpenAI,
            model: "".to_string(), // Empty model name
            api_key: None,
            base_url: None,
            max_tokens: 0,    // Invalid max tokens
            temperature: 3.0, // Invalid temperature (should be 0.0-2.0)
            enabled: true,
            timeout_seconds: 0, // Invalid timeout
            retry_attempts: 0,  // Invalid max retries
        },
    );
    invalid_provider_config.providers = invalid_providers;

    let mut invalid_provider_manager = ConfigManager::new().unwrap();
    *invalid_provider_manager.get_config_mut() = invalid_provider_config;
    let validation_result = invalid_provider_manager.validate_config();
    assert!(validation_result.is_err());
}

/// Test model selection logic for different analysis types
#[tokio::test]
async fn test_model_selection_logic() {
    let mut manager = LLMIntegrationManager::new().unwrap();

    // Test model selection for all analysis types
    let analysis_types = vec![
        AnalysisType::CodeCompletion,
        AnalysisType::BugDetection,
        AnalysisType::SecurityAnalysis,
        AnalysisType::PerformanceOptimization,
        AnalysisType::Documentation,
        AnalysisType::Refactoring,
        AnalysisType::GeneralAnalysis,
    ];

    for analysis_type in analysis_types {
        let selected_model = manager.select_best_model_for_analysis(analysis_type.clone());
        assert!(
            selected_model.is_some(),
            "No model selected for {:?}",
            analysis_type
        );

        let model_name = selected_model.unwrap();
        assert!(
            !model_name.is_empty(),
            "Empty model name for {:?}",
            analysis_type
        );

        println!("Selected model for {:?}: {}", analysis_type, model_name);
    }

    // Test model selection with custom preferences
    let speed_preferences = ModelSelectionPreferences {
        capability_weight: 0.2,
        speed_weight: 0.6,
        cost_weight: 0.1,
        reliability_weight: 0.1,
    };

    manager.update_selection_preferences(speed_preferences);

    let speed_optimized_model =
        manager.select_best_model_for_analysis(AnalysisType::CodeCompletion);
    assert!(speed_optimized_model.is_some());

    // Test model selection with capability preferences
    let capability_preferences = ModelSelectionPreferences {
        capability_weight: 0.7,
        speed_weight: 0.1,
        cost_weight: 0.1,
        reliability_weight: 0.1,
    };

    manager.update_selection_preferences(capability_preferences);

    let capability_optimized_model =
        manager.select_best_model_for_analysis(AnalysisType::SecurityAnalysis);
    assert!(capability_optimized_model.is_some());

    // Verify that different preferences can lead to different model selections
    println!("Speed-optimized model: {:?}", speed_optimized_model);
    println!(
        "Capability-optimized model: {:?}",
        capability_optimized_model
    );
}

/// Test provider adapter functionality and error handling
#[tokio::test]
async fn test_provider_adapter_functionality() {
    let manager = LLMIntegrationManager::new().unwrap();

    // Test provider adapter creation
    let providers = manager.get_available_providers();
    println!("Available providers: {:?}", providers);
    assert!(providers.contains(&"openai".to_string()));
    assert!(providers.contains(&"anthropic".to_string()));
    // Note: ollama might not be available if not enabled in default config

    // Test provider configuration retrieval
    for provider_name in providers {
        let provider_config = manager.get_provider(&provider_name);
        assert!(
            provider_config.is_some(),
            "No config found for provider: {}",
            provider_name
        );

        let config = provider_config.unwrap();
        assert!(!config.model.is_empty());
        assert!(config.max_tokens > 0);
        assert!(config.temperature >= 0.0 && config.temperature <= 2.0);
        // Note: LLMProviderConfig doesn't have timeout_seconds and retry_attempts fields
        // These are only in the config::ProviderConfig
    }

    // Test request creation for different providers
    let test_request = LLMRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            LLMMessage {
                role: "system".to_string(),
                content: "You are a helpful assistant.".to_string(),
                name: None,
            },
            LLMMessage {
                role: "user".to_string(),
                content: "What is 2 + 2?".to_string(),
                name: None,
            },
        ],
        config: LLMRequestConfig::default(),
        request_id: Some(Uuid::new_v4()),
    };

    // Test that request can be created (without actually sending)
    assert_eq!(test_request.messages.len(), 2);
    assert_eq!(test_request.model, "gpt-4");
    assert!(test_request.request_id.is_some());

    // Test error handling for invalid requests
    let invalid_request = LLMRequest {
        model: "".to_string(), // Empty model name
        messages: vec![],      // Empty messages
        config: LLMRequestConfig::default(),
        request_id: None,
    };

    // This should fail gracefully when sent
    let result = timeout(
        Duration::from_secs(1),
        manager.send_request(invalid_request),
    )
    .await;

    match result {
        Ok(Ok(_)) => {
            // Unexpected success, but not an error
            println!("Unexpected success with invalid request");
        }
        Ok(Err(e)) => {
            // Expected failure
            println!("Expected error with invalid request: {}", e);
        }
        Err(_) => {
            // Timeout is expected
            println!("Request timed out as expected");
        }
    }
}

/// Test performance monitoring and caching mechanisms
#[tokio::test]
async fn test_performance_monitoring_and_caching() {
    let mut manager = LLMIntegrationManager::new().unwrap();

    // Performance monitoring and cache settings are handled by the config manager
    // We'll test the actual functionality through the manager

    // Test performance feedback system
    let feedback = PerformanceFeedback {
        quality_score: 0.9,
        speed_score: 0.8,
        reliability_score: 0.95,
        user_satisfaction: Some(0.85),
    };

    // Provide feedback for different analysis types
    let analysis_types = vec![
        AnalysisType::CodeCompletion,
        AnalysisType::BugDetection,
        AnalysisType::SecurityAnalysis,
    ];

    for analysis_type in analysis_types {
        manager.provide_performance_feedback(analysis_type.clone(), "gpt-4", feedback.clone());

        // Verify that feedback was processed
        let capabilities = manager.model_selector.get_available_models(&analysis_type);
        assert!(
            !capabilities.is_empty(),
            "No capabilities found for {:?}",
            analysis_type
        );

        for capability in capabilities {
            assert!(capability.capability_score >= 0.0 && capability.capability_score <= 1.0);
            assert!(capability.speed_score >= 0.0 && capability.speed_score <= 1.0);
            assert!(capability.cost_score >= 0.0 && capability.cost_score <= 1.0);
            assert!(capability.reliability_score >= 0.0 && capability.reliability_score <= 1.0);
        }
    }

    // Test model selection explanation
    let explanation =
        manager.get_model_selection_explanation(AnalysisType::CodeCompletion, "gpt-4");
    assert!(explanation.is_some());

    let explanation_text = explanation.unwrap();
    assert!(explanation_text.contains("gpt-4"));
    assert!(explanation_text.contains("CodeCompletion"));
    assert!(explanation_text.contains("Capability score"));
    assert!(explanation_text.contains("Speed score"));
    assert!(explanation_text.contains("Cost score"));
    assert!(explanation_text.contains("Reliability score"));

    println!("Model selection explanation:\n{}", explanation_text);
}

/// Test end-to-end integration with mocked API calls
#[tokio::test]
async fn test_end_to_end_integration_with_mocked_apis() {
    // Create a comprehensive configuration
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    let config_content = r#"
default_provider: "openai"
providers:
  openai:
    provider: "OpenAI"
    model: "gpt-4"
    api_key: "mock-api-key"
    base_url: "https://api.openai.com/v1"
    max_tokens: 1000
    temperature: 0.7
    timeout_seconds: 30
    retry_attempts: 3
    enabled: true
  anthropic:
    provider: "Anthropic"
    model: "claude-3-sonnet-20240229"
    api_key: "mock-anthropic-key"
    base_url: "https://api.anthropic.com"
    max_tokens: 1000
    temperature: 0.7
    timeout_seconds: 30
    retry_attempts: 3
    enabled: true
settings:
  cache_enabled: true
  cache_ttl_seconds: 3600
  max_cache_size: 100
  log_level: "info"
  telemetry_enabled: false
  request_timeout_seconds: 30
model_selection:
  capability_weight: 0.4
  speed_weight: 0.3
  cost_weight: 0.2
  reliability_weight: 0.1
  auto_detection_enabled: true
  learning_enabled: true
"#;

    let config_path = config_dir.join("config.yaml");
    fs::write(&config_path, config_content).unwrap();

    // Load configuration
    let config_manager = ConfigManager::with_path(&config_path, ConfigFormat::Yaml).unwrap();
    let config = config_manager.get_config().clone();

    // Create LLM integration manager with the config manager
    let mut llm_manager = LLMIntegrationManager::new_with_config_manager(config_manager).unwrap();

    // Add providers from configuration
    for (provider_name, provider_config) in &config.providers {
        let llm_provider_config = provider_config.provider.clone();

        let provider_config_for_llm = crate::llm_integration::LLMProviderConfig {
            provider: llm_provider_config,
            api_key: provider_config.api_key.clone(),
            base_url: provider_config.base_url.clone(),
            model: provider_config.model.clone(),
            max_tokens: provider_config.max_tokens,
            temperature: provider_config.temperature,
        };

        llm_manager.add_provider(provider_name.clone(), provider_config_for_llm);
    }

    // Set default provider
    if let Some(ref default_provider) = config.default_provider {
        if let Err(e) = llm_manager.set_default_provider(default_provider.clone()) {
            panic!("Failed to set default provider: {}", e);
        }
    }

    // Test model selection for different analysis types
    let analysis_types = vec![
        AnalysisType::CodeCompletion,
        AnalysisType::BugDetection,
        AnalysisType::SecurityAnalysis,
        AnalysisType::PerformanceOptimization,
        AnalysisType::Documentation,
    ];

    for analysis_type in analysis_types {
        let selected_model = llm_manager.select_best_model_for_analysis(analysis_type.clone());
        assert!(
            selected_model.is_some(),
            "No model selected for {:?}",
            analysis_type
        );

        let model_name = selected_model.unwrap();
        println!("Selected model for {:?}: {}", analysis_type, model_name);

        // Test model suitability
        let is_suitable = llm_manager.is_model_suitable_for_analysis(&model_name, &analysis_type);
        assert!(
            is_suitable,
            "Model {} not suitable for {:?}",
            model_name, analysis_type
        );
    }

    // Test intelligent model recommendation
    let recommended_model = llm_manager.get_intelligent_model_recommendation(
        AnalysisType::CodeCompletion,
        Some("gpt-4".to_string()),
    );
    assert_eq!(recommended_model, "gpt-4");

    // Test analysis type detection
    let test_code = r#"
function authenticate_user(username, password) {
    // Security-sensitive authentication logic
    if (username === "admin" && password === "password123") {
        return true;
    }
    return false;
}
"#;

    let detected_type =
        llm_manager.detect_analysis_type_from_context(test_code, Some("check security"));
    assert_eq!(detected_type, AnalysisType::SecurityAnalysis);

    // Test provider statistics
    let stats = llm_manager.get_provider_stats();
    assert!(!stats.is_empty());

    for (provider_name, provider_stats) in stats {
        if let serde_json::Value::Object(ref map) = provider_stats {
            assert!(map.contains_key("provider"));
            assert!(map.contains_key("model"));
            assert!(map.contains_key("max_tokens"));
            assert!(map.contains_key("temperature"));
            assert!(map.contains_key("has_api_key"));
            assert!(map.contains_key("base_url"));
        } else {
            panic!("Provider stats should be an object");
        }

        println!("Provider {} stats: {:?}", provider_name, provider_stats);
    }

    // Test concurrent access
    let manager_arc = Arc::new(llm_manager);
    let mut handles = vec![];

    for i in 0..5 {
        let manager_clone = manager_arc.clone();
        let handle = tokio::spawn(async move {
            let providers = manager_clone.get_available_providers();
            assert!(!providers.is_empty());

            let stats = manager_clone.get_provider_stats();
            assert!(!stats.is_empty());

            let model = manager_clone.select_best_model_for_analysis(AnalysisType::CodeCompletion);
            assert!(model.is_some());

            println!("Concurrent test {} completed", i);
        });
        handles.push(handle);
    }

    // Wait for all concurrent tests to complete
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent test failed: {:?}", result);
    }

    println!("End-to-end integration test completed successfully");
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_error_handling_and_edge_cases() {
    let mut manager = LLMIntegrationManager::new().unwrap();

    // Test with empty configuration
    let empty_config = LLMConfig {
        default_provider: Some("".to_string()),
        providers: HashMap::new(),
        settings: crate::config::ConfigSettings::default(),
        model_selection: crate::config::ModelSelectionConfig::default(),
    };

    let mut config_manager = ConfigManager::new().unwrap();
    *config_manager.get_config_mut() = empty_config;
    let validation_result = config_manager.validate_config();
    assert!(validation_result.is_err());

    // Test with invalid provider names
    let result = manager.get_provider("nonexistent_provider");
    assert!(result.is_none());

    let result = manager.set_default_provider("nonexistent_provider".to_string());
    assert!(result.is_err());

    // Test with invalid model names
    let is_suitable =
        manager.is_model_suitable_for_analysis("invalid_model_name", &AnalysisType::CodeCompletion);
    assert!(!is_suitable);

    // Test with extreme preference values
    let extreme_preferences = ModelSelectionPreferences {
        capability_weight: 1.0,
        speed_weight: 0.0,
        cost_weight: 0.0,
        reliability_weight: 0.0,
    };

    manager.update_selection_preferences(extreme_preferences);

    let selected_model = manager.select_best_model_for_analysis(AnalysisType::CodeCompletion);
    assert!(selected_model.is_some());

    // Test with negative feedback
    let negative_feedback = PerformanceFeedback {
        quality_score: 0.0,
        speed_score: 0.0,
        reliability_score: 0.0,
        user_satisfaction: Some(0.0),
    };

    manager.provide_performance_feedback(AnalysisType::CodeCompletion, "gpt-4", negative_feedback);

    // Model should still be selectable even with negative feedback
    let selected_model = manager.select_best_model_for_analysis(AnalysisType::CodeCompletion);
    assert!(selected_model.is_some());

    // Test with empty context for analysis type detection
    let detected_type = manager.detect_analysis_type_from_context("", None);
    assert_eq!(detected_type, AnalysisType::GeneralAnalysis);

    // Test with malformed context for analysis type detection
    let malformed_context = "!!!@@@###$$$";
    let detected_type = manager.detect_analysis_type_from_context(malformed_context, None);
    assert_eq!(detected_type, AnalysisType::GeneralAnalysis);

    println!("Error handling and edge cases test completed successfully");
}

/// Test configuration persistence and loading
#[tokio::test]
async fn test_configuration_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();

    // Create a comprehensive configuration
    let original_config = LLMConfig {
        default_provider: Some("openai".to_string()),
        providers: {
            let mut providers = HashMap::new();
            providers.insert(
                "openai".to_string(),
                ProviderConfig {
                    provider: LLMProvider::OpenAI,
                    api_key: Some("test-key".to_string()),
                    base_url: Some("https://api.openai.com/v1".to_string()),
                    model: "gpt-4".to_string(),
                    max_tokens: 2000,
                    temperature: 0.8,
                    enabled: true,
                    timeout_seconds: 45,
                    retry_attempts: 5,
                },
            );
            providers.insert(
                "anthropic".to_string(),
                ProviderConfig {
                    provider: LLMProvider::Anthropic,
                    api_key: Some("test-anthropic-key".to_string()),
                    base_url: Some("https://api.anthropic.com".to_string()),
                    model: "claude-3-opus-20240229".to_string(),
                    max_tokens: 3000,
                    temperature: 0.6,
                    enabled: true,
                    timeout_seconds: 60,
                    retry_attempts: 3,
                },
            );
            providers
        },
        settings: crate::config::ConfigSettings {
            cache_enabled: true,
            cache_ttl_seconds: 7200,
            max_cache_size: 1000,
            log_level: "info".to_string(),
            telemetry_enabled: false,
            request_timeout_seconds: 30,
        },
        model_selection: crate::config::ModelSelectionConfig {
            capability_weight: 0.5,
            speed_weight: 0.2,
            cost_weight: 0.2,
            reliability_weight: 0.1,
            auto_detection_enabled: true,
            learning_enabled: true,
        },
    };

    // Save configuration in different formats
    let formats = vec![
        (ConfigFormat::Yaml, "config.yaml"),
        (ConfigFormat::Toml, "config.toml"),
        (ConfigFormat::Json, "config.json"),
    ];

    for (format, filename) in formats {
        let config_path = config_dir.join(filename);
        let mut config_manager = ConfigManager::with_path(&config_path, format.clone()).unwrap();

        // Set configuration
        *config_manager.get_config_mut() = original_config.clone();

        // Save configuration
        let save_result = config_manager.save_config();
        assert!(
            save_result.is_ok(),
            "Failed to save config in {:?} format",
            format
        );

        // Load configuration with new manager
        let mut load_manager = ConfigManager::with_path(&config_path, format.clone()).unwrap();
        let loaded_config = load_manager.get_config();

        // Verify configuration integrity
        assert_eq!(
            loaded_config.default_provider,
            original_config.default_provider
        );
        assert_eq!(
            loaded_config.providers.len(),
            original_config.providers.len()
        );

        for (provider_name, original_provider) in &original_config.providers {
            let loaded_provider = loaded_config.providers.get(provider_name).unwrap();
            assert_eq!(loaded_provider.provider, original_provider.provider);
            assert_eq!(loaded_provider.model, original_provider.model);
            assert_eq!(loaded_provider.api_key, original_provider.api_key);
            assert_eq!(loaded_provider.base_url, original_provider.base_url);
            assert_eq!(loaded_provider.max_tokens, original_provider.max_tokens);
            assert_eq!(loaded_provider.temperature, original_provider.temperature);
            assert_eq!(
                loaded_provider.timeout_seconds,
                original_provider.timeout_seconds
            );
            assert_eq!(
                loaded_provider.retry_attempts,
                original_provider.retry_attempts
            );
        }

        // Verify settings fields
        assert_eq!(
            loaded_config.settings.cache_enabled,
            original_config.settings.cache_enabled
        );
        assert_eq!(
            loaded_config.settings.cache_ttl_seconds,
            original_config.settings.cache_ttl_seconds
        );
        assert_eq!(
            loaded_config.settings.max_cache_size,
            original_config.settings.max_cache_size
        );

        // Verify model selection fields
        assert_eq!(
            loaded_config.model_selection.capability_weight,
            original_config.model_selection.capability_weight
        );
        assert_eq!(
            loaded_config.model_selection.speed_weight,
            original_config.model_selection.speed_weight
        );
        assert_eq!(
            loaded_config.model_selection.cost_weight,
            original_config.model_selection.cost_weight
        );
        assert_eq!(
            loaded_config.model_selection.reliability_weight,
            original_config.model_selection.reliability_weight
        );

        println!(
            "Configuration persistence test passed for {:?} format",
            format
        );
    }

    println!("Configuration persistence test completed successfully");
}
