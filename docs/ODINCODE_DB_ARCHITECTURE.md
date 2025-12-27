# OdinCode Database Architecture — Authoritative Resolution

**Status**: LOCKED — Phase 0.5.2 Implementation Contract
**Date**: 2025-12-24
**Purpose**: Resolve SQLiteGraph DB location ambiguity with clean separation of concerns

---

## PROBLEM STATEMENT

### Initial Ambiguity

Phase 0.5 schema documented:
- `execution_log.db` location: "Same directory as `codegraph.db`"
- BUT: Did not specify WHERE that directory is
- AND: Did not clarify if `codegraph.db` is owned by OdinCode or external

### Incorrect Assumption to Avoid

❌ **WRONG**: Assume `codegraph.db` or `syncore_codegraph.db` exists at hardcoded paths

**Why wrong**:
- `magellan_tools` opens existing DBs in read-only mode
- Those DBs are created by Magellan, not OdinCode
- Past usage (SynCore) used `syncore_codegraph.db`, but that's not a contract for OdinCode
- Hardcoding paths creates environment coupling

---

## AUTHORITATIVE DESIGN DECISION (LOCKED)

### OdinCode Database Root

**At runtime**, OdinCode owns one DB root directory:

```
$ODINCODE_HOME/db/
```

**Inside it**:
```
db/
├── execution_log.db        # OWNED by OdinCode (Phase 0.5)
├── codegraph.db            # PROVIDED by Magellan (read/write for Phase 0.5)
└── (future)
    ├── vectors.db
    ├── mvcc.db
    └── ...
```

**Ownership Rules**:
1. **OdinCode owns the directory structure**
2. **Magellan populates `codegraph.db`** (external indexer)
3. **Execution memory writes to both** (`execution_log.db` + `codegraph.db` graph entities)
4. **Queries may JOIN logically, never physically** (separate SQLite connections)

---

## IMPLEMENTATION CONTRACT (PHASE 0.5.2)

### What Execution Tools MUST Accept

**API Surface**:
```rust
impl ExecutionDb {
    /// Open execution memory at given DB root
    ///
    /// # Arguments
    /// * `db_root` - Directory containing execution_log.db and codegraph.db
    ///
    /// # Expected Files
    /// * `db_root/execution_log.db` - Created if missing (owned by OdinCode)
    /// * `db_root/codegraph.db` - Must exist or returns Error::DbNotFound
    pub fn open(db_root: &Path) -> Result<Self>;
}
```

### What Execution Tools MUST EXPECT

**File 1: `execution_log.db`** (OdinCode-owned)
- **Created automatically** if missing
- **Full schema initialization** (executions, execution_artifacts tables + triggers)
- **Writable**: `INSERT` operations for execution logging
- **Never deleted**: Append-only log

**File 2: `codegraph.db`** (Magellan-provided)
- **Must exist**: If missing, `ExecutionDb::open()` returns `Error::CodegraphNotFound`
- **Read-write access**: Phase 0.5 requires INSERT to graph_entities and graph_edges
- **Minimal schema**: Must have `graph_entities` and `graph_edges` tables
- **No schema creation**: Execution tools DO NOT initialize codegraph.db

---

## INVARIANTS (NON-NEGOTIABLE)

### D1: Database Root Unification
- All DB operations are relative to a single `db_root: &Path`
- No hardcoded absolute paths
- No environment variable dependence in execution_tools

### D2: Execution Log Auto-Creation
- If `db_root/execution_log.db` is missing, create it with full schema
- Schema versioning tracked within execution_log.db (future-proofing)
- Triggers and indexes created on initialization

### D3: Codegraph Dependency Check
- If `db_root/codegraph.db` is missing, return `Error::CodegraphNotFound`
- Tests use this error to SKIP gracefully
- Runtime operations FAIL explicitly (not silent) if codegraph.db missing

### D4: Connection Separation
- `execution_log.db`: Separate SQLite connection
- `codegraph.db`: Separate SQLite connection
- No ATTACH DATABASE (cross-database queries via application-layer joins only)

---

## TEST BEHAVIOR (DETERMINISTIC & CLEAN)

### What Tests MUST Do

**Setup**:
```rust
#[test]
fn test_record_execution_success() {
    // 1. Create temporary directory
    let temp_dir = tempfile::tempdir()?;
    let db_root = temp_dir.path();

    // 2. Create minimal codegraph.db
    let codegraph_path = db_root.join("codegraph.db");
    create_minimal_codegraph_db(&codegraph_path)?;

    // 3. Open ExecutionDb (auto-creates execution_log.db)
    let exec_db = ExecutionDb::open(db_root)?;

    // 4. Test execution recording
    // ...
}
```

