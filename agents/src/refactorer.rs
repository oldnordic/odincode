//! Refactorer Agent
//!
//! This module implements the Refactorer agent that uses LLM integration
//! to analyze code and provide intelligent refactoring suggestions.

use anyhow::Result;
use odincode_core::{CodeEngine, CodeFile, CodeSuggestion, Severity, SuggestionType};
use odincode_ltmc::LTMManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};

use crate::llm_integration::{
    LLMIntegrationManager, LLMMessage, LLMProvider, LLMRequest, LLMRequestConfig,
};
use crate::models::Agent;

/// Refactoring request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringRequest {
    /// The code file to analyze
    pub file: CodeFile,
    /// Specific refactoring type (optional)
    pub refactoring_type: Option<RefactoringType>,
    /// Target area for refactoring (line numbers or function names)
    pub target_area: Option<String>,
    /// Refactoring goals or constraints
    pub goals: Option<String>,
    /// Language of the code
    pub language: String,
}

/// Types of refactoring operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RefactoringType {
    /// Extract method/function
    ExtractMethod,
    /// Rename variable/function/class
    Rename,
    /// Inline method/function
    InlineMethod,
    /// Extract interface/trait
    ExtractInterface,
    /// Move method/function
    MoveMethod,
    /// Replace conditional with polymorphism
    ReplaceConditionalWithPolymorphism,
    /// Introduce parameter object
    IntroduceParameterObject,
    /// Remove duplicate code
    RemoveDuplicates,
    /// Simplify conditional logic
    SimplifyConditional,
    /// Optimize performance
    OptimizePerformance,
    /// Improve error handling
    ImproveErrorHandling,
    /// Enhance readability
    EnhanceReadability,
    /// General refactoring analysis
    General,
}

impl std::fmt::Display for RefactoringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefactoringType::ExtractMethod => write!(f, "extract method"),
            RefactoringType::Rename => write!(f, "rename"),
            RefactoringType::InlineMethod => write!(f, "inline method"),
            RefactoringType::ExtractInterface => write!(f, "extract interface"),
            RefactoringType::MoveMethod => write!(f, "move method"),
            RefactoringType::ReplaceConditionalWithPolymorphism => {
                write!(f, "replace conditional with polymorphism")
            }
            RefactoringType::IntroduceParameterObject => write!(f, "introduce parameter object"),
            RefactoringType::RemoveDuplicates => write!(f, "remove duplicates"),
            RefactoringType::SimplifyConditional => write!(f, "simplify conditional"),
            RefactoringType::OptimizePerformance => write!(f, "optimize performance"),
            RefactoringType::ImproveErrorHandling => write!(f, "improve error handling"),
            RefactoringType::EnhanceReadability => write!(f, "enhance readability"),
            RefactoringType::General => write!(f, "general refactoring"),
        }
    }
}

/// Refactoring suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSuggestion {
    /// Type of refactoring
    pub refactoring_type: RefactoringType,
    /// Original code snippet
    pub original_code: String,
    /// Refactored code snippet
    pub refactored_code: String,
    /// Description of the refactoring
    pub description: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Expected benefits
    pub benefits: Vec<String>,
    /// Potential risks or side effects
    pub risks: Vec<String>,
    /// Line numbers affected
    pub affected_lines: Vec<usize>,
    /// Implementation steps
    pub implementation_steps: Vec<String>,
}

/// Refactoring analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringAnalysis {
    /// Overall code quality score
    pub quality_score: f32,
    /// Identified refactoring opportunities
    pub opportunities: Vec<RefactoringSuggestion>,
    /// Code complexity metrics
    pub complexity_metrics: ComplexityMetrics,
    /// Code smells detected
    pub code_smells: Vec<CodeSmell>,
    /// Recommendations summary
    pub summary: String,
}

/// Code complexity metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity
    pub cyclomatic_complexity: f32,
    /// Cognitive complexity
    pub cognitive_complexity: f32,
    /// Lines of code
    pub lines_of_code: usize,
    /// Number of functions/methods
    pub function_count: usize,
    /// Maximum nesting depth
    pub max_nesting_depth: usize,
    /// Maintainability index
    pub maintainability_index: f32,
}

