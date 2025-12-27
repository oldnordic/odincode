# Phase 0.6 — Evidence Queries (PLANNING ONLY)

**Status**: PLANNING ONLY — NO CODE AUTHORIZED
**Type**: Read-Only Query Interface Specification
**Dependency**: Phase 0.5.2 (Execution Memory) COMPLETE
**Date**: 2025-12-24

---

## SECTION 1 — SCOPE & NON-GOALS (LOCKED)

### IN SCOPE (What Phase 0.6 WILL Do)

✅ **Read-only queries** over existing execution memory data
✅ **Deterministic SQL queries** with guaranteed ordering
✅ **Evidence retrieval** from execution_log.db and codegraph.db
✅ **Temporal filtering** (since/until timestamps)
✅ **Pattern aggregation** (grouped counts, recurring diagnostics)
✅ **Correlation** between executions and graph entities
✅ **Explicit "insufficient evidence" responses** when data is missing

### OUT OF SCOPE (What Phase 0.6 WILL NOT Do)

❌ **NO inference** — queries return facts, not "likely cause" or "probably"
❌ **NO scoring** — no ranking, no confidence scores, no probabilities
❌ **NO embeddings** — no vector search, no RAG, no semantic similarity
❌ **NO mutations** — no INSERT, UPDATE, DELETE, schema changes
❌ **NO triggers** — no database modifications of any kind
❌ **NO causal claims** — "temporal adjacency" only (X happened before Y)
❌ **NO machine learning** — no pattern recognition beyond COUNT/GROUP BY
❌ **NO UI layer** — console only, no Ratatui (Phase 0.7+)
❌ **NO code-as-data** — files are file paths only, no embeddings

### Core Principle

**EVIDENCE ONLY** — Phase 0.6 answers "what happened when" questions using persisted facts.

If insufficient evidence exists to answer a query, the query returns an explicit "insufficient evidence" indicator rather than guessing.

---

## SECTION 2 — DATA SOURCES (FACTS)

### execution_log.db Tables (OdinCode-Owned)

**Location**: `$ODINCODE_HOME/db/execution_log.db`
**Ownership**: Created and managed by OdinCode
**Status**: Read-only for Phase 0.6

#### Table: executions

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT | UUID v4 string (primary key) |
| tool_name | TEXT | Tool function name (whitelisted) |
| arguments_json | TEXT | Canonicalized JSON arguments |
| timestamp | INTEGER | Unix milliseconds since epoch |
| success | BOOLEAN | true = success, false = failure |
| exit_code | INTEGER | Process exit code (NULL if N/A) |
| duration_ms | INTEGER | Execution time in milliseconds (NULL if N/A) |
| error_message | TEXT | Human-readable error (NULL if success) |

**Indexes Used by Phase 0.6**:
- `idx_executions_tool` (tool_name)
- `idx_executions_timestamp` (timestamp)
- `idx_executions_success` (success)
- `idx_executions_tool_timestamp` (tool_name, timestamp)

#### Table: execution_artifacts

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Auto-increment primary key |
| execution_id | TEXT | Foreign key → executions.id |
| artifact_type | TEXT | "stdout", "stderr", or "diagnostics" |
| content_json | TEXT | JSON payload (structured output) |

**Indexes Used by Phase 0.6**:
- `idx_artifacts_execution` (execution_id)
- `idx_artifacts_type` (artifact_type)
- `idx_artifacts_execution_type` (execution_id, artifact_type)

### codegraph.db Tables (Magellan-Owned + OdinCode Execution Edges)

**Location**: `$ODINCODE_HOME/db/codegraph.db`
**Ownership**: Populated by Magellan; execution nodes/edges added by OdinCode
**Status**: Read-only for Phase 0.6

#### Table: graph_entities

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Auto-increment primary key |
| kind | TEXT | Entity kind (file, symbol, execution, diagnostic) |
| name | TEXT | Entity name |
| file_path | TEXT | File path (NULL for non-file entities) |
| data | TEXT | JSON metadata |

**Phase 0.6 Usage**:
- **Execution entities**: `kind = 'execution'`, `name = '<tool_name>:<uuid>'`
- **File entities**: `kind = 'file'`, `file_path = absolute path`
- **Diagnostic entities**: `kind = 'diagnostic'` (if created by lsp_check logging)

