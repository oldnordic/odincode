# Phase 0.5.2 — Constraint Checklist & Test Matrix

**Status**: LOCKED — Implementation Contract
**Date**: 2025-12-24
**Phase**: 0.5.2 — Execution Memory Implementation (TDD-FIRST)
**Previous**: docs/ODINCODE_DB_ARCHITECTURE.md (DB root resolution)

---

## STEP 2 — CONSTRAIN: IMPLEMENTATION CHECKLIST

This section restates the FULL implementation constraints as a non-negotiable checklist derived from:
- `docs/PHASE_0_5_EXECUTION_MEMORY.md` (locked schema)
- `docs/ODINCODE_DB_ARCHITECTURE.md` (DB root ownership)

### A. FILESYSTEM CONSTRAINTS

- [ ] **A1**: `ExecutionDb::open(db_root: &Path)` accepts explicit directory path
- [ ] **A2**: `execution_log.db` path = `db_root.join("execution_log.db")`
- [ ] **A3**: `codegraph.db` path = `db_root.join("codegraph.db")`
- [ ] **A4**: If `codegraph.db` missing → return `Err(Error::CodegraphNotFound)`
- [ ] **A5**: If `execution_log.db` missing → auto-create with full schema
- [ ] **A6**: No hardcoded paths like `/home/feanor/Projects/` or `syncore_codegraph.db`
- [ ] **A7**: No environment variable dependence in execution_tools

### B. SQLITE (execution_log.db) CONSTRAINTS

**Schema (EXACT match to `docs/PHASE_0_5_EXECUTION_MEMORY.md`)**:
- [ ] **B1**: `executions` table with columns: `id TEXT PRIMARY KEY`, `tool_name TEXT`, `arguments_json TEXT`, `timestamp INTEGER`, `success BOOLEAN`, `exit_code INTEGER`, `duration_ms INTEGER`, `error_message TEXT`
- [ ] **B2**: `execution_artifacts` table with columns: `id INTEGER PRIMARY KEY AUTOINCREMENT`, `execution_id TEXT`, `artifact_type TEXT`, `content_json TEXT`
- [ ] **B3**: Indexes: `idx_executions_tool`, `idx_executions_timestamp`, `idx_executions_success`, `idx_executions_tool_timestamp`
- [ ] **B4**: Indexes: `idx_artifacts_execution`, `idx_artifacts_type`, `idx_artifacts_execution_type`

**Triggers (VALIDATION ENFORCEMENT)**:
- [ ] **B5**: `validate_tool_name` trigger → ABORT if tool_name not in allowed set
- [ ] **B6**: `validate_timestamp` trigger → ABORT if timestamp < 2020-01-01 or > now + 1 day
- [ ] **B7**: `validate_artifact_type` trigger → ABORT if artifact_type not in ('stdout', 'stderr', 'diagnostics')
- [ ] **B8**: `validate_json` trigger → ABORT if content_json not valid JSON

**Invariants**:
- [ ] **B9**: NO UPDATE operations on executions table (append-only)
- [ ] **B10**: NO DELETE operations on executions table (append-only)
- [ ] **B11**: NO UPDATE operations on execution_artifacts table (write-once)
- [ ] **B12**: NO DELETE operations on execution_artifacts table (write-once)

**Deterministic Querying**:
- [ ] **B13**: All SELECT queries include `ORDER BY` clause
- [ ] **B14**: Time-based queries use `ORDER BY timestamp ASC`
- [ ] **B15**: Tool-based queries use `ORDER BY timestamp ASC` secondary sort

### C. SQLITEGRAPH (codegraph.db) CONSTRAINTS

**Entity Requirements**:
- [ ] **C1**: Execution entity kind = `"execution"` (case-sensitive)
- [ ] **C2**: Execution entity name format = `"<tool_name>:<uuid>"` (e.g., "splice_patch:550e8400-...")
- [ ] **C3**: Execution entity `file_path` = `NULL`
- [ ] **C4**: Execution entity `data` JSON contains: `{"tool": "...", "timestamp": ..., "success": ..., "execution_id": "..."}`

**Edge Type Requirements**:
- [ ] **C5**: Allowed edge types ONLY: `EXECUTED_ON`, `AFFECTED`, `PRODUCED`, `REFERENCED`
- [ ] **C6**: Edge `data` JSON always contains `"execution_id"` field

