//! OdinCode Tools Module
//!
//! The tools module provides various utility functions and integrations
//! for the OdinCode system, including integration with development tools
//! and external services.

pub mod linters;
pub mod manager;
pub mod mcp;
pub mod models;
pub mod multi_edit;
pub mod tool_models;

pub use linters::*;
pub use manager::*;
pub use mcp::*;
pub use models::*;
pub use multi_edit::*;
pub use tool_models::*;

#[cfg(test)]
mod tests {
    use super::*;
    use odincode_agents::AgentCoordinator;
    use odincode_core::CodeEngine;
    use odincode_ltmc::LTMManager;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_tool_manager_creation() {
        let core_engine = CodeEngine::new();
        let ltmc_manager = LTMManager::new();
        let llm_manager = odincode_core::llm_integration::LLMIntegrationManager::new().unwrap();
        let ltmc_integration = odincode_agents::ltmc_integration::LTMCIntegration::new(
            Arc::new(ltmc_manager.clone()),
            Arc::new(core_engine.clone()),
            Arc::new(llm_manager),
        );
        let agent_coordinator = AgentCoordinator::new(
            Arc::new(core_engine.clone()),
            Arc::new(ltmc_manager.clone()),
            Arc::new(ltmc_integration),
        );
        let tool_manager = ToolManager::new(core_engine, ltmc_manager, agent_coordinator);

        assert_eq!(tool_manager.tools.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_tool_registration() {
        let core_engine = CodeEngine::new();
        let ltmc_manager = LTMManager::new();
        let llm_manager = odincode_core::llm_integration::LLMIntegrationManager::new().unwrap();
        let ltmc_integration = odincode_agents::ltmc_integration::LTMCIntegration::new(
            Arc::new(ltmc_manager.clone()),
            Arc::new(core_engine.clone()),
            Arc::new(llm_manager),
        );
        let agent_coordinator = AgentCoordinator::new(
            Arc::new(core_engine.clone()),
            Arc::new(ltmc_manager.clone()),
            Arc::new(ltmc_integration),
        );
        let tool_manager = ToolManager::new(core_engine, ltmc_manager, agent_coordinator);

        let mut config = HashMap::new();
        config.insert("path".to_string(), "/usr/bin/rustc".to_string());

        let tool_id = tool_manager
            .register_tool(
                "Rust Compiler".to_string(),
                "The Rust compiler tool".to_string(),
                ToolType::BuildSystem,
                config,
            )
            .await
            .unwrap();

        let tool = tool_manager.get_tool(tool_id).await.unwrap();
        assert!(tool.is_some());
        assert_eq!(tool.unwrap().name, "Rust Compiler");
    }
}
