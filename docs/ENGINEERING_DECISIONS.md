# Engineering Decisions: Advanced Tool Orchestration

**Source Material**:
- [Advanced Tool Use](https://www.anthropic.com/engineering/advanced-tool-use) (Anthropic Engineering, 2024)
- [Code Execution with MCP](https://www.anthropic.com/engineering/code-execution-with-mcp) (Anthropic Engineering, 2024)

**Documented**: 2025-12-26
**Context**: OdinCode internal tool architecture (file_read, splice_patch, memory_query, etc.)

---

## Executive Summary

These articles describe three architectural patterns for efficient LLM tool orchestration. While originally presented in the context of MCP (Model Context Protocol), the principles apply universally to any tool system—including our internal tools like `file_read`, `splice_patch`, `memory_query`, `lsp_check`, and the magellan/symbol query tools.

**Key Metrics from Source**:
- Tool Search: 85% reduction in token usage for tool definitions
- Programmatic Calling: 37% reduction in total tokens for complex workflows
- Progressive Disclosure: 98.7% reduction (150K → 2K tokens) in one case

---

## Pattern 1: Progressive Tool Discovery

### The Problem

Loading all tool definitions upfront consumes massive context before any work begins:

```
Traditional approach:
- All tool definitions loaded upfront (~72K tokens for 50+ tools)
- Conversation history competes for remaining space
- Context consumption: ~77K tokens before work begins
```

**Analogy to OdinCode**: If we expose all 20+ tools with full schemas, examples, and documentation upfront, we're burning 10-20K tokens before the LLM even reads the user's request.

### The Solution

Discover tools on-demand rather than stuffing everything into context:

```
With discovery:
- Only core search tool loaded (~500 tokens)
- Tools discovered as needed (3-5 relevant tools, ~3K tokens)
- Total: ~8.7K tokens, preserving 95% of context window
```

**Implementation Pattern**:

1. **Always-loaded core tools** (the "toolbox"):
   - `file_search` — find files by pattern
   - `file_glob` — find files by glob pattern
   - `help` — discover available capabilities

2. **Discoverable tools** (loaded on-demand):
   - `splice_patch` — only when editing symbols
   - `lsp_check` — only when diagnosing
   - `symbols_in_file` — only when querying code structure
   - `memory_query` — only when grounding in execution history

3. **Search mechanism**:
   - Keyword-based search on tool names and descriptions
   - Category tags (edit, query, diagnostic, git)
   - Detail levels: name-only → summary → full schema

### Decision for OdinCode

**ADOPT**: Implement a tool discovery system where:
- Core tools (file_search, file_glob, file_read) are always visible
- Specialized tools (splice_patch, lsp_check, magellan queries) are discovered via search
- PromptMode (Phase 9.9) already implements similar constraint logic—extend to discovery

**Rationale**: Our PromptMode system already constrains tools per mode. Progressive discovery is the natural next step: load only tools allowed in current mode.

---

## Pattern 2: Programmatic Orchestration

### The Problem

Traditional tool calling creates two fundamental issues:

1. **Context pollution from intermediate results**
   - Analyzing a 10MB log file? Entire file enters context.
   - Fetching customer data across tables? Every record accumulates.
   - These intermediate results consume tokens regardless of relevance.

2. **Inference overhead and manual synthesis**
   - Each tool call = full model inference pass
   - LLM must "eyeball" data to extract relevant info
   - Natural language orchestration is error-prone

### The Solution

Let the LLM write code that orchestrates tools directly:

```python
# Instead of 5 separate tool calls with 5 inference passes:
team = await get_team_members("engineering")
expenses = await asyncio.gather(*[
    get_expenses(m["id"], "Q3") for m in team
])
exceeded = [(m["name"], sum(e["amount"] for e in exp))
            for m, exp in zip(team, expenses)
            if sum(e["amount"] for e in exp) > budget[m["level"]]]
print(json.dumps(exceeded))
```

**Benefits**:
- Parallel execution (`asyncio.gather`)
- Data stays in execution environment, not LLM context
- Only final output reaches the LLM
- Reduced from 43,588 to 27,297 tokens (37% reduction)

### Decision for OdinCode

**DEFER**: Programmatic orchestration is compelling but introduces complexity:
- Requires secure code execution environment
- Adds infrastructure overhead
- Our current AUTO/GATED tool categories provide simpler orchestration

**FUTURE PATH**: If we add code execution (e.g., for bash_exec scripts), consider:
- Marking tools as `allowed_callers: ["code_execution"]`
- Using code for bulk operations (e.g., batch file edits)
- Keeping simple workflows as direct tool calls

---

## Pattern 3: Usage Examples Over Pure Schema

### The Problem

JSON Schema defines structure, not usage patterns:

```json
{
  "description": "Create a support ticket",
  "parameters": {
    "due_date": {"type": "string"},
    "priority": {"enum": ["low", "medium", "high", "critical"]}
  }
}
```

Unanswered questions:
- What format for `due_date`? "2024-11-06" or "Nov 6, 2024"?
- When should `priority` be "critical" vs "high"?
- What conventions exist for optional fields?

### The Solution

Include concrete usage examples in tool definitions:

```json
{
  "name": "splice_patch",
  "description": "Replace a symbol's definition with new code",
  "input_schema": {...},
  "input_examples": [
    {
      "file": "src/main.rs",
      "symbol": "process_data",
      "kind": "FunctionDefinition",
      "with_file": "/tmp/new_definition.rs"
    }
  ]
}
```

**Results**: Accuracy improved from 72% to 90% on complex parameter handling.

### Decision for OdinCode

**ALREADY IMPLEMENTED**: Our `ToolMetadata` includes examples (Phase 9.6):
```rust
pub struct ToolMetadata {
    pub examples: Vec<ToolExample>,
}
```

**ACTION ITEM**: Ensure all 20+ tools have comprehensive examples covering:
- Minimal invocation (required params only)
- Typical usage (common optional params)
- Edge cases (error conditions, special values)

---

## Cross-Cutting Insights

### 1. Layer Features Strategially

Don't use all patterns everywhere. Start with the biggest bottleneck:

| Symptom | Solution |
|---------|----------|
| Tool definitions consuming >10K tokens | Progressive Discovery |
| Large intermediate results | Programmatic Orchestration |
| Parameter errors, malformed calls | Usage Examples |

### 2. Prompt Caching Compatibility

Deferred tool loading doesn't break prompt caching because:
- Deferred tools are excluded from initial prompt
- Only added after search completes
- System prompt + core tools remain cacheable

**OdinCode implication**: Our FrameStack (Phase 9.7) already supports caching—the system prompt and core frames are cacheable.

### 3. Security Considerations

Programmatic execution introduces new attack vectors:
- Code injection via tool parameters
- Resource exhaustion (infinite loops)
- Data exfiltration through side channels

**Mitigation**:
- Sandboxed execution environment
- Resource limits (CPU, memory, time)
- Input sanitization for all tool parameters

### 4. Privacy-Preserving Operations

Intermediate results staying in execution environment = privacy benefit:

```
Google Sheets → [execution env] → Salesforce
                      ↓
                  Model only sees "Updated 1000 leads"
```

**OdinCode application**: `splice_patch` could operate on sensitive code without LLM seeing full file contents—only reporting "Patched 3 functions".

---

## Specific Recommendations for OdinCode

### Immediate (Phase 9.10+)

1. **Add tool search capability**
   ```rust
   pub struct ToolSearch {
       pub name: String,
       pub category: ToolCategory,
       pub keywords: Vec<String>,
   }
   ```

2. **Expand `ToolMetadata` examples**
   - Each tool needs 3-5 examples
   - Cover minimal, typical, and edge case usage
   - Include expected output formats

3. **Extend PromptMode with discovery**
   - Query mode: show only aggregate tools
   - Explore mode: show only read/query tools
   - Mutation mode: show only edit tools (after mode switch)

### Medium Term

1. **Consider programmatic orchestration for bulk ops**
   - Batch file editing
   - Multi-file refactoring
   - Bulk symbol queries

2. **Tool result summarization**
   - Large file reads → summarize before LLM sees
   - Symbol query results → deduplicate, cluster
   - Git logs → extract relevant commits only

### Out of Scope

- Full code execution environment (infrastructure heavy)
- Dynamic tool loading (requires tool registry)
- Multi-agent tool sharing (coordination complexity)

---

## References

- Anthropic Engineering. "Introducing advanced tool use on the Claude Developer Platform". https://www.anthropic.com/engineering/advanced-tool-use
- Anthropic Engineering. "Code execution with MCP: building more efficient AI agents". https://www.anthropic.com/engineering/code-execution-with-mcp
- OdinCode Phase 9.9: Internal Control Prompts (PromptMode enforcement)
- OdinCode Phase 9.6: Tool Examples in ToolMetadata
- OdinCode Phase 9.7: FrameStack with prompt caching support