#### Table: graph_edges

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | Auto-increment primary key |
| from_id | INTEGER | Foreign key → graph_entities.id (source) |
| to_id | INTEGER | Foreign key → graph_entities.id (target) |
| edge_type | TEXT | Relationship type (EXECUTED_ON, AFFECTED, PRODUCED, REFERENCED) |
| data | TEXT | JSON payload |

**Phase 0.6 Usage**:
- **EXECUTED_ON**: execution → file/symbol (tool operated on target)
- **AFFECTED**: execution → file (tool modified target)
- **PRODUCED**: execution → diagnostic (tool generated error/warning)
- **REFERENCED**: execution → symbol (tool queried symbol)

### Cross-Database Correlation

**How to JOIN execution_log.db and codegraph.db**:

1. **Execution Entity Lookup**:
   ```sql
   -- In codegraph.db, find execution entity
   SELECT id, kind, name, data
   FROM graph_entities
   WHERE kind = 'execution'
     AND json_extract(data, '$.execution_id') = '<uuid>';
   ```

2. **Graph Traversal from Execution**:
   ```sql
   -- Find all files touched by an execution
   SELECT ge.id, ge.kind, ge.name, ge.file_path, edge_type
   FROM graph_edges e
   JOIN graph_entities ge ON e.to_id = ge.id
   WHERE e.from_id = <execution_entity_id>
     AND ge.kind IN ('file', 'symbol', 'diagnostic');
   ```

3. **Reverse Lookup (File → Executions)**:
   ```sql
   -- Find all executions that touched a file
   SELECT ge.id, ge.kind, ge.name, ge.data, e.edge_type
   FROM graph_edges e
   JOIN graph_entities ge ON e.from_id = ge.id
   WHERE e.to_id = <file_entity_id>
     AND ge.kind = 'execution';
   ```

**Critical Invariant**:
- Every `executions.id` (SQLite) MUST have corresponding `graph_entities` row with `kind='execution'`
- The `execution_id` is stored in both:
  - `executions.id` (SQLite, primary key)
  - `graph_entities.data` (SQLiteGraph, JSON field: `data.execution_id`)

**Best-Effort Dual-Write Gap**:
- If `graph_entities` lacks execution entity, fall back to SQLite-only queries
- Queries must remain deterministic even with partial graph data
- Return explicit "graph entity missing" indicator in results

---

## SECTION 3 — QUERY CATALOG (MINIMAL SET)

### Q1: ListExecutionsByTool

**Purpose**: Retrieve all executions of a specific tool, optionally filtered by time range

**Inputs**:
- `tool_name: &str` (required) — Exact tool name from whitelist
- `since: Option<i64>` — Unix timestamp ms (inclusive), None = beginning of time
- `until: Option<i64>` — Unix timestamp ms (exclusive), None = now
- `limit: Option<usize>` — Max results, None = no limit

**Output Schema**:
```rust
struct ExecutionSummary {
    execution_id: String,  // UUID
    tool_name: String,
    timestamp: i64,        // Unix ms
    success: bool,
    exit_code: Option<i32>,
    duration_ms: Option<i64>,
    error_message: Option<String>,
}
```

**Deterministic Ordering**:
```sql
ORDER BY timestamp ASC, id ASC
```

**SQL (execution_log.db)**:
```sql
SELECT
    id,
    tool_name,
    timestamp,
    success,
    exit_code,
    duration_ms,
    error_message
FROM executions
WHERE tool_name = ?1
  AND (?2 IS NULL OR timestamp >= ?2)
  AND (?3 IS NULL OR timestamp < ?3)
ORDER BY timestamp ASC, id ASC
LIMIT ?4;
```

**Evidence Guarantees**:
- ✅ Returns all executions matching criteria (if limit permits)
- ✅ Empty result = no executions recorded for this tool
- ⚠️ Does NOT verify graph entity existence (use Q5 for full details)

**Limitations**:
- ❌ Does not include artifacts (use Q5 for full execution with artifacts)
- ❌ Does not include graph relationships (use Q5 for graph edges)

---

### Q2: ListFailuresByTool

**Purpose**: Retrieve only failed executions of a specific tool

