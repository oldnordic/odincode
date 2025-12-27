//! Chat thread management (Phase 8.6)
//!
//! Spawns a fire-and-forget thread for LLM LLM I/O.
//! Thread sends events via mpsc::channel to main thread.
//! Main thread handles all persistence.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::llm::chat;
use crate::llm::chat_events::{ChatEvent, ChatSender};
use crate::llm::frame_stack::FrameStack;

/// Write to debug log file
fn debug_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("/tmp/odincode_debug.log")
    {
        let _ = writeln!(file, "{}", msg);
        let _ = file.flush();
    }
}

/// Active chat thread handle (for cleanup)
#[derive(Debug)]
pub struct ChatThreadHandle {
    handle: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    session_id: String,
}

impl ChatThreadHandle {
    /// Create new handle from thread components
    fn new(handle: JoinHandle<()>, shutdown: Arc<AtomicBool>, session_id: String) -> Self {
        Self {
            handle: Some(handle),
            shutdown,
            session_id,
        }
    }

    /// Get session ID for this thread
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Check if thread is still running
    pub fn is_running(&self) -> bool {
        self.handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }

    /// Request shutdown (thread will check on next iteration)
    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }

    /// Join thread with timeout
    pub fn join_timeout(mut self, duration: Duration) -> Result<(), ThreadTimeoutError> {
        if let Some(handle) = self.handle.take() {
            let start = std::time::Instant::now();
            while start.elapsed() < duration {
                if handle.is_finished() {
                    // Thread finished, try to join
                    return handle.join().map_err(|_| ThreadTimeoutError::JoinError);
                }
                thread::sleep(Duration::from_millis(50));
            }
            // Timeout elapsed
            self.shutdown();
            Err(ThreadTimeoutError::Timeout {
                session_id: self.session_id,
                elapsed: start.elapsed(),
            })
        } else {
            Ok(())
        }
    }
}

/// Thread timeout error
#[derive(Debug)]
pub enum ThreadTimeoutError {
    /// Thread did not finish within timeout
    Timeout {
        session_id: String,
        elapsed: Duration,
    },
    /// Thread panicked or join failed
    JoinError,
}

impl std::fmt::Display for ThreadTimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreadTimeoutError::Timeout {
                session_id,
                elapsed,
            } => write!(
                f,
                "Chat thread {:?} timed out after {:?}",
                session_id, elapsed
            ),
            ThreadTimeoutError::JoinError => write!(f, "Chat thread failed to join"),
        }
    }
}

impl std::error::Error for ThreadTimeoutError {}

/// Spawn a background chat thread
///
/// # Arguments
/// * `db_root` - Path to database root (for LLM config)
/// * `user_message` - User's chat message
/// * `tx` - Channel sender to main thread
/// * `session_id` - Optional session ID (if None, generates new one)
///
/// # Returns
/// Handle for thread management (shutdown, join, etc.)
///
/// # CRITICAL INVARIANT
/// Thread does ONLY LLM I/O. NO database writes. NO tool execution.
/// All persistence happens on main thread.
pub fn spawn_chat_thread(
    db_root: &Path,
    user_message: String,
    tx: ChatSender,
    session_id: Option<String>,
) -> ChatThreadHandle {
    let session_id = session_id.unwrap_or_else(generate_session_id);
    let db_root = db_root.to_path_buf();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Send Started event immediately
    let _ = tx.send(ChatEvent::Started {
        session_id: session_id.clone(),
        user_message: user_message.clone(),
    });

    // Clone session_id for the handle (thread consumes the original)
    let session_id_for_handle = session_id.clone();

    // Spawn thread
    let handle = thread::spawn(move || {
        // CRITICAL: Only LLM I/O here. NO DB writes.
        let result = chat::chat(&user_message, &db_root, |chunk| {
            // Check shutdown flag before sending
            if shutdown_clone.load(Ordering::Relaxed) {
                return;
            }
            let _ = tx.send(ChatEvent::Chunk {
                session_id: session_id.clone(),
                content: chunk.to_string(),
            });
        });

        // Send terminal event
        match result {
            Ok(full_response) => {
                let _ = tx.send(ChatEvent::Complete {
                    session_id,
                    full_response,
                });
            }
            Err(e) => {
                let _ = tx.send(ChatEvent::Error {
                    session_id,
                    error: e,
                });
            }
        }
    });

    ChatThreadHandle::new(handle, shutdown, session_id_for_handle)
}

