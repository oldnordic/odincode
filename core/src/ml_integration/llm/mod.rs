//! LLM Integration Module
//!
//! Provides integration with Large Language Models for code analysis and generation.

pub mod integration;

// Re-export key types
pub use integration::{
    LLMConfig, LLMIntegration, LLMProvider, LLMRequest, LLMRequestType, LLMResponse, LLMStats,
    TokenUsage,
};
