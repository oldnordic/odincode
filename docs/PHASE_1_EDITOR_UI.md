# Phase 1 — Editor UI

**Phase**: 1 — Editor UI
**Status**: PLANNING ONLY — NO CODE AUTHORIZED
**Type**: Terminal User Interface Specification
**Dependency**: Phase 0 COMPLETE (0.1 → 0.6)
**Date**: 2025-12-24

---

## SECTION 1 — PURPOSE & BOUNDARIES

### What the UI Does

The Editor UI is a **deterministic surface** over existing tools. It provides:

- **Visualization** of tool outputs (files, diagnostics, evidence)
- **Invocation** of tools with explicit arguments
- **Navigation** of codebase structure (files, symbols, references)
- **Query** of execution memory (evidence from Phase 0.6)

The UI is a **window**, not a brain. It displays what tools return. It does not interpret.

### What the UI Explicitly Does Not Do

The UI must NOT:

- **NO suggestions** — No "did you mean", no auto-complete based on "intelligence"
- **NO auto-fix** — No automatic application of fixes
- **NO inference** — No "this probably caused that", no "likely fix is..."
- **NO rankings** — No "most relevant", no "confidence scores"
- **NO background indexing** — No async file watching, no hidden refresh
- **NO policy enforcement** — No "you shouldn't do this", no warnings beyond tool output
- **NO hidden state** — All state is visible or persisted in SQLite
- **NO caching of interpretations** — Cache file contents only; never cache "what this means"
- **NO speculative execution** — No "predictive" actions, no pre-computation

### Core Principle

**DETERMINISTIC DISPLAY** — The UI shows exactly what the tool returned, nothing more, nothing less.

---

## SECTION 2 — UI PRINCIPLES (DERIVED FROM PHASE 0)

### P1: Every Action is a Tool Call

User actions map 1:1 to tool functions from Phase 0:

| UI Action | Tool Function | Module |
|-----------|---------------|--------|
| Read file | `file_read(path)` | file_tools |
| Write file | `file_write(path, contents)` | file_tools |
| Search code | `file_search(pattern, root)` | file_tools |
| List symbols | `symbols_in_file(pattern)` | magellan_tools |
| Find references | `references_to_symbol_name(name)` | magellan_tools |
| Apply patch | `splice_patch(args)` | splice_tools |
| Check diagnostics | `lsp_check(path)` | lsp_tools |
| Query evidence | Q1-Q8 functions | evidence_queries |

The UI adds NO new operations. It only provides a surface over existing tools.

### P2: Every Result is Evidence-Backed

Display elements must correspond to tool return values:

| Display Element | Evidence Source |
|----------------|-----------------|
| File tree | `file_glob()` + file existence checks |
| Code contents | `file_read()` |
| Symbol list | `symbols_in_file()` |
| Diagnostic list | `lsp_check()` |
| Execution history | `list_executions_by_tool()` |
| Failure history | `list_failures_by_tool()` |
| File execution history | `find_executions_by_file()` |

The UI invents NO data. All displayed information is traceable to a tool call.

### P3: No Hidden State

**Transient State** (UI only, lost on exit):
- Current selection (which file/symbol is focused)
- Scroll position
- Panel sizing
- Command history (for convenience, optional)

**Persistent State** (SQLite):
- All execution records (execution_log.db)
- All graph relationships (codegraph.db)
- File contents (on disk, not in UI)

**State That Must Not Exist**:
- "Likely fix" cache (interpreting evidence is NOT stored)
- "File risk score" (no derived metrics)
- "Action recommendations" (no policy layer)
- "User preferences" beyond display settings

### P4: Failures are Never Hidden

- Tool errors are displayed in full
- No "simplified" error messages
- No suppressing "noisy" diagnostics
- stderr is always shown (from ExecutionDetails.artifacts)

### P5: Deterministic Output

Given the same database state and same user action, the UI displays identical output:
- Sorted lists (no random ordering)
- Stable panel layouts
- Reproducible query results

---

## SECTION 3 — ARCHITECTURE OVERVIEW

### Process Model

