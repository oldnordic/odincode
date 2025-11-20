//! Core ML integration manager

use anyhow::Result;
use std::sync::Arc;
use uuid::Uuid;

use crate::ml_integration::config::MLIntegrationConfig;
use crate::ml_integration::metadata::TrainingData;
use crate::CodeEngine;
use odincode_ltmc::manager::LTMManager;

/// Core ML integration manager
///
/// This is the central coordinator for ML operations,
/// handling basic lifecycle management and delegating
/// to specialized components.
pub struct MLIntegrationCore {
    /// ML integration configuration
    pub config: MLIntegrationConfig,
    /// Reference to core code engine
    pub core_engine: Arc<CodeEngine>,
    /// LTMC manager for persistent learning
    pub ltmc_manager: Arc<LTMManager>,
}

impl MLIntegrationCore {
    /// Create new ML integration core
    pub async fn new(config: MLIntegrationConfig) -> Result<Self> {
        // For now, create placeholder core engine and ltmc manager
        // In practice, these would be injected
        let core_engine = Arc::new(CodeEngine::new()?);
        let ltmc_manager = Arc::new(LTMManager::new());

        Ok(Self {
            config,
            core_engine,
            ltmc_manager,
        })
    }

    /// Get configuration reference
    pub fn config(&self) -> &MLIntegrationConfig {
        &self.config
    }

    /// Get core engine reference
    pub fn core_engine(&self) -> &Arc<CodeEngine> {
        &self.core_engine
    }

    /// Get LTMC manager reference
    pub fn ltmc_manager(&self) -> &Arc<LTMManager> {
        &self.ltmc_manager
    }

    /// Update configuration
    pub fn update_config(&mut self, new_config: MLIntegrationConfig) {
        self.config = new_config;
    }

    /// Get file from core engine
    pub async fn get_file(&self, file_id: &Uuid) -> Result<Option<crate::CodeFile>> {
        self.core_engine.get_file(*file_id).await
    }

    /// Store learning pattern in LTMC
    pub async fn store_learning_pattern(
        &self,
        pattern: odincode_ltmc::LearningPattern,
    ) -> Result<()> {
        self.ltmc_manager.store_pattern(pattern).await.map(|_| ())
    }

    /// Retrieve learning patterns from LTMC
    pub async fn get_learning_patterns(
        &self,
        query: &str,
    ) -> Result<Vec<odincode_ltmc::LearningPattern>> {
        self.ltmc_manager.search_patterns(None, query).await
    }
}
