# Tool Result Compaction & Memory Query Design

**Date**: 2025-12-27
**Status**: Design Phase - Brainstorming
**Issue**: LLM amnesia after tool execution, context bloat, error loops

---

## Problem Statement

### Current Issues

1. **LLM Amnesia**: After tool executes, LLM responds as if it has no context
2. **Context Bloat**: Every tool result sent to LLM in full → exponential token growth
3. **Error Loops**: LLM ignores tool errors, retries same failed operation infinitely
4. **No Retrieval**: Old tool results lost, no way to query past work

### Observed Failures

- Tool executed (file_glob: 40 files found)
- Result displayed in UI
- LLM responds: "I don't have any previous context about files you asked me to count. This appears to be the start of our conversation."
- LLM never saw the tool result

---

## Investigation Findings

### OpenCode's Approach

**Compaction Strategy:**
- Only compacts when >20,000 tokens can be saved
- Protects last 2 conversation turns (never pruned)
- Uses LLM agent to generate summary of what happened
- Summary REPLACES old content (not blank "cleared" message)

**Compaction Agent Prompt:**
```
You are a helpful AI assistant tasked with summarizing conversations.
Focus on information that would be helpful for continuing the conversation:
- What was done
- What is currently being worked on
- Which files are being modified
- What needs to be done next
- Key user requests, constraints, or preferences
```

**Storage:**
- Everything in database (MessageV2, Part tables)
- Full tool outputs preserved
- Compaction flag marks content as compacted
- LLM sees summary OR full content

**Critical Flaw in OpenCode:**
- LLM still gets stuck in error loops (ignores "file not found" errors)
- LLM doesn't automatically query memory
- Summary generation costs tokens + time

---

## Design: Working Memory vs Long-Term Memory

### Mental Model

```
┌─────────────────────────────────────────────────────────────┐
│                    LLM Context Window                        │
│                   (Working Memory)                            │
│                                                              │
│  • System prompt (~500 tokens)                                │
│  • Recent conversation (~5,000 tokens)                         │
│  • Last 5-10 tool results (metadata ~500 tokens)               │
│                                                              │
│  Total: ~6,000 tokens (stable, doesn't grow indefinitely)      │
└─────────────────────────────────────────────────────────────┘
                              │
                              │ memory_query tool
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   execution_log.db                              │
│                   (Long-Term Memory)                           │
│                                                              │
│  • Every tool execution (full output)                         │
│  • Timestamps, execution_ids                                   │
│  • Tool inputs, outputs, errors                               │
│  • File modifications                                         │
│  • LSP diagnostics                                            │
│                                                              │
│  Complete history, retrievable on demand                     │
└─────────────────────────────────────────────────────────────┘
```

### Key Principle

**Database = Source of Truth**
- Everything stored, never lost
- Full history preserved
- Retrievable via queries

**Context = Working Memory**
- Recent context for continuity
- Active edit loop
- What we're working on NOW

**memory_query = Access to Long-Term Memory**
- LLM tool for retrieving historical data
- Structured queries (by execution_id, session_id, tool, file, etc.)

---

## Proposed Solution

### 1. Database Schema (Already Exists)

```sql
CREATE TABLE executions (
    id TEXT PRIMARY KEY NOT NULL,        -- execution_id
    tool_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    exit_code INTEGER,
    duration_ms INTEGER,
    error_message TEXT
);

CREATE TABLE execution_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    execution_id TEXT NOT NULL,
    artifact_type TEXT NOT NULL,
    content_json TEXT NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES executions(id)
);
```

### 2. Compaction Strategy

**Metadata Format (in context):**
```
[Tool file_glob]: OK - Found 42 .rs files in src/
(execution_id: abc123, compacted - use memory_query for full list)
```

**What gets compacted:**
- Informational tools: `file_glob`, `symbols_in_file`, `count_files`
- Old edit loops that completed successfully
- Anything beyond last 5-10 tool results

**What stays in context (un-compacted):**
- Last 5 tool results (configurable)
- Active edit loop: `file_read` → `file_edit` → `lsp_check` → retry
- Recent conversation turns

### 3. Memory Query Tool

**Tool Definition:**
```rust
memory_query(
    execution_id?: string,  // Query by specific execution
    session_id?: string,    // Query by session (all tool calls)
    tool_name?: string,     // Query by tool type
    file_path?: string,     // Query by file affected
    since?: timestamp,      // Time range
    limit?: number          // Max results
)
```

**Returns:**
```json
{
  "execution_id": "abc123",
  "tool": "file_glob",
  "timestamp": 1735312345,
  "success": true,
  "arguments": {"pattern": "*.rs", "root": "src"},
  "output": "src/main.rs\nsrc/lib.rs\n...",  // Full output
  "output_preview": "42 files found"
}
```

### 4. Prompt-Level Enforcement

**System Prompt Addition:**
```
CRITICAL CONSTRAINTS:
1. READ ALL TOOL RESULTS COMPLETELY before responding
2. If tool returns ERROR, STOP immediately. DO NOT retry the same operation
3. When you see "(compacted - use memory_query)" metadata:
   - You MUST call memory_query(execution_id: "xxx") to retrieve full details
   - DO NOT guess or make up information
   - DO NOT proceed without retrieving if you need that data
4. Never ignore tool feedback, especially errors
```

---

## File Edit Workflow Consideration

### The Edit Loop Problem

```
1. file_read(src/main.rs) → LLM sees code (500 lines)
2. file_edit(src/main.rs, ...) → makes change
3. lsp_check(src/main.rs) → 3 errors
4. LLM needs: original code + edit + errors → retry
```

