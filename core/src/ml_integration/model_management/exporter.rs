//! Model Exporter
//!
//! Handles exporting trained models to various formats for sharing and deployment.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

use crate::ml_integration::metadata::{ModelMetadata, ModelType};
use crate::ml_integration::models::TrainedModel;

/// Supported export formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExportFormat {
    /// JSON format for model metadata and parameters
    Json,
    /// Binary format for large models
    Binary,
    /// ONNX format for model interoperability
    Onnx,
    /// Pickle format (Python compatibility)
    Pickle,
    /// Custom OdinCode format
    Odin,
}

/// Export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Export format
    pub format: ExportFormat,
    /// Destination path or URL
    pub destination: String,
    /// Whether to include metadata
    pub include_metadata: bool,
    /// Whether to compress the output
    pub compress: bool,
    /// Whether to validate before export
    pub validate_before_export: bool,
    /// Additional export options
    pub options: HashMap<String, String>,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Odin,
            destination: String::new(),
            include_metadata: true,
            compress: false,
            validate_before_export: true,
            options: HashMap::new(),
        }
    }
}

/// Export result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    /// Export success status
    pub success: bool,
    /// Exported file path
    pub exported_path: Option<String>,
    /// Export warnings
    pub warnings: Vec<String>,
    /// Export errors
    pub errors: Vec<String>,
    /// Export statistics
    pub stats: ExportStats,
}

/// Export statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportStats {
    /// Exported model size in bytes
    pub model_size: u64,
    /// Number of parameters exported
    pub parameter_count: usize,
    /// Export duration in milliseconds
    pub export_duration_ms: u64,
    /// Validation duration in milliseconds
    pub validation_duration_ms: u64,
    /// Compression ratio (if compressed)
    pub compression_ratio: Option<f32>,
}

/// Model Exporter
///
/// Handles exporting trained models to various formats for sharing and deployment.
pub struct ModelExporter {
    /// Supported formats
    supported_formats: Vec<ExportFormat>,
    /// Export cache
    export_cache: HashMap<String, ExportResult>,
}

impl ModelExporter {
    /// Create a new model exporter
    pub fn new() -> Self {
        Self {
            supported_formats: vec![
                ExportFormat::Json,
                ExportFormat::Binary,
                ExportFormat::Onnx,
                ExportFormat::Odin,
            ],
            export_cache: HashMap::new(),
        }
    }

    /// Export a model using the given configuration
    pub async fn export_model(
        &mut self,
        model: &Box<dyn TrainedModel>,
        metadata: &ModelMetadata,
        config: ExportConfig,
    ) -> Result<ExportResult> {
        let start_time = std::time::Instant::now();

        info!("Exporting model to: {}", config.destination);
        debug!("Export format: {:?}", config.format);

        let mut result = ExportResult {
            success: false,
            exported_path: None,
            warnings: Vec::new(),
            errors: Vec::new(),
            stats: ExportStats {
                model_size: 0,
                parameter_count: 0,
                export_duration_ms: 0,
                validation_duration_ms: 0,
                compression_ratio: None,
            },
        };

        // Validate before export if requested
        if config.validate_before_export {
            let validation_start = std::time::Instant::now();
            if let Err(e) = self.validate_model_for_export(model, metadata).await {
                result.warnings.push(format!("Validation warning: {}", e));
            }
            result.stats.validation_duration_ms = validation_start.elapsed().as_millis() as u64;
        }

        // Export based on format
        let export_result = match config.format {
            ExportFormat::Json => self.export_json(model, metadata, &config).await,
            ExportFormat::Binary => self.export_binary(model, metadata, &config).await,
            ExportFormat::Onnx => self.export_onnx(model, metadata, &config).await,
            ExportFormat::Pickle => self.export_pickle(model, metadata, &config).await,
            ExportFormat::Odin => self.export_odin(model, metadata, &config).await,
        };

        match export_result {
            Ok((exported_path, model_size, parameter_count)) => {
                result.success = true;
                result.exported_path = Some(exported_path);
                result.stats.model_size = model_size;
                result.stats.parameter_count = parameter_count;

                info!("Successfully exported model to: {}", config.destination);
            }
            Err(e) => {
                result.errors.push(e.to_string());
                warn!("Failed to export model to {}: {}", config.destination, e);
            }
        }

        // Compress if requested
        if config.compress && result.success {
            if let Err(e) = self.compress_exported_file(&config.destination).await {
                result.warnings.push(format!("Compression warning: {}", e));
            } else {
                // Update compression ratio
                if let Ok(compressed_size) = std::fs::metadata(&config.destination) {
                    result.stats.compression_ratio =
                        Some(compressed_size.len() as f32 / result.stats.model_size as f32);
                }
            }
        }

        result.stats.export_duration_ms = start_time.elapsed().as_millis() as u64;

        // Cache the result
        self.export_cache
            .insert(config.destination.clone(), result.clone());

        Ok(result)
    }

