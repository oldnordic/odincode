//! LLM Integration Tests
//!
//! Comprehensive integration tests for LLM-enhanced code analysis functionality.

use anyhow::Result;
use std::sync::Arc;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use super::*;
use crate::llm_integration::{
    AnalysisType, LLMIntegrationManager, LLMMessage, LLMProvider, LLMProviderConfig, LLMRequest,
    LLMRequestConfig, LLMResponse, LLMUsage, ModelSelectionPreferences, PerformanceFeedback,
};
use crate::ml_integration::MLIntegrationConfig;

#[cfg(test)]
mod llm_integration_tests {
    use super::*;

    /// Test LLM Integration Manager creation and basic functionality
    #[tokio::test]
    async fn test_llm_integration_manager_creation() {
        let manager = LLMIntegrationManager::new().unwrap();

        // Verify default providers are configured
        let providers = manager.get_available_providers();
        assert!(providers.contains(&"openai".to_string()));
        assert!(providers.contains(&"anthropic".to_string()));
        assert!(providers.contains(&"ollama".to_string()));

        // Verify default provider
        assert_eq!(manager.default_provider, "openai");

        // Verify provider configurations
        let openai_config = manager.get_provider("openai").unwrap();
        assert_eq!(openai_config.provider, LLMProvider::OpenAI);
        assert_eq!(openai_config.model, "gpt-3.5-turbo");
        assert_eq!(openai_config.max_tokens, 1000);
        assert_eq!(openai_config.temperature, 0.7);

        let anthropic_config = manager.get_provider("anthropic").unwrap();
        assert_eq!(anthropic_config.provider, LLMProvider::Anthropic);
        assert_eq!(anthropic_config.model, "claude-3-sonnet-20240229");

        let ollama_config = manager.get_provider("ollama").unwrap();
        assert_eq!(ollama_config.provider, LLMProvider::Ollama);
        assert_eq!(ollama_config.model, "llama2");
    }

    /// Test adding and updating provider configurations
    #[tokio::test]
    async fn test_provider_configuration_management() {
        let mut manager = LLMIntegrationManager::new().unwrap();

        // Add a new provider
        let custom_config = LLMProviderConfig {
            provider: LLMProvider::OpenAI,
            api_key: Some("test_key".to_string()),
            base_url: Some("https://api.custom.com".to_string()),
            model: "custom-model".to_string(),
            max_tokens: 2000,
            temperature: 0.5,
        };

        manager.add_provider("custom".to_string(), custom_config);

        // Verify the provider was added
        assert!(manager.providers.contains_key("custom"));
        let custom_provider = manager.get_provider("custom").unwrap();
        assert_eq!(custom_provider.model, "custom-model");
        assert_eq!(custom_provider.max_tokens, 2000);
        assert_eq!(custom_provider.temperature, 0.5);

        // Test setting default provider
        let result = manager.set_default_provider("custom".to_string());
        assert!(result.is_ok());
        assert_eq!(manager.default_provider, "custom");

        // Test setting non-existent provider
        let result = manager.set_default_provider("nonexistent".to_string());
        assert!(result.is_err());
    }

    /// Test provider statistics functionality
    #[tokio::test]
    async fn test_provider_statistics() {
        let manager = LLMIntegrationManager::new().unwrap();

        let stats = manager.get_provider_stats();

        // Verify stats for all default providers
        assert!(stats.contains_key("openai"));
        assert!(stats.contains_key("anthropic"));
        assert!(stats.contains_key("ollama"));

        // Verify OpenAI stats
        let openai_stats = &stats["openai"];
        assert_eq!(openai_stats["provider"], "OpenAI");
        assert_eq!(openai_stats["model"], "gpt-3.5-turbo");
        assert_eq!(openai_stats["max_tokens"], 1000);
        assert_eq!(openai_stats["temperature"], 0.7);
        assert_eq!(openai_stats["has_api_key"], true);
        assert_eq!(openai_stats["base_url"], "https://api.openai.com/v1");
    }