**Inputs**:
- `tool_name: &str` (required)
- `since: Option<i64>` — Unix timestamp ms
- `limit: Option<usize>` — Max results

**Output Schema**:
```rust
struct FailureSummary {
    execution_id: String,
    tool_name: String,
    timestamp: i64,
    exit_code: Option<i32>,
    error_message: Option<String>,
}
```

**Deterministic Ordering**:
```sql
ORDER BY timestamp DESC, id DESC
```

**SQL (execution_log.db)**:
```sql
SELECT
    id,
    tool_name,
    timestamp,
    exit_code,
    error_message
FROM executions
WHERE tool_name = ?1
  AND success = 0
  AND (?2 IS NULL OR timestamp >= ?2)
ORDER BY timestamp DESC, id DESC
LIMIT ?3;
```

**Evidence Guarantees**:
- ✅ Returns most recent failures first (DESC order)
- ✅ `success = 0` filter enforced at database level
- ⚠️ `exit_code = NULL` possible for tools without process exit codes

**Limitations**:
- ❌ Does NOT return stdout/stderr (use Q5 for artifacts)
- ❌ Does NOT indicate whether fix was attempted (temporal query only)

---

### Q3: FindExecutionsByDiagnosticCode

**Purpose**: Find all executions that produced a specific diagnostic code (e.g., "E0425")

**Inputs**:
- `code: &str` (required) — Exact diagnostic code (e.g., "E0425")
- `limit: Option<usize>` — Max results

**Output Schema**:
```rust
struct DiagnosticExecution {
    execution_id: String,
    tool_name: String,
    timestamp: i64,
    diagnostic_code: String,
    diagnostic_level: String,  // "error" | "warning"
    diagnostic_message: String,
    file_name: String,
}
```

**Deterministic Ordering**:
```sql
ORDER BY e.timestamp ASC, e.id ASC
```

**SQL (execution_log.db)**:
```sql
SELECT DISTINCT
    e.id AS execution_id,
    e.tool_name,
    e.timestamp,
    json_extract(a.content_json, '$.code') AS diagnostic_code,
    json_extract(a.content_json, '$.level') AS diagnostic_level,
    json_extract(a.content_json, '$.message') AS diagnostic_message,
    json_extract(a.content_json, '$.file_name') AS file_name
FROM executions e
JOIN execution_artifacts a ON e.id = a.execution_id
WHERE a.artifact_type = 'diagnostics'
  AND json_extract(a.content_json, '$.code') = ?1
ORDER BY e.timestamp ASC, e.id ASC
LIMIT ?2;
```

**Evidence Guarantees**:
- ✅ Returns executions with diagnostics matching exact code
- ⚠️ Requires `execution_artifacts` rows with `artifact_type='diagnostics'`
- ⚠️ Relies on JSON extraction (malformed JSON = row skipped)

**Limitations**:
- ❌ Does NOT verify diagnostic was resolved (temporal query only)
- ❌ Does NOT prove causality (only shows diagnostic occurred)

---

### Q4: FindExecutionsByFile

**Purpose**: Find all executions that operated on a specific file path

**Inputs**:
- `file_path: &str` (required) — Exact absolute file path
- `since: Option<i64>` — Unix timestamp ms
- `limit: Option<usize>` — Max results

**Output Schema**:
```rust
struct FileExecution {
    execution_id: String,
    tool_name: String,
    timestamp: i64,
    success: bool,
    edge_type: String,  // EXECUTED_ON | AFFECTED | REFERENCED
}
```

**Deterministic Ordering**:
```sql
ORDER BY e.timestamp DESC, e.id DESC
```

**SQL (codegraph.db)**:
```sql
SELECT DISTINCT
    json_extract(ge_exec.data, '$.execution_id') AS execution_id,
    json_extract(ge_exec.data, '$.tool') AS tool_name,
    json_extract(ge_exec.data, '$.timestamp') AS timestamp,
    json_extract(ge_exec.data, '$.success') AS success,
    edge_type
FROM graph_entities ge_exec
JOIN graph_edges e ON ge_exec.id = e.from_id
JOIN graph_entities ge_file ON e.to_id = ge_file.id
WHERE ge_exec.kind = 'execution'
  AND ge_file.kind = 'file'
  AND ge_file.file_path = ?1
  AND (?2 IS NULL OR json_extract(ge_exec.data, '$.timestamp') >= ?2)
ORDER BY json_extract(ge_exec.data, '$.timestamp') DESC,
         ge_exec.id DESC
LIMIT ?3;
```