**Required Edge Patterns (per tool type)**:
- [ ] **C7**: `file_read`, `file_write`, `file_create` → EXECUTED_ON edge to file entity
- [ ] **C8**: `splice_patch`, `splice_plan` → EXECUTED_ON edge to file(s)
- [ ] **C9**: `lsp_check` → EXECUTED_ON edge to project root file
- [ ] **C10**: `symbols_in_file`, `references_to_symbol_name` → REFERENCED edge to symbol entity(s)
- [ ] **C11**: `splice_patch` → REFERENCED edge to target symbol (if applicable)
- [ ] **C12**: `lsp_check` → PRODUCED edge to diagnostic entities (if errors present)
- [ ] **C13**: `file_write`, `file_create` → AFFECTED edge to file entity
- [ ] **C14**: `splice_patch` → AFFECTED edge to file in changed_files

**Forbidden Edges**:
- [ ] **C15**: NO `execution → execution` edges
- [ ] **C16**: NO `symbol → execution` edges (reverse only)
- [ ] **C17**: NO `diagnostic → execution` edges (reverse only)

**Write Ordering**:
- [ ] **C18**: SQLite writes (execution_log.db) happen BEFORE SQLiteGraph writes
- [ ] **C19**: SQLite COMMIT happens before opening SQLiteGraph transaction
- [ ] **C20**: Graph entity INSERT happens before graph edge INSERTs

### D. FAILURE SEMANTICS (NON-NEGOTIABLE)

**Write Sequence**:
- [ ] **D1**: Step 1 = `BEGIN TRANSACTION` (execution_log.db)
- [ ] **D2**: Step 2 = `INSERT INTO executions`
- [ ] **D3**: Step 3 = `INSERT INTO execution_artifacts` (if any)
- [ ] **D4**: Step 4 = `COMMIT` (execution_log.db)
- [ ] **D5**: Step 5 = `BEGIN TRANSACTION` (codegraph.db)
- [ ] **D6**: Step 6 = `INSERT INTO graph_entities` (execution node)
- [ ] **D7**: Step 7 = `INSERT INTO graph_edges` (all edges)
- [ ] **D8**: Step 8 = `COMMIT` (codegraph.db)

**Failure Cases**:
- [ ] **D9**: If Step 2-4 fails → NO data written to either DB (return error)
- [ ] **D10**: If Step 5-8 fails → execution_log.db data persists, graph missing (return error, log discrepancy)
- [ ] **D11**: If Step 6-7 partial failure → orphaned execution entity in graph (return error, log discrepancy)
- [ ] **D12**: If system crash → SQLite ACID rollback (both DBs unchanged)

**NO Recovery Actions**:
- [ ] **D13**: NO automatic retry on any failure
- [ ] **D14**: NO exponential backoff
- [ ] **D15**: NO cross-database transaction rollback
- [ ] **D16**: Caller decides retry strategy

### E. IMPLEMENTATION CONSTRAINTS

**Concurrency**:
- [ ] **E1**: NO async code
- [ ] **E2**: NO background threads
- [ ] **E3**: NO spawn/join operations

**Testing**:
- [ ] **E4**: Real SQLite databases (no in-memory mocks)
- [ ] **E5**: Real SQLiteGraph schema (no fake graph tables)
- [ ] **E6**: Tests create temp `db_root` via `tempfile::tempdir()`
- [ ] **E7**: Tests SKIP gracefully if codegraph.db initialization fails

**File Size**:
- [ ] **E8**: `src/execution_tools/mod.rs` ≤ 300 LOC
- [ ] **E9**: `src/execution_tools/db.rs` ≤ 300 LOC
- [ ] **E10**: `src/execution_tools/record.rs` ≤ 300 LOC
- [ ] **E11**: `src/execution_tools/query.rs` ≤ 300 LOC
- [ ] **E12**: `src/execution_tools/graph.rs` ≤ 300 LOC

**Dependencies**:
- [ ] **E13**: Add `uuid = "1.0"` to Cargo.toml
- [ ] **E14**: Use existing `rusqlite = "0.32"`
- [ ] **E15**: Use existing `anyhow = "1.0"`
- [ ] **E16**: Use existing `serde_json` (already in dependencies)

