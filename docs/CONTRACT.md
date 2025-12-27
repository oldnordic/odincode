# OdinCode v0 Contract — Phase 0

**Status**: FROZEN SCOPE
**Phase**: 0 — Tool Substrate Only
**Date**: 2025-12-24

---

## NON-NEGOTIABLE PRINCIPLES

1. **NO INTELLIGENCE** — We build tools, not AI
2. **NO GUESSING** — Every operation grounded in facts
3. **NO EDITOR UI** — Console tool layer only
4. **NO ASYNC** — Synchronous, deterministic execution
5. **NO MOCKS** — Real tools, real filesystem, real compiler

---

## SCOPE (PHASE 0 ONLY)

We build ONLY the tool orchestration layer.

**IN SCOPE ✅**:
- File operations (read, write, create, search, glob)
- Splice integration (patch, plan)
- Magellan integration (query SQLiteGraph directly)
- LSP integration (rust-analyzer, cargo check)
- Structured error handling
- TDD with real integration tests

**OUT OF SCOPE ❌**:
- Editor UI (ratatui, terminal)
- LLM API integration
- Memory abstraction
- Agents
- Workflows
- Plans
- Auto-refactoring
- Auto-anything

---

## TOOL CONTRACTS

### 1. file_read
```rust
fn file_read(path: &Path) -> Result<String>
```
- Reads entire file
- Returns error if missing
- No buffering, no streaming

### 2. file_write
```rust
fn file_write(path: &Path, contents: &str) -> Result<()>
```
- Atomic overwrite (write + rename)
- fsync required
- No partial writes

### 3. file_create
```rust
fn file_create(path: &Path, contents: &str) -> Result<()>
```
- Creates if not exists
- Errors if exists (no overwrite)

### 4. file_search
```rust
fn file_search(pattern: &str, path: &Path) -> Result<Vec<SearchMatch>>
```
- Wraps `rg` (ripgrep)
- Returns structured matches
- Deterministic output

### 5. glob
```rust
fn glob_files(pattern: &str) -> Result<Vec<PathBuf>>
```
- Lists files matching pattern
- Sorted deterministically

### 6. splice_apply
```rust
fn splice_apply(file: &Path, symbol: &str, kind: Option<SymbolKind>, replacement: &Path) -> Result<SpliceOutput>
```
- Calls `splice patch` binary
- Captures stdout/stderr
- Fails on non-zero exit

### 7. magellan_query
```rust
fn magellan_query(db: &Path, sql: &str) -> Result<Vec<Row>>
```
- Queries SQLiteGraph directly
- NO calling magellan CLI
- Returns raw rows

### 8. lsp_check
```rust
fn lsp_check(path: &Path) -> Result<Vec<Diagnostic>>
```
- Calls `rust-analyzer` or `cargo check`
- Returns structured diagnostics
- Parse errors from JSON/text

---

## ARCHITECTURE RULES

1. **MAX FILE SIZE**: 300 LOC per module
2. **NO ASYNC**: Everything synchronous
3. **NO THREADS**: Except process execution
4. **NO GLOBAL STATE**: Pass dependencies explicitly
5. **TDD REQUIRED**: Write failing tests first
6. **REAL TOOLS ONLY**: No mocks for filesystem, splice, magellan, LSP

---

## VALIDATION CRITERIA

Phase 0 complete when:
- ✅ All 8 tools implemented
- ✅ All tests pass with real tools
- ✅ No mocks (except CI stubs)
- ✅ cargo check passes
- ✅ cargo test passes
- ✅ All files ≤ 300 LOC

---

## DELIVERABLES

```
/src
  - lib.rs
  - tools/
      - mod.rs
      - file.rs        (file_read, file_write, file_create)
      - search.rs      (file_search, glob)
      - splice.rs      (splice_apply)
      - magellan.rs    (magellan_query)
      - lsp.rs         (lsp_check)

/tests
  - file_tools_tests.rs
  - search_tools_tests.rs
  - splice_tools_tests.rs
  - magellan_tools_tests.rs
  - lsp_tools_tests.rs
```

---

## WHAT HAPPENS NEXT

Phase 0.1: File tools (file_read, file_write, file_create)
Phase 0.2: Search tools (file_search, glob)
Phase 0.3: Splice integration
Phase 0.4: Magellan integration
Phase 0.5: LSP integration

**NO PHASE 1 UNTIL PHASE 0 COMPLETE**

---

*This contract is frozen. Scope changes require explicit renegotiation.*