**Fallback SQL (execution_log.db if graph missing)**:
```sql
SELECT
    id AS execution_id,
    tool_name,
    timestamp,
    success,
    'UNKNOWN' AS edge_type
FROM executions
WHERE tool_name IN ('file_read', 'file_write', 'file_create', 'splice_patch', 'splice_plan', 'lsp_check')
  AND json_extract(arguments_json, '$.file') = ?1
ORDER BY timestamp DESC, id DESC
LIMIT ?2;
```

**Evidence Guarantees**:
- ✅ Uses graph relationships if available (more accurate)
- ⚠️ Falls back to arguments_json parsing if graph entities missing
- ⚠️ Fallback may miss executions (e.g., splice_patch with multiple files)

**Limitations**:
- ❌ Graph-based query requires execution entities exist (best-effort dual-write gap)
- ❌ Fallback query is approximate (argument structure may vary)

---

### Q5: GetExecutionDetails

**Purpose**: Retrieve complete execution record including all artifacts and graph edges

**Inputs**:
- `execution_id: &str` (required) — UUID of execution

**Output Schema**:
```rust
struct ExecutionDetails {
    execution: ExecutionRecord,
    artifacts: Vec<ArtifactRecord>,
    graph_entity: Option<GraphEntityRecord>,
    graph_edges: Vec<GraphEdgeRecord>,
}

struct ExecutionRecord {
    id: String,
    tool_name: String,
    arguments_json: String,
    timestamp: i64,
    success: bool,
    exit_code: Option<i32>,
    duration_ms: Option<i64>,
    error_message: Option<String>,
}

struct ArtifactRecord {
    artifact_type: String,  // stdout | stderr | diagnostics
    content_json: String,
}

struct GraphEntityRecord {
    entity_id: i64,
    kind: String,
    name: String,
    file_path: Option<String>,
    data: String,
}

struct GraphEdgeRecord {
    edge_id: i64,
    edge_type: String,
    target_entity_id: i64,
    target_kind: String,
    target_name: String,
}
```

**Deterministic Ordering**:
- `artifacts`: ORDER BY artifact_type ASC
- `graph_edges`: ORDER BY edge_type ASC, target_entity_id ASC

**SQL (execution_log.db)**:
```sql
-- Execution record
SELECT id, tool_name, arguments_json, timestamp, success,
       exit_code, duration_ms, error_message
FROM executions
WHERE id = ?1;

-- Artifacts
SELECT artifact_type, content_json
FROM execution_artifacts
WHERE execution_id = ?1
ORDER BY artifact_type ASC;
```

**SQL (codegraph.db)**:
```sql
-- Execution entity
SELECT id, kind, name, file_path, data
FROM graph_entities
WHERE kind = 'execution'
  AND json_extract(data, '$.execution_id') = ?1;

-- Graph edges
SELECT
    e.id AS edge_id,
    e.edge_type,
    e.to_id AS target_entity_id,
    ge.kind AS target_kind,
    ge.name AS target_name
FROM graph_edges e
JOIN graph_entities ge ON e.to_id = ge.id
WHERE e.from_id = (SELECT id FROM graph_entities
                   WHERE kind = 'execution'
                     AND json_extract(data, '$.execution_id') = ?1)
ORDER BY e.edge_type ASC, e.to_id ASC;
```

**Evidence Guarantees**:
- ✅ Returns complete execution record if exists
- ✅ Returns empty artifacts list if none recorded
- ⚠️ `graph_entity = None` if dual-write failed (best-effort gap)
- ⚠️ `graph_edges = []` if graph missing or execution entity missing

**Failure Modes**:
- ❌ Returns `Err(Error::ExecutionNotFound)` if execution_id not in SQLite
- ⚠️ Returns `Ok(graph_entity: None)` if SQLite exists but graph entity missing

---

### Q6: GetLatestOutcomeForFile

**Purpose**: Get the most recent execution outcome for a specific file

**Inputs**:
- `file_path: &str` (required) — Exact absolute file path

