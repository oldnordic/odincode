# Phase 0.5 — Execution Memory & Outcome Logging

**Status**: PHASE 0.5.1 — SCHEMA FINALIZATION (LOCKED)
**Type**: Contract-Level Specification
**Dependency**: Phase 0.4 (LSP Integration) COMPLETE

---

## AUTHORITY

This document is **authoritative specification** for Phase 0.5.

It is **NOT a proposal**. It is the **required next layer** given completed tools.

---

## WHY PHASE 0.5 IS REQUIRED (FACTUAL BASIS)

### Current State (Phase 0 Complete)

**Execution Truth**: ✅ COMPLETE
- file_tools → Filesystem operations
- splice_tools → Span-safe mutations
- magellan_tools → Graph structure queries
- lsp_tools → Compiler diagnostics

**Deterministic Execution**: ✅ VERIFIED
- All 27 tests passing
- All tools produce reproducible results
- All operations grounded in facts

**Historical Truth**: ❌ MISSING
- Tool invocations are ephemeral
- Outcomes are not persisted
- No record of success/failure patterns
- No causal links between operations and errors

### The Problem (Last Source of Suffering)

The system can:
1. ✅ Read files
2. ✅ Query structure
3. ✅ Make changes
4. ✅ Verify compilation
5. ❌ **NOT REMEMBER WHAT HAPPENED**

This means:
- The LLM cannot learn from past mistakes
- Failed patterns are repeated
- Successful approaches are rediscovered
- No evidence-based constraint enforcement

**This is the final gap in deterministic execution.**

---

## PHASE 0.5 GOAL (STRICT)

### Primary Objective

**Persist tool execution outcomes as facts, not interpretations**

So future actions can be constrained by evidence, not memory.

### What This Is NOT

❌ NOT "learning" in ML sense
❌ NOT "scoring" or "ranking"
❌ NOT "embeddings" or vector search
❌ NOT "AI memory" or "agent context"
❌ NOT "heuristics" or "recommendations"

### What This IS

✅ Audit-grade logging
✅ Temporal causal relationships
✅ Evidence-based queries
✅ Deterministic pattern accumulation

**Think**: Version control for operations, not code.

---

## PHASE 0.5 SCOPE (PRECISE)

### New Tool Family

