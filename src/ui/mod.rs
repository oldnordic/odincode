//! Phase 1 Editor UI — Terminal interface (Phase 4.3: NLP-First)
//!
//! UI is a deterministic surface only:
//! - NO async
//! - NO background threads
//! - NO inference, suggestions, or policy
//! - Every action = explicit tool call
//!
//! Phase 4.3 Input Model:
//! - Natural language input (no ":") → NLP intent → LLM planner
//! - Commands start with ':' prefix
//! - Supported commands: :open, :read, :lsp, :evidence, :find, :plan, :help, :quit

pub mod approval; // Phase 9.2: Approval state management
pub mod chat_compact; // Phase 8.6: Chat compaction
pub mod chat_persist; // Phase 8.6: Chat persistence
pub mod handlers;
pub mod input;
pub mod state;
pub mod tool_state; // Phase 9.5: Tool execution state machine
pub mod trace; // Phase 9.3: Loop trace viewer
pub mod view;

// Re-exports
pub use approval::{ApprovalResponse, ApprovalScope, ApprovalState, PendingApproval};
pub use input::{parse_command, render_help, Command};
pub use state::{App, AppState, ChatMessage, ChatRole, Panel};
pub use view::render;

/// UI result type
pub type Result<T> = std::result::Result<T, Error>;

/// UI errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Command error: {0}")]
    Command(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("LLM error: {0}")]
    Llm(String), // NEW in Phase 4.3
}
