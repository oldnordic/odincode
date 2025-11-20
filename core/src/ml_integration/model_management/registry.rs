//! Model Registry
//!
//! Central registry for managing trained models, their metadata, and lifecycle.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::ml_integration::metadata::{ModelMetadata, ModelType};
use crate::ml_integration::models::TrainedModel;

/// Model registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryConfig {
    /// Maximum number of models to keep in memory
    pub max_models_in_memory: usize,
    /// Whether to persist models to disk
    pub persist_to_disk: bool,
    /// Directory for model persistence
    pub persistence_directory: String,
    /// Whether to enable model versioning
    pub enable_versioning: bool,
    /// Maximum number of versions per model
    pub max_versions_per_model: usize,
}

impl Default for ModelRegistryConfig {
    fn default() -> Self {
        Self {
            max_models_in_memory: 100,
            persist_to_disk: true,
            persistence_directory: "./models".to_string(),
            enable_versioning: true,
            max_versions_per_model: 5,
        }
    }
}

/// Model registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRegistryEntry {
    /// Model metadata
    pub metadata: ModelMetadata,
    /// Model reference (stored separately for memory efficiency)
    pub model_loaded: bool,
    /// Model file path (if persisted)
    pub model_path: Option<String>,
    /// Registration timestamp
    pub registered_at: chrono::DateTime<chrono::Utc>,
    /// Last accessed timestamp
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// Access count
    pub access_count: u64,
    /// Model size in bytes
    pub model_size: u64,
    /// Model status
    pub status: ModelStatus,
}

/// Model status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModelStatus {
    /// Model is active and ready for use
    Active,
    /// Model is being trained
    Training,
    /// Model is deprecated
    Deprecated,
    /// Model is being loaded
    Loading,
    /// Model has errors
    Error(String),
}

/// Model search criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSearchCriteria {
    /// Model type filter
    pub model_type: Option<ModelType>,
    /// Version filter
    pub version: Option<String>,
    /// Status filter
    pub status: Option<ModelStatus>,
    /// Created after date
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    /// Created before date
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    /// Description contains text
    pub description_contains: Option<String>,
    /// Minimum accuracy
    pub min_accuracy: Option<f64>,
    /// Tags filter
    pub tags: Vec<String>,
}

impl Default for ModelSearchCriteria {
    fn default() -> Self {
        Self {
            model_type: None,
            version: None,
            status: None,
            created_after: None,
            created_before: None,
            description_contains: None,
            min_accuracy: None,
            tags: Vec::new(),
        }
    }
}

