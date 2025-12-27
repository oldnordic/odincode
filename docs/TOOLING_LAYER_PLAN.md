# ODINCODE TOOLING LAYER — COMPREHENSIVE PLAN

## 1. HIGH-LEVEL ARCHITECTURE

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              OBLIVION LLM                                   │
│                         (OpenAI / Anthropic / GLM)                          │
└───────────────────────────────────────┬─────────────────────────────────────┘
                                        │
                                        │ Tool Call Request
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TOOL ROUTER / CLASSIFIER                          │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┬───────────────┐  │
│  │   Editor    │      OS     │     Git     │  Analysis   │    Memory     │  │
│  │   Tools     │    Tools    │    Tools    │    Tools    │    Tools      │  │
│  └─────────────┴─────────────┴─────────────┴─────────────┴───────────────┘  │
│                                   │                                         │
│                                   ▼                                         │
│                    ┌───────────────────────────────┐                        │
│                    │   CIRCUIT BREAKER & LIMITS    │                        │
│                    │  • Max calls per turn         │                        │
│                    │  • Repeat detection           │                        │
│                    │  • Timeout enforcement        │                        │
│                    │  • Stall detection            │                        │
│                    └───────────────────────────────┘                        │
└───────────────────────────────────────┬─────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           TOOL EXECUTION ENGINE                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    TOOL MAPPER (Dispatch Table)                      │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    PRECONDITION CHECKER                              │   │
│  │  • File existence  • Workspace valid  • DB available  • Permissions  │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    EXECUTOR (with timeout)                           │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                   │                                         │
│                                   ▼                                         │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                    EXECUTION DB (SQLite)                             │   │
│  │  • Log all calls  • Store full results  • Track durations/errors     │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────┬─────────────────────────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           OUTPUT ROUTER & CLASSIFIER                        │
│  ┌─────────────┬─────────────┬─────────────┬─────────────┬───────────────┐  │
│  │   Chat      │  Code View  │  Diagnostic │   Explorer  │    Console    │  │
│  │  Summary    │   Content   │   Panel     │   Update    │    Log        │  │
│  └─────────────┴─────────────┴─────────────┴─────────────┴───────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. PHASE A — TOOL INVENTORY & CLASSIFICATION

### A.1 Tool Taxonomy

| Category | Purpose | Loop-Safe | Auto-Gated |
|----------|---------|-----------|------------|
| **Editor** | File read/write/edit operations | YES | Auto: read, Gated: write/edit |
| **OS** | Command execution and file system queries | NO | Always Gated |
| **Git** | Repository state and history | YES | Always Auto |
| **Analysis** | Code queries, diagnostics, search | YES | Always Auto |
| **Memory** | Execution result persistence and queries | YES | Always Auto |

### A.2 Complete Tool Inventory

