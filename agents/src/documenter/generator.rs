//! Documenter Generator
//!
//! This module handles the generation of different types of documentation
//! using LLM integration and analysis results.

use anyhow::Result;
use std::sync::Arc;
use tracing::{debug, info};

use crate::documenter::analysis::CodeAnalysis;
use crate::documenter::types::{
    DocumentationMetadata, DocumentationRequest, DocumentationResult, DocumentationSuggestion,
    DocumentationSuggestionType, DocumentationType, DocumenterConfig, SuggestionPriority,
};
use crate::llm_integration::{LLMIntegrationManager, LLMMessage, LLMRequest, LLMRequestConfig};
use chrono::{DateTime, Utc};
use odincode_core::CodeSuggestion;
use odincode_ltmc::LTMManager;
use uuid::Uuid;

/// Documentation generator for different documentation types
pub struct DocumentationGenerator {
    /// LLM integration manager
    llm_manager: Arc<LLMIntegrationManager>,
    /// LTMC manager for pattern storage and learning
    ltmc_manager: Arc<LTMManager>,
    /// Generator configuration
    config: DocumenterConfig,
}

impl DocumentationGenerator {
    /// Create a new documentation generator
    pub fn new(
        llm_manager: Arc<LLMIntegrationManager>,
        ltmc_manager: Arc<LTMManager>,
        config: DocumenterConfig,
    ) -> Self {
        Self {
            llm_manager,
            ltmc_manager,
            config,
        }
    }