**Scope**:
- [ ] **E17**: NO logging framework required (tools work standalone)
- [ ] **E18**: NO embeddings, NO RAG, NO vector search
- [ ] **E19**: NO query language builder
- [ ] **E20**: NO aggregation or summary tables
- [ ] **E21**: NO lifecycle management (pruning, archival)
- [ ] **E22**: NO features beyond contract specification

---

## STEP 3 — DECIDE: TEST MATRIX DESIGN

This section defines the FAILING TESTS that prove contract compliance using REAL databases (no mocks).

### TEST A: SQLite Schema Creation + Trigger Enforcement

**Purpose**: Verify execution_log.db initialization

**Setup**:
```rust
let temp_dir = tempfile::tempdir()?;
let db_root = temp_dir.path();
create_minimal_codegraph_db(&db_root.join("codegraph.db"))?;
let exec_db = ExecutionDb::open(db_root)?;
```

**Assertions**:
- [ ] execution_log.db file exists at `db_root/execution_log.db`
- [ ] `executions` table exists (query `sqlite_master`)
- [ ] `execution_artifacts` table exists
- [ ] All indexes exist (query `sqlite_master`)
- [ ] All 4 triggers exist (`validate_tool_name`, `validate_timestamp`, `validate_artifact_type`, `validate_json`)

**Failure test**:
- [ ] Insert execution with invalid tool_name → trigger aborts
- [ ] Insert execution with future timestamp → trigger aborts
- [ ] Insert artifact with invalid type → trigger aborts
- [ ] Insert artifact with invalid JSON → trigger aborts

---

### TEST B: Record Execution Success

**Purpose**: Verify basic execution recording

**Setup**:
```rust
let execution_id = Uuid::new_v4().to_string();
let args = serde_json::json!({"file": "src/lib.rs"});
```

**Action**:
```rust
exec_db.record_execution(
    &execution_id,
    "file_read",
    &args,
    1735036800000,
    true,
    None,
    Some(150),
    None
)?;
```

**Assertions** (SQLite):
- [ ] `executions` table has exactly 1 row
- [ ] Row `id` equals execution_id
- [ ] Row `tool_name` equals "file_read"
- [ ] Row `arguments_json` equals args JSON
- [ ] Row `success` equals true
- [ ] Row `duration_ms` equals 150
- [ ] Row `exit_code` is NULL

**Assertions** (SQLiteGraph):
- [ ] `graph_entities` has 1 row with `kind = 'execution'`
- [ ] Entity `name` equals "file_read:<uuid>"
- [ ] Entity `data` JSON contains `execution_id`

**Deterministic query**:
- [ ] Query executions by tool_name → ORDER BY timestamp ASC enforced
- [ ] Query all executions → ORDER BY timestamp ASC enforced

---

### TEST C: Record Execution with Artifacts

**Purpose**: Verify stdout/stderr/diagnostics storage

**Setup**:
```rust
let execution_id = Uuid::new_v4().to_string();
let stdout = serde_json::json!({"text": "Patched src/lib.rs"});
let stderr = serde_json::json!({"text": ""});
let diagnostics = serde_json::json!([
    {"level": "error", "message": "E0425", "file_name": "lib.rs", "line_start": 10, "code": "E0425"}
]);
```

**Action**:
```rust
exec_db.record_execution_with_artifacts(
    &execution_id,
    "splice_patch",
    &args,
    timestamp,
    success,
    None,
    Some(500),
    None,
    &[("stdout", &stdout), ("stderr", &stderr), ("diagnostics", &diagnostics)]
)?;
```

**Assertions**:
- [ ] `execution_artifacts` has 3 rows for execution_id
- [ ] One row with `artifact_type = 'stdout'`, content matches
- [ ] One row with `artifact_type = 'stderr'`, content matches
- [ ] One row with `artifact_type = 'diagnostics'`, content matches
- [ ] All rows valid JSON (enforced by trigger)
- [ ] Query artifacts by execution_id → returns in deterministic order

---

### TEST D: Graph Write — Execution Entity + EXECUTED_ON Edge

**Purpose**: Verify SQLiteGraph integration

**Setup**:
```rust
// First, create a file entity in codegraph.db
let file_id = create_test_file_entity(&codegraph_conn, "src/lib.rs")?;

let execution_id = Uuid::new_v4().to_string();
exec_db.record_execution_on_file(
    &execution_id,
    "file_read",
    &args,
    timestamp,
    true,
    None,
    Some(50),
    None,
    "src/lib.rs"  // Target file for EXECUTED_ON edge
)?;
```