**Output Schema**:
```rust
struct LatestFileOutcome {
    execution_id: String,
    tool_name: String,
    timestamp: i64,
    success: bool,
    edge_type: String,  // EXECUTED_ON | AFFECTED
}
```

**Deterministic Ordering**:
```sql
ORDER BY timestamp DESC, id DESC
LIMIT 1
```

**SQL (codegraph.db)**:
```sql
SELECT
    json_extract(ge_exec.data, '$.execution_id') AS execution_id,
    json_extract(ge_exec.data, '$.tool') AS tool_name,
    json_extract(ge_exec.data, '$.timestamp') AS timestamp,
    json_extract(ge_exec.data, '$.success') AS success,
    edge_type
FROM graph_entities ge_exec
JOIN graph_edges e ON ge_exec.id = e.from_id
JOIN graph_entities ge_file ON e.to_id = ge_file.id
WHERE ge_exec.kind = 'execution'
  AND ge_file.kind = 'file'
  AND ge_file.file_path = ?1
  AND e.edge_type IN ('EXECUTED_ON', 'AFFECTED')
ORDER BY json_extract(ge_exec.data, '$.timestamp') DESC
LIMIT 1;
```

**Evidence Guarantees**:
- ✅ Returns most recent execution (DESC timestamp)
- ⚠️ Returns NULL if no executions found for file
- ⚠️ Returns "UNKNOWN" edge_type if graph missing (fallback query)

**Fallback SQL (execution_log.db)**:
```sql
SELECT id, tool_name, timestamp, success
FROM executions
WHERE tool_name IN ('file_write', 'splice_patch')
  AND json_extract(arguments_json, '$.file') = ?1
ORDER BY timestamp DESC
LIMIT 1;
```

---

### Q7: GetRecurringDiagnostics

**Purpose**: Find diagnostics that occur repeatedly (grouped by code and file)

**Inputs**:
- `threshold: usize` (required) — Minimum occurrence count (e.g., 3)
- `since: Option<i64>` — Unix timestamp ms

**Output Schema**:
```rust
struct RecurringDiagnostic {
    diagnostic_code: String,
    file_name: String,
    occurrence_count: i64,
    first_seen: i64,
    last_seen: i64,
    execution_ids: Vec<String>,  // All execution IDs with this diagnostic
}
```

**Deterministic Ordering**:
```sql
ORDER BY occurrence_count DESC, diagnostic_code ASC, file_name ASC
```

**SQL (execution_log.db)**:
```sql
SELECT
    json_extract(a.content_json, '$.code') AS diagnostic_code,
    json_extract(a.content_json, '$.file_name') AS file_name,
    COUNT(DISTINCT e.id) AS occurrence_count,
    MIN(e.timestamp) AS first_seen,
    MAX(e.timestamp) AS last_seen,
    GROUP_CONCAT(e.id, ',') AS execution_ids
FROM executions e
JOIN execution_artifacts a ON e.id = a.execution_id
WHERE a.artifact_type = 'diagnostics'
  AND (?2 IS NULL OR e.timestamp >= ?2)
GROUP BY diagnostic_code, file_name
HAVING occurrence_count >= ?1
ORDER BY occurrence_count DESC, diagnostic_code ASC, file_name ASC;
```

**Evidence Guarantees**:
- ✅ Returns diagnostics exceeding threshold
- ✅ `execution_ids` comma-separated string (parse in application code)
- ⚠️ Excludes diagnostics with `occurrence_count < threshold`
- ⚠️ Relies on `execution_artifacts` rows existing

**Limitations**:
- ❌ Does NOT prove same error (same code may occur in different contexts)
- ❌ Does NOT indicate whether resolved (temporal query only)

---

### Q8: FindPriorFixesForDiagnostic

**Purpose**: Find mutation executions (splice/file_write) that occurred AFTER diagnostic occurrences

**Inputs**:
- `code: &str` (required) — Diagnostic code (e.g., "E0425")
- `file_path: Option<&str>` — Filter to specific file (None = all files)
- `since: Option<i64>` — Lookback window start

**Output Schema**:
```rust
struct PriorFix {
    execution_id: String,
    tool_name: String,        // splice_patch | file_write
    timestamp: i64,
    diagnostic_execution_id: String,  // Preceding diagnostic
    temporal_gap_ms: i64,     // Time between diagnostic and fix attempt
    success: bool,
}
```

