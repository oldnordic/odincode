# Phase 9.2 — Gated Tools & Approvals: Implementation Notes

## STEP 0 FINDINGS

### 1. Existing Infrastructure (Phase 9.0)

#### 1.1 PendingGatedTool Struct
**Location**: `src/llm/chat_loop.rs:63-70`

```rust
/// Pending GATED tool (awaiting user approval)
#[derive(Debug, Clone)]
pub struct PendingGatedTool {
    /// Tool name
    pub tool: String,
    /// Tool arguments
    pub args: std::collections::HashMap<String, String>,
    /// Step number when tool was requested
    pub step: usize,
}
```

#### 1.2 LoopState Pause/Resume
**Location**: `src/llm/chat_loop.rs:72-148`

```rust
impl LoopState {
    /// Pause the loop (waiting for GATED tool approval)
    pub fn pause(&mut self, pending: PendingGatedTool) {
        self.paused = true;
        self.pending_gated_tool = Some(pending);
    }

    /// Resume the loop (after user approval/denial)
    pub fn resume(&mut self) {
        self.paused = false;
        self.pending_gated_tool = None;
    }

    /// Get pending tool if paused
    pub fn pending_tool(&self) -> Option<&PendingGatedTool> {
        self.pending_gated_tool.as_ref()
    }
}
```

**CRITICAL**: Pause mechanism already exists. Loop can be paused mid-execution and resumed later.

#### 1.3 LoopAction Enum
**Location**: `src/llm/chat_loop.rs:511-540`

```rust
pub enum LoopAction {
    None,
    ExecuteTool(String, std::collections::HashMap<String, String>),
    ToolExecuted(ToolResult),
    ToolFailed(ToolResult),
    RequestApproval(String, std::collections::HashMap<String, String>),
    ToolApproved,
    ToolDenied,
    InjectErrorAndContinue(String),
}
```

**Key Finding**: `RequestApproval` action exists but is not fully connected to UI flow.

### 2. Tool Classification

**Location**: `src/execution_engine/chat_tool_runner.rs:14-34`

```rust
/// AUTO tools - execute immediately without approval
pub const AUTO_TOOLS: &[&str] = &[
    "file_read",
    "file_search",
    "file_glob",
    "symbols_in_file",
    "references_to_symbol_name",
    "references_from_file_to_symbol_name",
    "lsp_check", // AUTO but rate-limited
];

/// GATED tools - require user approval (Phase 9.1)
pub const GATED_TOOLS: &[&str] = &[
    "file_write",
    "file_create",
];

/// FORBIDDEN tools - never executable in chat loop
pub const FORBIDDEN_TOOLS: &[&str] = &[
    "splice_patch",
    "splice_plan",
];
```

**Key Finding**: Tool classification is already in place. Need to add approval UX.

### 3. ChatEvent::ApprovalRequired Handling (Current STUB)

**Location**: `src/ui/state.rs:687-695`

```rust
ChatEvent::ApprovalRequired { tool, .. } => {
    // Clear any tool status
    self.clear_tool_status();
    // Log that approval is needed
    self.log(format!("[Loop] Approval required for: {} (Phase 9.1 will implement UI)", tool));
    // Cleanup thread state (loop paused, waiting for UI implementation)
    self.cleanup_chat_thread();
    return true; // Terminal for now (will resume in 9.2)
}
```

