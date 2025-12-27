//! Chat loop — Multi-step tool execution (Phase 9.0 → 9.7)
//!
//! MAIN-thread only. Background thread does LLM I/O only.
//! Loop progresses via event processing (non-blocking).
//!
//! # Flow
//!
//! 1. User sends message → add to FrameStack → spawn chat thread
//! 2. LLM responds with ChatEvent::Complete
//! 3. Append response to FrameStack (assistant frame)
//! 4. Parse for TOOL_CALL
//! 5. If AUTO tool: execute, add result to FrameStack, trigger next LLM call
//! 6. If GATED tool: emit ApprovalRequired, pause loop
//! 7. If FORBIDDEN: inject error, continue
//! 8. If no TOOL_CALL: loop complete
//!
//! Phase 9.7: FrameStack maintains full conversation history, fixing LLM amnesia.

mod chat_loop;
mod constants;
mod event_handler;
mod gated_execution;
mod loop_action;
mod loop_state;
mod loop_state_mode;
mod loop_state_types;

// Re-export public API
pub use chat_loop::ChatLoop;
pub use constants::{MAX_AUTO_STEPS, HiddenToolResult, MAX_CONTEXT_MESSAGES};
pub use loop_action::LoopAction;
pub use loop_state::{LoopState, PendingGatedTool};
