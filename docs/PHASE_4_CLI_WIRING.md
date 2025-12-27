# Phase 4 — End-to-End CLI Wiring Plan

**Phase**: 4 — CLI Wiring
**Status**: PLANNING ONLY — NO CODE AUTHORIZED
**Type**: Architectural Wiring Specification
**Dependency**: Phases 0–3 COMPLETE and LOCKED
**Date**: 2025-12-24

---

## 1. Scope & Non-Goals (LOCKED)

### In Scope (What Phase 4 Wiring Does)

✅ **Wire existing modules into a single CLI application**
- Connect Phase 1 (UI), Phase 2 (LLM), Phase 3 (Executor), Phase 0.5 (Execution Memory), Phase 0.6 (Evidence Queries)
- Define entry points for different CLI modes

✅ **Define end-to-end "happy path"**
- User inputs goal → LLM generates plan → User approves plan → Executor executes → Results logged → Evidence queryable → UI displays results

✅ **Define failure paths for all error scenarios**
- LLM plan invalid → rejection
- Preconditions fail → execution halts
- Tool invocation fails → failure logged, execution stops
- Evidence write fails → SQLiteGraph best-effort, SQLite must succeed
- UI render errors — displayed to user, logged

### Out of Scope (What Phase 4 Wiring Explicitly Does Not Do)

❌ **NO new modules** — Only composition of existing Phases 0–3
❌ **NO new tools** — Only 11 whitelisted tools from Phase 0
❌ **NO async** — All execution remains synchronous
❌ **NO background threads** — Single-threaded execution only
❌ **NO autonomous workflows** — Every action requires user trigger
❌ **NO policy layer** — Preconditions only, no behavioral rules
❌ **NO retries or self-healing** — Failure is terminal
❌ **NO learning or adaptation** — Stateless execution
❌ **NO new prompts** — Use prompts defined in Phase 2 contracts
❌ **NO network I/O** — All operations are local
❌ **NO LLM API integration** — LLM integration is Phase 2; Phase 4 just wires it

### Core Principle

**COMPOSITION OVER INNOVATION** — Phase 4 is the "glue code" that makes existing modules work together end-to-end.

---

## 2. Runtime Entry Points

### CLI Modes (Minimal Set)

The `odincode` binary supports multiple invocation modes:

```
odincode [options] <mode> [mode-args]

MODES:
  (no mode)    → TUI mode (Phase 1), requires db_root
  tui           → TUI mode (explicit), requires db_root
  plan          → CLI-only planning mode: generate plan and exit
  execute       → CLI-only execution mode: load plan and execute
  evidence      → CLI-only evidence query mode: query and exit

OPTIONS:
  --db-root <path>  Database root (default: current directory)
  --plan-file <file> Plan file path (for 'execute' mode)
  --json            Output JSON (for scripting and integration)

FLAGS:
  --version       Show version and exit
  --help          Show help and exit
```

### Entry Point Specifications

#### Mode 1: TUI (default/interactive)
```
odincode [--db-root <path>]
```
- Starts Phase 1 ratatui TUI
- Requires `db_root` with `codegraph.db` (or auto-creates `execution_log.db`)
- All features available via command interface
- Supports `:plan <goal>` command to invoke LLM planning
- Supports `:execute <plan_id>` command to execute stored plans

**Exit codes**: 0 (clean quit), 1 (error)

#### Mode 2: plan
```
odincode plan --db-root <path> "<user goal>"
```
- Invokes Phase 2 LLM planner to generate plan from natural language goal
- Writes plan to `plans/<plan_id>.json` in db_root
- Outputs plan summary to stdout (plain text or JSON)
- Does NOT execute the plan
- Requires `codegraph.db` for evidence queries

**Exit codes**: 0 (valid plan generated), 1 (invalid input/LLM failure), 2 (evidence unavailable)

#### Mode 3: execute
```
odincode execute --db-root <path> --plan-file <plan>
odincode execute --db-root <path> --plan-file plans/<plan_id>.json
```
- Loads plan from JSON file
- Creates `ApprovedPlan` (auto-approved for CLI mode)
- Executes plan via Phase 3 Executor
- Logs results to execution memory
- Outputs results to stdout (plain text or JSON)
- Requires `codegraph.db` (for MagellanDb tools)

**Exit codes**: 0 (all steps succeeded), 1 (execution failed), 2 (db error)

#### Mode 4: evidence
```
odincode evidence --db-root <path> <query> [query-args...]
odincode evidence --db-root <path> Q1 <tool> [options]
odincode evidence --db_root <path> Q4 <path> [options]
```
- Invokes Phase 0.6 evidence queries
- Outputs results to stdout (JSON by default)
- Does NOT modify any state

**Queries**:
- `Q1` - list_executions_by_tool
- `Q2` - list_failures_by_tool
- `Q3` - find_executions_by_diagnostic_code
- `Q4` - find_executions_by_file
- `Q5` - get_execution_details
- `Q6` - get_latest_outcome_for_file
- `Q7` - get_recurring_diagnostics
- `Q8` - find_prior_fixes_for_diagnostic

**Exit codes**: 0 (query succeeded, may return empty), 1 (query failed), 2 (db not found)

### db_root Resolution

```
Priority:
1. --db-root <path> flag (highest)
2. $ODINCODE_HOME env var (if set, use $ODINCODE_HOME/db)
3. Current directory "." (default)

Inside db_root:
- execution_log.db — auto-created if missing
- codegraph.db — must exist (error if missing)
- plans/ — plan JSON files stored here
```

---

