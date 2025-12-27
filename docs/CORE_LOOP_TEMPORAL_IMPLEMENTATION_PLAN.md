# Temporal Grounding Implementation Plan

**Goal:** Force the LLM to ground every action in execution timeline truth — no guessing, no memory, no "I think."

## Core Principle

> **The LLM must read before it acts.**

Before any action, the LLM must query memory to answer:
- Where am I in the timeline?
- What happened immediately before?
- What is the current state?

---

## Files to Modify

### 1. Core Execution Memory
| File | Changes |
|------|---------|
| `src/execution_tools/db.rs` | Add temporal query helpers, ensure timestamp indexing |
| `src/execution_tools/memory_query.rs` | Add timeline-specific queries, `last_n_executions`, `get_position` |
| `src/execution_tools/execution_summary.rs` | Add timeline summary: current step, recent failures, pending work |

### 2. Tool Execution Layer
| File | Changes |
|------|---------|
| `src/execution_engine/tool_mapper.rs` | Add pre-flight memory check for all mutation tools |
| `src/execution_engine/result.rs` | Add execution timestamp to all results |
| `src/execution_engine/errors.rs` | Add error type: `NoGrounding` — when model acts without query |

### 3. LLM Integration
| File | Changes |
|------|---------|
| `src/llm/chat.rs` | Add timeline context to every prompt |
| `src/llm/chat_events.rs` | Add event: `TimelineCheckpoint` — forces model position awareness |
| `src/llm/types.rs` | Add `TimelinePosition` struct — step index, last query time |
| `src/llm/planner.rs` | Validate plan includes grounding queries before edits |

### 4. UI Layer
| File | Changes |
|------|---------|
| `src/ui/state.rs` | Add `timeline_position: TimelinePosition` to app state |
| `src/ui/view.rs` | Show current timeline position in tool result panel |

---

## New Workflow: Pre-Flight Grounding

### Before ANY Tool Call

The system enforces:

```
LLM Request: "I want to edit src/foo.rs"
     ↓
System Check: When was your last memory_query?
     ↓
If > 5 seconds ago → REJECT with error:
     "Must query timeline before action. Call memory_query first."
     ↓
LLM Calls: memory_query(timeline_summary, last=10)
     ↓
System Returns: Timeline with execution IDs, timestamps, outcomes
     ↓
LLM Now Grounded: "I see step 3 failed with type mismatch at src/foo.rs:42"
     ↓
Action Allowed
```

---

## New Memory Query API

### Timeline Queries (Add to `memory_query.rs`)

```rust
/// Get current position in execution timeline
pub fn get_timeline_position(db_root: &Path) -> Result<TimelinePosition> {
    // Returns:
    // - total_executions (where we are)
    // - last_execution_id
    // - last_success
    // - last_failure
    // - current_step_number
}

/// Get last N executions in chronological order
pub fn get_recent_timeline(db_root: &Path, n: usize) -> Result<Vec<TimelineEntry>> {
    // Returns: last N executions with:
    // - execution_id
    // - timestamp
    // - tool_name
    // - success/failure
    // - affected_path
    // - error_summary (if failed)
}

/// Get what happened immediately before current action
pub fn get_preceding_context(db_root: &Path, before_id: i64) -> Result<Vec<TimelineEntry>> {
    // Returns: executions that led up to this point
}

/// Get unresolved failures (things that must be fixed)
pub fn get_pending_failures(db_root: &Path) -> Result<Vec<FailureRecord>> {
    // Returns: failures without corresponding successful retry
}
```

---

## Grounding Protocol: Step by Step

### Phase 1: Before Planning

```
LLM: "User wants to change X"

REQUIRED QUERY:
memory_query(get_timeline_position)

RESULT:
{
  "position": {
    "total_executions": 142,
    "last_execution_id": 142,
    "current_step": 5,
    "last_execution": {
      "id": 142,
      "tool": "lsp_check",
      "success": false,
      "error": "Type mismatch in src/foo.rs:42"
    }
  }
}

LLM NOW KNOWS:
- "I am at execution 142"
- "Last thing I did was LSP check that failed"
- "I am in step 5 of the loop"
```

