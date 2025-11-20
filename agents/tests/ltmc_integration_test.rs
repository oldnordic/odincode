//! LTMC Integration Tests
//!
//! This module contains comprehensive tests for the LTMC-Agent integration system.

use anyhow::Result;
use odincode_agents::{
    ltmc_integration::{
        AgentFeedback, EventPriority, FeedbackMetrics, FeedbackProcessorConfig, FeedbackType,
        LearningEvent, LearningEventType, ProcessorType, RealTimeMetrics,
    },
    AgentCoordinator, AgentExecutionResult, AgentType, LTMCIntegration, LearningRequest,
};
use odincode_core::llm_integration::LLMIntegrationManager;
use odincode_core::{CodeEngine, CodeFile, CodeSuggestion, SuggestionType};
use odincode_ltmc::{LTMManager, LearningPattern, PatternType};
use std::collections::HashMap;
use tokio;
use uuid::Uuid;

#[tokio::test]
async fn test_ltmc_integration_creation() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    assert_eq!(
        integration
            .session_manager
            .learning_sessions
            .read()
            .await
            .len(),
        0
    );
    assert_eq!(integration.ltmc_manager.pattern_cache.read().await.len(), 0);
    Ok(())
}

#[tokio::test]
async fn test_start_learning_session() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let session_id = integration
        .start_learning_session(
            agent_id,
            AgentType::CodeGenerator,
            "Test context".to_string(),
        )
        .await
        .unwrap();

    let sessions = integration.session_manager.learning_sessions.read().await;
    assert!(sessions.contains_key(&session_id));

    let session = sessions.get(&session_id).unwrap();
    assert_eq!(session.agent_id, agent_id);
    assert_eq!(session.agent_type, AgentType::CodeGenerator);
    assert_eq!(
        session.status,
        odincode_agents::ltmc_integration::LearningSessionStatus::Active
    );
    Ok(())
}

#[tokio::test]
async fn test_process_learning_request() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let request = LearningRequest {
        agent_id: Uuid::new_v4(),
        agent_type: AgentType::CodeGenerator,
        context: "Test context".to_string(),
        file_id: None,
        query: "code generation".to_string(),
        pattern_types: vec![PatternType::CodePattern],
    };

    let response = integration.process_learning_request(request).await.unwrap();

    assert!(!response.request_id.is_nil());
    assert_eq!(response.confidence, 0.0); // No patterns initially
    assert!(response.session_id.is_some());
    Ok(())
}

#[tokio::test]
async fn test_store_agent_execution() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager.clone(), core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let execution_result = AgentExecutionResult {
        description: "Test execution".to_string(),
        success: true,
        suggestions: vec![],
        execution_time_ms: 100,
        confidence: 0.8,
    };

    let pattern_id = integration
        .store_agent_execution(agent_id, AgentType::CodeGenerator, None, &execution_result)
        .await
        .unwrap();

    assert!(!pattern_id.is_nil());

    // Verify pattern was stored
    let pattern = ltmc_manager.get_pattern(pattern_id).await.unwrap();
    assert!(pattern.is_some());
    assert_eq!(pattern.unwrap().pattern_type, PatternType::UserInteraction);
    Ok(())
}

#[tokio::test]
async fn test_get_learning_insights() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    // Store some execution patterns first
    let agent_id = Uuid::new_v4();
    let execution_result = AgentExecutionResult {
        description: "Test execution".to_string(),
        success: true,
        suggestions: vec![],
        execution_time_ms: 100,
        confidence: 0.8,
    };

    integration
        .store_agent_execution(agent_id, AgentType::CodeGenerator, None, &execution_result)
        .await
        .unwrap();

    let insights = integration
        .get_learning_insights(AgentType::CodeGenerator)
        .await
        .unwrap();

    assert_eq!(insights.agent_type, AgentType::CodeGenerator);
    assert!(insights.total_patterns >= 1);
    assert!(insights.success_rate >= 0.0);
    assert!(insights.average_confidence >= 0.0);
    assert!(!insights.insights.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_complete_learning_session() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let session_id = integration
        .start_learning_session(
            agent_id,
            AgentType::CodeGenerator,
            "Test context".to_string(),
        )
        .await
        .unwrap();

    integration
        .complete_learning_session(session_id, "Test summary".to_string())
        .await
        .unwrap();

    let sessions = integration.session_manager.learning_sessions.read().await;
    let session = sessions.get(&session_id).unwrap();
    assert_eq!(
        session.status,
        odincode_agents::ltmc_integration::LearningSessionStatus::Completed
    );
    Ok(())
}