**If we compact step 1:**
- LLM can't see what it's editing
- Makes edit blindly
- Can't fix errors (never saw original code)

**Solution: Smart Compaction Rules**

**Informational tools (compact immediately):**
- `file_glob` → just metadata needed
- `symbols_in_file` → just metadata needed
- `count_files`, `fs_stats` → just numbers needed

**Edit-loop tools (keep in context while active):**
- `file_read` → LLM needs the CODE
- `file_edit` → LLM needs to see what changed
- `lsp_check` → LLM needs the ERRORS
- `splice_patch` → LLM needs context

**Implementation:**
```rust
fn should_compact(tool: &str) -> bool {
    !matches!(tool,
        "file_read" | "file_edit" | "lsp_check" |
        "splice_patch" | "splice_plan"
    )
}

// OR: Keep last N results un-compacted regardless
const KEEP_RECENT: usize = 5;
```

**When edit loop completes (LSP passes):**
- Mark those tool results as compacted
- Store in database
- Metadata remains: "[Tool file_read]: OK (exec_id: xyz, compacted)"

---

## Error Loop Prevention

### The Problem

```
LLM: "read /src/main.rs"
Tool: "Error: File not found"
LLM: ignores → "read /src/main.rs"
Tool: "Error: File not found"
[stuck forever]
```

### Solution: Structural Enforcement

**1. Error Results Stand Out**
```
╔═══════════════════════════════════════════════════════════════╗
║  ⛔ TOOL ERROR: file_read FAILED                              ║
║  ─────────────────────────────────────────────────────────────  ║
║  Error: File not found: /src/main.rs                           ║
║  Timestamp: 2025-12-27T10:30:45Z                               ║
║                                                            ║
║  REQUIRED: You MUST acknowledge this error before continuing.   ║
║  DO NOT retry the same operation without fixing the path.       ║
╚═══════════════════════════════════════════════════════════════╝
```

**2. Loop Detection**
```rust
// Detect: same tool + same args + failed → hard stop
if tool_failed && same_args_used_in_last_n_attempts(3) {
    return LoopAction::FatalError(
        "Tool failed 3 times with same inputs. User intervention required."
    );
}
```

**3. System-Level Interruption**
When LLM ignores error and retries:
```
⛠️ SYSTEM INTERRUPTION: You ignored the previous error.
The file does not exist: /src/main.rs
Choose a different approach. DO NOT retry the same tool call.
```

---

## Token Savings Analysis

### Without Compaction

| Tool Calls | Avg Output | Context Tokens |
|-----------|-----------|----------------|
| 10        | 5,000     | 50,000         |
| 50        | 5,000     | 250,000        |
| 100       | 5,000     | 500,000        |

### With Compaction

| Tool Calls | In Context (last 5) | Metadata (old 95) | Total |
|-----------|---------------------|-------------------|-------|
| 10        | 25,000              | 500               | 25,500 |
| 50        | 25,000              | 2,500             | 27,500 |
| 100       | 25,000              | 5,000             | 30,000 |

**Savings:**
- 10 calls: ~50%
- 50 calls: ~89%
- 100 calls: ~94%

**Context stays bounded** at ~30,000 tokens regardless of conversation length.

---

## Implementation Plan

### Phase 1: Database Recording (VERIFY)
- [ ] Verify all tool executions record to execution_log.db
- [ ] Verify execution_ids are generated and stored
- [ ] Verify full output is preserved
- [ ] Test retrieval of stored executions

### Phase 2: Memory Query Tool (IMPLEMENT)
- [ ] Create `memory_query` tool definition
- [ ] Implement query by execution_id
- [ ] Implement query by session_id
- [ ] Implement query by tool_name
- [ ] Implement query by file_path
- [ ] Add to tool registry

### Phase 3: Compaction Metadata (IMPLEMENT)
- [ ] Add `compacted: bool` field to `Frame::ToolResult`
- [ ] Add `execution_id: Option<String>` field
- [ ] Update compaction message format with metadata
- [ ] Update `build_messages()` to handle compaction

### Phase 4: Smart Compaction Rules (IMPLEMENT)
- [ ] Keep last N tool results un-compacted
- [ ] Edit-loop tools stay un-compacted while active
- [ ] Informational tools compact to metadata
- [ ] Auto-compact threshold

### Phase 5: Error Prevention (IMPLEMENT)
- [ ] Make error results visually distinct
- [ ] Implement loop detection
- [ ] Add system interruption for ignored errors
- [ ] Test error scenarios

### Phase 6: Prompt Enforcement (IMPLEMENT)
- [ ] Add system prompt constraints
- [ ] Add mode-specific prompts (MUTATION mode, etc.)
- [ ] Test LLM compliance

---

## Open Questions

1. **What is the right value for KEEP_RECENT?**
   - 3? 5? 10?
   - Should it be configurable?

2. **How do we know when edit loop is "done"?**
   - LSP passes?
   - User says "done"?
   - Time-based?

3. **Should summaries be generated?**
   - OpenCode uses LLM to summarize
   - We use inline metadata (faster, cheaper)
   - Is metadata enough?

4. **How to handle file_read compaction for large files?**
   - If file is 10,000 lines, do we keep it in context?
   - What if multiple large files are edited?

---

## References

- OpenCode compaction: `/home/feanor/Projects/opencode/packages/opencode/src/session/compaction.ts`
- OpenCode message format: `/home/feanor/Projects/opencode/packages/opencode/src/session/message-v2.ts`
- Current implementation: `/home/feanor/Projects/odincode/docs/TOOL_RESULT_COMPACTION_PLAN.md`
- FrameStack: `/home/feanor/Projects/odincode/src/llm/frame_stack.rs`