## 3. Component Composition Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         OdinCode CLI (single binary)                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  TUI Mode (src/main.rs)                                             │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │   ratatui   │  │  Command    │  │  Action     │  │   Evidence   │  │   │
│  │  │     TUI     │  │  Parser     │  │  Console    │  │     Panel    │  │   │
│  │  │   (Phase 1) │  │ (ui/input) │ │ (ui/state)  │  │ (ui/evidence)│  │   │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬───┘  └──────┬──────┘  │   │
│  │         │                   │                │              │      │   │
│  │         └───────────────────┴──────────────────────────────────────────┘      │   │
│  │                             │                                          │   │
│  │  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  │  Command Dispatcher (ui/input.rs)                                  │   │
│  │  │  ┌──────────────────────────────────────────────────────────────┐   │   │
│  │  │  │  :plan "<goal>"                                            │   │   │
│  │  │  │    └─→ SessionContext (llm/session.rs)                   │   │   │
│  │  │  │         │                                                     │   │
│  │  │  │  │  ┌────▼───────────────┐                                      │   │   │
│  │  │  │  │  │llm/client.rs     │  ← LLM API call (external)          │   │   │
│  │  │  │  │  └────┬───────────────┘                                      │   │   │
│  │  │  │  │       │                                                       │   │   │
│  │  │  │  │       └─→ parse_plan(), validate_plan() (llm/planner.rs)      │ │   │
│  │  │  │  │             │                                                     │   │   │
│  │  │  │  │             └─→ Plan → serialize → plans/<plan_id>.json      │ │   │
│  │  │  │ │                                                                      │   │
│  │  │  │  │  ┌────▼───────────────────────┐                                  │   │   │
│  │  │  │  │  │ :execute <plan_id>      │                                  │   │   │
│  │  │  │  │  │    └─→ Executor (execution_engine/executor.rs)                │   │
│  │  │  │  │  │             │                                             │   │   │
│  │  │  │  │  │             ├─→ ExecutionDb (execution_tools/db.rs)        │   │   │
│  │  │  │  │  │             │                                             │   │   │
│  │  │  │  │  │             └─→ tool_mapper() → Phase 0 tools          │   │   │
│  │  │  │  │ │                                                                      │   │
│  │  │  │  │  │  ┌────▼───────────────────────────┐                              │   │   │
│  │  │  │  │  │  │  :evidence Q1...     │                              │   │   │
│  │  │ │  │  │  │    └─→ EvidenceDb (evidence_queries/db.rs)                     │   │   │
││  │  │ │  │  │  │             │                                            │   │   │
│  │  │  │  │  │  │             └─→ query Q1-Q8 → results → UI       │   │   │
││  │  │  │  │  │                                                                      │   │   │
│  │ │ │ │ │ │  │  └────────────────────────────────────────────────────────────────┘   │
│  │ │ │ │ │ │ │  │                                                                         │
│  └───────────────────────────────────────────────────────────────────────────────┘   │
│                                                                             │
│  ┌───────────────────────────────────────────────────────────────────────────────┐   │
│  │  CLI-only Mode (main.rs:run_cli_mode)                                     │   │
│  │  │  ┌───────────────────────────────────────────────────────────────────┐ │   │
│  │  │  │  │  plan │  │  │ execute │  │  evidence │  │                         │ │ │ │ │   │   │
│  │  │  │  │  │ mode │  │  │   mode   │  │  │    mode    │  │                         │ │ │ │ │   │   │
│  │  │  │  │  │      │  │  │   │    │      │  │  │      │    │  │  │  │ │   │   │   │   │   │ │   │ │   │   │   │ │   │ │   │││ │ │ │ │ │   │ │ │ │ │ │   │ │   │ │ │ ││ │ │ │ │ │   │ │ │ │ │ │ │   │ │ │ │ │ || │ │ │ │ │ │ │ │ │ │ │ │   │   │   │ │ │ │ │ ││ │ │ │ │ │ │   │ │ │ │ │ │ │ │   │ │ │ │ || │ │ │ │ │ │   │   │ │   │ │ │ │   │   │ │ │ │ || │ │ │ │ │ │   │ │ │ │ │ │   │ │ │ │ │ │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │   │ │ │ │ │ │  │ │ │ │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │ │   │   │ │ │ │ │ │ │   │ │   │ || │ │ │ │ │ │ │   │   │ │ │ │ │ │ │   │ │ || │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │ │   │ │ │ │ │ │   │   │   │ │ || │ │ │ │ │ │ │ │   │ || │ │ │ │ │ │ │   │   │ │ │ │ │ │ │ │ || │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │ │ │ │   │   │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │ │ │   │ │ │ │ │ || │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │ │ │ || │ │ │ │ │ │   │   │ │ │ || │ │ │ │ │ || │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │ │ │ || │ │ │ || │ || || || || || || || || || || || || || || || |||| │ │ │ │ │ │ │ │ │ │ │   │ │ │ || │ || || || || || || || || || || || || || |||| │ │ || || || || || || || || || || || || ||   │ │ │ │ │ │ │ │ │ || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || || │ || || || ||   │ │ || || || ||   │ │ │ || || || || || || || || || || || || || │ │ || || │ || || || || || || │ || || || │ || || || || || || │ || || || || || │ │ │ || || || || || || || || || || || || || || │ ||   │ │ || │ || || || || || || || │ │ │ || │ || || || || │ || │ || || || || │ │ || || │ || │ || || || || || || || || || || || || || || || │ || || │ │   │ │ │ || │ || || ||   │ │ || || || || │ || || │ │ || || || || │ || || || || || || || ||   │ │ │ │ ||   │ │ │ || || || || || || || || || || || || || || || || || || || │ || || || || || ||   │ │ │ || || || || || || || │ || || || || || || || || || || || │ │ || || │ || || || || || || │ || || || || || || ||   │ │ || || │ || || || || │ || │ || │ │ || ||   │ │ │ || || || || || │ │ │ │   │ │ || || || │ │   │ │ || || || │ || || || || || || || || │ │   │ || || │ || || │ │ │ || │ │ ||   │ │ │ || || || │ || │   │   │ │ │ || │ || || ||   │ ||   │ || || || || || || || || │ │ │ ||   │ │ || || || || || │ ||   │ || || ||   || || || || │ │ │ || || │   │ || │ || || │ || || || || || || │ || │ || || || || │ │ || || || || || || ||   │ │ || || │ || │ || │ │ || || │ || || || │ || || || || || || || || || || │ || │ │ || || || || || │ │ || || │ || || || || || || │ || │ || || || │ || || || || || || || || || || || || || || || || || || │ │ || || || || │ │ || || │ │ || │ || || || || || || │ │ || │ || │ │ || │ || || || || || || || || │ || │ || || │ │ || || || │ │ || │ || || || || │ || │ || || || │ │ │ │ || │ || || │ │ │ || || || │ │ || || │ || || || │ || || │ || || || │ │ │ || │ || │ || || || || || || || || || │ │ || │ || │ || || || || │ │ || │ || │ │ || │ │ || │ || || │ │ │ || │ || || || || || || || || || || || || || || || || || │ │ │ || || || │ │ || │ || || || │ │ || │ || || │ || || || │ │ || || │ || || || │ │ || │ || || || || │ │ || || │ || || || │ || || │ || │ || │ │ │ || │ || │ || || │ || │ || || || │ │ │ || │ │ || │ || │ || │ │ || │ || │ || || │ || │ │ || │ || │ || || │ │ │ │ │ || │ || || || │ │ || || │ │ || │ || || │ || │ │ || │ │ || │ │ │ │ || || || || || │ │ || │ || || || || || || || │ │ │ || || │ │ │ || │ || │ │ │ || │ │ │ || │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ || │ || || || || │ │ || || || │ || || || || || │ │ || │ │ │ || || || || || || || │ │ │ │ || || │ │ || || || │ || || │ || || || │ || || || || || │ │ || │ || || │ || || || || │ || │ || │ || || │ || || || || || || │ || || │ || || || || || || || || || │ │ || │ || || │ || || || │ │ || || || || || || │ || || || || || │ │ || || || │ │ │ || || || || || │ || || || │ || || || || || │ │ │ || || │ || || || │ || || || || || || || │ │ │ || │ || || || │ │ │ || || || || || │ │ │ || || │ || │ || || || || || || │ │ || || │ || || │ || || || || || │ || || || || || || │ │ || || || │ │ || || │ || || || || || || || || || || || || │ │ │ || || || || │ │ || │ || │ || || || │ || || || │ || || || || │ │ │ || || │ │ || │ || || || │ │ || │ || || || || │ │ │ || │ │ || │ || || │ || || │ │ || │ || │ || │ │ || │ || || || || || || || || || │ || || || │ || │ || │ || │ || │ || || || │ │ || │ || │ │ || │ │ │ || || || || || || │ || || || │ │ || │ || │ || │ || || || │ || || || || │ │ || || || │ │ || │ || || || │ │ │ || || || || || │ || │ || │ │ || │ │ || │ || || │ || │ || │ || │ || │ │ || │ │ || || || || || || || || || || || || || │ │ || │ || || || │ │ || │ || || || || || || │ || || │ || │ │ || │ │ || │ │ │ || │ || │ || || │ │ || || │ │ │ || │ || │ || || || || || │ || || || │ │ || │ || │ || │ || || || || || │ │ || │ │ || │ || │ || || || || │ || || │ │ │ || │ || │ │ || │ │ │ || │ || │ || │ || │ || │ || │ || || │ || || || │ │ │ │ │ │ │ || │ || || │ || │ │ || || │ │ │ || || │ │ || │ || || || │ || || || || || │ │ || || │ || │ || │ || || || || || || || || || || || || || || || || || │ │ │ || │ || || │ │ || || || || │ │ || │ || || │ || || || || │ │ || │ │ │ || || │ || │ || │ || || │ || │ || || || │ │ || │ || || │ │ │ || │ || || │ │ || │ │ || │ || │ │ │ || │ │ │ │ │ || || │ │ │ || || || || │ || || │ │ || │ || │ │ │ │ || || || │ │ || || || │ │ || │ || │ │ || │ || || || │ │ || │ │ │ │ || │ │ │ || │ │ || || │ │ │ || || │ || │ │ │ │ │ || || │ || || || || || || || │ │ || │ │ │ || │ │ │ │ │ │ || || || │ │ || │ │ │ │ │ │ │ || │ || || │ || │ │ │ || │ │ || │ │ │ || || │ │ || │ │ │ │ || │ │ || || || │ │ │ │ │ || │ │ │ || │ || │ || │ │ || │ │ || │ || || │ │ │ || │ │ │ │ || │ || || │ │ │ || │ || │ │ │ │ │ || │ │ || │ │ │ │ || || │ || │ │ │ │ │ │ │ │ │ │ │ │ │ || │ │ || │ || │ │ || || │ || │ │ │ || || || │ │ │ || │ || │ │ || │ │ │ │ │ │ │ │ || || || || │ │ || │ || || │ │ │ || || │ │ || || || │ │ || │ │ || │ || │ │ │ || │ │ │ || │ || │ || │ │ || │ │ │ || │ || │ || || │ || │ || || │ │ │ │ │ │ │ || │ │ │ || │ || │ || │ || │ │ │ │ || │ || │ │ || │ │ │ || │ || │ || │ || │ │ || │ │ │ || │ │ │ || │ │ │ │ │ │ │ │ │ || │ │ || │ │ │ │ │ || │ │ │ │ || │ || │ │ │ || │ || || │ │ │ │ │ || │ || │ │ || │ │ || │ │ || │ || │ │ || || │ || || │ │ │ || │ │ || │ │ │ │ || │ │ │ │ │ │ │ │ │ │ || │ || │ || │ || │ || │ || || │ || │ │ || │ │ || || │ │ │ │ || │ │ │ │ || │ │ │ │ │ || │ │ || │ │ │ || │ │ || │ │ │ │ │ || │ │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ │ || │ │ || │ │ │ || │ │ │ │ │ || │ || │ │ || │ │ || │ │ || │ || │ || || || │ │ │ || │ │ || │ || │ │ || │ || || │ │ │ │ || │ || │ || │ || │ │ │ || │ || │ │ || │ │ || || │ || │ │ || || || │ || │ || || │ │ || │ │ || │ │ || || │ │ || │ │ │ │ │ || │ || │ │ │ || │ || │ │ || || || │ │ │ │ │ │ │ │ || │ || │ │ │ │ │ || || │ │ || │ │ │ │ │ || │ || │ │ || │ │ │ │ || │ || │ || │ || │ || │ │ │ │ │ || │ || │ │ || │ │ || │ │ || || │ │ || │ │ │ │ || │ || │ │ || │ │ || || │ │ || │ || || │ │ || │ || || │ │ │ │ || │ || │ │ │ │ │ │ │ || │ || │ || │ || │ │ || │ │ || │ │ │ │ │ │ │ || │ │ || │ │ │ || │ │ || │ │ || │ │ │ │ || │ || │ │ || │ || │ │ || │ || │ || │ || │ │ │ │ || │ │ │ │ │ │ │ || │ || │ │ │ │ │ │ │ │ │ │ || │ │ │ │ │ || || │ │ │ || │ || │ │ │ │ || │ || │ || │ │ │ || || │ || │ || │ || │ │ │ │ || │ │ │ || || │ │ || │ || │ │ || │ │ || │ │ || │ || || || │ │ || │ │ || │ │ || || │ │ || │ || │ │ || │ || │ │ || │ || │ || │ || │ || │ || │ │ │ || │ || │ │ || │ || │ │ || │ │ │ │ || │ │ || │ │ || │ || │ │ │ │ │ │ || │ || │ || │ || │ │ │ │ │ || │ │ || │ │ || │ │ │ │ │ │ │ │ │ │ │ │ │ │ || │ || │ │ │ │ │ || || │ │ │ │ || │ │ │ || │ │ || │ │ │ │ │ │ || || │ || || │ │ │ │ || │ │ || │ || || │ │ │ │ │ || │ || │ │ │ │ │ │ || || │ || │ │ │ │ || │ │ || │ │ │ │ || │ │ │ │ │ || │ || │ || │ │ || │ │ │ │ │ │ || │ │ || │ || │ │ || │ │ │ │ │ || │ || │ || │ │ │ │ │ || || │ │ || │ │ │ || │ │ │ || │ │ │ || │ || │ || │ │ │ │ │ │ || │ │ │ │ │ │ │ │ │ || || || || │ │ └──────────┘```

---

## 4. Data Flow (FACTS)

### Input → Output Flow by Mode

#### TUI Mode (Interactive)
```
USER INPUT (natural language or command)
    │
    ├─→ :plan "<goal>" command
    │   │
    │   ├─→ llm/session.rs::SessionContext::classify_intent(goal)
    │   │
    │   ├─→ llm/session.rs::SessionContext::propose_plan(goal)
    │   │
    │   └─→ llm/session.rs::SessionContext::render_plan_for_ui() → JSON plan
    │
    ├─→ User approves plan (Y/N prompt)
    │   │
    │   └─→ execution_engine::executor::Executor::execute(approved_plan)
    │       │
    │       ├─→ ExecutionDb::record_execution_with_artifacts() per step
    │       │
    │       └─→ Returns ExecutionResult
    │
    └─→ UI displays results (Action Console, Evidence Panel)
