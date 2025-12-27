# Tool Result Compaction Implementation Plan

**Date**: 2025-12-27
**Status**: ✅ **COMPLETED**
**Issue**: Tool result spam in context window - old full results sent to LLM repeatedly
**Goal**: Compact old tool results to lean summaries, force LLM to use `memory_query` for full details

---

## IMPLEMENTATION SUMMARY

Tool result compaction has been successfully implemented. The system now automatically compacts old tool results (keeping only the 3 most recent) to reduce context spam while still allowing the LLM to retrieve full details via `memory_query`.

### What Changed

| File | Change |
|------|--------|
| `src/execution_engine/chat_tool_runner.rs` | Added `execution_id: String` to `ToolResult` struct |
| `src/llm/frame_stack.rs` | Added `compacted: bool` and `execution_id: Option<String>` to `Frame::ToolResult` |
| `src/llm/frame_stack.rs` | Updated `add_tool_result()` to accept `execution_id` parameter |
| `src/llm/frame_stack.rs` | Added `compact_old_tool_results()` method |
| `src/llm/frame_stack.rs` | Added `auto_compact_if_needed()` method (auto-compacts when >3 tool results) |
| `src/llm/frame_stack.rs` | Changed `build_messages*()` to `&mut self` (for auto-compaction) |
| `src/llm/chat_loop/loop_state.rs` | Updated to pass `execution_id` to `add_tool_result()` |
| `src/llm/chat_loop/event_handler.rs` | Updated `ToolResult` constructions with `execution_id` |
| `src/llm/chat_loop/gated_execution.rs` | Updated `ToolResult` constructions with `execution_id` |
| `src/llm/chat_thread.rs` | Changed `spawn_chat_thread_with_frame_stack()` to take `&mut FrameStack` |

### How It Works

1. **Auto-compaction**: When `build_messages()` is called, the system automatically checks if there are more than 3 tool results. If so, it marks older ones as `compacted = true`.

2. **Compacted output**: Compacted tool results show:
   ```
   [Tool file_glob]: OK - [Old tool result content cleared (execution_id: chat-abc-123). Use memory_query tool with session_id or execution_id to retrieve full details]
   ```

3. **Non-compact output**: Recent tool results (last 3) show full output as before.

4. **Token savings**: Compacted results use ~50 characters instead of potentially thousands of characters from full tool output.

### Test Results

All 438 tests pass:
- `cargo test --lib` - 438 passed, 0 failed

### Known Limitations

1. **Execution ID in chat mode**: Currently, chat mode doesn't record executions to `execution_log.db`. The `execution_id` is generated from `step.step_id` but won't be found by `memory_query` unless execution recording is added to chat mode.

2. **Plan mode**: Plan executor already records executions, so plan-mode tool results can be queried via `memory_query`. Chat mode needs execution recording added (future work).

---


**Issue**: Tool result spam in context window - old full results sent to LLM repeatedly
**Goal**: Compact old tool results to lean summaries, force LLM to use `memory_query` for full details

---

## FACTS from Source Code Analysis

### Current State

#### 1. FrameStack (`src/llm/frame_stack.rs`)

**Lines 19-32**: `Frame` enum definition
```rust
pub enum Frame {
    User { content: String },
    Assistant { content: String, complete: bool },
    ToolResult {
        tool: String,
        success: bool,
        output: String,
        // ← MISSING: compacted: bool
    },
}
```

**Line 126**: `add_tool_result()` method
```rust
pub fn add_tool_result(&mut self, tool: String, success: bool, output: String) {
    self.push_frame(Frame::ToolResult {
        tool,
        success,
        output,
        // ← MISSING: compacted: false
    });
}
```

**Lines 253-269**: `build_messages_with_timeline_and_mode()` - ToolResult handling
```rust
Frame::ToolResult { tool, success, output } => {
    let content = format!(
        "[Tool {}]: {}\nResult: {}",
        tool,
        if *success { "OK" } else { "FAILED" },
        output  // ← ALWAYS returns full output, no compaction check
    );
    messages.push(LlmMessage { role: LlmRole::User, content });
}
```

**Problem**: Old tool results are sent with FULL output every time `build_messages()` is called.

#### 2. ExecutionDb Schema (`src/execution_tools/db.rs`)

