//! LLM Integration — Deterministic planner + adapter layer
//!
//! Phase 8.1: Two separate LLM entrypoints:
//! - chat()   → stream<TextChunk> (conversational, NO approval/DB/plans)
//! - plan()   → Plan (structured, requires approval)
//!
//! **IMPORTANT**: Chat and Plan are ISOLATED lanes:
//! - Chat: responses stream directly, no Plan objects, no DB writes
//! - Plan: creates Plan objects, requires approval, writes to execution DB

pub mod adapters;
pub mod chat; // Phase 8.1: Isolated chat lane
pub mod chat_events; // Phase 8.6: Chat thread events
pub mod chat_loop; // Phase 9.0: Multi-step tool loop driver
pub mod chat_thread; // Phase 8.6: Chat thread spawning
pub mod contracts;
pub mod discovery; // Phase 10.6: Progressive tool discovery integration
pub mod frame_stack; // Phase 9.7: Conversation frame stack
pub mod planner;
pub mod router;
pub mod session;
pub mod tool_call; // Phase 9.0: TOOL_CALL parser
pub mod types;

// Re-export main types
pub use types::{
    AuthorizationStatus, EvidenceSummary, Intent, Plan, PlanAuthorization, PromptMode,
    SessionContext, Step,
};

// Re-export contract functions
pub use contracts::{build_user_prompt, render_evidence_summary, system_prompt, tool_schema};

// Re-export planner functions
pub use planner::{parse_plan, validate_plan, PlanError};

// Re-export router
pub use router::{generate_tool_whitelist, tool_is_allowed, ToolRouter, TOOL_WHITELIST};

// Re-export session
pub use session::{
    create_session,
    log_plan_edit, // Phase 4.5
    log_plan_generation,
    log_stream_chunk, // Phase 4.4
    propose_plan,
    propose_plan_streaming, // Phase 4.4
    render_plan_for_ui,
    LlmSession,
    SessionError,
};

// Re-export adapters (Phase 5)
pub use adapters::{create_adapter_from_config, transport::AdapterError, Adapter};

// Re-export chat (Phase 8.1: Isolated chat lane)
pub use chat::{chat, chat_with_messages, ChatError};

// Re-export chat_loop (Phase 9.0: Multi-step tool loop)
pub use chat_loop::{ChatLoop, LoopAction, LoopState, PendingGatedTool, MAX_AUTO_STEPS};

// Re-export frame_stack (Phase 9.7: Conversation frame stack)
pub use frame_stack::{Frame, FrameStack};

// Re-export discovery (Phase 10.6: Progressive tool discovery)
pub use discovery::{discover_tools_for_chat, discover_tools_for_plan, ToolDiscoveryContext};

/// LLM integration error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Plan error: {0}")]
    Plan(#[from] PlanError),

    #[error("Session error: {0}")]
    Session(#[from] SessionError),

    #[error("Execution DB error: {0}")]
    ExecutionDb(#[from] crate::execution_tools::Error),
}

/// Result type for LLM operations
pub type Result<T> = std::result::Result<T, Error>;
