# Implementation Gap Analysis: GLM Capabilities vs. OdinCode

**Date**: 2025-12-26
**Purpose**: Compare GLM documented capabilities with OdinCode implementation and external research

---

## Executive Summary

| Category | GLM Offers | OdinCode Has | Gap |
|-----------|------------|--------------|-----|
| **Deep Thinking** | ✅ Exposed reasoning | ❌ Not integrated | Store reasoning_content in DB |
| **Streaming** | ✅ SSE output | ✅ Implemented | None |
| **Structured Output** | ✅ JSON mode | ❌ Not enforced | Add response_format to requests |
| **Context Caching** | ✅ Auto ~50% savings | ⚠️ Partial (FrameStack) | Align with GLM's automatic approach |
| **Function Calling** | ✅ Tools + streaming | ✅ Implemented | Add tool_stream (glm-4.6) |
| **Tool Streaming** | ✅ GLM-4.6 only | ❌ Not implemented | Low priority |

---

## Detailed Comparison

### 1. Deep Thinking / Reasoning Content

#### What GLM Provides

```python
# GLM API response structure
{
  "choices": [{
    "message": {
      "reasoning_content": "Step-by-step reasoning...",
      "content": "Final answer",
      "thinking_tokens": 127
    }
  }]
}
```

**Key insight**: The model's internal reasoning is **exposed separately** from the final answer.

#### What OdinCode Has

- **Phase 9.8**: `strip_reasoning_content()` — strips reasoning chunks from display
- **Phase 9.8**: `normalize.rs` (286 LOC) — handles reasoning content in streaming

**Current behavior**: We **strip and discard** reasoning content instead of storing it.

```rust
// src/llm/adapters/normalize.rs (Phase 9.8)
pub fn strip_reasoning_content(input: &str) -> String {
    // Strips <reasoning>, [REASONING], Thinking: prefixes
    // Result: reasoning content is LOST
}
```

#### Gap Identified

❌ **Critical Gap**: Reasoning content is discarded instead of persisted to database.

**Why it matters** (from `CURRENT_CAPABILITIES.md`):
- "No record of which mutations caused errors"
- "Cannot trace 'error E0425 was caused by splice_patch on src/lib.rs:123'"
- "No evidence-based constraint enforcement"

**Solution** (from `GLM_COMPLETE_CAPABILITIES.md`):
> "Store `reasoning_content` in execution_log for debugging mode classification"

**Implementation path**:
1. Add `reasoning_content` column to `executions` table
2. Update `strip_reasoning_content()` to return both stripped text AND original reasoning
3. Log reasoning to execution_db when recording LLM calls

---

### 2. Streaming Messages (SSE)

#### What GLM Provides

```python
for chunk in response:
    delta = chunk.choices[0].delta
    if hasattr(delta, 'content') and delta.content:
        full_content += delta.content
    if hasattr(delta, 'reasoning_content') and delta.reasoning_content:
        reasoning += delta.reasoning_content
```

**SSE format**: Server-Sent Events with `delta.content` chunks

#### What OdinCode Has

✅ **Phase 5.0**: HTTP adapters with SSE streaming
- `src/llm/adapters/openai.rs` (293 LOC) — OpenAI-compatible SSE
- `src/llm/adapters/glm.rs` (103 LOC) — GLM adapter wrapper
- `src/llm/adapters/ollama.rs` (317 LOC) — NDJSON streaming

✅ **Phase 9.8**: `parse_sse_chunk()` in normalize.rs (286 LOC)
```rust
pub fn parse_sse_chunk(
    chunk: &str,
    state: &mut StreamingState,
) -> Vec<StreamingEvent>
```

**Gap**: ✅ **None** — Streaming is fully implemented.

---

### 3. Structured Output (JSON Mode)

#### What GLM Provides

```python
response = client.chat.completions.create(
    model="glm-4.7",
    messages=[...],
    response_format={"type": "json_object"}  # ← Guarantees JSON
)
result = json.loads(response.choices[0].message.content)
```

**Key benefit**: "reliable guarantees for programmatic processing"

#### What OdinCode Has

❌ **Not enforced** — We send plain requests without JSON mode enforcement.

**Current implementation** (Phase 5.0):
```rust
// src/llm/adapters/openai.rs
let body = json!({
    "model": self.model,
    "messages": messages,
    "stream": stream,
    // NO response_format here
});
```

#### Gap Identified

⚠️ **Medium Gap**: No structured output enforcement for tool responses.

**Why it matters**:
- Tool results must be parsed from natural language
- No guarantee of parseable format
- "What actually happened when this exact kind of action was tried before?" becomes fuzzy

**Solution**:
```rust
// Add to requests when structured output needed
if needs_structured_output {
    body["response_format"] = serde_json::json!({"type": "json_object"});
}
```

---

### 4. Context Caching

#### What GLM Provides

**Automatic caching** — no manual configuration needed:
- Repeated system prompts → ~50% cost reduction
- Repeated document content → automatic cache hit
- Visible via `usage.prompt_tokens_details.cached_tokens`