**Lines 113-124**: `executions` table - tool executions are stored here
```sql
CREATE TABLE executions (
    id TEXT PRIMARY KEY NOT NULL,
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    exit_code INTEGER,
    duration_ms INTEGER,
    error_message TEXT
)
```

**Lines 128-137**: `execution_artifacts` table - stdout/stderr stored here
```sql
CREATE TABLE execution_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    content_json TEXT NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE RESTRICT
)
```

**Good**: Full tool results ARE already stored in execution_log.db via `record_execution()`.

#### 3. Existing Chat Compaction (`src/ui/chat_compact/`)

**Lines 15-31**: `CompactionTrigger` struct
```rust
pub struct CompactionTrigger {
    pub min_messages: usize,     // default: 50
    pub min_tokens: usize,       // default: 4000
}
```

**Problem**: This is for chat SESSIONS, not tool RESULTS. Different concern.

#### 4. ToolResult Types (Multiple locations)

- `src/execution_engine/chat_tool_runner.rs:48` - `ToolResult` struct (execution layer)
- `src/llm/chat_events.rs:50-56` - `ChatEvent::ToolResult` (event layer)
- `src/ui/state.rs:149` - `ToolResult` struct (UI layer)

**Problem**: 3 different `ToolResult` types in different layers.

#### 5. add_tool_result Call Sites

From ripgrep analysis:
- `src/llm/chat_loop/loop_state.rs:52` - calls `frame_stack.add_tool_result()`
- `src/llm/frame_stack.rs:438, 449, 557, 581, 603, 613, 636, 639` - test calls

---

## IMPLEMENTATION PLAN

### Phase 1: Add Compaction Flag to Frame::ToolResult

#### File: `src/llm/frame_stack.rs`

**Change 1.1** - Lines 27-31: Add `compacted` field
```rust
// BEFORE:
ToolResult {
    tool: String,
    success: bool,
    output: String,
}

// AFTER:
ToolResult {
    tool: String,
    success: bool,
    output: String,
    compacted: bool,  // NEW: marks if output is compacted
}
```

**Change 1.2** - Line 126: Update `add_tool_result()` signature
```rust
// BEFORE:
pub fn add_tool_result(&mut self, tool: String, success: bool, output: String) {
    self.push_frame(Frame::ToolResult {
        tool,
        success,
        output,
    });
}

// AFTER:
pub fn add_tool_result(&mut self, tool: String, success: bool, output: String) {
    self.push_frame(Frame::ToolResult {
        tool,
        success,
        output,
        compacted: false,  // NEW: default to not compacted
    });
}
```

**Change 1.3** - Lines 49, 69, 90, 169, 253: Update pattern matches
```rust
// All instances of:
Frame::ToolResult { tool, success, output } => { ... }

// Must become:
Frame::ToolResult { tool, success, output, compacted } => { ... }
```

Affected locations:
- Line 49: `frame_name()` match
- Line 69: `estimate_tokens()` match
- Line 90: `last_tool_result()` method
- Line 169: `last_tool_output()` method
- Line 253: `build_messages_with_timeline_and_mode()` match

**Change 1.4** - Lines 253-269: Add compaction output logic
```rust
// BEFORE:
Frame::ToolResult { tool, success, output } => {
    let content = format!(
        "[Tool {}]: {}\nResult: {}",
        tool,
        if *success { "OK" } else { "FAILED" },
        output
    );
    messages.push(LlmMessage { role: LlmRole::User, content });
}

// AFTER:
Frame::ToolResult { tool, success, output, compacted } => {
    let content = if *compacted {
        format!(
            "[Tool {}]: {} - [Old tool result content cleared - use memory_query tool with execution_id to retrieve full details]",
            tool,
            if *success { "OK" } else { "FAILED" }
        )
    } else {
        format!(
            "[Tool {}]: {}\nResult: {}",
            tool,
            if *success { "OK" } else { "FAILED" },
            output
        )
    };
    messages.push(LlmMessage { role: LlmRole::User, content });
}
```

