# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

OdinCode is a **deterministic tool substrate** for LLM-powered refactoring — NOT an AI system itself. It provides boring, factual tools that an LLM MUST use instead of guessing about the filesystem, codebase structure, or compiler output.

**Core Philosophy**: "ZERO GUESSING" — Every operation grounded in facts from real tools.

## Current Status

**Phase**: 0 — Tool Substrate (COMPLETE)
**Test Coverage**: 27/27 integration tests passing
**Max LOC**: 300 LOC per module (all files comply)

### Completed Modules

- ✅ **file_tools** — Filesystem operations (read, write, create, search, glob)
- ✅ **splice_tools** — Span-safe refactoring via Splice binary
- ✅ **magellan_tools** — SQLiteGraph queries (read-only direct access)
- ✅ **lsp_tools** — Compiler diagnostics via cargo check

## Build & Test Commands

### Standard Development

```bash
# Check compilation
cargo check

# Run all tests
cargo test

# Run specific test suite
cargo test --test file_tools_tests
cargo test --test splice_tools_tests
cargo test --test magellan_tools_tests
cargo test --test lsp_tools_tests

# Run with output
cargo test -- --nocapture

# Build binary
cargo build --release
```

### Test Organization

All tests are **integration tests** using real tools:
- `tests/file_tools_tests.rs` — 13 tests covering file operations
- `tests/splice_tools_tests.rs` — 5 tests covering Splice integration
- `tests/magellan_tools_tests.rs` — 5 tests covering SQLiteGraph queries
- `tests/lsp_tools_tests.rs` — 4 tests covering compiler diagnostics

**Critical**: Tests use **real tools only** (no mocks). Tests gracefully skip if external tools unavailable:
- `splice` binary (from PATH)
- `magellan` binary (from PATH)
- `cargo` (required for lsp_tests)

## Architecture

### Module Structure

```
src/
├── lib.rs                    # Library root, exports all public APIs
├── main.rs                   # Binary placeholder (Phase 0 only)
├── file_tools/              # Filesystem operations
│   ├── mod.rs               # Module exports
│   ├── file_read.rs        # Read file contents
│   ├── file_write.rs       # Atomic overwrite with fsync
│   ├── file_create.rs      # Create if not exists
│   ├── file_search.rs      # Ripgrep wrapper
│   └── file_glob.rs         # Glob pattern matching
├── splice_tools/            # Splice binary integration
│   ├── mod.rs               # SpliceResult struct + exports
│   ├── splice_patch.rs      # Single symbol replacement
│   └── splice_plan.rs       # Multi-step refactoring plans
├── magellan_tools/          # SQLiteGraph direct queries
│   ├── mod.rs               # Module exports
│   └── db.rs                # MagellanDb with rusqlite (249 LOC, max allowed)
└── lsp_tools/               # Compiler diagnostics
    ├── mod.rs               # Module exports
    └── check.rs             # cargo check JSON parsing (110 LOC)
```

### Tool Integration Patterns

**All tools follow strict patterns**:

1. **No async** — Everything synchronous
2. **No background threads** — Except process execution (std::process::Command)
3. **No global state** — Dependencies passed explicitly
4. **Opaque JSON payloads** — For SQLiteGraph data (no per-property access)
5. **Real tools only** — Direct subprocess calls to external binaries
6. **Deterministic output** — All queries sorted (ORDER BY in SQL, sorted() for collections)

### External Tool Dependencies

The project depends on three external tools (see `docs/EXTERNAL_TOOLS_API.md`):

1. **Magellan** (`/home/feanor/.local/bin/magellan`)
   - Purpose: Codebase indexer (tree-sitter → SQLiteGraph)
   - CLI: `magellan watch --root <DIR> --db <FILE> [--debounce-ms <N>]`
   - Used by: Tests only (creates temp Cargo projects, runs indexing)
   - OdinCode: Queries DB directly via rusqlite (no CLI calls)

2. **Splice** (`/home/feanor/.local/bin/splice`)
   - Purpose: Span-safe refactoring (tree-sitter + cargo validation)
   - CLI: `splice patch --file <FILE> --symbol <NAME> [--kind <KIND>] --with <FILE>`
   - CLI: `splice plan --file <PLAN.json>`
   - Used by: `splice_patch()`, `splice_plan()` wrappers
   - Requires: Cargo workspace context for validation

3. **SQLiteGraph** (Rust library at `/home/feanor/Projects/sqlitegraph`)
   - Purpose: Graph database (SQLite + HNSW vectors)
   - Schema: `graph_entities` (id, kind, name, file_path, data), `graph_edges` (id, from_id, to_id, edge_type, data)
   - Used by: `MagellanDb::open_readonly()` — rusqlite direct access, no CLI
   - **CRITICAL**: Use concrete `SqliteGraphBackend`, not trait object. See `docs/EXTERNAL_TOOLS_API.md`.

