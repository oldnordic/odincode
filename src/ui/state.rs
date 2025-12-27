//! Application state for Phase 1 UI (Phase 4.3: NLP-First)
//!
//! State is split between:
//! - Transient UI state (selection, scroll, input buffer)
//! - Persistent DB state (execution_log.db, codegraph.db)
//! - Planning state (Phase 4.3): current plan, errors, LLM session
//!
//! Phase 9.2: Gated tool approvals
//! Phase 9.6-A: File tree removed

use crate::evidence_queries::EvidenceDb;
use crate::llm::chat::ChatError;
use crate::llm::chat_events::{ChatEvent, ChatReceiver, ChatSender};
use crate::llm::chat_thread::ChatThreadHandle;
use crate::llm::{ChatLoop, LlmSession, LoopAction, Plan};
use crate::ui::approval::{ApprovalResponse, ApprovalState};
use crate::ui::tool_state::{ToolExecutionState, ToolQueueEntry};
use std::path::PathBuf;

/// Write to debug log file
fn debug_log(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
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

/// Maximum number of chat messages to retain (prevents unbounded memory growth)
const MAX_CHAT_MESSAGES: usize = 200;

/// Main application state
pub struct App {
    /// Database root path
    pub db_root: PathBuf,
    /// Evidence database connection
    pub ev_db: Option<EvidenceDb>,
    /// Currently selected file
    pub selected_file: Option<String>,
    /// Current input buffer
    pub input_buffer: String,
    /// Output console messages
    pub console_messages: Vec<ConsoleMessage>,
    /// Chat transcript (separate from console output)
    pub chat_messages: Vec<ChatMessage>,
    /// Active panel
    pub active_panel: Panel,
    /// Should quit
    pub should_quit: bool,

    // Phase 4.3: Planning state
    /// Current plan being generated/approved
    pub current_plan: Option<Plan>,
    /// Plan error message (if any)
    pub plan_error: Option<String>,
    /// Planning status message ("Planning..." or error text)
    pub planning_message: Option<String>,
    /// LLM session for tracking plans/authorization
    pub llm_session: Option<LlmSession>,
    /// Current planning state
    planning_state: PlanningState,

    // Phase 4.5: Plan editing state
    /// Edit buffer for plan text editing
    edit_buffer: String,
    /// Original plan (preserved during editing)
    original_plan: Option<Plan>,

    // Phase 8.5: Chat error diagnostics
    /// Chat transport/error state (NOT shown in chat transcript)
    pub chat_error: Option<ChatError>,

    // Phase 8.6: Chat thread management
    /// Active chat thread handle (for cleanup)
    pub chat_thread_handle: Option<ChatThreadHandle>,
    /// Chat event receiver (from background thread)
    pub chat_event_receiver: Option<ChatReceiver>,
    /// Current chat session ID (for persistence)
    pub current_chat_session_id: Option<String>,

    // Phase 9.6-B1: Chat loop integration
    /// Chat loop driver for multi-step tool execution
    chat_loop: Option<ChatLoop>,
    /// Chat event sender (for loop continuation)
    pub chat_event_sender: Option<ChatSender>,

    // Phase 9.2: Gated tool approval state
    /// Session-scoped approval state
    pub approval_state: ApprovalState,
    /// Channel sender for approval responses back to chat loop
    pub approval_tx: Option<std::sync::mpsc::Sender<ApprovalResponse>>,

    // Phase 9.3: Chat scroll and trace viewer state
    /// Chat scroll offset (0 = bottom/latest, higher = further back)
    chat_scroll_offset: usize,
    /// Whether autoscroll is enabled (true = follow latest)
    autoscroll_enabled: bool,
    /// Whether trace viewer panel is visible (Phase 9.3: reserved, Phase 9.4: now used)
    trace_viewer_visible: bool,
    // Phase 9.4: Trace viewer data
    /// Cached trace rows for display
    trace_rows: Vec<crate::ui::trace::TraceRow>,
    /// Error message from last trace load (if any)
    trace_error: Option<String>,

    // Phase 9.5: Tool execution state machine
    /// Currently active tool (if any)
    pub current_tool: Option<ToolQueueEntry>,
    /// History of executed tools (for status display)
    pub tool_history: Vec<ToolQueueEntry>,

    // Phase 9.7: Tool result display
    /// Latest tool result for display in Tool Result panel
    pub latest_tool_result: Option<ToolResult>,

    // Phase 9.7: Timeline grounding (temporal enforcement)
    /// Current timeline position (step, last execution, pending failures)
    pub timeline_position: Option<crate::llm::types::TimelinePosition>,
    /// Timestamp of last memory_query (milliseconds since epoch)
    pub last_query_time_ms: Option<i64>,

    // Phase 9.9: Internal prompt mode
    /// Current prompt mode (determines which internal prompt is injected)
    pub current_prompt_mode: Option<crate::llm::types::PromptMode>,
}

/// Console message for output panel
#[derive(Debug, Clone)]
pub struct ConsoleMessage {
    pub content: String,
    pub timestamp: u64,
}

/// Chat message for transcript (separate from console output)
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

/// Tool result for display in Tool Result panel (Phase 9.7)
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Tool name that was executed
    pub tool: String,
    /// Step number
    pub step: usize,
    /// Whether execution succeeded
    pub success: bool,
    /// Standard output (if any)
    pub stdout: Option<String>,
    /// Standard error (if any)
    pub stderr: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Affected file path (if any)
    pub affected_path: Option<String>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Timestamp when result was received
    pub timestamp: u64,
}

/// Role in chat conversation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatRole {
    User,
    Assistant,
    Thinking, // "Thinking..." indicator while waiting for LLM
    // Phase 9.0: Tool status (ephemeral, not persisted)
    ToolStatus {
        tool: String,
        step: usize,
        start_timestamp: u64,
    },
}

