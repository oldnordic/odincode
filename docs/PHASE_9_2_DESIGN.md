# Phase 9.2 — Gated Tools & Approvals: Design Document

## GOAL

Implement session-scoped approvals for GATED tools (file_write, file_create) in the multi-step chat loop, allowing user to approve once per session without repeated prompts.

## NON-NEGOTIABLE CONSTRAINTS

1. Tool execution enforcement stays in engine; LLM emits TOOL_CALL only
2. Only ONE TOOL_CALL per LLM response unless user asks otherwise
3. No async runtime; std::thread only
4. No breaking changes to Phase 9.0 loop semantics for AUTO tools
5. Every gated write/create must be auditable: approval event + execution event logged
6. Approval is per-session only; cleared on new chat session
7. UI must show clear approval prompt (y=once, a=session, n=deny) and remain responsive
8. No new file > 300 LOC; split modules if needed
9. Must preserve /quit and Ctrl+C immediate exit

## ARCHITECTURE DESIGN

### 1. Data Structures

#### 1.1 ApprovalScope Enum
**File**: `src/ui/approval.rs` (NEW, <200 LOC)

```rust
/// Scope of tool approval
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalScope {
    /// Approve this specific tool invocation only
    Once { tool: String },
    /// Approve all GATED tools for current session
    SessionAllGated,
}

impl ApprovalScope {
    /// Check if a given tool is approved under this scope
    pub fn covers(&self, tool: &str) -> bool {
        match self {
            ApprovalScope::Once { tool: t } => t == tool,
            ApprovalScope::SessionAllGated => true,
        }
    }

    /// Display text for UI
    pub fn display_text(&self) -> &str {
        match self {
            ApprovalScope::Once { .. } => "once (this tool)",
            ApprovalScope::SessionAllGated => "session (all gated tools)",
        }
    }
}
```

#### 1.2 PendingApproval Structure
**File**: `src/ui/approval.rs`

```rust
/// Pending approval awaiting user response
#[derive(Debug, Clone)]
pub struct PendingApproval {
    /// Chat session ID
    pub session_id: String,
    /// Tool being approved
    pub tool: String,
    /// Tool arguments (for display)
    pub args: std::collections::HashMap<String, String>,
    /// Step number when tool was requested
    pub step: usize,
    /// Affected file path (if any)
    pub affected_path: Option<String>,
    /// Timestamp of request
    pub requested_at: std::time::SystemTime,
}

impl PendingApproval {
    /// Create new pending approval
    pub fn new(
        session_id: String,
        tool: String,
        args: std::collections::HashMap<String, String>,
        step: usize,
        affected_path: Option<String>,
    ) -> Self {
        Self {
            session_id,
            tool,
            args,
            step,
            affected_path,
            requested_at: std::time::SystemTime::now(),
        }
    }

    /// Format approval prompt for UI display
    pub fn format_prompt(&self) -> String {
        let mut prompt = format!("GATED Tool: {}\n", self.tool);
        if let Some(ref path) = self.affected_path {
            prompt.push_str(&format!("  File: {}\n", path));
        }
        prompt.push_str("  Approve? [y=once, a=session, n=deny, q=quit]");
        prompt
    }
}
```

#### 1.3 ApprovalState
**File**: `src/ui/approval.rs`

```rust
/// Session-scoped approval state
#[derive(Debug, Clone, Default)]
pub struct ApprovalState {
    /// Approve all GATED tools for this session
    pub approved_all_gated: bool,
    /// Tools approved for single use (tool name)
    pub approved_once: std::collections::HashSet<String>,
    /// Current pending approval (if any)
    pub pending: Option<PendingApproval>,
}

impl ApprovalState {
    /// Create new approval state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a tool is approved
    pub fn is_approved(&self, tool: &str) -> bool {
        self.approved_all_gated || self.approved_once.contains(tool)
    }

    /// Grant approval for a scope
    pub fn grant(&mut self, scope: ApprovalScope) {
        match scope {
            ApprovalScope::Once { tool } => {
                self.approved_once.insert(tool);
            }
            ApprovalScope::SessionAllGated => {
                self.approved_all_gated = true;
            }
        }
    }

    /// Set pending approval
    pub fn set_pending(&mut self, pending: PendingApproval) {
        self.pending = Some(pending);
    }

    /// Clear pending approval
    pub fn clear_pending(&mut self) {
        self.pending = None;
    }

    /// Reset state (call on new chat session)
    pub fn reset(&mut self) {
        self.approved_all_gated = false;
        self.approved_once.clear();
        self.pending = None;
    }
}
```

### 2. UI State Integration

#### 2.1 App Struct Changes
**File**: `src/ui/state.rs` (split into smaller modules)

