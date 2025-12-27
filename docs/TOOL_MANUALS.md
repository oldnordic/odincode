# Tool Manuals — Authoritative Reference

**Purpose**: Precise documentation for external tools used by OdinCode
**Scope**: Magellan, Splice, SQLiteGraph (sqlite3)
**Constraint**: No guessing, no assumptions — observed behavior only

---

## Magellan

### Binary Location
```
/home/feanor/.local/bin/magellan
```

### Command Syntax

**Only Command**: `watch`

```bash
magellan watch --root <DIR> --db <FILE> [--debounce-ms <N>]
```

### Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `--root <DIR>` | Path | Yes | Directory to watch recursively |
| `--db <FILE>` | Path | Yes | Path to SQLiteGraph database file |
| `--debounce-ms <N>` | Integer | No | Debounce delay in milliseconds (default: 500) |

### Behavior

**Observed Operation**:
1. Starts background file watcher on `--root` directory
2. Recursively scans all `.rs` files
3. Uses tree-sitter to parse Rust code
4. Extracts symbols (functions, structs, enums, traits, impls)
5. Extracts references between symbols
6. Writes to SQLiteGraph database (`--db` path)
7. On file changes, waits `--debounce-ms` then re-indexes changed files

**Indexing Trigger**:
- Initial scan on start
- File modification times change
- Debounce delay prevents excessive re-indexing

**No CLI Query Interface**:
- Magellan does NOT provide query commands
- All queries happen via direct SQLiteGraph access
- Use `sqlite3 <db_path>` to query

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Normal shutdown |
| 1 | Error in arguments or file system |

---

## Splice

### Binary Location
```
/home/feanor/.local/bin/splice
```

### Commands

Two subcommands: `patch`, `plan`

---

### splice patch

**Purpose**: Apply a patch to a symbol's span

```bash
splice patch [OPTIONS] --file <FILE> --symbol <SYMBOL> --with <FILE>
```

#### Options

| Option | Short | Type | Required | Description |
|--------|-------|------|----------|-------------|
| `--file <FILE>` | `-f` | Path | Yes | Path to the source file containing the symbol |
| `--symbol <SYMBOL>` | `-s` | String | Yes | Symbol name to patch |
| `--kind <KIND>` | `-k` | Enum | No | Optional symbol kind filter |
| `--with <FILE>` | `-w` | Path | Yes | Path to file containing replacement content |
| `--analyzer <MODE>` | | Enum | No | Optional rust-analyzer validation mode |
| `--verbose` | `-v` | Flag | No | Enable verbose logging |
| `--help` | `-h` | Flag | No | Print help |

#### Symbol Kind Values

Allowed values for `--kind`:
- `function`
- `struct`
- `enum`
- `trait`
- `impl`

**Default**: No kind filter (matches any kind)

#### rust-analyzer Modes

Allowed values for `--analyzer`:
- `off`: Disable rust-analyzer validation (default)
- `os`: Use rust-analyzer from PATH
- `path`: Use rust-analyzer from explicit path

#### Observed Behavior

1. Reads target file (`--file`)
2. Finds symbol span using tree-sitter (filtered by `--kind` if specified)
3. Reads replacement content from `--with` file
4. Replaces symbol's span with new content
5. Validates with rust-analyzer if `--analyzer` is enabled
6. Writes modified file back

#### Cargo Workspace Requirement

**Critical**: Splice requires running within a Cargo workspace

Error if not in workspace:
```
Error: Cargo check failed in workspace '/path': error: could not find `Cargo.toml` in `/path` or any parent directory
```

#### Output

**Success** (stdout):
```
Patched <file_path>
```

**Changed Files Detection**:
- If stdout contains "Patched", add file to `changed_files` list
- If no "Patched" in stdout, `changed_files` is empty

**Failure** (stderr):
```
Error: Symbol '<symbol_name>' not found in <file_path>
Error: Cargo check failed
```

---

### splice plan

**Purpose**: Execute a multi-step refactoring plan

```bash
splice plan [OPTIONS] --file <FILE>
```

#### Options

| Option | Short | Type | Required | Description |
|--------|-------|------|----------|-------------|
| `--file <FILE>` | `-f` | Path | Yes | Path to the plan.json file |
| `--verbose` | `-v` | Flag | No | Enable verbose logging |
| `--help` | `-h` | Flag | No | Print help |