## Key Constraints (NON-NEGOTIABLE)

From `docs/CONTRACT.md`:

1. **NO INTELLIGENCE** — We build tools, not AI
2. **NO GUESSING** — Every operation grounded in facts
3. **NO EDITOR UI** — Console tool layer only (Phase 0)
4. **NO ASYNC** — Synchronous, deterministic execution
5. **NO MOCKS** — Real tools, real filesystem, real compiler

### File Size Constraint

**MAX 300 LOC per module** — Enforced across all source files.

Largest file: `src/magellan_tools/db.rs` at 249 LOC (within limit).

## TDD Workflow (Strict)

**Tests first, implementation second.**

1. Write failing test
2. Prove test fails
3. Implement minimal code to pass
4. Verify test passes
5. Repeat

No implementation without test coverage.

## Error Handling

All tools use `thiserror` for structured errors:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileReadError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),
}
```

Pattern: `pub type Result<T> = std::result::Result<T, ErrorName>;`

## Critical Implementation Notes

### SQLiteGraph Integration (MagellanDb)

**Reference**: `docs/EXTERNAL_TOOLS_API.md` → "SQLiteGraph" section

**Common Pitfalls** (from `/home/feanor/Projects/magellan/docs/SQLITEGRAPH_GUIDE.md`):

1. **Opaque JSON payloads** — All data stored as single JSON field (no per-property access like Neo4j)
2. **Use concrete type** — `SqliteGraphBackend`, not `Box<dyn GraphBackend>` (needed for delete operations)
3. **Import GraphBackend trait** — Required to call trait methods like `neighbors()`
4. **Correct field names** — `edge_type` not `edge_filter` in `NeighborQuery`
5. **Manual cascade deletes** — Delete dependents before parent (no automatic cascade)

Example:
```rust
use sqlitegraph::{
    SqliteGraphBackend,
    GraphBackend,  // ✅ MUST import trait
    NeighborQuery,
    BackendDirection,
};

let backend = SqliteGraphBackend::from_graph(sqlite_graph)?;
let neighbors = backend.neighbors(node_id, NeighborQuery {
    direction: BackendDirection::Outgoing,
    edge_type: Some("DEFINES".to_string()),  // ✅ edge_type, not edge_filter
})?;
```

### Splice Integration

**Requires Cargo workspace** — Splice validates changes with `cargo check`, so it MUST run within a Cargo project.

Tests create minimal `Cargo.toml` + `src/lib.rs` structure to satisfy this requirement.

Changed files detection: Parse stdout for "Patched" substring to populate `changed_files` list.

### File Operations

**Atomic writes**: `file_write()` uses temp file + fsync + rename pattern:
1. Write to `file.tmp`
2. fsync temporary file
3. `rename()` over original (atomic on POSIX)

**Deterministic glob**: `file_glob()` results are sorted for reproducible output.

## Documentation Structure

- `docs/CONTRACT.md` — **FROZEN** scope for Phase 0 (authoritative)
- `docs/TODO.md` — Phase tracking and completion status
- `docs/CURRENT_CAPABILITIES.md` — What exists vs. what's missing (execution memory gap)
- `docs/PHASE_0_5_EXECUTION_MEMORY.md` — Locked schema for execution logging (next phase)
- `docs/EXTERNAL_TOOLS_API.md` — **CRITICAL**: Magellan, Splice, SQLiteGraph APIs (preserve after chat compaction)
- `docs/TOOL_MANUALS.md` — CLI reference for Magellan, Splice, sqlite3

## What's NOT Implemented (Yet)

From `docs/CURRENT_CAPABILITIES.md`:

- ❌ **Outcome Persistence** — No record of tool invocations
- ❌ **Pattern Storage** — No record of success/failure patterns
- ❌ **Causal Linking** — No connection between operations and errors
- ❌ **Temporal Queries** — Cannot query "has this failed before?"

**Next Phase**: 0.5 — Execution Memory & Outcome Logging (schema LOCKED, awaiting implementation)

See `docs/PHASE_0_5_EXECUTION_MEMORY.md` for locked schema.

## Development Workflow

When adding new functionality:

1. Read `docs/CONTRACT.md` — Verify scope allows feature
2. Read `docs/TODO.md` — Check phase status
3. Write failing test in `tests/*_tools_tests.rs`
4. Run `cargo test` — Prove failure
5. Implement in `src/<tool>_tools/<module>.rs`
6. Ensure file ≤ 300 LOC
7. Run `cargo test` — Prove success
8. Update `docs/TODO.md` — Mark task complete

**Never skip TDD.**