```
┌─────────────────────────────────────────────────────────────┐
│                    OdinCode Binary (Single Process)         │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │   ratatui   │───→│  Tool Layer │───→│  External   │     │
│  │     UI      │    │  (Phase 0)  │    │   Tools     │     │
│  │             │←───│             │←───│ (splice,    │     │
│  │  (Terminal  │    │             │    │  magellan,  │     │
│  │   Surface)  │    │             │    │  cargo)     │     │
│  └─────────────┘    └─────────────┘    └─────────────┘     │
│         │                   │                                 │
│         │                   │                                 │
│         ↓                   ↓                                 │
│  ┌─────────────┐    ┌─────────────┐                         │
│  │   Transient │    │  Persistent │                         │
│  │  UI State   │    │  Databases  │                         │
│  │ (selection, │    │  (exec log, │                         │
│  │  scrolling) │    │  codegraph) │                         │
│  └─────────────┘    └─────────────┘                         │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

**Key Characteristics**:
- **Single process** — No background threads
- **Synchronous** — Tool calls block UI; no async
- **No daemon** — Runs only when invoked
- **Direct tool execution** — No intermediary API

### Event Loop Model

```
┌─────────────────────────────────────────────────────────────┐
│                        Event Loop                           │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  1. READ INPUT  ←  stdin (keypress, command)                │
│       │                                                        │
│       ↓                                                        │
│  2. PARSE COMMAND  ←  Map to tool function                   │
│       │                                                        │
│       ↓                                                        │
│  3. EXECUTE TOOL  ←  Call Phase 0 function                   │
│       │                                                        │
│       ↓                                                        │
│  4. CAPTURE OUTPUT  ←  Get tool result                       │
│       │                                                        │
│       ↓                                                        │
│  5. RENDER UI  ←  Update display via ratatui                 │
│       │                                                        │
│       └──→ Loop to 1                                         │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

**Ratatui Integration**:
- Terminal backend (crossterm)
- Event-driven (keystroke → action → render)
- Layout-based (panels with fixed ratios)
- No automatic refresh (only on state change)

### Data Flow

```
USER INPUT
   │
   ↓
┌──────────────┐
│  Parse UI    │  "What tool does the user want?"
│  Command     │
└──────────────┘
   │
   ↓
┌──────────────┐
│  Invoke Tool │  Call Phase 0 function with arguments
│  (Phase 0)   │
└──────────────┘
   │
   ├──────────────────────────────────────┐
   │                                      │
   ↓                                      ↓
┌──────────────┐                    ┌──────────────┐
│  Tool Result │                    │ Auto-Record  │
│  (Returned)  │                    │ (ExecutionDb)│
└──────────────┘                    └──────────────┘
   │
   ↓
┌──────────────┐
│ Render Panel │  "Display what the tool returned"
│  Update      │
└──────────────┘
   │
   ↓
USER SEES RESULT
```

**Key Point**: The UI does NOT interpret results. It renders them as-is.

---

## SECTION 4 — CORE UI PANELS (MINIMAL SET)

### Panel 1: File Explorer

**Purpose**: Navigate filesystem using file_tools

**Data Source**:
- `file_glob("**/*", project_root)` for listing
- File existence checks for validity

**Display**:
- Tree structure (directories nested)
- File names only (NO icons, NO decorations)
- Full path in status bar on selection

**Interactions**:
- Enter: Open file in Code View
- Expand/Collapse directories
- Filter: Accept glob pattern (via command input)

**What It Does NOT Show**:
- NO file "type" indicators (no icons)
- NO git status (untracked, modified — not our concern)
- NO "recent files" (use evidence queries instead)

**Evidence Integration**:
- Optional: Show execution count badge from Q4 (find_executions_by_file)
- Badge is literal count, NOT a "hotness" indicator

---

### Panel 2: Code View

**Purpose**: Display file contents (read-only in Phase 1)

**Data Source**:
- `file_read(path)` for contents
- `lsp_check(project_root)` for diagnostics overlay

**Display**:
- Raw text with syntax highlighting (optional, via syntect)
- Line numbers (deterministic: 1-based)
- Diagnostics inline (error/warning markers at line_start)

**Interactions**:
- Scroll: Navigate file
- Select line: Copy to clipboard (optional)
- "Jump to diagnostic": Focus Diagnostics Panel