**Minimal `codegraph.db` Schema** (for tests only):
```sql
-- Copied from SQLiteGraph, NOT mocked
CREATE TABLE graph_entities (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    file_path TEXT,
    data TEXT NOT NULL
);

CREATE TABLE graph_edges (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id INTEGER NOT NULL,
    to_id INTEGER NOT NULL,
    edge_type TEXT NOT NULL,
    data TEXT NOT NULL
);
```

### What Tests MUST NOT Do

- ❌ Assume `/tmp/codegraph.db` exists
- ❌ Assume user environment has Magellan-created DBs
- ❌ Mock `codegraph.db` schema (use real SQLite)
- ❌ Hardcode paths like `/home/feanor/Projects/syncore/syncore_codegraph.db`

---

## SEPARATION OF CONCERNS (WHY THIS DESIGN)

### OdinCode Responsibilities

**Owns**:
- `db_root/` directory structure
- `execution_log.db` (full lifecycle: create, write, query)
- Graph writes to `codegraph.db` (execution entities, edges)

**Does NOT own**:
- `codegraph.db` file creation
- File/symbol indexing (Magellan's job)
- Schema migration of `codegraph.db`

### Magellan Responsibilities

**Owns**:
- File watching and parsing
- Symbol/reference extraction
- `codegraph.db` creation and population

**Does NOT own**:
- `execution_log.db` (OdinCode's audit log)
- Execution entity insertion (OdinCode's responsibility)

### Clean Boundary

**Interface**: Both write to `codegraph.db`, but:
- Magellan writes: File nodes, Symbol nodes, DEFINES edges, REFERENCES edges
- OdinCode writes: Execution nodes, EXECUTED_ON/AFFECTED/PRODUCED/REFERENCED edges

**No conflicts**: Entity kinds are disjoint (Magellan: File/Symbol, OdinCode: execution)

---

## RUNTIME CONFIGURATION (FUTURE, NOT PHASE 0.5)

### Environment Variable (Future)

```
export ODINCODE_HOME=/path/to/odincode/root
```

**Default**: If `$ODINCODE_HOME` not set, use:
- `$HOME/.odincode/db/` (Unix)
- `%APPDATA%/odincode/db/` (Windows)

### Configuration File (Future)

`~/.odincode/config.toml`:
```toml
[database]
root = "/path/to/db/root"
```

**Phase 0.5 Scope**: Tests use `tempfile::tempdir()`, no config file parsing

---

## MIGRATION PATH (IF CODEGRAPH DB EXISTS ELSEWHERE)

### For Users with Existing Magellan DBs

**Scenario**: User has `/my/project/codegraph.db` from Magellan

**Option 1: Symlink** (recommended for development)
```bash
mkdir -p ~/.odincode/db
ln -s /my/project/codegraph.db ~/.odincode/db/codegraph.db
```

**Option 2: Copy** (for isolation)
```bash
mkdir -p ~/.odincode/db
cp /my/project/codegraph.db ~/.odincode/db/codegraph.db
```

**Option 3: Configuration** (future)
```bash
# Set ODINCODE_HOME to project root
export ODINCODE_HOME=/my/project
# OdinCode finds /my/project/codegraph.db automatically
```

---

## ACCEPTANCE CRITERIA (PHASE 0.5.2)

Phase 0.5.2 implementation MUST:

1. ✅ `ExecutionDb::open(db_root: &Path)` accepts directory path
2. ✅ Auto-create `db_root/execution_log.db` if missing
3. ✅ Return `Error::CodegraphNotFound` if `db_root/codegraph.db` missing
4. ✅ Tests create temp dir + minimal `codegraph.db` (no hardcoded paths)
5. ✅ Tests SKIP gracefully if `codegraph.db` schema incompatible
6. ✅ No hardcoded DB names or paths in production code
7. ✅ Two separate SQLite connections (no ATTACH)

---

## VERIFICATION CHECKLIST

Before marking Phase 0.5.2 complete:

- [ ] All file paths relative to `db_root` parameter
- [ ] `execution_log.db` creation tested (missing file scenario)
- [ ] `codegraph.db` missing scenario tested (explicit error)
- [ ] Tests use `tempfile::tempdir()` for isolation
- [ ] No references to `syncore_codegraph.db` in execution_tools
- [ ] No references to `/home/feanor/Projects/` in execution_tools
- [ ] `cargo test` passes with no environment dependencies

---

*Last Updated: 2025-12-24*
*Status: LOCKED — Phase 0.5.2 Implementation Contract*
*Purpose: Resolve DB location ambiguity with clean separation*
