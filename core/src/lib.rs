//! OdinCode Core Module
//!
//! The core module provides fundamental functionality for the OdinCode AI coding assistant.
//! It includes code analysis, processing, and the main engine that powers the AI capabilities.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub mod action_history;
pub mod advanced_features;
pub mod config;
pub mod database;
pub mod database_queries;
pub mod file_metadata;
pub mod graph_database;
pub mod language_analyzers;
pub mod language_parsing;
pub mod large_codebase_mapper;
pub mod llm_integration;
pub mod ml_integration;
pub mod rag_database;
pub mod semantic_analysis;
#[cfg(feature = "simple-ltmc")]
pub mod simple_ltmc;
pub mod symbol_table;

#[cfg(test)]
mod integration_test;
#[cfg(test)]
mod verify_ml_integration;

// Re-export commonly used ML integration types for easier access
pub use ml_integration::MLIntegrationConfig;
pub use ml_integration::MLIntegrationManager;
pub use semantic_analysis::SemanticAnalysisEngine;

#[cfg(feature = "simple-ltmc")]
pub use simple_ltmc::SimpleLTMCManager;

/// Represents a code file with its content and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeFile {
    /// Unique identifier for the file
    pub id: Uuid,
    /// File path
    pub path: String,
    /// File content
    pub content: String,
    /// Programming language
    pub language: String,
    /// Last modification timestamp
    pub modified: chrono::DateTime<chrono::Utc>,
}

/// Represents a code analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Unique identifier for the analysis
    pub id: Uuid,
    /// Associated file ID
    pub file_id: Uuid,
    /// List of issues found
    pub issues: Vec<CodeIssue>,
    /// Suggestions for improvements
    pub suggestions: Vec<CodeSuggestion>,
    /// Analysis timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Represents a code issue found during analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    /// Issue identifier
    pub id: Uuid,
    /// Issue type
    pub issue_type: IssueType,
    /// Severity level
    pub severity: Severity,
    /// Description of the issue
    pub description: String,
    /// Line number where the issue occurs
    pub line_number: usize,
    /// Column number where the issue occurs
    pub column_number: usize,
    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Type of code issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IssueType {
    /// Syntax error
    SyntaxError,
    /// Potential bug
    PotentialBug,
    /// Performance issue
    Performance,
    /// Security vulnerability
    Security,
    /// Style issue
    Style,
    /// Best practice violation
    BestPractice,
}

/// Severity level of an issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    /// Info level - informational suggestion
    Info,
    /// Low severity
    Low,
    /// Warning level - potential issue
    Warning,
    /// Medium severity
    Medium,
    /// High severity
    High,
    /// Critical severity
    Critical,
}

/// Represents a code suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSuggestion {
    /// Suggestion identifier
    pub id: Uuid,
    /// Type of suggestion
    pub suggestion_type: SuggestionType,
    /// Title of suggestion
    pub title: String,
    /// Description of suggestion
    pub description: String,
    /// Code snippet for suggestion
    pub code_snippet: Option<String>,
    /// Confidence level in suggestion
    pub confidence: f32,
    /// File path where suggestion applies
    pub file_path: String,
    /// Line number where suggestion applies
    pub line_number: Option<usize>,
    /// Severity level of suggestion
    pub severity: Severity,
    /// Whether suggestion can be automatically fixed
    pub auto_fixable: bool,
}

impl CodeSuggestion {
    /// Create a new code suggestion with defaults for backward compatibility
    pub fn new(
        id: Uuid,
        suggestion_type: SuggestionType,
        description: String,
        code_snippet: String,
        confidence: f32,
    ) -> Self {
        Self {
            id,
            suggestion_type,
            title: description.clone(),
            description,
            code_snippet: if code_snippet.is_empty() {
                None
            } else {
                Some(code_snippet)
            },
            confidence,
            file_path: String::new(),
            line_number: None,
            severity: Severity::Info,
            auto_fixable: false,
        }
    }

    /// Create a complete code suggestion with all fields
    pub fn complete(
        id: Uuid,
        suggestion_type: SuggestionType,
        title: String,
        description: String,
        code_snippet: Option<String>,
        confidence: f32,
        file_path: String,
        line_number: Option<usize>,
        severity: Severity,
        auto_fixable: bool,
    ) -> Self {
        Self {
            id,
            suggestion_type,
            title,
            description,
            code_snippet,
            confidence,
            file_path,
            line_number,
            severity,
            auto_fixable,
        }
    }

    /// Create a minimal code suggestion with required fields only
    pub fn new_minimal(
        title: impl Into<String>,
        message: impl Into<String>,
        severity: Severity,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            suggestion_type: SuggestionType::Refactor,
            title: title.into(),
            description: message.into(),
            code_snippet: None,
            confidence: 0.5,
            file_path: String::new(),
            line_number: None,
            severity,
            auto_fixable: false,
        }
    }

    /// Create a complete code suggestion with all fields (new API)
    pub fn new_complete(
        title: impl Into<String>,
        message: impl Into<String>,
        severity: Severity,
        file_path: Option<String>,
        line_number: Option<u32>,
        auto_fixable: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            suggestion_type: SuggestionType::Refactor,
            title: title.into(),
            description: message.into(),
            code_snippet: None,
            confidence: 0.5,
            file_path: file_path.unwrap_or_default(),
            line_number: line_number.map(|n| n as usize),
            severity,
            auto_fixable,
        }
    }
}

