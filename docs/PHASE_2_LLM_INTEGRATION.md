# Phase 2 — LLM Integration

**Phase**: 2 — LLM Integration Layer
**Status**: PLANNING ONLY — NO CODE AUTHORIZED
**Type**: Architectural Specification
**Dependency**: Phase 0 COMPLETE (0.1 → 0.6), Phase 1 COMPLETE
**Date**: 2025-12-24

---

## SECTION 1 — ROLE OF THE LLM

### What the LLM Is Allowed To Do

The LLM serves as a **planner and explainer** — NOT an executor.

**Permitted Capabilities**:
- ✅ Parse user intent from natural language
- ✅ Map intent to structured plans (tool + arguments)
- ✅ Explain tool results in natural language
- ✅ Reference evidence (Q1-Q8) to provide context
- ✅ Suggest next steps based on evidence
- ✅ Answer questions about the codebase using tools

### What the LLM Is Explicitly Forbidden From Doing

**Prohibited Capabilities**:
- ❌ NO direct filesystem access
- ❌ NO direct database access (raw SQL, raw DB connections)
- ❌ NO direct process execution (calling splice, cargo, etc.)
- ❌ NO implicit permissions (every action requires explicit user authorization)
- ❌ NO autonomous workflows (no background agents)
- ❌ NO self-healing or automatic retries
- ❌ NO speculative actions (no "try this and see")
- ❌ NO embedding/RAG operations (Phase 2 scope)
- ❌ NO policy enforcement ("you shouldn't do this")
- ❌ NO learning or adaptation (stateless between calls)

### Trust Boundaries

```
┌─────────────────────────────────────────────────────────────┐
│                     TRUST BOUNDARY                           │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│   USER INPUT ──→ LLM (plan/explain) ──→ STRUCTURED PLAN     │
│                                                           │
│   STRUCTURED PLAN ──→ VALIDATION ──→ TOOL EXECUTION        │
│                      (must pass all checks)              │
│                                                           │
│   TOOL RESULT ──→ LLM (explain) ──→ USER SEES OUTPUT        │
│                                                           │
└─────────────────────────────────────────────────────────────┘
```

**Key Principle**: The LLM proposes; tools act. The LLM NEVER touches system state directly.

---

## SECTION 2 — PROMPT CONTRACTS

### System Prompt (Immutable)

The system prompt is a fixed contract that defines the LLM's constraints. It MUST include:

**Identity Section**:
```
You are OdinCode, a deterministic code refactoring assistant.
You do NOT execute code directly. You propose plans; tools execute them.
You do NOT have filesystem access. You do NOT have database access.
You ONLY access the codebase through the tool functions provided.
```

**Tool Schema Section**:
```
Available tools:
1. file_read(path) → String
2. file_write(path, contents) → ()
3. file_create(path, contents) → ()
4. file_search(pattern, root) → Vec<SearchMatch>
5. file_glob(pattern, root) → Vec<PathBuf>
6. splice_patch(file, symbol, kind, with) → SpliceResult
7. splice_plan(plan_file) → SpliceResult
8. symbols_in_file(pattern) → Vec<SymbolRow>
9. references_to_symbol_name(name) → Vec<ReferenceRow>
10. references_from_file_to_symbol_name(file, name) → Vec<ReferenceRow>
11. lsp_check(path) → Vec<Diagnostic>

Available evidence queries (Q1-Q8):
- list_executions_by_tool(tool, since, until, limit)
- list_failures_by_tool(tool, since, limit)
- find_executions_by_diagnostic_code(code, limit)
- find_executions_by_file(path, since, limit)
- get_execution_details(execution_id)
- get_latest_outcome_for_file(path)
- get_recurring_diagnostics(threshold, since)
- find_prior_fixes_for_diagnostic(code, file, since)
```

**Constraints Section**:
```
CRITICAL CONSTRAINTS:
1. You MUST propose a plan before any action
2. You CANNOT execute tools directly — return structured plan only
3. You MUST reference evidence (Q1-Q8) when available
4. You CANNOT claim causality — only temporal adjacency
5. You MUST surface tool errors verbatim — no rewording
6. You MUST return explicit "insufficient evidence" if data missing
```