```python
# First call: pays full price
response1 = client.chat.completions.create(...)

# Second call with same system prompt: gets cache discount
response2 = client.chat.completions.create(...)
cached_tokens = response2.usage.prompt_tokens_details.cached_tokens  # 800 tokens saved
```

#### What OdinCode Has

⚠️ **Partial** — FrameStack (Phase 9.7) implements manual caching:
- System prompt is cached across calls
- But no tracking of `cached_tokens` usage
- No automatic detection of repeated content

**Gap**: ⚠️ **Medium Gap** — Manual vs automatic, no cost visibility.

---

### 5. Function Calling

#### What GLM Provides

```python
tools = [{
    "type": "function",
    "function": {
        "name": "get_weather",
        "description": "Get weather for city",
        "parameters": {
            "type": "object",
            "properties": {
                "city": {"type": "string"},
                "unit": {"type": "string", "enum": ["celsius", "fahrenheit"]}
            },
            "required": ["city"]
        }
    }
}]

response = client.chat.completions.create(
    model="glm-4.7",
    messages=[...],
    tools=tools,
    tool_choice="auto"
)

# Handle tool_calls
for tool_call in response.choices[0].message.tool_calls:
    args = json.loads(tool_call.function.arguments)
    result = get_weather(args["city"])
```

#### What OdinCode Has

✅ **Implemented** — But differently:
1. **Phase 2**: Plan-based workflow (NOT direct tool calling)
2. **Phase 9.2**: GATED tools with approval workflow
3. **Phase 9.8**: Message normalization for tool loops

**OdinCode's approach**:
```
User input → LLM generates Plan → User approves → Execute tools
```

**GLM/OpenAI's approach**:
```
User input → LLM calls tools directly → Return results → LLM responds
```

**Gap**: ✅ **None** — Different architecture, not a gap.
- OdinCode: Explicit plan approval (safer, auditable)
- GLM: Direct tool calling (faster, less control)

---

### 6. Tool Streaming (GLM-4.6 Only)

#### What GLM Provides

```python
response = client.chat.completions.create(
    model="glm-4.6",  # Only model with tool streaming
    stream=True,
    tool_stream=True,  # ← Key parameter
    tools=[...]
)

# Tool parameters stream incrementally
for chunk in response:
    if delta.tool_calls:
        # Arguments arrive piece by piece
        final_tool_calls[index].function.arguments += chunk
```

**Benefit**: "Start tool execution before full response completes"

#### What OdinCode Has

❌ **Not implemented** — We wait for full response before parsing tool calls.

**Gap**: ⚠️ **Low Priority** — Optimization, not functionality.
- Current: Wait for full tool_call JSON
- Improvement: Could start tool execution as args stream in
- Trade-off: Complexity vs. latency reduction (~100-200ms)

---

## Cross-Cutting Analysis

### What Claude Code Does (from LEAK docs)

**Key patterns from `CLAUDE_CODE_SYSTEM_PROMPTS_LEAK.md`:**

1. **Modular prompts** (40+ fragments)
   - OdinCode: ❌ Single system prompt + mode prompts
   - Gap: Could modularize internal prompts

2. **READ-ONLY enforcement**
   - Claude Code: Explore/Plan agents CANNOT write
   - OdinCode: ✅ PromptMode enforces this (Phase 9.9)

3. **TodoWrite pattern** (2,167 tokens of examples)
   - Shows when AND when not to use
   - OdinCode: ❌ No task tracking in LLM context

4. **Plan Mode workflow** (5-phase)
   - Understanding → Design → Review → Final Plan → Execute
   - OdinCode: ✅ Similar (Generate → Edit → Approve → Execute)

### What Anthropic Recommends (from ENGINEERING_DECISIONS.md)

**Progressive Tool Discovery**:
- Load only core tools initially
- Discover specialized tools on-demand
- OdinCode: TOOL_WHITELIST (20 tools) always loaded
- Gap: Could implement tool search for token savings

**Programmatic Orchestration**:
- LLM writes code to orchestrate tools
- 37% token reduction for complex workflows
- OdinCode: ❌ Not implemented
- Gap: Future optimization for bulk operations

**Usage Examples Over Schema**:
- Show when AND when not to use each tool
- OdinCode: ⚠️ Partial (some tools have examples)
- Gap: Comprehensive examples for all 20 tools

---

## OdinCode Strengths (vs. GLM/Claude Code)