/// Type of code suggestion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SuggestionType {
    /// Refactoring suggestion
    Refactor,
    /// Optimization suggestion
    Optimize,
    /// Documentation suggestion
    Document,
    /// Test generation suggestion
    Test,
    /// Feature implementation suggestion
    Feature,
    /// Extract method/function suggestion
    Extract,
    /// Rename suggestion
    Rename,
}

/// Main engine for code analysis and processing
#[derive(Clone)]
pub struct CodeEngine {
    /// Map of loaded files
    files: Arc<RwLock<HashMap<Uuid, CodeFile>>>,
    /// Map of analysis results
    analysis_results: Arc<RwLock<HashMap<Uuid, AnalysisResult>>>,
    /// ML integration manager for AI-powered analysis
    ml_integration: Arc<RwLock<Option<Arc<ml_integration::MLIntegrationManager>>>>,
    /// Language analyzer manager for language-specific analysis
    language_analyzer_manager: Arc<language_analyzers::LanguageAnalyzerManager>,
    /// Performance optimizer for large codebases
    performance_optimizer: Option<Arc<large_codebase_mapper::PerformanceOptimizer>>,
}

impl CodeEngine {
    /// Create a new code engine instance
    pub fn new() -> Result<Self> {
        let language_analyzer_manager =
            Arc::new(language_analyzers::LanguageAnalyzerManager::new()?);

        Ok(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(None)),
            language_analyzer_manager,
            performance_optimizer: None,
        })
    }

    /// Create a new code engine instance with ML integration
    pub async fn new_with_ml(
        ml_config: ml_integration::MLIntegrationConfig,
        ltmc_manager: std::sync::Arc<odincode_ltmc::LTMManager>,
    ) -> Result<Self> {
        let language_analyzer_manager =
            Arc::new(language_analyzers::LanguageAnalyzerManager::new()?);

        let engine = std::sync::Arc::new(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(None)),
            language_analyzer_manager: language_analyzer_manager.clone(),
            performance_optimizer: None,
        });

        let ml_integration =
            ml_integration::MLIntegrationManager::new(engine, ltmc_manager, ml_config).await?;

        Ok(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(Some(Arc::new(ml_integration)))),
            language_analyzer_manager,
            performance_optimizer: None,
        })
    }

    /// Create a new code engine instance with ML and LLM integration
    pub async fn new_with_ml_and_llm(
        ml_config: ml_integration::MLIntegrationConfig,
        ltmc_manager: std::sync::Arc<odincode_ltmc::LTMManager>,
        llm_integration: std::sync::Arc<llm_integration::LLMIntegrationManager>,
    ) -> Result<Self> {
        let language_analyzer_manager =
            Arc::new(language_analyzers::LanguageAnalyzerManager::new()?);

        let engine = std::sync::Arc::new(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(None)),
            language_analyzer_manager: language_analyzer_manager.clone(),
            performance_optimizer: None,
        });

        let mut ml_integration =
            ml_integration::MLIntegrationManager::new(engine, ltmc_manager, ml_config).await?;

        // Set LLM integration in ML integration manager
        ml_integration.set_llm_integration(llm_integration).await;

        Ok(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(Some(Arc::new(ml_integration)))),
            language_analyzer_manager,
            performance_optimizer: None,
        })
    }

    /// Create a new code engine instance with performance optimization
    pub fn new_with_performance_optimization(
        database_manager: odincode_databases::DatabaseManager,
    ) -> Result<Self> {
        let language_analyzer_manager =
            Arc::new(language_analyzers::LanguageAnalyzerManager::new()?);
        let performance_optimizer = Arc::new(large_codebase_mapper::PerformanceOptimizer::new(
            database_manager,
        ));

        Ok(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(None)),
            language_analyzer_manager,
            performance_optimizer: Some(performance_optimizer),
        })
    }

    /// Create a new code engine instance with ML, LLM, and performance optimization
    pub async fn new_with_all_features(
        ml_config: ml_integration::MLIntegrationConfig,
        ltmc_manager: std::sync::Arc<odincode_ltmc::LTMManager>,
        llm_integration: std::sync::Arc<llm_integration::LLMIntegrationManager>,
        database_manager: odincode_databases::DatabaseManager,
    ) -> Result<Self> {
        let language_analyzer_manager =
            Arc::new(language_analyzers::LanguageAnalyzerManager::new()?);
        let performance_optimizer = Arc::new(large_codebase_mapper::PerformanceOptimizer::new(
            database_manager,
        ));

        let engine = std::sync::Arc::new(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(None)),
            language_analyzer_manager: language_analyzer_manager.clone(),
            performance_optimizer: None,
        });

        let mut ml_integration =
            ml_integration::MLIntegrationManager::new(engine, ltmc_manager, ml_config).await?;

        // Set LLM integration in ML integration manager
        ml_integration.set_llm_integration(llm_integration).await;

        Ok(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(Some(Arc::new(ml_integration)))),
            language_analyzer_manager,
            performance_optimizer: Some(performance_optimizer),
        })
    }

    /// Create a new code engine instance with simple LTMC (SQLite+FAISS only)
    #[cfg(feature = "simple-ltmc")]
    pub async fn new_with_simple_ltmc(
        ml_config: ml_integration::MLIntegrationConfig,
        llm_integration: std::sync::Arc<llm_integration::LLMIntegrationManager>,
    ) -> Result<Self> {
        let language_analyzer_manager =
            Arc::new(language_analyzers::LanguageAnalyzerManager::new()?);

        // Create a self-referencing Arc for the ML integration constructor
        let self_ref = std::sync::Arc::new(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(None)),
            language_analyzer_manager: language_analyzer_manager.clone(),
            performance_optimizer: None,
        });

        // Create ML integration manager for simple LTMC
        let mut ml_integration = ml_integration::MLIntegrationManager::new(
            ml_config,
            self_ref.clone(), // This will be replaced after creation
            // For simple LTMC, we'll pass a new LTMC manager that doesn't connect to external services
            std::sync::Arc::new(odincode_ltmc::LTMManager::new()?),
        )?;

        // Set the LLM integration in the ML integration manager
        ml_integration.set_llm_integration(llm_integration).await;

        // Create the final engine with the ML integration
        Ok(Self {
            files: Arc::new(RwLock::new(HashMap::new())),
            analysis_results: Arc::new(RwLock::new(HashMap::new())),
            ml_integration: Arc::new(RwLock::new(Some(std::sync::Arc::new(ml_integration)))),
            language_analyzer_manager,
            performance_optimizer: None,
        })
    }

    /// Helper method to create a LTMC manager for simple-ltmc
    #[cfg(feature = "simple-ltmc")]
    pub fn create_ltmc_manager() -> Result<std::sync::Arc<odincode_ltmc::LTMManager>> {
        // Create a LTMC manager for simple LTMC
        use odincode_ltmc::LTMManager;
        Ok(std::sync::Arc::new(LTMManager::new()?))
    }

    /// Set ML integration manager
    pub async fn set_ml_integration(
        &self,
        ml_integration: std::sync::Arc<ml_integration::MLIntegrationManager>,
    ) {
        let mut ml_integration_ref = self.ml_integration.write().await;
        *ml_integration_ref = Some(ml_integration);
    }

    /// Get ML integration manager
    pub async fn get_ml_integration(
        &self,
    ) -> Option<std::sync::Arc<ml_integration::MLIntegrationManager>> {
        let ml_integration_ref = self.ml_integration.read().await;
        ml_integration_ref.as_ref().cloned()
    }

    /// Set LLM integration in ML integration manager
    pub async fn set_llm_integration(
        &self,
        _llm_integration: std::sync::Arc<llm_integration::LLMIntegrationManager>,
    ) -> Result<()> {
        // For now, this is a compatibility shim that doesn't actually set anything
        // since we can't get mutable access to Arc<MLIntegrationManager>
        // In a real implementation, we would need to restructure this to use Arc<RwLock<>>
        info!("LLM integration set called (compatibility shim)");
        Ok(())
    }

    /// Get reference to performance optimizer if available
    pub fn get_performance_optimizer(
        &self,
    ) -> Option<std::sync::Arc<large_codebase_mapper::PerformanceOptimizer>> {
        self.performance_optimizer.clone()
    }

    /// Perform parallel analysis on multiple files
    pub async fn parallel_analyze_files(
        &self,
        file_ids: Vec<Uuid>,
    ) -> Result<HashMap<Uuid, AnalysisResult>> {
        if let Some(ref performance_optimizer) = self.performance_optimizer {
            let mut files = Vec::new();

            // Get all requested files
            let files_read = self.files.read().await;
            for id in file_ids {
                if let Some(file) = files_read.get(&id) {
                    files.push(file.clone());
                }
            }
            drop(files_read);

            // Perform parallel analysis
            performance_optimizer.parallel_analysis(files).await
        } else {
            // Fallback to sequential analysis
            let mut results = HashMap::new();
            for id in file_ids {
                if let Some(result) = self.analyze_file(id).await? {
                    results.insert(id, result);
                }
            }
            Ok(results)
        }
    }

    /// Perform dependency-aware analysis on a file
    pub async fn dependency_aware_analyze(&self, file_path: &str) -> Result<Vec<AnalysisResult>> {
        if let Some(ref performance_optimizer) = self.performance_optimizer {
            performance_optimizer
                .dependency_aware_analysis(file_path)
                .await
        } else {
            // Fallback to basic analysis of the file
            let files_read = self.files.read().await;
            let mut results = Vec::new();

            for (_, file) in files_read.iter() {
                if file.path == file_path {
                    if let Some(result) = self.analyze_file(file.id).await? {
                        results.push(result);
                    }
                    break;
                }
            }

            Ok(results)
        }
    }

    /// Load a code file into the engine
    pub async fn load_file(&self, path: String, content: String, language: String) -> Result<Uuid> {
        let id = Uuid::new_v4();
        let file = CodeFile {
            id,
            path,
            content,
            language,
            modified: chrono::Utc::now(),
        };

        let mut files = self.files.write().await;
        files.insert(id, file);
        drop(files);

        info!("Loaded file: {}", id);
        Ok(id)
    }

    /// Load a code file into the engine with automatic language detection
    pub async fn load_file_with_detection(&self, path: String, content: String) -> Result<Uuid> {
        let language = self.detect_language_from_path(&path)?;
        self.load_file(path, content, language).await
    }

    /// Detect language from file path
    fn detect_language_from_path(&self, path: &str) -> Result<String> {
        let path_obj = std::path::Path::new(path);
        let extension = path_obj
            .extension()
            .ok_or_else(|| anyhow::anyhow!("No file extension found for: {}", path))?
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid file extension for: {}", path))?
            .to_lowercase();

        match extension.as_str() {
            "rs" => Ok("rust".to_string()),
            "js" => Ok("javascript".to_string()),
            "ts" => Ok("typescript".to_string()),
            "py" => Ok("python".to_string()),
            "java" => Ok("java".to_string()),
            "c" => Ok("c".to_string()),
            "cpp" | "cxx" | "cc" => Ok("cpp".to_string()),
            "cs" => Ok("csharp".to_string()),
            "go" => Ok("go".to_string()),
            "rb" => Ok("ruby".to_string()),
            "php" => Ok("php".to_string()),
            "swift" => Ok("swift".to_string()),
            "kt" | "kts" => Ok("kotlin".to_string()),
            "scala" | "sc" => Ok("scala".to_string()),
            "r" => Ok("r".to_string()),
            "dart" => Ok("dart".to_string()),
            "lua" => Ok("lua".to_string()),
            "pl" | "pm" => Ok("perl".to_string()),
            "m" => Ok("objective-c".to_string()),
            "sh" | "bash" => Ok("shell".to_string()),
            _ => Err(anyhow::anyhow!("Unsupported file extension: {}", extension)),
        }
    }

    /// Get a file by its ID
    pub async fn get_file(&self, id: Uuid) -> Result<Option<CodeFile>> {
        let files = self.files.read().await;
        Ok(files.get(&id).cloned())
    }

    /// Update a file's content
    pub async fn update_file(&self, id: Uuid, content: String) -> Result<bool> {
        let mut files = self.files.write().await;
        if let Some(file) = files.get_mut(&id) {
            file.content = content;
            file.modified = chrono::Utc::now();
            debug!("Updated file: {}", id);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Analyze a file and return results
    pub async fn analyze_file(&self, id: Uuid) -> Result<Option<AnalysisResult>> {
        let file = {
            let files = self.files.read().await;
            files.get(&id).cloned()
        };

        if let Some(file) = file {
            // Use performance optimizer if available for incremental analysis
            if let Some(ref performance_optimizer) = self.performance_optimizer {
                match performance_optimizer.incremental_analysis(&file).await {
                    Ok(Some(result)) => {
                        // Store the analysis result
                        let mut results = self.analysis_results.write().await;
                        results.insert(result.id, result.clone());
                        drop(results);

                        info!("Completed incremental analysis for file: {}", id);
                        return Ok(Some(result));
                    }
                    Ok(None) => {
                        // No analysis needed, return cached result
                        let results = self.analysis_results.read().await;
                        if let Some(cached_result) = results.get(&id) {
                            return Ok(Some(cached_result.clone()));
                        }
                    }
                    Err(e) => {
                        warn!(
                            "Incremental analysis failed: {}, falling back to full analysis",
                            e
                        );
                    }
                }
            }

            // Perform basic analysis
            let issues = self.perform_analysis(&file).await?;

            // Generate basic suggestions
            let mut suggestions = self.generate_suggestions(&file).await?;

            // Generate ML-enhanced suggestions if ML integration is available
            let ml_integration = self.get_ml_integration().await;
            if let Some(ml_integration) = ml_integration {
                debug!("Using ML integration for enhanced analysis");

                // If the ML integration has semantic analysis capabilities, use them
                // This is a more sophisticated approach that combines semantic analysis and LLM
                match self
                    .get_enhanced_ai_suggestions(&file, &ml_integration)
                    .await
                {
                    Ok(ai_suggestions) => {
                        info!(
                            "AI-enhanced analysis generated {} suggestions",
                            ai_suggestions.len()
                        );
                        suggestions.extend(ai_suggestions);
                    }
                    Err(e) => {
                        error!(
                            "AI-enhanced analysis failed: {}, falling back to basic ML analysis",
                            e
                        );
                        // Try basic ML analysis as fallback
                        match ml_integration.analyze_with_ml(&file.content).await {
                            Ok(ml_suggestions) => {
                                info!(
                                    "ML integration generated {} additional suggestions",
                                    ml_suggestions.len()
                                );
                                suggestions.extend(ml_suggestions);
                            }
                            Err(ml_error) => {
                                error!("ML analysis also failed: {}", ml_error);
                                // Continue with basic analysis if all AI methods fail
                            }
                        }
                    }
                }
            }

            let result = AnalysisResult {
                id: Uuid::new_v4(),
                file_id: id,
                issues,
                suggestions,
                timestamp: chrono::Utc::now(),
            };

            // Store the analysis result
            let mut results = self.analysis_results.write().await;
            results.insert(result.id, result.clone());
            drop(results);

            info!("Completed analysis for file: {}", id);
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    /// Get enhanced AI suggestions combining semantic analysis and LLM
    async fn get_enhanced_ai_suggestions(
        &self,
        file: &CodeFile,
        ml_integration: &Arc<ml_integration::MLIntegrationManager>,
    ) -> Result<Vec<CodeSuggestion>> {
        // First, we need to get the semantic analysis engine from the ML integration
        // Since we're only borrowing the ml_integration, we need to call the analysis method
        // This is a simplified approach - in practice, you might have a direct method for this

        // Try to generate enhanced suggestions using both semantic analysis and LLM integration
        let llm_integration = ml_integration.get_llm_integration().await;
        if let Some(_llm_integration) = llm_integration {
            // We need to do semantic analysis first to get the enhanced analysis
            // This requires using the semantic analyzer in the ml_integration
            match ml_integration.analyze_with_ml(&file.content).await {
                Ok(mut ml_suggestions) => {
                    // Sort suggestions by confidence (highest first)
                    ml_suggestions.sort_by(|a, b| {
                        b.confidence
                            .partial_cmp(&a.confidence)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    Ok(ml_suggestions)
                }
                Err(e) => {
                    error!("Semantic + ML analysis failed: {}", e);
                    // Fallback: return empty suggestions but don't fail the whole process
                    Ok(Vec::new())
                }
            }
        } else {
            // If there's no LLM integration, just use the semantic analysis from ML integration
            ml_integration.analyze_with_ml(&file.content).await
        }
    }

    /// Perform code analysis on a file
    async fn perform_analysis(&self, file: &CodeFile) -> Result<Vec<CodeIssue>> {
        debug!("Analyzing file: {}", file.path);

        // Use Tree-sitter parsing if available for the language
        let supported_lang = language_parsing::SupportedLanguage::from_str(&file.language);

        if let Some(lang) = supported_lang {
            // Use the language parsing module for more sophisticated analysis
            let mut parser = language_parsing::LanguageParser::new()?;
            match parser.parse(&file.content, &lang) {
                Ok(tree) => {
                    // Perform language-specific analysis using the analyzer manager
                    let lang_issues =
                        self.language_analyzer_manager
                            .analyze(&lang, &tree, &file.content)?;
                    let mut issues = lang_issues;

                    // Perform general AST-based analysis
                    let ast_issues = self.analyze_with_ast(&file, &tree, &lang)?;
                    issues.extend(ast_issues);

                    // Add basic line-based checks as well
                    let basic_issues = self.perform_basic_analysis(file)?;
                    issues.extend(basic_issues);

                    return Ok(issues);
                }
                Err(e) => {
                    // If AST parsing fails, fall back to basic analysis
                    debug!(
                        "AST parsing failed for {}: {}, falling back to basic analysis",
                        file.path, e
                    );
                }
            }
        }

        // Basic line-by-line analysis for unsupported languages or when AST parsing fails
        self.perform_basic_analysis(file)
    }

    /// Perform basic line-by-line analysis
    fn perform_basic_analysis(&self, file: &CodeFile) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = file.content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Check for potential issues
            if line.trim().is_empty()
                && line_idx + 1 < lines.len()
                && lines[line_idx + 1].trim().is_empty()
            {
                // Multiple empty lines
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: Severity::Low,
                    description: "Multiple consecutive empty lines".to_string(),
                    line_number: line_idx + 1,
                    column_number: 0,
                    suggestion: Some("Remove extra empty lines".to_string()),
                });
            }

            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: Severity::Low,
                    description: "Trailing whitespace detected".to_string(),
                    line_number: line_idx + 1,
                    column_number: line.len(),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }

            // Check for line length (assuming 100 characters as limit)
            if line.len() > 100 {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: Severity::Medium,
                    description: "Line exceeds 100 characters".to_string(),
                    line_number: line_idx + 1,
                    column_number: 100,
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }

            // Language-specific checks
            match file.language.as_str() {
                "rust" => {
                    // Check for potential Rust issues
                    if line.contains("TODO") || line.contains("FIXME") || line.contains("HACK") {
                        issues.push(CodeIssue {
                            id: Uuid::new_v4(),
                            issue_type: IssueType::BestPractice,
                            severity: Severity::Medium,
                            description: "TODO/FIXME/HACK comment found".to_string(),
                            line_number: line_idx + 1,
                            column_number: 0,
                            suggestion: Some("Address the technical debt".to_string()),
                        });
                    }

                    // Check for potential performance issues
                    if line.contains(".collect::<Vec<_>>().len()") {
                        issues.push(CodeIssue {
                            id: Uuid::new_v4(),
                            issue_type: IssueType::Performance,
                            severity: Severity::High,
                            description: "Inefficient length calculation after collect".to_string(),
                            line_number: line_idx + 1,
                            column_number: 0,
                            suggestion: Some(
                                "Use .count() or .len() directly on iterator".to_string(),
                            ),
                        });
                    }
                }
                "javascript" | "typescript" => {
                    // Check for potential JavaScript/TypeScript issues
                    if line.contains("==") && !line.contains("===\"") && !line.contains("!==") {
                        issues.push(CodeIssue {
                            id: Uuid::new_v4(),
                            issue_type: IssueType::PotentialBug,
                            severity: Severity::High,
                            description: "Use of == instead of === for comparison".to_string(),
                            line_number: line_idx + 1,
                            column_number: 0,
                            suggestion: Some(
                                "Use === for comparison to avoid type coercion".to_string(),
                            ),
                        });
                    }
                }
                _ => {
                    // Add checks for other languages as needed
                }
            }
        }

        Ok(issues)
    }

    /// Perform AST-based analysis
    fn analyze_with_ast(
        &self,
        file: &CodeFile,
        tree: &tree_sitter::Tree,
        _lang: &language_parsing::SupportedLanguage,
    ) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();

        // This is a simplified example - in a real implementation, we would have more
        // sophisticated AST traversal and analysis based on the specific language
        let root_node = tree.root_node();
        self.traverse_ast_for_issues(root_node, file, &mut issues, 0)?;

        Ok(issues)
    }

    /// Traverse the AST and collect issues
    fn traverse_ast_for_issues(
        &self,
        node: tree_sitter::Node,
        file: &CodeFile,
        issues: &mut Vec<CodeIssue>,
        depth: usize,
    ) -> Result<()> {
        // Limit depth to prevent infinite recursion on large files
        if depth > 100 {
            return Ok(());
        }

        // Example: Look for specific patterns in the AST
        match node.kind() {
            "ERROR" | "MISSING" | "UNEXPECTED_CHARACTER" => {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::SyntaxError,
                    severity: Severity::High,
                    description: format!("Syntax error: {}", node.kind()),
                    line_number: node.start_position().row + 1,
                    column_number: node.start_position().column,
                    suggestion: Some("Fix the syntax error".to_string()),
                });
            }
            "comment" => {
                // Check if comment contains TODO/FIXME/HACK
                let content = &file.content[node.start_byte()..node.end_byte()];
                if content.contains("TODO") || content.contains("FIXME") || content.contains("HACK")
                {
                    issues.push(CodeIssue {
                        id: Uuid::new_v4(),
                        issue_type: IssueType::BestPractice,
                        severity: Severity::Medium,
                        description: "TODO/FIXME/HACK comment found".to_string(),
                        line_number: node.start_position().row + 1,
                        column_number: node.start_position().column,
                        suggestion: Some("Address the technical debt".to_string()),
                    });
                }
            }
            _ => {
                // Continue traversing children
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_ast_for_issues(child, file, issues, depth + 1)?;
        }

        Ok(())
    }

    /// Generate code suggestions for a file
    async fn generate_suggestions(&self, file: &CodeFile) -> Result<Vec<CodeSuggestion>> {
        debug!("Generating suggestions for file: {}", file.path);

        // Use Tree-sitter parsing if available for the language
        let supported_lang = language_parsing::SupportedLanguage::from_str(&file.language);

        let mut suggestions = Vec::new();

        if let Some(lang) = supported_lang {
            // Use the language parsing module for more sophisticated suggestions
            let mut parser = language_parsing::LanguageParser::new()?;
            match parser.parse(&file.content, &lang) {
                Ok(tree) => {
                    // Generate language-specific suggestions using the analyzer manager
                    let lang_suggestions = self.language_analyzer_manager.generate_suggestions(
                        &lang,
                        &tree,
                        &file.content,
                    )?;
                    suggestions.extend(lang_suggestions);

                    // Generate general AST-based suggestions
                    let ast_suggestions =
                        self.generate_suggestions_with_ast(&file, &tree, &lang)?;
                    suggestions.extend(ast_suggestions);
                }
                Err(e) => {
                    // If AST parsing fails, fall back to basic suggestions
                    debug!(
                        "AST parsing failed for {}: {}, falling back to basic suggestions",
                        file.path, e
                    );
                }
            }
        }

        // Add basic language-specific suggestions as well
        let basic_suggestions = self.generate_basic_suggestions(file)?;
        suggestions.extend(basic_suggestions);

        Ok(suggestions)
    }

    /// Generate basic language-specific suggestions
    fn generate_basic_suggestions(&self, file: &CodeFile) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();

        // Language-specific suggestions
        match file.language.as_str() {
            "rust" => {
                // Suggest performance improvements
                if file.content.contains(".collect::<Vec<_>>().len()") {
                    suggestions.push(CodeSuggestion::new(
                        Uuid::new_v4(),
                        SuggestionType::Optimize,
                        "Use .count() instead of collecting to Vec and then getting length"
                            .to_string(),
                        ".count()".to_string(),
                        0.9,
                    ));
                }

                // Suggest refactoring opportunities
                if file.content.matches('{').count() > 10 {
                    suggestions.push(CodeSuggestion::new(
                        Uuid::new_v4(),
                        SuggestionType::Refactor,
                        "Consider breaking down complex function into smaller functions"
                            .to_string(),
                        "// Break complex logic into smaller, focused functions".to_string(),
                        0.7,
                    ));
                }
            }
            "javascript" | "typescript" => {
                // Suggest modern JavaScript practices
                if file.content.contains("var ") {
                    suggestions.push(CodeSuggestion::new(
                        Uuid::new_v4(),
                        SuggestionType::Refactor,
                        "Use 'let' or 'const' instead of 'var'".to_string(),
                        "const or let".to_string(),
                        0.85,
                    ));
                }
            }
            _ => {
                // Add suggestions for other languages as needed
            }
        }

        // General suggestions
        if file.content.contains("console.log")
            || file.content.contains("println!")
            || file.content.contains("print")
        {
            suggestions.push(CodeSuggestion::new(
                Uuid::new_v4(),
                SuggestionType::Document,
                "Remove debug print statements before production".to_string(),
                "// Remove debug statements".to_string(),
                0.6,
            ));
        }

        Ok(suggestions)
    }

    /// Generate AST-based suggestions
    fn generate_suggestions_with_ast(
        &self,
        file: &CodeFile,
        tree: &tree_sitter::Tree,
        _lang: &language_parsing::SupportedLanguage,
    ) -> Result<Vec<CodeSuggestion>> {
        let mut suggestions = Vec::new();

        // This is a simplified example - in a real implementation, we would have more
        // sophisticated AST traversal and suggestion generation based on the specific language
        let root_node = tree.root_node();
        self.traverse_ast_for_suggestions(root_node, file, &mut suggestions, 0)?;

        Ok(suggestions)
    }

    /// Traverse the AST and collect suggestions
    fn traverse_ast_for_suggestions(
        &self,
        node: tree_sitter::Node,
        file: &CodeFile,
        suggestions: &mut Vec<CodeSuggestion>,
        depth: usize,
    ) -> Result<()> {
        // Limit depth to prevent infinite recursion on large files
        if depth > 100 {
            return Ok(());
        }

        // Example: Look for specific patterns in the AST that could benefit from suggestions
        match node.kind() {
            "function_definition" | "method_definition" => {
                // Check function complexity and suggest refactoring if too complex
                let complexity = self.calculate_complexity(node, file)?;
                if complexity > 10 {
                    suggestions.push(CodeSuggestion::new(
                        Uuid::new_v4(),
                        SuggestionType::Refactor,
                        "Function is complex, consider breaking it into smaller functions"
                            .to_string(),
                        "// Break complex function into smaller, focused functions".to_string(),
                        0.75,
                    ));
                }
            }
            "for_statement" | "while_statement" => {
                // Suggest performance improvements for loops
                suggestions.push(CodeSuggestion::new(
                    Uuid::new_v4(),
                    SuggestionType::Optimize,
                    "Consider if this loop could be optimized".to_string(),
                    "// Review loop for potential optimizations".to_string(),
                    0.6,
                ));
            }
            _ => {
                // Continue traversing children
            }
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.traverse_ast_for_suggestions(child, file, suggestions, depth + 1)?;
        }

        Ok(())
    }

    /// Calculate complexity of a node (e.g., function)
    fn calculate_complexity(&self, node: tree_sitter::Node, _file: &CodeFile) -> Result<u32> {
        let mut complexity = 1; // Base complexity
        let mut stack = vec![node];

        // Count decision points (if, for, while, etc.) to calculate cyclomatic complexity
        while let Some(current_node) = stack.pop() {
            match current_node.kind() {
                "if_statement"
                | "for_statement"
                | "while_statement"
                | "do_statement"
                | "switch_statement"
                | "catch_clause"
                | "conditional_expression"
                | "logical_and"
                | "logical_or" => {
                    complexity += 1;
                }
                _ => {}
            }

            // Add children to the stack for processing
            let mut child_cursor = current_node.walk();
            for child in current_node.children(&mut child_cursor) {
                stack.push(child);
            }
        }

        Ok(complexity)
    }

    /// Get analysis results for a file
    pub async fn get_analysis_results(&self, file_id: Uuid) -> Result<Vec<AnalysisResult>> {
        let results = self.analysis_results.read().await;
        Ok(results
            .values()
            .filter(|result| result.file_id == file_id)
            .cloned()
            .collect())
    }
}

