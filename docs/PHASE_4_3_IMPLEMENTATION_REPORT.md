# Phase 4.3 Implementation Report: NLP-First UI (Chat-by-Default) + Error Surfacing

**Date**: 2025-12-24
**Status**: COMPLETE
**Test Coverage**: 243/243 tests passing (added 10 new tests)
**Type**: ERGONOMICS PHASE — UI input routing + presentation only

## Overview

Phase 4.3 transforms the TUI into a "Claude Code"-style NLP-first interface. Users can now type natural language without command prefixes, and the UI automatically routes to the LLM planner. All LLM failures are surfaced visibly in the UI with dedicated error states.

### Design Goals

1. **Chat-by-Default** — Non-":" input → NLP intent → LLM planner
2. **Error Surfacing** — All failures shown in UI, no silent drops
3. **State Machine** — PlanningInProgress, PlanReady, PlanError states
4. **No Scope Creep** — UI routing + presentation only, no tool semantics changes

## Implementation

### Files Modified

| File | LOC Before | LOC After | Changes |
|------|-----------|-----------|---------|
| `src/ui/input.rs` | 176 | 259 | Added NlpIntent, Help, Find, Plan variants; chat-by-default routing |
| `src/ui/state.rs` | 143 | 226 | Added AppState variants; planning fields; transition methods |
| `src/ui/view.rs` | 239 | 293 | Added planning state rendering; LLM error display |
| `src/ui/mod.rs` | 37 | 41 | Added handlers module; LlmError variant |
| `src/ui/handlers.rs` | 0 | 209 | NEW: Command handlers (refactored from main.rs) |
| `src/main.rs` | 296 | 258 | Updated for new routing; y/n approval handlers |
| `tests/ui_nlp_mode_tests.rs` | 0 | 334 | NEW: NLP mode integration tests |

### Files Created

| File | LOC | Purpose |
|------|-----|---------|
| `src/ui/handlers.rs` | 209 | TUI command handlers (moved from main.rs for LOC compliance) |
| `tests/ui_nlp_mode_tests.rs` | 334 | Integration tests for NLP-first UI |

### New Commands

| Command | Syntax | Purpose |
|---------|--------|---------|
| **:help** | `:help` or `:h` | Show help with "type natural language" |
| **:find** | `:find <pattern>` | Find symbols/files via Magellan |
| **:plan** | `:plan <intent>` | Explicit plan command (power users) |

### New Command Variants

```rust
pub enum Command {
    // Existing...
    None,
    Quit,
    Open(String),
    Read(String),
    Lsp(String),
    Evidence { query: String, args: Vec<String> },

    // NEW in Phase 4.3
    Help,           // Show help message
    Find(String),   // Find symbols/files
    Plan(String),   // Explicit plan command
    NlpIntent(String), // Natural language intent (chat-by-default)
}
```

### New AppState Variants

```rust
pub enum AppState {
    Running,                // Normal operation
    Quitting,               // Exit requested
    PlanningInProgress,     // LLM is generating plan (NEW)
    PlanReady,              // Plan generated, waiting approval (NEW)
    PlanError,              // LLM failed, showing error (NEW)
}
```

### New App Fields

```rust
pub struct App {
    // ... existing fields ...

    // Phase 4.3: Planning state
    pub current_plan: Option<Plan>,
    pub plan_error: Option<String>,
    pub planning_message: Option<String>,
    pub llm_session: Option<LlmSession>,
    planning_state: PlanningState, // Internal enum
}
```

## New Input Routing

### Phase 4.2 (Before)

```
User input → Parse as :command → Execute
Non-":" input → Command::None → Ignored
```

### Phase 4.3 (After)

```
User input → Starts with ":"?
    ├── YES → Parse as command
    │   ├── :q, :quit → Quit
    │   ├── :open <path> → Open file
    │   ├── :read <path> → Read file
    │   ├── :lsp [path] → LSP check
    │   ├── :evidence <query> → Query evidence
    │   ├── :help → Show help (NEW)
    │   ├── :find <pattern> → Find symbols (NEW)
    │   └── :plan <intent> → Explicit plan (NEW)
    └── NO → NlpIntent(text) → LLM planner → PlanReady → y/n
```

## Error Surfacing

### All LLM Errors Are Visible

When LLM fails, the UI shows:

```
┌─ LLM Error ─────────────────────┐
│                                   │
│ Plan validation failed:          │
│ Invalid evidence query: Q99      │
│                                   │
│ Press Enter to continue           │
└───────────────────────────────────┘
```

### Execution Memory Logging

All failures are logged with:
- `tool_name = "llm_plan"`
- `success = false`
- `artifact_type = "validation_error"` or `"stderr"`

## Test Coverage

### New Tests Added (10 tests)

| Test | Scenario |
|------|----------|
| `test_a_nlp_intent_routes_to_llm_planner` | Non-":" input routes to NlpIntent |
| `test_b_command_input_does_not_call_llm` | ":" commands don't trigger LLM |
| `test_c_llm_failure_logged_to_execution_memory` | Plan validation errors logged |
| `test_d_help_command_renders_correctly` | :help contains "natural language" |
| `test_e_find_command_returns_sorted_results` | :find returns deterministic results |
| `test_f_planning_state_transitions` | AppState transitions correctly |
| `test_g_unknown_command_rejected` | Unknown commands return None |
| `test_h_find_command_parsing` | :find parses pattern correctly |
| `test_i_plan_command_parsing` | :plan parses intent correctly |
| `test_j_empty_input_returns_none` | Empty/whitespace handling |

### Full Test Results

```
Total: 243/243 tests passing (10 new NLP mode tests)
```

## Constraints Compliance

| Constraint | Status |
|------------|--------|
| NO async | ✅ All code is synchronous |
| NO background threads | ✅ No thread spawning |
| NO network calls | ✅ No HTTP/requests |
| No tool semantics changes | ✅ Tools unchanged |
| No evidence schema changes | ✅ Schema unchanged |
| Max 300 LOC per file | ✅ All files comply |
| Chat-by-default routing | ✅ Implemented |
| Error surfacing | ✅ All failures visible |

## User Experience Changes

### Before (Phase 4.2)

```
:read src/lib.rs
```

### After (Phase 4.3)

```
read src/lib.rs           ← No ":" needed!
:find main                 ← NEW command
:help                      ← Shows "type natural language"
```

### Planning Flow

```
1. User types: "fix the error in main"
2. UI shows: "Planning..." (yellow border)
3. Plan generated: "Plan Ready!" (green border)
4. User types: 'y'
5. Plan executed (stub in Phase 4.3, full in future phase)
```

## File Size Verification

All files comply with the 300 LOC constraint:

```
src/ui/handlers.rs: 209 lines ✓
src/ui/input.rs:     259 lines ✓
src/ui/mod.rs:        41 lines ✓
src/ui/state.rs:     226 lines ✓
src/ui/view.rs:       293 lines ✓
src/main.rs:          258 lines ✓
```

## Verification

```bash
$ cargo test
test result: ok. 243 passed; 0 failed; 0 ignored
```

All 243 tests pass, including:
- 10 new NLP mode tests
- 14 UI smoke tests
- All existing tests (no regressions)

## Sign-Off

**Implementation**: Complete
**Tests**: All passing (243/243)
**Documentation**: This report

---

STOP — Phase 4.3 NLP-First UI implementation complete; awaiting acceptance or revisions.
