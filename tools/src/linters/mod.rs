//! Linters Module
//!
//! This module provides advanced linting capabilities for the OdinCode system,
//! supporting multiple programming languages with configurable rules.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;

use odincode_core::{CodeEngine, CodeFile, CodeIssue, IssueType, Severity};

/// Represents a linter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinterConfig {
    /// Language the linter targets
    pub language: String,
    /// Name of the linter
    pub name: String,
    /// Description of the linter
    pub description: String,
    /// Enabled rules
    pub enabled_rules: Vec<String>,
    /// Disabled rules
    pub disabled_rules: Vec<String>,
    /// Severity overrides for specific rules
    pub severity_overrides: HashMap<String, Severity>,
    /// Custom configuration parameters
    pub custom_params: HashMap<String, String>,
}

/// Represents a linter rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinterRule {
    /// Unique identifier for the rule
    pub id: String,
    /// Name of the rule
    pub name: String,
    /// Description of the rule
    pub description: String,
    /// Issue type this rule detects
    pub issue_type: IssueType,
    /// Default severity of the rule
    pub default_severity: Severity,
    /// Category of the rule
    pub category: String,
}

/// Main linter manager that handles multiple linters for different languages
pub struct LinterManager {
    /// Map of linter configurations
    pub linters: RwLock<HashMap<String, LinterConfig>>,
    /// Map of available rules
    pub rules: RwLock<HashMap<String, LinterRule>>,
    /// Reference to the core code engine
    pub core_engine: std::sync::Arc<CodeEngine>,
}

impl LinterManager {
    /// Create a new linter manager
    pub fn new(core_engine: std::sync::Arc<CodeEngine>) -> Self {
        Self {
            linters: RwLock::new(HashMap::new()),
            rules: RwLock::new(HashMap::new()),
            core_engine,
        }
    }

    /// Register a new linter configuration
    pub async fn register_linter(&self, config: LinterConfig) -> Result<()> {
        let language = config.language.clone();
        let mut linters = self.linters.write().await;
        linters.insert(language.clone(), config);
        drop(linters);

        info!("Registered linter for language: {}", language);
        Ok(())
    }

    /// Register a new linter rule
    pub async fn register_rule(&self, rule: LinterRule) -> Result<()> {
        let id = rule.id.clone();
        let mut rules = self.rules.write().await;
        rules.insert(id.clone(), rule);
        drop(rules);

        info!("Registered linter rule: {}", id);
        Ok(())
    }

    /// Lint a file using the appropriate linter
    pub async fn lint_file(&self, file_id: Uuid) -> Result<Vec<CodeIssue>> {
        // Get the file
        let file = self.core_engine.get_file(file_id).await?;
        if file.is_none() {
            return Err(anyhow::anyhow!("File not found: {}", file_id));
        }
        let file = file.unwrap();

        debug!("Linting file: {} (language: {})", file.path, file.language);

        // Get the appropriate linter configuration
        let linter_config = {
            let linters = self.linters.read().await;
            linters.get(&file.language).cloned()
        };

        if let Some(config) = linter_config {
            // Apply the linter to the file
            let mut issues = Vec::new();

            // For now, we'll implement basic linting based on language
            match file.language.as_str() {
                "rust" => {
                    issues.extend(self.lint_rust_file(&file, &config).await?);
                }
                "javascript" | "typescript" => {
                    issues.extend(self.lint_javascript_file(&file, &config).await?);
                }
                "python" => {
                    issues.extend(self.lint_python_file(&file, &config).await?);
                }
                _ => {
                    // For other languages, we'll do basic checks
                    issues.extend(self.lint_generic_file(&file, &config).await?);
                }
            }

            Ok(issues)
        } else {
            // If no specific linter is configured, do basic checks
            self.lint_generic_file(
                &file,
                &LinterConfig {
                    language: file.language.clone(),
                    name: "Generic Linter".to_string(),
                    description: "Basic linter for any language".to_string(),
                    enabled_rules: vec![
                        "trailing_whitespace".to_string(),
                        "line_length".to_string(),
                    ],
                    disabled_rules: vec![],
                    severity_overrides: HashMap::new(),
                    custom_params: HashMap::new(),
                },
            )
            .await
        }
    }