**Constraints**:
- **Read-only** in Phase 1 (no inline editing)
- Editing will be via explicit splice_patch/file_write commands

**What It Does NOT Show**:
- NO inline hints/suggestions
- NO "reachable" indicators
- NO "unused" dimming (beyond what lsp_check returns)

---

### Panel 3: Action Console

**Purpose**: Explicit tool invocation and result display

**Data Source**:
- User input for command/arguments
- Tool results for output

**Display**:
```
Command: splice_patch --file src/lib.rs --symbol foo --with patches/new.rs

Executing...

Result: Success
Changed files: src/lib.rs
Duration: 234ms

stdout:
[Patched] src/lib.rs: Replaced symbol 'foo' (function)
```

**Interactions**:
- Command input: Free-form text (parsed to tool + args)
- History: Up/Down arrow (local to session, NOT persisted)
- Help: "?" or "help" shows available commands

**Supported Commands** (Phase 1):
```
file_read <path>
file_write <path>
file_create <path>
file_search <pattern> [--root <dir>]
file_glob <pattern>

splice_patch --file <path> --symbol <name> --kind <kind> --with <file>
splice_plan --file <plan.json>

magellan_status
magellan_symbols <file>
magellan_refs_to <symbol>
magellan_refs_from <file> <symbol>

lsp_check [--path <project_root>]

evidence_list <tool> [--since <ts>] [--until <ts>] [--limit <n>]
evidence_failures <tool> [--since <ts>] [--limit <n>]
evidence_code <code> [--limit <n>]
evidence_file <path> [--limit <n>]
evidence_details <execution_id>
evidence_latest <path>
evidence_recurring <threshold> [--since <ts>]
evidence_prior <code> [--file <path>]
```

**What It Does NOT Do**:
- NO command auto-completion (beyond basic tab-complete of tool names)
- NO "smart" argument suggestions
- NO "recommended next actions"

---

### Panel 4: Evidence Panel

**Purpose**: Display execution history from Phase 0.6 queries

**Data Source**:
- EvidenceDb queries (Q1-Q8)
- execution_log.db + codegraph.db

**Display Modes** (toggle via command):

**Mode 1: By Tool** (Q1)
```
Tool: splice_patch
Executions: 3

┌─────────────────────────────────────────────────────────────┐
│ 2025-12-24 10:23:45  SUCCESS  234ms  src/lib.rs             │
│ 2025-12-24 09:15:32  FAILED  178ms  src/main.rs             │
│   Error: Symbol 'bar' not found                              │
│ 2025-12-23 16:45:01  SUCCESS  312ms  src/lib.rs             │
└─────────────────────────────────────────────────────────────┘
```

**Mode 2: Failures Only** (Q2)
```
Tool: splice_patch
Failures: 1

┌─────────────────────────────────────────────────────────────┐
│ 2025-12-24 09:15:32  EXIT 1  src/main.rs                   │
│   Error: Symbol 'bar' not found                              │
└─────────────────────────────────────────────────────────────┘
```

**Mode 3: By File** (Q4)
```
File: src/lib.rs
Executions: 7

┌─────────────────────────────────────────────────────────────┐
│ 2025-12-24 10:23:45  splice_patch  SUCCESS  AFFECTED       │
│ 2025-12-24 09:30:12  lsp_check      SUCCESS  EXECUTED_ON    │
│ 2025-12-24 09:15:00  file_read      SUCCESS  EXECUTED_ON    │
└─────────────────────────────────────────────────────────────┘
```

**Mode 4: Recurring Diagnostics** (Q7)
```
Threshold: 3 occurrences

┌─────────────────────────────────────────────────────────────┐
│ E0425  src/lib.rs  7 occurrences  (2025-12-20 → 2025-12-24) │
│ E0382  src/main.rs  4 occurrences  (2025-12-22 → 2025-12-24) │
└─────────────────────────────────────────────────────────────┘
```

**Interactions**:
- Select execution: Show details via Q5
- Filter: By tool, file, time range
- Export: Copy execution_id for reference

**What It Does NOT Show**:
- NO "fix rate" calculations (not evidence)
- NO "risk scores"
- NO "recommended actions"

---