```

#### plan Mode (CLI-only planning)
```
USER INPUT (goal string)
    │
    ├─→ main.rs:run_cli_mode("plan")
    │   │
    │   ├─→ Create SessionContext (llm/session.rs)
    │   │
    │   ├─→ SessionContext::propose_plan(goal) → Plan (in-memory)
    │   │
    │   ├─→ Create PlanAuthorization, mark as auto-approved
    │   │
    │   ├─→ Serialize plan to plans/<plan_id>.json
    │   │
    │   └─→ Output plan summary to stdout
    │       │
    │       └─→ User sees: "Plan written to plans/<plan_id>.json"
    │
    └─→ EXIT 0
```

#### execute Mode (CLI-only execution)
```
USER INPUT (plan_id or file path)
    │
    ├─→ main.rs:run_cli_mode("execute")
    │   │
    │   ├─→ Load plan from plans/<plan_id>.json (or --plan-file path)
    │   │
    │   ├─→ Parse Plan from JSON (llm/planner.rs:parse_plan())
    │   │
    │   ├─→ Create PlanAuthorization, mark as auto-approved
    │   │
    │   ├─→ Open ExecutionDb (execution_tools/db.rs)
    │   │   ├─→ execution_log.db auto-created if missing
    │   │   └─→ codegraph.db must exist
    │   │
    │   ├─→ Open MagellanDb (magellan_tools/db.rs)
   │   │   └─→→ Required for Magellan tools only
    │   │
    │   ├─→ Create Executor with callbacks
    │   │       │
    │   │       ├─→ AutoApprove (no prompt)
