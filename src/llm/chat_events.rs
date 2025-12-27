//! Chat thread events (Phase 8.6 + Phase 9.0)
//!
//! Events sent from background chat thread to main thread via mpsc::channel.
//! Main thread receives events in event loop, updates UI, persists to DB.
//! Phase 9.0: Added loop events for multi-step tool execution.

use crate::llm::chat::ChatError;
use std::sync::mpsc;

/// Channel sender for chat events
pub type ChatSender = mpsc::Sender<ChatEvent>;
/// Channel receiver for chat events
pub type ChatReceiver = mpsc::Receiver<ChatEvent>;

/// Event sent from chat thread to main thread
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// Streaming chunk received from LLM
    Chunk { session_id: String, content: String },
    /// LLM response complete (full response available)
    Complete {
        session_id: String,
        full_response: String,
    },
    /// Chat request failed
    Error {
        session_id: String,
        error: ChatError,
    },
    /// Thread spawned (user message sent)
    Started {
        session_id: String,
        user_message: String,
    },
    // Phase 9.0: Multi-step loop events
    /// Loop step started (AUTO tool executing)
    LoopStepStarted {
        session_id: String,
        step: usize,
        tool: String,
    },
    /// Tool call detected in LLM response
    ToolCallDetected {
        session_id: String,
        tool: String,
        category: String,
    },
    /// Tool execution completed (Phase 9.0)
    /// Phase 9.1: Added path for UI synchronization
    ToolResult {
        session_id: String,
        tool: String,
        success: bool,
        preview: String,
        path: Option<String>,
    },
    /// GATED tool requires user approval (Phase 9.0 stub for 9.1)
    /// Phase 9.2: Added affected_path for UI display
    ApprovalRequired {
        session_id: String,
        tool: String,
        args: String,
        affected_path: Option<String>,
    },
    /// Multi-step loop completed (normal termination or max steps)
    LoopComplete {
        session_id: String,
        steps_taken: usize,
        reason: String,
    },
}

impl ChatEvent {
    /// Get session ID for this event
    pub fn session_id(&self) -> &str {
        match self {
            ChatEvent::Chunk { session_id, .. } => session_id,
            ChatEvent::Complete { session_id, .. } => session_id,
            ChatEvent::Error { session_id, .. } => session_id,
            ChatEvent::Started { session_id, .. } => session_id,
            ChatEvent::LoopStepStarted { session_id, .. } => session_id,
            ChatEvent::ToolCallDetected { session_id, .. } => session_id,
            ChatEvent::ToolResult { session_id, .. } => session_id,
            ChatEvent::ApprovalRequired { session_id, .. } => session_id,
            ChatEvent::LoopComplete { session_id, .. } => session_id,
        }
    }