#[tokio::test]
async fn test_learning_statistics() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let stats = integration.get_learning_statistics().await.unwrap();

    assert_eq!(stats.total_sessions, 0);
    assert_eq!(stats.total_patterns_learned, 0);
    assert_eq!(stats.total_agent_executions, 0);
    assert_eq!(stats.average_confidence, 0.0);
    Ok(())
}

#[tokio::test]
async fn test_agent_coordinator_with_integration() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let ltmc_integration = std::sync::Arc::new(LTMCIntegration::new(
        ltmc_manager.clone(),
        core_engine.clone(),
        llm_manager,
    ));

    let coordinator = AgentCoordinator::new(core_engine, ltmc_manager, ltmc_integration);

    assert_eq!(coordinator.agents.read().await.len(), 0);
    Ok(())
}

#[tokio::test]
async fn test_full_integration_workflow() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let ltmc_integration = std::sync::Arc::new(LTMCIntegration::new(
        ltmc_manager.clone(),
        core_engine.clone(),
        llm_manager,
    ));

    let coordinator = AgentCoordinator::new(core_engine.clone(), ltmc_manager, ltmc_integration);

    // Register an agent
    let agent_id = coordinator
        .register_agent(
            AgentType::CodeGenerator,
            "Test Generator".to_string(),
            "A test code generator agent".to_string(),
            vec!["generation".to_string(), "completion".to_string()],
            0.7,
        )
        .await
        .unwrap();

    // Create a test file and add it to the core engine
    let file_id = core_engine
        .load_file(
            "/test/test.rs".to_string(),
            "fn test() { println!(\"test\"); }".to_string(),
            "Rust".to_string(),
        )
        .await
        .unwrap();

    // Execute agent on file
    let suggestions = coordinator
        .execute_agent_on_file(agent_id, file_id)
        .await
        .unwrap();

    // Verify execution completed
    assert!(suggestions.is_some());

    // Verify learning session was created and completed
    let stats = coordinator
        .ltmc_integration
        .get_learning_statistics()
        .await
        .unwrap();
    assert!(stats.total_sessions >= 1);
    assert!(stats.total_agent_executions >= 1);
    Ok(())
}

#[tokio::test]
async fn test_pattern_cache_functionality() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    // Store a pattern in LTMC
    let pattern = LearningPattern {
        id: Uuid::new_v4(),
        pattern_type: PatternType::CodePattern,
        content: "test pattern".to_string(),
        context: std::collections::HashMap::new(),
        created: chrono::Utc::now(),
        last_accessed: chrono::Utc::now(),
        access_count: 0,
        confidence: 0.8,
    };

    let pattern_id = integration
        .ltmc_manager
        .store_pattern(pattern)
        .await
        .unwrap();

    // Process a learning request that should use the pattern
    let request = LearningRequest {
        agent_id: Uuid::new_v4(),
        agent_type: AgentType::CodeGenerator,
        context: "Test context".to_string(),
        file_id: None,
        query: "test pattern".to_string(),
        pattern_types: vec![PatternType::CodePattern],
    };

    let response = integration.process_learning_request(request).await.unwrap();

    // The pattern should be found and cached
    assert!(response.confidence > 0.0);

    // Verify cache contains the pattern
    let cache = integration.ltmc_manager.pattern_cache.read().await;
    assert!(
        !cache.is_empty(),
        "Pattern cache should not be empty after processing request"
    );
    Ok(())
}

