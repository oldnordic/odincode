//! Tool Management Models
//!
//! This module defines the data structures for tool management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Tool integration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolIntegration {
    /// Unique identifier for the tool
    pub id: Uuid,
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool type
    pub tool_type: ToolType,
    /// Current status of the tool
    pub status: ToolStatus,
    /// Configuration parameters
    pub config: HashMap<String, String>,
    /// When the tool was created
    pub created: DateTime<Utc>,
    /// When the tool was last updated
    pub last_updated: DateTime<Utc>,
}

/// Tool type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolType {
    /// Code linter
    Linter,
    /// Code formatter
    Formatter,
    /// Testing framework
    TestingFramework,
    /// Build system
    BuildSystem,
    /// Version control system
    VersionControl,
    /// Debugger
    Debugger,
    /// Package manager
    PackageManager,
    /// IDE integration
    IDE,
}

/// Tool status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolStatus {
    /// Tool is not configured
    NotConfigured,
    /// Tool is configured but not connected
    Disconnected,
    /// Tool is connected and ready
    Connected,
    /// Tool is currently executing
    Executing,
    /// Tool has an error
    Error,
    /// Tool is disabled
    Disabled,
}