### User Prompt (UI-Provided)

The user prompt consists of:

**Required Elements**:
1. User intent — Natural language request
2. Current context — Selected file, diagnostic, or execution
3. Available evidence — Summarized from Q1-Q8 (NOT raw DB)

**Prompt Structure**:
```
User Request: "{user_input}"

Context:
- Current file: {selected_file}
- Current diagnostic: {selected_diagnostic}
- Evidence summary:
  - Executions for this file: {count}
  - Failures for this file: {count}
  - Recurring diagnostics: {list}

Available tools: {tool_schema}

Response format: Structured plan or explanation
```

### Tool Schema Exposure (Read-Only)

The LLM receives a read-only JSON schema describing available tools:

```json
{
  "tools": [
    {
      "name": "file_read",
      "description": "Read file contents",
      "parameters": {
        "path": {"type": "string", "required": true}
      },
      "returns": "string (file contents)"
    },
    {
      "name": "splice_patch",
      "description": "Apply span-safe symbol replacement",
      "parameters": {
        "file": {"type": "string", "required": true},
        "symbol": {"type": "string", "required": true},
        "kind": {"type": "string", "required": false},
        "with": {"type": "string", "required": true}
      },
      "returns": "SpliceResult { success, changed_files, stdout, stderr }"
    }
    // ... other tools
  ]
}
```

**Critical**: Tool names and parameters match Phase 0 function signatures exactly.

### Evidence Exposure (Q1-Q8 Summaries Only)

The LLM receives **summarized** evidence, NOT raw database access:

```
EVIDENCE SUMMARY (read-only):
- Q1 (tool_executions): {tool: "splice_patch", count: 3, last_success: true}
- Q2 (failures): {tool: "splice_patch", count: 1, last_error: "Symbol not found"}
- Q3 (diagnostic_executions): {code: "E0425", occurrences: 7}
- Q4 (file_executions): {file: "src/lib.rs", executions: 5, last: "success"}
- Q5 (execution_details): {execution_id: "...", tool: "...", success: true}
- Q6 (latest_outcome): {file: "src/lib.rs", success: true, timestamp: "..."}
- Q7 (recurring): {code: "E0425", file: "src/lib.rs", count: 7}
- Q8 (prior_fixes): {code: "E0425", fix_attempts: 2, temporal_gaps: [1234, 5678]}
```

**Critical**: The LLM NEVER sees raw SQL. It receives pre-aggregated summaries.

---

## SECTION 3 — INTENT → PLAN → ACTION MODEL

### How User Text Becomes a Structured Plan

**Step 1: User Input**
```
User: "Fix the E0425 error in src/lib.rs"
```

**Step 2: LLM Parses Intent**
The LLM classifies the intent into one of:
- `READ` — Request information
- `MUTATE` — Modify code
- `QUERY` — Search or inspect
- `EXPLAIN` — Understand why something happened

**Step 3: LLM Queries Evidence**
The LLM receives pre-summarized evidence:
```
EVIDENCE: Q8 shows 2 prior fix attempts for E0425 in src/lib.rs
EVIDENCE: Q4 shows 5 total executions on src/lib.rs
EVIDENCE: Q3 shows 7 total E0425 occurrences
```

**Step 4: LLM Generates Structured Plan**
```json
{
  "plan_id": "plan_123",
  "intent": "MUTATE",
  "steps": [
    {
      "step_id": "step_1",
      "tool": "file_read",
      "arguments": {"path": "src/lib.rs"},
      "precondition": "file exists"
    },
    {
      "step_id": "step_2",
      "tool": "splice_patch",
      "arguments": {
        "file": "src/lib.rs",
        "symbol": "foo",
        "kind": "fn",
        "with": "patches/fix_e0425.rs"
      },
      "precondition": "file contains symbol 'foo'",
      "requires_confirmation": true
    },
    {
      "step_id": "step_3",
      "tool": "lsp_check",
      "arguments": {"path": "."},
      "precondition": "step_2 succeeded"
    }
  ],
  "evidence_referenced": ["Q8", "Q4"]
}
```

### How Plans Are Validated Against Constraints