impl ChatRole {
    /// Check if this role is ephemeral (not persisted to DB)
    pub fn is_ephemeral(&self) -> bool {
        matches!(self, ChatRole::Thinking | ChatRole::ToolStatus { .. })
    }

    /// Get display name for tool status
    pub fn tool_status_display(&self) -> Option<String> {
        if let ChatRole::ToolStatus {
            tool,
            step,
            start_timestamp,
        } = self
        {
            let elapsed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
                - start_timestamp;
            // Phase 9.3: Add token counter (n/a - adapters don't expose usage)
            Some(format!(
                "Running {} (step {})... {}s | tokens: n/a",
                tool, step, elapsed
            ))
        } else {
            None
        }
    }
}

/// Panel selection (Phase 9.6-A: FileExplorer removed, Phase 9.7: CodeView → ToolResult)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    ToolResult, // Phase 9.7: Renamed from CodeView
    ActionConsole,
    EvidencePanel,
    DiagnosticsPanel,
}

/// Application state phases (Phase 4.5: Extended with editing state)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Running,            // Normal operation
    Quitting,           // Exit requested
    PlanningInProgress, // LLM is generating plan (NEW in 4.3)
    PlanReady,          // Plan generated, waiting approval (NEW in 4.3)
    PlanError,          // LLM failed, showing error (NEW in 4.3)
    EditingPlan,        // User editing plan text (NEW in 4.5)
    AwaitingApproval,   // Phase 9.2: Awaiting user approval for GATED tool
}

/// Internal planning state (not exposed externally)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlanningState {
    None,
    InProgress,
    Ready,
    Error,
    Editing, // NEW in 4.5
}

impl App {
    /// Create new application with given db_root
    pub fn new(db_root: PathBuf) -> Self {
        // Try to open EvidenceDb (may fail if execution_log.db missing)
        let ev_db = EvidenceDb::open(&db_root).ok();

        App {
            db_root,
            ev_db,
            selected_file: None,
            input_buffer: String::new(),
            console_messages: Vec::new(),
            chat_messages: Vec::new(),
            active_panel: Panel::ToolResult, // Phase 9.7: Renamed from CodeView
            should_quit: false,
            // Phase 4.3: Initialize planning state
            current_plan: None,
            plan_error: None,
            planning_message: None,
            llm_session: None,
            planning_state: PlanningState::None,
            // Phase 4.5: Initialize editing state
            edit_buffer: String::new(),
            original_plan: None,
            // Phase 8.5: Initialize chat error state
            chat_error: None,
            // Phase 8.6: Initialize chat thread state
            chat_thread_handle: None,
            chat_event_receiver: None,
            current_chat_session_id: None,
            // Phase 9.6-B1: Initialize chat loop state
            chat_loop: None,
            chat_event_sender: None,
            // Phase 9.2: Initialize approval state
            approval_state: ApprovalState::new(),
            approval_tx: None,
            // Phase 9.3: Initialize scroll state
            chat_scroll_offset: 0,
            autoscroll_enabled: true,
            trace_viewer_visible: false,
            // Phase 9.4: Initialize trace viewer data
            trace_rows: Vec::new(),
            trace_error: None,
            // Phase 9.5: Initialize tool execution state
            current_tool: None,
            tool_history: Vec::new(),
            // Phase 9.7: Initialize tool result display
            latest_tool_result: None,
            // Phase 9.7: Initialize timeline grounding state
            timeline_position: None,
            last_query_time_ms: None,
            // Phase 9.9: Initialize prompt mode
            current_prompt_mode: None,
        }
    }

    /// Get current application state
    pub fn state(&self) -> AppState {
        if self.should_quit {
            AppState::Quitting
        } else {
            match self.planning_state {
                PlanningState::None => AppState::Running,
                PlanningState::InProgress => AppState::PlanningInProgress,
                PlanningState::Ready => AppState::PlanReady,
                PlanningState::Error => AppState::PlanError,
                PlanningState::Editing => AppState::EditingPlan,
            }
        }
    }

    /// Add console message
    pub fn log(&mut self, message: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.console_messages.push(ConsoleMessage {
            content: message,
            timestamp,
        });
    }

    /// Set active panel
    pub fn set_panel(&mut self, panel: Panel) {
        self.active_panel = panel;
    }

    /// Handle character input
    pub fn handle_char(&mut self, c: char) {
        // Phase 4.5: Route to edit buffer if in editing mode
        if matches!(self.planning_state, PlanningState::Editing) {
            self.edit_buffer.push(c);
        } else {
            self.input_buffer.push(c);
        }
    }

    /// Handle backspace
    pub fn handle_backspace(&mut self) {
        // Phase 4.5: Route to edit buffer if in editing mode
        if matches!(self.planning_state, PlanningState::Editing) {
            self.edit_buffer.pop();
        } else {
            self.input_buffer.pop();
        }
    }

    /// Handle enter (execute command)
    pub fn handle_enter(&mut self) {
        let _input = std::mem::take(&mut self.input_buffer);
        // Command execution handled by main loop
        // We just clear the buffer here
    }

    /// Quit application
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Open file
    pub fn open_file(&mut self, path: String) {
        self.log(format!("Opened: {}", path));
        self.selected_file = Some(path);
    }

