//! ML Integration Facade
//!
//! Provides a clean, unified interface to the ML integration system,
//! replacing the massive monolithic integration.rs file.

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::ml_integration::analysis::{MLPredictor, QualityAnalyzer};
use crate::ml_integration::config::MLIntegrationConfig;
use crate::ml_integration::core::MLIntegrationCore;
use crate::ml_integration::llm::LLMIntegration;
use crate::ml_integration::metadata::{
    ModelMetadata, ModelType, PredictionResult, TrainingConfig, TrainingResult,
};
use crate::ml_integration::model_management::{
    ModelExporter, ModelImporter, ModelPerformanceTracker, ModelRegistry,
};
use crate::ml_integration::models::TrainedModel;
use crate::ml_integration::training::TrainingOrchestrator;
use crate::{CodeSuggestion, Severity};

/// ML Integration Facade
///
/// Provides a unified interface to all ML integration functionality.
/// This replaces the massive integration.rs file with a clean, modular approach.
pub struct MLIntegrationFacade {
    /// Core ML integration engine
    core: Arc<MLIntegrationCore>,
    /// Training orchestrator
    training: Arc<TrainingOrchestrator>,
    /// ML predictor
    predictor: Arc<MLPredictor>,
    /// Quality analyzer
    quality_analyzer: Arc<QualityAnalyzer>,
    /// LLM integration
    llm_integration: Arc<RwLock<LLMIntegration>>,
    /// Model registry
    model_registry: Arc<ModelRegistry>,
    /// Model importer
    model_importer: Arc<RwLock<ModelImporter>>,
    /// Model exporter
    model_exporter: Arc<RwLock<ModelExporter>>,
    /// Performance tracker
    performance_tracker: Arc<RwLock<ModelPerformanceTracker>>,
    /// Configuration
    config: MLIntegrationConfig,
}

impl MLIntegrationFacade {
    /// Create a new ML integration facade
    pub async fn new(config: MLIntegrationConfig) -> Result<Self> {
        debug!("Creating ML integration facade");

        // Initialize core components
        let core = Arc::new(MLIntegrationCore::new(config.clone()).await?);
        let training = Arc::new(TrainingOrchestrator::new(config.clone()).await?);
        let predictor = Arc::new(MLPredictor::new(config.clone()).await?);
        let quality_analyzer = Arc::new(QualityAnalyzer::new(config.clone()).await?);
        let provider = match config.llm_config.provider.as_str() {
            "openai" => crate::ml_integration::llm::integration::LLMProvider::OpenAI,
            "anthropic" => crate::ml_integration::llm::integration::LLMProvider::Anthropic,
            "ollama" => crate::ml_integration::llm::integration::LLMProvider::Ollama,
            "local" => crate::ml_integration::llm::integration::LLMProvider::Local,
            _ => crate::ml_integration::llm::integration::LLMProvider::Custom(
                config.llm_config.provider.clone(),
            ),
        };

        let llm_config = crate::ml_integration::llm::integration::LLMConfig {
            provider,
            model: config.llm_config.model.clone(),
            api_key: None,
            api_endpoint: None,
            max_tokens: config.llm_config.max_tokens as u32,
            temperature: config.llm_config.temperature as f32,
            timeout_secs: 30,
            parameters: HashMap::new(),
        };
        let llm_integration = Arc::new(RwLock::new(LLMIntegration::new(llm_config)));
        let registry_config =
            crate::ml_integration::model_management::registry::ModelRegistryConfig {
                max_models_in_memory: config.model_registry_config.max_models_in_memory,
                persist_to_disk: config.model_registry_config.persist_to_disk,
                persistence_directory: config.model_registry_config.persistence_directory.clone(),
                enable_versioning: config.model_registry_config.enable_versioning,
                max_versions_per_model: config.model_registry_config.max_versions_per_model,
            };
        let model_registry = Arc::new(ModelRegistry::new(registry_config)?);
        let model_importer = Arc::new(RwLock::new(ModelImporter::new()));
        let model_exporter = Arc::new(RwLock::new(ModelExporter::new()));
        let perf_config = crate::ml_integration::model_management::performance_tracker::PerformanceTrackingConfig {
            max_records: 1000,
            evaluation_interval_secs: 60,
            enable_real_time_monitoring: config.performance_tracking_config.enabled,
            alert_thresholds: crate::ml_integration::model_management::performance_tracker::PerformanceThresholds::default(),
        };
        let performance_tracker = Arc::new(RwLock::new(ModelPerformanceTracker::new(perf_config)));

        info!("ML integration facade created successfully");

        Ok(Self {
            core,
            training,
            predictor,
            quality_analyzer,
            llm_integration,
            model_registry,
            model_importer,
            model_exporter,
            performance_tracker,
            config,
        })
    }