### Panel 5: Diagnostics Panel

**Purpose**: Display compiler diagnostics from lsp_tools + evidence context

**Data Source**:
- `lsp_check(project_root)` for current diagnostics
- Q3 (find_executions_by_diagnostic_code) for historical context
- Q7 (get_recurring_diagnostics) for recurrence

**Display**:
```
┌─────────────────────────────────────────────────────────────┐
│ ERROR  E0425  src/lib.rs:23                                 │
│        cannot find value `x` in this scope                   │
│        Seen 7 times since 2025-12-20                         │
│                                                              │
│ ERROR  E0382  src/main.rs:45                                │
│        use of moved value: `data`                            │
│        Seen 4 times since 2025-12-22                         │
│                                                              │
│ WARN   dead_code  src/util.rs:12                            │
│        function `unused_helper` is never used                │
└─────────────────────────────────────────────────────────────┘
```

**Interactions**:
- Select diagnostic: Jump to location in Code View
- Show evidence: "Prior fixes" (Q8) for this diagnostic code

**What It Does NOT Show**:
- NO "quick fix" buttons (not Phase 1)
- NO "auto-fix all" option
- NO suppression suggestions

---

### Panel Layout (Initial State)

```
┌─────────────────────────────────────────────────────────────┐
│  ODINCODE                                                    │
├──────────────┬──────────────────────────────────────────────┤
│              │  Code View                    Diagnostics     │
│  File        │  ┌─────────────────────────────┐  ┌──────────┐ │
│  Explorer    │  │  1  pub fn foo() {          │  │ E0425    │ │
│              │  │  2      let x = bar();      │  │ src/lib  │ │
│  src/        │  │  3      x + 1               │  │          │ │
│    lib.rs ✓  │  │  4  }                       │  │ E0382    │ │
│    main.rs   │  │  5                           │  │ src/main │ │
│    util.rs   │  │  6  pub fn baz() {          │  │          │ │
│  tests/      │  │  7      ...                 │  │          │ │
│    test.rs   │  │  8  }                       │  │          │ │
│              │  │                             │  │          │ │
│              │  └─────────────────────────────┘  └──────────┘ │
├──────────────┴──────────────────────────────────────────────┤
│  Action Console                                              │
│  > splice_patch --file src/lib.rs --symbol foo               │
│                                                              │
│  Result: Success. Changed files: src/lib.rs                 │
└─────────────────────────────────────────────────────────────┘
│  Evidence: [Tool] [File] [Failures] [Recurring]             │
└─────────────────────────────────────────────────────────────┘
```

**Keybindings** (initial set, configurable):
- `Tab`: Cycle panel focus
- `Ctrl+E`: Focus Evidence Panel
- `Ctrl+D`: Focus Diagnostics Panel
- `Ctrl+L`: Focus File Explorer
- `Ctrl+C`: Focus Action Console
- `Ctrl+Q`: Quit (after confirmation)

---

## SECTION 5 — COMMAND MODEL

### Command Entry

Commands are entered via the Action Console:

```
> <tool_name> <arg1> <arg2> ... <argN>
```

**Parsing Rules**:
1. First token = tool_name (must match Phase 0 function)
2. Remaining tokens = arguments (tool-specific format)
3. Flags = `--flag_name value` or `--flag_name` (boolean)

**Example Commands**:
```
> file_read src/lib.rs
> splice_patch --file src/lib.rs --symbol foo --with patches/new.rs
> evidence_list splice_patch --since 1735036800000 --limit 10
> lsp_check
```

### Argument Supply

**Positional Arguments**:
```
file_read <path>              # path is positional
```

**Flag Arguments**:
```
splice_patch --file <path> --symbol <name> --with <file>
```

**Optional Arguments**:
```
evidence_list <tool> [--since <ts>] [--until <ts>] [--limit <n>]
```

**Complex Arguments** (JSON):
```
splice_plan --file plan.json
```

### Result Rendering

**Success Result**:
```
> splice_patch --file src/lib.rs --symbol foo --with new.rs

✓ SUCCESS
Execution ID: 550e8400-e29b-41d4-a716-446655440000
Duration: 234ms
Changed files:
  - src/lib.rs

stdout:
[Patched] src/lib.rs: Replaced symbol 'foo'
```

