//! Code Generator Agent
//!
//! This module implements the Code Generator agent that uses LLM integration
//! to generate intelligent code suggestions and completions.

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

/// Code generation request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationRequest {
    /// The code file context
    pub file: CodeFile,
    /// Cursor position for generation
    pub cursor_position: usize,
    /// Generation context (previous lines, function signatures, etc.)
    pub context: String,
    /// Language of the code
    pub language: String,
    /// Generation type (completion, function, class, etc.)
    pub generation_type: GenerationType,
    /// Optional constraints or requirements
    pub constraints: Option<String>,
}

/// Types of code generation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GenerationType {
    /// Code completion at cursor
    Completion,
    /// Function generation
    Function,
    /// Class/Struct generation
    Class,
    /// Method implementation
    Method,
    /// Full file generation
    File,
    /// Refactoring suggestion
    Refactor,
    /// Test generation
    Test,
}

impl std::fmt::Display for GenerationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerationType::Completion => write!(f, "completion"),
            GenerationType::Function => write!(f, "function"),
            GenerationType::Class => write!(f, "class"),
            GenerationType::Method => write!(f, "method"),
            GenerationType::File => write!(f, "file"),
            GenerationType::Refactor => write!(f, "refactoring"),
            GenerationType::Test => write!(f, "test"),
        }
    }
}

/// Code generation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenerationResponse {
    /// Generated code
    pub generated_code: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Explanation of the generation
    pub explanation: String,
    /// Suggested insertion position
    pub insertion_position: usize,
    /// Additional suggestions or alternatives
    pub alternatives: Vec<String>,
}

/// Code Generator Agent
pub struct CodeGeneratorAgent {
    /// Agent instance
    agent: Agent,
    /// LLM integration manager
    llm_manager: Arc<LLMIntegrationManager>,
    /// Core code engine
    core_engine: Arc<CodeEngine>,
    /// LTMC manager
    ltmc_manager: Arc<LTMManager>,
}

impl CodeGeneratorAgent {
    /// Create a new Code Generator agent
    pub fn new(
        llm_manager: Arc<LLMIntegrationManager>,
        core_engine: Arc<CodeEngine>,
        ltmc_manager: Arc<LTMManager>,
    ) -> Self {
        let agent = Agent {
            id: uuid::Uuid::new_v4(),
            agent_type: crate::models::AgentType::CodeGenerator,
            name: "Code Generator".to_string(),
            description: "Intelligent code generation using LLM integration".to_string(),
            created: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            capabilities: vec![
                "code_completion".to_string(),
                "function_generation".to_string(),
                "class_generation".to_string(),
                "test_generation".to_string(),
                "refactoring".to_string(),
            ],
            confidence_threshold: 0.7,
        };

        Self {
            agent,
            llm_manager,
            core_engine,
            ltmc_manager,
        }
    }