/// Spawn a background chat thread with conversation history (Phase 9.7 → 9.8)
///
/// # Arguments
/// * `db_root` - Path to database root (for LLM config)
/// * `frame_stack` - Full conversation history (builds messages array with context)
/// * `tx` - Channel sender to main thread
/// * `session_id` - Optional session ID (if None, generates new one)
///
/// # Returns
/// Handle for thread management (shutdown, join, etc.)
///
/// # Phase 9.7: Context Continuity
/// # Phase 9.8: Multi-Turn Message Support
///
/// This function builds a proper message array from the FrameStack,
/// ensuring the LLM receives each conversation turn as a separate message
/// with correct role-per-message structure. This fixes the bug where
/// adapters collapsed the entire conversation into a single "user" message.
pub fn spawn_chat_thread_with_frame_stack(
    db_root: &Path,
    frame_stack: &mut FrameStack,
    tx: ChatSender,
    session_id: Option<String>,
) -> ChatThreadHandle {
    let session_id = session_id.unwrap_or_else(generate_session_id);
    let db_root = db_root.to_path_buf();
    let messages = frame_stack.build_messages();
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Extract user message from frame stack (last user message)
    let last_user_message = frame_stack
        .iter()
        .rev()
        .find_map(|f| {
            if let crate::llm::frame_stack::Frame::User(msg) = f {
                Some(msg.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "(continue)".to_string());

    // Diagnostic logging
    debug_log(&format!(
        "[CHAT_THREAD] Spawning: session_id={}, frames={}, messages={}",
        session_id,
        frame_stack.len(),
        messages.len()
    ));

    // Send Started event immediately
    let _ = tx.send(ChatEvent::Started {
        session_id: session_id.clone(),
        user_message: last_user_message,
    });

    // Clone session_id for the handle (thread consumes the original)
    let session_id_for_handle = session_id.clone();
    // Clone again for use inside closure (we need it after the closure)
    let session_id_for_log = session_id_for_handle.clone();

    // Spawn thread
    let handle = thread::spawn(move || {
        debug_log(&format!("[CHAT_THREAD] {}: started", session_id));

        // CRITICAL: Only LLM I/O here. NO DB writes.
        // Phase 9.8: Use chat_with_messages for proper multi-turn support
        debug_log(&format!("[CHAT_THREAD] {}: calling chat_with_messages with {} messages", session_id, messages.len()));
        let mut chunk_count = 0;
        let result = chat::chat_with_messages(&messages, &db_root, |chunk| {
            // Check shutdown flag before sending
            if shutdown_clone.load(Ordering::Relaxed) {
                return;
            }
            chunk_count += 1;
            debug_log(&format!("[CHAT_THREAD] {}: Chunk #{}: {} chars", session_id, chunk_count, chunk.len()));
            let _ = tx.send(ChatEvent::Chunk {
                session_id: session_id.clone(),
                content: chunk.to_string(),
            });
        });

        debug_log(&format!("[CHAT_THREAD] {}: Total chunks sent: {}", session_id, chunk_count));

        // Log result before sending terminal event
        match &result {
            Ok(resp) => {
                debug_log(&format!(
                    "[CHAT_THREAD] {}: SUCCESS, response length={} chars",
                    session_id,
                    resp.len()
                ));
            }
            Err(e) => {
                debug_log(&format!("[CHAT_THREAD] {}: ERROR - {:?}", session_id, e));
            }
        }

        // Send terminal event
        match result {
            Ok(full_response) => {
                debug_log(&format!("[CHAT_THREAD] {}: Sending Complete event (len={})", session_id, full_response.len()));
                match tx.send(ChatEvent::Complete {
                    session_id: session_id.clone(),
                    full_response,
                }) {
                    Ok(()) => {
                        debug_log(&format!("[CHAT_THREAD] {}: Complete event sent successfully", session_id));
                    }
                    Err(_) => {
                        debug_log(&format!("[CHAT_THREAD] {}: ERROR - Failed to send Complete event (receiver dropped?)", session_id));
                    }
                }
            }
            Err(e) => {
                debug_log(&format!("[CHAT_THREAD] {}: Sending Error event: {:?}", session_id, e));
                let _ = tx.send(ChatEvent::Error {
                    session_id,
                    error: e,
                });
            }
        }

        debug_log(&format!("[CHAT_THREAD] {}: exiting", session_id_for_log));
    });

    ChatThreadHandle::new(handle, shutdown, session_id_for_handle)
}

/// Generate session ID (UUID v4 format)
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("chat-{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_session_id() {
        let id1 = generate_session_id();
        let id2 = generate_session_id();

        assert!(id1.starts_with("chat-"));
        assert!(id2.starts_with("chat-"));
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_thread_timeout_error_display() {
        let err = ThreadTimeoutError::Timeout {
            session_id: "test-123".to_string(),
            elapsed: Duration::from_secs(5),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("timed out"));
        assert!(msg.contains("5s"));

        let err2 = ThreadTimeoutError::JoinError;
        let msg2 = format!("{}", err2);
        assert!(msg2.contains("failed to join"));
    }
}
