# Phase 3 — Plan Execution Engine

**Phase**: 3 — Plan Execution Engine
**Status**: PLANNING ONLY — NO CODE AUTHORIZED
**Type**: Execution Specification
**Dependency**: Phase 0 COMPLETE (0.1 → 0.6), Phase 1 COMPLETE, Phase 2 COMPLETE
**Date**: 2025-12-24

---

## SECTION 1 — SCOPE & NON-GOALS

### IN SCOPE (What Phase 3 WILL Do)

✅ **Execute validated plans** produced by Phase 2 LLM integration
✅ **Execute steps one at a time** in strict sequential order
✅ **Call existing Phase 0 tools** only (file, splice, magellan, lsp)
✅ **Record execution evidence** for every step via Phase 0.5 execution memory
✅ **Stop immediately on failure** — no partial completion, no "best effort"
✅ **Require explicit user approval** before executing any plan
✅ **Provide deterministic audit trail** — every execution reconstructable
✅ **Return step results** to UI for display

### OUT OF SCOPE (What Phase 3 WILL NOT Do)

❌ **NO async operations** — all execution is synchronous
❌ **NO background threads** — no concurrent execution
❌ **NO retries** — failure is failure; executor does not retry
❌ **NO autonomy** — executor only runs approved plans; no self-directed action
❌ **NO plan mutation** — executor executes plan exactly as validated
❌ **NO inference** — executor does not interpret or "fix" failed steps
❌ **NO tool chaining beyond plan** — executor calls only what the plan specifies
❌ **NO parallel execution** — steps execute sequentially, never concurrently
❌ **NO speculative execution** — no "predictive" or "preparatory" steps
❌ **NO policy layer** — executor does not judge plan quality; it executes

### Core Principle

**DETERMINISTIC EXECUTION** — Given the same approved plan, the executor produces identical results.

---

## SECTION 2 — EXECUTION ENGINE ROLE (WHAT IT IS / IS NOT)

### What the Execution Engine IS

The execution engine is a **deterministic step runner** that:

1. **Receives** an approved `Plan` from Phase 2
2. **Validates** that the plan has user authorization
3. **Executes** each `Step` in order using Phase 0 tools
4. **Records** every invocation to Phase 0.5 execution memory
5. **Returns** step results to UI for display
6. **Stops** immediately on first failure

### What the Execution Engine Is NOT

The execution engine is NOT:

| NOT | Reason |
|-----|--------|
| An LLM client | LLM integration is Phase 2; executor only runs approved plans |
| A scheduler | No concurrent execution, no resource management |
| A retry mechanism | Failure is terminal; no "try again" logic |
| A policy enforcer | Plan validation is Phase 2; executor executes what's approved |
| An interpreter | No script language; plans are pre-validated data structures |
| A state machine | Execution state is explicit (current step index), not implicit |
| An optimizer | No reordering, no batching, no "smart" execution |

### Trust Boundary

```
┌─────────────────────────────────────────────────────────────┐
│                     USER AUTHORIZATION                      │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│   APPROVED PLAN ──→ EXECUTION ENGINE ──→ TOOL INVOCATIONS    │
│                       (Phase 3)         (Phase 0)           │
│                                                           │
│   STEP RESULTS ◄── EXECUTION ENGINE ◄── TOOL OUTPUT          │
│                                                           │
│   EVIDENCE LOGGED ──→ EXECUTION MEMORY (Phase 0.5)          │
│                                                           │
└─────────────────────────────────────────────────────────────┘
```

The execution engine sits **between** the plan (Phase 2) and the tools (Phase 0). It has:

- **NO direct LLM access** — that's Phase 2's job
- **NO direct tool access beyond Phase 0 APIs** — uses existing interfaces
- **NO policy authority** — executes what was approved

---

## SECTION 3 — INPUT & OUTPUT CONTRACTS

### Input Contract

The execution engine accepts exactly ONE input type:

```rust
pub struct ApprovedPlan {
    pub plan: Plan,                    // From Phase 2
    pub authorization: PlanAuthorization, // User-approved
}
```

**Preconditions** (caller MUST satisfy):
1. `plan` MUST have passed `validate_plan()` from Phase 2
2. `authorization` MUST be in `AuthorizationStatus::Approved` state
3. `plan.plan_id` MUST match `authorization.plan_id`

