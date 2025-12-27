# Phase 9: Chat Loop & Session Modularization

**Date**: 2025-12-26
**Status**: Planning Phase
**Objective**: Refactor large files (chat_loop.rs: 985 LOC, session.rs: 429 LOC) to meet 300 LOC constraint without losing features

---

## Problem Statement

**Current Violations**:
| File | Current LOC | Target | Excess |
|------|-------------|--------|--------|
| `src/llm/chat_loop.rs` | 985 | 300 | +685 |
| `src/llm/session.rs` | 429 | 300 | +129 |

**Root Cause**: Organic growth through multiple phases (9.0 → 9.9) without refactoring.

**Requirements**:
1. **Zero feature loss** — All existing functionality must be preserved
2. **Zero behavior changes** — All tests must continue to pass
3. **Proper separation of concerns** — Each module has single, clear responsibility
4. **TDD approach** — Tests first, prove they pass, then refactor

---

## Analysis of Current Structure

### `chat_loop.rs` (985 LOC) Breakdown

| Section | Lines | Purpose |
|---------|-------|---------|
| Module doc, imports, constants | 1-44 | Setup |
| `PendingGatedTool` struct | 71-79 | Pending tool data |
| `LoopState` struct + impl | 45-213 | Loop state management |
| `ChatLoop` struct | 215-223 | Main driver |
| `ChatLoop::process_event` | 258-421 | Event processing (164 LOC) |
| `ChatLoop::execute_tool_and_continue` | 423-467 | Tool execution |
| `ChatLoop::handle_approval` | 469-508 | Approval handling |
| `ChatLoop::handle_denial` | 510-552 | Denial handling |
| `ChatLoop::execute_gated_tool` | 554-618 | Gated execution |
| `LoopAction` enum | 642-665 | Action types |
| **Tests** | 667-985 | **318 LOC** |

### `session.rs` (429 LOC) Breakdown

| Section | Lines | Purpose |
|---------|-------|---------|
| Module doc, imports, error types | 1-36 | Setup |
| `render_plan_for_ui` | 38-72 | Plan rendering |
| `propose_plan` | 74-103 | Plan proposal |
| `log_plan_generation` | 105-174 | Plan logging |
| `propose_plan_streaming` | 176-215 | Streaming proposal |
| `log_stream_chunk` | 217-265 | Chunk logging |
| `log_plan_edit` | 267-318 | Edit logging |
| `LlmSession` struct + impl | 320-376 | Session state |
| **Tests** | 377-429 | **52 LOC** |

---

## Proposed New Structure

### Phase 9A: `chat_loop.rs` Modularization

**Target**: Split into 5 files, each < 300 LOC

```
src/llm/chat_loop/
├── mod.rs                    # Facade, exports, ~50 LOC
├── constants.rs              # Constants, deprecated items, ~40 LOC
├── loop_state.rs             # LoopState + PendingGatedTool, ~180 LOC
├── loop_action.rs            # LoopAction enum, ~30 LOC
├── chat_loop.rs              # ChatLoop struct (basic operations), ~200 LOC
└── event_handler.rs          # process_event logic, ~180 LOC
```

**Module Responsibilities**:

| File | Responsibility |
|------|----------------|
| `mod.rs` | Public API exports, module documentation |
| `constants.rs` | MAX_AUTO_STEPS, deprecated types |
| `loop_state.rs` | LoopState, PendingGatedTool, state management |
| `loop_action.rs` | LoopAction enum definition |
| `chat_loop.rs` | ChatLoop: new, set_sender, start, end, state |
| `event_handler.rs` | process_event, execute_tool, handle_approval/denial |

**Test file remains**: `tests/phase_9_*.rs` (no changes to test structure)

### Phase 9B: `session.rs` Modularization

**Target**: Split into 4 files, each < 300 LOC

```
src/llm/session/
├── mod.rs                    # Facade, exports, ~50 LOC
├── errors.rs                 # SessionError enum, ~40 LOC
├── plan_render.rs            # render_plan_for_ui, ~50 LOC
├── plan_proposal.rs          # propose_plan, propose_plan_streaming, ~100 LOC
├── plan_logging.rs           # log_plan_generation, log_stream_chunk, log_plan_edit, ~180 LOC
└── session_state.rs          # LlmSession struct, ~80 LOC
```

**Module Responsibilities**:

| File | Responsibility |
|------|----------------|
| `mod.rs` | Public API exports, module documentation |
| `errors.rs` | SessionError enum definition |
| `plan_render.rs` | Plan rendering for UI |
| `plan_proposal.rs` | Plan proposal (non-streaming + streaming) |
| `plan_logging.rs` | All logging functions (plan, stream, edit) |
| `session_state.rs` | LlmSession struct and impl |

**Test file remains**: `tests/phase_9_*.rs` (no changes to test structure)

---

## Implementation Steps

### Step 1: Test Baseline (Phase 9.0.1)

**Verify all existing tests pass before refactoring:**

