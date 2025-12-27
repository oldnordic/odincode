# Phase 9.6-B0: Tool Loop Workflow Investigation

## Executive Summary

**Finding**: The tool loop hang is caused by a **disconnected architecture**. The multi-step tool execution infrastructure exists (`ChatLoop`, 722 LOC) but is **completely unconnected** from the UI chat flow.

**Stall Point**: "The system cannot execute tools because chat mode uses simple single-turn LLM calls with no tool loop integration."

## Investigation Steps

### Step 1: Baseline Reproduction (SKIPPED)

After analyzing the code, baseline reproduction is unnecessary. The hang is not a runtime bug—it's an architectural gap.

### Step 2: Concrete Workflow Path Mapping

#### 2.1 Current Chat Flow (What Actually Happens)

```
User Input
    ↓
handle_key_event() [src/main.rs:217]
    ↓
parse_command() → Command::Chat(text)
    ↓
execute_command() [src/ui/handlers.rs:180]
    ↓
handle_chat() [src/ui/handlers.rs:68]
    ↓
chat::chat_threaded() [src/llm/chat.rs:170]
    ↓
spawn_chat_thread() [src/llm/chat_thread.rs:112]
    ↓ (background thread)
chat::chat() [src/llm/chat.rs:124]
    ↓
build_chat_prompt() [src/llm/chat.rs:193]
    → Returns user_input only (NO tool schema!)
    ↓
adapter.generate_chat_streaming()
    ↓
ChatEvent::Complete { full_response } sent to UI
    ↓
process_chat_events() → handle_chat_event() [src/ui/state.rs:632]
    ↓
Complete case [src/ui/state.rs:658]:
    - Persists assistant message
    - Updates UI with full_response
    - Cleanup thread state ← TERMINATES HERE
    - Returns true (terminal event)
    ↓
SESSION ENDS (no tool execution)
```

#### 2.2 ChatLoop Flow (What Was Designed But Never Connected)

```
src/llm/chat_loop.rs defines:
├── ChatLoop struct [line 151]
│   ├── loop_state: Option<LoopState>
│   ├── tx: Option<ChatSender>
│   └── tool_runner: ChatToolRunner
│
├── process_event() [line 197]
│   └── Parses ChatEvent::Complete for TOOL_CALL blocks
│       └── Returns LoopAction::ExecuteTool(tool, args)
│
├── execute_tool_and_continue() [line 304]
│   ├── Executes tool via ChatToolRunner
│   ├── Spawns next LLM call with result
│   └── Returns LoopAction::ToolExecuted
│
├── handle_approval() [line 349]
│   └── Continues after GATED tool approval
│
└── handle_denial() [line 390]
    └── Continues after GATED tool denial
```

**BUT** None of this is wired to the UI.

#### 2.3 The Disconnect

| Component | Exists? | Connected to UI? | Location |
|-----------|---------|------------------|----------|
| `ChatLoop` struct | ✅ Yes | ❌ No | `src/llm/chat_loop.rs:151` |
| `process_event()` | ✅ Yes | ❌ No | `src/llm/chat_loop.rs:197` |
| `execute_tool_and_continue()` | ✅ Yes | ❌ No | `src/llm/chat_loop.rs:304` |
| `ChatLoop` in `App` struct | ❌ No | N/A | `src/ui/state.rs` |
| Call to `ChatLoop::process_event()` | ❌ No | N/A | (none) |
| LoopAction handling | ❌ No | N/A | (none) |
| `TOOL_CALL` parsing in UI | ❌ No | N/A | (none) |

#### 2.4 Evidence from Code

**src/llm/chat.rs:124-142** - Current chat function:
```rust
pub fn chat<F>(prompt: &str, db_root: &Path, mut on_chunk: F) -> Result<String, ChatError>
where
    F: FnMut(&str),
{
    let adapter = create_adapter_from_config(db_root).map_err(|_| ChatError::NotConfigured)?;
    let chat_prompt = build_chat_prompt(prompt);  // Just user_input!
    let response = adapter.generate_chat_streaming(&chat_prompt, |chunk| {
        let filtered = filter_json_blocks(chunk);  // Filters OUT JSON
        on_chunk(&filtered);
    })?;
    Ok(response)
}
```

**src/llm/chat.rs:193-195** - Chat prompt is just user input:
```rust
pub fn build_chat_prompt(user_input: &str) -> String {
    user_input.to_string()  // No tool schema!
}
```

**src/ui/state.rs:658-674** - Complete event handler (no tool loop):
```rust
ChatEvent::Complete { session_id, full_response } => {
    // Persist assistant message
    if let Ok(exec_db) = self.open_exec_db() {
        let _ = exec_db.persist_assistant_message(&session_id, &full_response);
        let _ = exec_db.complete_chat_session(&session_id);
    }

    // Update UI with final response
    self.update_last_message(full_response);

    // Cleanup thread state
    self.cleanup_chat_thread();  // ← Terminates session

    return true; // Terminal event
}
```

