//! LLM session — UI integration boundary and logging
//!
//! Handles:
//! - Plan rendering for UI display
//! - User authorization tracking
//! - Logging interactions to execution_log.db
//!
//! UI calls these functions; LLM layer does NOT execute tools directly.

mod errors;
mod plan_logging;
mod plan_proposal;
mod plan_render;
mod session_state;

// Re-export public API
pub use errors::SessionError;
pub use plan_logging::{log_plan_edit, log_plan_generation, log_stream_chunk};
pub use plan_proposal::{propose_plan, propose_plan_streaming};
pub use plan_render::render_plan_for_ui;
pub use session_state::{create_session, LlmSession};
