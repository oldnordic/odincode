//! Prompt contracts — System prompt, tool schema, evidence summary rendering
//!
//! Defines the fixed contract between LLM and OdinCode.
//! No inference, no policy — just facts.

mod evidence;
mod prompts;

// Public exports
pub use evidence::render_evidence_summary;
pub use prompts::{
    chat_system_prompt, internal_prompt, internal_prompt_explore_mode, internal_prompt_mutation_mode,
    internal_prompt_presentation_mode, internal_prompt_query_mode, system_prompt, tool_schema,
};

use crate::llm::types::EvidenceSummary;

/// Build user prompt from context and evidence
///
/// Combines user intent with evidence summary into LLM prompt.
/// UI calls this; LLM receives result.
pub fn build_user_prompt(
    user_intent: &str,
    selected_file: Option<&str>,
    current_diagnostic: Option<&str>,
    evidence_summary: &EvidenceSummary,
) -> String {
    let mut prompt = format!("User Request: \"{}\"\n\n", user_intent);
    prompt.push_str("Context:\n");

    if let Some(file) = selected_file {
        prompt.push_str(&format!("- Current file: {}\n", file));
    }

    if let Some(diag) = current_diagnostic {
        prompt.push_str(&format!("- Current diagnostic: {}\n", diag));
    }

    prompt.push('\n');
    prompt.push_str(&render_evidence_summary(evidence_summary));
    prompt.push_str("\nResponse format: Structured plan or explanation\n");

    prompt
}