**Validation Rules (enforced by OdinCode, NOT by LLM)**:

1. **Tool Existence Check**
   - Plan references only known tools (whitelist)
   - Unknown tools → rejection with explicit error

2. **Argument Type Check**
   - Arguments match tool schema
   - Missing required arguments → rejection

3. **Precondition Verification**
   - Each step's preconditions checked BEFORE execution
   - Failed preconditions → plan halted with explanation

4. **Evidence Requirement Check**
   - Plans referencing history MUST cite evidence queries
   - Claims without evidence → rejection with "insufficient evidence"

**Example Validation Flow**:
```rust
fn validate_plan(plan: &Plan) -> Result<ValidationResult> {
    for step in &plan.steps {
        // 1. Tool exists
        if !TOOL_WHITELIST.contains(&step.tool) {
            return Err(Error::UnknownTool(step.tool.clone()));
        }

        // 2. Arguments valid
        validate_arguments(&step.tool, &step.arguments)?;

        // 3. Preconditions checked
        if !check_preconditions(&step).await? {
            return Err(Error::PreconditionFailed(step.step_id.clone()));
        }
    }
    Ok(ValidationResult::Valid)
}
```

### How Invalid Plans Are Rejected Deterministically

**Rejection Modes**:

1. **Unknown Tool**
   - Error: `"Tool 'xyz' not found in available tools"`
   - Action: Return list of available tools

2. **Invalid Arguments**
   - Error: `"Invalid arguments for 'splice_patch': missing 'symbol'"`
   - Action: Return tool schema

3. **Precondition Failure**
   - Error: `"Precondition failed: file 'src/lib.rs' does not exist"`
   - Action: Halt plan, return error + explanation

4. **Insufficient Evidence**
   - Error: `"Plan references 'prior fixes' but Q8 returns empty results"`
   - Action: Halt plan, indicate insufficient evidence

5. **Forbidden Operation**
   - Error: `"Operation not allowed: direct filesystem write not permitted"`
   - Action: Halt plan, explain constraint

**All rejections are deterministic** — same plan always produces same rejection.

---

## SECTION 4 — TOOL ROUTING RULES

### Which Intents Map To Which Tools

| User Intent | Mapped Tool(s) | Required Arguments | Mandatory Preconditions |
|-------------|-----------------|-------------------|-------------------------|
| "Read file X" | file_read | path | file exists |
| "Search for X" | file_search | pattern, root | root exists |
| "Find symbol X" | symbols_in_file | pattern | codegraph.db exists |
| "Find refs to X" | references_to_symbol_name | name | codegraph.db exists |
| "Fix error X" | file_read → splice_patch → lsp_check | varies | file exists, symbol exists |
| "Check code" | lsp_check | path | Cargo project |
| "List history" | list_executions_by_tool | tool_name | execution_log.db exists |

### Required Arguments Per Tool

**Exact Mappings** (from Phase 0 function signatures):

```rust
file_read:
  - path: String (required)

file_write:
  - path: String (required)
  - contents: String (required)

file_create:
  - path: String (required)
  - contents: String (required)

file_search:
  - pattern: String (required)
  - root: PathBuf (required)

file_glob:
  - pattern: String (required)
  - root: PathBuf (required)

splice_patch:
  - file: PathBuf (required)
  - symbol: String (required)
  - kind: Option<String> (optional)
  - with: PathBuf (required)

splice_plan:
  - file: PathBuf (required)

symbols_in_file:
  - pattern: String (required)

references_to_symbol_name:
  - name: String (required)

references_from_file_to_symbol_name:
  - file: PathBuf (required)
  - name: String (required)

lsp_check:
  - path: PathBuf (required)
```

### Mandatory Preconditions

**Preconditions checked BEFORE tool execution**:

```rust
// file_* tools
 precondition!("file exists", file_path.exists());

// splice_patch
precondition!("file is in Cargo workspace", is_cargo_project(file));
precondition!("symbol exists in file", symbol_exists(file, symbol));

// magellan_tools
precondition!("codegraph.db exists", codegraph_db_exists());
precondition!("magellan has indexed file", file_indexed(file));

// lsp_tools
precondition!("Cargo project exists", cargo_toml_exists(path));
```

