# Phase 3 — Plan Execution Engine Design Decisions

**Date**: 2025-12-24
**Status**: DECISIONS LOCKED — Implementation approved
**Reference**: docs/PHASE_3_PLAN_EXECUTION.md

---

## SECTION 1 — PUBLIC API

### Primary Entry Point

```rust
/// Executor: synchronous step runner for approved plans
pub struct Executor {
    db: ExecutionDb,
    magellan_db: Option<MagellanDb>,
    confirmation_callback: Box<dyn ConfirmationCallback>,
    progress_callback: Box<dyn ProgressCallback>,
}

impl Executor {
    /// Create new executor with database connections and callbacks
    pub fn new(
        db: ExecutionDb,
        magellan_db: Option<MagellanDb>,
        confirmation_callback: Box<dyn ConfirmationCallback>,
        progress_callback: Box<dyn ProgressCallback>,
    ) -> Self;

    /// Execute an approved plan
    ///
    /// Returns ExecutionResult with all completed step results.
    /// Stops immediately on first failure.
    pub fn execute(&mut self, approved: ApprovedPlan) -> Result<ExecutionResult, ExecutionError>;
}
```

### Input Type

```rust
/// Approved plan for execution
///
/// Combines a validated Plan with user authorization.
pub struct ApprovedPlan {
    pub plan: Plan,
    pub authorization: PlanAuthorization,
}
```

**Validation** (called by `execute()` before running):
- `authorization.is_approved() == true`
- `plan.plan_id == authorization.plan_id`

### Output Types

```rust
/// Execution result
///
/// Contains all step results executed (up to failure point).
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionResult {
    pub plan_id: String,
    pub status: ExecutionStatus,
    pub step_results: Vec<StepResult>,
    pub total_duration_ms: i64,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    Completed,  // All steps succeeded
    Failed,     // Stopped at failure
    Partial,    // Reserved for future
}

/// Single step execution result
#[derive(Debug, Clone, PartialEq)]
pub struct StepResult {
    pub step_id: String,
    pub tool_name: String,
    pub success: bool,
    pub execution_id: String,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: i64,
    pub diagnostic_artifacts: Vec<DiagnosticArtifact>,
}

/// Diagnostic artifact (from lsp_check)
#[derive(Debug, Clone, PartialEq)]
pub struct DiagnosticArtifact {
    pub level: String,
    pub message: String,
    pub file_name: String,
    pub line_start: i64,
    pub code: Option<String>,
}
```

---

## SECTION 2 — CALLBACK TRAITS

### Confirmation Callback

```rust
/// Callback for user confirmation during execution
///
/// Called when step.requires_confirmation == true.
/// Executor BLOCKS until callback returns.
pub trait ConfirmationCallback: Send + Sync {
    /// Request user approval for a step
    ///
    /// Returns true if user approves, false if denied.
    fn request_confirmation(&self, step: &Step) -> bool;
}
```

**Auto-approve implementation** (for testing):
```rust
pub struct AutoApprove;

impl ConfirmationCallback for AutoApprove {
    fn request_confirmation(&self, _step: &Step) -> bool {
        true  // Always approve
    }
}
```

**Auto-deny implementation** (for testing):
```rust
pub struct AutoDeny;

impl ConfirmationCallback for AutoDeny {
    fn request_confirmation(&self, _step: &Step) -> bool {
        false  // Always deny
    }
}
```

### Progress Callback

```rust
/// Callback for progress updates during execution
pub trait ProgressCallback: Send + Sync {
    /// Called before step execution begins
    fn on_step_start(&self, step: &Step);

    /// Called after step completes successfully
    fn on_step_complete(&self, result: &StepResult);

    /// Called after step fails
    fn on_step_failed(&self, result: &StepResult);
}
```

**No-op implementation** (for testing):
```rust
pub struct NoopProgress;

impl ProgressCallback for NoopProgress {
    fn on_step_start(&self, _step: &Step) {}
    fn on_step_complete(&self, _result: &StepResult) {}
    fn on_step_failed(&self, _result: &StepResult) {}
}
```