**Deterministic Ordering**:
```sql
ORDER BY diagnostic_timestamp ASC, fix_timestamp ASC
```

**SQL (execution_log.db)**:
```sql
WITH diagnostics AS (
    SELECT
        id AS diagnostic_execution_id,
        timestamp AS diagnostic_timestamp,
        json_extract(content_json, '$.file_name') AS file_name
    FROM executions e
    JOIN execution_artifacts a ON e.id = a.execution_id
    WHERE a.artifact_type = 'diagnostics'
      AND json_extract(a.content_json, '$.code') = ?1
      AND (?3 IS NULL OR e.timestamp >= ?3)
),
fixes AS (
    SELECT
        id AS fix_execution_id,
        timestamp AS fix_timestamp,
        tool_name,
        success
    FROM executions
    WHERE tool_name IN ('splice_patch', 'file_write')
      AND (?3 IS NULL OR timestamp >= ?3)
)
SELECT
    f.fix_execution_id AS execution_id,
    f.tool_name,
    f.fix_timestamp AS timestamp,
    d.diagnostic_execution_id,
    (f.fix_timestamp - d.diagnostic_timestamp) AS temporal_gap_ms,
    f.success
FROM diagnostics d
JOIN fixes f ON f.fix_timestamp > d.diagnostic_timestamp
WHERE (?2 IS NULL OR d.file_name = ?2)
ORDER BY d.diagnostic_timestamp ASC, f.fix_timestamp ASC;
```

**Evidence Guarantees**:
- ✅ Returns all mutation executions after each diagnostic occurrence
- ✅ `temporal_gap_ms` positive (fix AFTER diagnostic)
- ⚠️ Does NOT prove causality (temporal adjacency only)
- ⚠️ May include unrelated edits (false positives)

**Limitations**:
- ❌ "Fix" is assumption — query only proves temporal adjacency
- ❌ Cannot verify fix resolved the diagnostic (temporal gap)
- ❌ May include edits to different parts of same file

**Evidence Language Rule**:
- Document must say: "execution X occurred after diagnostic Y"
- NOT: "execution X fixed diagnostic Y"
- NOT: "execution X likely caused diagnostic Y"

---

## SECTION 4 — ORDERING & STABILITY RULES (MANDATORY)

### Canonical Ordering Principles

**P1: Time-Based Ordering** (Primary)
- Most queries use `timestamp ASC` (chronological) or `timestamp DESC` (reverse chronological)
- Tie-breaker: `id ASC` (SQLite) or entity `id ASC` (SQLiteGraph)

**P2: Lexicographic Tie-Breakers** (Secondary)
- After timestamp: sort by string fields (diagnostic_code, file_name, tool_name)
- After strings: sort by numeric IDs (execution_id, entity_id)

**P3: Deterministic JSON Comparisons**

When comparing `arguments_json` (e.g., for duplicate detection):
1. Parse JSON to canonical form
2. Sort object keys alphabetically
3. Remove whitespace
4. Compare canonicalized strings

**Example**:
```json
// Original (non-canonical)
{"file": "src/lib.rs", "symbol": "foo"}
{"symbol": "foo", "file": "src/lib.rs"}

// Canonicalized (identical)
{"file":"src/lib.rs","symbol":"foo"}
{"file":"src/lib.rs","symbol":"foo"}
```

### Query-Specific Ordering

| Query | Primary Sort | Secondary Sort | Tertiary Sort |
|-------|-------------|----------------|---------------|
| Q1 (ListExecutionsByTool) | timestamp ASC | id ASC | — |
| Q2 (ListFailuresByTool) | timestamp DESC | id DESC | — |
| Q3 (FindExecutionsByDiagnosticCode) | timestamp ASC | id ASC | — |
| Q4 (FindExecutionsByFile) | timestamp DESC | id DESC | — |
| Q5 (GetExecutionDetails artifacts) | artifact_type ASC | — | — |
| Q5 (GetExecutionDetails edges) | edge_type ASC | target_entity_id ASC | — |
| Q6 (GetLatestOutcomeForFile) | timestamp DESC | — | — |
| Q7 (GetRecurringDiagnostics) | occurrence_count DESC | diagnostic_code ASC | file_name ASC |
| Q8 (FindPriorFixesForDiagnostic) | diagnostic_timestamp ASC | fix_timestamp ASC | — |