**Precondition Failure = Plan Halt**
- No retries
- No alternative suggestions
- Explicit error message returned to user

### Failure Handling

**Rule**: The LLM MUST surface tool errors verbatim.

**Example Flow**:
```
1. LLM proposes: splice_patch on src/lib.rs, symbol "bar"
2. Validation: symbol "bar" exists → proceed
3. Execution: splice_patch returns error "Symbol 'bar' not found"
4. LLM response: MUST include exact error text
   ❌ "The symbol wasn't found" (rewritten)
   ❌ "There might be a typo" (inference)
   ✅ "splice_patch failed: Symbol 'bar' not found" (verbatim)
```

**LLM Post-Failure Behavior**:
- Explain what the error means
- Reference evidence (e.g., "Q1 shows this file was just modified")
- Suggest next step ONLY if evidence supports it
- Never guess at root cause

---

## SECTION 5 — EVIDENCE FEEDBACK LOOP

### How Execution Results Are Fed Back To The LLM

**Immediate Feedback Loop**:
```
PLAN EXECUTION
    │
    ├─→ SUCCESS → Tool output → LLM explains result
    │
    └─→ FAILURE → Tool error → LLM explains error
                │
                └─→ LLM may request evidence
                    │
                    └─→ Evidence summary returned
                        │
                        └─→ LLM generates revised plan
```

**Evidence Injection Points**:
1. **Before planning**: LLM receives evidence summary
2. **After failure**: LLM may request specific evidence queries
3. **After success**: LLM may explain result using evidence context

### How The LLM May Reference Evidence

**Allowed Evidence Patterns**:

1. **Temporal Adjacency** (from Q8):
   - ✅ "Q8 shows splice_patch on foo occurred 5 minutes before E0425 appeared"
   - ❌ "splice_patch on foo caused E0425"

2. **Historical Success Rate** (from Q1/Q2):
   - ✅ "Q1 shows splice_patch succeeded 2/3 times on this file"
   - ❌ "splice_patch has a 67% success rate" (not a metric)

3. **Recurring Diagnostics** (from Q7):
   - ✅ "Q7 shows E0425 occurred 7 times in src/lib.rs"
   - ❌ "src/lib.rs is error-prone"

4. **File Execution History** (from Q4):
   - ✅ "Q4 shows 3 prior edits to src/lib.rs"
   - ❌ "src/lib.rs is frequently modified"

**Forbidden Evidence Patterns**:
- ❌ NO causal language ("caused", "fixed", "prevented")
- ❌ NO probability or confidence ("likely", "probably", "might")
- ❌ NO risk assessment ("risky", "safe", "dangerous")
- ❌ NO policy statements ("should not", "better to avoid")

### "Insufficient Evidence" Handling Rules

**When Evidence Is Missing**:

1. **Empty Query Results**
   - Response: "No evidence found for [query]"
   - Action: State that no data available, proceed with caution

2. **Codegraph.db Missing**
   - Response: "Structural data not available (codegraph.db missing)"
   - Action: Proceed with file operations only, skip graph queries

3. **No Prior Executions**
   - Response: "No prior execution history for this file/tool"
   - Action: Treat as new context, no historical constraints

**LLM Behavior**:
- State evidence status explicitly
- Do NOT guess or interpolate
- Offer to proceed with user confirmation if evidence insufficient

---

## SECTION 6 — SAFETY & DETERMINISM GUARANTEES

### Why Hallucination Is Structurally Impossible

**Architecture Prevents Hallucination**:

1. **Tool Boundary**
   - LLM returns structured plan, NOT tool calls
   - System validates plan before execution
   - Invalid plans rejected before execution

2. **Evidence Boundary**
   - LLM receives summarized evidence, NOT raw DB
   - Evidence summaries are deterministic (from SQL queries)
   - No room for LLM to "invent" evidence

3. **Precondition Boundary**
   - Every step validated before execution
   - Preconditions checked against real system state
   - Failed preconditions halt execution

4. **Error Boundary**
   - Tool errors returned verbatim
   - LLM cannot reword or interpret errors
   - User sees exact error from tool