**Error Result**:
```
> splice_patch --file src/lib.rs --symbol nonexistent --with new.rs

✗ FAILED
Execution ID: 550e8400-e29b-41d4-a716-446655440001
Duration: 45ms
Exit code: 1

Error: Symbol 'nonexistent' not found in src/lib.rs

stderr:
Error: Symbol lookup failed
  Caused by: No symbol named 'nonexistent' in file
```

**Query Result (Evidence)**:
```
> evidence_failures splice_patch --limit 5

Tool: splice_patch
Showing 5 most recent failures

┌─────────────────────────────────────────────────────────────┐
│ 2025-12-24 09:15:32  EXIT 1  src/main.rs                   │
│    Symbol 'bar' not found                                   │
│                                                              │
│ 2025-12-23 14:22:10  EXIT 1  src/lib.rs                    │
│    Symbol 'baz' not found                                   │
│                                                              │
│ 2025-12-23 11:05:45  EXIT 1  src/util.rs                   │
│    Invalid kind: 'struct' (expected 'fn')                   │
└─────────────────────────────────────────────────────────────┘
```

### Failure Display

**Rule**: Failures are NEVER hidden.

All of the following MUST be displayed:
- Tool error messages (verbatim)
- Exit codes
- stderr output
- Diagnostic details (file, line, code, message)

**What is NOT done**:
- NO "simplified" error messages
- NO suppressing "noisy" errors
- NO grouping "similar" errors (user sees all occurrences)

---

## SECTION 6 — STATE MANAGEMENT

### Transient State (UI Only)

Lost on exit; NOT persisted:

| State | Purpose | Example |
|-------|---------|---------|
| Current panel focus | Which panel receives input | "File Explorer selected" |
| Selection | Which item is focused | "src/lib.rs line 23" |
| Scroll position | Viewport offset | "Code view scrolled to line 50" |
| Panel sizes | Layout ratios | "Code View 60%, File Explorer 20%" |
| Command history | Convenience for re-running | "Up arrow shows previous command" |
| Query filters | Current evidence query | "Showing failures since T" |

**Implementation**: In-memory structs only; no serialization.

### Persistent State (Databases)

Survives exit; managed by SQLite:

| State | Database | Access Pattern |
|-------|----------|----------------|
| Execution records | execution_log.db | Write: ExecutionDb; Read: EvidenceDb |
| Graph entities | codegraph.db | Write: ExecutionDb/Magellan; Read: EvidenceDb |
| Graph edges | codegraph.db | Write: ExecutionDb/Magellan; Read: EvidenceDb |
| Artifacts | execution_log.db | Write: ExecutionDb; Read: EvidenceDb |

**Implementation**: rusqlite queries; NO caching in UI layer.

### State That Must Never Exist

These are FORBIDDEN (violate "no intelligence" principle):

| Forbidden State | Why Forbidden | Correct Approach |
|-----------------|---------------|------------------|
| "File risk score" | Not evidence-based | Use Q7 (recurring diagnostics) |
| "Likely fix cache" | Requires inference | Use Q8 (temporal adjacency), interpret yourself |
| "User preferences" for behavior | Policy layer | No policy in Phase 1 |
| "Interpretation cache" | Hidden state | Always re-run Q1-Q8 |
| "Recommended next action" | Intelligence layer | User decides; UI only shows evidence |
| "Action effectiveness score" | Requires causality claim | Show Q5 details; let user decide |

### State Transitions

