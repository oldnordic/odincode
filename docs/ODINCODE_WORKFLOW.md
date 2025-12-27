# OdinCode Workflow: Magellan + Splice Factual Refactoring

**Status**: Operational Workflow
**Created**: 2025-12-24
**Core Principle**: **Zero Guessing** - Facts first, precision execution, validation always.

---

## The Loop

```
1. OBSERVE (Magellan) → Get facts: What exists? What affects what?
2. REASON (LLM + Facts) → Plan refactoring based on dependencies
3. EXECUTE (Splice) → Span-safe changes with validation
4. VALIDATE (Compiler + Magellan) → Verify integrity
→ Repeat until clean
```

---

## Quick Reference

### Setup (One-Time)

```bash
# Keep Magellan running
magellan watch --root /home/feanor/Projects/syncore --db /home/feanor/Projects/syncore/syncore_codegraph.db > /tmp/magellan.log 2>&1 &

# Trigger initial indexing
find src -name "*.rs" -exec touch {} \; && sleep 5

# Verify
magellan watch --root . --db codegraph.db --status
# Output: files: X, symbols: Y, references: Z
```

### Essential Magellan Queries

```bash
# 1. What will break if I change X?
sqlite3 codegraph.db \
  "SELECT e1.name FROM graph_entities e1
   JOIN graph_edges g ON e1.id = g.from_id
   JOIN graph_entities e2 ON g.to_id = e2.id
   WHERE e2.name = 'SymbolName'
   AND g.edge_type = 'REFERENCES';"

# 2. Where is X defined?
sqlite3 codegraph.db \
  "SELECT e1.name FROM graph_entities e1
   JOIN graph_edges g ON e1.id = g.from_id
   JOIN graph_entities e2 ON g.to_id = e2.id
   WHERE e2.name = 'SymbolName'
   AND g.edge_type = 'DEFINES';"

# 3. What does X depend on?
sqlite3 codegraph.db \
  "SELECT e2.name FROM graph_entities e2
   JOIN graph_edges g ON e2.id = g.to_id
   JOIN graph_entities e1 ON g.from_id = e1.id
   WHERE e1.name = 'SymbolName'
   AND g.edge_type = 'REFERENCES';"

# 4. Graph stats
sqlite3 codegraph.db \
  "SELECT kind, COUNT(*) FROM graph_entities GROUP BY kind;"
```

### Splice Commands

```bash
# Single symbol replacement
splice patch \
  --file src/vector.rs \
  --symbol VectorStore \
  --kind struct \
  --with new_vectorstore.rs \
  --analyzer rust-analyzer

# Multi-step plan
splice plan --file refactor_plan.json
```

### Validation

```bash
# Compilation
cargo check 2>&1 | grep "^error\[E" | wc -l  # Should be 0

# Graph integrity
sqlite3 codegraph.db "SELECT COUNT(*) FROM graph_entities WHERE kind = 'Symbol';"  # Should not decrease

# Tests
cargo test --lib

# Magellan log
tail -50 /tmp/magellan.log | grep ERROR  # Should be empty
```

---

## Complete Example: Refactor VectorStore

### Step 1: OBSERVE

```bash
# What depends on VectorStore?
sqlite3 syncore_codegraph.db <<'SQL'
SELECT e1.name
FROM graph_entities e1
JOIN graph_edges g ON e1.id = g.from_id
JOIN graph_entities e2 ON g.to_id = e2.id
WHERE e2.name = 'VectorStore'
AND g.edge_type = 'REFERENCES';
SQL
```

Output: `router.rs, code_suite.rs, mcp_server.rs` (15 files total)

### Step 2: REASON

**Prompt**: "VectorStore depends on HnswVectorIndex, WarmupController. 15 files reference it. Plan to replace HnswVectorIndex with sqlitegraph::hnsw::HnswIndex."

**Plan**:
1. Create sqlitegraph wrapper
2. Replace HnswVectorIndex type alias
3. Update VectorStore struct
4. Update 15 reference sites

### Step 3: EXECUTE

```bash
# Apply with Splice
splice patch --file src/vector.rs --symbol VectorStore --kind struct --with new_vectorstore.rs --analyzer rust-analyzer
```

### Step 4: VALIDATE

```bash
cargo check && sqlite3 syncore_codegraph.db "SELECT COUNT(*) FROM graph_entities WHERE kind = 'Symbol';" && cargo test
```

Result: ✅ All pass

---

## Rules

### DO ✅
- Query Magellan FIRST
- Use Splice for changes
- Validate after EVERY step
- Keep Magellan running

### DON'T ❌
- Don't read code and guess
- Don't edit files directly
- Don't skip validation
- Don't stop Magellan

---

## Mantra

> **"I query Magellan for facts. I plan with facts. I execute with Splice. I validate. I repeat."**

**No guessing. Just facts.**