**Example Hallucination Prevention**:
```
LLM: "I'll run lsp_check" → REJECTED
    → System: LLM cannot execute tools. Return plan instead.

LLM: "Q8 shows this fix worked before" → REJECTED
    → System: Q8 returns temporal_gap_ms, not "worked". State correctly.

LLM: "This is 90% likely to succeed" → REJECTED
    → System: Confidence scores not permitted. State evidence only.
```

### How Guessing Is Detected And Rejected

**Detection Mechanisms**:

1. **Plan Validation**
   - Unknown tools → rejection
   - Invalid arguments → rejection
   - Unreferenced claims → "provide evidence for this claim"

2. **Evidence Verification**
   - LLM cites "Q8 shows X" → System checks Q8 summary
   - If Q8 summary doesn't support claim → rejection
   - LLM must correct or drop claim

3. **Response Validation**
   - LLM outputs confidence score → rejected
   - LLM outputs causal language → rejected
   - LLM rewords tool error → rejected

**Rejection Response**:
```
System: "Response contains disallowed element: confidence score"
System: "Permitted: evidence summaries, tool output, explanations"
System: "Rejected: '90% likely to succeed'"
System: "Correct format: 'Q1 shows 2/3 prior attempts succeeded'"
```

### How Every Decision Is Reconstructable Post-Hoc

**Audit Trail Design**:

1. **Plan Storage**
   - Every plan assigned UUID
   - Plan stored in execution_log.db
   - Includes: LLM input, evidence summary, generated plan

2. **Execution Trace**
   - Each step execution recorded
   - Preconditions logged
   - Tool outputs stored as artifacts

3. **LLM Response Storage**
   - LLM explanation stored
   - Evidence citations stored
   - User authorization recorded

**Reconstruction Query** (for audit):
```sql
-- Reconstruct full conversation for execution_id
SELECT
    e.tool_name, e.arguments_json, e.timestamp, e.success,
    a.artifact_type, a.content_json
FROM executions e
LEFT JOIN execution_artifacts a ON e.id = a.execution_id
WHERE e.id = ?
ORDER BY a.artifact_type;
```

**Output**: Complete trace of what was asked, what evidence was provided, what plan was generated, and what result occurred.

---

## SECTION 7 — UI INTEGRATION

### How The UI Invokes The LLM

**Invocation Flow**:

```
USER INPUT (text)
    │
    ↓
┌─────────────────────────────────────────────────────────────┐
│  UI: Gather Context                                            │
│  - Current file selection                                     │
│  - Current diagnostic (if any)                                │
│  - Relevant evidence (Q1-Q8 summaries)                       │
└─────────────────────────────────────────────────────────────┘
    │
    ↓
┌─────────────────────────────────────────────────────────────┐
│  LLM Layer: Generate Plan                                     │
│  - Parse user intent                                         │
│  - Check evidence                                            │
│  - Generate structured plan                                  │
└─────────────────────────────────────────────────────────────┘
    │
    ↓
┌─────────────────────────────────────────────────────────────┐
│  UI: Display Plan                                             │
│  - Show steps                                                │
│  - Show evidence cited                                       │
│  - Request user authorization                               │
└─────────────────────────────────────────────────────────────┘
    │
    ↓
USER AUTHORIZATION (Y/N)
    │
    ↓ (if Y)
┌─────────────────────────────────────────────────────────────┐
│  Execution Layer: Execute Plan                               │
│  - Validate each step                                       │
│  - Execute tools                                            │
│  - Record results                                           │
└─────────────────────────────────────────────────────────────┘
    │
    ↓
┌─────────────────────────────────────────────────────────────┐
│  LLM Layer: Explain Results                                  │
│  - Receive tool outputs                                      │
│  - Generate explanation                                      │
└─────────────────────────────────────────────────────────────┘
    │
    ↓
┌─────────────────────────────────────────────────────────────┐
│  UI: Display Explanation                                      │
│  - Show what happened                                       │
│  - Show tool outputs                                        │
│  - Show evidence context                                    │
└─────────────────────────────────────────────────────────────┘
```

### How LLM Output Is Rendered