### Phase 2: Before Editing

```
LLM: "I need to fix src/foo.rs:42"

REQUIRED QUERY:
memory_query(get_recent_timeline, n=5)

RESULT (CHRONOLOGICAL):
[
  {id: 138, tool: "file_edit", path: "src/foo.rs", success: true},
  {id: 139, tool: "lsp_check", success: false, error: "Type mismatch"},
  {id: 140, tool: "file_edit", path: "src/foo.rs", success: true},
  {id: 141, tool: "lsp_check", success: false, error: "Type mismatch"},
  {id: 142, tool: "lsp_check", success: false, error: "Type mismatch"}
]

LLM NOW KNOWS:
- "I tried editing twice (138, 140)"
- "Both edits failed LSP (139, 141, 142)"
- "I am in a failure loop"
- "Must query the actual errors to understand what's wrong"
```

### Phase 3: Before Declaring Success

```
LLM: "I think the fix worked"

REQUIRED QUERY:
memory_query(
    tool="lsp_check",
    success=true,
    after_id=142
)

RESULT:
[
  {id: 143, tool: "lsp_check", success: true}
]

LLM NOW KNOWS:
- "LSP passed at execution 143"
- "I can confirm success"
```

---

## Enforcement Mechanism: The Grounding Gate

### In `tool_mapper.rs` — Pre-Flight Check

```rust
/// Invoke a tool with grounding check
pub fn invoke_tool_with_grounding(
    step: &Step,
    exec_db: &ExecutionDb,
    last_query_time: Option<u64>,
) -> Result<ToolInvocation, ExecutionError> {
    // Mutation tools require recent memory query
    if is_mutation_tool(&step.tool) {
        let now = current_timestamp_ms();

        // Require memory query within last 10 seconds
        let required = last_query_time.map_or(true, |t| now - t > 10_000);

        if required {
            return Err(ExecutionError::GroundingRequired {
                tool: step.tool.clone(),
                reason: "Must call memory_query before mutation tool".to_string(),
                required_query: "memory_query(timeline_summary)".to_string(),
            });
        }
    }

    invoke_tool(step, exec_db, magellan_db)
}

fn is_mutation_tool(tool: &str) -> bool {
    matches!(tool,
        "file_edit" | "file_write" | "file_create" | "splice_patch"
    )
}
```