**Executor MUST reject** if:
- Plan is not validated (return `ExecutionError::InvalidPlan`)
- Authorization is not approved (return `ExecutionError::NotAuthorized`)
- Plan ID mismatch (return `ExecutionError::PlanIdMismatch`)

### Output Contract

The execution engine produces exactly ONE output type:

```rust
pub struct ExecutionResult {
    pub plan_id: String,              // Copy of input plan_id
    pub status: ExecutionStatus,       // Completed, Failed, Partial
    pub step_results: Vec<StepResult>, // One per attempted step
    pub total_duration_ms: i64,       // Wall-clock time
}

pub enum ExecutionStatus {
    Completed,    // All steps succeeded
    Failed,        // Stopped at first failure
    Partial,       // Not possible under current spec (reserved)
}

pub struct StepResult {
    pub step_id: String,              // Copy of step.step_id
    pub tool_name: String,            // Copy of step.tool
    pub success: bool,                // true = succeeded, false = failed
    pub execution_id: String,         // UUID from execution memory
    pub stdout: Option<String>,       // Captured stdout (if any)
    pub stderr: Option<String>,       // Captured stderr (if any)
    pub error_message: Option<String>, // Error (if failed)
    pub duration_ms: i64,            // Execution time
    pub diagnostic_artifacts: Vec<DiagnosticArtifact>, // From lsp_check
}

pub struct DiagnosticArtifact {
    pub level: String,      // "error", "warning", "info"
    pub message: String,    // Diagnostic message
    pub file_name: String,  // Affected file
    pub line_start: i64,    // Line number (if applicable)
    pub code: Option<String>, // Error code (e.g., "E0425")
}
```

**Guarantees**:
- `step_results` length ≤ `plan.steps` length (stops on failure)
- `step_results` is in same order as `plan.steps` (up to failure point)
- Every `step_results[i]` corresponds to `plan.steps[i]`
- Every successful step has a valid `execution_id` in execution memory
- Total duration is sum of all step durations

---

## SECTION 4 — PLAN STEP MODEL

### Step Execution Flow

For each step in the plan (in order):

```
┌─────────────────────────────────────────────────────────────┐
│ STEP EXECUTION                                              │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│ 1. CHECK PRECONDITION                                        │
│    → Verify step precondition (file exists, Cargo project, etc.) │
│    → If fails: return StepResult{success: false}           │
│    → STOP                                                    │
│                                                           │
│ 2. CHECK CONFIRMATION (if required)                          │
│    → If step.requires_confirmation:                         │
│    →   Prompt user: "Execute step N: TOOL(ARGS)?"           │
│    →   If user denies: return StepResult{success: false}    │
│    →   STOP                                                    │
│                                                           │
│ 3. INVOKE TOOL                                               │
│    → Map step.tool to Phase 0 function                      │
│    → Serialize arguments from step.arguments                  │
│    → Call tool synchronously                                 │
│    → Capture stdout, stderr, diagnostics                    │
│                                                           │
│ 4. RECORD EXECUTION                                          │
│    → Generate execution_id (UUID)                           │
│    → Call ExecutionDb::record_execution_with_artifacts()    │
│    → Store: tool_name, arguments, timestamp, success,       │
│    →        stdout, stderr, diagnostics                     │
│                                                           │
│ 5. RETURN STEP RESULT                                        │
│    → If success: continue to next step                      │
│    → If failure: STOP, return ExecutionResult::Failed       │
│                                                           │
└─────────────────────────────────────────────────────────────┘
```

### Step-to-Tool Mapping

The executor maps `step.tool` strings to Phase 0 functions:

| step.tool | Phase 0 Function | Module | Output Handling |
|-----------|-----------------|--------|-----------------|
| `file_read` | `file_read(path)` | file_tools | Returns String content |
| `file_write` | `file_write(path, contents)` | file_tools | Returns (), success/failure |
| `file_create` | `file_create(path, contents)` | file_tools | Returns (), success/failure |
| `file_search` | `file_search(pattern, root)` | file_tools | Returns Vec<SearchMatch> |
| `file_glob` | `file_glob(pattern, root)` | file_tools | Returns Vec<PathBuf> |
| `splice_patch` | `splice_patch(args)` | splice_tools | Returns SpliceResult |
| `splice_plan` | `splice_plan(plan_file)` | splice_tools | Returns SpliceResult |
| `symbols_in_file` | `MagellanDb::symbols_in_file()` | magellan_tools | Returns Vec<SymbolRow> |
| `references_to_symbol_name` | `MagellanDb::references_to_symbol_name()` | magellan_tools | Returns Vec<ReferenceRow> |
| `references_from_file_to_symbol_name` | `MagellanDb::references_from_file_to_symbol_name()` | magellan_tools | Returns Vec<ReferenceRow> |
| `lsp_check` | `lsp_check(path)` | lsp_tools | Returns Vec<Diagnostic> |