| Feature | OdinCode | GLM/Claude Code | Advantage |
|---------|-----------|-----------------|------------|
| **Plan approval** | ✅ User approves before execution | ❌ Direct execution | OdinCode safer |
| **Evidence logging** | ✅ Full audit trail | ⚠️ Limited | OdinCode more auditable |
| **Tool whitelisting** | ✅ 20 tools, explicit | ❌ Open access | OdinCode more controlled |
| **GATED tools** | ✅ file_write/file_create require approval | ❌ No gating | OdinCode safer |
| **READ-ONLY modes** | ✅ PromptMode enforcement | ⚠️ Prompt discipline | OdinCode enforced |
| **Multi-database** | ✅ SQLite + SQLiteGraph + Neo4j (LTMC) | ❌ Single store | OdinCode richer |
| **FrameStack caching** | ✅ Explicit timeline caching | ⚠️ Auto only | OdinCode more transparent |
| **Tool preconditions** | ✅ Explicit validation | ⚠️ Implicit | OdinCode more robust |

---

## Priority Gaps

### HIGH Priority

1. **Store reasoning_content to database**
   - Why: Debug mode selection, audit decisions, causal tracing
   - Effort: Medium (schema change + logging update)
   - Impact: High

2. **Add structured output enforcement**
   - Why: Reliable tool response parsing
   - Effort: Low (add `response_format` to requests)
   - Impact: Medium

### MEDIUM Priority

3. **Expand tool usage examples**
   - Why: LLM needs to know how to use tools correctly
   - Effort: Medium (3-5 examples per tool × 20 tools)
   - Impact: Medium (reduced tool misuse)

4. **Implement progressive tool discovery**
   - Why: Token savings (85% reduction per Anthropic)
   - Effort: Medium (tool search + lazy loading)
   - Impact: High for long contexts

### LOW Priority

5. **Tool streaming (glm-4.6)**
   - Why: Latency optimization (~100-200ms savings)
   - Effort: Medium (streaming argument assembly)
   - Impact: Low (UX improvement only)

---

## Implementation Roadmap

### Phase A: Reasoning Persistence (HIGH)

1. Schema change:
   ```sql
   ALTER TABLE executions ADD COLUMN reasoning_content TEXT;
   ```

2. Update `strip_reasoning_content()`:
   ```rust
   pub fn split_reasoning_content(input: &str) -> (String, Option<String>) {
       // Returns: (stripped_text, reasoning_content)
   }
   ```

3. Update logging to capture reasoning:
   ```rust
   let (response_text, reasoning) = split_reasoning_content(&llm_output);
   exec_db.record_with_reasoning(exec_id, response_text, reasoning)?;
   ```

### Phase B: Structured Output (MEDIUM)

1. Add `response_format` to LLM adapter:
   ```rust
   pub struct LlmRequest {
       pub messages: Vec<LlmMessage>,
       pub response_format: Option<ResponseFormat>,
       // ...
   }
   ```

2. Use for tool result parsing:
   ```rust
   let response = adapter.generate_with_format(
       messages,
       ResponseFormat::Json  // Enforce JSON
   );
   ```

### Phase C: Tool Examples (MEDIUM)

For each tool in TOOL_WHITELIST, add to `ToolMetadata`:
```rust
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub examples: Vec<ToolExample>,
    pub when_not_to_use: Vec<String>,  // NEW
    pub reasoning: Vec<String>,         // NEW
}
```

---

## Summary Matrix

| Capability | GLM Has | OdinCode Has | Gap | Priority |
|------------|----------|--------------|-----|----------|
| Deep Thinking (reasoning_content) | ✅ | ❌ Stripped | Store in DB | HIGH |
| Streaming (SSE) | ✅ | ✅ Phase 5.0+9.8 | None | — |
| Structured Output (JSON mode) | ✅ | ❌ Not enforced | Add to requests | MEDIUM |
| Context Caching (automatic) | ✅ | ⚠️ Manual (FrameStack) | Token visibility | MEDIUM |
| Function Calling | ✅ | ✅ Different architecture | — | — |
| Tool Streaming (glm-4.6) | ✅ | ❌ Not implemented | Low priority | LOW |
| Tool Examples | ✅ Best practices | ⚠️ Partial | Comprehensive examples | MEDIUM |
| Modular Prompts | ✅ 40+ fragments | ⚠️ Mode prompts | Consider | LOW |
| Progressive Discovery | ✅ 85% token savings | ❌ All tools loaded | Tool search | MEDIUM |

---

## Decision Matrix

| Gap | Effort | Impact | Dependencies | Recommendation |
|-----|--------|-------|--------------|----------------|
| Store reasoning_content | Medium | High | Phase 0.5 schema | **DO IT** |
| Structured output | Low | Medium | Phase 5 adapters | **DO IT** |
| Tool examples | Medium | Medium | Phase 9.x | **CONSIDER** |
| Progressive discovery | Medium | High | Phase 5 factory | **CONSIDER** |
| Tool streaming | Medium | Low | Phase 9.8 streaming | **DEFER** |
| Modular prompts | High | Low | Phase 9.9 prompts | **DEFER** |

---

*Last Updated: 2025-12-26*
*Analysis based on: GLM_COMPLETE_CAPABILITIES.md, CURRENT_CAPABILITIES.md, CORE_LOOP.md, TODO.md, ENGINEERING_DECISIONS.md, CLAUDE_CODE_SYSTEM_PROMPTS_LEAK.md*
