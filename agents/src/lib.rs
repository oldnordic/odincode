//! OdinCode Agents Module
//!
//! The agents module provides specialized AI agents for different coding tasks
//! within the OdinCode system. These agents leverage the core engine and LTMC
//! system to provide intelligent code assistance.

pub mod code_generator;
pub mod code_understanding;
pub mod coordinator;
pub mod documenter;
pub mod llm_integration;
pub mod ltmc_integration;
pub mod models;
pub mod refactorer;
pub mod test_generator;
pub mod vulnerability_scanner;

pub use code_generator::*;
pub use code_understanding::*;
pub use coordinator::*;
pub use documenter::*;
pub use llm_integration::*;
pub use ltmc_integration::*;
pub use models::*;
pub use refactorer::*;
pub use test_generator::*;
pub use vulnerability_scanner::*;

#[cfg(test)]
mod tests {
    use super::*;
    use odincode_core::CodeEngine;
    use odincode_ltmc::LTMManager;

    #[tokio::test]
    async fn test_agent_coordinator_creation() {
        let core_engine = CodeEngine::new();
        let ltmc_manager = LTMManager::new();
        let llm_manager = odincode_core::llm_integration::LLMIntegrationManager::new().unwrap();
        let ltmc_integration = LTMCIntegration::new(
            std::sync::Arc::new(ltmc_manager.clone()),
            std::sync::Arc::new(core_engine.clone()),
            std::sync::Arc::new(llm_manager),
        );
        let coordinator = AgentCoordinator::new(
            std::sync::Arc::new(core_engine),
            std::sync::Arc::new(ltmc_manager),
            std::sync::Arc::new(ltmc_integration),
        );

        assert_eq!(coordinator.agents.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let core_engine = CodeEngine::new();
        let ltmc_manager = LTMManager::new();
        let llm_manager = odincode_core::llm_integration::LLMIntegrationManager::new().unwrap();
        let ltmc_integration = LTMCIntegration::new(
            std::sync::Arc::new(ltmc_manager.clone()),
            std::sync::Arc::new(core_engine.clone()),
            std::sync::Arc::new(llm_manager),
        );
        let coordinator = AgentCoordinator::new(
            std::sync::Arc::new(core_engine),
            std::sync::Arc::new(ltmc_manager),
            std::sync::Arc::new(ltmc_integration),
        );

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

        let agent = coordinator.get_agent(agent_id).await.unwrap();
        assert!(agent.is_some());
        assert_eq!(agent.unwrap().name, "Test Generator");
    }
}
