//! Model Management Module
//!
//! Provides model import/export, performance tracking, and lifecycle management
//! for trained ML models in the OdinCode system.

pub mod exporter;
pub mod importer;
pub mod performance_tracker;
pub mod registry;

// Re-export key types
pub use exporter::{ExportConfig, ExportFormat, ModelExporter};
pub use importer::{ImportConfig, ImportFormat, ModelImporter};
pub use performance_tracker::{
    ModelPerformanceTracker, PerformanceRecord, PerformanceStats, PerformanceThresholds,
    PerformanceTrackingConfig,
};
pub use registry::{ModelRegistry, ModelRegistryConfig, ModelRegistryEntry, ModelSearchCriteria};