```bash
# Run Phase 9 tests
cargo test --test phase_9_2_approval_tests
cargo test --test phase_9_3_observability_tests
cargo test --test phase_9_4_trace_viewer_tests
cargo test --test phase_9_5_tool_state_tests
cargo test --test phase_9_8_adapter_messages_tests

# Run library tests for chat_loop and session
cargo test --lib chat_loop
cargo test --lib session
```

**Exit criteria**: All tests pass, count documented.

### Step 2: Extract `chat_loop/constants.rs` (Phase 9.0.2)

**Create** `src/llm/chat_loop/constants.rs`:
```rust
//! Chat loop constants

/// Maximum AUTO tool steps per loop (safety limit)
pub const MAX_AUTO_STEPS: usize = 10;

/// DEPRECATED (Phase 9.7): Replaced by FrameStack with MAX_FRAMES limit
#[deprecated(since = "0.9.7", note = "Use FrameStack::MAX_FRAMES instead")]
pub const MAX_CONTEXT_MESSAGES: usize = 10;

use crate::execution_engine::ToolResult;

/// DEPRECATED (Phase 9.7): Use FrameStack::ToolResult instead
#[derive(Debug, Clone)]
#[deprecated(since = "0.9.7", note = "Use FrameStack::ToolResult instead")]
pub struct HiddenToolResult {
    pub tool: String,
    pub formatted: String,
}
```

**Tests**:
- `test_max_auto_steps_const`

### Step 3: Extract `chat_loop/loop_action.rs` (Phase 9.0.3)

**Create** `src/llm/chat_loop/loop_action.rs`:
```rust
//! Action returned by chat loop processing

/// Action returned by chat loop processing
#[derive(Debug)]
pub enum LoopAction {
    None,
    ExecuteTool(String, std::collections::HashMap<String, String>),
    ToolExecuted(ToolResult),
    ToolFailed(ToolResult),
    RequestApproval(String, std::collections::HashMap<String, String>),
    ToolApproved,
    ToolDenied,
    InjectError(String),
    LoopComplete(String),
    LoopError,
}
```

**Tests**: None (simple enum, no logic)

### Step 4: Extract `chat_loop/loop_state.rs` (Phase 9.0.4)

**Create** `src/llm/chat_loop/loop_state.rs`:
- `PendingGatedTool` struct
- `LoopState` struct
- `LoopState` impl (all methods except event processing)

**Tests**:
- `test_loop_state_new`
- `test_loop_state_should_continue`
- `test_loop_state_pause_resume`
- `test_loop_state_complete`
- `test_loop_state_add_hidden_result`
- `test_hidden_context_string`
- `test_pending_gated_tool`
- `test_prompt_mode_*` (all 9 tests)

### Step 5: Extract `chat_loop/event_handler.rs` (Phase 9.0.5)

**Create** `src/llm/chat_loop/event_handler.rs`:
- `process_event` logic
- `execute_tool_and_continue`
- `handle_approval`
- `handle_denial`
- `execute_gated_tool`

**Key**: These methods operate on `ChatLoop` but contain the bulk of the logic.

**Tests**: Integration tests (via ChatLoop)

### Step 6: Create `chat_loop/chat_loop.rs` (Phase 9.0.6)

**Refactor** `src/llm/chat_loop.rs` → `src/llm/chat_loop/chat_loop.rs`:
- `ChatLoop` struct definition
- `ChatLoop::new`
- `ChatLoop::set_sender`
- `ChatLoop::start`
- `ChatLoop::state`
- `ChatLoop::end`
- Delegate event handling to `event_handler` module

**Tests**:
- `test_chat_loop_new`
- `test_chat_loop_set_sender`
- `test_chat_loop_end`

### Step 7: Create `chat_loop/mod.rs` (Phase 9.0.7)

**Create** `src/llm/chat_loop/mod.rs`:
```rust
//! Chat loop — Multi-step tool execution (Phase 9.0 → 9.7)
//!
//! MAIN-thread only. Background thread does LLM I/O only.
//! Loop progresses via event processing (non-blocking).

mod constants;
mod loop_action;
mod loop_state;
mod chat_loop;
mod event_handler;

// Re-export public API
pub use constants::{MAX_AUTO_STEPS, MAX_CONTEXT_MESSAGES, HiddenToolResult};
pub use loop_action::LoopAction;
pub use loop_state::{LoopState, PendingGatedTool};
pub use chat_loop::ChatLoop;
```

**Update** `src/llm/mod.rs`:
```rust
// Before:
pub mod chat_loop;

// After:
pub mod chat_loop;  // Now refers to chat_loop/mod.rs
```

### Step 8: Extract `session/errors.rs` (Phase 9.0.8)

**Create** `src/llm/session/errors.rs`:
```rust
//! Session errors

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Plan error: {0}")]
    PlanError(#[from] crate::llm::planner::PlanError),

    #[error("Execution DB error: {0}")]
    ExecutionDbError(#[from] crate::execution_tools::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Execution recording error: {0}")]
    ExecutionRecordingError(String),

    #[error("Adapter error: {0}")]
    AdapterError(#[from] crate::llm::adapters::transport::AdapterError),

    #[error("LLM not configured")]
    LlmNotConfigured,
}
```