**Executor MUST NOT** call any function not in this mapping.

**Executor MUST reject** any `step.tool` not in the whitelist (return `ExecutionError::UnknownTool`).

---

## SECTION 5 — EXECUTION STATE MACHINE

### States

```rust
pub enum ExecutorState {
    Idle,                           // No plan loaded
    PlanLoaded(ApprovedPlan),      // Plan loaded, not started
    StepInProgress {                // Currently executing a step
        plan: ApprovedPlan,
        current_step: usize,
        completed_steps: Vec<StepResult>,
    },
    Completed(ExecutionResult),     // All steps succeeded
    Failed(ExecutionResult),        // Stopped at failure
}
```

### State Transitions

```
┌──────────┐
│   Idle   │
└─────┬────┘
      │ load_plan(approved_plan)
      ▼
┌──────────────┐
│  PlanLoaded  │
└───────┬───────┘
        │ execute_step()
        ▼
┌──────────────────────┐
│  StepInProgress      │◄─────────────────┐
│  (step N of M)       │                   │
└───────┬──────────────┘                   │
        │                                  │
        ├─ success AND more steps?       │
        │   ▼                              │
        │ ┌─────────────────────┐          │
        │ │  StepInProgress      │          │
        │ │  (step N+1 of M)      │──────────┘
        │ └──────────▲───────────┘
        │            │
        │            └─ (repeat)
        │
        ├─ success AND last step
        │   ▼
        │ ┌──────────┐
        │ │ Completed │
        │ └──────────┘
        │
        └─ failure (precondition OR tool)
            ▼
        ┌──────────┐
        │  Failed  │
        └──────────┘
```

### Invariant

**At all times**: `completed_steps.len() == current_step`

**In English**: The number of completed steps equals the current step index (0 before first step, 1 after first, etc.).

---

## SECTION 6 — UI ↔ EXECUTOR BOUNDARY

### UI Initiates Execution

```rust
// UI calls executor with approved plan
let result = executor.execute(approved_plan)?;

// UI displays result
match &result.status {
    ExecutionStatus::Completed => {
        ui.show_message("All steps completed successfully");
    }
    ExecutionStatus::Failed => {
        ui.show_message(&format!("Execution failed at step {}", result.step_results.len()));
    }
}

// UI shows each step result
for step_result in &result.step_results {
    ui.show_step_result(step_result);
}
```

### Executor Requests User Confirmation

When a step has `requires_confirmation == true`:

```rust
// executor (NOT UI) prompts via callback
trait ConfirmationCallback {
    fn request_confirmation(&self, step: &Step) -> bool;
}

// UI implements callback
struct UiCallback;

impl ConfirmationCallback for UiCallback {
    fn request_confirmation(&self, step: &Step) -> bool {
        // Display prompt to user
        // Return true if user approves, false otherwise
        // This is a BLOCKING call — executor waits
    }
}
```

**Requirement**: Executor MUST block on confirmation request. No async, no timeout.

### UI Displays Step Progress

Executor provides progress updates during execution:

```rust
trait ProgressCallback {
    fn on_step_start(&self, step: &Step);
    fn on_step_complete(&self, result: &StepResult);
    fn on_step_failed(&self, result: &StepResult);
}
```

UI implements these callbacks to update display in real-time.

---

## SECTION 7 — TOOL INVOCATION RULES

### Rule 1: Exact Argument Matching

Executor MUST pass arguments to tools **exactly as specified** in `step.arguments`.

- NO additional arguments
- NO omitted arguments
- NO argument transformation
- NO "smart" defaults

**Example**:
```rust
// Step says:
step.arguments = {"file": "src/lib.rs", "symbol": "foo"}

// Executor calls:
splice_patch(
    file: "src/lib.rs",   // exact match
    symbol: "foo",         // exact match
    with: ???              // ERROR: missing required argument
)
```