### Stability Guarantees

**S1: Same Input → Same Order**
- Running query twice with same inputs produces identical order
- Ordering depends only on persisted data (no runtime nondeterminism)

**S2: Pagination Support**
- Queries with `limit` can be paginated by resuming from last row
- Next page: `WHERE (timestamp, id) > (?, ?) ORDER BY timestamp ASC, id ASC`

**S3: Cross-Run Consistency**
- Order is stable across DB restarts
- Order is stable across application restarts
- Order is stable across different machines (same DB copy)

---

## SECTION 5 — FAILURE MODES & SKIP RULES

### F1: codegraph.db Missing

**Condition**: `db_root/codegraph.db` does not exist

**Behavior**:
- Return `Err(Error::CodegraphNotFound)` for graph-only queries (Q4, Q5 graph parts, Q6, Q8)
- Allow SQLite-only fallback for Q1, Q2, Q3, Q5 (SQLite parts), Q7
- Do NOT auto-create `codegraph.db` (Phase 0.6 is read-only)

**Evidence Response**:
- Return explicit error: "Codegraph database not found at <path>"
- Do NOT fall back to partial results without indication

### F2: Graph Lacks Execution Entities (Best-Effort Dual-Write Gap)

**Condition**: `executions` table has rows, but `graph_entities` has no `kind='execution'` rows

**Behavior**:
- **Q1, Q2, Q3, Q7**: Work normally (SQLite-only queries)
- **Q4, Q6**: Use fallback queries (parse `arguments_json`)
- **Q5**: Return `Ok(graph_entity: None, graph_edges: [])`
- **Q8**: SQLite-only (no graph edges needed)

**Evidence Response**:
- Q5: Include `graph_entity_available: bool` flag in output
- Q4, Q6: Include `data_source: "graph" | "fallback"` indicator

### F3: Diagnostics Artifacts Missing or Malformed

**Condition**: `execution_artifacts` rows with `artifact_type='diagnostics'` missing or invalid JSON

**Behavior**:
- **Q3**: Skip rows with `json_valid(content_json) = 0`
- **Q7**: Exclude malformed rows from aggregation
- **Q8**: Join produces no matches if diagnostics missing

**Evidence Response**:
- Return empty result set (not error) for Q3, Q7, Q8
- Log warning to internal error log (not user-visible)

### F4: Insufficient Evidence

**Condition**: Query executes successfully but returns zero rows

**Behavior**:
- Return empty vector (not error)
- Include `evidence_sufficient: true` flag (data exists but matches no criteria)

**Evidence Response**:
- Document: "No executions matching criteria found"
- NOT: "Insufficient data" (data exists, just no matches)

**Contrast with**:
- **True insufficient evidence**: Required indexes missing, tables missing → explicit error
- **Zero matches**: Query executed, no data → empty result (success)

### F5: Timestamp Range Queries with No Data

**Condition**: `since`/`until` parameters exclude all recorded data

**Behavior**:
- Return empty result set (not error)
- Do NOT suggest widening time range (no inference)

**Evidence Response**:
- "No executions found in time range [<since>, <until>)"
- Not: "Try earlier date" (no recommendations)

---

## SECTION 6 — ACCEPTANCE CRITERIA (FOR FUTURE IMPLEMENTATION)

### A1: Read-Only SQL Compliance

Each query MUST be implementable with:
- ✅ `SELECT` statements only
- ✅ `JOIN` operations (SQLite only, no cross-database joins)
- ✅ `GROUP BY` / `HAVING` for aggregations
- ✅ `json_extract()` for JSON field access
- ❌ NO `INSERT`, `UPDATE`, `DELETE`, `CREATE`, `DROP`, `ALTER`
- ❌ NO trigger creation
- ❌ NO schema modifications

### A2: Deterministic Results

Each query MUST satisfy:
- ✅ `ORDER BY` clause with deterministic tie-breakers
- ✅ Same inputs → same order across multiple runs
- ✅ Pagination support (stable ordering with `limit`/`offset`)

### A3: Explicit Evidence Signals