| Tool Name | Category | Input Schema | Output Schema | UI Target | Auto | Loop-Safe |
|-----------|----------|--------------|---------------|-----------|------|-----------|
| **file_read** | Editor | `{path, offset?, limit?}` | `{content, line_count, truncated}` | Code View | Auto | Yes |
| **file_write** | Editor | `{path, content}` | `{bytes_written, path}` | Console | Gated | No |
| **file_edit** | Editor | `{path, old_string, new_string}` | `{changed, diff}` | Console | Gated | No |
| **file_create** | Editor | `{path, content}` | `{created, path}` | Console | Gated | No |
| **file_glob** | Editor | `{pattern, root}` | `{paths[], count}` | Explorer | Auto | Yes |
| **grep** | Editor | `{pattern, glob?, context_lines?}` | `{matches[], count}` | Explorer | Auto | Yes |
| **bash_exec** | OS | `{command, timeout?, env?}` | `{stdout, stderr, exit_code}` | Console | Gated | No |
| **wc** | OS | `{files[], mode}` | `{lines, words, bytes, per_file}` | Chat | Auto | Yes |
| **git_status** | Git | `{path?, format?}` | `{branch, changed[], staged[]}` | Explorer | Auto | Yes |
| **git_diff** | Git | `{path?, cached?}` | `{diff, hunks[]}` | Code View | Auto | Yes |
| **git_log** | Git | `{path?, limit?, format?}` | `{commits[], count}` | Explorer | Auto | Yes |
| **symbols_query** | Analysis | `{file, kind?}` | `{symbols[], count}` | Explorer | Auto | Yes |
| **references_query** | Analysis | `{symbol_name, max_results}` | `{refs[], locations[]}` | Explorer | Auto | Yes |
| **diagnostics_query** | Analysis | `{path?}` | `{diagnostics[], count}` | Diagnostic Panel | Auto | Yes |
| **count_files** | Analysis | `{pattern, root, by_extension}` | `{total_count, by_extension}` | Chat | Auto | Yes |
| **count_lines** | Analysis | `{pattern, root}` | `{total_lines, file_count, per_file}` | Chat | Auto | Yes |
| **fs_stats** | Analysis | `{path, max_depth}` | `{file_count, dir_count, bytes}` | Chat | Auto | Yes |
| **memory_query** | Memory | `{execution_id?, tool?, limit}` | `{executions[], summaries[]}` | Chat | Auto | Yes |
| **execution_summary** | Memory | `{execution_id}` | `{summary, lean_output}` | Chat | Auto | Yes |

### A.3 New Tools Required

| Tool | Priority | Complexity | Dependencies |
|------|----------|------------|--------------|
| `file_edit` | HIGH | Medium | file_read, diff generation |
| `bash_exec` | HIGH | High | Process spawning, allowlist |
| `wc` | MEDIUM | Low | File I/O |
| `git_status` | HIGH | Medium | git2 crate or CLI |
| `git_diff` | HIGH | Medium | git2 crate or CLI |
| `git_log` | MEDIUM | Medium | git2 crate or CLI |
| `memory_query` | HIGH | Low | Execution DB |
| `execution_summary` | MEDIUM | Medium | LLM summarization |

---

## 3. PHASE B — EXECUTION SAFETY & CIRCUIT BREAKERS

### B.1 Circuit Breaker States

```
┌─────────┐   Failure Threshold    ┌───────────┐
│  CLOSED │  ───────────────────▶  │   OPEN    │
│(Normal) │   (5 consecutive)      │(Stopped)  │
└─────────┘                         └───────────┘
    ▲                                    │
    │                                    │ Timeout
    │                                    │ (30 sec)
    │                                    ▼
    │                             ┌─────────────┐
    └──────────────────────────── │   HALF-OPEN │
          Successful test         │  (Testing)  │
                                   └─────────────┘
```

### B.2 Limits and Budgets

| Limit | Default | Max | Rationale |
|-------|---------|-----|-----------|
| `max_tool_calls_per_turn` | 20 | 50 | Prevent infinite loops |
| `max_identical_calls` | 2 | 5 | Detect stuck patterns |
| `tool_timeout_ms` | 30000 | 120000 | Per-tool timeout |
| `session_execution_budget` | 100 | 500 | Total calls per session |
| `stall_threshold` | 5 | 10 | No state change steps |
| `output_truncate_chars` | 10000 | 50000 | Prevent token spam |

### B.3 Stall Detection Algorithm