#### Plan File Format (JSON)

**Structure**:
```json
{
  "steps": [
    {
      "file": "path/to/file.rs",
      "symbol": "symbol_name",
      "kind": "function|struct|enum|trait|impl",
      "with": "path/to/replacement.rs"
    },
    ...
  ]
}
```

**Required Fields**:
- `steps`: Array of step objects

**Step Object Fields**:
- `file`: Path to source file (required)
- `symbol`: Symbol name to patch (required)
- `kind`: Symbol kind filter (optional)
- `with`: Path to replacement content (required)

#### Observed Behavior

1. Reads plan file (`--file`)
2. Parses JSON
3. Executes each step sequentially using `splice patch` logic
4. Stops on first failure
5. Does NOT validate entire plan before execution

#### Cargo Workspace Requirement

Same as `splice patch` — must run within Cargo workspace

#### Output

**Success**: No stdout (silent)

**Failure**: stderr with step number and error

---

## SQLiteGraph (sqlite3)

### Database Schema

**Actual Observed Schema** from `/home/feanor/Projects/syncore/syncore_codegraph.db`:

#### Tables

```
graph_entities
graph_edges
graph_labels
graph_properties
graph_meta
graph_meta_history
hnsw_indexes
hnsw_vectors
hnsw_layers
hnsw_entry_points
sqlite_sequence
```

#### graph_entities Table

**Schema**:
```sql
CREATE TABLE graph_entities (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    kind      TEXT NOT NULL,
    name      TEXT NOT NULL,
    file_path TEXT,
    data      TEXT NOT NULL
);

CREATE INDEX idx_entities_kind_id ON graph_entities(kind, id);
```

**Columns**:
- `id`: Auto-increment primary key
- `kind`: Entity type ("File", "Symbol", "Reference")
- `name`: Entity name (file path, symbol name, etc.)
- `file_path`: Source file path (NULL for non-file entities)
- `data`: JSON metadata

**Data Field Content** (JSON):
- For Symbols: `{"byte_start": N, "byte_end": N, "kind": "Struct|Function|Enum|Trait|Impl", "name": "SymbolName"}`
- For Files: May contain file metadata

**Observed Entity Kinds**:
- `File` — Source file entities
- `Symbol` — Code symbols (functions, structs, enums, traits, impls)
- `Reference` — References to symbols

#### graph_edges Table

**Schema**:
```sql
CREATE TABLE graph_edges (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    from_id   INTEGER NOT NULL,
    to_id     INTEGER NOT NULL,
    edge_type TEXT NOT NULL,
    data      TEXT NOT NULL
);

CREATE INDEX idx_edges_from ON graph_edges(from_id);
CREATE INDEX idx_edges_to ON graph_edges(to_id);
CREATE INDEX idx_edges_type ON graph_edges(edge_type);
```

**Columns**:
- `id`: Auto-increment primary key
- `from_id`: Foreign key to `graph_entities.id`
- `to_id`: Foreign key to `graph_entities.id`
- `edge_type`: Relationship type
- `data`: JSON metadata

**Observed Edge Types**:
- `DEFINES` — Symbol defined in file (file → symbol)
- `REFERENCES` — Symbol references another symbol (symbol → symbol)

**Data Field Content** (JSON):
- For DEFINES: May contain span information
- For REFERENCES: `{"byte_start": N, "byte_end": N}`

#### Sample Queries

**Count entities by kind**:
```sql
SELECT kind, COUNT(*) FROM graph_entities GROUP BY kind;
```

**Sample output**:
```
File|337
Reference|4497
Symbol|5021
```

**Count edges by type**:
```sql
SELECT edge_type, COUNT(*) FROM graph_edges GROUP BY edge_type;
```

**Sample output**:
```
DEFINES|5021
REFERENCES|4497
```

**Get all symbols in a file**:
```sql
SELECT ge.id, ge.name, json_extract(ge.data, '$.kind') as symbol_kind
FROM graph_entities ge
WHERE ge.kind = 'Symbol'
  AND ge.file_path LIKE '%/lib.rs'
ORDER BY ge.name ASC;
```