│   │   │       ├─→ Progress logging to stdout
│   │   │       └── Step results logging to execution memory
│   │   │
│   │   └─→ executor.execute(approved_plan)
    │       │
    │       ├─→ ExecutionDb::record_execution_with_artifacts() per step
    │   │       └─→ Returns ExecutionResult
    │   │
    └─→ Output summary to stdout
        │
        └─→ User sees: "Plan executed: 3 steps, 2 succeeded, 1 failed"
```

#### evidence Mode (CLI-only query)
```
USER INPUT (query + args)
    │
    ├─→ main.rs:run_cli_mode("evidence")
    │   │
    │   ├─→ Open EvidenceDb (evidence_queries/db.rs)
    │   │   ├─→ execution_log.db (auto-created if missing)
    │   │   └─→ codegraph.db (optional, graceful degradation)
    │   │
    │   ├─→ Dispatch query (Q1-Q8)
    │   │   ├─→ Q1: list_executions_by_tool(tool)
│   │   ├─→ Q2: list_failures_by_tool(tool)
│   │   ├─→ Q3: find_executions_by_diagnostic_code(code)
│   │   ├─→ Q4: find_executions_by_file(path)
│   │   ├─→ Q5: get_execution_details(execution_id)
│   │   ├─→ Q6: get_latest_outcome_for_file(path)
│   │   ├─→ Q7: get_recurring_diagnostics(threshold)
│   │   └─→ Q8: find_prior_fixes_for_diagnostic(code, file)
    │   │