```rust
// Stall detection: detect loops with no progress
struct StallDetector {
    state_snapshots: VecDeque<StateSnapshot>,
    threshold: usize,
}

struct StateSnapshot {
    step_number: usize,
    files_modified: HashSet<PathBuf>,
    tools_invoked: Vec<String>,
    checksum: String,  // Hash of relevant state
}

impl StallDetector {
    fn is_stalled(&self) -> Option<StallReason> {
        if self.state_snapshots.len() < self.threshold {
            return None;
        }

        // Check if last N snapshots have identical state
        let recent: Vec<_> = self.state_snapshots
            .iter()
            .rev()
            .take(self.threshold)
            .map(|s| &s.checksum)
            .collect();

        if recent.windows(2).all(|w| w[0] == w[1]) {
            return Some(StallReason::NoStateChange);
        }

        // Check for tool call loops (same tools in same order)
        let tool_seqs: Vec<Vec<String>> = self.state_snapshots
            .iter()
            .rev()
            .take(self.threshold)
            .map(|s| s.tools_invoked.clone())
            .collect();

        if tool_seqs.windows(2).all(|w| w[0] == w[1]) {
            return Some(StallReason::ToolLoop);
        }

        None
    }
}
```

### B.4 Circuit Breaker Schema

```rust
// src/execution_engine/circuit_breaker.rs

use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,     // Normal operation
    Open,       // Circuit tripped, block calls
    HalfOpen,   // Testing if recovery is possible
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: usize,     // Failures before opening
    pub success_threshold: usize,     // Successes to close again
    pub open_timeout: Duration,       // How long to stay open
    pub half_open_max_calls: usize,   // Test calls in half-open
}

pub struct CircuitBreaker {
    state: CircuitState,
    failure_count: usize,
    success_count: usize,
    last_failure_time: Option<Instant>,
    half_open_calls: usize,
    config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn try_execute<F, R>(&mut self, tool: &str, f: F) -> Result<R, CircuitError>
    where
        F: FnOnce() -> Result<R, Box<dyn std::error::Error>>,
    {
        // Check state before execution
        match self.state {
            CircuitState::Open => {
                if self.should_attempt_reset() {
                    self.transition_to_half_open();
                } else {
                    return Err(CircuitError::CircuitOpen(tool.to_string()));
                }
            }
            CircuitState::HalfOpen => {
                if self.half_open_calls >= self.config.half_open_max_calls {
                    return Err(CircuitError::CircuitOpen(tool.to_string()));
                }
            }
            CircuitState::Closed => {}
        }

        // Execute and track result
        let result = f();

        match &result {
            Ok(_) => self.on_success(),
            Err(_) => self.on_failure(),
        }

        result.map_err(|e| CircuitError::ExecutionFailed(e.to_string()))
    }

    fn on_success(&mut self) {
        self.failure_count = 0;

        match self.state {
            CircuitState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    self.transition_to_closed();
                }
            }
            CircuitState::Closed => {}
            CircuitState::Open => unreachable!(),
        }
    }

    fn on_failure(&mut self) {
        self.failure_count += 1;
        self.last_failure_time = Some(Instant::now());
        self.success_count = 0;

        if self.failure_count >= self.config.failure_threshold {
            self.transition_to_open();
        }
    }
}
```

### B.5 Files to Create/Modify

| File | Action | LOC Est |
|------|--------|---------|
| `src/execution_engine/circuit_breaker.rs` | CREATE | 200 |
| `src/execution_engine/stall_detector.rs` | CREATE | 150 |
| `src/execution_engine/execution_budget.rs` | CREATE | 120 |
| `src/execution_engine/safety_config.rs` | CREATE | 80 |
| `src/execution_engine/mod.rs` | MODIFY (exports) | 10 |
| `src/execution_engine/chat_tool_runner.rs` | MODIFY (integrate) | 50 |

---

## 4. PHASE C — UI ROUTING RULES

### C.1 Routing Decision Tree