    /// Read file contents
    pub fn read_file(&mut self, path: String) -> Result<String, std::io::Error> {
        match crate::file_tools::file_read(std::path::Path::new(&path)) {
            Ok(contents) => {
                self.log(format!("Read {} bytes from {}", contents.len(), path));
                Ok(contents)
            }
            Err(e) => {
                self.log(format!("Failed to read {}: {}", path, e));
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, e))
            }
        }
    }

    // Phase 4.3: Planning state methods

    /// Set state to PlanningInProgress
    pub fn set_planning_in_progress(&mut self) {
        self.planning_state = PlanningState::InProgress;
        self.planning_message = Some("Planning...".to_string());
        self.plan_error = None;
    }

    /// Set state to PlanReady with the given plan
    pub fn set_plan_ready(&mut self, plan: Plan) {
        self.planning_state = PlanningState::Ready;
        self.current_plan = Some(plan);
        self.planning_message = None;
        self.plan_error = None;
    }

    /// Set state to PlanError with error message
    pub fn set_plan_error(&mut self, error: String) {
        self.planning_state = PlanningState::Error;
        self.plan_error = Some(error.clone());
        self.planning_message = Some(format!("Error: {}", error));
    }

    /// Clear planning state (return to normal)
    pub fn clear_planning_state(&mut self) {
        self.planning_state = PlanningState::None;
        self.current_plan = None;
        self.plan_error = None;
        self.planning_message = None;
    }

    /// Get current plan reference
    pub fn current_plan(&self) -> Option<&Plan> {
        self.current_plan.as_ref()
    }

    /// Get planning message (for display in UI)
    pub fn planning_message(&self) -> Option<&str> {
        self.planning_message.as_deref()
    }

    /// Get plan error (for display in UI)
    pub fn plan_error(&self) -> Option<&str> {
        self.plan_error.as_deref()
    }

    // Phase 4.5: Plan editing methods

    /// Enter edit mode (preserves original plan)
    pub fn enter_edit_mode(&mut self) {
        if let Some(ref plan) = self.current_plan {
            self.original_plan = Some(plan.clone());
            self.edit_buffer = serde_json::to_string_pretty(plan).unwrap_or_default();
            self.planning_state = PlanningState::Editing;
        }
    }

    /// Clear edit buffer for fresh editing
    pub fn clear_edit_buffer(&mut self) {
        self.edit_buffer.clear();
    }

    /// Discard edits and return to PlanReady
    pub fn discard_edits(&mut self) {
        self.edit_buffer.clear();
        self.original_plan = None;
        self.planning_state = PlanningState::Ready;
    }

    /// Save edits (update current_plan with edited version)
    pub fn save_edits(&mut self, edited_plan: Plan) {
        self.current_plan = Some(edited_plan);
        self.edit_buffer.clear();
        self.original_plan = None;
        self.planning_state = PlanningState::Ready;
    }

    /// Save edits with logging to execution database
    pub fn save_edits_with_logging(
        &mut self,
        exec_db: &crate::execution_tools::ExecutionDb,
        edited_plan: Plan,
    ) -> Result<(), String> {
        if let Some(ref original) = self.original_plan {
            use crate::llm::log_plan_edit;
            log_plan_edit(exec_db, &original.plan_id, &edited_plan, "user edit")
                .map_err(|e| format!("Failed to log edit: {}", e))?;
        }
        self.save_edits(edited_plan);
        Ok(())
    }

    /// Get current edit buffer content
    pub fn edit_buffer(&self) -> &str {
        &self.edit_buffer
    }

    /// Get original plan ID (if in edit mode)
    pub fn original_plan_id(&self) -> Option<&str> {
        self.original_plan.as_ref().map(|p| p.plan_id.as_str())
    }

    // Chat transcript methods

    /// Add a user message to chat transcript
    pub fn add_user_message(&mut self, content: String) {
        self.chat_messages.push(ChatMessage {
            role: ChatRole::User,
            content,
        });
        // Enforce bounded history: evict oldest if at limit
        if self.chat_messages.len() > MAX_CHAT_MESSAGES {
            self.chat_messages.remove(0);
        }
    }

    /// Add an assistant message to chat transcript
    pub fn add_assistant_message(&mut self, content: String) {
        self.chat_messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content,
        });
        // Enforce bounded history: evict oldest if at limit
        if self.chat_messages.len() > MAX_CHAT_MESSAGES {
            self.chat_messages.remove(0);
        }
    }

    /// Add or update the "Thinking..." indicator
    pub fn set_thinking(&mut self) {
        // Remove existing thinking message if any
        self.chat_messages.retain(|m| m.role != ChatRole::Thinking);
        // Add new thinking indicator
        self.chat_messages.push(ChatMessage {
            role: ChatRole::Thinking,
            content: String::new(),
        });
        // Enforce bounded history: evict oldest if at limit
        if self.chat_messages.len() > MAX_CHAT_MESSAGES {
            self.chat_messages.remove(0);
        }
    }

    /// Clear thinking indicator (replace with first actual content)
    pub fn clear_thinking_with_content(&mut self, content: String) {
        // Find and replace thinking message with assistant response
        for msg in self.chat_messages.iter_mut() {
            if msg.role == ChatRole::Thinking {
                msg.role = ChatRole::Assistant;
                msg.content = content;
                return;
            }
        }
        // If no thinking message found, add assistant message
        self.add_assistant_message(content);
    }

    /// Update the last message (for streaming: replace thinking with content)
    ///
    /// Phase 9.10 FIX: Also replace Thinking role with Assistant when updating content.
    /// This handles the case where Complete event arrives without any Chunk events.
    pub fn update_last_message(&mut self, content: String) {
        if let Some(last) = self.chat_messages.last_mut() {
            last.content = content;
            // CRITICAL FIX: If last message is Thinking, change to Assistant
            // This handles the case where callback was called once (fallback path)
            // and only Complete event was sent, no Chunk events
            if last.role == ChatRole::Thinking {
                last.role = ChatRole::Assistant;
            }
        }
    }

    /// Get chat history (for LLM context)
    /// Returns (user_message, assistant_response) pairs, skipping ephemeral messages.
    pub fn chat_history(&self) -> Vec<(String, String)> {
        let mut history = Vec::new();
        let mut last_user = String::new();

        for m in &self.chat_messages {
            match m.role {
                ChatRole::User => {
                    last_user = m.content.clone();
                }
                ChatRole::Assistant => {
                    if !last_user.is_empty() {
                        history.push((last_user.clone(), m.content.clone()));
                        last_user = String::new();
                    }
                }
                ChatRole::Thinking => {}
                ChatRole::ToolStatus { .. } => {}
            }
        }

        history
    }

    /// Clear chat transcript
    pub fn clear_chat(&mut self) {
        self.chat_messages.clear();
    }

    // Phase 9.0: ToolStatus lifecycle methods

    /// Add or update a tool status message
    /// If a ToolStatus for this tool already exists, updates it in place.
    /// Otherwise adds a new ToolStatus message.
    pub fn set_tool_status(&mut self, tool: String, step: usize, start_timestamp: Option<u64>) {
        // Check if we already have a ToolStatus for this tool
        let timestamp = start_timestamp.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        // Look for existing ToolStatus and update it
        for msg in self.chat_messages.iter_mut() {
            if let ChatRole::ToolStatus { tool: ref t, .. } = msg.role {
                if t == &tool {
                    msg.role = ChatRole::ToolStatus {
                        tool: tool.clone(),
                        step,
                        start_timestamp: timestamp,
                    };
                    msg.content = format!("Running {} (step {})...", tool, step);
                    return;
                }
            }
        }

        // No existing ToolStatus, add new one
        self.chat_messages.push(ChatMessage {
            role: ChatRole::ToolStatus {
                tool: tool.clone(),
                step,
                start_timestamp: timestamp,
            },
            content: format!("Running {} (step {})...", tool, step),
        });

        // Enforce bounded history
        if self.chat_messages.len() > MAX_CHAT_MESSAGES {
            self.chat_messages.remove(0);
        }
    }

    /// Update tool status with result (success/failure)
    /// Converts ToolStatus to Assistant message with result info.
    pub fn complete_tool_status(&mut self, tool: &str, success: bool, preview: &str) {
        // Find and remove the ToolStatus message
        self.chat_messages.retain(|m| {
            if let ChatRole::ToolStatus { tool: ref t, .. } = m.role {
                t != tool
            } else {
                true
            }
        });

        // Add a brief result message (ephemeral, will be replaced by next LLM response)
        let status = if success { "✓" } else { "✗" };
        self.chat_messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: format!("{} {}: {}", status, tool, preview),
        });

        // Enforce bounded history
        if self.chat_messages.len() > MAX_CHAT_MESSAGES {
            self.chat_messages.remove(0);
        }
    }

    /// Clear all tool status messages
    pub fn clear_tool_status(&mut self) {
        self.chat_messages
            .retain(|m| !matches!(m.role, ChatRole::ToolStatus { .. }));
    }

    /// Get the current tool status display (for view rendering)
    pub fn get_tool_status_display(&self) -> Option<String> {
        self.chat_messages
            .iter()
            .find_map(|m| m.role.tool_status_display())
    }

    // Phase 8.5: Chat error diagnostics methods

    /// Set chat error (called when chat request fails)
    /// Error is routed to diagnostics panel, NOT chat transcript
    pub fn set_chat_error(&mut self, error: ChatError) {
        self.chat_error = Some(error);
        // Remove thinking indicator when error occurs
        self.chat_messages.retain(|m| m.role != ChatRole::Thinking);
    }

    /// Clear chat error (called on new chat request)
    pub fn clear_chat_error(&mut self) {
        self.chat_error = None;
    }

    /// Get human-readable description of chat error for diagnostics panel
    pub fn chat_error_description(&self) -> Option<&'static str> {
        self.chat_error.as_ref().map(|e| match e {
            ChatError::TransportError => "Transport error: Cannot reach LLM service",
            ChatError::HttpError => "HTTP error: LLM service returned error",
            ChatError::AuthError => "Authentication failed: Check API key",
            ChatError::RateLimitedError => "Rate limited: Too many requests",
            ChatError::InvalidResponseError => "Invalid response: LLM returned malformed data",
            ChatError::ConfigurationError => "Configuration error: Check LLM settings",
            ChatError::NotConfigured => "Not configured: Set up LLM provider",
        })
    }

    // Phase 8.6: Chat event processing methods

    /// Process chat events from background thread
    /// Returns true if session is complete (terminal event received)
    pub fn process_chat_events(&mut self) -> bool {
        if self.chat_event_receiver.is_some() {
            // Take receiver temporarily to avoid borrow checker issues
            let rx = self.chat_event_receiver.take().unwrap();

            // Process all available events (non-blocking)
            while let Ok(event) = rx.try_recv() {
                // Log event type for debugging
                use crate::llm::chat_events::ChatEvent;
                let event_type = match &event {
                    ChatEvent::Started { .. } => "Started",
                    ChatEvent::Chunk { .. } => "Chunk",
                    ChatEvent::Complete { .. } => "Complete",
                    ChatEvent::Error { .. } => "Error",
                    ChatEvent::ToolResult { .. } => "ToolResult",
                    ChatEvent::ApprovalRequired { .. } => "ApprovalRequired",
                    ChatEvent::LoopComplete { .. } => "LoopComplete",
                    _ => "Unknown",
                };
                debug_log(&format!("[UI] Received event: {}", event_type));

                if self.handle_chat_event(event) {
                    // Terminal event received, session complete
                    // Put receiver back before returning
                    self.chat_event_receiver = Some(rx);
                    return true;
                }
            }

            // Put receiver back
            self.chat_event_receiver = Some(rx);
        }
        false
    }

    /// Handle a single chat event
    /// Phase 9.6-B1: Integrates ChatLoop for multi-step tool execution
    /// Returns true if this is a terminal event (LoopComplete or Error)
    fn handle_chat_event(&mut self, event: ChatEvent) -> bool {
        use std::collections::HashMap;

        // Phase 9.10: Diagnostic logging for chat loop
        let loop_session_id = self.chat_loop.as_ref().and_then(|cl| cl.state().map(|s| s.session_id.clone()));
        let event_session_id = event.session_id();
        let event_type = format!("{:?}", std::mem::discriminant(&event));
        let _ = std::fs::OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open("/tmp/odincode_debug.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "[UI_EVENT] type={}, loop_session={}, event_session={}",
                    event_type,
                    loop_session_id.as_deref().unwrap_or("(none)"),
                    event_session_id)
            });

        // First, process the event through ChatLoop if active
        let loop_action = if let Some(ref mut chat_loop) = self.chat_loop {
            let action = chat_loop.process_event(&event, &self.db_root);
            let action_name = format!("{:?}", std::mem::discriminant(&action));
            let _ = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open("/tmp/odincode_debug.log")
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "[UI_EVENT] loop_action={}", action_name)
                });
            Some(action)
        } else {
            None
        };

        // Then handle the raw event (for UI updates)
        match event {
            ChatEvent::Started {
                session_id,
                user_message,
            } => {
                // Create new session in DB
                if let Ok(exec_db) = self.open_exec_db() {
                    let _ = exec_db.create_chat_session(&session_id);
                    let _ = exec_db.persist_user_message(&session_id, &user_message);
                }
                self.current_chat_session_id = Some(session_id);

                // Phase 9.8: Set "Thinking..." for continuation threads too
                // This is critical for tool continuation - after tool executes,
                // the spawned thread sends Started, and we need to show "Thinking..."
                // so that Chunk events can replace it with actual content.
                self.set_thinking();

                // Auto-scroll to bottom on new message
                self.chat_scroll_to_end();
            }
            ChatEvent::Chunk { content, .. } => {
                // Check if we still have a Thinking message (first chunk)
                let has_thinking = self
                    .chat_messages
                    .iter()
                    .any(|m| m.role == ChatRole::Thinking);

                if has_thinking {
                    // First chunk: replace thinking with assistant message
                    self.clear_thinking_with_content(content);
                } else {
                    // Subsequent chunks: update existing assistant message
                    self.update_last_message(content);
                }
                // Auto-scroll to show latest content
                self.chat_scroll_to_end();
            }
            ChatEvent::Complete {
                session_id,
                full_response,
            } => {
                // Phase 9.6-B1: Do NOT terminate here - let ChatLoop decide
                // Update UI with response
                self.update_last_message(full_response.clone());

                // Persist assistant message
                if let Ok(exec_db) = self.open_exec_db() {
                    let _ = exec_db.persist_assistant_message(&session_id, &full_response);
                }

                // Handle LoopAction if any (this is where tools are executed)
                if let Some(action) = loop_action {
                    return self.handle_loop_action(action);
                }

                // No loop active - this was simple chat without tools
                // Cleanup and terminate
                if let Ok(exec_db) = self.open_exec_db() {
                    let _ = exec_db.complete_chat_session(&session_id);
                }
                self.cleanup_chat_thread();
                return true; // Terminal event
            }
            ChatEvent::Error { error, .. } => {
                // Route error to diagnostics
                self.set_chat_error(error);

                // Cleanup thread state
                self.cleanup_chat_thread();

                return true; // Terminal event
            }
            // Phase 9.0: Loop events - emit ToolStatus messages to chat
            ChatEvent::LoopStepStarted { tool, step, .. } => {
                // Add or update tool status in chat
                self.set_tool_status(tool, step, None);
            }
            ChatEvent::ToolCallDetected { tool, category, .. } => {
                // Log for debugging, ToolStatus already shown by LoopStepStarted
                self.log(format!(
                    "[Loop] Tool call detected: {} ({})",
                    tool, category
                ));
            }
            ChatEvent::ToolResult {
                tool,
                success,
                preview,
                path,
                session_id: _,
            } => {
                // Phase 9.7: Store the latest tool result for display
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                self.latest_tool_result = Some(ToolResult {
                    tool: tool.clone(),
                    step: self.current_tool.as_ref().map(|t| t.step).unwrap_or(0),
                    success,
                    stdout: if success && !preview.is_empty() {
                        Some(preview.clone())
                    } else {
                        None
                    },
                    stderr: None, // stderr not provided in this event
                    error: if !success {
                        Some(preview.clone())
                    } else {
                        None
                    },
                    affected_path: path.clone(),
                    duration_ms: self.current_tool.as_ref().and_then(|t| {
                        if let ToolExecutionState::Completed { duration_ms } = t.state {
                            Some(duration_ms)
                        } else {
                            None
                        }
                    }),
                    timestamp,
                });

                // Phase 9.6-A: Update selected_file based on file tool results
                if let Some(ref file_path) = path {
                    match tool.as_str() {
                        "file_read" | "file_write" | "file_create" => {
                            self.selected_file = Some(file_path.clone());
                        }
                        _ => {}
                    }
                }
                // Complete tool status with result
                self.complete_tool_status(&tool, success, &preview);
            }
            ChatEvent::ApprovalRequired {
                session_id,
                tool,
                args,
                affected_path,
            } => {
                use crate::ui::approval::PendingApproval;

                // Clear any tool status
                self.clear_tool_status();

                // Parse args back to HashMap (was serialized as JSON)
                let args_map: HashMap<String, String> =
                    serde_json::from_str(&args).unwrap_or_default();

                // Create pending approval
                let pending = PendingApproval::new(
                    session_id.clone(),
                    tool.clone(),
                    args_map,
                    1, // Step will be updated from loop state
                    affected_path,
                );

                // Set in approval state
                self.approval_state.set_pending(pending);

                // Log that approval is needed
                self.log(format!("[Loop] Approval required for: {}", tool));

                // Note: Don't terminate - loop stays paused, waiting for user input
                // The main loop will check for pending approval and render prompt
                return false; // Not terminal - waiting for user input
            }
            ChatEvent::LoopComplete { .. } => {
                // Clear any remaining tool status
                self.clear_tool_status();
                // Log loop completion
                self.log("[Loop] Complete".to_string());
                // Phase 9.4: Auto-refresh trace after loop completes
                if self.trace_viewer_visible {
                    if let Ok(exec_db) = self.open_exec_db() {
                        self.refresh_trace(&exec_db, 20);
                    }
                }
                // Cleanup thread state
                self.cleanup_chat_thread();
                // End the loop
                self.chat_loop = None;
                return true; // Terminal event
            }
        }
        false
    }

    /// Phase 9.6-B1: Handle LoopAction from ChatLoop
    /// Returns true if this action terminates the loop
    fn handle_loop_action(&mut self, action: LoopAction) -> bool {
        use crate::llm::LoopAction;

        match action {
            LoopAction::None => {
                // No action needed, continue processing
                false
            }
            LoopAction::ExecuteTool(tool, args) => {
                // Execute AUTO tool immediately via ChatLoop
                if let Some(ref mut chat_loop) = self.chat_loop {
                    match chat_loop.execute_tool_and_continue(tool, args, &self.db_root) {
                        Ok(LoopAction::ToolExecuted(result)) => {
                            // Tool succeeded, loop continues
                            self.log(format!("[Loop] Tool executed: {}", result.tool));
                            false
                        }
                        Ok(LoopAction::ToolFailed(result)) => {
                            // Tool failed, loop continues with error injected
                            self.log(format!("[Loop] Tool failed: {}", result.tool));
                            false
                        }
                        Err(e) => {
                            // Fatal error, terminate loop
                            self.log(format!("[Loop] Fatal error: {}", e));
                            self.cleanup_chat_thread();
                            self.chat_loop = None;
                            true
                        }
                        _ => {
                            // Other actions shouldn't happen here
                            false
                        }
                    }
                } else {
                    false
                }
            }
            LoopAction::LoopComplete(final_response) => {
                // Loop completed naturally (no tool call)
                self.log("[Loop] Complete (no tool)".to_string());
                // Update UI with final response if different
                if !final_response.is_empty() {
                    self.update_last_message(final_response);
                }
                self.cleanup_chat_thread();
                self.chat_loop = None;
                true
            }
            LoopAction::LoopError => {
                // Loop terminated due to error
                self.log("[Loop] Error".to_string());
                self.cleanup_chat_thread();
                self.chat_loop = None;
                true
            }
            LoopAction::RequestApproval(_, _) => {
                // Already handled via ApprovalRequired event
                false
            }
            LoopAction::InjectError(msg) => {
                // Error injected, loop continues
                self.log(format!("[Loop] Error: {}", msg));
                false
            }
            // These actions shouldn't occur in the main event loop
            LoopAction::ToolExecuted(_)
            | LoopAction::ToolFailed(_)
            | LoopAction::ToolApproved
            | LoopAction::ToolDenied => false,
        }
    }

    /// Clean up chat thread state after completion
    pub fn cleanup_chat_thread(&mut self) {
        self.chat_thread_handle = None;
        self.chat_event_receiver = None;
        self.chat_event_sender = None;
        // Keep session_id for history, clear on next new chat
    }

    /// Phase 9.6-B1: Set the active chat loop
    pub fn set_chat_loop(&mut self, chat_loop: ChatLoop) {
        self.chat_loop = Some(chat_loop);
    }

    /// Phase 9.6-B1: Get mutable reference to chat loop (for approval handling)
    pub fn chat_loop_mut(&mut self) -> Option<&mut ChatLoop> {
        self.chat_loop.as_mut()
    }

    /// Open execution database (helper for persistence)
    /// Phase 9.4: pub for use by main.rs key bindings
    pub fn open_exec_db(&self) -> Result<crate::execution_tools::ExecutionDb, String> {
        use crate::execution_tools::ExecutionDb;
        ExecutionDb::open(&self.db_root).map_err(|e| format!("Failed to open DB: {}", e))
    }

    // Phase 9.2: Approval methods

    /// Get pending approval (if any)
    pub fn pending_approval(&self) -> Option<&crate::ui::approval::PendingApproval> {
        self.approval_state.pending_approval()
    }

    /// Send approval response to chat loop
    pub fn send_approval_response(&mut self, response: ApprovalResponse) {
        if let Some(ref tx) = self.approval_tx {
            let _ = tx.send(response);
        }
        // Clear pending approval state
        self.approval_state.clear_pending();
    }

    /// Check if tool is approved under current session state
    pub fn is_tool_approved(&self, tool: &str) -> bool {
        self.approval_state.is_approved(tool)
    }

    /// Reset approval state (call on new chat session)
    pub fn reset_approval_state(&mut self) {
        self.approval_state.reset();
    }

    // Phase 9.3: Chat scroll methods

    /// Check if autoscroll is enabled
    pub fn autoscroll_enabled(&self) -> bool {
        self.autoscroll_enabled
    }

    /// Get current scroll offset (0 = bottom/latest)
    pub fn chat_scroll_offset(&self) -> usize {
        self.chat_scroll_offset
    }

    /// Scroll up by N lines (disables autoscroll)
    pub fn chat_scroll_up(&mut self, lines: usize) {
        self.autoscroll_enabled = false;
        self.chat_scroll_offset += lines;
        // Clamp to available messages
        let max_offset = self.chat_messages.len();
        if self.chat_scroll_offset > max_offset {
            self.chat_scroll_offset = max_offset;
        }
    }

    /// Scroll down by N lines (may re-enable autoscroll if at bottom)
    pub fn chat_scroll_down(&mut self, lines: usize) {
        if self.chat_scroll_offset >= lines {
            self.chat_scroll_offset -= lines;
        } else {
            self.chat_scroll_offset = 0;
            // At bottom - re-enable autoscroll
            self.autoscroll_enabled = true;
        }
    }

    /// Scroll to end (reset offset and re-enable autoscroll)
    pub fn chat_scroll_to_end(&mut self) {
        self.chat_scroll_offset = 0;
        self.autoscroll_enabled = true;
    }

    // Phase 9.4: Trace viewer methods

    /// Check if trace viewer is visible
    pub fn trace_viewer_visible(&self) -> bool {
        self.trace_viewer_visible
    }

    /// Get cached trace rows
    pub fn trace_rows(&self) -> &[crate::ui::trace::TraceRow] {
        &self.trace_rows
    }

    /// Get trace error message (if any)
    pub fn trace_error(&self) -> Option<&str> {
        self.trace_error.as_deref()
    }

    /// Toggle trace viewer visibility and load trace data
    pub fn toggle_trace_viewer(
        &mut self,
        exec_db: &crate::execution_tools::ExecutionDb,
        limit: usize,
    ) {
        self.trace_viewer_visible = !self.trace_viewer_visible;
        if self.trace_viewer_visible {
            // Load trace data when opening
            self.refresh_trace(exec_db, limit);
        }
    }

    /// Refresh trace data from execution_log.db
    pub fn refresh_trace(&mut self, exec_db: &crate::execution_tools::ExecutionDb, limit: usize) {
        use crate::ui::trace::query_last_loop_trace;

        match query_last_loop_trace(exec_db.conn(), limit as i64) {
            Ok(rows) => {
                self.trace_rows = rows;
                self.trace_error = None;
            }
            Err(e) => {
                self.trace_error = Some(e);
                self.trace_rows.clear();
            }
        }
    }

    /// Refresh trace after an approval event
    pub fn on_approval_event_refresh_trace(
        &mut self,
        exec_db: &crate::execution_tools::ExecutionDb,
        limit: usize,
    ) {
        // Only refresh if viewer is visible
        if self.trace_viewer_visible {
            self.refresh_trace(exec_db, limit);
        }
    }

    /// Hide trace viewer (Phase 9.5)
    pub fn hide_trace_viewer(&mut self) {
        self.trace_viewer_visible = false;
    }

    /// Get loop header text for display
    /// Returns None if loop is idle (no tool status, no pending approval)
    /// Phase 9.7: Shows context usage bar from FrameStack
    pub fn loop_header_text(&self) -> Option<String> {
        // First check if awaiting approval (highest priority)
        if let Some(pending) = self.approval_state.pending_approval() {
            // Phase 9.7: Add context usage bar to approval prompt
            let context_bar = self
                .chat_loop
                .as_ref()
                .and_then(|loop_state| loop_state.state())
                .map(|state| state.context_usage_bar(10))
                .unwrap_or_else(|| "[..........] 0%".to_string());

            return Some(format!(
                "Loop paused: approval required for {} | {} (a=all, y=once, n=deny, q=quit)",
                pending.tool, context_bar
            ));
        }

        // Check if there's an active tool status
        for msg in &self.chat_messages {
            if let ChatRole::ToolStatus {
                ref tool,
                step,
                start_timestamp,
            } = msg.role
            {
                let elapsed = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
                    - start_timestamp;

                // Phase 9.7: Add context usage bar from FrameStack
                let context_info = self
                    .chat_loop
                    .as_ref()
                    .and_then(|loop_state| loop_state.state())
                    .map(|state| {
                        let tokens = state.frame_stack().total_tokens();
                        let usage_bar = state.context_usage_bar(10);
                        format!("{} tokens | {}", tokens, usage_bar)
                    })
                    .unwrap_or_else(|| "n/a tokens".to_string());

                return Some(format!(
                    "Loop: step {} | tool={} | {}s | {}",
                    step, tool, elapsed, context_info
                ));
            }
        }

        // No active loop
        None
    }

    // ========== Phase 9.5: Tool Execution State Machine ==========

    /// Check if there's an active tool
    pub fn has_active_tool(&self) -> bool {
        self.current_tool.is_some()
    }

    /// Get current tool state (if any)
    pub fn current_tool_state(&self) -> Option<ToolExecutionState> {
        self.current_tool.as_ref().map(|entry| entry.state.clone())
    }

    /// Get current tool elapsed time in milliseconds (if running)
    pub fn current_tool_elapsed_ms(&self) -> Option<u64> {
        self.current_tool
            .as_ref()
            .and_then(|entry| entry.state.elapsed_ms())
    }

    /// Queue a new tool for execution
    pub fn queue_tool(&mut self, tool: String, step: usize, affected_path: Option<String>) {
        let entry = ToolQueueEntry::new(tool, step, affected_path);
        self.log(format!("[Tool] Queued: {}", entry.display_text()));
        self.current_tool = Some(entry);
    }

    /// Start the current queued tool
    pub fn start_current_tool(&mut self) {
        if let Some(ref mut entry) = self.current_tool {
            entry.start();
            let display_text = entry.display_text();
            self.log(format!("[Tool] Started: {}", display_text));
        }
    }

    /// Complete the current tool with duration
    pub fn complete_current_tool(&mut self, duration_ms: u64) {
        if let Some(mut entry) = self.current_tool.take() {
            entry.complete(duration_ms);
            self.log(format!("[Tool] Completed: {}", entry.display_text()));
            // Add to history
            self.tool_history.push(entry);
        }
    }

    /// Mark the current tool as failed
    pub fn fail_current_tool(&mut self, error: String) {
        if let Some(mut entry) = self.current_tool.take() {
            entry.fail(error.clone());
            self.log(format!(
                "[Tool] Failed: {} - {}",
                entry.display_text(),
                error
            ));
            // Add to history
            self.tool_history.push(entry);
        }
    }

    /// Cancel the current tool
    pub fn cancel_current_tool(&mut self) {
        if let Some(mut entry) = self.current_tool.take() {
            entry.cancel();
            self.log(format!("[Tool] Cancelled: {}", entry.display_text()));
            // Add to history
            self.tool_history.push(entry);
        }
    }

    /// Check if current tool has timed out
    pub fn check_current_tool_timeout(&self, timeout_ms: u64) -> bool {
        if let Some(ref entry) = self.current_tool {
            entry.is_timed_out(std::time::Duration::from_millis(timeout_ms))
        } else {
            false
        }
    }

    /// Handle tool timeout (Phase 9.5) — transition to Timeout state
    pub fn handle_tool_timeout(&mut self, timeout_ms: u64) -> bool {
        if self.check_current_tool_timeout(timeout_ms) {
            if let Some(ref mut entry) = self.current_tool {
                entry.timeout();
                let display_text = entry.display_text();
                self.log(format!("[Tool] TIMED OUT: {}", display_text));
                // Move to history
                let completed = self.current_tool.take();
                if let Some(entry) = completed {
                    self.tool_history.push(entry);
                }
                return true;
            }
        }
        false
    }

    /// Get tool status text for display
    pub fn tool_status_text(&self) -> Option<String> {
        self.current_tool.as_ref().map(|entry| entry.display_text())
    }

    /// Get tool history
    pub fn tool_history(&self) -> Vec<ToolQueueEntry> {
        self.tool_history.clone()
    }

    /// Check if tool panel should be visible
    pub fn tool_panel_visible(&self) -> bool {
        self.current_tool.is_some()
    }

    /// Clear current tool without logging (internal use)
    pub fn clear_current_tool(&mut self) {
        self.current_tool = None;
    }

    // Phase 9.7: Timeline grounding methods

    /// Update timeline position (call when memory_query succeeds)
    pub fn update_timeline_position(&mut self, position: crate::llm::types::TimelinePosition) {
        self.timeline_position = Some(position);
    }

    /// Get current timeline position (if available)
    pub fn timeline_position(&self) -> Option<&crate::llm::types::TimelinePosition> {
        self.timeline_position.as_ref()
    }

    /// Record that memory_query was just called
    pub fn record_memory_query(&mut self) {
        self.last_query_time_ms = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
        );
    }

    /// Get the last memory_query time (for grounding check)
    pub fn last_query_time_ms(&self) -> Option<i64> {
        self.last_query_time_ms
    }

    /// Check if grounding is required (mutation tools need fresh memory query)
    pub fn requires_grounding(&self, is_mutation: bool) -> bool {
        if !is_mutation {
            return false;
        }
        if let Some(last_query) = self.last_query_time_ms {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            now - last_query > 10_000 // 10 seconds
        } else {
            true // Never queried
        }
    }
}