#[tokio::test]
async fn test_learning_session_lifecycle() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();

    // Start session
    let session_id = integration
        .start_learning_session(
            agent_id,
            AgentType::CodeGenerator,
            "Test context".to_string(),
        )
        .await
        .unwrap();

    // Process learning request (should use existing session)
    let request = LearningRequest {
        agent_id,
        agent_type: AgentType::CodeGenerator,
        context: "Test context".to_string(),
        file_id: None,
        query: "test query".to_string(),
        pattern_types: vec![PatternType::CodePattern],
    };

    let response = integration.process_learning_request(request).await.unwrap();
    assert_eq!(response.session_id, Some(session_id));

    // Complete session
    integration
        .complete_learning_session(session_id, "Test summary".to_string())
        .await
        .unwrap();

    // Verify session is completed
    let sessions = integration.session_manager.learning_sessions.read().await;
    let session = sessions.get(&session_id).unwrap();
    assert_eq!(
        session.status,
        odincode_agents::ltmc_integration::LearningSessionStatus::Completed
    );
    Ok(())
}

// ============================================================================
// REAL-TIME FEEDBACK SYSTEM TESTS
// ============================================================================

#[tokio::test]
async fn test_register_feedback_channel() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    // Verify channel was registered
    let channels = integration.feedback_manager.feedback_channels.read().await;
    assert!(channels.contains_key(&agent_id));
    drop(channels);

    // Verify processor was created
    let processors = integration
        .feedback_manager
        .feedback_processors
        .read()
        .await;
    assert!(processors.contains_key(&agent_id));
    let processor = processors.get(&agent_id).unwrap();
    assert_eq!(processor.agent_id, agent_id);
    assert_eq!(processor.processor_type, ProcessorType::RealTime);
    drop(processors);

    // Verify receiver works
    drop(receiver); // This should not panic
    Ok(())
}

#[tokio::test]
async fn test_submit_feedback() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::ExecutionSuccess,
        content: "Test execution completed successfully".to_string(),
        performance_metrics: FeedbackMetrics {
            execution_time_ms: 150,
            memory_usage_bytes: 1024,
            cpu_usage_percent: 0.5,
            success_rate: 1.0,
            confidence_score: 0.9,
            suggestions_generated: 3,
            errors_encountered: 0,
        },
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    // Submit feedback
    integration.submit_feedback(feedback.clone()).await.unwrap();

    // Verify performance metrics were updated
    let metrics = integration.get_real_time_metrics().await.unwrap();
    assert_eq!(metrics.total_feedback_processed, 1);
    assert!(metrics.learning_efficiency > 0.0);
    assert!(metrics.error_rate >= 0.0);
    Ok(())
}

#[tokio::test]
async fn test_subscribe_to_learning_events() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let mut receiver = integration.subscribe_to_learning_events().await.unwrap();

    // Verify subscriber was registered
    let subscribers = integration.metrics_manager.event_subscribers.read().await;
    assert_eq!(subscribers.len(), 1);
    drop(subscribers);

    // Submit feedback to generate an event
    let agent_id = Uuid::new_v4();
    let _channel_receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::ExecutionSuccess,
        content: "Test execution".to_string(),
        performance_metrics: FeedbackMetrics::default(),
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    integration.submit_feedback(feedback).await.unwrap();

    // Check if event was received
    let event =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), receiver.recv()).await;

    assert!(event.is_ok());
    let event = event.unwrap();
    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, LearningEventType::FeedbackReceived);
    Ok(())
}