    /// Lint a Rust file
    async fn lint_rust_file(
        &self,
        file: &CodeFile,
        config: &LinterConfig,
    ) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = file.content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "trailing_whitespace"),
                    description: "Trailing whitespace detected".to_string(),
                    line_number: line_idx + 1,
                    column_number: line.len(),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }

            // Check for line length
            if line.len() > 100 {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "line_length"),
                    description: "Line exceeds 100 characters".to_string(),
                    line_number: line_idx + 1,
                    column_number: 100,
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }

            // Check for TODO comments
            if line.contains("TODO") || line.contains("FIXME") || line.contains("HACK") {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::BestPractice,
                    severity: self.get_rule_severity(config, "todo_comments"),
                    description: "TODO/FIXME/HACK comment found".to_string(),
                    line_number: line_idx + 1,
                    column_number: 0,
                    suggestion: Some("Address the technical debt".to_string()),
                });
            }

            // Check for inefficient patterns
            if line.contains(".collect::<Vec<_>>().len()") {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Performance,
                    severity: self.get_rule_severity(config, "inefficient_pattern"),
                    description: "Inefficient length calculation after collect".to_string(),
                    line_number: line_idx + 1,
                    column_number: 0,
                    suggestion: Some("Use .count() or .len() directly on iterator".to_string()),
                });
            }
        }

        Ok(issues)
    }

    /// Lint a JavaScript/TypeScript file
    async fn lint_javascript_file(
        &self,
        file: &CodeFile,
        config: &LinterConfig,
    ) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = file.content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "trailing_whitespace"),
                    description: "Trailing whitespace detected".to_string(),
                    line_number: line_idx + 1,
                    column_number: line.len(),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }

            // Check for line length
            if line.len() > 100 {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "line_length"),
                    description: "Line exceeds 100 characters".to_string(),
                    line_number: line_idx + 1,
                    column_number: 100,
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }

            // Check for loose equality
            if line.contains("==") && !line.contains("===\"") && !line.contains("!==") {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::PotentialBug,
                    severity: self.get_rule_severity(config, "loose_equality"),
                    description: "Use of == instead of === for comparison".to_string(),
                    line_number: line_idx + 1,
                    column_number: 0,
                    suggestion: Some("Use === for comparison to avoid type coercion".to_string()),
                });
            }

            // Check for var usage
            if line.contains("var ") {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::BestPractice,
                    severity: self.get_rule_severity(config, "var_usage"),
                    description: "Use of 'var' instead of 'let' or 'const'".to_string(),
                    line_number: line_idx + 1,
                    column_number: 0,
                    suggestion: Some("Use 'let' or 'const' instead of 'var'".to_string()),
                });
            }
        }

        Ok(issues)
    }

    /// Lint a Python file
    async fn lint_python_file(
        &self,
        file: &CodeFile,
        config: &LinterConfig,
    ) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = file.content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "trailing_whitespace"),
                    description: "Trailing whitespace detected".to_string(),
                    line_number: line_idx + 1,
                    column_number: line.len(),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }

            // Check for line length
            if line.len() > 100 {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "line_length"),
                    description: "Line exceeds 100 characters".to_string(),
                    line_number: line_idx + 1,
                    column_number: 100,
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }

            // Check for print statements (not in function definitions)
            if line.trim_start().starts_with("print(") {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::BestPractice,
                    severity: self.get_rule_severity(config, "debug_print"),
                    description: "Debug print statement found".to_string(),
                    line_number: line_idx + 1,
                    column_number: 0,
                    suggestion: Some("Remove debug print statements before production".to_string()),
                });
            }
        }

        Ok(issues)
    }

    /// Lint a file with generic checks
    async fn lint_generic_file(
        &self,
        file: &CodeFile,
        config: &LinterConfig,
    ) -> Result<Vec<CodeIssue>> {
        let mut issues = Vec::new();
        let lines: Vec<&str> = file.content.lines().collect();

        for (line_idx, line) in lines.iter().enumerate() {
            // Check for trailing whitespace
            if line.ends_with(' ') || line.ends_with('\t') {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "trailing_whitespace"),
                    description: "Trailing whitespace detected".to_string(),
                    line_number: line_idx + 1,
                    column_number: line.len(),
                    suggestion: Some("Remove trailing whitespace".to_string()),
                });
            }

            // Check for line length
            if line.len() > 100 {
                issues.push(CodeIssue {
                    id: Uuid::new_v4(),
                    issue_type: IssueType::Style,
                    severity: self.get_rule_severity(config, "line_length"),
                    description: "Line exceeds 100 characters".to_string(),
                    line_number: line_idx + 1,
                    column_number: 100,
                    suggestion: Some("Break line into multiple lines".to_string()),
                });
            }
        }

        Ok(issues)
    }

    /// Get the severity for a rule, considering overrides
    fn get_rule_severity(&self, config: &LinterConfig, rule_id: &str) -> Severity {
        // Check if there's an override for this rule
        if let Some(severity) = config.severity_overrides.get(rule_id) {
            return severity.clone();
        }

        // If the rule is disabled, return Low severity (effectively ignoring it)
        if config.disabled_rules.contains(&rule_id.to_string()) {
            return Severity::Low;
        }

        // Default severity for all rules is Medium
        Severity::Medium
    }

    /// Get all registered linters
    pub async fn get_all_linters(&self) -> Result<Vec<LinterConfig>> {
        let linters = self.linters.read().await;
        let result: Vec<LinterConfig> = linters.values().cloned().collect();
        Ok(result)
    }

    /// Get all registered rules
    pub async fn get_all_rules(&self) -> Result<Vec<LinterRule>> {
        let rules = self.rules.read().await;
        let result: Vec<LinterRule> = rules.values().cloned().collect();
        Ok(result)
    }
}