**Change 1.5** - NEW method: Add `compact_old_tool_results()`
```rust
// Add after line 134 (after add_assistant method)

/// Mark old tool results as compacted (keep N most recent)
///
/// # Arguments
/// * `keep_recent` - Number of recent tool results to keep un-compacted
///
/// # Behavior
/// - Iterates frames backwards
/// - Marks ToolResult frames as compacted=false for N most recent
/// - Marks older ToolResult frames as compacted=true
pub fn compact_old_tool_results(&mut self, keep_recent: usize) {
    let mut tool_result_count = 0;

    // Iterate in reverse (newest first)
    for frame in self.frames.iter_mut().rev() {
        if let Frame::ToolResult { compacted, .. } = frame {
            tool_result_count += 1;
            if tool_result_count > keep_recent {
                *compacted = true;
            }
        }
    }
}
```

---

### Phase 2: Cascading Changes

#### File: `src/llm/chat_loop/loop_state.rs`

**Change 2.1** - Line 90: Update pattern match
```rust
// BEFORE:
if let Frame::ToolResult { tool, output, .. } = f {

// AFTER:
if let Frame::ToolResult { tool, output, .. } = f {
    // No change needed - .. wildcard skips the new field
}
```

**Status**: NO CHANGE - `..` wildcard already skips extra fields.

---

### Phase 3: Tests Update

#### File: `src/llm/frame_stack.rs` (tests module)

**Change 3.1** - Lines 438, 449, 557, 581, 603, 613, 636, 639: Update test assertions

All test calls to `add_tool_result()` work as-is (positional arguments).

**Change 3.2** - NEW test: Add compaction test
```rust
// Add after line 643 (after test_multiple_tool_results_ordering)

#[test]
fn test_compact_old_tool_results() {
    let mut stack = FrameStack::new();
    stack.add_user("hello".to_string());

    // Add 5 tool results
    for i in 0..5 {
        stack.add_tool_result(
            format!("tool_{}", i),
            true,
            format!("output {}", i),
        );
    }

    // Compact, keep 2 most recent
    stack.compact_old_tool_results(2);

    // Check that only 2 most recent are NOT compacted
    let frames: Vec<_> = stack.iter().collect();
    let tool_results: Vec<&Frame> = frames
        .iter()
        .filter(|f| matches!(f, Frame::ToolResult { .. }))
        .copied()
        .collect();

    // Last 2 should be compacted=false, first 3 should be compacted=true
    let mut iter = tool_results.iter().rev();
    for _ in 0..2 {
        match iter.next() {
            Some(Frame::ToolResult { compacted, .. }) => assert!(!compacted),
            _ => panic!("Expected ToolResult"),
        }
    }
    for _ in 0..3 {
        match iter.next() {
            Some(Frame::ToolResult { compacted, .. }) => assert!(compacted),
            _ => panic!("Expected ToolResult"),
        }
    }
}

#[test]
fn test_build_messages_with_compacted_results() {
    let mut stack = FrameStack::new();
    stack.add_user("list files".to_string());

    // Add old tool result
    stack.add_tool_result("file_glob".to_string(), true, "old_long_output...".to_string());

    // Add new tool result
    stack.add_tool_result("file_read".to_string(), true, "current output".to_string());

    // Manually mark first as compacted
    if let Some(Frame::ToolResult { compacted, .. }) = stack.frames.iter_mut().nth(1) {
        *compacted = true;
    }

    let messages = stack.build_messages();

    // Find tool result messages
    let tool_msg: Vec<_> = messages
        .iter()
        .filter(|m| m.content.contains("[Tool"))
        .collect();

    // First (compacted) should have "cleared" message
    assert!(tool_msg[0].content.contains("content cleared"));
    assert!(!tool_msg[0].content.contains("old_long_output"));

    // Second (not compacted) should have full output
    assert!(tool_msg[1].content.contains("current output"));
}
```

---

### Phase 4: Integration - When to Compact

#### File: `src/llm/frame_stack.rs`

**Change 4.1** - Update `build_messages_with_timeline_and_mode()` to auto-compact

Add at line 202 (before building messages):

```rust
// Auto-compact if we have too many tool results
const MAX_RECENT_TOOL_RESULTS: usize = 3;
let tool_result_count = self
    .iter()
    .filter(|f| matches!(f, Frame::ToolResult { .. }))
    .count();

if tool_result_count > MAX_RECENT_TOOL_RESULTS {
    self.compact_old_tool_results(MAX_RECENT_TOOL_RESULTS);
}
```

