# External Tools API Reference — Source Truth Documentation

**Purpose**: Preserve critical API/context for Magellan, Splice, SQLiteGraph after chat compaction
**Source**: Direct observation of source code and documentation from `/home/feanor/Projects/`
**Constraint**: NO GUESSING — only documented facts from actual tool implementations

---

## Why This Document Exists

After LLM chat compaction, context about these custom/new tools is lost because:
- They were NOT in training data (too new)
- They have unique APIs not found in mainstream tools
- Re-learning them requires reading source code each time

This document captures **authoritative reference** directly from source code and manuals.

---

## Table of Contents

1. [Magellan — Codebase Mapper](#magellan)
2. [Splice — Span-Safe Refactoring](#splice)
3. [SQLiteGraph — Graph Database](#sqlitegraph)
4. [Integration Patterns](#integration)

---

## Magellan

### Source Location
```
/home/feanor/Projects/magellan/
```

### Purpose (from CONTRACT.md)
> "Magellan is a dumb, deterministic codebase mapping tool.
> Purpose: Observe files, extract symbols and references, persist facts to sqlitegraph.
> Role: Provide facts to external intelligence (LLM). No reasoning. No refactoring. No orchestration."

### What Magellar Does

**Observes**:
- Watches files for changes using `notify` crate
- Parses source code using `tree-sitter`
- Extracts symbols (functions, structs, enums, traits, methods, modules)
- Extracts references (calls, type usage, imports)

**Persists**:
- Stores facts in SQLiteGraph database
- Creates File nodes and Symbol nodes
- Creates DEFINES edges (File → Symbol)
- Creates REFERENCES edges (Symbol → Symbol)

**Answers Queries**:
- "Where is symbol X defined?"
- "Where is symbol X referenced?"
- "Which file changed?"
- "Which symbols were affected by this file change?"

### What Magellan Does NOT Do

- Does NOT modify code
- Does NOT generate patches
- Does NOT rename symbols
- Does NOT call cargo
- Does NOT call rust-analyzer
- Does NOT perform semantic reasoning
- Does NOT infer intent
- Does NOT orchestrate workflows
- Does NOT cache LLM state
- Does NOT contain MCP logic
- Does NOT contain agent logic

**"Magellan is standalone. All intelligence lives outside Magellan."**

### CLI Interface

**Only Command**: `watch`

```bash
magellan watch --root <DIR> --db <FILE> [--debounce-ms <N>]
```

**Arguments**:
- `--root <DIR>`: Directory to watch recursively (REQUIRED)
- `--db <FILE>`: Path to SQLiteGraph database file (REQUIRED)
- `--debounce-ms <N>`: Debounce delay in milliseconds (OPTIONAL, default: 500)

**No CLI query interface** — All queries happen via direct SQLite access

### Data Model (from CONTRACT.md)

**Nodes**:
```
File { path: String, hash: String }
Symbol { name: String, kind: SymbolKind, byte_start: usize, byte_end: usize }
```

**Edges**:
```
DEFINES (File → Symbol)
REFERENCES (Symbol → Symbol)
```

**SymbolKind enum**:
```
Function
Struct
Enum
Trait
Method
Module
Unknown
```

### Guarantees

- **Determinism**: Same input → same graph state
- **Observability**: All changes persisted to sqlitegraph
- **Update-on-change**: File change → delete all derived data → re-ingest
- **Query correctness**: Answers reflect actual persisted state
- **No semantic inference**: Facts extracted from AST only

### Technical Constraints

- Language: Rust
- Graph backend: SQLiteGraph ONLY
- Parsing: tree-sitter ONLY
- File watching: notify crate
- No Neo4j
- No in-memory-only structures
- No LSP integration
- No macro expansion
- No type inference

### Schema Version

**Current version**: v0
**Supported language**: Rust (only)
**Symbol types**: functions, structs, enums, traits, methods, modules
**Reference types**: calls, type usage, imports

---

## Splice

### Source Location
```
/home/feanor/Projects/splice/
```

### Purpose (from manual.md)
> "Splice is a span-safe refactoring tool for Rust code that performs
> byte-accurate, AST-validated replacements using SQLiteGraph as the
> ground-truth code graph."

### Core Philosophy

1. **Span Safety**: All operations work with exact byte spans (start, end positions)
2. **Validation Gates**: Every patch passes tree-sitter reparse + cargo check
3. **Atomic Rollback**: Any failure triggers automatic rollback
4. **No Magic**: Explicit parameters only, no hidden behavior

### What Splice Does

- Replace function bodies, struct definitions, enum variants, trait definitions, impl blocks
- Validate syntax with tree-sitter after every patch
- Validate semantics with cargo check after every patch
- Rollback atomically if ANY validation fails
- Orchestrate multi-step refactors with JSON plans

### What Splice Does NOT Do

- Cross-file reference tracking (use `rg` or IDE features)
- Automatic symbol discovery (you must know exact symbol names)
- Smart refactoring (no "rename all references" - that's Task 9+)
- Configuration files (CLI only)
- Persistent databases (on-the-fly graph creation)

### Commands

#### splice patch

Apply a single patch to a symbol's span.

```bash
splice patch \
  --file <PATH> \
  --symbol <NAME> \
  [--kind <KIND>] \
  --with <FILE>
```

**Required Arguments**:
- `--file <PATH>`: Path to source file containing the symbol
- `--symbol <NAME>`: Symbol name to patch
- `--with <FILE>`: Path to file containing replacement content

**Optional Arguments**:
- `--kind <KIND>`: Symbol kind filter (`function`, `struct`, `enum`, `trait`, `impl`)
- `--analyzer <MODE>`: rust-analyzer mode (`off`, `os`)
- `-v, --verbose`: Enable verbose logging

#### splice plan

Execute a multi-step refactoring plan.

```bash
splice plan --file <PLAN.json>
```

**Required Arguments**:
- `--file <PLAN.json>`: Path to JSON plan file

**Optional Arguments**:
- `-v, --verbose`: Enable verbose logging

**Plan Format**:
```json
{
  "steps": [
    {
      "file": "src/lib.rs",
      "symbol": "foo",
      "kind": "function",
      "with": "patches/foo.rs"
    }
  ]
}
```

**Execution Behavior**:
1. Steps execute **sequentially** in order
2. Execution **stops on first failure**
3. Previous successful steps **remain applied** (no global rollback)
4. Each step has **atomic rollback** via validation gates

### Symbol Kinds

| Kind | Example |
|------|---------|
| `function` | `pub fn foo() {}` |
| `struct` | `pub struct Foo;` |
| `enum` | `pub enum Bar {}` |
| `trait` | `pub trait Baz {}` |
| `impl` | `impl Foo {}` |

### Validation Gates

Every patch passes through multiple validation gates:

1. **UTF-8 Boundary Validation**: Ensures byte positions align with UTF-8 boundaries
2. **Tree-Sitter Reparse**: Validates syntax using tree-sitter-rust
3. **Cargo Check**: Validates semantic correctness via full workspace compilation
4. **rust-analyzer (Optional)**: Opt-in via `--analyzer os` flag

**Rollback Behavior**:
- **Automatic**: Any gate failure triggers immediate rollback
- **Atomic**: Original file restored atomically (temp + fsync + rename)
- **Safe**: No partial patch states possible

### Error Handling

**Common Errors**:

| Error | Cause | Solution |
|-------|-------|----------|
| `Symbol not found` | Symbol name doesn't match | Check spelling, verify `--file`, add `--kind` |
| `Ambiguous symbol` | Multiple files define same symbol | Add `--file` to disambiguate |
| `Parse validation failed` | Patch file has syntax error | Run `cargo check` on patch file first |
| `Cargo check failed` | Patch introduces type error | Fix types in patch file first |

### Technical Details

**How Splice Works**:
1. Extract Symbols: Parse source file with tree-sitter-rust
2. Resolve Symbol: Find symbol byte span in AST
3. Read Replacement: Load patch file content
4. Replace Span: Use ropey for byte-exact replacement
5. Validate: Run tree-sitter reparse + cargo check
6. Commit or Rollback: Atomic based on validation

**Byte Spans vs Line/Column**:
Splice uses **byte offsets** (not line/column) because:
- Deterministic (independent of editor line endings)
- Exact (no ambiguity in multi-byte characters)
- Fast (no conversion overhead)

### Current Limitations (v0.1.0)

1. No Cross-File Reference Tracking
2. No Resume Mode (failed plans leave partial state)
3. No Auto-Discovery (must know exact symbol names)
4. No Persistent Database (graph created on-the-fly)
5. No Dry-Run Mode (can't preview without applying)
6. Single-File Symbols (can't patch symbols across multiple files)

---

## SQLiteGraph

### Source Location
```
/home/feanor/Projects/sqlitegraph/
```

### Purpose (from manual.md)
> "sqlitegraph is a graph database built on SQLite with a minimal API surface."

### Critical API Patterns (from SQLITEGRAPH_GUIDE.md)

**CRITICAL RULE**: "Do NOT assume sqlitegraph APIs work like other graph databases.
Read the source code before using any API."

### Architecture: Backend Trait Pattern

```rust
pub trait GraphBackend {
    fn insert_node(&self, node: NodeSpec) -> Result<i64, SqliteGraphError>;
    fn get_node(&self, id: i64) -> Result<GraphEntity, SqliteGraphError>;
    fn insert_edge(&self, edge: EdgeSpec) -> Result<i64, SqliteGraphError>;
    fn neighbors(&self, node: i64, query: NeighborQuery) -> Result<Vec<i64>, SqliteGraphError>;
    // ... other methods
}
```

**Key Implementation**: `SqliteGraphBackend` is the concrete SQLite implementation.

### Lesson 1: Opaque JSON Payloads (NO Per-Property Access)

**❌ WRONG**:
```rust
// WRONG: Assuming per-property access like Neo4j
let node = graph.get_node(id)?;
let name: String = node.get_property("name")?;
```

**✅ CORRECT**:
```rust
// CORRECT: All data stored as opaque JSON payload
#[derive(Serialize, Deserialize)]
struct FileNode {
    path: String,
    hash: String,
}

// Store entire struct as JSON
let node_spec = NodeSpec {
    kind: "File".to_string(),
    name: path.clone(),
    file_path: Some(path.clone()),
    data: serde_json::to_value(FileNode {
        path: path.to_string(),
        hash: hash.to_string(),
    })?,
};

let id = backend.insert_node(node_spec)?;

// Retrieve: Deserialize entire payload
let entity = backend.get_node(id)?;
let file_node: FileNode = serde_json::from_value(entity.data)?;
```

**Why This Design?**
- Simplicity: Single column stores all properties
- Flexibility: Each consumer owns their schema
- Performance: No property table joins
- Schemaless: Add fields without migrations

### Lesson 2: Use Concrete Type, Not Trait Object

**❌ WRONG**:
```rust
// WRONG: Using trait object and downcasting
let graph: Box<dyn GraphBackend> = open_graph(db_path, &cfg)?;
// Try to downcast to access delete operations
let backend = graph.as_any().downcast::<SqliteGraphBackend>()?;
// ERROR: as_any() method doesn't exist
```

**✅ CORRECT**:
```rust
// CORRECT: Directly create concrete type
let sqlite_graph = SqliteGraph::open(db_path)?;
let backend = SqliteGraphBackend::from_graph(sqlite_graph);

// Now backend has full API access
backend.graph().delete_entity(node_id)?;
backend.graph().delete_edge(edge_id)?;
```

**When to Use Each**:
- **Use `GraphBackend` trait**: When you only need read/query operations
- **Use `SqliteGraphBackend`**: When you need delete operations or direct SQLite access

### Lesson 3: NeighborQuery Field Names

**❌ WRONG**:
```rust
// WRONG: Assuming neo4j-style field names
let neighbor_ids = backend.neighbors(node_id, NeighborQuery {
    direction: BackendDirection::Outgoing,
    edge_filter: Some("DEFINES".to_string()),  // ❌ Wrong field name
    node_filter: None,                          // ❌ Field doesn't exist
})?;
```

**✅ CORRECT**:
```rust
// CORRECT: Use actual field names
let neighbor_ids = backend.neighbors(node_id, NeighborQuery {
    direction: BackendDirection::Outgoing,
    edge_type: Some("DEFINES".to_string()),  // ✅ Correct
})?;
```

**NeighborQuery Definition**:
```rust
pub struct NeighborQuery {
    pub direction: BackendDirection,  // Outgoing | Incoming
    pub edge_type: Option<String>,    // Filter by edge type
    // NO node_filter field
}
```

### Lesson 4: Must Import Trait to Use Methods

**❌ COMMON MISTAKE**:
```rust
use sqlitegraph::{SqliteGraphBackend, NeighborQuery, BackendDirection};
// Missing: GraphBackend trait import

let neighbors = backend.neighbors(node_id, NeighborQuery::default())?;
// ERROR: no method named `neighbors` found for `SqliteGraphBackend`
```

**✅ CORRECT**:
```rust
use sqlitegraph::{
    SqliteGraphBackend,
    GraphBackend,  // ✅ Import trait
    NeighborQuery,
    BackendDirection,
};

let neighbors = backend.neighbors(node_id, NeighborQuery::default())?;
// ✅ Works! GraphBackend trait is in scope
```

### Core Types Reference

```rust
/// Node specification for insertion
pub struct NodeSpec {
    pub kind: String,              // Node label (e.g., "File", "Symbol")
    pub name: String,              // Human-readable name
    pub file_path: Option<String>, // Optional file path
    pub data: serde_json::Value,   // Opaque payload
}

/// Edge specification for insertion
pub struct EdgeSpec {
    pub from: i64,                 // Source node ID
    pub to: i64,                   // Target node ID
    pub edge_type: String,         // Edge label (e.g., "DEFINES")
    pub data: serde_json::Value,   // Opaque payload
}

/// Neighbor query configuration
pub struct NeighborQuery {
    pub direction: BackendDirection,  // Outgoing | Incoming
    pub edge_type: Option<String>,    // Filter by edge type
}

/// Graph entity (node)
pub struct GraphEntity {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub file_path: Option<String>,
    pub data: serde_json::Value,
}

/// Direction for traversals
pub enum BackendDirection {
    Outgoing,
    Incoming,
}

/// Node ID wrapper
pub struct NodeId(i64);

impl NodeId {
    pub fn from(id: i64) -> Self;
    pub fn as_i64(&self) -> i64;
}
```

### Common Pitfalls

**Pitfall 1: Forgetting serde_json::Value**
```rust
// ❌ WRONG
let data = "some string";

// ✅ CORRECT
let data = serde_json::to_value(some_struct)?;
```

**Pitfall 2: Using wrong NodeId type**
```rust
// ❌ WRONG
let node_id: i64 = backend.insert_node(node_spec)?;

// ✅ CORRECT
let id: i64 = backend.insert_node(node_spec)?;
let node_id = NodeId::from(id);
```

**Pitfall 3: Assuming cascade delete**
```rust
// ❌ WRONG: Deleting file node doesn't auto-delete symbols
backend.graph().delete_entity(file_id)?;
// Symbols still exist, edges are broken

// ✅ CORRECT: Manually delete symbols first
self.delete_file_symbols(file_id)?;
backend.graph().delete_entity(file_id)?;
```

**Pitfall 4: Not checking Option results**
```rust
// ❌ WRONG: Will panic on None
let node: FileNode = serde_json::from_value(entity.data).unwrap();

// ✅ CORRECT: Handle deserialization failure
let node: Option<FileNode> = serde_json::from_value(entity.data).ok();
let node = match node {
    Some(n) => n,
    None => return Ok(None),
};
```

### CLI Commands

**Binary**: `sqlitegraph`

```bash
sqlitegraph [--backend sqlite] [--db memory|PATH] --command <subcommand> [args]
```

**Deterministic subcommands**:
- `status` (default) – backend + entity count
- `list` – entity IDs + names (ascending id)
- `subgraph --root N --depth D [--types edge=CALLS --types node=Fn]`
- `pipeline --dsl "<dsl>"` or `--file pipeline.json`
- `explain-pipeline --dsl "<dsl>"`
- `dsl-parse --input "<expr>"`
- `safety-check [--strict]`

### DSL Syntax

**Pattern queries**:
- `CALLS->USES` → Two-leg pattern
- `CALLS*3` → Repeated pattern (3 hops)

**K-hop queries**:
- `3-hop type=Fn` → 3-hop traversal with node type filter

**Pipeline**:
- `pattern CALLS*3 filter type=Module` → Pattern with filter

---

## Integration Patterns

### Typical Workflow

**1. Start Magellan**:
```bash
magellan watch --root . --db codegraph.db --debounce-ms 500 > /tmp/magellan.log 2>&1 &
```

**2. Trigger indexing**:
```bash
find src -name "*.rs" -exec touch {} \; && sleep 5
```

**3. Query codebase via SQLiteGraph**:
```bash
# Using Rust API (OdinCode pattern)
let db = MagellanDb::open_readonly("codegraph.db")?;
let symbols = db.symbols_in_file("%/lib.rs")?;
let refs = db.references_to_symbol_name("foo")?;

# Using sqlite3 CLI
sqlite3 codegraph.db <<EOF
SELECT ge.name
FROM graph_entities ge
JOIN graph_edges e ON ge.id = e.to_id
WHERE ge.kind = 'Symbol'
  AND e.edge_type = 'REFERENCES';
EOF
```

**4. Apply refactoring with Splice**:
```bash
# Single symbol
splice patch --file src/lib.rs --symbol old_function --kind function --with patches/new_function.rs

# Multi-step plan
splice plan --file refactor_plan.json
```

**5. Verify with compiler**:
```bash
cargo check --message-format=json 2>&1 | rg '"compiler-message"'
```

### Data Flow

```
File System → Magellan (tree-sitter) → SQLiteGraph (nodes + edges)
                                                    ↓
                                              OdinCode (queries)
                                                    ↓
                                               Splice (patches)
                                                    ↓
                                             Compiler (validates)
```

### Schema Reference (from syncore_codegraph.db)

**graph_entities table**:
```sql
CREATE TABLE graph_entities (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    kind      TEXT NOT NULL,        -- "File", "Symbol", "Reference"
    name      TEXT NOT NULL,        -- File path or symbol name
    file_path TEXT,                 -- Source file path
    data      TEXT NOT NULL         -- JSON metadata
);
```

**graph_edges table**:
```sql
CREATE TABLE graph_edges (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id   INTEGER NOT NULL,     -- References graph_entities.id
    to_id     INTEGER NOT NULL,     -- References graph_entities.id
    edge_type TEXT NOT NULL,        -- "DEFINES", "REFERENCES"
    data      TEXT NOT NULL         -- JSON metadata
);
```

**Observed Entity Kinds**:
- `File` — Source file entities
- `Symbol` — Code symbols (functions, structs, enums, traits, impls)
- `Reference` — References to symbols

**Observed Edge Types**:
- `DEFINES` — Symbol defined in file (file → symbol)
- `REFERENCES` — Symbol references another symbol (symbol → symbol)

---

## Quick Reference Card

### Magellan

| Command | Purpose |
|---------|---------|
| `magellan watch --root . --db codegraph.db` | Start indexing |
| `find src -name "*.rs" \| exec touch {} \;` | Trigger re-index |

### Splice

| Command | Purpose |
|---------|---------|
| `splice patch --file src/lib.rs --symbol foo --kind function --with new_foo.rs` | Single patch |
| `splice plan --file plan.json` | Multi-step plan |

### SQLiteGraph

| Operation | Code |
|-----------|------|
| Open DB | `SqliteGraph::open(path)` |
| Create backend | `SqliteGraphBackend::from_graph(graph)` |
| Insert node | `backend.insert_node(NodeSpec{...})` |
| Insert edge | `backend.insert_edge(EdgeSpec{...})` |
| Query neighbors | `backend.neighbors(node_id, NeighborQuery{...})` |
| **CRITICAL** | Import `GraphBackend` trait to call methods |
| **CRITICAL** | Use `SqliteGraphBackend` for delete operations |
| **CRITICAL** | All data stored as opaque JSON (not per-property) |

---

*Last Updated: 2025-12-24*
*Source: Direct observation of source code, manuals, and databases*
*Version: Magellan v0, Splice v0.1.0, SQLiteGraph 0.2.10*
