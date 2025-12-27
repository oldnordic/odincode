# OdinCode Architecture: Factual Feedback Loop for LLM-Powered Refactoring

**Status**: Design Document
**Created**: 2025-12-23
**Core Principle**: We are NOT building a refactoring tool. We ARE building the substrate that lets an LLM refactor without guessing, drifting, or hallucinating.

---

## The Fundamental Insight

### What Refactoring Tools Try (and Fail) to Do

Classic AI refactoring tools attempt to be smart:
- Infer intent
- Understand semantics
- Apply transformations
- Hope nothing breaks

**Why they fail:**
- Language semantics are complex
- Real codebases are messy
- Edge cases explode
- Trust erodes

That's why people still refactor manually.

### What We're Building Instead (and Why It Works)

We split the problem correctly:

#### 1. LLM = Intelligence
- Decides what to change
- Reasons about intent
- Plans steps

#### 2. Tools = Ground Truth
- What exists
- Where it is
- What references what
- What broke
- What passed

**No guessing. No inference. No pretending.**

---

## The Core Architecture: Splice + Codebase Mapper

### What Splice Provides

**Splice** is a span-safe refactoring kernel for Rust using SQLiteGraph:

```bash
# Symbol-aware patching
splice patch --file <FILE> --symbol <SYMBOL> --with <FILE>

# Multi-step refactoring plans
splice plan --file <PLAN.json>

# rust-analyzer validation
splice patch --analyzer <off|os|path>
```

**Capabilities:**
1. Span-safe transformations at exact symbol boundaries
2. SQLiteGraph backend (graph + SQLite + HNSW in one)
3. rust-analyzer integration for compiler validation
4. Plan-based multi-step refactorings

### What No Other LLM Tool Has

**Current LLM Tools** (Claude Code, Copilot, Cursor, etc.):
- ❌ LLM guesses what to change
- ❌ No precise span awareness
- ❌ No graph-based dependency tracking
- ❌ No validation against real compiler
- ❌ Changes break things silently

**Our Stack** (Splice + Mapper):
- ✅ Mapper: "Here's exactly what exists, where, and what it affects"
- ✅ LLM: "Based on these facts, here's what should change"
- ✅ Splice: Applies changes at exact symbol spans using SQLiteGraph
- ✅ rust-analyzer: Validates against real compiler
- ✅ Mapper: Persists what changed and what broke

---

## The Factual Feedback Loop

### Architecture Diagram

```
┌─────────────────────────────────────────┐
│  Codebase Mapping Tool (Facts)          │
│  - What exists                           │
│  - Where it is                           │
│  - What references what                  │
└──────────────┬──────────────────────────┘
               │ Ground Truth
               ↓
┌─────────────────────────────────────────┐
│  LLM (Intelligence)                     │
│  - Decides what to change                │
│  - Plans the refactoring                 │
└──────────────┬──────────────────────────┘
               │ Plan
               ↓
┌─────────────────────────────────────────┐
│  Splice (Execution)                     │
│  - Applies changes at exact spans        │
│  - Uses SQLiteGraph for dependencies    │
│  - Validates with rust-analyzer          │
└──────────────┬──────────────────────────┘
               │ Results
               ↓
┌─────────────────────────────────────────┐
│  Mapper (Validation & Memory)           │
│  - What broke                            │
│  - What passed                           │
│  - Persist to knowledge graph            │
└──────────────┬──────────────────────────┘
               │ Updated State
               └──────→ Loop until compiler passes
```

### The Loop Steps

1. **Observe**: Mapper provides ground truth about codebase state
2. **Decide**: LLM reasons about what needs to change (based on facts, not guesses)
3. **Act**: Splice applies changes at exact symbol spans
4. **Validate**: Compiler/rust-analyzer judges success or failure
5. **Persist**: Mapper stores results in knowledge graph
6. **Reason**: LLM analyzes what broke vs what passed
7. **Repeat**: Loop until compiler gives positive feedback

**At every step:**
- Facts come from tools
- Validation comes from compiler/analyzer
- Memory is externalized
- The LLM never "assumes"

---

## Why This Avoids Code Drift

Code drift happens when:
- Changes are made without full visibility
- Feedback is partial or delayed
- The model fills gaps with guesses

**Our loop prevents drift because:**
- Every decision grounded in mapper's facts
- Every change validated by compiler
- Every result persisted to knowledge graph
- No progress until compiler approves

---