**Plan Display**:
```
Proposed Plan:
─────────────────────────────────────────
Step 1: Read file src/lib.rs
  Evidence: Q4 shows 3 prior executions

Step 2: Apply splice_patch
  Symbol: foo
  From: patches/fix_e0425.rs
  Evidence: Q8 shows 2 prior fix attempts

Step 3: Run cargo check
  Verify: Error E0425 is resolved
─────────────────────────────────────────
Evidence cited: Q4, Q8
Requires confirmation: Yes (modifies code)

[A]ccept  [R]eject  [M]odify
```

**Explanation Display**:
```
Result: SUCCESS
─────────────────────────────────────────
splice_patch completed successfully:
  - Replaced symbol 'foo' in src/lib.rs
  - Duration: 234ms

cargo check completed:
  - No errors found
  - 1 warning (unused import)

Evidence context:
  - Q4: This is the 4th execution on src/lib.rs
  - Q8: Prior fix 5 minutes ago (gap: 300000ms)
  - Q3: No E0425 diagnostics found
─────────────────────────────────────────
[Enter to continue]
```

### How User Authorizes Each Action

**Authorization Modes**:

1. **Step-by-Step**
   - Each step requires explicit [A]ccept
   - User reviews before proceeding
   - Failed steps halt execution

2. **Plan-Authorize**
   - User authorizes entire plan upfront
   - All steps execute without further prompts
   - Only halt on error

3. **Interactive**
   - User can modify plan before execution
   - Add/remove steps
   - Adjust arguments

**Authorization Prompt**:
```
Step 2 of 3: Apply splice_patch
  Symbol: foo
  File: src/lib.rs
  From: patches/fix_e0425.rs

This will MODIFY src/lib.rs.
Evidence: Q8 shows 2 prior attempts.

Execute? [y/N/a/c] _
(y=yes, n=no, a=all, c=cancel plan)
```

---

## SECTION 8 — EXTENSION POINTS (OUT OF SCOPE)

### Where RAG/Embeddings Could Attach Later

**NON-BINDING** — These are NOT part of Phase 2:

**E1: Vector Search Integration Point**
```
CURRENT (Phase 2):
Evidence summary → LLM

FUTURE (NOT Phase 2):
Code embeddings → Vector search → Enhanced evidence summary → LLM
```

**What it WOULD enable** (NOT Phase 2):
- Semantic code search ("code similar to this pattern")
- Embedding-based similarity ("functions that do X")
- Vector-based recommendations

**Phase 2 Constraint**: LLM uses text-only evidence summaries from Q1-Q8.

### Where Policy Layers Could Attach Later

**NON-BINDING** — These are NOT part of Phase 2:

**E2: Policy Layer Integration Point**
```
CURRENT (Phase 2):
LLM → Plan → Validation (preconditions) → Execution

FUTURE (NOT Phase 2):
LLM → Plan → Policy Check → Validation → Execution
```

**What it WOULD enable** (NOT Phase 2):
- "This operation is not allowed per team policy"
- "Requires approval from maintainer"
- "Blocking rule: no direct main branch edits"

**Phase 2 Constraint**: No policy layer. Preconditions only.

### Where Agent Coordination Could Attach Later

**NON-BINDING** — These are NOT part of Phase 2:

**E3: Multi-Agent Integration Point**
```
CURRENT (Phase 2):
Single LLM → Plan → Execution

FUTURE (NOT Phase 2):
Specialist agents → Plan coordination → Execution
```

**What it WOULD enable** (NOT Phase 2):
- Separate "file agent", "build agent", "test agent"
- Agent negotiation for complex tasks
- Workflow automation

**Phase 2 Constraint**: Single LLM, single session, no agent coordination.

---

## SECTION 9 — ACCEPTANCE CRITERIA (FOR PHASE 2 IMPLEMENTATION)

### A1: LLM As Planner Only

The implementation MUST satisfy:
- [ ] LLM returns structured plans, NOT tool calls
- [ ] System validates all plans before execution
- [ ] Invalid plans rejected with explicit error messages
- [ ] LLM cannot directly call tools

### A2: Evidence-Only Constraints

