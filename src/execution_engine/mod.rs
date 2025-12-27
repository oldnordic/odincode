//! Execution engine — synchronous step runner for approved plans
//!
//! Executes validated plans from Phase 2 LLM integration.

mod chat_tool_runner;
mod circuit_breaker;
mod errors;
mod execution_budget;
mod executor;
mod output_kind;
mod preconditions;
mod result;
mod safety_config;
mod stall_detector;
mod structural_summary;
mod tool_catalog;
mod tool_mapper;
mod tool_registry;
mod tool_router;
mod tool_memory;

pub use chat_tool_runner::{
    format_tool_result_for_context, ChatToolCategory, ChatToolRunner, ToolResult, AUTO_TOOLS,
    FORBIDDEN_TOOLS, GATED_TOOLS,
};
pub use circuit_breaker::{CircuitBreaker, CircuitError, CircuitState};
pub use execution_budget::{BudgetError, ToolBudgetTracker};
pub use output_kind::ToolOutputKind;
pub use safety_config::SafetyConfig;
pub use stall_detector::{StallDetector, StallReason};
pub use errors::ExecutionError;
pub use executor::{ConfirmationCallback, Executor, ProgressCallback};
pub use result::{DiagnosticArtifact, ExecutionResult, ExecutionStatus, StepResult};
pub use tool_mapper::invoke_tool;
pub use tool_registry::{
    ArgumentType, ResourceRequirement, SideEffectLevel, ToolArgument, ToolCapability,
    ToolClassification, ToolExamples, ToolMetadata, ToolRegistry,
};
pub use tool_router::{RoutingConfig, RoutingDestination, RoutingRule, ToolRouter, UserIntent};
pub use tool_memory::{ExecutionOutcome, ExecutionRecord, ExecutionRecommendation, ToolMemory, ToolStatistics};

// Re-export types from Phase 2 for convenience
use crate::llm::types::{Plan, PlanAuthorization, Step};
use uuid::Uuid;

/// Approved plan for execution
///
/// Combines a validated Plan with user authorization.
pub struct ApprovedPlan {
    pub plan: Plan,
    pub authorization: PlanAuthorization,
}

/// Auto-approve callback (for testing)
#[derive(Clone, Copy)]
pub struct AutoApprove;

impl ConfirmationCallback for AutoApprove {
    fn request_confirmation(&self, _step: &Step) -> bool {
        true
    }
}

/// Auto-deny callback (for testing)
#[derive(Clone, Copy)]
pub struct AutoDeny;

impl ConfirmationCallback for AutoDeny {
    fn request_confirmation(&self, _step: &Step) -> bool {
        false
    }
}

/// No-op progress callback (for testing)
#[derive(Clone, Copy)]
pub struct NoopProgress;

impl ProgressCallback for NoopProgress {
    fn on_step_start(&self, _step: &Step) {}
    fn on_step_complete(&self, _result: &StepResult) {}
    fn on_step_failed(&self, _result: &StepResult) {}
}

/// Generate unique execution ID
pub fn generate_execution_id() -> String {
    Uuid::new_v4().to_string()
}