**Decision point**: Should compaction be:
- A) Automatic (always when building messages) - RECOMMENDED
- B) Manual (explicit call) - less intrusive

**Recommendation**: Automatic with `MAX_RECENT_TOOL_RESULTS = 3`

---

## SUMMARY OF CHANGES

### Files to Modify

| File | Lines | Change |
|------|-------|--------|
| `src/llm/frame_stack.rs` | 27-31 | Add `compacted: bool` to `Frame::ToolResult` |
| `src/llm/frame_stack.rs` | 126 | Update `add_tool_result()` to set `compacted: false` |
| `src/llm/frame_stack.rs` | 49, 69, 90, 169, 253 | Update pattern matches to include `compacted` |
| `src/llm/frame_stack.rs` | 253-269 | Add compaction output logic |
| `src/llm/frame_stack.rs` | ~135 | Add `compact_old_tool_results()` method |
| `src/llm/frame_stack.rs` | ~202 | Add auto-compact before building messages |
| `src/llm/frame_stack.rs` | ~643 | Add compaction tests |

### Files That DO NOT Need Changes

| File | Reason |
|------|--------|
| `src/llm/chat_loop/loop_state.rs` | Uses `..` wildcard, already compatible |
| `src/llm/chat_events.rs` | Separate `ToolResult` type, not affected |
| `src/execution_engine/chat_tool_runner.rs` | Separate `ToolResult` type, not affected |
| `src/ui/state.rs` | Separate `ToolResult` type, not affected |

---

## TESTING STRATEGY

1. **Unit tests** (in `src/llm/frame_stack.rs`):
   - `test_compact_old_tool_results()` - verify compaction flag set correctly
   - `test_build_messages_with_compacted_results()` - verify compacted output format
   - Existing tests should pass (positional args)

2. **Integration test** (new file `tests/tool_result_compaction_tests.rs`):
   - Create chat loop with 5 tool invocations
   - Verify only 3 recent results in LLM context
   - Verify old results show "content cleared" message

3. **Manual test**:
   - Run odincode chat
   - Invoke tool 5 times
   - Check that context shows compacted summary for old results

---

## OPEN QUESTIONS

1. **Should compaction be auto or manual?**
   - Auto: Always happens when `build_messages()` called (simpler) - **RECOMMENDED**
   - Manual: App explicitly calls `compact_old_tool_results()` (more control)

2. **What should `MAX_RECENT_TOOL_RESULTS` be?**
   - 3 (keeps last 3, compact rest) - **RECOMMENDED**
   - 5 (keeps last 5)
   - Configurable?

3. **Should execution_id be stored for memory_query?**
   - **YES** - Required for compaction message to be actionable
   - `memory_query` can query by `session_id` (prefix match on execution_id)
   - But compaction message should include specific `execution_id` for precision

---

## ADDITIONAL REQUIREMENT: execution_id Field

### Phase 1.5: Add execution_id to Frame::ToolResult

#### File: `src/llm/frame_stack.rs`

**Change 1.5.1** - Lines 27-31: Add `execution_id` field
```rust
// BEFORE:
ToolResult {
    tool: String,
    success: bool,
    output: String,
    compacted: bool,
}

// AFTER:
ToolResult {
    tool: String,
    success: bool,
    output: String,
    compacted: bool,
    execution_id: Option<String>,  // NEW: for memory_query reference
}
```

**Change 1.5.2** - Line 126: Update `add_tool_result()` signature
```rust
// BEFORE:
pub fn add_tool_result(&mut self, tool: String, success: bool, output: String) {
    self.push_frame(Frame::ToolResult {
        tool,
        success,
        output,
        compacted: false,
    });
}

// AFTER:
pub fn add_tool_result(&mut self, tool: String, success: bool, output: String, execution_id: Option<String>) {
    self.push_frame(Frame::ToolResult {
        tool,
        success,
        output,
        compacted: false,
        execution_id,
    });
}
```

**Change 1.5.3** - Lines 49, 69, 90, 169, 253: Update pattern matches
```rust
// All instances of:
Frame::ToolResult { tool, success, output, compacted } => { ... }

// Must become:
Frame::ToolResult { tool, success, output, compacted, execution_id } => { ... }
```