If required arguments are missing, executor returns `ExecutionError::MissingArgument`.

### Rule 2: Whitelist Enforcement

Executor MUST reject any `step.tool` not in the 11-tool whitelist.

**Valid tools** (from Phase 2):
- `file_read`, `file_write`, `file_create`
- `file_search`, `file_glob`
- `splice_patch`, `splice_plan`
- `symbols_in_file`, `references_to_symbol_name`, `references_from_file_to_symbol_name`
- `lsp_check`

**Invalid** (executor rejects):
- Any tool not in above list
- Typos: `file_rad`, `splice_pach`, etc.
- New tools: `git_commit`, `cargo_build`, etc.

### Rule 3: Synchronous Blocking Calls

All tool invocations MUST be synchronous:

```rust
// WRONG (async):
let result = tokio::spawn(tool_call()).await?;

// WRONG (background thread):
let handle = thread::spawn(|| tool_call());
let result = handle.join().unwrap();

// CORRECT (blocking):
let result = tool_call()?;  // blocks until complete
```

Executor does NOT proceed until tool returns (or fails).

### Rule 4: No Tool Chaining

Each step invokes exactly ONE tool.

**Wrong** (chaining):
```rust
// One step calling multiple tools
file_read(path)?;
file_write(path, new_contents)?;
```

**Correct** (separate steps):
```rust
// Step 1: read
file_read(path)?

// Step 2: write
file_write(path, new_contents)?
```

### Rule 5: Output Capture

Executor MUST capture all tool outputs:

| Tool | Captured Outputs |
|------|-----------------|
| `file_read` | Content (as String) |
| `file_write`, `file_create` | Success/failure |
| `file_search` | Matches (Vec<SearchMatch>) |
| `file_glob` | Paths (Vec<PathBuf>) |
| `splice_patch`, `splice_plan` | SpliceResult (stdout, changed_files) |
| Magellan tools | Query results (Vec<Row>) |
| `lsp_check` | Diagnostics (Vec<Diagnostic>) |

All captured outputs MUST be stored in execution memory artifacts.

---

## SECTION 8 — EVIDENCE & LOGGING GUARANTEES

### Every Step MUST Be Logged

For each executed step, executor MUST call:

```rust
exec_db.record_execution_with_artifacts(
    execution_id,
    step.tool,
    arguments_json,
    timestamp,
    success,
    exit_code,
    duration_ms,
    error_message,
    artifacts,
)?;
```

### Artifact Storage

| Artifact Type | Content | Required For |
|---------------|---------|--------------|
| `stdout` | Tool stdout | splice_patch, splice_plan |
| `stderr` | Tool stderr | splice_patch, splice_plan, lsp_check |
| `diagnostics` | Structured diagnostics | lsp_check |
| `result` | Return value (JSON) | All tools |
| `step_context` | step_id, plan_id | Audit trail |

**Requirement**: All artifacts MUST be JSON-serializable.

### Execution ID Generation

Each step execution generates a unique `execution_id`:

```rust
pub fn generate_execution_id(plan_id: &str, step_id: &str) -> String {
    format!("{}:{}", plan_id, step_id)  // Not a UUID; collision is possible
}

// BETTER:
use uuid::Uuid;
pub fn generate_execution_id(_plan_id: &str, _step_id: &str) -> String {
    Uuid::new_v4().to_string()  // Guaranteed unique
}
```

**Requirement**: Execution IDs MUST be unique across all time.

### Audit Trail Completeness

Given an `execution_id`, the system MUST be able to reconstruct:

1. **What** was executed (tool_name, arguments)
2. **When** it was executed (timestamp)
3. **Whether** it succeeded (success, exit_code, error_message)
4. **What** was produced (stdout, stderr, diagnostics, result)
5. **Why** it was executed (plan_id, step_id, user_intent)

This information MUST be queryable via:
- Direct SQL query on `execution_log.db`
- Evidence queries (Q1-Q8) from Phase 0.6
- Execution details API

---

## SECTION 9 — FAILURE SEMANTICS

### Failure Categories

