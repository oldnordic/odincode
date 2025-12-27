# The Core Loop (Authoritative, No Guessing)

You're defining a closed-loop, correctness-driven edit pipeline where nothing is trusted unless validated by tools.

## Golden Rule

**The model is never allowed to "believe" it changed code.**
**Only tools and validators can confirm reality.**

---

## Canonical Edit Workflow (Step-by-Step)

### 0. Intent Detection (Plan Mode)

User intent: "Change X"

Model is **not allowed to edit yet**.

**Output:**
- Hypothesis of what might need to change
- Must request grounding tools

---

### 1. Impact Analysis (Read-Only, Mandatory)

**Tool calls (no writing allowed):**

- `magellan_query` → Which symbols, files, and references are impacted if X changes?
- `symbols_query` / `references_query`
- Optional: `git_status` (ensure clean baseline)

**Result:**
- Authoritative list of files + symbols
- Stored in Execution DB
- Injected as **facts**, not prose

**If model skips this → hard failure.**

---

### 2. Change Planning (Constrained)

Model must produce:
- Exact files to change
- Exact operations (replace / insert / delete)
- Justification referencing **execution IDs**, not memory text

**No code yet.**

---

### 3. Controlled Mutation (Single File or Patch)

**Allowed tools only:**
- `splice_patch` or
- `file_edit`

**Rules:**
- One logical change per call
- No speculative multi-file edits
- Edits must reference analysis execution IDs

**Tool executes → returns:**
- Diff
- Checksum
- Success/failure

Stored in memory.

---

### 4. Immediate Validation (Non-Negotiable)

**Mandatory tools:**
- `lsp_check`

**Optional:** `cargo check`, `tsc`, etc. via `bash_exec`

**Outcomes:**
- ✅ Pass → proceed
- ❌ Fail → error objects logged

**Important:** The model is **not allowed to explain the error yet.**

---

### 5. Error-Driven Correction Loop (Grounded)

If validation fails, the model is forced to:
1. `memory_query` → retrieve:
   - Last edit execution
   - LSP errors
   - Previous failed attempts
2. Reason **only over retrieved facts**
3. Propose **one corrective edit**
4. Repeat steps 3 → 4

**No guessing.**
**No "probably".**
**No silent retries.**

The stall detector + execution budget guard this loop.

---

### 6. Convergence Gate

**When:**
- LSP passes
- No new diagnostics
- Circuit breaker closed

**Then and only then:**

---

### 7. Presentation Layer

System outputs:
- ✔ What changed (file + diff summary)
- ✔ Why (linked execution IDs)
- ✔ Validation proof (LSP pass)

**The model does not summarize from memory — it references the execution log.**

---

## Why This Eliminates Hallucinations

| Problem | Why it disappears |
| --- | --- |
| "Model assumes edit worked" | Only tools can confirm |
| Context amnesia | Memory is queried explicitly |
| Guessing fixes | Fixes must reference errors |
| Infinite loops | Stall detector + budgets |
| Overlong context | Only decisions re-enter context |
| Model variance | Logic enforced by system |

---

## Key Enforcement Mechanisms (Critical)

### 1. Tool-Gated Writing

The model cannot write text edits without:
- Prior analysis execution IDs
- An active Edit Session

### 2. Memory-Forced Reasoning

If a tool fails:
- Next model step **must include `memory_query`**
- Absence = hard stop

### 3. Truth Flags on Tool Results

Each tool result injected as:
```json
{
  "execution_id": 4312,
  "tool": "lsp_check",
  "status": "failed",
  "authority": "ground_truth",
  "summary": "Type mismatch in src/foo.rs:42"
}
```

The model is instructed:
> **Do not reinterpret `ground_truth` results.**

---

## Why This Is Better Than Claude Code / MCP

| Claude Code | OdinCode (your design) |
| --- | --- |
| Prompt discipline | Explicit execution graph |
| Summarization | Externalized truth |
| Implicit trust | Forced memory grounding |
| Hopes the model behaves | Does not allow it not to |

---

## The Mental Model (This Matters)

You are **not** building:
> "An LLM that edits code"

You **are** building:
> A **verifier-driven compiler pipeline** where the LLM proposes deltas

**The LLM is a planner, not an authority.**

---

## Final Take

This workflow is:
- Slower per edit ✔
- Massively safer ✔
- Self-correcting ✔
- Context-independent ✔
- Model-agnostic ✔

And most importantly:

> **Every delivered result is backed by executed reality, not belief.**