│   └─→ Output results to stdout (JSON format default)
        │
        └─→ User sees: {"tool": "splice_patch", "executions": [...]}
```

### Database Location & Storage

#### db_root Resolution Priority
```
1. --db_root <path> flag
2. $ODINCODE_HOME env var → $ODINCODE_HOME/db/
3. Current directory "."
```

#### File Structure Within db_root
```
db_root/
├── execution_log.db     # Execution records, artifacts (auto-created)
├── codegraph.db         # Code graph from Magellan (must exist)
├── plans/               # Plan JSON files (auto-created directory)
│   └── <plan_id>.json    # Plan serialization
```

#### Plan Storage

**Location**: `db_root/plans/<plan_id>.json`

**Format**:
```json
{
  "plan_id": "plan_<timestamp>",
  "intent": "MUTATE",
  "steps": [
    {
      "step_id": "step_1",
      "tool": "file_read",
      "arguments": {"path": "src/lib.rs"},
      "precondition": "file exists",
      "requires_confirmation": false
    }
  ],
  "evidence_referenced": ["Q1", "Q8"]
}
```

**Lifecycle**:
- Created by `plan` mode or `:plan` command
- Read by `execute` mode
- NOT deleted after execution (history)

**NOT stored**:
- Execution results (logged to execution_log.db, not plans/)
- User preferences
- Evidence summaries (computed on demand)

---

## 5. Interaction Contracts

### Contract 1: UI ↔ LLM (Phase 2)

#### Input (UI → LLM)
```rust
use odincode::llm::session::{SessionContext, classify_intent};

let context = SessionContext::new(db_root, ev_db)?;
let intent = context.classify_intent(user_goal)?;

match intent {
    Intent::MUTATE => {
        let plan = context.propose_plan(user_goal)?;
        ui.display_plan(plan);
    }
    Intent::READ => {
        // Direct tool invocation, no planning needed
    }
    Intent::QUERY => {
        // Evidence query only, no planning needed
    }
    Intent::EXPLAIN => {
        // LLM explains existing evidence
    }
}
```

#### Output (LLM → UI)
```rust
// Plan is a Plan struct from llm/types.rs
let plan = context.propose_plan(goal)?;

// Rendered via UI (TUI or CLI)
ui.display_plan(plan)?;
```

**Error Handling**:
- Invalid user goal → "Goal is too vague: {explanation}"
- Evidence insufficient → "Insufficient evidence: {what's missing}"
- LLM API failure → "Failed to contact LLM: {error}"

---

### Contract 2: UI ↔ Executor (Phase 3)

#### Input (UI → Executor)
```rust
use odincode::execution_engine::{Executor, ApprovedPlan};

let db = ExecutionDb::open(db_root)?;
let magellan_db = MagellanDb::open_readonly(&db_root.join("codegraph.db"))?;

let executor = Executor::new(
    db,
    Some(magellan_db),
    Box::new(AutoApprove),     // CLI mode: auto-approve
    Box::new(NoopProgress),  // CLI mode: logging only
);

let approved = ApprovedPlan {
    plan: parse_plan(plan_json)?,
    authorization: PlanAuthorization::new(plan.plan_id.clone()),
    // Auto-approved for CLI
};

let result = executor.execute(approved)?;
```

#### Output (Executor → UI)
```rust
// ExecutionResult from execution_engine/result.rs
match result.status {
    ExecutionStatus::Completed => {
        ui.show("All steps completed successfully");
    }
    ExecutionStatus::Failed => {
        ui.show(format!("Execution failed at step {}", result.step_results.len()));
    }
}
```

**Callback Implementations for CLI Mode**:
- `ConfirmationCallback`: `AutoApprove` (always returns true, no prompt)
- `ProgressCallback`: `LoggingProgress` (writes to stdout and execution memory)

---

### Contract 3: CLI ↔ Evidence (Phase 0.6)

#### Input (CLI → Evidence)
```rust
use odincode::evidence_queries::{EvidenceDb, queries::Query};