```rust
/// Add to App struct:
pub struct App {
    // ... existing fields ...

    /// Phase 9.2: Session-scoped approval state
    pub approval_state: ApprovalState,

    /// Phase 9.2: Channel sender for approval responses back to chat loop
    pub approval_tx: Option<std::sync::mpsc::Sender<ApprovalResponse>>,
}

/// New enum for approval responses
#[derive(Debug, Clone)]
pub enum ApprovalResponse {
    ApproveOnce(String),           // tool name
    ApproveSessionAllGated,        // all tools
    Deny(String),                  // tool name
    Quit,                          // immediate exit
}
```

**IMPORTANT**: Since `state.rs` is already 849 LOC, the approval state and response types must live in a separate module (`src/ui/approval.rs`).

### 3. Event Flow Design

#### 3.1 Approval Request Flow

```
1. Chat loop encounters GATED tool
   → LoopState::pause(pending_tool)
   → LoopAction::RequestApproval(tool, args)
   → ChatEvent::ApprovalRequired { session_id, tool, args, affected_path }

2. Main thread receives ApprovalRequired event
   → App::handle_chat_event()
   → Enter ApprovalMode in AppState
   → Create PendingApproval
   → Store in approval_state.pending
   → Render approval prompt in input bar

3. User types response (y/a/n/q)
   → Input handler detects ApprovalMode
   → Map to ApprovalResponse
   → Send via approval_tx channel
   → Exit ApprovalMode

4. Chat loop thread receives approval
   → LoopAction::RequestApproval waits on channel
   → Process approval:
     * ApproveOnce/SessionAllGated → execute_gated_tool() → continue loop
     * Deny → inject error message → continue loop
     * Quit → terminate loop immediately
```

#### 3.2 Channel Architecture

**Option Selected**: `mpsc::Sender<ApprovalResponse>` stored in `LoopState`

```rust
// In src/llm/chat_loop.rs, extend LoopState:

pub struct LoopState {
    // ... existing fields ...

    /// Channel to receive approval responses from UI
    approval_rx: Option<std::sync::mpsc::Receiver<ApprovalResponse>>,
}

impl LoopState {
    /// Set approval receiver (called when starting loop)
    pub fn set_approval_channel(&mut self, rx: std::sync::mpsc::Receiver<ApprovalResponse>) {
        self.approval_rx = Some(rx);
    }

    /// Wait for approval response (blocking, with timeout)
    pub fn wait_for_approval(&mut self, timeout_ms: u64) -> ApprovalResponse {
        if let Some(ref rx) = self.approval_rx {
            let deadline = std::time::Instant::now()
                + std::time::Duration::from_millis(timeout_ms);
            let mut response = ApprovalResponse::Deny("timeout".to_string());

            while std::time::Instant::now() < deadline {
                match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(r) => {
                        response = r;
                        break;
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                }
            }
            response
        } else {
            // No channel configured - auto-deny
            ApprovalResponse::Deny("no_channel".to_string())
        }
    }
}
```

### 4. UI Prompt Design

#### 4.1 Input Bar (Approval Mode)
**File**: `src/ui/view.rs` (update `render_input_bar`)

```rust
AppState::AwaitingApproval => {
    // Phase 9.2: Approval prompt
    let pending = app.pending_approval().unwrap(); // Safe in this state

    let mut content = vec![
        Line::from(Span::styled(
            "GATED Tool Approval",
            Style::default().fg(Color::Yellow).bold(),
        )),
        Line::from(""),
        Line::from(format!("  Tool: {}", pending.tool)),
    ];

    if let Some(ref path) = pending.affected_path {
        content.push(Line::from(format!("  File: {}", path)));
    }

    content.push(Line::from(""));
    content.push(Line::from(Span::styled(
        "  [y] once  [a] session  [n] deny  [q] quit",
        Style::default().fg(Color::Green),
    )));

    content
}
```

#### 4.2 Input Handling
**File**: `src/ui/handlers.rs` (update key handler)

```rust
// In main input handler, check for AwaitingApproval state:
if app.state() == AppState::AwaitingApproval {
    match input.as_str() {
        "y" | "Y" => {
            if let Some(ref pending) = app.approval_state.pending {
                let tool = pending.tool.clone();
                app.send_approval_response(ApprovalResponse::ApproveOnce(tool));
            }
        }
        "a" | "A" => {
            app.send_approval_response(ApprovalResponse::ApproveSessionAllGated);
        }
        "n" | "N" => {
            if let Some(ref pending) = app.approval_state.pending {
                let tool = pending.tool.clone();
                app.send_approval_response(ApprovalResponse::Deny(tool));
            }
        }
        "q" | "Q" => {
            app.send_approval_response(ApprovalResponse::Quit);
        }
        _ => {
            // Invalid input, ignore
        }
    }
    return; // Don't process as chat
}
```

### 5. Logging Schema

#### 5.1 New Artifact Types
**File**: `src/execution_tools/db.rs` (update `validate_artifact_type` trigger)

```sql
-- Add to artifact_type validation:
SELECT CASE
    WHEN NEW.artifact_type NOT IN (
        -- ... existing types ...
        'approval_granted', 'approval_denied'
    ) THEN
        RAISE(ABORT, 'Invalid artifact_type')
END;
```

#### 5.2 Approval Event Recording