**execution_memory_tools/** (name negotiable, function is not)

Location: `src/execution_tools/`

### Responsibilities

#### 1. Record Tool Invocations

For every tool call (file_tools, splice_tools, magellan_tools, lsp_tools):

Store:
- `tool_name`: e.g., "splice_patch", "file_write", "lsp_check"
- `arguments`: Canonicalized JSON representation
- `target_paths`: Files affected (if any)
- `target_symbols`: Symbols affected (if any)
- `timestamp`: Unix timestamp (milliseconds)
- `execution_id`: UUID for operation grouping

#### 2. Record Outcomes

Store:
- `success`: Boolean
- `exit_code`: Integer (if applicable)
- `stdout`: Captured output (splice, cargo)
- `stderr`: Error output (splice, cargo)
- `diagnostics`: Structured JSON from lsp_tools
- `duration_ms`: Execution time
- `error_message`: Failure reason (if failed)

#### 3. Persist to Dual Storage

**SQLite** (Temporal Log)
- Table: `executions`
  - Columns: id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message
- Table: `execution_artifacts`
  - Columns: execution_id, artifact_type, content_json
  - artifact_type: "stdout", "stderr", "diagnostics"

**SQLiteGraph** (Causal Relationships)
- Table: `graph_entities` (existing)
  - Add rows for: execution nodes (kind: "execution")
- Table: `graph_edges` (existing)
  - Add edges: execution → file, execution → symbol, execution → diagnostic
  - edge_type: "EXECUTED_ON", "AFFECTED", "PRODUCED"

---

## WHAT THIS ENABLES (FACTUAL QUERIES)

After Phase 0.5, the system can ask evidence-based questions:

### Historical Pattern Queries

```rust
// Has this splice pattern failed before?
let failures = execution_log
    .failed_executions("splice_patch")
    .with_symbol("foo")
    .with_file("src/lib.rs");

// Which edits caused E0425 historically?
let error_causes = execution_log
    .executions_producing_diagnostic("E0425")
    .after_timestamp(start_time);

// Which fixes resolved error X in this repo?
let successful_fixes = execution_log
    .executions_resolving_error("E0425")
    .with_success(true);

// Is this action a known antipattern?
let antipatterns = execution_log
    .pattern_match(&current_action)
    .filter_failure_rate_threshold(0.7);
```

### Causal Tracing

```rust
// What operations affected this symbol before error?
let operations = graph_db
    .execution_chain_affecting_symbol("foo")
    .before_diagnostic(diagnostic_id);

// What files were touched by this operation?
let affected_files = graph_db
    .files_touched_by_execution(execution_id);
```

### Temporal Constraints

```rust
// Enforce: Don't repeat failed approach
if execution_log.has_failed_recently(&proposed_action, Duration::days(7)) {
    return Err(Error::KnownAntiPattern);
}

// Enforce: Only use proven approaches for this error
let proven_fixes = execution_log
    .successful_resolutions_for_error("E0425")
    .in_repository(repo_path);
```

---

## ARCHITECTURAL INTEGRATION

### Tool Wrapping

Each existing tool needs optional execution logging:

```rust
// Before (current)
let result = splice_patch(&args)?;

// After (with logging)
let result = splice_patch(&args)?;
execution_log.record(
    "splice_patch",
    &args,
    &result,
    &timestamp
)?;
```

**Constraint**: Logging MUST be optional, not mandatory.
- Tools work standalone (no logging)
- Logging is wrapper/add-on, not core dependency
- Tests can run without execution DB

### Database Initialization

```rust
// execution_tools/db.rs
pub struct ExecutionDb {
    sqlite_conn: Connection,
    graph_db_path: PathBuf,
}

impl ExecutionDb {
    // Creates execution_log.db + updates graph.db
    pub fn initialize(path: &Path) -> Result<Self> {
        // 1. Create execution_log.db with schema
        // 2. Connect to existing graph.db
        // 3. Add execution nodes/edges to graph
    }
}
```

---

## CONSTRAINTS (CARRY-FORWARD)

### Non-Negotiable

1. **NO ASYNC** - All logging synchronous
2. **NO BACKGROUND THREADS** - Log during execution only
3. **SQLite ONLY** - No external databases
4. **SQLiteGraph ONLY** - No custom graph storage
5. **READ/WRITE VIA TOOLS** - No direct DB access from tools
6. **FILES ≤ 300 LOC** - Each module
7. **TDD MANDATORY** - Write failing tests first
8. **REAL TOOLS ONLY** - No mock execution DB

### Optional Logging

- Tools MUST work without execution DB
- Logging is a concern, not a dependency
- Tests can run with in-memory DB

---

## LOCKED SCHEMA (CONTRACT-LEVEL)

This section is **frozen** and **non-negotiable**.
All implementation MUST conform to this schema exactly.
No deviations permitted without explicit contract renegotiation.

---

## 1. FINAL SQLITE SCHEMA (LOCKED)

### Database File

**Filename**: `execution_log.db`
**Location**: Same directory as `codegraph.db` (project root)
**Format**: SQLite 3

### Table: executions

**Purpose**: Append-only log of tool invocations

```sql
CREATE TABLE executions (
    -- Primary key: UUID v4 string format
    -- Example: "550e8400-e29b-41d4-a716-446655440000"
    id TEXT PRIMARY KEY NOT NULL,

    -- Tool name: exact function name from odincode API
    -- Allowed values:
    --   "file_read", "file_write", "file_create"
    --   "file_search", "file_glob"
    --   "splice_patch", "splice_plan"
    --   "symbols_in_file", "references_to_symbol_name", "references_from_file_to_symbol_name"
    --   "lsp_check"
    tool_name TEXT NOT NULL,

    -- Arguments: canonicalized JSON representation
    -- Format: JSON object with tool-specific structure
    -- Example for splice_patch:
    --   {"file": "src/lib.rs", "symbol": "foo", "kind": "fn", "with": "patches/foo.rs"}
    -- Example for lsp_check:
    --   {"path": "/path/to/project"}
    arguments_json TEXT NOT NULL,

    -- Timestamp: Unix milliseconds since epoch
    -- Example: 1735036800000 (2025-12-24 00:00:00 UTC)
    timestamp INTEGER NOT NULL,

    -- Success flag: true if operation completed without error
    -- NULL if operation was interrupted (crash, system failure)
    success BOOLEAN NOT NULL,

    -- Exit code: process exit code for external tool calls
    -- NULL for tools without exit codes (file operations, magellan queries)
    -- Standard interpretation: 0 = success, non-zero = failure
    exit_code INTEGER,

    -- Duration: execution time in milliseconds
    -- NULL if timing unavailable
    duration_ms INTEGER,

    -- Error message: human-readable failure description
    -- NULL if success=true
    -- Format: Free text, may contain structured error codes
    error_message TEXT
);
```

**Indexes**:
```sql
-- Query by tool name
CREATE INDEX idx_executions_tool ON executions(tool_name);

-- Query by time range (chronological queries)
CREATE INDEX idx_executions_timestamp ON executions(timestamp);

-- Query by success/failure status
CREATE INDEX idx_executions_success ON executions(success);

-- Composite index: tool + timestamp (common query pattern)
CREATE INDEX idx_executions_tool_timestamp ON executions(tool_name, timestamp);
```

**Constraints**:
```sql
-- tool_name must be from allowed set
CREATE TRIGGER validate_tool_name BEFORE INSERT ON executions
BEGIN
    SELECT CASE
        WHEN NEW.tool_name NOT IN (
            'file_read', 'file_write', 'file_create',
            'file_search', 'file_glob',
            'splice_patch', 'splice_plan',
            'symbols_in_file', 'references_to_symbol_name', 'references_from_file_to_symbol_name',
            'lsp_check'
        ) THEN RAISE(ABORT, 'Invalid tool_name')
    END;
END;

-- timestamp must be reasonable (not future, not before 2020-01-01)
CREATE TRIGGER validate_timestamp BEFORE INSERT ON executions
BEGIN
    SELECT CASE
        WHEN NEW.timestamp < 1577836800000 THEN RAISE(ABORT, 'Timestamp too old')
        WHEN NEW.timestamp > strftime('%s', 'now') * 1000 + 86400000 THEN RAISE(ABORT, 'Timestamp in future')
    END;
END;
```

**Invariants**:
- **Append-only**: Rows are NEVER updated or deleted
- **id uniqueness**: No two executions share same UUID
- **Timestamp monotonic**: Within a single session, timestamps are non-decreasing
- **No cascading deletes**: Foreign key reference does NOT trigger deletion

---

### Table: execution_artifacts

**Purpose**: Store structured outputs (stdout, stderr, diagnostics) linked to executions

```sql
CREATE TABLE execution_artifacts (
    -- Auto-increment primary key
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    -- Foreign key reference to executions table
    execution_id TEXT NOT NULL,

    -- Artifact type discriminator
    -- Allowed values (case-sensitive):
    --   "stdout" - Captured standard output from external tools
    --   "stderr" - Captured standard error from external tools
    --   "diagnostics" - Structured compiler diagnostics from lsp_tools
    artifact_type TEXT NOT NULL,

    -- Content: structured JSON representation
    -- For "stdout"/"stderr": JSON string with raw text
    --   {"text": "full output here"}
    -- For "diagnostics": JSON array of diagnostic objects
    --   [{"level": "error", "message": "...", "file_name": "...", "line_start": 1, "code": "E0425"}]
    content_json TEXT NOT NULL,

    -- Foreign key constraint
    FOREIGN KEY (execution_id) REFERENCES executions(id) ON DELETE RESTRICT
);
```

**Indexes**:
```sql
-- Query all artifacts for an execution
CREATE INDEX idx_artifacts_execution ON execution_artifacts(execution_id);

-- Query by artifact type
CREATE INDEX idx_artifacts_type ON execution_artifacts(artifact_type);

-- Composite: execution + type (specific artifact retrieval)
CREATE INDEX idx_artifacts_execution_type ON execution_artifacts(execution_id, artifact_type);
```

**Constraints**:
```sql
-- artifact_type must be from allowed set
CREATE TRIGGER validate_artifact_type BEFORE INSERT ON execution_artifacts
BEGIN
    SELECT CASE
        WHEN NEW.artifact_type NOT IN ('stdout', 'stderr', 'diagnostics') THEN
            RAISE(ABORT, 'Invalid artifact_type')
    END;
END;

-- content_json must be valid JSON
CREATE TRIGGER validate_json BEFORE INSERT ON execution_artifacts
BEGIN
    SELECT json_valid(NEW.content_json) = 1 OR RAISE(ABORT, 'Invalid JSON in content_json');
END;
```

**Invariants**:
- **Write-once**: Rows are NEVER updated or deleted
- **Execution linkage**: Every artifact references valid execution_id
- **Type specificity**: At most one artifact of each type per execution (enforced at application level)
- **JSON validity**: content_json is always valid JSON (enforced by trigger)

---

## 2. FINAL SQLITEGRAPH MAPPING (LOCKED)

### Graph Entity Kinds

**Execution Nodes**:
```sql
-- kind field value for execution entities
KIND_EXECUTION = "execution"
```

**Entity Schema**:
```sql
-- Insert execution entity into existing graph_entities table
INSERT INTO graph_entities (
    kind,
    name,
    file_path,
    data
) VALUES (
    'execution',           -- kind: fixed value
    '<tool_name>:<uuid>',  -- name: e.g., "splice_patch:550e8400-..."
    NULL,                  -- file_path: NULL for execution nodes
    '{                     -- data: JSON metadata
        "tool": "<tool_name>",
        "timestamp": <milliseconds>,
        "success": <true/false>,
        "execution_id": "<uuid>"
    }'
);
```

**Allowed tool_name values** (same as executions table):
- "file_read", "file_write", "file_create"
- "file_search", "file_glob"
- "splice_patch", "splice_plan"
- "symbols_in_file", "references_to_symbol_name", "references_from_file_to_symbol_name"
- "lsp_check"

**Data field schema** (JSON):
```json
{
    "tool": "splice_patch",
    "timestamp": 1735036800000,
    "success": true,
    "execution_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

---

### Graph Edge Types

**Edge Type Values** (case-sensitive):
```sql
EDGE_EXECUTED_ON  = "EXECUTED_ON"   -- execution → file/symbol
EDGE_AFFECTED     = "AFFECTED"      -- execution → file (modified)
EDGE_PRODUCED     = "PRODUCED"      -- execution → diagnostic
EDGE_REFERENCED   = "REFERENCED"    -- execution → symbol (queried)
```

---

### Required Edge Patterns

**1. Execution → File (EXECUTED_ON)**
```sql
-- When tool operates on a file
INSERT INTO graph_edges (
    from_id,
    to_id,
    edge_type,
    data
) VALUES (
    <execution_entity_id>,
    <file_entity_id>,
    'EXECUTED_ON',
    '{
        "operation": "<operation_type>",
        "execution_id": "<uuid>"
    }'
);
```

**Applies to**:
- file_read (operation_type: "read")
- file_write (operation_type: "write")
- file_create (operation_type: "create")
- splice_patch (operation_type: "patch")
- splice_plan (operation_type: "plan")
- lsp_check (operation_type: "check")

**2. Execution → Symbol (REFERENCED)**
```sql
-- When tool queries or affects a symbol
INSERT INTO graph_edges (
    from_id,
    to_id,
    edge_type,
    data
) VALUES (
    <execution_entity_id>,
    <symbol_entity_id>,
    'REFERENCED',
    '{
        "query_type": "<query_type>",
        "execution_id": "<uuid>"
    }'
);
```

**Applies to**:
- symbols_in_file (query_type: "symbols_in_file")
- references_to_symbol_name (query_type: "references_to_symbol_name")
- references_from_file_to_symbol_name (query_type: "references_from_file_to_symbol")
- splice_patch (query_type: "target_symbol")

**3. Execution → Diagnostic (PRODUCED)**
```sql
-- When tool produces compiler diagnostics
INSERT INTO graph_edges (
    from_id,
    to_id,
    edge_type,
    data
) VALUES (
    <execution_entity_id>,
    <diagnostic_entity_id>,
    'PRODUCED',
    '{
        "severity": "<error|warning>",
        "code": "<E0425>",
        "execution_id": "<uuid>"
    }'
);
```

**Applies to**:
- lsp_check (when diagnostics present)
- splice_patch (when splice produces errors)

**4. Execution → File (AFFECTED)**
```sql
-- When tool modifies a file
INSERT INTO graph_edges (
    from_id,
    to_id,
    edge_type,
    data
) VALUES (
    <execution_entity_id>,
    <file_entity_id>,
    'AFFECTED',
    '{
        "change_type": "<patched|created>",
        "execution_id": "<uuid>"
    }'
);
```

**Applies to**:
- file_write (change_type: "created" if new file)
- file_create (change_type: "created")
- splice_patch (change_type: "patched" when changed_files contains file)

---

### Forbidden Edges

**NEVER create these edges**:
- execution → execution (no execution-to-execution relationships)
- symbol → execution (reverse relationship only)
- diagnostic → execution (reverse relationship only)
- execution → execution (no chaining)

**Rationale**: Executions are temporal events, not structural entities. They reference structural entities (files, symbols, diagnostics) but are not referenced by them.

---

## 3. INVARIANTS (NON-NEGOTIABLE)

### SQLite Invariants

**I1: Execution Immutability**
- Rows in `executions` table are NEVER updated or deleted
- Corrections are made by inserting new rows with correction rationale in error_message

**I2: Artifact Write-Once**
- Rows in `execution_artifacts` table are NEVER updated or deleted
- Missing artifacts are indicated by absence, not NULL values

**I3: Timestamp Monotonicity**
- Within a single logging session, timestamps are non-decreasing
- Clock skew is detected and rejected if timestamp < last_timestamp - 60000 (1 minute tolerance)

**I4: UUID Uniqueness**
- No two executions share the same UUID
- UUID generation MUST use v4 (random) or v7 (time-ordered)

**I5: JSON Validity**
- All `*_json` columns contain valid JSON
- Enforced by database triggers

### SQLiteGraph Invariants

**G1: Entity Existence**
- All edge targets must reference existing entity IDs
- Foreign key constraints enforced at database level

**G2: Execution Entity Uniqueness**
- One execution entity per execution_id
- No duplicate execution nodes with same UUID

**G3: Edge Type Correctness**
- All edges use allowed edge_type values only
- Enforced by application logic (no database trigger due to existing schema)

**G4: Data Field Completeness**
- All graph entity data fields contain execution_id
- All graph edge data fields contain execution_id

### Cross-System Invariants

**X1: Dual-Write Consistency**
- Every execution in SQLite MUST have corresponding entity in SQLiteGraph
- Every execution_artifact MUST have corresponding edge in SQLiteGraph (if applicable)

**X2: Referential Integrity**
- execution_artifacts.execution_id references executions.id
- graph_edges reference valid entity IDs

**X3: Atomicity Expectation**
- Best-effort dual-write is REQUIRED
- System documents partial writes (see Failure Semantics below)
- No automatic rollback of successful writes

---

## 4. FAILURE SEMANTICS

### Write Ordering

**Atomic Dual-Write Attempt**:
```
1. BEGIN TRANSACTION (execution_log.db)
2. INSERT INTO executions
3. INSERT INTO execution_artifacts (if any)
4. COMMIT
5. BEGIN TRANSACTION (codegraph.db)
6. INSERT INTO graph_entities (execution node)
7. INSERT INTO graph_edges (all edges)
8. COMMIT
```

### Failure Cases

**Case 1: SQLite Write Fails (Step 2-4)**
- **Result**: No data written to either database
- **Recovery**: Return error to caller, no retry
- **State**: Consistent (both databases unchanged)

**Case 2: SQLiteGraph Write Fails (Step 5-8)**
- **Result**: Execution logged in execution_log.db, but missing from graph
- **Recovery**: Log discrepancy to internal error log, mark execution with special flag
- **State**: Inconsistent (temporal log exists, graph relationships missing)
- **Mitigation**: Provide repair tool to rebuild graph entities from execution_log

**Case 3: Partial SQLiteGraph Write Fails (Step 6-7)**
- **Result**: Some edges written, execution entity written, but incomplete
- **Recovery**: Log error, provide repair tool
- **State**: Inconsistent (graph has orphaned execution entity)

**Case 4: System Crash During Write**
- **Result**: Transaction rollback (SQLite ACID properties)
- **Recovery**: No partial writes
- **State**: Consistent (both databases unchanged)

### Retry Policy

**No Automatic Retries**:
- All write failures are immediately reported to caller
- No exponential backoff
- No retry loops

**Rationale**:
- Execution logging is a concern, not a dependency
- Tools must function even if logging fails
- Caller decides retry strategy

### Repair Semantics

**Graph Reconstruction Tool** (future, not part of Phase 0.5):
- Read all executions from execution_log.db
- Rebuild execution entities in graph_entities
- Rebuild all edges in graph_edges
- Idempotent: can be run multiple times safely

---

## 5. OUT-OF-SCOPE (RECONFIRMED)

### NOT Supported by This Schema

❌ **Query Language**
- No SQL query builder
- No graph query language
- No pattern matching syntax

❌ **Aggregation**
- No summary tables
- No materialized views
- No rollup statistics

❌ **Lifecycle Management**
- No pruning of old executions
- No retention policies
- No archival mechanisms

❌ **Cross-Session Relationships**
- No session grouping
- No workflow tracking
- No multi-step transaction support

❌ **Performance Optimization**
- No denormalization
- No caching layers
- No query optimization hints

❌ **Security**
- No access control
- No encryption at rest
- No authentication

### Rationale

These features are **intentionally excluded** from Phase 0.5:
- They add complexity beyond audit logging
- They can be added in future phases if needed
- Current schema focuses on facts, not convenience

---

## DATA SCHEMA (PRELIMINARY) - DEPRECATED

The schema section above has been LOCKED and promoted to contract-level specification.
The preliminary section has been removed to avoid ambiguity.

Reference the **LOCKED SCHEMA** section above for all implementation details.



---

## TEST REQUIREMENTS (TDD)

### Unit Tests

```rust
// tests/execution_tools_tests.rs

#[test]
fn test_record_execution_success() {
    // Create temp DB
    // Record splice_patch success
    // Query executions table
    // Assert: 1 row, success=true
}

#[test]
fn test_record_execution_with_diagnostics() {
    // Record lsp_check with errors
    // Query execution_artifacts
    // Assert: diagnostics JSON preserved
}

#[test]
fn test_query_failed_executions_by_tool() {
    // Record 3 splice_patch: 2 fail, 1 success
    // Query failed executions
    // Assert: returns 2 failures only
}

#[test]
fn test_graph_execution_nodes_created() {
    // Record execution
    // Query graph_entities for kind='execution'
    // Assert: 1 execution node exists
}

#[test]
fn test_graph_execution_file_edges() {
    // Record splice_patch on lib.rs
    // Query graph_edges for EXECUTED_ON
    // Assert: edge exists between execution and lib.rs
}
```

### Integration Tests

```rust
#[test]
fn test_full_workflow_logging() {
    // 1. file_read → record execution
    // 2. splice_patch → record execution + success
    // 3. lsp_check → record execution + diagnostics
    // 4. Query: all executions in chronological order
    // 5. Query: execution chain for lib.rs
    // 6. Assert: complete causal chain
}
```

---

## DELIVERABLES (IF AUTHORIZED)

### Code Structure

```
src/
  - execution_tools/
      - mod.rs (exports)
      - db.rs (ExecutionDb, ≤ 300 LOC)
      - record.rs (record_execution, ≤ 300 LOC)
      - query.rs (query_executions, ≤ 300 LOC)
      - graph.rs (graph integration, ≤ 300 LOC)
```

### Files Modified

- `src/lib.rs` - Add `pub mod execution_tools;`
- `Cargo.toml` - Add `uuid = "1.0"` dependency (for execution IDs)

### Test Files

- `tests/execution_tools_tests.rs` - Integration tests

### Documentation

- This document (`docs/PHASE_0_5_EXECUTION_MEMORY.md`)
- Updated `docs/TODO.md`

---

## ACCEPTANCE CRITERIA

Phase 0.5 complete when:

1. ✅ All execution recording modules implemented (≤ 300 LOC each)
2. ✅ All query modules implemented (≤ 300 LOC each)
3. ✅ Graph integration working (nodes + edges)
4. ✅ All tests passing with real SQLite + SQLiteGraph
5. ✅ Tools work standalone (no logging dependency)
6. ✅ Tools work with logging (wrapper pattern)
7. ✅ `cargo check` passes
8. ✅ `cargo test` passes
9. ✅ Temporal queries functional
10. ✅ Causal tracing functional

---

## WHAT COMES AFTER (CONTEXT ONLY)

### Future Phases (NOT AUTHORIZED, NOT PLANNED)

**Phase 0.6**: Pattern Queries (read-only evidence access)
- Query builder for anti-patterns
- Proven fix lookup
- Historical analysis tools

**Phase 0.7**: Ratatui UI (pure presentation)
- Display execution history
- Show causal chains
- Visualize graph relationships

**Phase 0.8**: Multi-language adapters
- C/C++ compilation logging
- Python tool integration
- Language-agnostic execution tracking

### Why Phase 0.5 is Keystone

Without execution memory:
- Phase 0.6 has nothing to query
- Phase 0.7 has nothing to display
- Phase 0.8 has no cross-language patterns

**Phase 0.5 enables everything above.**

---

## REQUIRED AUTHORIZATION

### Current State

**AWAITING**: "AUTHORIZE Phase 0.5 — Execution Memory (PLANNING ONLY)"

### What Happens After Authorization

1. ✅ This document becomes **active specification**
2. ✅ TODO.md updated with Phase 0.5 tasks
3. ✅ Implementation begins (OBSERVE → CONSTRAIN → DECIDE → ACT → VERIFY → REPORT)
4. ✅ TDD methodology enforced strictly

### What Happens Without Authorization

- System remains at Phase 0.4 (LSP Integration)
- No execution logging capability
- No historical pattern memory
- No evidence-based constraint enforcement

**This is correct state until explicitly authorized.**

---

## CRITICAL REMINDERS

### This Is NOT

❌ "AI memory"
❌ "Vector embeddings"
❌ "RAG system"
❌ "Learning algorithms"
❌ "Agent context"

### This IS

✅ **Audit logging** (like git reflog)
✅ **Causal tracking** (like git blame)
✅ **Evidence storage** (like database logs)
✅ **Pattern accumulation** (like compiler error databases)

**The difference**: This enables the LLM to query facts, not to "learn" in ML sense.

---

*Last Updated: 2025-12-24*
*Status: PHASE 0.5.1 — SCHEMA LOCKED*
*Phase: 0.5 — Execution Memory & Outcome Logging*

**SCHEMA STATUS**: FROZEN - CONTRACT LEVEL
**IMPLEMENTATION READY**: Awaiting Phase 0.5.2 authorization