let ev_db = EvidenceDb::open(db_root)?;

match query {
    "Q1" => {
        let results = ev_db.list_executions_by_tool(tool, None, None, None, None)?;
        // Display results
    }
    "Q4" => {
        let results = ev_db.find_executions_by_file(path, None, None)?;
        // Display results
    }
    // ... other queries
}
```

#### Output (Evidence → CLI)
```rust
// Results are structs from evidence_queries/types.rs
// Q1: Vec<ExecutionRow>
// Q4: Vec<FileExecutionRow>
// etc.

// Output to stdout (JSON if --json flag)
println!("{}", serde_json::to_string(&results)?);
```

---

### Contract 4: Error Propagation Rules

#### Rule: Verbatim Tool Errors
```
Tool error → Executor: propagate error exactly
Executor error → UI: display error exactly

WRONG:
"File not found" → "The file could not be found"

RIGHT:
"File not found" → "File not found"
```

#### Error → Exit Code Mapping
| Error Type | Exit Code | Notes |
|------------|-----------|-------|
| Invalid input (unknown mode, args) | 1 | CLI contract only |
| LLM plan invalid | 1 | User sees error |
| Execution failed | 1 | Partial results logged |
| DB not found | 2 | execution_log.db OR codegraph.db missing |
| Evidence query error | 2 | Query failed, or db missing |

---

## 6. Plan Approval Workflow (NO AUTONOMY)

### Step 1: Plan Generation
```
User: "Fix E0425 error in src/lib.rs"
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│  LLM Planning (llm/planner.rs)                                   │
│  ├─→ parse_plan(user_goal) → Plan                            │
│  ├─→ validate_plan(plan) → ValidationResult               │
│  ├─→ render_plan_for_ui(plan) → String                   │
│  └─→ Store plan → plans/<plan_id>.json                      │
│                                                               │
│  Plan: {                                                     │
│    plan_id: "plan_1735036800000",                     │
│    intent: "MUTATE",                                          │
│    steps: [...],                                             │
│    evidence_referenced: ["Q4", "Q8"]                      │
│  }                                                             │
└─────────────────────────────────────────────────────────────────┘
```

### Step 2: Plan Display (TUI Only)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  PLAN PROPOSED FOR APPROVAL                                            │
│                                                                         │
│  Plan: plan_1735036800000                                          │
│  Intent: MUTATE                                                        │
│                                                                         │
│  Evidence Referenced: Q4 (file executions), Q8 (prior fix attempts)        │
│                                                                         │
│  Steps (3):                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐ │
│  │ 1. file_read(src/lib.rs)                                         │ │
│  │    Evidence: Q4 shows 3 prior executions on src/lib.rs            │ │
│  │    Preconditions: file exists                                     │ │
│  │    Confirmation: No                                               │ │
│  │                                                                  │ │
│  │ 2. splice_patch(file: src/lib.rs, symbol: old_function,          │ │
│  │    with: patches/fix_e0425.rs)                                  │ │
│  │    Evidence: Q8 shows 2 prior fix attempts, gaps: 5min, 2min  │ │
│  │    Preconditions: symbol exists                                  │ │
│  │    Confirmation: YES (modifies code)                              │ │ │
│  │                                                                  │
│  │ 3. lsp_check(root: ".")                                         │
│    Evidence: Q4 shows prior errors in src/lib.rs                    │
│ │    Preconditions: Cargo project exists                             │
│ │    Confirmation: No                                               │ │
│  └──────────────────────────────────────────────────────────────┘  │
│                                                                         │
│  [A]ccept  [R]eject  [M]odify  [C]ancel               │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Step 3: User Authorization
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  USER DECIDES                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  If [A]ccept:                                                 │
│    ├─→ Create PlanAuthorization::new(plan_id)                       │
│    ├─→ authorization.approve()                                     │
│─→  Create ApprovedPlan { plan, authorization }              │
│    │
│    └─→ Proceed to execution                                         │
│                                                                       │
│  If [M]odify:                                                │
│    └─→ TUI edit → return to Step 2                             │
│                                                                       │
│  If [R]eject: or [C]ancel:                                     │
│    └─→ Delete plan JSON, no execution                              │
│                                                                       │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Step 4: Execution (TUI or CLI)
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  EXECUTION                                                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   Executor::execute(approved_plan)                                  │
│    │                                                                   │
│    ├─→ For each step:                                                 │
│    │   ├─→ Check precondition                                          │
│   │   │   ├─→ Pass → Continue                                 │
│   │   │   └─→ Fail → Halt, return Failed                  │
│   │   │                                                           │
│   │   ├─→ If requires_confirmation:                                │
│   │   │   └─→ UI prompts user (TUI only)                          │
│   │   │       ├─→ Yes → Continue                                 │
│   │   │       └─→ No → Halt, return Failed                      │
│   │   │                                                           │
│   │   ├─→ Invoke tool                                              │
│   │   │   └─→ Record to execution memory                           │
│   │   │                                                           │
│   │   └─→ Return StepResult                                      │
│    │                                                                   │
│    └─→ Return ExecutionResult                                    │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Step 5: Evidence Logging
```
┌─────────────────────────────────────────────────────────────────────────────┐
│  EVERY STEP LOGGED                                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Tool Invocation → ExecutionDb::record_execution_with_artifacts()       │
│    │                                                                   │
│    ├─→ executions table: tool_name, arguments_json, timestamp,     │
│    │   success, exit_code, duration_ms, error_message            │
│    │                                                           │
│    └─→ execution_artifacts table: execution_id, artifact_type, │
│        content_json                                               │
│                                                                         │
│  Artifacts logged:                                                      │
│    - stdout: Tool stdout (splice_patch, lsp_check)                 │
│    - stderr: Tool stderr (splice_patch, lsp_check)                 │
│    - diagnostics: Vec<Diagnostic> (lsp_check only)                 │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 7. Failure Taxonomy (End-to-End)

