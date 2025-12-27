# OdinCode

**A deterministic tool substrate for LLM-powered refactoring.**

OdinCode is NOT an AI system. It provides boring, factual tools that an LLM MUST use instead of guessing about the filesystem, codebase structure, or compiler output.

## Philosophy

**ZERO GUESSING** — Every operation grounded in facts from real tools.

- **NO INTELLIGENCE** — We build tools, not AI
- **NO GUESSING** — Every operation grounded in facts
- **NO EDITOR UI** — Console tool layer only
- **NO ASYNC** — Synchronous, deterministic execution
- **NO MOCKS** — Real tools, real filesystem, real compiler

## Status

**Phase**: 0 — Tool Substrate (COMPLETE)
**Test Coverage**: 986 integration tests passing
**Max LOC**: 300 LOC per module (all files comply)

## Capabilities

### File Operations (`file_tools`)
- `file_read(path)` — Read file contents
- `file_write(path, contents)` — Atomic overwrite with fsync
- `file_create(path, contents)` — Create if not exists
- `file_search(pattern, root)` — Ripgrep-powered search
- `file_glob(pattern, root)` — Glob pattern matching

### Refactoring (`splice_tools`)
- `splice_patch(file, symbol, kind, with)` — Span-safe symbol replacement
- `splice_plan(plan_file)` — Multi-step refactoring plans

### Code Navigation (`magellan_tools`)
- Direct SQLiteGraph queries via `MagellanDb`
- Symbol lookup, reference finding, graph traversal

### Diagnostics (`lsp_tools`)
- `lsp_check(path)` — Compiler diagnostics via `cargo check`

## Building

```bash
# Clone the repository
git clone https://github.com/oldnordic/odincode.git
cd odincode

# Build the binary
cargo build --release

# Run tests
cargo test

# Install (optional)
cp target/release/odincode ~/.local/bin/
```

## Requirements

- Rust 1.75+
- External tools (optional, for full functionality):
  - `magellan` — Codebase indexer (tree-sitter → SQLiteGraph)
  - `splice` — Span-safe refactoring
  - `cargo` — For lsp_check

## Documentation

See `docs/` for detailed documentation:

- `CONTRACT.md` — Phase 0 scope (frozen)
- `TODO.md` — Phase tracking and completion status
- `EXTERNAL_TOOLS_API.md` — External tool APIs
- `TOOL_MANUALS.md` — CLI reference

## License

GPL-3.0