Each query MUST return:
- ✅ Empty result (not error) when no data matches criteria
- ✅ Explicit error when required databases/tables missing
- ✅ Flags indicating data source (graph vs. fallback)
- ✅ No "insufficient evidence" guessing (return what exists)

### A4: No Schema Changes

Implementation MUST NOT require:
- ❌ New tables
- ❌ New indexes (use existing indexes from Phase 0.5.2)
- ❌ New columns
- ❌ Trigger modifications
- ❌ Constraint additions

### A5: Best-Effort Graceful Degradation

When graph missing or incomplete:
- ✅ Fall back to SQLite-only queries where possible
- ✅ Return explicit indicators of data source
- ✅ Return partial results rather than failing
- ❌ Do NOT hide fallback condition from caller

### A6: Zero Inference Enforcement

Query results MUST NOT include:
- ❌ "likely cause" language
- ❌ "probably fixed" claims
- ❌ Confidence scores
- ❌ Probability rankings
- ❌ Causal assertions (temporal adjacency only)

**Allowed Evidence Language**:
- ✅ "execution X occurred at timestamp T"
- ✅ "diagnostic D occurred before fix attempt F"
- ✅ "file F was touched by 3 executions"
- ✅ "diagnostic code E0425 occurred 7 times"

**Forbidden Language**:
- ❌ "execution X fixed diagnostic D"
- ❌ "file F is high-risk"
- ❌ "this approach usually works"
- ❌ "likely caused by"

---

## IMPLEMENTATION NOTES (FUTURE REFERENCE)

### Module Structure (IF AUTHORIZED)

```
src/execution_tools/
├── query.rs          (Existing: 44 LOC)
├── evidence_queries.rs  (NEW: ≤ 300 LOC)
│   ├── Q1_ListExecutionsByTool
│   ├── Q2_ListFailuresByTool
│   ├── Q3_FindExecutionsByDiagnosticCode
│   ├── Q4_FindExecutionsByFile
│   ├── Q5_GetExecutionDetails
│   ├── Q6_GetLatestOutcomeForFile
│   ├── Q7_GetRecurringDiagnostics
│   └── Q8_FindPriorFixesForDiagnostic
└── mod.rs            (Update exports)
```

### Test Structure (IF AUTHORIZED)

```
tests/evidence_queries_tests.rs
├── test_q1_list_executions_by_tool
├── test_q2_list_failures_by_tool
├── test_q3_find_executions_by_diagnostic_code
├── test_q4_find_executions_by_file
├── test_q4_falls_back_to_sqlite_when_graph_missing
├── test_q5_get_execution_details_with_graph
├── test_q5_get_execution_details_without_graph
├── test_q6_get_latest_outcome_for_file
├── test_q7_get_recurring_diagnostics
├── test_q8_find_prior_fixes_for_diagnostic
└── test_deterministic_ordering_across_runs
```

### Dependencies (IF AUTHORIZED)

- No new dependencies (use existing `rusqlite`, `thiserror`, `uuid`)

---

## NON-GOALS REITERATION

### What Phase 0.6 Will NOT Do

❌ **NO UI layer** — Query API only, no Ratatui
❌ **NO CLI wrapper** — Library interface only
❌ **NO caching** — Direct DB queries only
❌ **NO query language** — Rust function calls only
❌ **NO async** — Synchronous queries only
❌ **NO connection pooling** — One connection per query
❌ **NO migrations** — Use Phase 0.5.2 schema as-is

---

## TERMINATION

**Phase**: 0.6 — Evidence Queries (PLANNING ONLY)
**Status**: PLANNING COMPLETE — Awaiting Acceptance or Revisions
**Date**: 2025-12-24

**DELIVERABLE**: This document (`docs/PHASE_0_6_EVIDENCE_QUERIES.md`)

**NEXT STEP** (IF AUTHORIZED):
- User reviews and approves/plans changes
- Then: "AUTHORIZE Phase 0.6 — Evidence Queries (IMPLEMENTATION)"
- Then: TDD implementation begins

**IF REJECTED**:
- User specifies changes required
- Revise document per feedback
- Resubmit for acceptance

---

*Last Updated: 2025-12-24*
*Status: PLANNING ONLY — NO CODE AUTHORIZED*
*Phase: 0.6 — Evidence Queries*
*Purpose: Read-only query interface over execution memory*