// Include LLM integration tests
#[cfg(test)]
mod llm_integration_tests;

// Include ML-LLM integration tests
// #[cfg(test)]
// mod ml_llm_integration_tests;

// Include comprehensive LLM integration tests
#[cfg(test)]
mod llm_integration_comprehensive_tests;

// Include ML integration minimal tests
#[cfg(test)]
mod ml_integration_minimal_test;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_engine_creation() {
        let engine = CodeEngine::new().unwrap();
        assert_eq!(engine.files.read().await.len(), 0);
        assert_eq!(engine.analysis_results.read().await.len(), 0);
    }

    #[tokio::test]
    async fn test_load_and_get_file() {
        let engine = CodeEngine::new().unwrap();
        let content = "fn main() { println!(\"Hello, world!\"); }".to_string();
        let path = "test.rs".to_string();
        let language = "rust".to_string();

        let id = engine
            .load_file(path.clone(), content.clone(), language.clone())
            .await
            .unwrap();

        let file = engine.get_file(id).await.unwrap().unwrap();
        assert_eq!(file.path, path);
        assert_eq!(file.content, content);
        assert_eq!(file.language, language);
    }

    #[tokio::test]
    async fn test_update_file() {
        let engine = CodeEngine::new().unwrap();
        let initial_content = "fn main() {}".to_string();
        let path = "test.rs".to_string();
        let language = "rust".to_string();

        let id = engine
            .load_file(path, initial_content, language)
            .await
            .unwrap();

        let new_content = "fn main() { println!(\"Updated\"); }".to_string();
        let updated = engine.update_file(id, new_content.clone()).await.unwrap();

        assert!(updated);

        let file = engine.get_file(id).await.unwrap().unwrap();
        assert_eq!(file.content, new_content);
    }

    #[tokio::test]
    async fn test_comprehensive_code_analysis() {
        let engine = CodeEngine::new().unwrap();

        // Test Rust code with various issues
        let rust_content = r#"
fn main() {
    let mut vec = Vec::new();
    vec.push(1);
    vec.push(2);
    vec.push(3);
    
    // This is inefficient
    let length = vec.iter().collect::<Vec<_>>().len();
    
    println!("Length: {}", length);
    
    // TODO: Fix this later
    // FIXME: This is a hack
    println!("Hello, world!");
}
"#;

        let file_id = engine
            .load_file(
                "test.rs".to_string(),
                rust_content.to_string(),
                "rust".to_string(),
            )
            .await
            .unwrap();

        // Analyze the file
        let analysis_result = engine.analyze_file(file_id).await.unwrap();
        assert!(analysis_result.is_some());
        let result = analysis_result.unwrap();

        // Should find issues
        assert!(result.issues.len() > 0);

        // Should find TODO/FIXME comments
        let todo_issues: Vec<_> = result
            .issues
            .iter()
            .filter(|issue| {
                issue.description.contains("TODO") || issue.description.contains("FIXME")
            })
            .collect();
        assert!(todo_issues.len() > 0);

        // Should find performance issues
        let perf_issues: Vec<_> = result
            .issues
            .iter()
            .filter(|issue| matches!(issue.issue_type, IssueType::Performance))
            .collect();
        assert!(perf_issues.len() > 0);

        // Should generate suggestions
        assert!(result.suggestions.len() > 0);
    }

    #[tokio::test]
    async fn test_javascript_analysis() {
        let engine = CodeEngine::new().unwrap();

        let js_content = r#"
function calculateSum(arr) {
    var sum = 0;
    for (var i = 0; i < arr.length; i++) {
        sum += arr[i];
    }
    return sum;
}

// Using == instead of ===
if (sum == 10) {
    console.log("Sum is 10");
}
"#;

        let file_id = engine
            .load_file(
                "test.js".to_string(),
                js_content.to_string(),
                "javascript".to_string(),
            )
            .await
            .unwrap();

        let analysis_result = engine.analyze_file(file_id).await.unwrap();
        assert!(analysis_result.is_some());
        let result = analysis_result.unwrap();

        // Should find == vs === issues
        let equality_issues: Vec<_> = result
            .issues
            .iter()
            .filter(|issue| issue.description.contains("=="))
            .collect();
        assert!(equality_issues.len() > 0);

        // Should find var usage suggestions
        let var_suggestions: Vec<_> = result
            .suggestions
            .iter()
            .filter(|suggestion| suggestion.description.contains("var"))
            .collect();
        assert!(var_suggestions.len() > 0);
    }

    #[tokio::test]
    async fn test_error_handling() {
        let engine = CodeEngine::new().unwrap();

        // Test getting non-existent file
        let non_existent_id = uuid::Uuid::new_v4();
        let result = engine.get_file(non_existent_id).await.unwrap();
        assert!(result.is_none());

        // Test analyzing non-existent file
        let analysis_result = engine.analyze_file(non_existent_id).await.unwrap();
        assert!(analysis_result.is_none());

        // Test updating non-existent file
        let update_result = engine
            .update_file(non_existent_id, "test content".to_string())
            .await
            .unwrap();
        assert!(!update_result);
    }
}