**Find references to a symbol**:
```sql
SELECT ref_from.name as referencing_file, ref_from.file_path
FROM graph_entities ref_from
JOIN graph_edges e ON ref_from.id = e.from_id
JOIN graph_entities ref_to ON e.to_id = ref_to.id
WHERE ref_to.kind = 'Symbol'
  AND ref_to.name = 'symbol_name'
  AND e.edge_type = 'REFERENCES'
ORDER BY ref_from.file_path ASC;
```

**Get symbol definition**:
```sql
SELECT ge.file_path, json_extract(ge.data, '$.byte_start') as start, json_extract(ge.data, '$.byte_end') as end
FROM graph_entities ge
WHERE ge.kind = 'Symbol'
  AND ge.name = 'symbol_name';
```

---

## sqlite3 CLI (Command Reference)

### Basic Usage

```bash
sqlite3 [OPTIONS] [FILENAME [SQL...]]
```

### Useful Options for OdinCode

| Option | Description |
|--------|-------------|
| `--readonly` | Open database read-only |
| `--header` | Turn column headers on |
| `--column` | Set output mode to 'column' (aligned) |
| `--json` | Set output mode to JSON |
| `--batch` | Force batch I/O (non-interactive) |

### Common Queries for SQLiteGraph

**Get schema**:
```bash
sqlite3 codegraph.db ".schema"
```

**List tables**:
```bash
sqlite3 codegraph.db ".tables"
```

**Get table schema**:
```bash
sqlite3 codegraph.db ".schema graph_entities"
```

**Run single query**:
```bash
sqlite3 codegraph.db "SELECT COUNT(*) FROM graph_entities WHERE kind='Symbol';"
```

**Run query with formatted output**:
```bash
sqlite3 --header --column codegraph.db "SELECT kind, COUNT(*) as count FROM graph_entities GROUP BY kind;"
```

**Export query to CSV**:
```bash
sqlite3 codegraph.db ".mode csv" ".output symbols.csv" "SELECT * FROM graph_entities WHERE kind='Symbol';"
```

### Batch SQL File

Create `queries.sql`:
```sql
.mode column
.headers on
SELECT kind, COUNT(*) as count FROM graph_entities GROUP BY kind;
SELECT edge_type, COUNT(*) as count FROM graph_edges GROUP BY edge_type;
```

Run:
```bash
sqlite3 codegraph.db < queries.sql
```

---

## Integration Examples

### Typical Workflow

**1. Start Magellan**:
```bash
magellan watch --root . --db codegraph.db --debounce-ms 500 > /tmp/magellan.log 2>&1 &
```

**2. Trigger indexing**:
```bash
find src -name "*.rs" -exec touch {} \; && sleep 5
```

**3. Query codebase**:
```bash
# Count symbols
sqlite3 codegraph.db "SELECT COUNT(*) FROM graph_entities WHERE kind='Symbol';"

# Find all references to a symbol
sqlite3 --header codegraph.db <<EOF
SELECT ge_from.file_path
FROM graph_entities ge_from
JOIN graph_edges e ON ge_from.id = e.from_id
JOIN graph_entities ge_to ON e.to_id = ge_to.id
WHERE ge_to.name = 'function_name'
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

---

## Error Handling

### Magellan Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `Permission denied` | Cannot read source files | Check file permissions |
| `Database locked` | Another process using DB | Stop other processes |
| `No such file or directory` | Root directory doesn't exist | Create directory first |

### Splice Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `Symbol '<name>' not found` | Symbol doesn't exist in file | Check symbol name and spelling |
| `Cargo check failed` | Not in Cargo workspace | Run from project root with Cargo.toml |
| `Invalid symbol kind` | Wrong --kind value | Use: function, struct, enum, trait, impl |
| `File not found` | Target or replacement file missing | Check file paths |

### SQLiteGraph Errors

| Error | Cause | Resolution |
|-------|-------|------------|
| `database is locked` | Magellan is writing to DB | Wait or use read-only mode |
| `no such table: graph_entities` | Not a SQLiteGraph database | Check DB path |
| `malformed JSON` | Corrupted data field | Re-index with Magellan |

---

## Version Information

Check versions:
```bash
# Magellan (no --version, use error output)
magellan watch 2>&1 | head -1

# Splice
splice --version

# SQLite
sqlite3 --version
```

---

*Last Updated: 2025-12-24*
*Source: Direct observation of tool behavior*
*No assumptions, no guessing — actual tool output only*