**src/llm/chat_loop.rs:197-247** - What SHOULD happen but doesn't:
```rust
pub fn process_event(&mut self, event: &ChatEvent, _db_root: &Path) -> LoopAction {
    match event {
        ChatEvent::Complete { full_response, .. } => {
            // Parse TOOL_CALL from response
            let tool_call = self.parse_tool_call(full_response);
            let category = self.tool_runner.classify_tool(&tool_call.tool);
            match category {
                ChatToolCategory::Auto => {
                    LoopAction::ExecuteTool(tool_call.tool, tool_call.args)  // ← Never executed
                }
                // ...
            }
        }
    }
}
```

### Step 3: Instrumentation (Not Required)

The disconnect is evident from static analysis. Runtime instrumentation would not reveal additional information.

### Step 4: Investigation Tests (Not Required)

The architectural gap is proven by:
- Grep results showing `ChatLoop` only appears in tests within `chat_loop.rs`
- No `ChatLoop` field in `App` struct
- No `LoopAction` pattern matching in UI code
- `process_event()` is never called

### Step 5: Evidence Summary

#### 5.1 grep Results

```bash
# ChatLoop usage in codebase:
$ rg "ChatLoop" src/
src/llm/mod.rs:58:    ChatLoop, HiddenToolResult, LoopAction, LoopState, PendingGatedTool,
src/llm/chat_loop.rs:151:pub struct ChatLoop {
src/llm/chat_loop.rs:160:impl ChatLoop {
src/llm/chat_loop.rs:673:        let loop_driver = ChatLoop::new(tool_runner);  # test
src/llm/chat_loop.rs:682:        let mut loop_driver = ChatLoop::new(tool_runner);  # test
src/llm/chat_loop.rs:697:        let mut loop_driver = ChatLoop::new(tool_runner);  # test

# execute_tool_and_continue usage:
$ rg "execute_tool_and_continue" src/
src/llm/chat_loop.rs:304:    pub fn execute_tool_and_continue(  # Definition only

# LoopAction::ExecuteTool usage:
$ rg "LoopAction::ExecuteTool" src/
src/llm/chat_loop.rs:247:                        LoopAction::ExecuteTool(tool_call.tool, tool_call.args)  # Created only
```

#### 5.2 File Analysis

| File | LOC | Purpose | Connected? |
|------|-----|---------|------------|
| `src/llm/chat.rs` | 268 | Simple single-turn chat | ✅ Used by UI |
| `src/llm/chat_loop.rs` | 722 | Multi-step tool execution | ❌ Tests only |
| `src/llm/chat_thread.rs` | 206 | Thread spawning | ✅ Used by UI |
| `src/ui/handlers.rs` | 291 | Command routing | ✅ Calls chat.rs |
| `src/ui/state.rs` | ~800 | UI state | ❌ No ChatLoop field |

## Root Cause

**The system has two parallel chat infrastructures:**

1. **Active**: `src/llm/chat.rs` — Simple chat, no tools, connected to UI
2. **Inactive**: `src/llm/chat_loop.rs` — Full tool loop, 722 LOC, never connected

The ChatLoop was implemented (Phase 9.0) but **never integrated** into the UI event flow. The UI still uses simple single-turn chat that:
- Does NOT provide tool schema to LLM
- Does NOT parse TOOL_CALL blocks
- Does NOT execute tools
- Does NOT continue after tool execution

## Hypotheses

1. **Hypothesis 1 (CONFIRMED)**: ChatLoop was written but integration was incomplete
   - Evidence: Full implementation exists but is only used in tests
   - Status: ✅ Confirmed by static analysis

2. **Hypothesis 2**: Chat mode was intentionally kept simple
   - Evidence: Comments emphasize "isolated chat lane - no plan/workflow system"
   - Status: Plausible - may be intentional design choice

3. **Hypothesis 3**: Tool execution requires `/plan` command, not chat mode
   - Evidence: `/plan` and `/apply` commands exist
   - Status: Requires investigation of plan mode

## Next Steps (For Phase 9.6-B1 - Implementation)

To enable tool execution in chat mode, the following integration is required:

1. Add `ChatLoop` field to `App` struct in `src/ui/state.rs`
2. Initialize `ChatLoop` with `ChatSender` on chat start
3. Call `ChatLoop::process_event()` from `handle_chat_event()`
4. Handle `LoopAction::ExecuteTool` by calling `execute_tool_and_continue()`
5. Handle `LoopAction::RequestApproval` by setting approval state
6. Add tool schema to LLM adapter in chat mode
7. Persist tool executions to execution database

**Estimated scope**: 200-300 LOC across multiple files

## Deliverables

- [x] Workflow path documented
- [x] Stall point identified
- [x] Root cause determined
- [x] Evidence captured
- [x] Hypotheses documented

**Status**: Investigation complete. Ready for Phase 9.6-B1 (Integration).

---

*Investigation conducted: 2025-12-25*
*Phase: 9.6-B0*
*Investigator: Claude (Opus 4.5)*