    /// Check if this is a terminal event (ends the thread)
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            ChatEvent::Complete { .. } | ChatEvent::Error { .. } | ChatEvent::LoopComplete { .. }
        )
    }

    /// Check if this is a loop event (Phase 9.0)
    pub fn is_loop_event(&self) -> bool {
        matches!(
            self,
            ChatEvent::LoopStepStarted { .. }
                | ChatEvent::ToolCallDetected { .. }
                | ChatEvent::ToolResult { .. }
                | ChatEvent::ApprovalRequired { .. }
                | ChatEvent::LoopComplete { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_event_session_id() {
        let sid = "test-session-123";
        assert_eq!(
            ChatEvent::Chunk {
                session_id: sid.to_string(),
                content: "hello".to_string()
            }
            .session_id(),
            sid
        );
        assert_eq!(
            ChatEvent::Complete {
                session_id: sid.to_string(),
                full_response: "full".to_string()
            }
            .session_id(),
            sid
        );
        assert_eq!(
            ChatEvent::Error {
                session_id: sid.to_string(),
                error: ChatError::TransportError
            }
            .session_id(),
            sid
        );
        assert_eq!(
            ChatEvent::Started {
                session_id: sid.to_string(),
                user_message: "hi".to_string()
            }
            .session_id(),
            sid
        );
    }

    #[test]
    fn test_chat_event_is_terminal() {
        assert!(ChatEvent::Complete {
            session_id: "x".to_string(),
            full_response: "ok".to_string()
        }
        .is_terminal());
        assert!(ChatEvent::Error {
            session_id: "x".to_string(),
            error: ChatError::TransportError
        }
        .is_terminal());
        assert!(ChatEvent::LoopComplete {
            session_id: "x".to_string(),
            steps_taken: 5,
            reason: "done".to_string()
        }
        .is_terminal());
        assert!(!ChatEvent::Chunk {
            session_id: "x".to_string(),
            content: "chunk".to_string()
        }
        .is_terminal());
        assert!(!ChatEvent::Started {
            session_id: "x".to_string(),
            user_message: "hi".to_string()
        }
        .is_terminal());
    }

    // Phase 9.0: Loop event tests
    #[test]
    fn test_loop_event_session_id() {
        let sid = "loop-session-456";
        assert_eq!(
            ChatEvent::LoopStepStarted {
                session_id: sid.to_string(),
                step: 1,
                tool: "file_read".to_string()
            }
            .session_id(),
            sid
        );
        assert_eq!(
            ChatEvent::ToolCallDetected {
                session_id: sid.to_string(),
                tool: "file_write".to_string(),
                category: "GATED".to_string()
            }
            .session_id(),
            sid
        );
        assert_eq!(
            ChatEvent::ToolResult {
                session_id: sid.to_string(),
                tool: "file_read".to_string(),
                success: true,
                preview: "content".to_string(),
                path: Some("src/lib.rs".to_string())
            }
            .session_id(),
            sid
        );
        assert_eq!(
            ChatEvent::ApprovalRequired {
                session_id: sid.to_string(),
                tool: "file_write".to_string(),
                args: "path: test.txt".to_string(),
                affected_path: Some("test.txt".to_string())
            }
            .session_id(),
            sid
        );
        assert_eq!(
            ChatEvent::LoopComplete {
                session_id: sid.to_string(),
                steps_taken: 3,
                reason: "done".to_string()
            }
            .session_id(),
            sid
        );
    }

    #[test]
    fn test_is_loop_event() {
        assert!(ChatEvent::LoopStepStarted {
            session_id: "x".to_string(),
            step: 1,
            tool: "file_read".to_string()
        }
        .is_loop_event());
        assert!(ChatEvent::ToolCallDetected {
            session_id: "x".to_string(),
            tool: "file_read".to_string(),
            category: "AUTO".to_string()
        }
        .is_loop_event());
        assert!(ChatEvent::ToolResult {
            session_id: "x".to_string(),
            tool: "file_read".to_string(),
            success: true,
            preview: "ok".to_string(),
            path: None
        }
        .is_loop_event());
        assert!(ChatEvent::ApprovalRequired {
            session_id: "x".to_string(),
            tool: "file_write".to_string(),
            args: "path: test.txt".to_string(),
            affected_path: None
        }
        .is_loop_event());
        assert!(ChatEvent::LoopComplete {
            session_id: "x".to_string(),
            steps_taken: 5,
            reason: "max_steps".to_string()
        }
        .is_loop_event());
        assert!(!ChatEvent::Chunk {
            session_id: "x".to_string(),
            content: "test".to_string()
        }
        .is_loop_event());
        assert!(!ChatEvent::Complete {
            session_id: "x".to_string(),
            full_response: "ok".to_string()
        }
        .is_loop_event());
        assert!(!ChatEvent::Error {
            session_id: "x".to_string(),
            error: ChatError::TransportError
        }
        .is_loop_event());
        assert!(!ChatEvent::Started {
            session_id: "x".to_string(),
            user_message: "hi".to_string()
        }
        .is_loop_event());
    }
}