```
┌─────────────────────────────────────────────────────────────────┐
│                    TOOL OUTPUT RECEIVED                         │
└─────────────────────────────────────┬───────────────────────────┘
                                  │
                                  ▼
                    ┌───────────────────────────────┐
                    │    What is the output kind?   │
                    └───────────────────────────────┘
                                  │
          ┌───────────────────────┼───────────────────────┐
          │                       │                       │
          ▼                       ▼                       ▼
    ┌───────────┐          ┌───────────┐          ┌───────────┐
    │ Structural│          │FileContent│          │  Textual  │
    │   Data    │          │           │          │           │
    └─────┬─────┘          └─────┬─────┘          └─────┬─────┘
          │                      │                      │
          ▼                      ▼                      ▼
    ┌───────────┐          ┌───────────┐          ┌───────────┐
    │  Size >   │          │ Path in   │          │  Direct   │
    │ 1KB?      │          │ project?  │          │  to Chat  │
    └─────┬─────┘          └─────┬─────┘          └───────────┘
          │                      │
     YES /│\ NO              YES/│\NO
          │                      │
          ▼                      ▼
    ┌───────────┐          ┌───────────┐
    │   CODE    │          │   CODE    │
    │   VIEW    │          │   VIEW    │
    │  + Summary│          │           │
    └───────────┘          └───────────┘
```

### C.2 Routing Rules Table

| Output Kind | Size | Content | Destination | Chat Shows |
|-------------|------|---------|-------------|------------|
| `Structural` (paths, symbols) | Any | List/Array | Code View/Explorer | "Found 42 files" + pointer |
| `Structural` (large) | >1KB | JSON | Explorer only | "Large result, see Explorer" |
| `FileContent` | Any | File text | Code View | "Loaded file.rs (234 lines)" |
| `NumericSummary` | Any | Counts/stats | Chat | Full summary |
| `Textual` | <500 chars | Message | Chat | Full text |
| `Textual` | ≥500 chars | Long message | Console + Chat pointer | Preview + pointer |
| `Error` | Any | Error message | Chat (highlighted) | Full error |
| `Void` | - | No output | Nowhere | Nothing |

### C.3 Chat Injection Format

```
┌─────────────────────────────────────────────────────────────────┐
│  CHAT MESSAGE (what LLM sees)                                   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Tool: file_glob                                                │
│  Status: ✓ Completed                                            │
│  Result: Found 42 files matching **/*.rs                        │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │ [View full results in Explorer →]                        │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### C.4 Files to Modify

| File | Changes | LOC Est |
|------|---------|---------|
| `src/execution_engine/output_kind.rs` | EXTEND (add routing rules) | 60 |
| `src/execution_engine/chat_tool_runner.rs` | MODIFY (apply routing) | 40 |
| `src/ui/render.rs` | MODIFY (routing display) | 80 |
| `src/ui/explorer.rs` | CREATE/MODIFY (structural display) | 200 |

---

## 5. PHASE D — MEMORY-AWARE TOOLS

### D.1 Execution DB Schema

```sql
-- Existing schema (already implemented)
CREATE TABLE executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    args_json TEXT NOT NULL,
    result_json TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL,
    success BOOLEAN NOT NULL,
    error_message TEXT
);

-- New index for efficient querying
CREATE INDEX idx_executions_session_tool ON executions(session_id, tool_name);
CREATE INDEX idx_executions_tool_time ON executions(tool_name, started_at DESC);
```

### D.2 Memory Query Tool Schema

```rust
// src/execution_tools/memory_query.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryArgs {
    /// Filter by execution ID (exact match)
    pub execution_id: Option<i64>,

    /// Filter by tool name
    pub tool: Option<String>,

    /// Filter by session ID
    pub session_id: Option<String>,

    /// Filter by success status
    pub success_only: Option<bool>,

    /// Maximum results to return
    pub limit: Option<usize>,

    /// Include full output (vs summary only)
    pub include_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub id: i64,
    pub session_id: String,
    pub tool_name: String,
    pub started_at: i64,
    pub duration_ms: i64,
    pub success: bool,
    pub summary: String,
    pub output_preview: String,
    pub full_output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQueryResult {
    pub executions: Vec<ExecutionSummary>,
    pub count: usize,
    pub has_more: bool,
}
```

### D.3 Memory Query Examples

```
User: "What did file_search return earlier?"