### F1: LLM Plan Invalid
**Who Handles**: Phase 2 (llm/planner.rs:validate_plan())

**Detection**: `validate_plan()` returns `Err(PlanError::InvalidTool)`

**User Sees** (TUI or CLI):
```
Error: Invalid plan: Unknown tool 'cargo build' (not in whitelist)
Available tools: file_read, file_write, ...
```

**What is Logged**:
- Plan NOT stored
- No execution attempt
- "llm_plan" artifact logged with validation error

**Exit Code**: 1

---

### F2: Preconditions Fail
**Who Handles**: Phase 3 (execution_engine/preconditions.rs:check_precondition())

**Detection**: Executor checks precondition BEFORE tool invocation

**User Sees** (TUI or CLI):
```
Error: Precondition failed for step 2: splice_patch
Reason: Symbol 'old_function' not found in src/lib.rs
Execution halted: step 3 NOT executed
```

**What is Logged**:
- Partial ExecutionResult with step_results for completed steps
- Failed step logged with `success=false`, error_message set

**Exit Code**: 1

---

### F3: Tool Invocation Fails
**Who Handles**: Phase 3 (execution_engine/tool_mapper.rs:invoke_tool())

**Detection**: Tool returns error or non-zero exit code

**User Sees** (TUI or CLI):
```
Step 3/3: lsp_check(".")

✗ FAILED
Error: cargo check failed
Exit code: 101

stderr:
error: unused variable: `x`
  --> src/lib.rs:45:5
```

**What is Logged**:
- StepResult with `success=false`, stdout/stderr/error_message set
- Full artifacts captured

**Exit Code**: 1

---

### F4: Evidence Write Fails (SQLiteGraph Best-Effort Gap)

**Who Handles**: Phase 0.5 (execution_tools/record.rs)

**Detection**: `ExecutionDb::record_execution_with_artices()` returns `Err`

**SQLite (execution_log.db) MUST succeed**:
- Execution record is primary
- Failure halts execution with `ExecutionError::RecordingError`

**SQLiteGraph (codegraph.db) is best-effort**:
- Graph writes to codegraph.db MAY fail without halting execution
- Error logged to stderr, execution continues with SQLite record

**User Sees** (TUI or CLI):
```
Step 2/3: splice_patch(...)
✓ SUCCESS (stdout: "Patched...")
Warning: Failed to write execution entity to codegraph.db
Error: No such table: graph_entities

Step 3/3: lsp_check(...)
✓ SUCCESS
```

**What is Logged**:
- SQLite: Full execution record with artifacts
- SQLiteGraph: None (best-effort gap acknowledged)

**Exit Code**: 0 (execution succeeded, partial logging)

**Why This Design Choice**:
- Execution evidence is PRIMARY
- Graph data is SECONDARY (useful but not required)
- Execution MUST NOT fail if graph write fails

---

### F5: Evidence Query Fails
**Who Handles**: Phase 0.6 (evidence_queries/queries.rs)

**Detection**: `EvidenceDb::open()` fails or query returns error

**User Sees** (TUI or CLI):
```
Error: codegraph.db not found in db_root
Q4 query results: NONE (empty list)
```

**What is Logged**:
- No query results
- No execution record

**Exit Code**: 2

---

### F6: UI Render Errors
**Who Handles**: Phase 1 (UI components)

**Detection**: UI render function returns error

**User Sees** (TUI only):
```
Error rendering Evidence Panel
```

**What is Logged**:
- Log error to stderr (visible in terminal)
- No additional logging required

**Exit Code**: 0

---

## 8. Determinism Guarantees

### D1: No Hidden State

**Fact**: All state is either transient (UI only) or persisted (SQLite).

**Transient State** (lost on exit, no persistence):
- Current file selection
- Current panel focus
- Scroll positions
- Command history

**Persistent State** (survives exit):
- All execution records
- All graph entities
- All evidence queries
- Plan JSON files

### D2: Stable Ordering

**All queries include deterministic ORDER BY**:
- Q1: timestamp DESC
- Q2: timestamp DESC
- Q3: timestamp DESC
- Q4: timestamp DESC
- Q5: timestamp DESC, id
- Q6: timestamp DESC
- Q7: timestamp DESC
- Q8: temporal gap ASC
- Q8: tie-breaker rules (see docs/PHASE_0_6_EVIDENCE_QUERIES.md)

### D3: Reproducible Runs

Given:
- Same inputs (goal, evidence)
- Same db_root (with same databases)
- Same codebase state

Expect:
- Same plan (LLM deterministic given same inputs)
- Same execution results (executor deterministic)
- Same evidence query results

### D4: No Async, No Concurrency

**Fact**: All execution is synchronous.

**Implications**:
- NO race conditions
- NO ordering ambiguity from concurrent operations
- Deterministic timestamps (within process constraints)

---

## 9. Security & Trust Boundaries

