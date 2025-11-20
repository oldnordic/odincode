//! ML Integration Manager - Compatibility Shim
//!
//! This module provides backward compatibility by delegating to the new MLIntegrationFacade.
//! The old massive integration.rs has been replaced with a modular architecture.

use anyhow::Result;
use std::sync::Arc;

// Re-export facade types for backward compatibility
pub use crate::ml_integration::facade::{
    MLIntegrationFacade, MLIntegrationStats as MLIntegrationManagerStats,
};

// Re-export config types
pub use crate::ml_integration::config::MLIntegrationConfig;

// Re-export metadata types
pub use crate::ml_integration::metadata::{
    HyperparameterSearchConfig, HyperparameterSearchType, ModelMetadata, ModelPerformanceMetrics,
    ModelType, PredictionResult, TrainingConfig, TrainingResult,
};

// Re-export model types
pub use crate::ml_integration::models::TrainedModel;

/// ML Integration Manager - Compatibility Wrapper
///
/// This provides backward compatibility with the old MLIntegrationManager API
/// while delegating to the new modular MLIntegrationFacade.
#[derive(Clone)]
pub struct MLIntegrationManager {
    facade: Arc<MLIntegrationFacade>,
}

impl MLIntegrationManager {
    /// Create new ML integration manager
    pub async fn new(
        _engine: Arc<crate::CodeEngine>,
        _ltmc_manager: Arc<odincode_ltmc::manager::LTMManager>,
        config: MLIntegrationConfig,
    ) -> Result<Self> {
        let facade = Arc::new(MLIntegrationFacade::new(config).await?);
        Ok(Self { facade })
    }

    /// Train a model
    pub async fn train_model(
        &self,
        model_type: ModelType,
        training_config: TrainingConfig,
    ) -> Result<TrainingResult> {
        self.facade.train_model(model_type, training_config).await
    }

    /// Make predictions
    pub async fn predict(&self, model_id: &str, input_data: &[f64]) -> Result<PredictionResult> {
        self.facade.predict(model_id, input_data).await
    }

    /// Analyze code quality
    pub async fn analyze_quality(&self, code: &str, language: &str) -> Result<Vec<String>> {
        self.facade.analyze_quality(code, language).await
    }

    /// Generate code using LLM
    pub async fn generate_code(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        self.facade.generate_code(prompt, context).await
    }

    /// Import model
    pub async fn import_model(&self, source: &str) -> Result<String> {
        self.facade.import_model(source).await
    }

    /// Export model
    pub async fn export_model(&self, model_id: &str, destination: &str) -> Result<()> {
        self.facade.export_model(model_id, destination).await
    }

    /// List models
    pub async fn list_models(
        &self,
    ) -> Result<Vec<crate::ml_integration::model_management::ModelRegistryEntry>> {
        self.facade.list_models().await
    }

    /// Get statistics
    pub async fn get_stats(&self) -> Result<MLIntegrationManagerStats> {
        self.facade.get_stats().await
    }

    /// Get configuration
    pub fn get_config(&self) -> &MLIntegrationConfig {
        self.facade.get_config()
    }

    // ===== ADDITIONAL COMPATIBILITY METHODS =====
    // These methods were requested but missing from the original implementation

    /// Set LLM integration (compatibility method)
    pub async fn set_llm_integration<T>(&mut self, _llm: T) {
        // Note: This is a compatibility shim - the new architecture handles LLM integration differently
        // For now, we'll just log this operation
        tracing::debug!("set_llm_integration called (compatibility shim)");
    }

    /// Get LLM integration (compatibility method)
    pub async fn get_llm_integration(&self) -> Option<crate::ml_integration::llm::LLMIntegration> {
        // Note: This is a compatibility shim - the new architecture handles LLM integration differently
        tracing::debug!("get_llm_integration called (compatibility shim)");
        None
    }

    /// Analyze with ML (compatibility method)
    pub async fn analyze_with_ml(&self, code: &str) -> Result<Vec<crate::CodeSuggestion>> {
        self.facade.analyze_with_ml(code).await
    }

    /// Analyze with LLM (compatibility method)
    pub async fn analyze_with_llm(&self, code: &str) -> Result<Vec<crate::CodeSuggestion>> {
        self.facade.analyze_with_llm(code).await
    }

    /// Train models (compatibility method)
    pub async fn train_models(
        &mut self,
        data: crate::ml_integration::metadata::TrainingData,
    ) -> Result<()> {
        // Since facade methods require &mut self, we need to use Arc::get_mut
        // This is a limitation of the compatibility shim
        if let Some(facade) = Arc::get_mut(&mut self.facade) {
            facade.train_models(data).await
        } else {
            anyhow::bail!("Cannot modify MLIntegrationManager - multiple references exist")
        }
    }

    /// Load models (compatibility method)
    pub async fn load_models(&mut self) -> Result<()> {
        if let Some(facade) = Arc::get_mut(&mut self.facade) {
            facade.load_models().await
        } else {
            anyhow::bail!("Cannot modify MLIntegrationManager - multiple references exist")
        }
    }

    /// Save models (compatibility method)
    pub async fn save_models(&self) -> Result<()> {
        self.facade.save_models().await
    }

    /// Get model registry (compatibility method)
    pub fn get_model_registry(&self) -> &crate::ml_integration::model_management::ModelRegistry {
        self.facade.get_model_registry()
    }

    /// Get quality analyzer (compatibility method)
    pub fn get_quality_analyzer(&self) -> &crate::ml_integration::analysis::QualityAnalyzer {
        self.facade.get_quality_analyzer()
    }
}

/// Default ML integration manager for backward compatibility
impl Default for MLIntegrationManager {
    fn default() -> Self {
        let config = MLIntegrationConfig::default();
        let facade = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { MLIntegrationFacade::new(config).await.unwrap() })
        });
        Self {
            facade: Arc::new(facade),
        }
    }
}