## Why This Is Not Overengineering

We didn't add:
- ❌ Semantic engines
- ❌ Complex refactoring rules
- ❌ Language-specific magic

We added:
- ✅ Persistence (knowledge graph)
- ✅ Determinism (exact spans)
- ✅ Enforcement (compiler validation)

**Those are infrastructure, not features.**

---

## The One Sentence Summary

If you ever have to explain this to yourself or someone else:

> **"We're building a factual feedback loop for LLMs, not an AI refactoring engine."**

That's it.

---

## OdinCode Design Principles

### 1. Compiler as Final Judge
- No step completes until compiler passes
- rust-analyzer validates every change
- Failures block progress, not enable workarounds

### 2. Externalized Memory
- Every tool call result: stored
- Every reasoning step: persisted
- Every decision: logged with evidence
- Every file change: tracked before/after

### 3. Zero Guessing Enforcement
- LLM cannot move forward without mapper facts
- Splice cannot apply without symbol validation
- Changes cannot persist without compiler approval
- Loop repeats until validation passes

### 4. Complete Traceability
For any refactoring, we can answer:
- What did the LLM decide? (with reasoning)
- What facts was it based on? (mapper queries)
- What did Splice change? (exact spans)
- What did the compiler say? (validation output)
- What broke in reality? (dependency graph)

---

## The Missing Piece: Codebase Mapper

**Splice has the execution engine but lacks the mapping layer.**

### Splice Can:
- Patch symbols at exact spans
- Execute multi-step plans
- Validate with rust-analyzer

### But It Doesn't:
- Track "what affects what" across entire codebase
- Know impact before applying changes
- Remember previous refactorings
- Provide situational awareness

### Mapper Provides:
- **Before change**: "If I touch this symbol, here's everything that breaks"
- **After change**: "Here's what actually broke in reality"
- **For LLM**: Complete dependency graph for reasoning
- **For validation**: Ground truth to compare against

---

## Implementation Requirements

### Codebase Mapper Must:

1. **Index Everything**
   - All symbols (functions, structs, enums, traits, impls)
   - All references between symbols
   - All file locations and spans
   - Store in SQLiteGraph with HNSW for semantic search

2. **Answer Questions**
   - "What references this symbol?"
   - "What does this symbol depend on?"
   - "If I change this, what breaks?"
   - "Where else does this pattern appear?"

3. **Track Changes**
   - Before/after state for every refactoring
   - Compiler validation results
   - Dependency impact analysis
   - Persistent history in knowledge graph

4. **Provide Ground Truth**
   - No inference, no guessing
   - Facts extracted from real code
   - Validated against compiler
   - Queryable by LLM for decision-making

### OdinCode Workflow Must:

1. **Forbid Guessing**
   - LLM cannot proceed without mapper facts
   - Splice cannot execute without symbol validation
   - No "hope this works" steps

2. **Enforce Validation**
   - Compiler = final judge
   - No partial success
   - Loop until clean compile

3. **Remember Everything**
   - Every tool call result → memory
   - Every reasoning step → memory
   - Every file change → memory
   - Every validation → memory

4. **Enable Rollback**
   - If refactoring fails, revert to last known good state
   - Use persistent history to undo changes
   - Never lose working code

---

## Why This Approach Wins

**Other tools try to be smart.**
**Our stack is relentlessly factual.**

- Splice: Span-accurate transformations
- Mapper: Complete dependency graph
- LLM: Intelligence layer on top of perfect facts
- Compiler: Final validation

**Zero guessing at every step.**

---

## Next Steps

1. **Build Codebase Mapper**
   - Parse Rust code with tree-sitter
   - Extract symbols and references
   - Store in SQLiteGraph
   - Provide query API

2. **Integrate with Splice**
   - Mapper provides facts to LLM
   - LLM generates Splice plans
   - Splice executes with validation
   - Mapper stores results

3. **Create OdinCode Interface**
   - Enforce the workflow
   - Prevent guessing
   - Loop until compiler passes
   - Externalize all memory

---

## Conclusion

This is why our work feels different, slower to explain, but ultimately much more solid than what most people are building.

We're not building an AI refactoring engine.

We're building the **factual feedback loop** that makes AI-powered refactoring actually work.

**No guessing. No drifting. No hallucinating. Just facts, intelligence, execution, validation, repeat.**

---

*This document will live in OdinCode as the architectural foundation that every tool and workflow must follow.*