### Step 9: Extract `session/plan_render.rs` (Phase 9.0.9)

**Create** `src/llm/session/plan_render.rs`:
```rust
//! Plan rendering for UI display

use crate::llm::types::Plan;

/// Render plan for UI display
///
/// Pure function, no side effects.
/// UI displays this to user before execution.
pub fn render_plan_for_ui(plan: &Plan) -> String {
    // ... implementation
}
```

**Tests**:
- `test_render_plan_for_ui`

### Step 10: Extract `session/plan_proposal.rs` (Phase 9.0.10)

**Create** `src/llm/session/plan_proposal.rs`:
- `propose_plan`
- `propose_plan_streaming`

**Tests**: Integration (requires adapter)

### Step 11: Extract `session/plan_logging.rs` (Phase 9.0.11)

**Create** `src/llm/session/plan_logging.rs`:
- `log_plan_generation`
- `log_stream_chunk`
- `log_plan_edit`

**Tests**: Integration (requires ExecutionDb)

### Step 12: Extract `session/session_state.rs` (Phase 9.0.12)

**Create** `src/llm/session/session_state.rs`:
- `create_session`
- `LlmSession` struct
- `LlmSession` impl

**Tests**:
- `test_session_authorization`

### Step 13: Create `session/mod.rs` (Phase 9.0.13)

**Create** `src/llm/session/mod.rs`:
```rust
//! LLM session — UI integration boundary and logging
//!
//! Handles:
//! - Plan rendering for UI display
//! - User authorization tracking
//! - Logging interactions to execution_log.db

mod errors;
mod plan_render;
mod plan_proposal;
mod plan_logging;
mod session_state;

// Re-export public API
pub use errors::SessionError;
pub use plan_render::render_plan_for_ui;
pub use plan_proposal::{propose_plan, propose_plan_streaming};
pub use plan_logging::{log_plan_generation, log_stream_chunk, log_plan_edit};
pub use session_state::{create_session, LlmSession};
```

**Update** `src/llm/mod.rs`:
```rust
// Before:
pub mod session;

// After:
pub mod session;  // Now refers to session/mod.rs
```

### Step 14: Final Verification (Phase 9.0.14)

**Run all tests**:
```bash
# All Phase 9 tests
cargo test --test phase_9

# Library unit tests
cargo test --lib chat_loop
cargo test --lib session

# Full test suite
cargo test
```

**Verify LOC**:
```bash
wc -l src/llm/chat_loop/*.rs
wc -l src/llm/session/*.rs
```

**Exit criteria**: All files < 300 LOC, all tests pass.

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| Breaking imports | Use `mod.rs` re-exports, transparent to external callers |
| Test breakage | Run tests after EACH step, fix before proceeding |
| Circular dependencies | Clear dependency hierarchy: constants → types → state → logic |
| Feature loss | TDD: write test for each feature, prove it passes, then move |

---

## Success Criteria

1. ✅ All files < 300 LOC
2. ✅ All existing tests pass (100% pass rate)
3. ✅ No changes to public API (transparent to callers)
4. ✅ Clear separation of concerns (each module single responsibility)
5. ✅ No feature loss (all functionality preserved)
6. ✅ No behavior changes (deterministic outputs unchanged)

---

## File Size Targets

| File | Target LOC | Actual After |
|------|------------|--------------|
| `chat_loop/mod.rs` | 50 | TBD |
| `chat_loop/constants.rs` | 40 | TBD |
| `chat_loop/loop_action.rs` | 30 | TBD |
| `chat_loop/loop_state.rs` | 180 | TBD |
| `chat_loop/chat_loop.rs` | 200 | TBD |
| `chat_loop/event_handler.rs` | 180 | TBD |
| `session/mod.rs` | 50 | TBD |
| `session/errors.rs` | 40 | TBD |
| `session/plan_render.rs` | 50 | TBD |
| `session/plan_proposal.rs` | 100 | TBD |
| `session/plan_logging.rs` | 180 | TBD |
| `session/session_state.rs` | 80 | TBD |

---

## Timeline Tracking

| Step | Status | Date |
|------|--------|------|
| 9.0.1: Test Baseline | ⏳ Pending | |
| 9.0.2: constants.rs | ⏳ Pending | |
| 9.0.3: loop_action.rs | ⏳ Pending | |
| 9.0.4: loop_state.rs | ⏳ Pending | |
| 9.0.5: event_handler.rs | ⏳ Pending | |
| 9.0.6: chat_loop.rs | ⏳ Pending | |
| 9.0.7: chat_loop/mod.rs | ⏳ Pending | |
| 9.0.8: errors.rs | ⏳ Pending | |
| 9.0.9: plan_render.rs | ⏳ Pending | |
| 9.0.10: plan_proposal.rs | ⏳ Pending | |
| 9.0.11: plan_logging.rs | ⏳ Pending | |
| 9.0.12: session_state.rs | ⏳ Pending | |
| 9.0.13: session/mod.rs | ⏳ Pending | |
| 9.0.14: Final Verification | ⏳ Pending | |

---

*Last Updated: 2025-12-26*
*Status: Planning Complete — Ready to Begin*