**File**: `src/execution_tools/record.rs` (add new methods)

```rust
impl ExecutionDb {
    /// Record approval granted event
    pub fn record_approval_granted(
        &self,
        session_id: &str,
        tool: &str,
        scope: &ApprovalScope,
        args: &std::collections::HashMap<String, String>,
    ) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let exec_id = format!("approval_granted_{}_{}", session_id, tool);

        let arguments = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "scope": format!("{:?}", scope),
            "args": args,
        });

        let artifact = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "scope": format!("{:?}", scope),
            "timestamp": timestamp,
        });

        self.record_execution_with_artifacts(
            &exec_id,
            "approval_granted",
            &arguments,
            timestamp,
            true,
            None, None, None,
            &[("approval_granted", &artifact)],
        )
    }

    /// Record approval denied event
    pub fn record_approval_denied(
        &self,
        session_id: &str,
        tool: &str,
        args: &std::collections::HashMap<String, String>,
        reason: &str,
    ) -> Result<()> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let exec_id = format!("approval_denied_{}_{}", session_id, tool);

        let arguments = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "args": args,
            "reason": reason,
        });

        let artifact = serde_json::json!({
            "session_id": session_id,
            "tool": tool,
            "reason": reason,
            "timestamp": timestamp,
        });

        self.record_execution_with_artifacts(
            &exec_id,
            "approval_denied",
            &arguments,
            timestamp,
            true,
            None, None, None,
            &[("approval_denied", &artifact)],
        )
    }
}
```

### 6. Module Splits (Required for LOC Compliance)

#### 6.1 Current File Sizes (Phase 9.1)
- `src/ui/state.rs`: 849 LOC → **MUST SPLIT**
- `src/ui/view.rs`: 467 LOC → near limit, may need split
- `src/llm/chat_loop.rs`: 706 LOC → **MUST SPLIT**

#### 6.2 Proposed Splits

**New file: `src/ui/approval.rs`** (<200 LOC)
- `ApprovalScope` enum
- `PendingApproval` struct
- `ApprovalState` struct
- `ApprovalResponse` enum

**New file: `src/ui/session.rs`** (extracted from state.rs, ~300 LOC)
- `current_chat_session_id`
- `chat_messages`
- `chat_error`
- Session lifecycle methods

**New file: `src/ui/chat_thread.rs`** (extracted from state.rs, ~200 LOC)
- `chat_thread_handle`
- `chat_event_receiver`
- Thread lifecycle methods

**New file: `src/llm/approval_loop.rs`** (extracted from chat_loop.rs, ~250 LOC)
- `handle_approval()` method
- `handle_denial()` method
- `execute_gated_tool()` method
- `wait_for_approval()` method

### 7. State Machine

#### 7.1 AppState Enum Extension
**File**: `src/ui/state.rs`

```rust
pub enum AppState {
    // ... existing variants ...

    /// Phase 9.2: Awaiting user approval for GATED tool
    AwaitingApproval,
}
```

#### 7.2 State Transitions

```
NormalInput → (GATED tool encountered) → AwaitingApproval
AwaitingApproval → (user responds) → NormalInput (loop continues)
AwaitingApproval → (user quits) → Quitting
```

### 8. Edge Cases

#### 8.1 /quit During Approval
- **Behavior**: Immediate exit, terminate chat loop, no tool execution
- **Implementation**: Send `ApprovalResponse::Quit`, loop terminates

#### 8.2 Ctrl+C During Approval
- **Behavior**: Same as /quit (existing behavior preserved)

#### 8.3 Timeout
- **Behavior**: If no response within 5 minutes, auto-deny and inject error
- **Rationale**: Prevent indefinite hangs

#### 8.4 Session Change
- **Behavior**: `ApprovalState::reset()` called on `ChatEvent::Started`
- **Implementation**: In `handle_chat_event()` for `Started` variant

#### 8.5 Multiple Pending Tools
- **Behavior**: Not possible — loop pauses on first GATED tool, blocks until response

## VALIDATION CRITERIA

1. All 8 integration tests pass (STEP 2)
2. No new clippy warnings
3. No file exceeds 300 LOC
4. Audit log contains approval events with scope
5. Session-scoped approval works (one 'a' allows multiple writes)
6. Single-tool approval works (one 'y' allows one write only)
7. Denial injects error but continues loop
8. /quit works immediately during approval

## IMPLEMENTATION ORDER (STEP 3)

1. Create `src/ui/approval.rs` with data structures
2. Create `src/llm/approval_loop.rs` with approval handling
3. Split `src/ui/state.rs` into smaller modules
4. Add `AwaitingApproval` state to `AppState`
5. Extend `ChatEvent::ApprovalRequired` with affected_path
6. Implement UI approval prompt in `view.rs`
7. Implement approval input handling in `handlers.rs`
8. Wire up approval_tx/approval_rx channel
9. Add logging methods to `ExecutionDb`
10. Update schema validation triggers
11. Write integration tests (STEP 2, TDD)
