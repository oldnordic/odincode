# Tool Classification Investigation

**Date**: 2025-12-27
**Issue**: User asked to "count files with ls and wc" - system tried file_glob, failed, told user to use "plan mode"

## FACTS from Source Code

### 1. TOOL_WHITELIST (`src/llm/router.rs:16-37`)

```rust
pub const TOOL_WHITELIST: &[&str] = &[
    "bash_exec",
    "display_text",
    "execution_summary",
    "file_create",
    "file_edit",
    "file_glob",      // ← In whitelist
    "file_read",
    "file_search",
    "file_write",
    "git_diff",
    "git_log",
    "git_status",
    "lsp_check",
    "memory_query",
    "references_from_file_to_symbol_name",
    "references_to_symbol_name",
    "splice_patch",
    "splice_plan",
    "symbols_in_file",
    "wc",             // ← In whitelist
];
```

**Total**: 20 tools

### 2. AUTO_TOOLS (`src/execution_engine/chat_tool_runner.rs:16-27`)

```rust
pub const AUTO_TOOLS: &[&str] = &[
    "file_read",
    "file_search",
    "file_glob",      // ← AUTO
    "symbols_in_file",
    "references_to_symbol_name",
    "references_from_file_to_symbol_name",
    "lsp_check",
    "count_files",
    "count_lines",
    "fs_stats",
];
```

**Total**: 10 tools

**`wc` is NOT in AUTO_TOOLS!**

### 3. GATED_TOOLS (`src/execution_engine/chat_tool_runner.rs:30`)

```rust
pub const GATED_TOOLS: &[&str] = &["file_write", "file_create"];
```

**Total**: 2 tools

### 4. FORBIDDEN_TOOLS (`src/execution_engine/chat_tool_runner.rs:33`)

```rust
pub const FORBIDDEN_TOOLS: &[&str] = &["splice_patch", "splice_plan"];
```

**Total**: 2 tools

### 5. PromptMode.allowed_tools() (`src/llm/types.rs:72-87`)

```rust
pub fn allowed_tools(&self) -> &'static [&'static str] {
    match self {
        PromptMode::Query => &[
            "count_files", "count_lines", "fs_stats", "wc", "memory_query",
        ],
        PromptMode::Explore => &[
            "file_search", "file_glob", "symbols_in_file",
            "references_to_symbol_name", "references_from_file_to_symbol_name",
            "file_read",
        ],
        PromptMode::Mutation => &[
            "memory_query", "magellan_query", "file_edit", "splice_patch",
            "lsp_check", "bash_exec",
        ],
        PromptMode::Presentation => &[],
    }
}
```

**Key**: `wc` is ONLY allowed in Query mode!

### 6. ChatToolRunner.classify_tool() (`src/execution_engine/chat_tool_runner.rs:121-133`)

```rust
pub fn classify_tool(&self, tool: &str) -> ChatToolCategory {
    if FORBIDDEN_TOOLS.contains(&tool) {
        return ChatToolCategory::Forbidden;
    }
    if GATED_TOOLS.contains(&tool) {
        return ChatToolCategory::Gated;
    }
    if AUTO_TOOLS.contains(&tool) {
        return ChatToolCategory::Auto;
    }
    // Unknown tools are forbidden
    ChatToolCategory::Forbidden  // ← `wc` falls through to here!
}
```

## ROOT CAUSE

**`wc` has INCONSISTENT classification across the codebase:**

| Location | Status |
|----------|--------|
| TOOL_WHITELIST | ✅ Included |
| AUTO_TOOLS | ❌ NOT included |
| GATED_TOOLS | ❌ NOT included |
| FORBIDDEN_TOOLS | ❌ NOT included |
| PromptMode::Query allowed_tools | ✅ Included |
| PromptMode::Explore allowed_tools | ❌ NOT included |
| PromptMode::Mutation allowed_tools | ❌ NOT included |

**What happens when user says "count files with wc":**

1. `classify_prompt_mode("count files with wc")` returns:
   - "count" is in QUERY_KEYWORDS → Should return Query mode
   - BUT check line 107: QUERY_KEYWORDS includes "count"
   - So it SHOULD work...

2. **Wait - the actual bug might be different**. Let me check the keyword list again.

## QUERY_KEYWORDS (`src/llm/router.rs:106-110`)

```rust
const QUERY_KEYWORDS: &[&str] = &[
    "how many", "how much", "count", "total", "sum", "number of",
    "lines of", "loc", "size of", "statistics", "stats",
    "frequency", "occurrences", "average", "median",
];
```

**"count" IS in QUERY_KEYWORDS (line 107)**

So "count files with wc" should match Query mode...

## UNANSWERED QUESTIONS

1. Why did the system tell the user to use "plan mode"?
   - "plan mode" is a CLI mode, not a chat PromptMode
   - The error messages don't mention "plan mode"
   - Did the LLM hallucinate this?

2. What is the ACTUAL error the user saw?
   - Need to check /tmp/odincode_debug.log
   - Need to see what the LLM response actually was

3. Is there another classification path I'm missing?

## DEBUG LOG ANALYSIS

From `/tmp/odincode_debug.log`:

```
[CHAT_LOOP] Complete event: response_len=456, has_tool_call=false
Full response: INSUFFICIENT_EVIDENCE: The tool `file_glob` is required...
Please execute the proposed plan to retrieve this information.

```json
{
  "plan_id": "count_rs_files_in_src",
  "intent": "QUERY",
  ...
}
```
```

**Key finding**: The LLM responded with "INSUFFICIENT_EVIDENCE" format which is from PLANNING mode (`src/llm/contracts/prompts.rs:56`), NOT chat mode.

## MESSAGE FLOW ANALYSIS

**Correct flow** (what SHOULD happen):
1. `frame_stack.build_messages()` → calls `chat_system_prompt()` (line 157)
2. `chat_system_prompt()` → says "TOOL_CALL format" (prompts.rs:95-113)
3. LLM responds with `TOOL_CALL: tool: file_glob args: ...`

**Actual flow** (what IS happening):
1. Messages are built with `chat_system_prompt()`
2. LLM responds with PLANNING format (INSUFFICIENT_EVIDENCE + JSON plan)
3. `has_tool_call()` returns false (not TOOL_CALL format)
4. Loop completes without executing tool

## ROOT CAUSE

The LLM (GLM-4 via api.z.ai) is IGNORING the chat_system_prompt() instructions and responding with PLANNING format instead. This is likely because:
- The LLM has been fine-tuned on planning prompts
- The chat_system_prompt() doesn't explicitly forbid planning format
- There's conflicting instructions in the prompts

## THE FIX

The `chat_system_prompt()` needs to be MORE EXPLICIT about NOT using planning format:

```rust
pub fn chat_system_prompt() -> String {
    r#"You are in CHAT mode. You MUST use TOOL_CALL format, NOT planning format.

TOOL_CALL FORMAT (REQUIRED):
TOOL_CALL:
  tool: <tool_name>
  args:
    <key>: <value>

DO NOT respond with:
- "INSUFFICIENT_EVIDENCE"
- JSON plans with plan_id/intent/steps
- Planning mode format

Use the TOOL_CALL format above when you need to use a tool."#
}
```

## NEXT STEPS

1. Update `chat_system_prompt()` to explicitly forbid planning format
2. Test that LLM responds with TOOL_CALL format
3. Add test case for "count files" workflow