/// Code smell detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSmell {
    /// Type of code smell
    pub smell_type: String,
    /// Description
    pub description: String,
    /// Severity level
    pub severity: Severity,
    /// Location (line numbers)
    pub location: Vec<usize>,
    /// Suggested fix
    pub suggested_fix: String,
}

/// Refactorer Agent
pub struct RefactorerAgent {
    /// Agent instance
    agent: Agent,
    /// LLM integration manager
    llm_manager: Arc<LLMIntegrationManager>,
    /// Core code engine
    core_engine: Arc<CodeEngine>,
    /// LTMC manager
    ltmc_manager: Arc<LTMManager>,
}

impl RefactorerAgent {
    /// Create a new Refactorer agent
    pub fn new(
        llm_manager: Arc<LLMIntegrationManager>,
        core_engine: Arc<CodeEngine>,
        ltmc_manager: Arc<LTMManager>,
    ) -> Self {
        let agent = Agent {
            id: uuid::Uuid::new_v4(),
            agent_type: crate::models::AgentType::Refactorer,
            name: "Code Refactorer".to_string(),
            description: "Intelligent code analysis and refactoring suggestions".to_string(),
            created: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            capabilities: vec![
                "code_analysis".to_string(),
                "refactoring_suggestions".to_string(),
                "complexity_analysis".to_string(),
                "code_smell_detection".to_string(),
                "performance_optimization".to_string(),
                "readability_improvement".to_string(),
            ],
            confidence_threshold: 0.8,
        };

        Self {
            agent,
            llm_manager,
            core_engine,
            ltmc_manager,
        }
    }

    /// Analyze code and provide refactoring suggestions
    pub async fn analyze_code(&self, request: RefactoringRequest) -> Result<RefactoringAnalysis> {
        info!(
            "Analyzing code for refactoring opportunities in {}",
            request.file.path
        );

        // Build the analysis prompt for LLM
        let prompt = self.build_analysis_prompt(&request).await?;

        // Get default model for OpenAI
        let model = self
            .llm_manager
            .get_default_model(&LLMProvider::OpenAI)
            .await
            .ok_or_else(|| anyhow::anyhow!("No default model available"))?;

        // Create LLM request
        let llm_request = LLMRequest {
            model: model.name.clone(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: prompt,
                name: None,
            }],
            config: LLMRequestConfig {
                max_tokens: Some(2000),
                temperature: 0.2,
                top_p: Some(0.9),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: None,
            },
            request_id: None,
        };

        // Analyze code using LLM
        let llm_response = self.llm_manager.send_request(llm_request).await?;

        // Parse and structure the analysis
        let analysis = self
            .parse_analysis_response(llm_response.content, &request)
            .await?;

        // Update agent activity
        self.update_activity().await;