The implementation MUST satisfy:
- [ ] LLM receives ONLY summarized evidence (Q1-Q8)
- [ ] NO raw SQL or direct DB access from LLM
- [ ] LLM citations validated against evidence summaries
- [ ] "Insufficient evidence" state explicitly handled

### A3: Verbatim Error Propagation

The implementation MUST satisfy:
- [ ] Tool errors returned to LLM verbatim
- [ ] LLM explanations include exact error text
- [ ] NO rewording or interpretation of tool errors
- [ ] Error stored in execution_log.db verbatim

### A4: Deterministic Plan Execution

The implementation MUST satisfy:
- [ ] Same inputs + same evidence = identical plans
- [ ] Plan steps executed in defined order
- [ ] Preconditions checked deterministically
- [ ] No retries or "self-healing" logic

### A5: Audit Trail Completeness

The implementation MUST satisfy:
- [ ] Every LLM conversation stored in execution_log.db
- [ ] Plans, evidence citations, user authorization stored
- [ ] Full reconstruction possible from execution ID
- [ ] Tool outputs stored as execution_artifacts

### A6: No Inference Enforcement

The implementation MUST satisfy:
- [ ] LLM outputs screened for forbidden patterns
- [ ] Confidence scores rejected
- [ ] Causal language rejected
- [ ] Risk assessments rejected
- [ ] Policy statements rejected (Phase 2)

### A7: Module Size Compliance

The implementation MUST satisfy:
- [ ] All source files ≤ 300 LOC
- [ ] LLM layer split across focused modules
- [ ] Tests verify all constraints

### A8: Test Coverage

Tests MUST cover:
- [ ] Plan validation (unknown tools, invalid args)
- [ ] Precondition checking (file exists, symbol exists)
- [ ] Evidence citation validation
- [ ] Verbatim error propagation
- [ ] Audit trail reconstruction
- [ ] Forbidden pattern rejection

---

## SECTION 10 — IMPLEMENTATION MODULE STRUCTURE (IF AUTHORIZED)

```
src/llm/
├── mod.rs              # Module exports, Error type (~50 LOC)
├── client.rs           # LLM API client (~300 LOC)
├── planner.rs          # Plan generation from LLM (~300 LOC)
├── validator.rs        # Plan validation logic (~300 LOC)
├── executor.rs         # Plan execution orchestrator (~300 LOC)
└── types.rs            # Plan, Step, Result types (~200 LOC)

tests/llm_tests.rs
├── test_plan_generation
├── test_plan_validation
├── test_precondition_checking
├── test_evidence_citation
├── test_verbatim_errors
└── test_audit_trail
```

**Total LOC**: ~1,750 (all modules ≤ 300 LOC)

---

## SECTION 11 — NON-GOALS REITERATION

### What Phase 2 Will NOT Do

❌ **NO autonomous workflows** — Every action requires user authorization
❌ **NO background agents** — Single LLM, single session, no parallel tasks
❌ **NO speculative suggestions** — LLM responds to user intent only
❌ **NO embeddings / RAG** — Text-only evidence summaries from Q1-Q8
❌ **NO policy enforcement** — Preconditions only, no behavioral rules
❌ **NO learning or adaptation** — Stateless between calls
❌ **NO retries or self-healing** — Failed plans fail explicitly
❌ **NO direct system access** — LLM proposes, tools execute

---

## TERMINATION

**Phase**: 2 — LLM Integration (PLANNING ONLY)
**Status**: PLANNING COMPLETE — Awaiting Acceptance or Revisions
**Date**: 2025-12-24

**DELIVERABLE**: This document (`docs/PHASE_2_LLM_INTEGRATION.md`)

**NEXT STEP** (IF AUTHORIZED):
- User reviews and approves/plans changes
- Then: "AUTHORIZE Phase 2 — LLM Integration (IMPLEMENTATION)"
- Then: TDD implementation begins

**IF REJECTED**:
- User specifies changes required
- Revise document per feedback
- Resubmit for acceptance

---

*Last Updated: 2025-12-24*
*Status: PLANNING ONLY — NO CODE AUTHORIZED*
*Phase: 2 — LLM Integration*
*Purpose: Deterministic LLM integration layer (planner + explainer, not executor)*