/// Model Registry
///
/// Central registry for managing trained models and their metadata.
pub struct ModelRegistry {
    /// Registry configuration
    config: ModelRegistryConfig,
    /// Model registry entries by model ID
    registry: Arc<RwLock<HashMap<String, ModelRegistryEntry>>>,
    /// Loaded models cache
    loaded_models: Arc<RwLock<HashMap<String, Box<dyn TrainedModel>>>>,
    /// Model versions by base name
    model_versions: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl ModelRegistry {
    /// Create a new model registry
    pub fn new(config: ModelRegistryConfig) -> Result<Self> {
        let registry = Self {
            config,
            registry: Arc::new(RwLock::new(HashMap::new())),
            loaded_models: Arc::new(RwLock::new(HashMap::new())),
            model_versions: Arc::new(RwLock::new(HashMap::new())),
        };

        // Initialize persistence directory if needed
        if registry.config.persist_to_disk {
            std::fs::create_dir_all(&registry.config.persistence_directory).with_context(|| {
                format!(
                    "Failed to create persistence directory: {}",
                    registry.config.persistence_directory
                )
            })?;
        }

        Ok(registry)
    }

    /// Register a new model
    pub async fn register_model(
        &self,
        model: Box<dyn TrainedModel>,
        metadata: ModelMetadata,
    ) -> Result<String> {
        debug!("Registering model: {}", metadata.id);

        let model_id = metadata.id;
        let model_size = self.estimate_model_size(model.as_ref());

        // Create registry entry
        let entry = ModelRegistryEntry {
            metadata: metadata.clone(),
            model_loaded: true,
            model_path: None,
            registered_at: chrono::Utc::now(),
            last_accessed: chrono::Utc::now(),
            access_count: 0,
            model_size,
            status: ModelStatus::Active,
        };

        // Add to registry
        {
            let mut registry = self.registry.write().await;
            registry.insert(model_id.to_string(), entry);
        }

        // Add to loaded models cache
        {
            let mut loaded_models = self.loaded_models.write().await;
            loaded_models.insert(model_id.to_string(), model);
        }

        // Update version tracking
        if self.config.enable_versioning {
            self.update_version_tracking(&model_id.to_string()).await?;
        }

        // Persist if enabled
        if self.config.persist_to_disk {
            self.persist_model(&model_id.to_string()).await?;
        }

        info!("Successfully registered model: {}", model_id);
        Ok(model_id.to_string())
    }

    /// Get a model by ID
    pub async fn get_model(&self, model_id: &str) -> Result<Option<Box<dyn TrainedModel>>> {
        debug!("Getting model: {}", model_id);

        // Check if model is loaded
        {
            let loaded_models = self.loaded_models.read().await;
            if let Some(model) = loaded_models.get(model_id) {
                // Update access statistics
                self.update_access_stats(model_id).await;
                return Ok(Some(model.clone()));
            }
        }

        // Try to load from disk if not in memory
        if self.config.persist_to_disk {
            if let Ok(model) = self.load_model_from_disk(model_id).await {
                // Cache the loaded model
                {
                    let mut loaded_models = self.loaded_models.write().await;
                    loaded_models.insert(model_id.to_string(), model.clone());
                }

                // Update access statistics
                self.update_access_stats(model_id).await;
                return Ok(Some(model));
            }
        }

        warn!("Model not found: {}", model_id);
        Ok(None)
    }

    /// Get model metadata
    pub async fn get_metadata(&self, model_id: &str) -> Result<Option<ModelMetadata>> {
        let registry = self.registry.read().await;
        Ok(registry.get(model_id).map(|entry| entry.metadata.clone()))
    }

    /// List models matching criteria
    pub async fn list_models(
        &self,
        criteria: ModelSearchCriteria,
    ) -> Result<Vec<ModelRegistryEntry>> {
        debug!("Listing models with criteria: {:?}", criteria);

        let registry = self.registry.read().await;
        let mut models: Vec<ModelRegistryEntry> = registry
            .values()
            .filter(|entry| self.matches_criteria(entry, &criteria))
            .cloned()
            .collect();

        // Sort by registration date (newest first)
        models.sort_by(|a, b| b.registered_at.cmp(&a.registered_at));

        Ok(models)
    }

    /// Update model metadata
    pub async fn update_metadata(&self, model_id: &str, metadata: ModelMetadata) -> Result<()> {
        debug!("Updating metadata for model: {}", model_id);

        let mut registry = self.registry.write().await;
        if let Some(entry) = registry.get_mut(model_id) {
            entry.metadata = metadata;
            info!("Updated metadata for model: {}", model_id);
            Ok(())
        } else {
            anyhow::bail!("Model not found: {}", model_id);
        }
    }

    /// Remove a model from the registry
    pub async fn remove_model(&self, model_id: &str) -> Result<()> {
        debug!("Removing model: {}", model_id);

        // Remove from registry
        {
            let mut registry = self.registry.write().await;
            registry.remove(model_id);
        }

        // Remove from loaded models
        {
            let mut loaded_models = self.loaded_models.write().await;
            loaded_models.remove(model_id);
        }

        // Remove from disk if persisted
        if self.config.persist_to_disk {
            self.remove_model_from_disk(model_id).await?;
        }

        info!("Successfully removed model: {}", model_id);
        Ok(())
    }

    /// Get registry statistics
    pub async fn get_registry_stats(&self) -> Result<RegistryStats> {
        let registry = self.registry.read().await;
        let loaded_models = self.loaded_models.read().await;

        let total_models = registry.len();
        let loaded_count = loaded_models.len();
        let total_size: u64 = registry.values().map(|e| e.model_size).sum();

        let status_counts = registry.values().fold(HashMap::new(), |mut counts, entry| {
            let status_str = format!("{:?}", entry.status);
            *counts.entry(status_str).or_insert(0) += 1;
            counts
        });

        let type_counts = registry.values().fold(HashMap::new(), |mut counts, entry| {
            let type_str = format!("{:?}", entry.metadata.model_type);
            *counts.entry(type_str).or_insert(0) += 1;
            counts
        });

        Ok(RegistryStats {
            total_models,
            loaded_models: loaded_count,
            total_size_bytes: total_size,
            status_counts,
            type_counts,
        })
    }

    /// Clean up old models to free memory
    pub async fn cleanup_memory(&self) -> Result<usize> {
        debug!("Cleaning up memory");

        let loaded_models = self.loaded_models.read().await;
        let registry = self.registry.read().await;

        if loaded_models.len() <= self.config.max_models_in_memory {
            return Ok(0);
        }

        // Find least recently used models
        let mut models_by_access: Vec<_> = registry
            .iter()
            .filter(|(_, entry)| entry.model_loaded)
            .collect();
        models_by_access.sort_by(|a, b| a.1.last_accessed.cmp(&b.1.last_accessed));

        let models_to_remove = loaded_models.len() - self.config.max_models_in_memory;
        let mut removed_count = 0;

        for (model_id, _) in models_by_access.iter().take(models_to_remove) {
            // Unload from memory
            {
                let mut loaded = self.loaded_models.write().await;
                loaded.remove(*model_id);
            }

            // Update registry entry
            {
                let mut registry = self.registry.write().await;
                if let Some(entry) = registry.get_mut(*model_id) {
                    entry.model_loaded = false;
                }
            }

            removed_count += 1;
        }

        info!("Cleaned up {} models from memory", removed_count);
        Ok(removed_count)
    }

    /// Check if model matches search criteria
    fn matches_criteria(&self, entry: &ModelRegistryEntry, criteria: &ModelSearchCriteria) -> bool {
        if let Some(model_type) = &criteria.model_type {
            if entry.metadata.model_type != *model_type {
                return false;
            }
        }

        if let Some(version) = &criteria.version {
            if entry.metadata.version != *version {
                return false;
            }
        }

        if let Some(status) = &criteria.status {
            if !matches!(&entry.status, status) {
                return false;
            }
        }

        if let Some(after) = &criteria.created_after {
            if entry.metadata.created_at < *after {
                return false;
            }
        }

        if let Some(before) = &criteria.created_before {
            if entry.metadata.created_at > *before {
                return false;
            }
        }

        if let Some(text) = &criteria.description_contains {
            if !entry
                .metadata
                .description
                .to_lowercase()
                .contains(&text.to_lowercase())
            {
                return false;
            }
        }

        if let Some(min_accuracy) = criteria.min_accuracy {
            let metrics = &entry.metadata.performance_metrics;
            if let Some(accuracy) = metrics.accuracy {
                if accuracy < min_accuracy {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Update access statistics
    async fn update_access_stats(&self, model_id: &str) {
        let mut registry = self.registry.write().await;
        if let Some(entry) = registry.get_mut(model_id) {
            entry.last_accessed = chrono::Utc::now();
            entry.access_count += 1;
        }
    }

    /// Update version tracking
    async fn update_version_tracking(&self, model_id: &str) -> Result<()> {
        // Extract base name (remove version suffix)
        let base_name = model_id.split('_').next().unwrap_or(model_id);

        let mut versions = self.model_versions.write().await;
        let version_list = versions
            .entry(base_name.to_string())
            .or_insert_with(Vec::new);

        if !version_list.contains(&model_id.to_string()) {
            version_list.push(model_id.to_string());

            // Limit versions per model
            if version_list.len() > self.config.max_versions_per_model {
                version_list.remove(0);
            }
        }

        Ok(())
    }

    /// Persist model to disk
    async fn persist_model(&self, model_id: &str) -> Result<()> {
        // This would implement actual model persistence
        // For now, it's a placeholder
        debug!("Persisting model to disk: {}", model_id);
        Ok(())
    }

    /// Load model from disk
    async fn load_model_from_disk(&self, model_id: &str) -> Result<Box<dyn TrainedModel>> {
        // This would implement actual model loading
        // For now, it's a placeholder
        anyhow::bail!("Model loading from disk not yet implemented: {}", model_id);
    }

    /// Remove model from disk
    async fn remove_model_from_disk(&self, model_id: &str) -> Result<()> {
        // This would implement actual model removal
        // For now, it's a placeholder
        debug!("Removing model from disk: {}", model_id);
        Ok(())
    }

    /// Estimate model size
    fn estimate_model_size(&self, _model: &dyn TrainedModel) -> u64 {
        // This is a simplified estimation
        // In practice, this would calculate actual model size
        1024 * 1024 // 1MB default
    }
}

/// Registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryStats {
    /// Total number of models
    pub total_models: usize,
    /// Number of models loaded in memory
    pub loaded_models: usize,
    /// Total size of all models in bytes
    pub total_size_bytes: u64,
    /// Count by status
    pub status_counts: HashMap<String, usize>,
    /// Count by model type
    pub type_counts: HashMap<String, usize>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new(ModelRegistryConfig::default()).unwrap()
    }
}