---

## SECTION 3 — ERROR TYPES

```rust
/// Execution engine errors
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Invalid plan: {0}")]
    InvalidPlan(String),

    #[error("Plan not authorized: {0}")]
    NotAuthorized(String),

    #[error("Plan ID mismatch: plan='{plan}', auth='{auth}'")]
    PlanIdMismatch { plan: String, auth: String },

    #[error("Precondition failed for step '{step}': {precondition} - {reason}")]
    PreconditionFailed {
        step: String,
        precondition: String,
        reason: String,
    },

    #[error("Tool not found in whitelist: '{0}'")]
    ToolNotFound(String),

    #[error("Missing required argument '{argument}' for tool '{tool}'")]
    MissingArgument { tool: String, argument: String },

    #[error("Tool execution failed: {tool} - {error}")]
    ToolExecutionFailed { tool: String, error: String },

    #[error("Execution DB error: {0}")]
    ExecutionDbError(#[from] crate::execution_tools::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Confirmation denied by user for step '{0}'")]
    ConfirmationDenied(String),
}
```

---

## SECTION 4 — TOOL MAPPING

The executor maps `step.tool` strings to Phase 0 function calls:

| step.tool | Function | Arguments (from step.arguments) |
|-----------|----------|---------------------------------|
| file_read | `file_read(Path::new(&path))` | path |
| file_write | `file_write(Path::new(&path), &contents)` | path, contents |
| file_create | `file_create(Path::new(&path), &contents)` | path, contents |
| file_search | `file_search(&pattern, Path::new(&root))` | pattern, root |
| file_glob | `file_glob(&pattern, Path::new(&root))` | pattern, root |
| splice_patch | `splice_patch(&PatchArgs { file, symbol, with, ... })` | file, symbol, with |
| splice_plan | `splice_plan(&PlanArgs { plan_file })` | plan_file |
| symbols_in_file | `magellan_db.symbols_in_file(&file_path)` | file_path |
| references_to_symbol_name | `magellan_db.references_to_symbol_name(&symbol)` | symbol |
| references_from_file_to_symbol_name | `magellan_db.references_from_file_to_symbol_name(&file_path, &symbol)` | file_path, symbol |
| lsp_check | `lsp_check(Path::new(&path))` | path |

**Notes**:
- `splice_patch` requires: `file`, `symbol`, `with` (all required)
- `splice_plan` requires: `plan_file`
- Magellan tools return `Error` if `magellan_db` is `None`

---

## SECTION 5 — PRECONDITION CHECKS

Each `step.precondition` maps to a runtime check:

| Precondition | Check Function |
|--------------|----------------|
| file exists | `Path::new(path).exists()` |
| Cargo workspace exists | `Path::new(path).join("Cargo.toml").exists()` |
| codegraph.db exists | `db_root.join("codegraph.db").exists()` |
| symbol exists | Magellan query returns > 0 results |

**Implementation**: Each check returns `Result<(), String>` where `Err` contains reason.

---

## SECTION 6 — EXECUTION FLOW

```
execute(approved_plan)
    │
    ├─→ Validate authorization
    │   └─→ If not approved: return Err(NotAuthorized)
    │
    ├─→ Initialize: step_results = [], start_time = now()
    │
    ├─→ For each step in plan.steps:
    │   │
    │   ├─→ callback.on_step_start(step)
    │   │
    │   ├─→ Check precondition
    │   │   └─→ If fails: log failure, return ExecutionResult::Failed
    │   │
    │   ├─→ If requires_confirmation:
    │   │   └─→ If callback returns false: log denial, return ExecutionResult::Failed
    │   │
    │   ├─→ Invoke tool (via tool_mapper)
    │   │   ├─→ Capture: stdout, stderr, duration, success/error
    │   │   └─→ For lsp_check: capture diagnostics
    │   │
    │   ├─→ Log to execution_db (record_execution_with_artifacts)
    │   │
    │   ├─→ Build StepResult
    │   │
    │   ├─→ If success:
    │   │   └─→ callback.on_step_complete(result)
    │   │
    │   ├─→ If failure:
    │   │   └─→ callback.on_step_failed(result)
    │   │   └─→ return ExecutionResult::Failed
    │   │
    │   └─→ Continue to next step
    │
    └─→ return ExecutionResult::Completed
```