#[tokio::test]
async fn test_real_time_metrics_tracking() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    // Get initial metrics
    let initial_metrics = integration.get_real_time_metrics().await.unwrap();
    assert_eq!(initial_metrics.total_feedback_processed, 0);
    assert_eq!(initial_metrics.active_sessions, 0);

    // Register agent and submit feedback
    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::ExecutionSuccess,
        content: "Test execution".to_string(),
        performance_metrics: FeedbackMetrics {
            execution_time_ms: 200,
            memory_usage_bytes: 2048,
            cpu_usage_percent: 0.8,
            success_rate: 1.0,
            confidence_score: 0.95,
            suggestions_generated: 5,
            errors_encountered: 0,
        },
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    integration.submit_feedback(feedback).await.unwrap();

    // Check updated metrics
    let updated_metrics = integration.get_real_time_metrics().await.unwrap();
    assert_eq!(updated_metrics.total_feedback_processed, 1);
    assert!(updated_metrics.system_load > 0.0);
    assert!(updated_metrics.learning_efficiency > 0.0);
    assert_eq!(updated_metrics.error_rate, 0.0);
    Ok(())
}

#[tokio::test]
async fn test_feedback_processor_statistics() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    // Get initial processor stats
    let processors = integration
        .feedback_manager
        .feedback_processors
        .read()
        .await;
    let initial_stats = processors.get(&agent_id).unwrap().stats.clone();
    assert_eq!(initial_stats.total_processed, 0);
    drop(processors);

    // Submit feedback
    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::ExecutionSuccess,
        content: "Test execution".to_string(),
        performance_metrics: FeedbackMetrics {
            execution_time_ms: 100,
            memory_usage_bytes: 512,
            cpu_usage_percent: 0.3,
            success_rate: 1.0,
            confidence_score: 0.8,
            suggestions_generated: 2,
            errors_encountered: 0,
        },
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    integration.submit_feedback(feedback).await.unwrap();

    // Check updated processor stats
    let processors = integration
        .feedback_manager
        .feedback_processors
        .read()
        .await;
    let updated_stats = processors.get(&agent_id).unwrap().stats.clone();
    assert_eq!(updated_stats.total_processed, 1);
    assert!(updated_stats.success_rate > 0.0);
    assert!(updated_stats.avg_processing_time_ms > 0.0);
    Ok(())
}

#[tokio::test]
async fn test_event_priority_determination() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    // Test different feedback types and their priorities
    let test_cases = vec![
        (FeedbackType::Error, EventPriority::Critical),
        (FeedbackType::ExecutionFailure, EventPriority::High),
        (FeedbackType::Performance, EventPriority::Low), // Default for fast execution
        (FeedbackType::Learning, EventPriority::Medium),
        (FeedbackType::ExecutionSuccess, EventPriority::Low),
    ];

    for (feedback_type, expected_priority) in test_cases {
        let feedback = AgentFeedback {
            feedback_id: Uuid::new_v4(),
            agent_id,
            agent_type: AgentType::CodeGenerator,
            feedback_type: feedback_type.clone(),
            content: format!("Test {:?} feedback", feedback_type),
            performance_metrics: FeedbackMetrics {
                execution_time_ms: match feedback_type {
                    FeedbackType::Performance => 500, // Medium execution time
                    _ => 100,
                },
                ..Default::default()
            },
            timestamp: chrono::Utc::now(),
            context: HashMap::new(),
        };

        let mut event_receiver = integration.subscribe_to_learning_events().await.unwrap();
        integration.submit_feedback(feedback).await.unwrap();

        // Check event priority
        if let Ok(Some(event)) = tokio::time::timeout(
            tokio::time::Duration::from_millis(100),
            event_receiver.recv(),
        )
        .await
        {
            assert_eq!(
                event.priority, expected_priority,
                "Feedback type {:?} should have priority {:?}",
                feedback_type, expected_priority
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn test_performance_feedback_with_high_execution_time() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    // Create performance feedback with high execution time
    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::Performance,
        content: "Slow execution detected".to_string(),
        performance_metrics: FeedbackMetrics {
            execution_time_ms: 6000, // High execution time
            memory_usage_bytes: 4096,
            cpu_usage_percent: 0.9,
            success_rate: 1.0,
            confidence_score: 0.7,
            suggestions_generated: 1,
            errors_encountered: 0,
        },
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    let mut event_receiver = integration.subscribe_to_learning_events().await.unwrap();
    integration.submit_feedback(feedback).await.unwrap();

    // Check that high execution time generates high priority event
    if let Ok(Some(event)) = tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        event_receiver.recv(),
    )
    .await
    {
        assert_eq!(event.priority, EventPriority::High);
    }
    Ok(())
}

#[tokio::test]
async fn test_learning_pattern_generation_from_feedback() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager.clone(), core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::ExecutionSuccess,
        content: "Successful code generation".to_string(),
        performance_metrics: FeedbackMetrics {
            execution_time_ms: 120,
            memory_usage_bytes: 1024,
            cpu_usage_percent: 0.4,
            success_rate: 1.0,
            confidence_score: 0.85,
            suggestions_generated: 4,
            errors_encountered: 0,
        },
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    integration.submit_feedback(feedback).await.unwrap();

    // Check that pattern was stored in LTMC
    let patterns = ltmc_manager
        .get_patterns_by_type(PatternType::UserInteraction)
        .await
        .unwrap();
    assert!(!patterns.is_empty());

    // Find the pattern we just created
    let agent_pattern = patterns
        .iter()
        .find(|p| p.context.get("agent_id") == Some(&agent_id.to_string()))
        .unwrap();

    assert_eq!(agent_pattern.pattern_type, PatternType::UserInteraction);
    assert!(agent_pattern.content.contains("CodeGenerator"));
    assert!(agent_pattern.content.contains("executed successfully"));
    assert_eq!(agent_pattern.confidence, 0.85);
    Ok(())
}