```
┌─────────────────────────────────────────────────────────────┐
│                     STATE TRANSITIONS                       │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  START (no state)                                            │
│    │                                                          │
│    ↓                                                          │
│  LOAD DBs (open execution_log.db, codegraph.db)              │
│    │                                                          │
│    ├─→ Error: Show "Database not found; run magellan first" │
│    │                                                          │
│    ↓                                                          │
│  READY (db_root loaded, empty selection)                      │
│    │                                                          │
│    ↓                                                          │
│  INTERACT (user navigates, runs commands)                     │
│    │                                                          │
│    ├─→ Each command: ExecutionDb::record()                  │
│    │                                                          │
│    ↓                                                          │
│  QUERY (user requests evidence)                              │
│    │                                                          │
│    ├─→ EvidenceDb::{Q1-Q8}()                                │
│    │                                                          │
│    ↓                                                          │
│  DISPLAY (render results to panels)                           │
│    │                                                          │
│    └──→ Loop to INTERACT                                     │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

---

## SECTION 7 — EXTENSION POINTS (FUTURE, NON-BINDING)

### E1: Embeddings Integration Point (Phase 2+)

**Where it COULD attach**: Between Tool Layer and UI Render

```
CURRENT:
┌─────────────┐    ┌─────────────┐
│  Tool Layer │───→│   UI Render │
└─────────────┘    └─────────────┘

FUTURE (NOT Phase 1):
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│  Tool Layer │───→│  RAG/Embed  │───→│   UI Render │
│             │    │  (Phase 2)  │    │             │
└─────────────┘    └─────────────┘    └─────────────┘
```

**What it WOULD enable** (NOT Phase 1):
- Semantic search ("code similar to X")
- Code-as-data queries ("all functions modifying value X")
- Embedding-based navigation

**Phase 1 Constraint**: UI MUST work without embeddings.

### E2: Policy Layer Attachment Point (Phase 3+)

**Where it COULD attach**: Between User Input and Tool Execution

```
CURRENT:
┌─────────────┐    ┌─────────────┐
│ User Command│───→│  Tool Layer │
└─────────────┘    └─────────────┘

FUTURE (NOT Phase 1):
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│ User Command│───→│  Policy     │───→│  Tool Layer │
│             │    │  (Phase 3)  │    │             │
└─────────────┘    └─────────────┘    └─────────────┘
```

**What it WOULD enable** (NOT Phase 1):
- "Block this action" (policy enforcement)
- "Suggest alternative" (recommendation)
- "Require confirmation" (safety checks)

**Phase 1 Constraint**: Commands execute directly; NO policy layer.

### E3: Agent Integration Point (Phase 4+)

**Where it COULD attach**: As alternative input source

```
CURRENT:
┌─────────────┐
│   Human     │───→ Commands ──→ Tool Layer
└─────────────┘

FUTURE (NOT Phase 1):
┌─────────────┐
│   Human     │───→ Commands ──┐
└─────────────┘                │
                               ├─→ Tool Layer