**Assertions** (SQLiteGraph):
- [ ] `graph_entities` has execution node with `kind = 'execution'`
- [ ] `graph_edges` has edge with `edge_type = 'EXECUTED_ON'`
- [ ] Edge `from_id` = execution entity ID
- [ ] Edge `to_id` = file entity ID
- [ ] Edge `data` JSON contains `"operation": "read"` and `"execution_id"`

---

### TEST E: Failure Semantics — Graph Write Failure After SQLite Commit

**Purpose**: Verify SQLite data persists when SQLiteGraph write fails

**Setup**:
```rust
// Create codegraph.db with INVALID schema (missing graph_edges table)
let codegraph_path = db_root.join("codegraph.db");
let conn = Connection::open(&codegraph_path)?;
conn.execute(
    "CREATE TABLE graph_entities (id INTEGER PRIMARY KEY, kind TEXT, name TEXT, file_path TEXT, data TEXT)",
    []
)?;
// DELIBERATELY SKIP graph_edges table → will cause INSERT failure
```

**Action**:
```rust
let result = exec_db.record_execution(...);
// Expect: Ok(()) for SQLite part, but internal error logged for graph failure
```

**Assertions**:
- [ ] Function returns `Ok(())` (SQLite success)
- [ ] `execution_log.db` has execution row (persisted despite graph failure)
- [ ] `codegraph.db` has NO execution entity (graph write failed)
- [ ] No panic, no crash (graceful degradation)

**Alternative test** (if graph write happens synchronously):
- [ ] Function returns `Err(Error::GraphWriteFailed)`
- [ ] `execution_log.db` still has execution row (SQLite committed before graph)
- [ ] `codegraph.db` has partial or no execution data

---

### TEST F: Deterministic Query Ordering

**Purpose**: Verify all queries return stable-sorted results

**Setup**:
```rust
// Record 5 executions with different timestamps (out of order)
exec_db.record_execution(..., timestamp_3, ...)?;
exec_db.record_execution(..., timestamp_1, ...)?;
exec_db.record_execution(..., timestamp_5, ...)?;
exec_db.record_execution(..., timestamp_2, ...)?;
exec_db.record_execution(..., timestamp_4, ...)?;
```

**Action**:
```rust
let executions = exec_db.query_by_tool("file_read")?;
```

**Assertions**:
- [ ] Results ordered by timestamp ASC (1, 2, 3, 4, 5)
- [ ] Order is deterministic across multiple queries
- [ ] No random ordering (no ORDER BY missing)

**Test edge cases**:
- [ ] Query with 0 results → empty Vec (not error)
- [ ] Query with 1 result → single-element Vec
- [ ] Query with 1000 results → all ordered correctly

---

### TEST G: Forbidden Edge Detection

**Purpose**: Verify invalid edges are rejected

**Setup**:
```rust
let execution_id = exec_db.record_execution(...)?;
let execution_entity_id = get_execution_entity_id(&codegraph_conn, &execution_id)?;
```

**Action** (attempt to create forbidden edge):
```rust
// Try to create execution → execution edge (FORBIDDEN)
let result = exec_db.create_graph_edge(
    execution_entity_id,
    execution_entity_id,  // Self-reference
    "EXECUTED_ON",
    &edge_data
);
```

**Assertions**:
- [ ] Function returns `Err(Error::ForbiddenEdgePattern)`
- [ ] No edge created in `graph_edges`
- [ ] Error message specifies which invariant violated

**Test other forbidden patterns**:
- [ ] symbol → execution edge → rejected
- [ ] diagnostic → execution edge → rejected

---

### TEST H: Full Workflow Integration

**Purpose**: End-to-end test of execution logging across tool families

**Setup**:
```rust
let temp_project = create_temp_cargo_project()?;
let db_root = tempfile::tempdir()?;
create_minimal_codegraph_db(&db_root.join("codegraph.db"))?;
let exec_db = ExecutionDb::open(&db_root)?;
```

**Actions**:
```rust
// 1. file_read
let result1 = file_read(&temp_project.path().join("Cargo.toml"))?;
exec_db.record_file_read(&result1, &start_time)?;

// 2. splice_patch
let result2 = splice_patch(&patch_args)?;
exec_db.record_splice_patch(&result2, &start_time)?;

// 3. lsp_check
let result3 = lsp_check(&temp_project.path())?;
exec_db.record_lsp_check(&result3, &start_time)?;
```