LLM → memory_query {
    tool: "file_search",
    session_id: "<current>",
    limit: 5
}

Result: [
    {
        id: 1234,
        tool: "file_search",
        summary: "Found 3 matches for 'parse_config' in src/",
        output_preview: "src/config.rs:42: fn parse_config...",
        full_output: "<available if requested>"
    }
]
```

### D.4 Lean Reinjection Pattern

```rust
// Instead of re-injecting full tool output:
// OLD: output_full (100KB of data)
// NEW: summary + execution_id

pub struct ToolResultForLLM {
    pub tool: String,
    pub status: ToolStatus,
    pub summary: String,           // 1-2 sentences
    pub execution_id: i64,         // Reference to full result
    pub lean_data: Option<String>, // Small, actionable subset
}

// LLM can query full result later if needed:
// "Use execution_id 1234 to get the full file list"
```

### D.5 Files to Create/Modify

| File | Action | LOC Est |
|------|--------|---------|
| `src/execution_tools/memory_query.rs` | CREATE | 180 |
| `src/execution_tools/execution_summary.rs` | CREATE | 120 |
| `src/execution_tools/mod.rs` | MODIFY (exports) | 10 |

---

## 6. PHASE E — INCREMENTAL IMPLEMENTATION PLAN

### E.1 Implementation Order

#### Step 1: Foundation (Week 1)
1. Create `circuit_breaker.rs` — Core circuit breaker logic
2. Create `stall_detector.rs` — Loop detection
3. Create `safety_config.rs` — Configuration structure
4. Add tests for all safety components
5. **Validation**: `cargo test --test safety_tests`

#### Step 2: Core Tools (Week 1-2)
1. Create `os_tools/wc.rs` — Word/line counting
2. Create `git_tools/status.rs` — Git status
3. Create `git_tools/diff.rs` — Git diff
4. Create `git_tools/log.rs` — Git log
5. Integrate tools into `tool_mapper.rs`
6. **Validation**: `cargo test --test git_tools_tests`, `cargo test --test os_tools_tests`

#### Step 3: Memory Layer (Week 2)
1. Create `memory_query.rs` — Query past executions
2. Create `execution_summary.rs` — Generate lean summaries
3. Update `ExecutionDb` with new indexes
4. Integrate into tool mapper
5. **Validation**: `cargo test --test memory_tools_tests`

#### Step 4: UI Routing (Week 2-3)
1. Extend `output_kind.rs` with routing rules
2. Update `chat_tool_runner.rs` to apply routing
3. Create UI components for structural display
4. Implement chat injection formatting
5. **Validation**: Manual UI testing + integration tests

#### Step 5: Bash Tool (Week 3)
1. Create `os_tools/bash_exec.rs` — Command execution
2. Implement command allowlist
3. Add timeout and output capture
4. Security audit (command injection prevention)
5. **Validation**: `cargo test --test bash_tools_tests` + security review

#### Step 6: Integration (Week 3-4)
1. Wire up circuit breakers in chat loop
2. Add stall detection to execution engine
3. Implement execution budgets
4. End-to-end testing
5. **Validation**: Full integration test suite

### E.2 Test Strategy

| Test Suite | Coverage | Command |
|------------|----------|---------|
| `safety_tests.rs` | Circuit breaker, stall detector, budgets | `cargo test --test safety_tests` |
| `git_tools_tests.rs` | All git operations | `cargo test --test git_tools_tests` |
| `os_tools_tests.rs` | wc, bash_exec (with real tools) | `cargo test --test os_tools_tests` |
| `memory_tools_tests.rs` | memory_query, execution_summary | `cargo test --test memory_tools_tests` |
| `routing_tests.rs` | UI routing logic | `cargo test --test routing_tests` |
| `integration_tests.rs` | End-to-end tool execution | `cargo test --test integration_tests` |

### E.3 Validation Commands

```bash
# After each step
cargo test --all-targets