**CRITICAL GAP**: This currently terminates the loop. Need to implement:
1. Enter approval state (don't terminate)
2. Store pending approval details
3. Render approval prompt
4. Handle user response (y/a/n)
5. Resume loop with decision

### 4. Session Management

**Location**: `src/ui/state.rs:68`

```rust
/// Current chat session ID (from chat_thread)
pub current_chat_session_id: Option<String>,
```

**Set on**: `ChatEvent::Started` (state.rs:628)

**Key Finding**: Session ID already tracked. Can use for approval scoping.

### 5. Execution Logging Schema

**Location**: `src/execution_tools/db.rs:108-186`

Existing tables:
- `executions` (id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message)
- `execution_artifacts` (id, execution_id, artifact_type, content_json)

**Validated artifact_types** (db.rs:226):
- `stdout`, `stderr`, `diagnostics`, `prompt`, `plan`, `validation_error`
- `llm_preflight`, `llm_plan_stream`, `plan_edit`
- `adapter_call`, `adapter_response`, `adapter_stream_chunk`, `adapter_error`
- `chat_user_message`, `chat_assistant_message`, `chat_session`, `chat_summary`

**NEEDED**: Add artifact types for approval events:
- `approval_granted` (tool, scope, args, session_id, timestamp)
- `approval_denied` (tool, args, session_id, timestamp, reason)

### 6. Chat Thread Architecture

**Location**: `src/llm/chat_thread.rs:99-163`

```rust
pub fn spawn_chat_thread(
    db_root: &Path,
    user_message: String,
    tx: ChatSender,
) -> ChatThreadHandle
```

**Key Finding**: Thread is fire-and-forget (one-way communication via mpsc).
**NEEDED**: Bidirectional channel for UI → loop communication (approval response).

### 7. Chat Loop Integration

**Location**: `src/llm/chat_loop.rs:240-284`

```rust
ChatToolCategory::Gated => {
    // Pause loop, request approval
    let pending = PendingGatedTool {
        tool: tool_call.tool.clone(),
        args: tool_call.args.clone(),
        step: state.step,
    };
    state.pause(pending);
    LoopAction::RequestApproval(tool_call.tool, tool_call.args)
}
```

**Key Finding**: Loop pauses when GATED tool encountered. `LoopAction::RequestApproval` returned.

### 8. GATED Tool Execution (Already Implemented)

**Location**: `src/llm/chat_loop.rs:430-487`

```rust
fn execute_gated_tool(
    &mut self,
    tool: &str,
    args: &std::collections::HashMap<String, String>,
    exec_db: &ExecutionDb,
) -> ToolResult {
    // ... executes GATED tool via tool_mapper::invoke_tool
}
```

**Key Finding**: Execution logic exists. Need to wire approval to call this.

### 9. ToolResult Structure (Phase 9.1)

**Location**: `src/execution_engine/chat_tool_runner.rs:48-62`

```rust
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub output_full: String,
    pub output_preview: String,
    pub error_message: Option<String>,
    pub affected_path: Option<String>,  // Phase 9.1: for UI synchronization
}
```

**Key Finding**: `affected_path` already available for file synchronization.

## DESIGN GAPS (To be filled in STEP 1)

### A. Approval State Storage
- **Where**: Add to `App` struct in `src/ui/state.rs`
- **Fields**:
  - `pending_approval: Option<PendingApproval>`
  - `approved_all_gated: bool` (per-session)
  - `approved_tools: HashSet<String>` (per-session)

### B. PendingApproval Structure
- **Fields**:
  - `session_id: String`
  - `tool: String`
  - `args: HashMap<String, String>`
  - `step: usize`
  - `affected_path: Option<String>` (from ToolResult)

### C. ApprovalScope Enum
```rust
pub enum ApprovalScope {
    Once { tool: String },      // Approve this specific tool invocation
    SessionAllGated,            // Approve all GATED tools for this session
}
```

### D. Bidirectional Channel
- **Need**: Channel from UI back to paused chat loop
- **Options**:
  1. `mpsc::Sender<ApprovalDecision>` stored in LoopState
  2. `Arc<Mutex<ApprovalState>>` shared between UI and loop
  3. `crossbeam::channel` for select() on multiple events

**RECOMMENDATION**: Option 1 - simpler, fits existing architecture.

### E. Approval Event Logging
Add to `execution_artifacts`:
- `approval_granted`: {session_id, tool, scope, args, timestamp}
- `approval_denied`: {session_id, tool, args, timestamp, reason}

## FILE SIZE CONSTRAINTS

**MAX 300 LOC per module** (non-negotiable)

Current file sizes (from Phase 9.1):
- `src/ui/state.rs`: 849 LOC — **MUST SPLIT** before adding approval state
- `src/ui/view.rs`: 467 LOC — near limit
- `src/llm/chat_loop.rs`: 706 LOC — **MUST SPLIT**

**Suggested splits**:
- `src/ui/approval.rs` (new) — approval state, pending approval struct, prompt rendering
- `src/llm/approval_loop.rs` (new) — approval state machine in chat loop

## TESTS NEEDED (STEP 2 - TDD)

1. `test_gated_tool_prompts_once` — first GATED tool prompts
2. `test_approve_all_allows_multiple_writes` — session scope approval
3. `test_single_tool_approval_scope` — once vs session behavior
4. `test_denied_tool_is_skipped_and_injected` — denial flow
5. `test_denial_does_not_end_loop` — loop continues after denial
6. `test_approval_cleared_on_new_session` — new chat clears approvals
7. `test_gated_tool_logged_with_scope` — audit logging
8. `test_quit_during_approval_exits_immediately` — /quit works during approval

## REFERENCES

- **CONTRACT**: `docs/CONTRACT.md` — Phase 0 contract, TDD requirements
- **Phase 9.0**: `docs/PHASE_9_0_*` — Multi-step tool loop
- **Phase 9.1**: `docs/PHASE_9_1_*` — File tree and active file sync
- **chat_loop.rs**: `src/llm/chat_loop.rs:1-706` — Loop state machine
- **chat_events.rs**: `src/llm/chat_events.rs:1-119` — Event definitions
- **chat_tool_runner.rs**: `src/execution_engine/chat_tool_runner.rs:1-422` — Tool classification
- **state.rs**: `src/ui/state.rs:1-849` — App state (needs split)
- **view.rs**: `src/ui/view.rs:1-467` — Panel rendering