**Assertions**:
- [ ] 3 execution rows in `executions` table
- [ ] All 3 execution entities in `graph_entities`
- [ ] EXECUTED_ON edges: execution → Cargo.toml, execution → src/lib.rs, execution → project
- [ ] REFERENCED edge: splice_patch execution → target symbol
- [ ] PRODUCED edges: lsp_check execution → diagnostic entities (if errors)
- [ ] AFFECTED edges: splice_patch execution → modified files
- [ ] Chronological query returns executions in correct order

---

## TEST SUMMARY MATRIX

| Test | SQLite | Artifacts | Graph | Failure | Ordering | Integration |
|------|--------|-----------|-------|---------|----------|-------------|
| A | ✅ triggers | ❌ | ❌ | ❌ | ❌ | ❌ |
| B | ✅ | ❌ | ✅ entity | ❌ | ✅ | ❌ |
| C | ✅ | ✅ | ❌ | ❌ | ✅ | ❌ |
| D | ✅ | ❌ | ✅ edges | ❌ | ❌ | ❌ |
| E | ✅ persist | ❌ | ✅ fail | ✅ | ❌ | ❌ |
| F | ✅ | ❌ | ❌ | ❌ | ✅ | ❌ |
| G | ❌ | ❌ | ✅ forbid | ❌ | ❌ | ❌ |
| H | ✅ | ❌ | ✅ | ❌ | ✅ | ✅ |

**Total Tests**: 8
**Test Files to Create**: `tests/execution_tools_tests.rs`

---

## TEST-FIRST METHODOLOGY

### Implementation Order

1. **Write Test A** → Prove schema initialization fails (no execution_tools module)
2. **Implement db.rs** → Schema creation triggers
3. **Test A passes** ✅

4. **Write Test B** → Prove execution recording fails (no record_execution)
5. **Implement record.rs** → Basic execution insert
6. **Test B passes** ✅

7. **Write Test C** → Prove artifacts fail (no artifact insert)
8. **Extend record.rs** → Artifact insertion
9. **Test C passes** ✅

10. **Write Test D** → Prove graph writes fail (no graph.rs)
11. **Implement graph.rs** → Execution entity + edges
12. **Test D passes** ✅

13. **Write Test E** → Prove failure semantics not handled
14. **Extend record.rs + graph.rs** → Dual-write failure handling
15. **Test E passes** ✅

16. **Write Test F** → Prove queries don't exist (no query.rs)
17. **Implement query.rs** → Read operations with ORDER BY
18. **Test F passes** ✅

19. **Write Test G** → Prove forbidden edges not validated
20. **Extend graph.rs** → Edge validation
21. **Test G passes** ✅

22. **Write Test H** → Prove end-to-end workflow fails
23. **Integrate all modules** → Full recording pipeline
24. **Test H passes** ✅

### Skip Behavior

All tests MUST:
- Use `tempfile::tempdir()` for db_root
- Create minimal `codegraph.db` schema (no external dependencies)
- Skip gracefully if `codegraph.db` schema initialization fails
- NEVER panic on missing external tools

---

## ACCEPTANCE CRITERIA

Phase 0.5.2 complete when:

1. ✅ All 8 tests failing initially (no execution_tools module)
2. ✅ All 8 tests passing after implementation
3. ✅ All production files ≤ 300 LOC
4. ✅ `cargo check` passes
5. ✅ `cargo test` passes (32/32 tests: 27 existing + 8 new - 3 duplicated)
6. ✅ Real SQLite databases (no mocks)
7. ✅ Real SQLiteGraph schema (no fakes)
8. ✅ Deterministic ordering in all queries
9. ✅ Failure semantics verified (Test E)
10. ✅ Forbidden edges rejected (Test G)

---

## NEXT STEP

**Authorization Required**: Proceed to STEP 4 — ACT (TDD implementation)

**Action**:
1. Create `tests/execution_tools_tests.rs` with 8 failing tests
2. Prove all tests fail with `cargo test`
3. Begin implementation in `src/execution_tools/`

**Constraint**: NO production code until tests fail first.

---

*Last Updated: 2025-12-24*
*Status: LOCKED — Implementation Contract*
*Phase: 0.5.2 — Execution Memory Implementation*
*Methodology: TDD-First (Tests → Implementation)*