---

## SECTION 7 — EXECUTION ID GENERATION

Each step execution generates a unique execution_id:

```rust
use uuid::Uuid;

pub fn generate_execution_id() -> String {
    Uuid::new_v4().to_string()
}
```

**Dependency**: Add `uuid = { version = "1.0", features = ["v4"] }` to Cargo.toml

---

## SECTION 8 — ARTIFACT STORAGE

When logging each step, store artifacts:

| Tool | Artifacts |
|------|-----------|
| file_read | stdout (content) |
| file_write, file_create | (none, just success) |
| file_search | stdout (JSON array of matches) |
| file_glob | stdout (JSON array of paths) |
| splice_patch | stdout, stderr |
| splice_plan | stdout, stderr |
| Magellan tools | stdout (JSON array of rows) |
| lsp_check | stdout, stderr, diagnostics (JSON array) |

**Implementation**:
```rust
let mut artifacts: Vec<(&str, &serde_json::Value)> = Vec::new();

if !stdout.is_empty() {
    artifacts.push(("stdout", &json!(stdout)));
}
if !stderr.is_empty() {
    artifacts.push(("stderr", &json!(stderr)));
}
if !diagnostics.is_empty() {
    artifacts.push(("diagnostics", &json!(diagnostics)));
}

exec_db.record_execution_with_artifacts(
    &execution_id,
    &step.tool,
    &arguments_json,
    timestamp,
    success,
    exit_code,
    Some(duration_ms),
    error_message,
    &artifacts,
)?;
```

---

## SECTION 9 — MODULE CALL GRAPH

```
executor.rs (Executor::execute)
    │
    ├─→ preconditions.rs (check_precondition)
    │
    ├─→ tool_mapper.rs (invoke_tool)
    │   ├─→ file_tools
    │   ├─→ splice_tools
    │   ├─→ magellan_tools
    │   └─→ lsp_tools
    │
    ├─→ execution_tools (record_execution_with_artifacts)
    │
    └─→ result.rs (ExecutionResult, StepResult)
```

---

## SECTION 10 — TEST CATEGORIES

### Category A: Authorization Rejection
- Test: Execute with `AuthorizationStatus::Pending` → Err(NotAuthorized)
- Test: Execute with `AuthorizationStatus::Rejected` → Err(NotAuthorized)
- Test: Execute with mismatched plan_id → Err(PlanIdMismatch)

### Category B: Single-Step Success
- Test: Execute plan with 1 step (file_read) → ExecutionResult::Completed
- Test: Verify step_results.len() == 1
- Test: Verify execution_id logged to DB

### Category C: Failure Stops Execution
- Test: 3-step plan, step 2 fails → Only step 1 and 2 in results
- Test: Verify step 3 not executed
- Test: Verify status == Failed

### Category D: Confirmation Denied
- Test: step.requires_confirmation == true, callback returns false → ExecutionResult::Failed
- Test: Verify tool not called

### Category E: Evidence Logged
- Test: After execution, query DB for each execution_id
- Test: Verify artifacts stored correctly

### Category F: Deterministic Execution IDs
- Test: Execute same plan twice
- Test: Verify execution_ids are different (UUID v4)

### Category G: Forbidden Tool Rejection
- Test: step.tool = "rm -rf /" → Err(ToolNotFound)

### Category H: Precondition Failure
- Test: file_read on non-existent file → Err(PreconditionFailed)
- Test: Verify tool not called

---

STOP — Phase 3 design decisions complete; ready for STEP 4 (ACT).