### New Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Grounding required: {reason}")]
    GroundingRequired {
        tool: String,
        reason: String,
        required_query: String,
    },
    // ... other errors
}
```

---

## Prompt Engineering: Timeline Context Injection

### In `chat.rs` — Every Prompt Includes

```rust
/// Inject timeline context into every prompt
fn inject_timeline_context(prompt: &str, position: &TimelinePosition) -> String {
    format!(
        "=== EXECUTION TIMELINE (GROUND TRUTH) ===\n\
         Current Position: Step {}\n\
         Total Executions: {}\n\
         Last Execution: #{} ({})\n\
         Last Query: {}ms ago\n\
         \n\
         REQUIRED: Before acting, you MUST query memory:\n\
         - Use memory_query(timeline_summary) to see recent history\n\
         - Use memory_query(tool=\"X\", success=false) to see failures\n\
         - Reference execution IDs, not memory\n\
         \n\
         === YOUR MESSAGE ===\n\
         {}",
        position.current_step,
        position.total_executions,
        position.last_execution_id,
        position.last_execution_status,
        position.time_since_last_query_ms,
        prompt
    )
}
```

---

## UI Display: Timeline Position

### In Tool Result Panel (Phase 9.7)

```
┌─────────────────────────────────────────┐
│ [Tool Result]                            │
├─────────────────────────────────────────┤
│ Timeline Position: Step 5 of ???         │
│ Last Execution: #142 (lsp_check) FAILED  │
│ Time Since Query: 2s ago ✓               │
│                                          │
│ ─────────────────────────────────────── │
│                                          │
│ RECENT HISTORY (last 5):                 │
│                                          │
│ #138 file_edit   src/foo.rs    SUCCESS  │
│ #139 lsp_check                  FAILED  │
│       → "Type mismatch at line 42"       │
│ #140 file_edit   src/foo.rs    SUCCESS  │
│ #141 lsp_check                  FAILED  │
│       → "Type mismatch at line 42"       │
│ #142 lsp_check                  FAILED  │
│       → "Type mismatch at line 42"       │
│                                          │
│ You are in a failure loop. Query memory │
│ for details before retrying.            │
└─────────────────────────────────────────┘
```

---

## Validation: No Guessing Test Cases

### Test 1: Edit Without Query → REJECT

```rust
#[test]
fn test_edit_without_memory_query_rejected() {
    // User says "edit src/foo.rs"
    // Last memory_query was 60 seconds ago
    // System: REJECT with GroundingRequired error
}
```

### Test 2: Declare Success Without Verification → REJECT

```rust
#[test]
fn test_success_without_verification_rejected() {
    // LLM says "I fixed it"
    // But no recent successful lsp_check in memory
    // System: REJECT with "Must verify success via lsp_check"
}
```

### Test 3: Fix References Wrong Execution ID → REJECT

```rust
#[test]
fn test_fix_references_nonexistent_id_rejected() {
    // LLM says "Fixing error from execution 999"
    // But execution 999 doesn't exist
    // System: REJECT with "Execution 999 not found"
}
```

### Test 4: Timeline Query Returns Ordered Results

```rust
#[test]
fn test_timeline_returns_chronological_order() {
    // Query last 5 executions
    // Results MUST be ordered by timestamp ascending
    // Each result includes execution_id, timestamp, outcome
}
```

---

## Implementation Order

### Phase 1: Memory Timeline Queries (1-2 days)
1. Add `get_timeline_position()` to `memory_query.rs`
2. Add `get_recent_timeline()` to `memory_query.rs`
3. Add `get_pending_failures()` to `memory_query.rs`
4. Add tests for chronological ordering

### Phase 2: Grounding Gate (1 day)
1. Add `GroundingRequired` error to `execution_engine/errors.rs`
2. Add pre-flight check to `tool_mapper.rs`
3. Track `last_query_time` in `App` state
4. Test that edits are rejected without recent query

### Phase 3: Prompt Integration (1 day)
1. Add `TimelinePosition` struct to `llm/types.rs`
2. Inject timeline into every prompt in `chat.rs`
3. Add instruction: "Reference execution IDs, not memory"
4. Test that prompts include timeline context

### Phase 4: UI Display (1 day)
1. Add timeline position to `ui/state.rs`
2. Update tool result panel to show timeline
3. Show recent history with execution IDs
4. Test timeline display

### Phase 5: Validation Tests (1 day)
1. Write test cases for rejection scenarios
2. Test forced memory queries
3. Verify no guessing is possible
4. Integration test: full loop with grounding

**Total: 5-7 days**

---

## Success Criteria

The implementation succeeds when:

1. ✅ Every mutation tool requires a `memory_query` within last 10 seconds
2. ✅ Timeline is always visible in the UI (current position, recent history)
3. ✅ Every prompt includes current timeline position
4. ✅ LLM responses reference execution IDs, not "I remember"
5. ✅ Declaring success requires a `memory_query` verification
6. ✅ Tests prove rejection when protocol is violated

---

## Key Insight

> **Context tells you what was SAID. Timeline tells you what HAPPENED.**

The LLM must read the timeline to know where it is.
The LLM must reference execution IDs to prove it's grounded.

No guessing. No "I think." No "I remember."

Only: "Execution log shows..."