    /// Generate code based on the request
    pub async fn generate_code(
        &self,
        request_param: CodeGenerationRequest,
    ) -> Result<CodeGenerationResponse> {
        info!(
            "Generating code for {:?} in {}",
            request_param.generation_type, request_param.language
        );

        // Build the prompt for LLM
        let prompt = self.build_generation_prompt(&request_param).await?;

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
                max_tokens: Some(1000),
                temperature: 0.3,
                top_p: Some(0.9),
                frequency_penalty: Some(0.0),
                presence_penalty: Some(0.0),
                stop_sequences: None,
            },
            request_id: None,
        };

        // Generate code using LLM
        let llm_response = self.llm_manager.send_request(llm_request).await?;

        // Parse and structure the response
        let response = self
            .parse_llm_response(llm_response.content, &request_param)
            .await?;

        // Update agent activity
        self.update_activity().await;

        Ok(response)
    }

    /// Build the generation prompt for LLM
    async fn build_generation_prompt(&self, request: &CodeGenerationRequest) -> Result<String> {
        let mut prompt = format!(
            "You are an expert {} programmer. Generate code based on the following context:\n\n",
            request.language
        );

        // Add file context
        prompt.push_str(&format!("File: {}\n", request.file.path));
        prompt.push_str(&format!("Language: {}\n\n", request.language));

        // Add code context
        if !request.context.is_empty() {
            prompt.push_str("Context:\n");
            prompt.push_str(&request.context);
            prompt.push_str("\n\n");
        }

        // Add generation type specific instructions
        match request.generation_type {
            GenerationType::Completion => {
                prompt.push_str("Generate code completion at the cursor position. ");
                prompt.push_str(
                    "Provide only the code that should be inserted, without explanations.\n",
                );
            }
            GenerationType::Function => {
                prompt.push_str("Generate a complete function implementation. ");
                prompt.push_str("Include proper error handling and documentation.\n");
            }
            GenerationType::Class => {
                prompt.push_str("Generate a complete class/struct implementation. ");
                prompt.push_str("Include proper encapsulation and methods.\n");
            }
            GenerationType::Method => {
                prompt.push_str("Generate a method implementation for the class. ");
                prompt.push_str("Follow the existing code style and patterns.\n");
            }
            GenerationType::File => {
                prompt.push_str("Generate a complete file implementation. ");
                prompt.push_str("Include all necessary imports and structure.\n");
            }
            GenerationType::Refactor => {
                prompt.push_str("Generate refactored code that improves quality. ");
                prompt.push_str("Focus on readability, performance, and best practices.\n");
            }
            GenerationType::Test => {
                prompt.push_str("Generate comprehensive test cases. ");
                prompt.push_str("Include unit tests, edge cases, and integration tests.\n");
            }
        }

        // Add constraints if provided
        if let Some(constraints) = &request.constraints {
            prompt.push_str(&format!("\nConstraints: {}\n", constraints));
        }

        // Add best practices reminder
        prompt.push_str("\nBest practices to follow:\n");
        prompt.push_str("- Use clear and descriptive names\n");
        prompt.push_str("- Include proper error handling\n");
        prompt.push_str("- Follow language-specific conventions\n");
        prompt.push_str("- Add appropriate comments\n");
        prompt.push_str("- Consider performance implications\n");

        Ok(prompt)
    }

    /// Parse LLM response and structure it
    async fn parse_llm_response(
        &self,
        llm_response: String,
        request: &CodeGenerationRequest,
    ) -> Result<CodeGenerationResponse> {
        // Extract generated code (remove explanations if any)
        let generated_code = self.extract_code_from_response(&llm_response);

        // Calculate confidence based on response quality
        let confidence = self.calculate_confidence(&generated_code, request).await?;

        // Generate explanation
        let explanation = self.generate_explanation(&generated_code, request).await?;

        // Determine insertion position
        let insertion_position = request.cursor_position;

        // Generate alternatives
        let alternatives = self.generate_alternatives(&generated_code, request).await?;

        Ok(CodeGenerationResponse {
            generated_code,
            confidence,
            explanation,
            insertion_position,
            alternatives,
        })
    }

    /// Extract code from LLM response
    fn extract_code_from_response(&self, response: &str) -> String {
        // Remove markdown code blocks if present
        let response = response.trim();

        if response.starts_with("```") {
            // Find the end of the first line (language specifier)
            if let Some(newline_pos) = response.find('\n') {
                let after_first_line = &response[newline_pos + 1..];

                // Find the closing code block
                if let Some(closing_pos) = after_first_line.rfind("```") {
                    return after_first_line[..closing_pos].trim().to_string();
                } else {
                    return after_first_line.trim().to_string();
                }
            }
        }

        // If no code blocks, return the response as is
        response.to_string()
    }

    /// Calculate confidence score for the generated code
    async fn calculate_confidence(
        &self,
        code: &str,
        request: &CodeGenerationRequest,
    ) -> Result<f32> {
        let mut confidence = 0.7_f32; // Base confidence

        // Boost confidence for well-structured code
        if code.lines().count() > 0 && !code.trim().is_empty() {
            confidence += 0.1;
        }

        // Check for language-specific patterns
        match request.language.to_lowercase().as_str() {
            "rust" => {
                if code.contains("fn ") || code.contains("struct ") || code.contains("impl ") {
                    confidence += 0.1;
                }
            }
            "python" => {
                if code.contains("def ") || code.contains("class ") {
                    confidence += 0.1;
                }
            }
            "javascript" | "typescript" => {
                if code.contains("function") || code.contains("const ") || code.contains("let ") {
                    confidence += 0.1;
                }
            }
            _ => {}
        }

        // Cap confidence at 1.0
        Ok(confidence.min(1.0))
    }

    /// Generate explanation for the code
    async fn generate_explanation(
        &self,
        _code: &str,
        request: &CodeGenerationRequest,
    ) -> Result<String> {
        let mut explanation = format!(
            "Generated {} code for {:?}",
            request.language, request.generation_type
        );

        // Add specific details based on generation type
        match request.generation_type {
            GenerationType::Function => {
                explanation.push_str(" with proper function signature and implementation");
            }
            GenerationType::Class => {
                explanation.push_str(" with complete class structure and methods");
            }
            GenerationType::Test => {
                explanation.push_str(" with comprehensive test coverage");
            }
            _ => {
                explanation.push_str(" based on the provided context");
            }
        }

        Ok(explanation)
    }

    /// Generate alternative code suggestions
    async fn generate_alternatives(
        &self,
        code: &str,
        request: &CodeGenerationRequest,
    ) -> Result<Vec<String>> {
        let mut alternatives = Vec::new();

        // For now, generate a simple alternative
        if request.generation_type == GenerationType::Function {
            let alt_code = format!("// Alternative implementation\n{}", code);
            alternatives.push(alt_code);
        }

        Ok(alternatives)
    }

    /// Update agent activity timestamp
    async fn update_activity(&self) {
        // This would update the agent's last_activity timestamp
        // For now, we'll just log it
        debug!("CodeGeneratorAgent activity updated");
    }

    /// Get agent information
    pub fn get_agent(&self) -> &Agent {
        &self.agent
    }

    /// Convert generation response to core suggestion
    pub fn to_suggestion(
        &self,
        response: CodeGenerationResponse,
        _file_path: &str,
    ) -> CodeSuggestion {
        CodeSuggestion {
            id: uuid::Uuid::new_v4(),
            suggestion_type: SuggestionType::Feature,
            title: "Generated Code".to_string(),
            description: response.explanation,
            code_snippet: Some(response.generated_code),
            confidence: response.confidence,
            file_path: _file_path.to_string(),
            line_number: None,
            severity: Severity::Info,
            auto_fixable: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_generator_creation() {
        let llm_manager = Arc::new(LLMIntegrationManager::new());
        let core_engine = Arc::new(CodeEngine::new());
        let ltmc_manager = Arc::new(LTMManager::new());

        let agent = CodeGeneratorAgent::new(llm_manager, core_engine, ltmc_manager);

        assert_eq!(agent.get_agent().name, "Code Generator");
        assert_eq!(
            agent.get_agent().agent_type,
            crate::models::AgentType::CodeGenerator
        );
    }

    #[test]
    fn test_extract_code_from_response() {
        let agent = CodeGeneratorAgent::new(
            Arc::new(LLMIntegrationManager::new()),
            Arc::new(CodeEngine::new()),
            Arc::new(LTMManager::new()),
        );

        // Test with code block
        let response = "```rust\nfn test() {\n    println!(\"Hello\");\n}\n```";
        let extracted = agent.extract_code_from_response(response);
        assert_eq!(extracted, "fn test() {\n    println!(\"Hello\");\n}");

        // Test without code block
        let response = "fn test() {\n    println!(\"Hello\");\n}";
        let extracted = agent.extract_code_from_response(response);
        assert_eq!(extracted, "fn test() {\n    println!(\"Hello\");\n}");
    }

    #[tokio::test]
    async fn test_calculate_confidence() {
        let agent = CodeGeneratorAgent::new(
            Arc::new(LLMIntegrationManager::new()),
            Arc::new(CodeEngine::new()),
            Arc::new(LTMManager::new()),
        );

        let request = CodeGenerationRequest {
            file: CodeFile {
                id: uuid::Uuid::new_v4(),
                path: "test.rs".to_string(),
                content: "".to_string(),
                language: "rust".to_string(),
                modified: chrono::Utc::now(),
            },
            cursor_position: 0,
            context: String::new(),
            language: "rust".to_string(),
            generation_type: GenerationType::Function,
            constraints: None,
        };

        let confidence = agent
            .calculate_confidence("fn test() {}", &request)
            .await
            .unwrap();
        assert!(confidence > 0.7);
    }
}