#[tokio::test]
async fn test_start_real_time_processing() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    // Start real-time processing
    integration
        .feedback_manager
        .start_real_time_processing()
        .await
        .unwrap();

    // Give background tasks time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Register agent and submit feedback
    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::ExecutionSuccess,
        content: "Test execution".to_string(),
        performance_metrics: FeedbackMetrics::default(),
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    integration.submit_feedback(feedback).await.unwrap();

    // Give background tasks time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Verify metrics were updated
    let metrics = integration.get_real_time_metrics().await.unwrap();
    assert!(metrics.total_feedback_processed >= 1);
    Ok(())
}

#[tokio::test]
async fn test_multiple_agent_feedback_processing() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    // Start real-time processing
    integration
        .feedback_manager
        .start_real_time_processing()
        .await
        .unwrap();

    // Register multiple agents
    let agent_ids = vec![Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let mut receivers = Vec::new();

    for &agent_id in &agent_ids {
        let receiver = integration
            .register_feedback_channel(agent_id)
            .await
            .unwrap();
        receivers.push(receiver);
    }

    // Submit feedback from all agents
    for (i, &agent_id) in agent_ids.iter().enumerate() {
        let feedback = AgentFeedback {
            feedback_id: Uuid::new_v4(),
            agent_id,
            agent_type: vec![
                AgentType::CodeGenerator,
                AgentType::TestGenerator,
                AgentType::Refactorer,
            ][i]
                .clone(),
            feedback_type: FeedbackType::ExecutionSuccess,
            content: format!("Agent {} execution", i + 1),
            performance_metrics: FeedbackMetrics {
                execution_time_ms: 100 + (i * 50) as u64,
                success_rate: 1.0,
                confidence_score: 0.8 + (i as f32 * 0.05),
                ..Default::default()
            },
            timestamp: chrono::Utc::now(),
            context: HashMap::new(),
        };

        integration.submit_feedback(feedback).await.unwrap();
    }

    // Give background tasks time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify all feedback was processed
    let metrics = integration.get_real_time_metrics().await.unwrap();
    assert_eq!(metrics.total_feedback_processed, 3);

    // Verify all processors have stats
    let processors = integration
        .feedback_manager
        .feedback_processors
        .read()
        .await;
    for &agent_id in &agent_ids {
        let processor = processors.get(&agent_id).unwrap();
        assert_eq!(processor.stats.total_processed, 1);
    }
    Ok(())
}