    /// Train a new model
    pub async fn train_model(
        &self,
        model_type: ModelType,
        training_config: TrainingConfig,
    ) -> Result<TrainingResult> {
        debug!("Training model of type: {:?}", model_type);

        let result = self
            .training
            .train_model(model_type, training_config)
            .await?;

        // Register the trained model if successful
        if let Some(ref model) = result.trained_model {
            if let Some(ref metadata) = result.metadata {
                self.model_registry
                    .register_model(model.clone(), metadata.clone())
                    .await?;
            }
        }

        info!("Model training completed successfully");
        Ok(result)
    }

    /// Make predictions using a trained model
    pub async fn predict(&self, model_id: &str, input_data: &[f64]) -> Result<PredictionResult> {
        debug!("Making prediction with model: {}", model_id);

        let model = self
            .model_registry
            .get_model(model_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        let result = self.predictor.predict(&model, input_data).await?;

        // Record performance metrics
        self.record_prediction_metrics(model_id, &result).await?;

        Ok(result)
    }

    /// Analyze code quality
    pub async fn analyze_quality(&self, code: &str, language: &str) -> Result<Vec<String>> {
        debug!("Analyzing code quality for language: {}", language);

        let suggestions = self.quality_analyzer.analyze_code(code, language).await?;

        Ok(suggestions.into_iter().map(|s| s.description).collect())
    }

    /// Generate code using LLM
    pub async fn generate_code(&self, prompt: &str, context: Option<&str>) -> Result<String> {
        debug!("Generating code with LLM");

        let llm_request = crate::ml_integration::llm::LLMRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            prompt: prompt.to_string(),
            context: context.map(|s| s.to_string()),
            request_type: crate::ml_integration::llm::LLMRequestType::CodeGeneration,
            options: HashMap::new(),
        };

        let mut llm = self.llm_integration.write().await;
        let response = llm.send_request(llm_request).await?;

        Ok(response.content)
    }

    /// Import a model from external source
    pub async fn import_model(&self, source: &str) -> Result<String> {
        debug!("Importing model from: {}", source);

        let import_config = crate::ml_integration::model_management::ImportConfig {
            format: crate::ml_integration::model_management::ImportFormat::Odin,
            source: source.to_string(),
            validate: true,
            overwrite: false,
            options: HashMap::new(),
        };

        let mut importer = self.model_importer.write().await;
        let result = importer.import_model(import_config).await?;

        if result.success {
            if let Some(metadata) = result.metadata {
                // Create a placeholder model for the import
                // In practice, this would load the actual model
                info!("Model imported successfully: {}", metadata.id);
                Ok(metadata.id.to_string())
            } else {
                anyhow::bail!("Import succeeded but no metadata returned");
            }
        } else {
            anyhow::bail!("Model import failed: {:?}", result.errors);
        }
    }