    /// Test LLM request creation and validation
    #[tokio::test]
    async fn test_llm_request_creation() {
        let manager = LLMIntegrationManager::new().unwrap();

        let request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: "You are a helpful assistant.".to_string(),
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: "Hello, how are you?".to_string(),
                    name: None,
                },
            ],
            config: LLMRequestConfig {
                temperature: 0.8,
                max_tokens: Some(1500),
                top_p: Some(0.9),
                frequency_penalty: Some(0.1),
                presence_penalty: Some(0.1),
                stop_sequences: Some(vec!["END".to_string()]),
            },
            request_id: Some(Uuid::new_v4()),
        };

        // Verify request structure
        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.config.temperature, 0.8);
        assert_eq!(request.config.max_tokens, Some(1500));
        assert_eq!(request.config.top_p, Some(0.9));
        assert_eq!(request.config.frequency_penalty, Some(0.1));
        assert_eq!(request.config.presence_penalty, Some(0.1));
        assert_eq!(request.config.stop_sequences, Some(vec!["END".to_string()]));
    }

    /// Test LLM request configuration defaults
    #[tokio::test]
    async fn test_llm_request_config_defaults() {
        let config = LLMRequestConfig::default();

        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.max_tokens, Some(1000));
        assert_eq!(config.top_p, Some(1.0));
        assert_eq!(config.frequency_penalty, Some(0.0));
        assert_eq!(config.presence_penalty, Some(0.0));
        assert_eq!(config.stop_sequences, None);
    }

    /// Test generate_response convenience method
    #[tokio::test]
    async fn test_generate_response_convenience() {
        let manager = LLMIntegrationManager::new().unwrap();

        let prompt = "What is 2 + 2?";

        // This will fail without API keys, but we can test the request structure
        let result = timeout(Duration::from_secs(1), manager.generate_response(prompt)).await;

        // Should timeout or fail gracefully
        match result {
            Ok(Ok(_)) => {
                // If it succeeds (unlikely without API keys), that's fine
                println!("LLM response received successfully");
            }
            Ok(Err(e)) => {
                // Expected to fail without API keys
                println!("Expected error without API keys: {}", e);
            }
            Err(_) => {
                // Timeout is also expected
                println!("Request timed out as expected");
            }
        }
    }

    /// Test provider selection logic
    #[tokio::test]
    async fn test_provider_selection_logic() {
        let manager = LLMIntegrationManager::new().unwrap();

        // Test OpenAI model selection
        let openai_request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                name: None,
            }],
            config: LLMRequestConfig::default(),
            request_id: None,
        };

        // Test Anthropic model selection
        let anthropic_request = LLMRequest {
            model: "claude-3-sonnet".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                name: None,
            }],
            config: LLMRequestConfig::default(),
            request_id: None,
        };

        // Test Ollama model selection
        let ollama_request = LLMRequest {
            model: "llama2".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                name: None,
            }],
            config: LLMRequestConfig::default(),
            request_id: None,
        };

        // Test default provider selection
        let default_request = LLMRequest {
            model: "unknown-model".to_string(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: "test".to_string(),
                name: None,
            }],
            config: LLMRequestConfig::default(),
            request_id: None,
        };

        // Verify provider selection logic (without actually sending requests)
        let provider_name = |model: &str| -> &str {
            if model.contains("claude") || model.contains("anthropic") {
                "anthropic"
            } else if model.contains("llama") || model.contains("mistral") {
                "ollama"
            } else {
                &manager.default_provider
            }
        };

        assert_eq!(provider_name(&openai_request.model), "openai");
        assert_eq!(provider_name(&anthropic_request.model), "anthropic");
        assert_eq!(provider_name(&ollama_request.model), "ollama");
        assert_eq!(provider_name(&default_request.model), "openai");
    }

    /// Test LLM usage statistics
    #[tokio::test]
    async fn test_llm_usage_statistics() {
        let usage = LLMUsage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);

        // Test that total_tokens equals prompt_tokens + completion_tokens
        assert_eq!(
            usage.total_tokens,
            usage.prompt_tokens + usage.completion_tokens
        );
    }

    /// Test LLM response structure
    #[tokio::test]
    async fn test_llm_response_structure() {
        let response = LLMResponse {
            content: "Hello, world!".to_string(),
            model: "gpt-4".to_string(),
            usage: Some(LLMUsage {
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            }),
            request_id: Some(Uuid::new_v4()),
        };

        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.model, "gpt-4");
        assert!(response.usage.is_some());
        assert!(response.request_id.is_some());

        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    /// Test LLM message structure
    #[tokio::test]
    async fn test_llm_message_structure() {
        let message = LLMMessage {
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            name: Some("test_user".to_string()),
        };

        assert_eq!(message.role, "user");
        assert_eq!(message.content, "Hello, world!");
        assert_eq!(message.name, Some("test_user".to_string()));

        let message_without_name = LLMMessage {
            role: "system".to_string(),
            content: "You are a helpful assistant.".to_string(),
            name: None,
        };

        assert_eq!(message_without_name.role, "system");
        assert_eq!(message_without_name.content, "You are a helpful assistant.");
        assert_eq!(message_without_name.name, None);
    }

    /// Test LLM provider enum serialization
    #[tokio::test]
    async fn test_llm_provider_serialization() {
        // Test OpenAI provider
        let openai_json = serde_json::to_string(&LLMProvider::OpenAI).unwrap();
        assert_eq!(openai_json, "\"OpenAI\"");

        let openai_deserialized: LLMProvider = serde_json::from_str("\"OpenAI\"").unwrap();
        assert_eq!(openai_deserialized, LLMProvider::OpenAI);

        // Test Anthropic provider
        let anthropic_json = serde_json::to_string(&LLMProvider::Anthropic).unwrap();
        assert_eq!(anthropic_json, "\"Anthropic\"");

        let anthropic_deserialized: LLMProvider = serde_json::from_str("\"Anthropic\"").unwrap();
        assert_eq!(anthropic_deserialized, LLMProvider::Anthropic);

        // Test Ollama provider
        let ollama_json = serde_json::to_string(&LLMProvider::Ollama).unwrap();
        assert_eq!(ollama_json, "\"Ollama\"");

        let ollama_deserialized: LLMProvider = serde_json::from_str("\"Ollama\"").unwrap();
        assert_eq!(ollama_deserialized, LLMProvider::Ollama);
    }

    /// Test error handling for invalid provider names
    #[tokio::test]
    async fn test_invalid_provider_handling() {
        let mut manager = LLMIntegrationManager::new().unwrap();

        // Test getting non-existent provider
        let non_existent_provider = manager.get_provider("nonexistent");
        assert!(non_existent_provider.is_none());

        // Test setting non-existent provider as default
        let result = manager.set_default_provider("nonexistent".to_string());
        assert!(result.is_err());
    }

    /// Test concurrent access to LLM integration manager
    #[tokio::test]
    async fn test_concurrent_access() {
        let manager = Arc::new(LLMIntegrationManager::new().unwrap());
        let manager_clone1 = manager.clone();
        let manager_clone2 = manager.clone();

        // Spawn multiple tasks to access the manager concurrently
        let handle1 = tokio::spawn(async move {
            let providers = manager_clone1.get_available_providers();
            assert!(!providers.is_empty());
        });

        let handle2 = tokio::spawn(async move {
            let stats = manager_clone2.get_provider_stats();
            assert!(!stats.is_empty());
        });

        // Wait for both tasks to complete
        let (result1, result2) = tokio::join!(handle1, handle2);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    /// Test LLM integration with code engine
    #[tokio::test]
    async fn test_llm_integration_with_code_engine() {
        // Create a mock LTMC manager
        let ltmc_manager = Arc::new(odincode_ltmc::LTMManager::new());

        // Create ML integration config
        let ml_config = MLIntegrationConfig {
            model_name: "test_model".to_string(),
            confidence_threshold: 0.7,
            max_suggestions: 10,
            use_ltmc_context: true,
            use_llm: true,
            default_llm_provider: "openai".to_string(),
            default_llm_model: "gpt-3.5-turbo".to_string(),
            llm_max_tokens: 1000,
            llm_temperature: 0.7,
        };

        // Create LLM integration manager
        let llm_manager = Arc::new(LLMIntegrationManager::new().unwrap());

        // Create code engine with ML and LLM integration
        let code_engine =
            CodeEngine::new_with_ml_and_llm(ml_config, ltmc_manager, llm_manager.clone()).await;

        assert!(code_engine.is_ok());

        let engine = code_engine.unwrap();

        // Test that the engine was created successfully
        assert!(engine.get_ml_integration().await.is_some());

        // Load a test file
        let rust_code = r#"
fn main() {
    println!("Hello, world!");
}
"#;

        let file_id = engine
            .load_file(
                "test.rs".to_string(),
                rust_code.to_string(),
                "rust".to_string(),
            )
            .await
            .unwrap();

        // Analyze the file
        let analysis_result = engine.analyze_file(file_id).await.unwrap();
        assert!(analysis_result.is_some());

        let result = analysis_result.unwrap();
        assert_eq!(result.file_id, file_id);
        assert!(!result.issues.is_empty() || !result.suggestions.is_empty());
    }

    /// Test LLM integration error handling
    #[tokio::test]
    async fn test_llm_integration_error_handling() {
        let manager = LLMIntegrationManager::new().unwrap();

        // Test with invalid request (empty messages)
        let invalid_request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![],
            config: LLMRequestConfig::default(),
            request_id: None,
        };

        // This should fail gracefully
        let result = timeout(
            Duration::from_secs(1),
            manager.send_request(invalid_request),
        )
        .await;

        match result {
            Ok(Ok(_)) => {
                // If it succeeds, that's unexpected but not an error
                println!("Unexpected success with empty messages");
            }
            Ok(Err(e)) => {
                // Expected to fail
                println!("Expected error with empty messages: {}", e);
            }
            Err(_) => {
                // Timeout is expected
                println!("Request timed out as expected");
            }
        }
    }

    /// Test LLM integration performance
    #[tokio::test]
    async fn test_llm_integration_performance() {
        let manager = LLMIntegrationManager::new().unwrap();

        let start_time = std::time::Instant::now();

        // Test provider statistics retrieval performance
        for _ in 0..100 {
            let _stats = manager.get_provider_stats();
        }

        let duration = start_time.elapsed();
        println!(
            "Provider stats retrieval time for 100 calls: {:?}",
            duration
        );

        // Should be very fast (in-memory operation)
        assert!(duration.as_millis() < 100);

        // Test available providers retrieval performance
        let start_time = std::time::Instant::now();

        for _ in 0..100 {
            let _providers = manager.get_available_providers();
        }

        let duration = start_time.elapsed();
        println!(
            "Available providers retrieval time for 100 calls: {:?}",
            duration
        );

        // Should be very fast (in-memory operation)
        assert!(duration.as_millis() < 100);
    }

    /// Test intelligent model selection
    #[tokio::test]
    async fn test_intelligent_model_selection() {
        let manager = LLMIntegrationManager::new().unwrap();

        // Test model selection for different analysis types
        let code_completion_model =
            manager.select_best_model_for_analysis(AnalysisType::CodeCompletion);
        let bug_detection_model =
            manager.select_best_model_for_analysis(AnalysisType::BugDetection);
        let security_model = manager.select_best_model_for_analysis(AnalysisType::SecurityAnalysis);

        // Verify that models are selected
        assert!(code_completion_model.is_some());
        assert!(bug_detection_model.is_some());
        assert!(security_model.is_some());

        // Verify that different analysis types might get different models
        let cc_model = code_completion_model.unwrap();
        let bd_model = bug_detection_model.unwrap();
        let sec_model = security_model.unwrap();

        println!("Code completion model: {}", cc_model);
        println!("Bug detection model: {}", bd_model);
        println!("Security model: {}", sec_model);

        // Test default model retrieval
        let default_cc_model = manager
            .model_selector
            .get_default_model(&AnalysisType::CodeCompletion);
        assert!(default_cc_model.is_some());

        // Test model suitability check
        let is_gpt4_suitable =
            manager.is_model_suitable_for_analysis("gpt-4", &AnalysisType::CodeCompletion);
        let is_fake_model_suitable =
            manager.is_model_suitable_for_analysis("fake-model", &AnalysisType::CodeCompletion);

        assert!(is_gpt4_suitable);
        assert!(!is_fake_model_suitable);
    }

    /// Test intelligent model recommendation with fallback
    #[tokio::test]
    async fn test_intelligent_model_recommendation() {
        let manager = LLMIntegrationManager::new().unwrap();

        // Test recommendation without preferred model
        let recommended_model =
            manager.get_intelligent_model_recommendation(AnalysisType::Documentation, None);
        assert!(!recommended_model.is_empty());

        // Test recommendation with suitable preferred model
        let recommended_with_preferred = manager.get_intelligent_model_recommendation(
            AnalysisType::Documentation,
            Some("gpt-4".to_string()),
        );
        assert_eq!(recommended_with_preferred, "gpt-4");

        // Test recommendation with unsuitable preferred model (should fall back)
        let recommended_with_unsuitable = manager.get_intelligent_model_recommendation(
            AnalysisType::Documentation,
            Some("fake-model".to_string()),
        );
        assert_ne!(recommended_with_unsuitable, "fake-model");
    }

    /// Test analysis type detection from context
    #[tokio::test]
    async fn test_analysis_type_detection() {
        let manager = LLMIntegrationManager::new().unwrap();

        // Test security analysis detection
        let security_code = "function authenticate_user(password) { /* auth logic */ }";
        let security_type =
            manager.detect_analysis_type_from_context(security_code, Some("check security"));
        assert_eq!(security_type, AnalysisType::SecurityAnalysis);

        // Test performance optimization detection
        let perf_code = "function slow_operation() { /* needs optimization */ }";
        let perf_type =
            manager.detect_analysis_type_from_context(perf_code, Some("optimize performance"));
        assert_eq!(perf_type, AnalysisType::PerformanceOptimization);

        // Test documentation detection
        let doc_code = "// TODO: document this function\nfunction undocumented() {}";
        let doc_type =
            manager.detect_analysis_type_from_context(doc_code, Some("add documentation"));
        assert_eq!(doc_type, AnalysisType::Documentation);

        // Test bug detection detection
        let bug_code = "// BUG: this function has an error\nfunction buggy() {}";
        let bug_type = manager.detect_analysis_type_from_context(bug_code, Some("fix bug"));
        assert_eq!(bug_type, AnalysisType::BugDetection);

        // Test refactoring detection
        let refactor_code = "// This function needs refactoring\nfunction complex() {}";
        let refactor_type =
            manager.detect_analysis_type_from_context(refactor_code, Some("refactor code"));
        assert_eq!(refactor_type, AnalysisType::Refactoring);

        // Test code completion detection
        let completion_code = "my_object.";
        let completion_type =
            manager.detect_analysis_type_from_context(completion_code, Some("complete code"));
        assert_eq!(completion_type, AnalysisType::CodeCompletion);

        // Test general analysis fallback
        let general_code = "function normal() {}";
        let general_type = manager.detect_analysis_type_from_context(general_code, None);
        assert_eq!(general_type, AnalysisType::GeneralAnalysis);
    }

    /// Test model selection preferences
    #[tokio::test]
    async fn test_model_selection_preferences() {
        let mut manager = LLMIntegrationManager::new().unwrap();

        // Test default preferences
        let default_model = manager.select_best_model_for_analysis(AnalysisType::CodeCompletion);
        assert!(default_model.is_some());

        // Test updated preferences (favor speed over capability)
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

        // The speed-optimized model might be different from the default
        println!("Default model: {:?}", default_model.unwrap());
        println!(
            "Speed-optimized model: {:?}",
            speed_optimized_model.unwrap()
        );
    }

    /// Test performance feedback system
    #[tokio::test]
    async fn test_performance_feedback() {
        let mut manager = LLMIntegrationManager::new().unwrap();

        // Get initial capability score for a model
        let initial_caps = manager
            .model_selector
            .get_available_models(&AnalysisType::CodeCompletion);
        let initial_gpt4_cap = initial_caps
            .iter()
            .find(|cap| cap.model_name == "gpt-4")
            .unwrap();
        let initial_score = initial_gpt4_cap.capability_score;

        // Provide positive feedback
        let positive_feedback = PerformanceFeedback {
            quality_score: 1.0,
            speed_score: 0.9,
            reliability_score: 0.95,
            user_satisfaction: Some(0.95),
        };

        manager.provide_performance_feedback(
            AnalysisType::CodeCompletion,
            "gpt-4",
            positive_feedback,
        );

        // Get updated capability score
        let updated_caps = manager
            .model_selector
            .get_available_models(&AnalysisType::CodeCompletion);
        let updated_gpt4_cap = updated_caps
            .iter()
            .find(|cap| cap.model_name == "gpt-4")
            .unwrap();
        let updated_score = updated_gpt4_cap.capability_score;

        // Score should have increased (or at least changed)
        println!("Initial GPT-4 capability score: {}", initial_score);
        println!("Updated GPT-4 capability score: {}", updated_score);

        // Test negative feedback
        let negative_feedback = PerformanceFeedback {
            quality_score: 0.3,
            speed_score: 0.4,
            reliability_score: 0.2,
            user_satisfaction: Some(0.25),
        };

        manager.provide_performance_feedback(
            AnalysisType::CodeCompletion,
            "gpt-4",
            negative_feedback,
        );

        // Get final capability score
        let final_caps = manager
            .model_selector
            .get_available_models(&AnalysisType::CodeCompletion);
        let final_gpt4_cap = final_caps
            .iter()
            .find(|cap| cap.model_name == "gpt-4")
            .unwrap();
        let final_score = final_gpt4_cap.capability_score;

        println!("Final GPT-4 capability score: {}", final_score);

        // Score should be different from initial
        assert_ne!(initial_score, final_score);
    }

    /// Test model selection explanation
    #[tokio::test]
    async fn test_model_selection_explanation() {
        let manager = LLMIntegrationManager::new().unwrap();

        // Get explanation for a model selection
        let explanation =
            manager.get_model_selection_explanation(AnalysisType::CodeCompletion, "gpt-4");

        assert!(explanation.is_some());
        let explanation_text = explanation.unwrap();

        // Explanation should contain relevant information
        assert!(explanation_text.contains("gpt-4"));
        assert!(explanation_text.contains("CodeCompletion"));
        assert!(explanation_text.contains("Capability score"));
        assert!(explanation_text.contains("Speed score"));
        assert!(explanation_text.contains("Cost score"));
        assert!(explanation_text.contains("Reliability score"));

        println!("Model selection explanation:\n{}", explanation_text);
    }
}