```rust
pub enum ExecutionError {
    // Input validation failures
    InvalidPlan(String),
    NotAuthorized(String),
    PlanIdMismatch,

    // Precondition failures
    PreconditionFailed {
        step_id: String,
        precondition: String,
        reason: String,
    },

    // Tool invocation failures
    ToolNotFound {
        tool: String,
    },
    MissingArgument {
        tool: String,
        argument: String,
    },
    ToolExecutionFailed {
        tool: String,
        error: String,
    },

    // System failures
    ExecutionMemoryError(String),
    IoError(String),
}
```

### Failure Behavior

When failure occurs:

1. **STOP immediately** — do not execute remaining steps
2. **Record the failure** — log to execution memory with `success = false`
3. **Return partial result** — `ExecutionResult::Failed` with completed steps
4. **DO NOT retry** — failure is terminal

**Example**:
```
Plan: 3 steps
Step 1: file_read("src/lib.rs") → SUCCESS
Step 2: splice_patch(...) → FAILURE (symbol not found)
Step 3: file_write(...) → NOT EXECUTED

Result: ExecutionResult::Failed {
    step_results: [
        StepResult { step_id: "step_1", success: true, ... },
        StepResult { step_id: "step_2", success: false, error: "symbol not found", ... },
    ],
    status: ExecutionStatus::Failed,
}
```

### Precondition Failures

Precondition checks happen BEFORE tool invocation:

| Precondition | Check | Failure If |
|--------------|-------|------------|
| `file exists` | `Path::new(path).exists()` | File not found |
| `Cargo project exists` | `Path::new(path).join("Cargo.toml").exists()` | No Cargo.toml |
| `codegraph.db exists` | `Path::new(db_root).join("codegraph.db").exists()` | DB not found |
| `symbol exists` | Magellan query returns 0 results | Symbol not found |

**Precondition failure is NOT a tool failure** — tool is never called.

---

## SECTION 10 — SECURITY & DETERMINISM GUARANTEES

### Security Guarantees

#### G1: No Execution Without Approval

Executor MUST NOT execute any step without:

1. **Validated plan** — passed `validate_plan()` from Phase 2
2. **User authorization** — `PlanAuthorization::is_approved() == true`

**Test**: `execute(ApprovedPlan { authorization: Pending })` → MUST return `ExecutionError::NotAuthorized`

#### G2: No Tool Whitelist Bypass

Executor MUST NOT call any tool outside the 11-tool whitelist.

**Test**: Attempt to execute step with `tool = "rm -rf /"` → MUST return `ExecutionError::ToolNotFound`

#### G3: No Argument Mutation

Executor MUST pass arguments exactly as specified.

**Test**: Step with `{"path": "../../etc/passwd"}` → tool receives exactly that string (no sanitization, no path expansion)

**Note**: Tools are responsible for their own security. Executor does NOT validate arguments beyond schema.

#### G4: No Privilege Escalation

Executor runs with same privileges as UI process.

- NO subprocess privilege changes
- NO `setuid`, `sudo`, etc.
- ALL tool calls inherit executor's permissions

### Determinism Guarantees

#### D1: Same Plan → Same Results

Given the same approved plan, executor MUST produce identical:

- Execution IDs (use deterministic generation or persist mapping)
- Step results (same tool outputs, same success/failure)
- Timing (not exact duration, but same order)
- Execution log entries (same timestamps ± drift, same artifacts)

**Test**: Execute same plan twice → `step_results[i].execution_id` identical

#### D2: No Hidden State

Executor state consists ONLY of:

- Current plan (ApprovedPlan)
- Current step index (usize)
- Completed step results (Vec<StepResult>)

**No hidden state**:
- NO caches
- NO "optimizations" that skip steps
- NO "remembered" results from previous runs

#### D3: No Non-Deterministic Ordering

Steps execute in plan order, never:
- Reordered (step N before step N-1)
- Parallelized (step N and step N+1 concurrently)
- Skipped (unless failure stops execution)

#### D4: Reproducible Evidence Logs

Given execution_id, query MUST return same results across runs.

**Test**: `SELECT * FROM executions WHERE id = ?` → deterministic

---

## SECTION 11 — TEST MATRIX (PLANNING ONLY)

### Unit Tests (Per Module)