    /// Generate documentation based on request type
    pub async fn generate_documentation(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<(String, Vec<DocumentationSuggestion>)> {
        info!(
            "Generating {} documentation for file: {}",
            request.documentation_type, request.file.path
        );

        let content = match request.documentation_type {
            DocumentationType::ApiDocumentation => {
                self.generate_api_documentation(request, analysis).await?
            }
            DocumentationType::CodeExplanation => {
                self.generate_code_explanation(request, analysis).await?
            }
            DocumentationType::InlineComments => {
                self.generate_inline_comments(request, analysis).await?
            }
            DocumentationType::ArchitectureDocumentation => {
                self.generate_architecture_documentation(request, analysis)
                    .await?
            }
            DocumentationType::UserGuide => self.generate_user_guide(request, analysis).await?,
            DocumentationType::Complete => {
                self.generate_complete_documentation(request, analysis)
                    .await?
            }
        };

        let suggestions = self
            .generate_improvement_suggestions(request, analysis)
            .await?;

        Ok((content, suggestions))
    }

    /// Generate API documentation
    async fn generate_api_documentation(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        debug!("Generating API documentation");

        let prompt = self.create_api_documentation_prompt(request, analysis)?;
        let system_prompt = self.get_api_documentation_system_prompt(&request.language);

        let llm_request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: prompt,
                    name: None,
                },
            ],
            config: LLMRequestConfig {
                max_tokens: Some(self.config.max_documentation_length),
                temperature: 0.3,
                ..Default::default()
            },
            request_id: None,
        };

        let response = self.llm_manager.send_request(llm_request).await?;
        Ok(response.content)
    }

    /// Generate code explanation
    async fn generate_code_explanation(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        debug!("Generating code explanation");

        let prompt = self.create_code_explanation_prompt(request, analysis)?;
        let system_prompt = self.get_code_explanation_system_prompt(&request.language);

        let llm_request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: prompt,
                    name: None,
                },
            ],
            config: LLMRequestConfig {
                max_tokens: Some(self.config.max_documentation_length),
                temperature: 0.4,
                ..Default::default()
            },
            request_id: None,
        };

        let response = self.llm_manager.send_request(llm_request).await?;
        Ok(response.content)
    }

    /// Generate inline comments
    async fn generate_inline_comments(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        debug!("Generating inline comments");

        let prompt = self.create_inline_comments_prompt(request, analysis)?;
        let system_prompt = self.get_inline_comments_system_prompt(&request.language);

        let llm_request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: prompt,
                    name: None,
                },
            ],
            config: LLMRequestConfig {
                max_tokens: Some(self.config.max_documentation_length),
                temperature: 0.2,
                ..Default::default()
            },
            request_id: None,
        };

        let response = self.llm_manager.send_request(llm_request).await?;
        Ok(response.content)
    }

    /// Generate architecture documentation
    async fn generate_architecture_documentation(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        debug!("Generating architecture documentation");

        let prompt = self.create_architecture_documentation_prompt(request, analysis)?;
        let system_prompt = self.get_architecture_documentation_system_prompt(&request.language);

        let llm_request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: prompt,
                    name: None,
                },
            ],
            config: LLMRequestConfig {
                max_tokens: Some(self.config.max_documentation_length),
                temperature: 0.5,
                ..Default::default()
            },
            request_id: None,
        };

        let response = self.llm_manager.send_request(llm_request).await?;
        Ok(response.content)
    }

    /// Generate user guide
    async fn generate_user_guide(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        debug!("Generating user guide");

        let prompt = self.create_user_guide_prompt(request, analysis)?;
        let system_prompt = self.get_user_guide_system_prompt(&request.language);

        let llm_request = LLMRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LLMMessage {
                    role: "system".to_string(),
                    content: system_prompt,
                    name: None,
                },
                LLMMessage {
                    role: "user".to_string(),
                    content: prompt,
                    name: None,
                },
            ],
            config: LLMRequestConfig {
                max_tokens: Some(self.config.max_documentation_length),
                temperature: 0.6,
                ..Default::default()
            },
            request_id: None,
        };

        let response = self.llm_manager.send_request(llm_request).await?;
        Ok(response.content)
    }

    /// Generate complete documentation
    async fn generate_complete_documentation(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        debug!("Generating complete documentation");

        // Generate all documentation types and combine them
        let api_doc = self.generate_api_documentation(request, analysis).await?;
        let code_explanation = self.generate_code_explanation(request, analysis).await?;
        let inline_comments = self.generate_inline_comments(request, analysis).await?;
        let arch_doc = self
            .generate_architecture_documentation(request, analysis)
            .await?;
        let user_guide = self.generate_user_guide(request, analysis).await?;

        let mut complete_doc = String::new();

        complete_doc.push_str("# Complete Documentation\n\n");
        complete_doc.push_str("## API Documentation\n\n");
        complete_doc.push_str(&api_doc);
        complete_doc.push_str("\n\n## Code Explanation\n\n");
        complete_doc.push_str(&code_explanation);
        complete_doc.push_str("\n\n## Inline Comments\n\n");
        complete_doc.push_str(&inline_comments);
        complete_doc.push_str("\n\n## Architecture Documentation\n\n");
        complete_doc.push_str(&arch_doc);
        complete_doc.push_str("\n\n## User Guide\n\n");
        complete_doc.push_str(&user_guide);

        Ok(complete_doc)
    }

    /// Generate improvement suggestions
    async fn generate_improvement_suggestions(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<Vec<DocumentationSuggestion>> {
        debug!("Generating improvement suggestions");

        let mut suggestions = Vec::new();

        // Analyze documentation coverage
        for element in &analysis.elements {
            if element.documentation.is_none() {
                let suggestion_type = match element.element_type {
                    crate::documenter::analysis::ElementType::Function => {
                        DocumentationSuggestionType::AddFunctionDocumentation
                    }
                    crate::documenter::analysis::ElementType::Class => {
                        DocumentationSuggestionType::AddFunctionDocumentation // Class documentation
                    }
                    _ => continue,
                };

                suggestions.push(DocumentationSuggestion {
                    suggestion_type,
                    description: format!("Add documentation for {}", element.name),
                    suggested_change: None,
                    reason: "Undocumented code elements reduce maintainability".to_string(),
                    priority: if element.complexity > 3.0 {
                        SuggestionPriority::High
                    } else {
                        SuggestionPriority::Medium
                    },
                    line_number: Some(element.line_number),
                });
            }
        }

        // Add suggestions based on complexity
        if analysis.complexity_score > 3.0 {
            suggestions.push(DocumentationSuggestion {
                suggestion_type: DocumentationSuggestionType::RefactorForDocumentation,
                description: "Consider refactoring complex code for better documentation"
                    .to_string(),
                suggested_change: None,
                reason: "High complexity makes documentation difficult".to_string(),
                priority: SuggestionPriority::Medium,
                line_number: None,
            });
        }

        // Add suggestions for code examples
        if request.style_preferences.include_examples {
            suggestions.push(DocumentationSuggestion {
                suggestion_type: DocumentationSuggestionType::AddCodeExamples,
                description: "Add code examples to improve understanding".to_string(),
                suggested_change: None,
                reason: "Code examples help users understand usage".to_string(),
                priority: SuggestionPriority::Low,
                line_number: None,
            });
        }

        Ok(suggestions)
    }

    /// Calculate metadata for generated documentation
    pub fn calculate_metadata(
        &self,
        content: &str,
        analysis: &CodeAnalysis,
    ) -> DocumentationMetadata {
        let lines_documented = content.lines().count();
        let functions_documented = analysis.functions.len();
        let examples_generated = if content.contains("```") {
            content.matches("```").count() / 2
        } else {
            0
        };
        let quality_score = self.calculate_quality_score(content, analysis);
        let reading_time_minutes = (lines_documented / 50).max(1);
        let standards_followed = self.get_standards_followed(
            &analysis
                .functions
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>()
                .join(","),
        );

        DocumentationMetadata {
            lines_documented,
            functions_documented,
            examples_generated,
            quality_score,
            reading_time_minutes,
            standards_followed,
        }
    }

    /// Calculate quality score for documentation
    fn calculate_quality_score(&self, content: &str, analysis: &CodeAnalysis) -> f64 {
        let mut score = 0.0;

        // Base score for having content
        if !content.is_empty() {
            score += 0.3;
        }

        // Score for structure (headings, sections)
        if content.contains('#') {
            score += 0.2;
        }

        // Score for code examples
        if content.contains("```") {
            score += 0.2;
        }

        // Score for documentation coverage
        score += analysis.documentation_coverage * 0.2;

        // Penalty for very short documentation
        if content.len() < 100 {
            score *= 0.5;
        }

        // Penalty for very long documentation (might be verbose)
        if content.len() > 10000 {
            score *= 0.8;
        }

        score.min(1.0)
    }

    /// Get standards followed based on language
    fn get_standards_followed(&self, language_hint: &str) -> Vec<String> {
        let mut standards = Vec::new();

        match language_hint.to_lowercase().as_str() {
            "rust" => {
                standards.push("Rust API Guidelines".to_string());
                standards.push("Rustdoc Conventions".to_string());
            }
            "python" => {
                standards.push("PEP 257 (Docstring Conventions)".to_string());
                standards.push("Sphinx Documentation".to_string());
            }
            "java" => {
                standards.push("Javadoc Conventions".to_string());
                standards.push("Java API Documentation".to_string());
            }
            "javascript" | "typescript" => {
                standards.push("JSDoc Conventions".to_string());
                standards.push("TypeDoc Standards".to_string());
            }
            _ => {
                standards.push("General Documentation Standards".to_string());
            }
        }

        standards
    }

    /// Create API documentation prompt
    fn create_api_documentation_prompt(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        let mut prompt = format!(
            "Generate API documentation for the following {} code:\n\n",
            request.language
        );
        prompt.push_str("```");
        prompt.push_str(&request.language);
        prompt.push_str("\n");
        prompt.push_str(&request.file.content);
        prompt.push_str("\n```\n\n");

        prompt.push_str(&format!("Target audience: {:?}\n", request.target_audience));
        prompt.push_str(&format!(
            "Detail level: {:?}\n",
            request.style_preferences.detail_level
        ));
        prompt.push_str(&format!(
            "Include examples: {}\n",
            request.style_preferences.include_examples
        ));

        if let Some(context) = &request.context {
            prompt.push_str(&format!("Additional context: {}\n", context));
        }

        prompt.push_str("\nGenerate comprehensive API documentation including function signatures, parameters, return values, and usage examples.");
        Ok(prompt)
    }

    /// Create code explanation prompt
    fn create_code_explanation_prompt(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        let mut prompt = format!(
            "Explain the following {} code in detail:\n\n",
            request.language
        );
        prompt.push_str("```");
        prompt.push_str(&request.language);
        prompt.push_str("\n");
        prompt.push_str(&request.file.content);
        prompt.push_str("\n```\n\n");

        prompt.push_str(&format!("Target audience: {:?}\n", request.target_audience));
        prompt.push_str(&format!(
            "Detail level: {:?}\n",
            request.style_preferences.detail_level
        ));

        if let Some(context) = &request.context {
            prompt.push_str(&format!("Additional context: {}\n", context));
        }

        prompt.push_str("\nProvide a clear explanation of what the code does, how it works, and the algorithms used.");
        Ok(prompt)
    }

    /// Create inline comments prompt
    fn create_inline_comments_prompt(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        let mut prompt = format!(
            "Add inline comments to the following {} code:\n\n",
            request.language
        );
        prompt.push_str("```");
        prompt.push_str(&request.language);
        prompt.push_str("\n");
        prompt.push_str(&request.file.content);
        prompt.push_str("\n```\n\n");

        prompt.push_str(&format!("Target audience: {:?}\n", request.target_audience));

        if let Some(context) = &request.context {
            prompt.push_str(&format!("Additional context: {}\n", context));
        }

        prompt.push_str("\nAdd appropriate inline comments to explain complex logic, important decisions, and non-obvious code.");
        Ok(prompt)
    }

    /// Create architecture documentation prompt
    fn create_architecture_documentation_prompt(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        let mut prompt = format!(
            "Generate architecture documentation for the following {} code:\n\n",
            request.language
        );
        prompt.push_str("```");
        prompt.push_str(&request.language);
        prompt.push_str("\n");
        prompt.push_str(&request.file.content);
        prompt.push_str("\n```\n\n");

        prompt.push_str(&format!(
            "Include diagrams: {}\n",
            request.style_preferences.include_diagrams
        ));
        prompt.push_str(&format!(
            "Include performance: {}\n",
            request.style_preferences.include_performance
        ));
        prompt.push_str(&format!(
            "Include security: {}\n",
            request.style_preferences.include_security
        ));

        if let Some(context) = &request.context {
            prompt.push_str(&format!("Additional context: {}\n", context));
        }

        prompt.push_str("\nGenerate high-level architecture documentation including design patterns, system components, and interactions.");
        Ok(prompt)
    }

    /// Create user guide prompt
    fn create_user_guide_prompt(
        &self,
        request: &DocumentationRequest,
        analysis: &CodeAnalysis,
    ) -> Result<String> {
        let mut prompt = format!(
            "Create a user guide for the following {} code:\n\n",
            request.language
        );
        prompt.push_str("```");
        prompt.push_str(&request.language);
        prompt.push_str("\n");
        prompt.push_str(&request.file.content);
        prompt.push_str("\n```\n\n");

        prompt.push_str(&format!("Target audience: {:?}\n", request.target_audience));
        prompt.push_str(&format!(
            "Include examples: {}\n",
            request.style_preferences.include_examples
        ));
        prompt.push_str(&format!(
            "Include troubleshooting: {}\n",
            request.style_preferences.include_troubleshooting
        ));

        if let Some(context) = &request.context {
            prompt.push_str(&format!("Additional context: {}\n", context));
        }

        prompt.push_str("\nCreate a user-friendly guide with getting started information, how-to guides, and troubleshooting tips.");
        Ok(prompt)
    }

    /// Get API documentation system prompt
    fn get_api_documentation_system_prompt(&self, language: &str) -> String {
        format!(
            "You are an expert technical writer specializing in {} API documentation. \
            Generate comprehensive, accurate, and well-structured API documentation. \
            Include function signatures, parameters, return values, usage examples, and important notes. \
            Use markdown formatting with clear headings and code blocks.",
            language
        )
    }

    /// Get code explanation system prompt
    fn get_code_explanation_system_prompt(&self, language: &str) -> String {
        format!(
            "You are an expert {} developer and educator. Explain the provided code in clear, \
            accessible language. Break down complex concepts, explain algorithms, and provide \
            context for design decisions. Use examples and analogies where helpful.",
            language
        )
    }

    /// Get inline comments system prompt
    fn get_inline_comments_system_prompt(&self, language: &str) -> String {
        format!(
            "You are an expert {} developer. Add inline comments to the provided code that \
            explain the purpose, logic, and important decisions. Focus on making the code \
            understandable to other developers. Return the complete code with added comments.",
            language
        )
    }

    /// Get architecture documentation system prompt
    fn get_architecture_documentation_system_prompt(&self, language: &str) -> String {
        format!(
            "You are a software architect specializing in {} systems. Generate high-level \
            architecture documentation that explains the design patterns, system components, \
            data flow, and architectural decisions. Include diagrams where appropriate and \
            discuss scalability, performance, and security considerations.",
            language
        )
    }

    /// Get user guide system prompt
    fn get_user_guide_system_prompt(&self, language: &str) -> String {
        format!(
            "You are a technical writer creating user guides for {} software. Write clear, \
            user-friendly documentation that helps users get started and use the code effectively. \
            Include getting started guides, how-to instructions, examples, and troubleshooting tips.",
            language
        )
    }
}