#[tokio::test]
async fn test_error_feedback_handling() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    let agent_id = Uuid::new_v4();
    let _receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();

    let feedback = AgentFeedback {
        feedback_id: Uuid::new_v4(),
        agent_id,
        agent_type: AgentType::CodeGenerator,
        feedback_type: FeedbackType::Error,
        content: "Compilation error occurred".to_string(),
        performance_metrics: FeedbackMetrics {
            execution_time_ms: 50,
            success_rate: 0.0,
            confidence_score: 0.1,
            errors_encountered: 1,
            ..Default::default()
        },
        timestamp: chrono::Utc::now(),
        context: HashMap::new(),
    };

    let mut event_receiver = integration.subscribe_to_learning_events().await.unwrap();
    integration.submit_feedback(feedback).await.unwrap();

    // Check that error generates critical priority event
    if let Ok(Some(event)) = tokio::time::timeout(
        tokio::time::Duration::from_millis(100),
        event_receiver.recv(),
    )
    .await
    {
        assert_eq!(event.priority, EventPriority::Critical);
        assert_eq!(event.event_type, LearningEventType::FeedbackReceived);
    }

    // Verify error rate was updated
    let metrics = integration.get_real_time_metrics().await.unwrap();
    assert!(metrics.error_rate > 0.0);
    Ok(())
}

#[tokio::test]
async fn test_full_real_time_feedback_workflow() -> Result<()> {
    let core_engine = std::sync::Arc::new(CodeEngine::new());
    let ltmc_manager = std::sync::Arc::new(LTMManager::new());
    let llm_manager = std::sync::Arc::new(LLMIntegrationManager::new()?);

    let integration = LTMCIntegration::new(ltmc_manager, core_engine, llm_manager);

    // Start real-time processing
    integration
        .feedback_manager
        .start_real_time_processing()
        .await
        .unwrap();

    // Register agent
    let agent_id = Uuid::new_v4();
    let mut feedback_receiver = integration
        .register_feedback_channel(agent_id)
        .await
        .unwrap();
    let mut event_receiver = integration.subscribe_to_learning_events().await.unwrap();

    // Start learning session
    let session_id = integration
        .start_learning_session(
            agent_id,
            AgentType::CodeGenerator,
            "Real-time feedback test".to_string(),
        )
        .await
        .unwrap();

    // Submit multiple feedback events
    let feedback_types = vec![
        FeedbackType::ExecutionSuccess,
        FeedbackType::Learning,
        FeedbackType::Performance,
        FeedbackType::ExecutionSuccess,
    ];

    for feedback_type in feedback_types {
        let feedback = AgentFeedback {
            feedback_id: Uuid::new_v4(),
            agent_id,
            agent_type: AgentType::CodeGenerator,
            feedback_type,
            content: "Workflow feedback".to_string(),
            performance_metrics: FeedbackMetrics {
                execution_time_ms: 150,
                success_rate: 1.0,
                confidence_score: 0.85,
                suggestions_generated: 2,
                ..Default::default()
            },
            timestamp: chrono::Utc::now(),
            context: HashMap::new(),
        };

        integration.submit_feedback(feedback).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Give background tasks time to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify comprehensive metrics
    let metrics = integration.get_real_time_metrics().await.unwrap();
    assert_eq!(metrics.total_feedback_processed, 4);
    assert!(metrics.learning_efficiency > 0.5);
    assert!(metrics.system_load > 0.0);

    // Verify learning session is still active
    let sessions = integration.session_manager.learning_sessions.read().await;
    let session = sessions.get(&session_id).unwrap();
    assert_eq!(
        session.status,
        odincode_agents::ltmc_integration::LearningSessionStatus::Active
    );

    // Complete the session
    drop(sessions);
    integration
        .complete_learning_session(
            session_id,
            "Real-time feedback workflow completed".to_string(),
        )
        .await
        .unwrap();

    // Verify final statistics
    let stats = integration.get_learning_statistics().await.unwrap();
    assert!(stats.total_agent_executions >= 4);
    assert!(stats.total_patterns_learned >= 4);
    Ok(())
}