        Ok(analysis)
    }

    /// Generate specific refactoring suggestion
    pub async fn generate_refactoring(
        &self,
        request: RefactoringRequest,
    ) -> Result<Vec<RefactoringSuggestion>> {
        info!(
            "Generating refactoring suggestions for {}",
            request.file.path
        );

        // Build the refactoring prompt for LLM
        let prompt = self.build_refactoring_prompt(&request).await?;

        // Get default model for OpenAI
        let model = self
            .llm_manager
            .get_default_model(&LLMProvider::OpenAI)
            .await
            .ok_or_else(|| anyhow::anyhow!("No default model available"))?;

        // Create LLM request
        let llm_request = LLMRequest {
            model: model.name.clone(),
            messages: vec![LLMMessage {
                role: "user".to_string(),
                content: prompt,
                name: None,
            }],
            config: LLMRequestConfig {
                max_tokens: Some(1500),
                temperature: 0.3,
                top_p: Some(0.9),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: None,
            },
            request_id: None,
        };

        // Generate refactoring using LLM
        let llm_response = self.llm_manager.send_request(llm_request).await?;

        // Parse and structure the refactoring suggestions
        let suggestions = self
            .parse_refactoring_response(llm_response.content, &request)
            .await?;

        // Update agent activity
        self.update_activity().await;

        Ok(suggestions)
    }

    /// Build the analysis prompt for LLM
    async fn build_analysis_prompt(&self, request: &RefactoringRequest) -> Result<String> {
        let mut prompt = format!(
            "You are an expert {} code analyst and refactoring specialist. Analyze the following code and provide comprehensive refactoring recommendations:\n\n",
            request.language
        );

        // Add file context
        prompt.push_str(&format!("File: {}\n", request.file.path));
        prompt.push_str(&format!("Language: {}\n\n", request.language));

        // Add code content
        prompt.push_str("Code:\n");
        prompt.push_str("```\n");
        prompt.push_str(&request.file.content);
        prompt.push_str("\n```\n\n");

        // Add specific refactoring type if provided
        if let Some(refactoring_type) = &request.refactoring_type {
            prompt.push_str(&format!("Focus on: {} refactoring\n\n", refactoring_type));
        }

        // Add target area if specified
        if let Some(target_area) = &request.target_area {
            prompt.push_str(&format!("Target area: {}\n\n", target_area));
        }

        // Add goals if provided
        if let Some(goals) = &request.goals {
            prompt.push_str(&format!("Goals: {}\n\n", goals));
        }

        // Add analysis requirements
        prompt.push_str("Please provide a comprehensive analysis including:\n");
        prompt.push_str("1. Overall code quality score (0.0-1.0)\n");
        prompt.push_str("2. Specific refactoring opportunities with:\n");
        prompt.push_str("   - Type of refactoring\n");
        prompt.push_str("   - Original vs refactored code\n");
        prompt.push_str("   - Confidence score (0.0-1.0)\n");
        prompt.push_str("   - Benefits and risks\n");
        prompt.push_str("   - Implementation steps\n");
        prompt.push_str("3. Code complexity metrics (cyclomatic, cognitive, etc.)\n");
        prompt.push_str("4. Code smells detected with severity and location\n");
        prompt.push_str("5. Summary of recommendations\n\n");

        // Add best practices reminder
        prompt.push_str("Consider these best practices:\n");
        prompt.push_str("- SOLID principles\n");
        prompt.push_str("- DRY (Don't Repeat Yourself)\n");
        prompt.push_str("- KISS (Keep It Simple, Stupid)\n");
        prompt.push_str("- YAGNI (You Aren't Gonna Need It)\n");
        prompt.push_str("- Clean Code principles\n");
        prompt.push_str("- Language-specific idioms and patterns\n");

        Ok(prompt)
    }

    /// Build the refactoring prompt for LLM
    async fn build_refactoring_prompt(&self, request: &RefactoringRequest) -> Result<String> {
        let mut prompt = format!(
            "You are an expert {} code refactoring specialist. Generate specific refactoring suggestions for the following code:\n\n",
            request.language
        );

        // Add file context
        prompt.push_str(&format!("File: {}\n", request.file.path));
        prompt.push_str(&format!("Language: {}\n\n", request.language));

        // Add code content
        prompt.push_str("Code:\n");
        prompt.push_str("```\n");
        prompt.push_str(&request.file.content);
        prompt.push_str("\n```\n\n");

        // Add specific refactoring type if provided
        if let Some(refactoring_type) = &request.refactoring_type {
            prompt.push_str(&format!("Refactoring type: {}\n\n", refactoring_type));
        }

        // Add target area if specified
        if let Some(target_area) = &request.target_area {
            prompt.push_str(&format!("Target area: {}\n\n", target_area));
        }

        // Add goals if provided
        if let Some(goals) = &request.goals {
            prompt.push_str(&format!("Goals: {}\n\n", goals));
        }

        // Add refactoring requirements
        prompt.push_str("Please provide specific refactoring suggestions including:\n");
        prompt.push_str("1. Type of refactoring\n");
        prompt.push_str("2. Original code snippet\n");
        prompt.push_str("3. Refactored code snippet\n");
        prompt.push_str("4. Description of changes\n");
        prompt.push_str("5. Confidence score (0.0-1.0)\n");
        prompt.push_str("6. Expected benefits\n");
        prompt.push_str("7. Potential risks\n");
        prompt.push_str("8. Affected line numbers\n");
        prompt.push_str("9. Implementation steps\n\n");

        Ok(prompt)
    }

    /// Parse LLM analysis response and structure it
    async fn parse_analysis_response(
        &self,
        llm_response: String,
        request: &RefactoringRequest,
    ) -> Result<RefactoringAnalysis> {
        // For now, create a basic analysis structure
        // In a real implementation, this would parse the LLM response more intelligently

        let quality_score = self.calculate_quality_score(&request.file.content).await?;
        let complexity_metrics = self
            .calculate_complexity_metrics(&request.file.content)
            .await?;
        let code_smells = self.detect_code_smells(&request.file.content).await?;
        let opportunities = self.generate_sample_opportunities(request).await?;
        let summary = self
            .generate_summary(&quality_score, &opportunities, &code_smells)
            .await?;

        Ok(RefactoringAnalysis {
            quality_score,
            opportunities,
            complexity_metrics,
            code_smells,
            summary,
        })
    }

    /// Parse LLM refactoring response and structure it
    async fn parse_refactoring_response(
        &self,
        _llm_response: String,
        request: &RefactoringRequest,
    ) -> Result<Vec<RefactoringSuggestion>> {
        // For now, generate sample suggestions
        // In a real implementation, this would parse the LLM response more intelligently

        let mut suggestions = Vec::new();

        // Generate a sample refactoring suggestion
        if request.file.content.len() > 100 {
            let suggestion = RefactoringSuggestion {
                refactoring_type: request
                    .refactoring_type
                    .clone()
                    .unwrap_or(RefactoringType::General),
                original_code: request.file.content.chars().take(100).collect::<String>() + "...",
                refactored_code: "// Refactored code would appear here\n".to_string(),
                description: "Sample refactoring suggestion based on code analysis".to_string(),
                confidence: 0.8,
                benefits: vec![
                    "Improved readability".to_string(),
                    "Better maintainability".to_string(),
                ],
                risks: vec!["Potential breaking changes".to_string()],
                affected_lines: vec![1, 2, 3],
                implementation_steps: vec![
                    "Identify the code section to refactor".to_string(),
                    "Apply the refactoring pattern".to_string(),
                    "Test the changes".to_string(),
                ],
            };
            suggestions.push(suggestion);
        }

        Ok(suggestions)
    }

    /// Calculate code quality score
    async fn calculate_quality_score(&self, code: &str) -> Result<f32> {
        let mut score = 0.5_f32; // Base score

        // Boost score for well-structured code
        if code.lines().count() > 10 {
            score += 0.1;
        }

        // Check for proper formatting (basic heuristics)
        if code.contains('\n') && code.lines().all(|line| line.len() < 120) {
            score += 0.1;
        }

        // Check for function/method definitions
        if code.contains("fn ") || code.contains("def ") || code.contains("function ") {
            score += 0.1;
        }

        // Check for proper error handling patterns
        if code.contains("Result<") || code.contains("try ") || code.contains("catch ") {
            score += 0.1;
        }

        // Cap score at 1.0
        Ok(score.min(1.0))
    }

    /// Calculate complexity metrics
    async fn calculate_complexity_metrics(&self, code: &str) -> Result<ComplexityMetrics> {
        let lines = code.lines().count();
        let function_count = code.matches("fn ").count()
            + code.matches("def ").count()
            + code.matches("function ").count();

        // Simple cyclomatic complexity estimation
        let cyclomatic_complexity = 1.0
            + code.matches("if ").count() as f32
            + code.matches("while ").count() as f32
            + code.matches("for ").count() as f32
            + code.matches("match ").count() as f32
            + code.matches("switch ").count() as f32;

        // Simple cognitive complexity estimation
        let cognitive_complexity = cyclomatic_complexity * 1.5;

        // Simple nesting depth calculation
        let max_nesting_depth = code
            .lines()
            .map(|line| line.chars().take_while(|&c| c == ' ' || c == '\t').count() / 4)
            .max()
            .unwrap_or(0);

        // Simple maintainability index calculation
        let maintainability_index =
            (100.0 - cyclomatic_complexity * 2.0 - lines as f32 * 0.1).max(0.0);

        Ok(ComplexityMetrics {
            cyclomatic_complexity,
            cognitive_complexity,
            lines_of_code: lines,
            function_count,
            max_nesting_depth,
            maintainability_index,
        })
    }

    /// Detect code smells
    async fn detect_code_smells(&self, code: &str) -> Result<Vec<CodeSmell>> {
        let mut smells = Vec::new();

        // Detect long functions
        let lines: Vec<&str> = code.lines().collect();
        for (i, chunk) in lines.chunks(50).enumerate() {
            if chunk.len() >= 50 {
                smells.push(CodeSmell {
                    smell_type: "Long Function".to_string(),
                    description: "Function is too long and should be broken down".to_string(),
                    severity: Severity::Medium,
                    location: vec![i * 50 + 1, (i + 1) * 50],
                    suggested_fix: "Extract smaller functions with single responsibilities"
                        .to_string(),
                });
            }
        }

        // Detect deep nesting
        for (i, line) in lines.iter().enumerate() {
            let indent = line.chars().take_while(|&c| c == ' ' || c == '\t').count();
            if indent > 16 {
                smells.push(CodeSmell {
                    smell_type: "Deep Nesting".to_string(),
                    description: "Code is nested too deeply".to_string(),
                    severity: Severity::High,
                    location: vec![i + 1],
                    suggested_fix: "Extract nested logic into separate functions".to_string(),
                });
            }
        }

        // Detect duplicate code (simple version)
        let code_lines: Vec<String> = lines.iter().map(|s| s.trim().to_string()).collect();
        for i in 0..code_lines.len() {
            for j in i + 1..code_lines.len() {
                if code_lines[i] == code_lines[j] && !code_lines[i].is_empty() {
                    smells.push(CodeSmell {
                        smell_type: "Duplicate Code".to_string(),
                        description: "Code is duplicated".to_string(),
                        severity: Severity::Medium,
                        location: vec![i + 1, j + 1],
                        suggested_fix: "Extract common code into a shared function".to_string(),
                    });
                }
            }
        }

        Ok(smells)
    }

    /// Generate sample refactoring opportunities
    async fn generate_sample_opportunities(
        &self,
        request: &RefactoringRequest,
    ) -> Result<Vec<RefactoringSuggestion>> {
        let mut opportunities = Vec::new();

        // Generate a sample opportunity
        if request.file.content.len() > 50 {
            let opportunity = RefactoringSuggestion {
                refactoring_type: RefactoringType::EnhanceReadability,
                original_code: request.file.content.chars().take(50).collect::<String>() + "...",
                refactored_code: "// Improved readability version\n".to_string(),
                description: "Improve code readability by applying clean code principles"
                    .to_string(),
                confidence: 0.75,
                benefits: vec![
                    "Better maintainability".to_string(),
                    "Easier understanding".to_string(),
                ],
                risks: vec!["Minor style changes".to_string()],
                affected_lines: vec![1, 2, 3],
                implementation_steps: vec![
                    "Review code for readability issues".to_string(),
                    "Apply consistent formatting".to_string(),
                    "Add meaningful variable names".to_string(),
                ],
            };
            opportunities.push(opportunity);
        }

        Ok(opportunities)
    }

    /// Generate analysis summary
    async fn generate_summary(
        &self,
        quality_score: &f32,
        opportunities: &[RefactoringSuggestion],
        code_smells: &[CodeSmell],
    ) -> Result<String> {
        let mut summary = format!("Code Quality Analysis Summary:\n");
        summary.push_str(&format!("- Overall Quality Score: {:.2}\n", quality_score));
        summary.push_str(&format!(
            "- Refactoring Opportunities: {}\n",
            opportunities.len()
        ));
        summary.push_str(&format!("- Code Smells Detected: {}\n", code_smells.len()));

        if !code_smells.is_empty() {
            summary.push_str("\nPriority Issues:\n");
            for smell in code_smells.iter().take(3) {
                summary.push_str(&format!(
                    "- {}: {} (Severity: {:?})\n",
                    smell.smell_type, smell.description, smell.severity
                ));
            }
        }

        if !opportunities.is_empty() {
            summary.push_str("\nTop Refactoring Opportunities:\n");
            for opportunity in opportunities.iter().take(2) {
                summary.push_str(&format!(
                    "- {}: {:.2} confidence\n",
                    opportunity.refactoring_type, opportunity.confidence
                ));
            }
        }

        Ok(summary)
    }

    /// Update agent activity timestamp
    async fn update_activity(&self) {
        debug!("RefactorerAgent activity updated");
    }

    /// Get agent information
    pub fn get_agent(&self) -> &Agent {
        &self.agent
    }

    /// Convert refactoring suggestion to core suggestion
    pub fn to_suggestion(
        &self,
        suggestion: RefactoringSuggestion,
        _file_path: &str,
    ) -> CodeSuggestion {
        CodeSuggestion {
            id: uuid::Uuid::new_v4(),
            title: "Refactoring".to_string(),
            suggestion_type: SuggestionType::Refactor,
            description: suggestion.description,
            code_snippet: Some(suggestion.refactored_code),
            confidence: suggestion.confidence,
            file_path: _file_path.to_string(),
            line_number: None,
            severity: odincode_core::Severity::Info,
            auto_fixable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_refactorer_creation() {
        let llm_manager = Arc::new(LLMIntegrationManager::new());
        let core_engine = Arc::new(CodeEngine::new());
        let ltmc_manager = Arc::new(LTMManager::new());

        let agent = RefactorerAgent::new(llm_manager, core_engine, ltmc_manager);

        assert_eq!(agent.get_agent().name, "Code Refactorer");
        assert_eq!(
            agent.get_agent().agent_type,
            crate::models::AgentType::Refactorer
        );
    }

    #[tokio::test]
    async fn test_calculate_quality_score() {
        let agent = RefactorerAgent::new(
            Arc::new(LLMIntegrationManager::new()),
            Arc::new(CodeEngine::new()),
            Arc::new(LTMManager::new()),
        );

        let code = r#"
fn main() {
    println!("Hello, world!");
}
"#;
        let score = agent.calculate_quality_score(code).await.unwrap();
        assert!(score > 0.5);
    }

    #[tokio::test]
    async fn test_calculate_complexity_metrics() {
        let agent = RefactorerAgent::new(
            Arc::new(LLMIntegrationManager::new()),
            Arc::new(CodeEngine::new()),
            Arc::new(LTMManager::new()),
        );

        let code = r#"
fn main() {
    if true {
        println!("Hello");
    }
}
"#;
        let metrics = agent.calculate_complexity_metrics(code).await.unwrap();
        assert!(metrics.cyclomatic_complexity > 1.0);
        assert!(metrics.lines_of_code > 0);
    }

    #[tokio::test]
    async fn test_detect_code_smells() {
        let agent = RefactorerAgent::new(
            Arc::new(LLMIntegrationManager::new()),
            Arc::new(CodeEngine::new()),
            Arc::new(LTMManager::new()),
        );

        // Test with deep nesting to trigger a code smell
        let code = r#"
fn main() {
    if true {
        if true {
            if true {
                if true {
                    if true {
                        println!("Deep nesting");
                    }
                }
            }
        }
    }
}
"#;
        let smells = agent.detect_code_smells(code).await.unwrap();
        assert!(!smells.is_empty());
    }

    #[test]
    fn test_refactoring_type_display() {
        assert_eq!(RefactoringType::ExtractMethod.to_string(), "extract method");
        assert_eq!(RefactoringType::Rename.to_string(), "rename");
        assert_eq!(RefactoringType::General.to_string(), "general refactoring");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Low.to_string(), "low");
        assert_eq!(Severity::High.to_string(), "high");
        assert_eq!(Severity::Critical.to_string(), "critical");
    }
}