impl Default for App {
    fn default() -> Self {
        App::new(PathBuf::from("."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_role_tool_status_display() {
        let timestamp = 1000;

        // Mock time - elapsed would be current - 1000
        let role = ChatRole::ToolStatus {
            tool: "file_read".to_string(),
            step: 1,
            start_timestamp: timestamp,
        };

        assert!(matches!(role, ChatRole::ToolStatus { .. }));
        assert!(role.is_ephemeral());
    }

    #[test]
    fn test_chat_role_is_ephemeral() {
        assert!(!ChatRole::User.is_ephemeral());
        assert!(!ChatRole::Assistant.is_ephemeral());
        assert!(ChatRole::Thinking.is_ephemeral());
        assert!(ChatRole::ToolStatus {
            tool: "test".to_string(),
            step: 1,
            start_timestamp: 0
        }
        .is_ephemeral());
    }

    #[test]
    fn test_set_tool_status() {
        let mut app = App::new(PathBuf::from("."));

        app.set_tool_status("file_read".to_string(), 1, None);

        // Should have a ToolStatus message
        assert!(app.chat_messages.iter().any(|m| matches!(
            m.role,
            ChatRole::ToolStatus { tool: ref t, .. } if t == "file_read"
        )));
    }

    #[test]
    fn test_set_tool_status_updates_existing() {
        let mut app = App::new(PathBuf::from("."));

        app.set_tool_status("file_read".to_string(), 1, None);
        app.set_tool_status("file_read".to_string(), 2, None);

        // Should only have one ToolStatus message (updated, not added)
        let count = app
            .chat_messages
            .iter()
            .filter(|m| matches!(m.role, ChatRole::ToolStatus { .. }))
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_complete_tool_status_removes_status() {
        let mut app = App::new(PathBuf::from("."));

        app.set_tool_status("file_read".to_string(), 1, None);
        app.complete_tool_status("file_read", true, "content here");

        // ToolStatus should be removed
        assert!(!app
            .chat_messages
            .iter()
            .any(|m| matches!(m.role, ChatRole::ToolStatus { .. })));

        // Should have an assistant result message
        assert!(app
            .chat_messages
            .iter()
            .any(|m| { m.role == ChatRole::Assistant && m.content.contains("✓ file_read") }));
    }

    #[test]
    fn test_clear_tool_status() {
        let mut app = App::new(PathBuf::from("."));

        app.set_tool_status("file_read".to_string(), 1, None);
        app.set_tool_status("file_search".to_string(), 2, None);

        app.clear_tool_status();

        // All ToolStatus messages should be removed
        assert!(!app
            .chat_messages
            .iter()
            .any(|m| matches!(m.role, ChatRole::ToolStatus { .. })));
    }

    #[test]
    fn test_chat_history_excludes_tool_status() {
        let mut app = App::new(PathBuf::from("."));

        app.add_user_message("hello".to_string());
        app.set_tool_status("file_read".to_string(), 1, None);
        app.add_assistant_message("hi there".to_string());

        let history = app.chat_history();

        // Should only have user and assistant messages, no ToolStatus
        assert_eq!(history.len(), 1); // One (user, assistant) pair
        assert_eq!(history[0].0, "hello");
        assert_eq!(history[0].1, "hi there");
    }

    #[test]
    fn test_get_tool_status_display() {
        let mut app = App::new(PathBuf::from("."));

        assert!(app.get_tool_status_display().is_none());

        app.set_tool_status("file_read".to_string(), 1, None);

        // Should return Some display string
        let display = app.get_tool_status_display();
        assert!(display.is_some());
        let display_str = display.unwrap();
        assert!(display_str.contains("file_read"));
        assert!(display_str.contains("step 1"));
    }
}