| Module | Test Category | Example Test |
|--------|---------------|--------------|
| `executor.rs` | State transitions | `test_idle_to_plan_loaded` |
| `executor.rs` | Precondition checks | `test_precondition_file_not_exists` |
| `executor.rs` | Tool invocation | `test_invoke_file_read_success` |
| `executor.rs` | Tool invocation failure | `test_invoke_file_read_not_found` |
| `executor.rs` | Confirmation callback | `test_confirmation_denied_stops_execution` |
| `executor.rs` | Evidence logging | `test_step_execution_logged` |
| `executor.rs` | Failure stops execution | `test_failure_halts_remaining_steps` |
| `types.rs` | ExecutionResult serialization | `test_result_serializes_to_json` |

### Integration Tests

| Test Category | Description |
|---------------|-------------|
| Full plan execution | Execute 3-step plan, verify all succeed |
| Failure at step 1 | Fail at first step, verify remaining not executed |
| Failure at step N | Fail at middle step, verify partial results |
| Confirmation required | Prompt user, deny, verify execution stops |
| Evidence completeness | Execute plan, query execution memory, verify all steps logged |
| Determinism | Execute same plan twice, verify identical execution_ids |

### Property-Based Tests

| Property | Description |
|----------|-------------|
| Step count invariants | `result.step_results.len() ≤ plan.steps.len()` |
| Execution ID uniqueness | All `execution_id` values are unique |
| No skipped steps | No gaps in `step_results` indices |
| Timestamp ordering | `step_results[i].timestamp ≤ step_results[i+1].timestamp` |

---

## SECTION 12 — EXPLICIT NON-FEATURES

### Features That Are NOT Included

| Feature | Status | Reason |
|---------|--------|--------|
| Async execution | ❌ NOT INCLUDED | Synchronous only |
| Background execution | ❌ NOT INCLUDED | No threads, no tasks |
| Parallel step execution | ❌ NOT INCLUDED | Steps execute sequentially |
| Retry logic | ❌ NOT INCLUDED | Failure is terminal |
| Rollback | ❌ NOT INCLUDED | No undo; changes persist |
| Checkpoint/resume | ❌ NOT INCLUDED | No pause/resume capability |
| Plan modification | ❌ NOT INCLUDED | Executor executes plan exactly |
| Plan optimization | ❌ NOT INCLUDED | No reordering, batching |
| Dry-run mode | ❌ NOT INCLUDED | Plan validation is Phase 2; Phase 3 executes |
| Progress bars | ❌ NOT INCLUDED | UI concern, not executor |
| Interactive debugging | ❌ NOT INCLUDED | No breakpoints, no step-through |
| Transaction semantics | ❌ NOT INCLUDED | No atomic multi-step commits |
| Compensation actions | ❌ NOT INCLUDED | No "cleanup on failure" |
| Plan templates | ❌ NOT INCLUDED | Plans are from LLM, not templates |
| Macro recording | ❌ NOT INCLUDED | No capture-and-replay |
| Plan scheduling | ❌ NOT INCLUDED | No queue, no delayed execution |
| Resource limits | ❌ NOT INCLUDED | No timeout, no memory limits |
| Sandboxing | ❌ NOT INCLUDED | Tools run as normal process |

### Features Reserved for Future Phases

| Feature | Possible Future Phase |
|---------|----------------------|
| Async execution | NOT PLANNED (violates determinism) |
| Parallel execution | NOT PLANNED (violates step-by-step) |
| Rollback | Phase 4+ (requires undo infrastructure) |
| Checkpoint/resume | Phase 4+ (requires state serialization) |

---

## SECTION 13 — MODULE STRUCTURE (PROPOSED)

```
src/execution_engine/
├── mod.rs                  # Module exports, public types
├── executor.rs             # Executor struct, execute() method
├── step_runner.rs          # Step execution logic
├── preconditions.rs        # Precondition checking
├── tool_mapper.rs          # step.tool → Phase 0 function
├── result.rs               # ExecutionResult, StepResult types
└── errors.rs               # ExecutionError enum
```

**Constraint**: Each file ≤ 300 LOC (tests exempt)

---

## SECTION 14 — DEPENDENCY SUMMARY

### Phase 3 Depends On

| Phase | Module | Usage |
|-------|--------|-------|
| Phase 0 | `file_tools` | Direct tool calls |
| Phase 0 | `splice_tools` | Direct tool calls |
| Phase 0 | `magellan_tools` | Direct tool calls |
| Phase 0 | `lsp_tools` | Direct tool calls |
| Phase 0.5 | `execution_tools` | Evidence logging |
| Phase 2 | `llm::types` | Plan, Step, Intent types |
| Phase 2 | `llm::planner` | Validation (re-check before execution) |

