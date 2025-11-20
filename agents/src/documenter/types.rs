//! Documenter Types
//!
//! This module contains all data types and structures used by the Documenter agent
//! for documentation generation and analysis.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::llm_integration::LLMProvider;
use odincode_core::CodeSuggestion;

/// Documentation request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationRequest {
    /// The code file to document
    pub file: odincode_core::CodeFile,
    /// Type of documentation to generate
    pub documentation_type: DocumentationType,
    /// Target audience for the documentation
    pub target_audience: TargetAudience,
    /// Documentation style preferences
    pub style_preferences: DocumentationStyle,
    /// Language of the code
    pub language: String,
    /// Additional context or requirements
    pub context: Option<String>,
}

/// Types of documentation that can be generated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentationType {
    /// API documentation (function signatures, parameters, returns)
    ApiDocumentation,
    /// Code explanation (what the code does and how it works)
    CodeExplanation,
    /// Inline comments (function and line-level comments)
    InlineComments,
    /// Architecture documentation (high-level design and structure)
    ArchitectureDocumentation,
    /// User guide (getting started, how-to guides)
    UserGuide,
    /// Complete documentation (all of the above)
    Complete,
}

/// Target audience for documentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TargetAudience {
    /// Beginner developers
    Beginner,
    /// Intermediate developers
    Intermediate,
    /// Expert developers
    Expert,
    /// Non-technical users
    NonTechnical,
    /// Mixed audience
    Mixed,
}

/// Documentation style preferences
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationStyle {
    /// Include code examples
    pub include_examples: bool,
    /// Include architecture diagrams
    pub include_diagrams: bool,
    /// Include performance considerations
    pub include_performance: bool,
    /// Include security considerations
    pub include_security: bool,
    /// Include troubleshooting information
    pub include_troubleshooting: bool,
    /// Documentation format (markdown, html, etc.)
    pub output_format: OutputFormat,
    /// Level of detail (concise, detailed, comprehensive)
    pub detail_level: DetailLevel,
}

/// Output format for generated documentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputFormat {
    /// Markdown format
    Markdown,
    /// HTML format
    Html,
    /// Plain text format
    PlainText,
    /// reStructuredText format
    RestructuredText,
    /// Javadoc format
    Javadoc,
    /// Doxygen format
    Doxygen,
}

/// Level of detail for documentation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DetailLevel {
    /// Concise documentation (essential information only)
    Concise,
    /// Detailed documentation (comprehensive but focused)
    Detailed,
    /// Comprehensive documentation (exhaustive coverage)
    Comprehensive,
}

/// Generated documentation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationResult {
    /// Unique identifier for the documentation
    pub id: Uuid,
    /// The generated documentation content
    pub content: String,
    /// Type of documentation generated
    pub documentation_type: DocumentationType,
    /// Target audience
    pub target_audience: TargetAudience,
    /// Output format
    pub output_format: OutputFormat,
    /// Metadata about the documentation
    pub metadata: DocumentationMetadata,
    /// Suggestions for improving the code based on documentation needs
    pub improvement_suggestions: Vec<DocumentationSuggestion>,
    /// Timestamp when documentation was generated
    pub generated_at: DateTime<Utc>,
}

/// Documentation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationMetadata {
    /// Number of lines documented
    pub lines_documented: usize,
    /// Number of functions documented
    pub functions_documented: usize,
    /// Number of code examples generated
    pub examples_generated: usize,
    /// Documentation quality score (0.0 to 1.0)
    pub quality_score: f64,
    /// Estimated reading time in minutes
    pub reading_time_minutes: usize,
    /// Language-specific standards followed
    pub standards_followed: Vec<String>,
}

/// Suggestion for improving code based on documentation analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationSuggestion {
    /// Type of suggestion
    pub suggestion_type: DocumentationSuggestionType,
    /// Description of the suggestion
    pub description: String,
    /// Suggested code change
    pub suggested_change: Option<String>,
    /// Reason for the suggestion
    pub reason: String,
    /// Priority of the suggestion
    pub priority: SuggestionPriority,
    /// Line number (if applicable)
    pub line_number: Option<usize>,
}

/// Types of documentation suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentationSuggestionType {
    /// Add missing function documentation
    AddFunctionDocumentation,
    /// Add missing parameter documentation
    AddParameterDocumentation,
    /// Add missing return value documentation
    AddReturnDocumentation,
    /// Add code examples
    AddCodeExamples,
    /// Improve naming for better documentation
    ImproveNaming,
    /// Add error handling documentation
    AddErrorDocumentation,
    /// Add performance considerations
    AddPerformanceDocumentation,
    /// Add security considerations
    AddSecurityDocumentation,
    /// Refactor complex code for better documentation
    RefactorForDocumentation,
}

/// Priority level for suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SuggestionPriority {
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

/// Documenter agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumenterConfig {
    /// Default LLM provider to use
    pub default_llm_provider: LLMProvider,
    /// Default documentation style
    pub default_style: DocumentationStyle,
    /// Maximum documentation length
    pub max_documentation_length: usize,
    /// Enable learning from user feedback
    pub enable_learning: bool,
    /// Quality threshold for documentation
    pub quality_threshold: f64,
}

impl Default for DocumenterConfig {
    fn default() -> Self {
        Self {
            default_llm_provider: LLMProvider::OpenAI,
            default_style: DocumentationStyle {
                include_examples: true,
                include_diagrams: true,
                include_performance: true,
                include_security: true,
                include_troubleshooting: true,
                output_format: OutputFormat::Markdown,
                detail_level: DetailLevel::Detailed,
            },
            max_documentation_length: 50000,
            enable_learning: true,
            quality_threshold: 0.7,
        }
    }
}

impl std::fmt::Display for DocumentationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentationType::ApiDocumentation => write!(f, "API Documentation"),
            DocumentationType::CodeExplanation => write!(f, "Code Explanation"),
            DocumentationType::InlineComments => write!(f, "Inline Comments"),
            DocumentationType::ArchitectureDocumentation => write!(f, "Architecture Documentation"),
            DocumentationType::UserGuide => write!(f, "User Guide"),
            DocumentationType::Complete => write!(f, "Complete Documentation"),
        }
    }
}