┌─────────────┐                │
│    Agent    │───→ Commands ──┘
│ (Phase 4)   │
└─────────────┘
```

**What it WOULD enable** (NOT Phase 1):
- Automated tool selection
- Multi-step workflows
- Agent-driven refactoring

**Phase 1 Constraint**: Human only; NO agent integration.

---

## SECTION 8 — ACCEPTANCE CRITERIA (FOR PHASE 1 IMPLEMENTATION)

### A1: Tool Surface Coverage

The UI MUST expose all Phase 0 tools:

- [ ] file_read, file_write, file_create invocable via command
- [ ] file_search, file_glob invocable via command
- [ ] splice_patch, splice_plan invocable via command
- [ ] magellan_status, magellan_symbols, magellan_refs_* invocable
- [ ] lsp_check invocable via command
- [ ] All Q1-Q8 evidence queries invocable via command

### A2: Display Completeness

The UI MUST display all tool outputs:

- [ ] File contents displayed fully (no truncation)
- [ ] Diagnostic details shown (code, level, message, location)
- [ ] Tool stdout displayed verbatim
- [ ] Tool stderr displayed verbatim
- [ ] Execution summaries show all fields (timestamp, success, duration, etc.)
- [ ] Evidence queries show all result fields

### A3: Evidence Integration

The UI MUST integrate Phase 0.6 evidence:

- [ ] Q1 (list_executions_by_tool) viewable
- [ ] Q2 (list_failures_by_tool) viewable
- [ ] Q3 (find_executions_by_diagnostic_code) viewable
- [ ] Q4 (find_executions_by_file) viewable
- [ ] Q5 (get_execution_details) viewable
- [ ] Q6 (get_latest_outcome_for_file) viewable
- [ ] Q7 (get_recurring_diagnostics) viewable
- [ ] Q8 (find_prior_fixes_for_diagnostic) viewable

### A4: Deterministic Behavior

The UI MUST behave deterministically:

- [ ] Same inputs produce identical displays
- [ ] Lists are sorted (no random ordering)
- [ ] Panel layouts are stable
- [ ] Query results are reproducible

### A5: Error Transparency

The UI MUST show all errors:

- [ ] Tool errors displayed verbatim
- [ ] Exit codes shown for all tool executions
- [ ] stderr always displayed (never hidden)
- [ ] Database errors shown explicitly
- [ ] NO "simplified" error messages

### A6: No Inference

The UI MUST NOT include:

- [ ] NO suggestions or recommendations
- [ ] NO auto-fix functionality
- [ ] NO "likely cause" language
- [ ] NO confidence scores or rankings
- [ ] NO policy enforcement ("you shouldn't do this")
- [ ] NO background operations (indexing, watching)
- [ ] NO hidden state (risk scores, interpretations)

### A7: Architecture Compliance

The implementation MUST:

- [ ] Use ratatui for terminal UI
- [ ] Single process (no background threads)
- [ ] Synchronous tool calls (no async)
- [ ] Direct Phase 0 function calls (no wrapper API)
- [ ] SQLite for persistent state (no custom format)
- [ ] Files ≤ 300 LOC per module

### A8: Test Coverage

Tests MUST cover:

- [ ] Unit tests for command parsing
- [ ] Unit tests for panel rendering logic
- [ ] Integration tests for tool invocation
- [ ] Integration tests for evidence query display
- [ ] Tests use real tools (no mocks)
- [ ] Tests pass with minimal database fixtures

---

## SECTION 9 — IMPLEMENTATION MODULE STRUCTURE (IF AUTHORIZED)

```
src/ui/
├── mod.rs              # Module exports (~50 LOC)
├── app.rs              # Main app struct, event loop (~300 LOC)
├── panels/
│   ├── mod.rs          # Panel exports (~50 LOC)
│   ├── file_explorer.rs # File tree panel (~300 LOC)
│   ├── code_view.rs    # Code display panel (~300 LOC)
│   ├── console.rs      # Action console (~300 LOC)
│   ├── evidence.rs     # Evidence panel (~300 LOC)
│   └── diagnostics.rs  # Diagnostics panel (~300 LOC)
├── commands/
│   ├── mod.rs          # Command parser (~100 LOC)
│   └── dispatcher.rs   # Map commands to tool calls (~200 LOC)
└── state/
    ├── mod.rs          # State exports (~50 LOC)
    └── transient.rs    # UI-only state (~200 LOC)
```

**Total LOC**: ~2,450 (all modules ≤ 300 LOC)

---

## SECTION 10 — NON-GOALS REITERATION

### What Phase 1 Will NOT Do

❌ **NO editor features** — No inline editing, no multi-cursor, no macros
❌ **NO IDE features** — No debuggers, no build integration, no test runners
❌ **NO intelligence** — No code completion, no suggestions, no refactoring assists
❌ **NO async operations** — All tool calls block the UI
❌ **NO background threads** — Single-threaded event loop only
❌ **NO plugin system** — No extensibility beyond defined panels
❌ **NO customization** — No themes, no keybinding remaps (Phase 1)
❌ **NO multi-window** — Single terminal window only
❌ **NO remote operations** — No SSH, no LSP, no network I/O

---

## TERMINATION

**Phase**: 1 — Editor UI (PLANNING ONLY)
**Status**: PLANNING COMPLETE — Awaiting Acceptance or Revisions
**Date**: 2025-12-24

**DELIVERABLE**: This document (`docs/PHASE_1_EDITOR_UI.md`)

**NEXT STEP** (IF AUTHORIZED):
- User reviews and approves/plans changes
- Then: "AUTHORIZE Phase 1 — Editor UI (IMPLEMENTATION)"
- Then: TDD implementation begins

**IF REJECTED**:
- User specifies changes required
- Revise document per feedback
- Resubmit for acceptance

---

*Last Updated: 2025-12-24*
*Status: PLANNING ONLY — NO CODE AUTHORIZED*
*Phase: 1 — Editor UI*
*Purpose: Terminal user interface over Phase 0 tools*