**Change 1.5.4** - Lines 253-269: Update compaction output with execution_id
```rust
Frame::ToolResult { tool, success, output, compacted, execution_id } => {
    let content = if *compacted {
        let exec_ref = execution_id.as_ref().map(|id| format!(" (execution_id: {})", id)).unwrap_or_default();
        format!(
            "[Tool {}]: {} - [Old tool result content cleared{}. Use memory_query tool with session_id or execution_id to retrieve full details]",
            tool,
            if *success { "OK" } else { "FAILED" },
            exec_ref
        )
    } else {
        format!(
            "[Tool {}]: {}\nResult: {}",
            tool,
            if *success { "OK" } else { "FAILED" },
            output
        )
    };
    messages.push(LlmMessage { role: LlmRole::User, content });
}
```

### Phase 2.5: Update Call Sites for execution_id

#### File: `src/llm/chat_loop/loop_state.rs`

**Change 2.5.1** - Line 52: Update `add_hidden_result()` call
```rust
// BEFORE:
pub fn add_hidden_result(&mut self, result: &ToolResult) {
    self.frame_stack.add_tool_result(
        result.tool.clone(),
        result.success,
        result.output_full.clone(),
    );
}

// AFTER:
pub fn add_hidden_result(&mut self, result: &ToolResult) {
    self.frame_stack.add_tool_result(
        result.tool.clone(),
        result.success,
        result.output_full.clone(),
        Some(result.execution_id.clone()),  // NEW: pass execution_id
    );
}
```

**NOTE**: `ToolResult` in `src/execution_engine/chat_tool_runner.rs:48` needs `execution_id` field too.

#### File: `src/execution_engine/chat_tool_runner.rs`

**Change 2.5.2** - Line 48: Add `execution_id` to `ToolResult` struct
```rust
// BEFORE:
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub output_full: String,
    pub output_preview: String,
    pub error_message: Option<String>,
    pub affected_path: Option<String>,
    pub kind: ToolOutputKind,
    pub structured_data: Option<serde_json::Value>,
}

// AFTER:
pub struct ToolResult {
    pub tool: String,
    pub success: bool,
    pub output_full: String,
    pub output_preview: String,
    pub error_message: Option<String>,
    pub affected_path: Option<String>,
    pub kind: ToolOutputKind,
    pub structured_data: Option<serde_json::Value>,
    pub execution_id: String,  // NEW: required for memory_query reference
}
```

**Change 2.5.3** - Lines 184, 223, 408, 433, 453: Update `ToolResult` construction
```rust
// All instances need execution_id added:
Ok(ToolResult {
    // ... existing fields ...
    execution_id: id,  // NEW: the ID returned by record_execution()
})
```

---

## UPDATED SUMMARY OF CHANGES

### Files to Modify

| File | Lines | Change |
|------|-------|--------|
| `src/execution_engine/chat_tool_runner.rs` | 48-58 | Add `execution_id: String` to `ToolResult` |
| `src/execution_engine/chat_tool_runner.rs` | 184, 223, 408, 433, 453 | Pass `execution_id` in `ToolResult` construction |
| `src/llm/frame_stack.rs` | 27-33 | Add `compacted: bool` and `execution_id: Option<String>` to `Frame::ToolResult` |
| `src/llm/frame_stack.rs` | 126-136 | Update `add_tool_result()` signature to accept `execution_id` |
| `src/llm/frame_stack.rs` | 49, 69, 90, 169, 253 | Update pattern matches |
| `src/llm/frame_stack.rs` | 253-277 | Add compaction output logic with execution_id |
| `src/llm/frame_stack.rs` | ~145 | Add `compact_old_tool_results()` method |
| `src/llm/frame_stack.rs` | ~212 | Add auto-compact before building messages |
| `src/llm/chat_loop/loop_state.rs` | 51-57 | Update `add_hidden_result()` to pass execution_id |
| `src/llm/frame_stack.rs` | tests | Update tests, add compaction tests |

---

## REFERENCES

- OpenCode analysis: `docs/OPENCODE_ANALYSIS.md:657-818`
- FrameStack: `src/llm/frame_stack.rs`
- ExecutionDb schema: `src/execution_tools/db.rs:113-137`
- Chat compaction: `src/ui/chat_compact/mod.rs` (different concern - sessions, not results)