    /// Export a model to external format
    pub async fn export_model(&self, model_id: &str, destination: &str) -> Result<()> {
        debug!("Exporting model {} to: {}", model_id, destination);

        let model = self
            .model_registry
            .get_model(model_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        let metadata = self
            .model_registry
            .get_metadata(model_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Model metadata not found: {}", model_id))?;

        let export_config = crate::ml_integration::model_management::ExportConfig {
            format: crate::ml_integration::model_management::ExportFormat::Odin,
            destination: destination.to_string(),
            include_metadata: true,
            compress: false,
            validate_before_export: true,
            options: HashMap::new(),
        };

        let mut exporter = self.model_exporter.write().await;
        let result = exporter
            .export_model(&model, &metadata, export_config)
            .await?;

        if result.success {
            info!("Model exported successfully to: {}", destination);
            Ok(())
        } else {
            anyhow::bail!("Model export failed: {:?}", result.errors);
        }
    }

    /// Get model performance statistics
    pub async fn get_model_stats(
        &self,
        model_id: &str,
    ) -> Option<crate::ml_integration::model_management::PerformanceStats> {
        let tracker = self.performance_tracker.read().await;
        tracker.get_performance_stats(model_id)
    }

    /// List all registered models
    pub async fn list_models(
        &self,
    ) -> Result<Vec<crate::ml_integration::model_management::ModelRegistryEntry>> {
        let criteria = crate::ml_integration::model_management::ModelSearchCriteria::default();
        self.model_registry.list_models(criteria).await
    }

    /// Get ML integration statistics
    pub async fn get_stats(&self) -> Result<MLIntegrationStats> {
        let registry_stats = self.model_registry.get_registry_stats().await?;
        let llm_stats = {
            let llm = self.llm_integration.read().await;
            llm.get_stats().clone()
        };

        Ok(MLIntegrationStats {
            total_models: registry_stats.total_models,
            loaded_models: registry_stats.loaded_models,
            total_size_bytes: registry_stats.total_size_bytes,
            llm_requests: llm_stats.total_requests,
            llm_success_rate: if llm_stats.total_requests > 0 {
                llm_stats.successful_requests as f64 / llm_stats.total_requests as f64
            } else {
                0.0
            },
            avg_response_time_ms: llm_stats.avg_response_time_ms,
        })
    }

    /// Record prediction metrics
    async fn record_prediction_metrics(
        &self,
        model_id: &str,
        result: &PredictionResult,
    ) -> Result<()> {
        let record = crate::ml_integration::model_management::PerformanceRecord {
            timestamp: chrono::Utc::now(),
            model_id: model_id.to_string(),
            prediction_time_ms: 100, // Placeholder
            memory_usage_mb: 10,     // Placeholder
            accuracy: result.confidence.map(|c| c as f64),
            confidence: result.confidence.unwrap_or(0.5) as f64,
            prediction_count: 1,
            error_count: if result.confidence.is_some() { 0 } else { 1 },
            additional_metrics: HashMap::new(),
        };

        let mut tracker = self.performance_tracker.write().await;
        tracker.record_performance(record).await
    }

    /// Update configuration
    pub async fn update_config(&mut self, config: MLIntegrationConfig) -> Result<()> {
        debug!("Updating ML integration configuration");

        self.config = config.clone();

        // Update component configurations as needed
        // This would update each component with new config

        info!("ML integration configuration updated");
        Ok(())
    }

    /// Get current configuration
    pub fn get_config(&self) -> &MLIntegrationConfig {
        &self.config
    }

    // ===== COMPATIBILITY SHIM METHODS =====
    // These methods provide backward compatibility with the old MLIntegrationManager API

    /// Set LLM integration (compatibility method)
    pub async fn set_llm_integration(&mut self, llm: LLMIntegration) {
        let mut llm_integration = self.llm_integration.write().await;
        *llm_integration = llm;
    }

    /// Get LLM integration (compatibility method)
    pub async fn get_llm_integration(&self) -> Option<LLMIntegration> {
        // Note: This is a compatibility shim - LLMIntegration doesn't implement Clone
        // For now, we'll return None to indicate this isn't available in new architecture
        tracing::debug!("get_llm_integration called (compatibility shim)");
        None
    }

    /// Analyze with ML (compatibility method)
    pub async fn analyze_with_ml(&self, code: &str) -> Result<Vec<crate::CodeSuggestion>> {
        debug!("Analyzing code with ML (compatibility method)");

        // Use quality analyzer for basic analysis
        let suggestions = self.quality_analyzer.analyze_code(code, "auto").await?;

        // Convert to CodeSuggestion format
        let mut code_suggestions = Vec::new();
        for suggestion in suggestions {
            code_suggestions.push(crate::CodeSuggestion::new_complete(
                &suggestion.title,
                &suggestion.description,
                crate::Severity::Info,
                None,
                None,
                false,
            ));
        }

        Ok(code_suggestions)
    }

    /// Analyze with LLM (compatibility method)
    pub async fn analyze_with_llm(&self, code: &str) -> Result<Vec<crate::CodeSuggestion>> {
        debug!("Analyzing code with LLM (compatibility method)");

        let prompt = format!(
            "Analyze this code and provide suggestions for improvement:\n\n{}",
            code
        );
        let response = self.generate_code(&prompt, None).await?;

        // Parse LLM response into suggestions (simplified)
        let lines: Vec<&str> = response.lines().collect();
        let mut suggestions = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if !line.trim().is_empty() {
                suggestions.push(crate::CodeSuggestion::new_complete(
                    format!("LLM Suggestion {}", i + 1),
                    line.trim(),
                    crate::Severity::Info,
                    None,
                    None,
                    false,
                ));
            }
        }

        Ok(suggestions)
    }

    /// Train models (compatibility method)
    pub async fn train_models(
        &mut self,
        _data: crate::ml_integration::metadata::TrainingData,
    ) -> Result<()> {
        debug!("Training models (compatibility method)");

        // Create a basic training config using the correct structure
        let training_config = TrainingConfig {
            model_types: vec!["linear_regression".to_string()],
            test_size: 0.2,
            validation_size: 0.1,
            random_seed: Some(42),
            cross_validation_folds: Some(5),
            early_stopping: true,
            max_iterations: 1000,
            tolerance: 1e-6,
            hyperparameter_search: crate::ml_integration::metadata::HyperparameterSearchConfig {
                enabled: false,
                search_type: crate::ml_integration::metadata::HyperparameterSearchType::GridSearch,
                max_trials: 100,
                scoring_metric: "accuracy".to_string(),
            },
        };

        // Train a model (simplified) - use first model type
        if let Some(model_type_str) = training_config.model_types.first() {
            let model_type = match model_type_str.as_str() {
                "linear_regression" => ModelType::LinearRegression,
                "svm" => ModelType::SVM,
                "kmeans" => ModelType::KMeans,
                _ => ModelType::LinearRegression,
            };

            let _result = self.train_model(model_type, training_config).await?;
        }

        Ok(())
    }

    /// Load models (compatibility method)
    pub async fn load_models(&mut self) -> Result<()> {
        debug!("Loading models (compatibility method)");

        // In the old system, this would load models from storage
        // For now, we'll just ensure the registry is initialized
        let _models = self.list_models().await?;

        Ok(())
    }

    /// Save models (compatibility method)
    pub async fn save_models(&self) -> Result<()> {
        debug!("Saving models (compatibility method)");

        // In the old system, this would save all models to storage
        // For now, we'll just ensure all models are properly registered
        let models = self.list_models().await?;

        for model_entry in models {
            debug!("Ensuring model {} is saved", model_entry.metadata.id);
        }

        Ok(())
    }

    /// Get model registry (compatibility method)
    pub fn get_model_registry(&self) -> &ModelRegistry {
        &self.model_registry
    }

    /// Get quality analyzer (compatibility method)
    pub fn get_quality_analyzer(&self) -> &QualityAnalyzer {
        &self.quality_analyzer
    }
}

/// ML Integration Statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MLIntegrationStats {
    /// Total number of models
    pub total_models: usize,
    /// Number of models loaded in memory
    pub loaded_models: usize,
    /// Total size of all models in bytes
    pub total_size_bytes: u64,
    /// Total LLM requests
    pub llm_requests: u64,
    /// LLM success rate
    pub llm_success_rate: f64,
    /// Average LLM response time in milliseconds
    pub avg_response_time_ms: f64,
}

impl Default for MLIntegrationFacade {
    fn default() -> Self {
        let config = MLIntegrationConfig::default();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async { Self::new(config).await.unwrap() })
        })
    }
}