    /// Export model to JSON format
    async fn export_json(
        &self,
        model: &Box<dyn TrainedModel>,
        metadata: &ModelMetadata,
        config: &ExportConfig,
    ) -> Result<(String, u64, usize)> {
        debug!("Exporting model to JSON: {}", config.destination);

        let export_data = serde_json::json!({
            "metadata": metadata,
            "model_type": model.model_type(),
            "export_timestamp": chrono::Utc::now(),
            "format_version": "1.0"
        });

        let content = serde_json::to_string_pretty(&export_data)?;

        std::fs::write(&config.destination, content)
            .with_context(|| format!("Failed to write JSON file: {}", config.destination))?;

        let model_size = std::fs::metadata(&config.destination)?.len() as u64;
        let parameter_count = self.estimate_parameter_count(model);

        Ok((config.destination.clone(), model_size, parameter_count))
    }

    /// Export model to binary format
    async fn export_binary(
        &self,
        model: &Box<dyn TrainedModel>,
        metadata: &ModelMetadata,
        config: &ExportConfig,
    ) -> Result<(String, u64, usize)> {
        debug!("Exporting model to binary: {}", config.destination);

        // For binary format, we serialize metadata and model type
        let export_data = serde_json::json!({
            "metadata": metadata,
            "model_type": model.model_type(),
            "export_timestamp": chrono::Utc::now(),
        });

        let json_content = serde_json::to_vec(&export_data)?;

        std::fs::write(&config.destination, json_content)
            .with_context(|| format!("Failed to write binary file: {}", config.destination))?;

        let model_size = std::fs::metadata(&config.destination)?.len() as u64;
        let parameter_count = self.estimate_parameter_count(model);

        Ok((config.destination.clone(), model_size, parameter_count))
    }

    /// Export model to ONNX format
    async fn export_onnx(
        &self,
        _model: &Box<dyn TrainedModel>,
        _metadata: &ModelMetadata,
        _config: &ExportConfig,
    ) -> Result<(String, u64, usize)> {
        debug!("Exporting model to ONNX");

        // ONNX export would require additional dependencies
        // This is a placeholder implementation
        anyhow::bail!("ONNX export not yet implemented");
    }

    /// Export model to Pickle format
    async fn export_pickle(
        &self,
        _model: &Box<dyn TrainedModel>,
        _metadata: &ModelMetadata,
        _config: &ExportConfig,
    ) -> Result<(String, u64, usize)> {
        debug!("Exporting model to Pickle");

        // Pickle export would require Python integration
        // This is a placeholder implementation
        anyhow::bail!("Pickle export not yet implemented");
    }

    /// Export model to OdinCode format
    async fn export_odin(
        &self,
        model: &Box<dyn TrainedModel>,
        metadata: &ModelMetadata,
        config: &ExportConfig,
    ) -> Result<(String, u64, usize)> {
        debug!("Exporting model to OdinCode format: {}", config.destination);

        let path = Path::new(&config.destination);

        // Create directory if it doesn't exist
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", config.destination))?;

        // Export metadata
        let metadata_path = path.join("metadata.json");
        let metadata_content = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&metadata_path, metadata_content)
            .with_context(|| format!("Failed to write metadata file: {:?}", metadata_path))?;

        // Export model metadata (not the model itself since it's not serializable)
        let model_path = path.join("model_info.json");
        let model_info = serde_json::json!({
            "model_type": model.model_type(),
            "metadata": model.get_metadata()
        });
        let model_data = serde_json::to_vec(&model_info)?;
        std::fs::write(&model_path, model_data)
            .with_context(|| format!("Failed to write model file: {:?}", model_path))?;

        // Calculate total size
        let total_size =
            std::fs::metadata(&metadata_path)?.len() + std::fs::metadata(&model_path)?.len();
        let parameter_count = self.estimate_parameter_count(model);

        Ok((config.destination.clone(), total_size, parameter_count))
    }

    /// Validate model before export
    async fn validate_model_for_export(
        &self,
        model: &Box<dyn TrainedModel>,
        metadata: &ModelMetadata,
    ) -> Result<()> {
        debug!("Validating model for export");

        // Basic validation checks
        if metadata.id.is_nil() {
            anyhow::bail!("Model ID is nil");
        }

        if metadata.version.is_empty() {
            anyhow::bail!("Model version is empty");
        }

        // Model-specific validation would go here
        // This is a basic implementation

        Ok(())
    }

    /// Compress exported file
    async fn compress_exported_file(&self, file_path: &str) -> Result<()> {
        debug!("Compressing exported file: {}", file_path);

        // This would implement compression logic
        // For now, it's a placeholder
        Ok(())
    }

    /// Estimate parameter count from model
    fn estimate_parameter_count(&self, model: &Box<dyn TrainedModel>) -> usize {
        // This is a simplified estimation
        // In practice, this would depend on the model type and architecture
        match model.model_type() {
            ModelType::LinearRegression => 100,
            ModelType::KMeans => 50,
            ModelType::SVM => 200,
            _ => 1000, // Default estimate
        }
    }

    /// Get supported export formats
    pub fn supported_formats(&self) -> &[ExportFormat] {
        &self.supported_formats
    }

    /// Clear export cache
    pub fn clear_cache(&mut self) {
        self.export_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, u64) {
        let cache_size = self.export_cache.len();
        let total_size: u64 = self.export_cache.values().map(|r| r.stats.model_size).sum();
        (cache_size, total_size)
    }
}

impl Default for ModelExporter {
    fn default() -> Self {
        Self::new()
    }
}