### B1: LLM Cannot Execute

**Constraint**: LLM (Phase 2) only returns plans. It NEVER calls tools directly.

**Enforcement**: Phase 3 Executor is the ONLY component that invokes Phase 0 tools.

**Trust Boundary**:
```
LLM (Phase 2)
  ↓
Plan (structured JSON)
  ↓
Executor (Phase 3)
  ↓
Phase 0 Tools
```

### B2: Executor Cannot Invent Steps

**Constraint**: Executor executes plan EXACTLY as written. It CANNOT add steps, skip steps, or modify arguments.

**Enforcement**: Executor validates each step against:
- Tool whitelist (known tools only)
- Required arguments (all required args present)
- Preconditions (all checks pass)

### B3: UI Cannot Bypass Authorization

**Constraint**: User approval required before execution. UI cannot auto-approve.

**Enforcement**:
- TUI: `[A]ccept` prompt before execution
- CLI: Auto-approve (`AutoApprove`) is ONLY for testing; production requires `--plan-file` flag

### B4: No Raw DB Leakage

**Constraint**: LLM NEVER sees raw database contents. ONLY summarized evidence.

**Enforcement**:
- LLM receives Q1-Q8 summaries via SessionContext
- NO direct SQL access
- NO raw JSON from evidence queries (processed types only)

---

## 10. Acceptance Criteria (For Phase 4 Implementation)

### A1: CLI Entry Points Exist

The implementation MUST provide:

- [ ] `odincode` starts TUI (default mode)
- [ ] `odincode tui` starts TUI (explicit mode)
- [ ] `odincode plan --db-root <path> <goal>` generates plan
- [ ] `odinexecute --db_root <path> --plan-file <plan>` executes plan
- [ ] `odincode evidence --db_root <path> <query> [args]` queries evidence

### A2: All Modules Wired Correctly

The implementation MUST satisfy:
- [ ] Phase 1 UI can invoke Phase 2 planning
- [ ] Phase 2 planning can generate auto-approved plans for CLI mode
- [ ] Phase 2 planning can write plan files for later execution
- [ ] Phase 3 executor can read plan files and execute them
- [ ] Phase 3 executor logs all steps to execution memory
- [ ] Evidence queries work against all execution results
- [ ] TUI evidence panel reflects new data after execution

### A3: db_root Resolution

The implementation MUST satisfy:
- [ ] `--db-root` flag overrides all other resolution
- [ ] `$ODINCODE_HOME/env var` works if set
- [ ] Default `.` works as db_root
- [ ] ExecutionDb auto-creates execution_log.db if missing
- [ ] Error if codegraph.db missing

### A4: Deterministic Execution

The implementation MUST satisfy:
- [ ] Same inputs → same plan → same execution results
- [ ] No hidden state affects execution
- [ ] All ordering is stable (ORDER BY in queries)
- [] Execution IDs are unique (UUID v4)

### A5: Evidence Completeness

The implementation MUST satisfy:
- [ ] Every execution step logged to execution_log.db
- [ ] All tool outputs captured as artifacts
- - stdout for splice_patch, lsp_check
- - stderr for splice_patch, lsp_check
- [ ] Evidence queries return all matching results (no hidden records)

### A6: Error Transparency

The implementation MUST satisfy:
- [ ] Tool errors displayed verbatim (no rewording)
- [ ] Exit codes mapped correctly (0=success, 1=failure, 2=db_error)
- [ ] UI displays all errors in full
- [ ] Errors include context (step number, plan_id)

### A7: No Async or Concurrency

The implementation MUST satisfy:
- [ ] NO async functions (all synchronous)
- [ ] NO background threads
- [ ] NO concurrent step execution
- [ ] Single-threaded event loop (TUI) or single-pass execution (CLI)

### A8: Test Coverage

Tests MUST cover:
- [ ] End-to-end: goal → plan → execution → evidence
- [ ] Failure modes: invalid plan, precondition failure, tool failure
- [ ] Evidence queries: Q1-Q8 all callable
- [ ] db_root resolution with and without --db-root flag
- [ ] JSON output format for scripting

---

## APPENDIX A: Non-Goals (Reiteration)

### What Phase 4 Wiring Will NOT Do

❌ **NO new tools** — Only 11 whitelisted tools from Phase 0
❌ **NO async operations** — All execution is synchronous
❌ **NO background threads** — No concurrent execution
❌ **NO retries** — Failure is terminal
❌ **NO autonomous workflows** — Every action requires user trigger
❌ **NO policy layer** — Preconditions only, no behavioral rules
❌ **NO learning or adaptation** — Stateless execution
❌ **NO embeddings / RAG** — Text-only evidence summaries from Q1-Q8
❌ **NO network I/O** — All operations are local

---

## TERMINATION

**Phase**: 4 — CLI Wiring (PLANNING ONLY)
**Status**: PLANNING COMPLETE — Awaiting Acceptance or Revisions
**Date**: 2025-12-24

**DELIVERABLE**: This document (`docs/PHASE_4_CLI_WIRING.md`)

**NEXT STEP** (IF AUTHORIZED):
- User reviews and approves/plans changes
- Then: "AUTHORIZE Phase 4 — CLI Wiring (IMPLEMENTATION)"
- Then: TDD implementation begins

**IF REJECTED**:
- User specifies changes required
- Revise document per feedback
- Resubmit for acceptance

---

*Last Updated: 2025-12-24*
*Status: PLANNING ONLY — NO CODE AUTHORIZED*
*Phase: 4 — CLI Wiring*
*Purpose: End-to-end wiring of existing modules (NO new features)*