# Clippy checks
cargo clippy --all-targets -D warnings

# Format check
cargo fmt --check

# Full build
cargo build --release

# Integration test (requires external tools)
cargo test --test integration_tests -- --nocapture
```

---

## 7. SUMMARY

### Files to Create (13 new files)

| File | Purpose | LOC Est |
|------|---------|---------|
| `src/execution_engine/circuit_breaker.rs` | Circuit breaker state machine | 200 |
| `src/execution_engine/stall_detector.rs` | Loop/stall detection | 150 |
| `src/execution_engine/safety_config.rs` | Safety configuration | 80 |
| `src/os_tools/mod.rs` | OS tools module | 20 |
| `src/os_tools/wc.rs` | Word/line/byte count | 120 |
| `src/os_tools/bash_exec.rs` | Bash execution with allowlist | 250 |
| `src/git_tools/mod.rs` | Git tools module | 20 |
| `src/git_tools/status.rs` | Git status | 100 |
| `src/git_tools/diff.rs` | Git diff | 120 |
| `src/git_tools/log.rs` | Git log | 100 |
| `src/execution_tools/memory_query.rs` | Query past executions | 180 |
| `src/execution_tools/execution_summary.rs` | Lean summary generation | 120 |
| `src/ui/explorer.rs` | Structural results display | 200 |

### Files to Modify (5 files)

| File | Changes | LOC Est |
|------|---------|---------|
| `src/execution_engine/output_kind.rs` | Add routing rules | +60 |
| `src/execution_engine/tool_mapper.rs` | Add new tools | +80 |
| `src/execution_engine/chat_tool_runner.rs` | Integrate safety | +50 |
| `src/lib.rs` | Export new modules | +10 |
| `src/ui/state.rs` | Add routing state | +30 |

### Total New Code

- **New files**: ~1,770 LOC
- **Modifications**: ~230 LOC
- **Tests**: ~800 LOC
- **Total**: ~2,800 LOC

All files within ≤300 LOC constraint.

---

## 8. FIRST CONCRETE CODE CHANGES

To begin implementation, start with:

1. **Phase B.1**: Create `src/execution_engine/circuit_breaker.rs`
2. **Phase B.2**: Create `src/execution_engine/stall_detector.rs`
3. **Phase B.3**: Create `src/execution_engine/safety_config.rs`

These three files form the foundation for execution safety. Once implemented and tested, proceed to tool implementation (Phase A) and UI routing (Phase C).

---

## APPENDIX: REFERENCE IMPLEMENTATIONS

### OpenCode Tooling Model Reference

The design of this tooling layer is informed by OpenCode's tooling model, with the following behavioral patterns extracted:

1. **Safety First**
   - Always check file modification times
   - Require user confirmation for destructive operations
   - Validate inputs before execution
   - Provide clear error messages

2. **Context Awareness**
   - Track file read/write times
   - Maintain working directory context
   - Session-based operation grouping
   - Permission scoping

3. **Deterministic Behavior**
   - Sort search results consistently
   - Use absolute paths internally
   - Generate reproducible diffs
   - Consistent output formatting

4. **Progressive Disclosure**
   - Show summaries first, details on demand
   - Truncate large outputs intelligently
   - Provide "more results" indicators
   - Link to full outputs when available

5. **Integration-Ready**
   - LSP integration for diagnostics
   - Git workflow integration
   - Plugin system extensibility
   - SDK for external integrations

6. **User Control**
   - Configurable timeouts
   - Permission overrides
   - Tool enable/disable per session
   - Custom command definitions

7. **Error Prevention**
   - Read files before editing
   - Check file existence before operations
   - Validate uniqueness of search patterns
   - Prevent accidental overwrites

This plan implements these behavioral patterns in idiomatic Rust, with additional safety mechanisms (circuit breakers, stall detection) not present in OpenCode.