### Phase 3 Does NOT Depend On

| Phase | Module | Reason |
|-------|--------|--------|
| Phase 2 | `llm::contracts` | Prompt contracts not needed for execution |
| Phase 2 | `llm::session` | Session management is UI concern |
| Phase 1 | `ui::*` | Executor is headless; UI calls executor |
| External | LLM APIs | No LLM calls in Phase 3 |

---

## APPENDIX A: EXECUTION EXAMPLE

### Input Plan

```json
{
  "plan_id": "plan_001",
  "intent": "MUTATE",
  "steps": [
    {
      "step_id": "step_1",
      "tool": "file_read",
      "arguments": {"path": "src/lib.rs"},
      "precondition": "file exists",
      "requires_confirmation": false
    },
    {
      "step_id": "step_2",
      "tool": "splice_patch",
      "arguments": {
        "file": "src/lib.rs",
        "symbol": "old_function",
        "with": "patches/fix.rs"
      },
      "precondition": "symbol exists",
      "requires_confirmation": true
    },
    {
      "step_id": "step_3",
      "tool": "lsp_check",
      "arguments": {"path": "."},
      "precondition": "Cargo project exists",
      "requires_confirmation": false
    }
  ],
  "evidence_referenced": ["Q8"]
}
```

### Execution Flow

```
[USER APPROVES PLAN]
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│ EXECUTOR: Load plan, verify authorization             │
│ ✓ Plan validated                                      │
│ ✓ Authorization approved                             │
└─────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│ STEP 1: file_read("src/lib.rs")                       │
│ ✓ Precondition: file exists (checked)                 │
│ ✓ No confirmation required                              │
│ ✓ Tool: file_read() → "content..."                    │
│ ✓ Log: execution_id="uuid-1", success=true             │
└─────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│ STEP 2: splice_patch(...)                               │
│ ✓ Precondition: symbol exists (checked)               │
│ ⚠ REQUIRES CONFIRMATION                               │
│   UI: "Execute splice_patch on old_function?"          │
│   USER: YES                                            │
│ ✓ Tool: splice_patch() → SpliceResult {...}           │
│ ✓ Log: execution_id="uuid-2", success=true             │
└─────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│ STEP 3: lsp_check(".")                                  │
│ ✓ Precondition: Cargo project exists (checked)        │
│ ✓ No confirmation required                              │
│ ✓ Tool: lsp_check() → [] (no errors)                   │
│ ✓ Log: execution_id="uuid-3", success=true             │
└─────────────────────────────────────────────────────────┘
       │
       ▼
┌─────────────────────────────────────────────────────────┐
│ RESULT: ExecutionStatus::Completed                     │
│ step_results: 3 entries, all success=true               │
│ total_duration_ms: 1523                                 │
└─────────────────────────────────────────────────────────┘
```

### Failure Example

```
[STEP 2: splice_patch(...)]
✓ Precondition: symbol exists (checked)
⚠ REQUIRES CONFIRMATION
  UI: "Execute splice_patch on old_function?"
  USER: YES
✗ Tool: splice_patch() → Error: "symbol not found"
✗ Log: execution_id="uuid-2", success=false, error="symbol not found"

STOP — EXECUTION FAILED

RESULT: ExecutionStatus::Failed
step_results: 2 entries (step_1 success=true, step_2 success=false)
```

---

## APPENDIX B: ERROR CODE MAPPING

| Error Code | Phase | Description |
|-----------|-------|-------------|
| `E001` | Executor | Invalid plan (validation failed) |
| `E002` | Executor | Not authorized (user did not approve) |
| `E003` | Executor | Plan ID mismatch |
| `E101` | Executor | Precondition failed: file not found |
| `E102` | Executor | Precondition failed: Cargo project not found |
| `E103` | Executor | Precondition failed: codegraph.db not found |
| `E104` | Executor | Precondition failed: symbol not found |
| `E201` | Executor | Tool not found in whitelist |
| `E202` | Executor | Missing required argument |
| `E301` | Tool | file_read: file not found |
| `E302` | Tool | file_write: permission denied |
| `E303` | Tool | splice_patch: splice binary not found |
| `E304` | Tool | lsp_check: cargo check failed |

---

STOP — Phase 3 planning complete; awaiting acceptance or revisions.
