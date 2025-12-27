# Phase 8.6 — Responsive Chat with Full Transcript Archive

**Type**: PLANNING PROPOSAL — Chat Streaming, Persistence, and Context Folding
**Status**: PLANNING ONLY — No Implementation Authorized
**Date**: 2025-12-25
**Dependencies**: Phase 8.1 (Chat Lane Isolation) COMPLETE, Phase 8.5 (Error Isolation) COMPLETE

---

## Executive Summary

Chat currently suffers from two problems:

1. **UI Lag**: Blocking HTTP call freezes main event loop for 10+ seconds before any text appears
2. **Ephemeral History**: Chat transcripts exist only in memory (`Vec<ChatMessage>`), lost on restart

This proposal solves both by:
- Introducing a fire-and-forget chat thread for LLM I/O only
- Handling all persistence on the main thread (SQLite + SQLiteGraph)
- Enabling context folding via compaction for long sessions
- Preserving all execution lane guarantees (synchronous, deterministic, approved)

**Critical**: Execution lane remains completely untouched. Chat is isolated from plan/workflow systems.

---

## OBSERVE — Current State Confirmed

### Blocking Behavior
**Reference**: `src/main.rs:126-146`

The main event loop renders UI, then calls `poll(Duration::from_millis(100))`.
If user presses Enter, `handle_key_event()` calls `execute_command()` which calls
`handle_chat()`. This blocks until HTTP response completes.

**Reference**: `src/ui/handlers.rs:67-112`

`handle_chat()` is synchronous blocking. Calls `chat::chat()` with callback that
updates app state, but `render()` never called until completion.

### In-Memory Only Storage
**Reference**: `src/ui/state.rs:29`

```rust
pub chat_messages: Vec<ChatMessage>,
```

- In-memory only, cleared on restart
- MAX_CHAT_MESSAGES = 200 (line 14) — FIFO eviction when limit reached
- No persistence to any database

### Streaming Works at HTTP Layer
**Reference**: `src/llm/chat.rs:122-140`

`chat()` function calls `adapter.generate_streaming()` with `on_chunk` callback.
Streaming works at HTTP layer but UI can't update during blocking call.

**Reference**: `src/llm/adapters/transport_ureq.rs:77-124`

`post_stream()` does `BufRead::read_line()` in loop. This is blocking I/O.

### Existing Persistence Pattern
**Reference**: `src/execution_tools/db.rs`

`ExecutionDb` manages dual connections:
- `execution_log.db` (SQLite) — temporal log
- `codegraph.db` (SQLiteGraph) — relationships

Auto-creates `execution_log.db` schema if missing.

**Reference**: `src/execution_tools/record.rs`

`record_execution_with_artifacts()` writes to SQLite in transaction, then creates
graph entity. Best-effort dual-write pattern.

### Constraints
**Reference**: `docs/CONTRACT.md:14`

```
NO ASYNC — Synchronous, deterministic execution
```

---

## CONSTRAIN — Invariants

### MUST REMAIN UNCHANGED (Execution Lane — Non-Negotiable)

1. Tools execute ONLY via CORE `execution_engine`
2. Plan approval workflow remains synchronous and blocking
3. Evidence DB queries remain read-only, synchronous
4. `execute_plan()` remains single-threaded, deterministic
5. `/quit` and `Ctrl+C` exit immediately from ANY state
6. All `execution_tools` tests continue passing
7. All `evidence_queries` tests continue passing
8. Deterministic execution logging preserved

### MAY CHANGE (Chat Lane ONLY)

1. Chat I/O may use ONE background thread for HTTP streaming
2. Chat state may use `Arc<Mutex<>>` for thread-safe message access
3. Chat persistence happens on MAIN thread (not in chat thread)
4. Chat messages written to SQLite + SQLiteGraph after completion
5. Chat compaction runs on main thread between sessions

### Responsibilities by Component

**Chat Thread** (Background, Short-Lived):
- LLM HTTP I/O only (ureq blocking calls)
- Sends events via `mpsc::Sender` to main thread
- Checks `Arc<AtomicBool>` shutdown flag on each iteration
- **FORBIDDEN**: NO database writes, NO tool execution, NO plan creation

**Main/UI Thread**:
- Receives chat events via `mpsc::Receiver`
- Updates UI state incrementally on each `Chunk` event
- Persists completed messages to SQLite (after `Final` event)
- Creates graph entities/edges in SQLiteGraph
- Manages chat session lifecycle
- Handles user input during active chat

**CORE Persistence Layer** (New Module):
- `persist_chat_session()` — writes session + messages to SQLite
- `create_chat_graph_entity()` — creates nodes in SQLiteGraph
- `create_chat_edge()` — creates edges in SQLiteGraph
- `should_compact()` — evaluates compaction triggers
- `compact_session()` — generates summary, updates graph

### Data Classification

**TRANSIENT** (Not Persisted):
- Streaming text chunks (individual fragments during response)
- "Thinking..." indicator state
- Partial response state before completion
- In-memory message buffer (evicted per FIFO rule at 200 messages)

**PERSISTENT** (SQLite):
- `chat_sessions`: session_id, start_time, end_time, message_count, compacted flag
- `chat_messages`: id, session_id, role, content, timestamp
- Errors (if any): `ChatError` type stored as diagnostic

**PERSISTENT** (SQLiteGraph):
- `chat_session` entity (kind="chat_session")
- `chat_message` entity (kind="chat_message")
- `chat_summary` entity (kind="chat_summary") — after compaction
- Edges: `ASKED_ABOUT`, `RESPONDED_WITH`, `MENTIONED_FILE`, `COMPACTED_TO`, `SUMMARY_OF`

---

## DECIDE — Architecture Design

### Option A: Fire-and-Forget Chat Thread with Deferred Persistence (RECOMMENDED)

#### Complete Data Flow

**1. User types "hello", presses Enter**
- `parse_command()` returns `Command::Chat("hello")`
- `execute_command()` calls `handle_chat()`

**2. handle_chat() spawns thread (non-blocking)**
- Generate session_id (UUID v4)
- Persist user message to SQLite immediately
- Create `chat_session` graph entity
- `app.set_thinking()` — adds "Thinking..." to `chat_messages`
- Force `render()` — user sees indicator immediately
- Create `Arc<AtomicBool> shutdown_flag = Arc::new(AtomicBool::new(false))`
- Create `mpsc::channel()`
- Spawn thread with `(prompt, db_root, tx, shutdown_flag.clone())`
- Store thread_handle, receiver, shutdown_flag in app state

**3. Chat thread execution**
- Calls `chat::chat()` with callback
- For each chunk received:
  - Check shutdown_flag
  - Send `Chunk(chunk_text)` via tx
- On completion: Send `Final(full_response)` via tx
- On error: Send `Error(chat_error)` via tx
- Thread exits

**4. Main loop iteration (100ms tick)**
- `poll()` returns (key available or timeout)
- Handle any key input (Ctrl+C, /quit work immediately)
- Check `rx.try_recv()`:
  - `Chunk(text)`: `app.update_last_message(text)`, `render()`
  - `Final(text)`: `app.finalize_chat(text)`
  - `Error(err)`: `app.set_chat_error(err)`
  - `Done`: `app.cleanup_chat_thread()`
- `render()` always called at end of loop

**5. On Final event (app.finalize_chat)**
- Persist assistant message to SQLite
- Create `chat_message` graph entity
- Create `RESPONDED_WITH` edge (user_msg -> asst_msg)
- Parse content for symbol mentions, create `ASKED_ABOUT` edges
- Parse content for file mentions, create `MENTIONED_FILE` edges
- Update `chat_sessions` end_time and message_count
- Evaluate compaction triggers
- If triggered: run compaction (synchronous, blocks next input)

**6. On Error event**
- Persist error to SQLite as diagnostic
- Update `chat_error` state (already exists from Phase 8.5)
- Remove "Thinking..." indicator

**7. Shutdown during active chat**
- `/quit` or `Ctrl+C`: `app.quit()`, `shutdown_flag.store(true)`
- Main loop breaks immediately
- Chat thread: checks flag on next iteration, exits early
- HTTP request may abort (acceptable)
- In-memory chat lost (acceptable, user intent to exit)

#### Ownership Model

- **Main thread owns**: App, ExecutionDb, Terminal, all persistence
- **Chat thread owns**: db_root PathBuf, channel Sender only
- **Shared**: `Arc<AtomicBool>` shutdown flag (no other shared state)
- **Channel**: `mpsc::unbounded<ChatEvent>` (chat data is tiny, backpressure impossible)

#### Persistence Sequence

**User Message** (Before Thread Spawn):
1. BEGIN TRANSACTION
2. INSERT INTO chat_sessions (id, start_time, message_count=1)
3. INSERT INTO chat_messages (session_id, role='user', content, timestamp)
4. INSERT INTO executions (id='chat_USER', tool_name='chat_message', ...)
5. INSERT INTO execution_artifacts (execution_id, artifact_type='chat_message', ...)
6. COMMIT
7. Create chat_session graph entity
8. Create chat_message graph entity
9. Spawn thread

**Assistant Message** (After Final Event):
1. BEGIN TRANSACTION
2. INSERT INTO chat_messages (session_id, role='assistant', content, timestamp)
3. UPDATE chat_sessions SET end_time=?, message_count=message_count+1
4. INSERT INTO executions (id='chat_ASSISTANT', ...)
5. INSERT INTO execution_artifacts (artifact_type='chat_message', ...)
6. COMMIT
7. Create chat_message graph entity
8. Create `RESPONDED_WITH` edge (user_msg_id -> asst_msg_id)
9. Parse content for symbol mentions:
   - Simple regex: `\b[A-Z][a-zA-Z_0-9]*\b`
   - Query codegraph.db for matching symbols
   - Create `ASKED_ABOUT` edges
10. Parse content for file mentions:
    - Regex: `\b[a-z_][a-z0-9_]*\.(rs|toml|json)\b`
    - Create `MENTIONED_FILE` edges

#### Compaction Trigger Conditions

- Message count > 50 in current session
- OR estimated token count > 4000 (count chars / 4)
- Evaluated after each assistant Final event

#### Compaction Process

1. Identify oldest N messages to compact (keep 10 most recent)
2. Build prompt from messages to summarize
3. Call `adapter.generate()` with summary request (synchronous, blocking)
4. BEGIN TRANSACTION
5. INSERT INTO executions (id='chat_compaction', ...)
6. INSERT INTO execution_artifacts (artifact_type='chat_summary', ...)
7. UPDATE chat_sessions SET compacted=1
8. COMMIT
9. Create `chat_summary` graph entity
10. Create `COMPACTED_TO` edges (compacted_msg -> summary)
11. Update in-memory `chat_messages`: replace with summary marker

#### Context Injection After Compaction

When loading chat for LLM context:
1. Query SQLite for uncompacted messages
2. For each compacted section, load summary text
3. Build prompt: "Previous conversation: [summary]... Recent: [actual messages]"

#### Tool Invocation Path

**CRITICAL INVARIANT**: ToolInvocation events emitted during chat MUST NOT auto-transition state. They are queued suggestions until the user explicitly runs `/plan`. This avoids accidental state jumps if a model emits tool syntax mid-sentence.

If LLM emits tool call in chat response:
1. Chat thread detects tool invocation pattern in response
2. Sends `ToolInvocation(tool_name, arguments)` via tx
3. Main thread receives event:
   a. Stores suggestion in `app.pending_tool_suggestions`
   b. **DOES NOT transition state** (no `AppState::PlanningInProgress`)
   c. **DOES NOT call `handle_plan` automatically**
4. User sees "(Tool suggested: file_read — type /plan to approve)" in hint text
5. User explicitly runs `/plan` to review and approve suggestions
6. After plan completion:
   a. Transitions back to chat state
   b. Resumes chat session (can ask follow-up)
   c. Persists plan execution result separately

**Chat does NOT execute tools. Chat SUGGESTS tools. User MUST EXPLICITLY APPROVE via /plan.**

#### Shutdown Semantics

- `/quit` key: handled in main loop before any other processing
- `Ctrl+C`: handled in main loop, sets `should_quit=true`
- `shutdown_flag`: `Arc<AtomicBool>` checked by chat thread each loop
- Thread join: attempted with 100ms timeout in cleanup
- If join fails: thread detached, allowed to finish/abort
- No waiting for chat completion on exit

### Option B: Pump/Step Model (ALTERNATIVE, NOT RECOMMENDED)

Same data flow as Option A, EXCEPT:
- No background thread
- Chat state machine with "step" function called each main loop iteration
- ureq replaced with non-blocking HTTP (would require different library)

**Rejected because**:
- ureq doesn't support non-blocking I/O
- Would require changing HTTP library (major scope increase)
- Step-based polling is more complex than threaded callback
- No actual benefit over Option A

---

## ACT — Step-by-Step Implementation Plan

### Step 1: Chat Events Module

**File**: `src/ui/chat_events.rs` (NEW, ~80 LOC)

**Purpose**: Define event types for chat thread to main thread communication

**Content**:
- `pub enum ChatEvent { Chunk(String), Final(String), Error(ChatError), ToolInvocation { tool: String, args: serde_json::Value }, Done }`
- No dependencies on session/plan modules
- `ChatError` imported from `llm::chat`

---

### Step 2: Chat Thread Module

**File**: `src/ui/chat_thread.rs` (NEW, ~180 LOC)

**Purpose**: Spawn and manage background chat thread

**Content**:
- `pub fn spawn_chat_thread(prompt: String, db_root: PathBuf, tx: mpsc::Sender<ChatEvent>, shutdown: Arc<AtomicBool>) -> JoinHandle<()>`
- Thread function: calls `chat::chat_threaded()`
- Sends Chunk events for each streaming fragment
- Sends Final on completion
- Sends Error on adapter failure
- Checks shutdown flag each iteration
- No database access

---

### Step 3: Modify Chat Module for Threading

**File**: `src/llm/chat.rs` (MODIFY, add ~40 LOC)

**Changes**:
- Add `pub fn chat_threaded<F>(prompt, db_root, on_chunk, shutdown) -> Result<String, ChatError>`
- `on_chunk` callback receives `Arc<AtomicBool>` for shutdown checking
- Function calls `adapter.generate_streaming()` with shutdown-aware callback
- Returns full response text

---

### Step 4: Chat Persistence Module

**File**: `src/ui/chat_persist.rs` (NEW, ~280 LOC)

**Purpose**: Persist chat sessions to SQLite + SQLiteGraph

**Content**:
- `pub fn persist_chat_session(exec_db: &ExecutionDb, session_id: &str, messages: &[ChatMessage]) -> Result<()>`
- `pub fn persist_user_message(exec_db: &ExecutionDb, session_id: &str, content: &str) -> Result<()>`
- `pub fn persist_assistant_message(exec_db: &ExecutionDb, session_id: &str, content: &str) -> Result<()>`
- `fn create_chat_session_entity(exec_db: &ExecutionDb, session_id: &str) -> Result<i64>`
- `fn create_chat_message_entity(exec_db: &ExecutionDb, session_id: &str, role: &str, content: &str) -> Result<i64>`
- `fn create_chat_edge(exec_db: &ExecutionDb, from_id: i64, to_id: i64, edge_type: &str) -> Result<()>`
- `pub fn should_compact(messages: &[ChatMessage]) -> bool`
- `pub fn compact_session(exec_db: &ExecutionDb, session_id: &str, messages: &[ChatMessage]) -> Result<Vec<usize>>`

---

### Step 5: Chat Compaction Module

**File**: `src/ui/chat_compact.rs` (NEW, ~220 LOC)

**Purpose**: Generate and persist chat summaries

**Content**:
- `pub fn generate_summary(messages: &[ChatMessage], db_root: &PathBuf) -> Result<String>`
  - Calls `adapter.generate()` with summary prompt
- `pub fn persist_summary(exec_db: &ExecutionDb, session_id: &str, summary: &str, compacted_ids: &[usize]) -> Result<()>`
  - Creates `chat_summary` graph entity
  - Creates `COMPACTED_TO` edges from original messages to summary
- Returns list of compacted message IDs

---

### Step 6: Modify App State

**File**: `src/ui/state.rs` (MODIFY, add ~60 LOC)

**Changes**:
- Add: `chat_thread_handle: Option<JoinHandle<()>>`
- Add: `chat_shutdown_flag: Option<Arc<AtomicBool>>`
- Add: `chat_event_receiver: Option<mpsc::Receiver<ChatEvent>>`
- Add: `current_chat_session_id: Option<String>`
- Modify: `chat_messages` becomes `Arc<Mutex<Vec<ChatMessage>>>`
- Add: `pub fn start_chat_thread(&mut self, prompt: String, db_root: PathBuf)`
- Add: `pub fn process_chat_events(&mut self) -> Result<()>`
- Add: `pub fn persist_chat(&mut self, exec_db: &ExecutionDb) -> Result<()>`
- Add: `pub fn cleanup_chat_thread(&mut self)`
- Add: `pub fn finalize_chat(&mut self, response: String)`
- Modify: `add_user_message()` to also persist immediately

---

### Step 7: Modify Chat Handler

**File**: `src/ui/handlers.rs` (MODIFY, rewrite `handle_chat`)

**Changes**:
- `handle_chat()` now non-blocking:
  1. Calls `app.add_user_message()`
  2. Calls `app.persist_user_message()` via exec_db
  3. Calls `app.start_chat_thread()`
  4. Returns immediately
- Process chat events in main loop instead

---

### Step 8: Modify Main Loop

**File**: `src/main.rs` (MODIFY, add ~20 lines in main loop)

**Changes**:
- In main loop after `poll()`:
  - `if app.chat_thread_active(): app.process_chat_events()?`
- `render()` called every iteration regardless
- `/quit` handled before any other processing

---

### Step 9: Modify Execution Database Triggers

**File**: `src/execution_tools/db.rs` (MODIFY, update triggers)

**Changes**:
- Add 'chat_message' to `tool_name` whitelist (for session/message logging)
- Add 'chat_compaction' to `tool_name` whitelist
- Add 'chat_summary' to `artifact_type` whitelist

---

### Step 10: Modify Execution Recording

**File**: `src/execution_tools/record.rs` (MODIFY, if needed)

**Changes**:
- May need helper function for chat-specific execution recording pattern
- Or reuse existing `record_execution_with_artifacts()`

---

## Data Models

### SQLite Schema (execution_log.db)

**New Table**: `chat_sessions`
```sql
CREATE TABLE chat_sessions (
    id TEXT PRIMARY KEY NOT NULL,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    message_count INTEGER NOT NULL DEFAULT 1,
    compacted INTEGER NOT NULL DEFAULT 0
);
```

**New Table**: `chat_messages`
```sql
CREATE TABLE chat_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES chat_sessions(id) ON DELETE CASCADE
);
```

**Modified Trigger**: `validate_tool_name`
Add 'chat_message' to allowed `tool_names`

**Modified Trigger**: `validate_artifact_type`
Add 'chat_summary', 'chat_compaction' to allowed `artifact_types`

### SQLiteGraph Schema (codegraph.db)

**New Entity**: `kind='chat_session'`
- data: `{session_id, start_time, end_time, message_count, compacted}`

**New Entity**: `kind='chat_message'`
- data: `{session_id, role, content, timestamp}`

**New Entity**: `kind='chat_summary'`
- data: `{summary_text, original_message_count, timestamp}`

**New Edge Types**:
- `CHAT_CONTEXT`: chat_session -> file
- `ASKED_ABOUT`: chat_message -> symbol (user asked about symbol X)
- `RESPONDED_WITH`: chat_message -> chat_message (conversation flow)
- `MENTIONED_FILE`: chat_message -> file
- `COMPACTED_TO`: chat_message -> chat_summary
- `SUMMARY_OF`: chat_summary -> chat_session

---

## Event Types (Between Chat Thread and Main)

Defined in `src/ui/chat_events.rs`:

```rust
pub enum ChatEvent {
    Chunk(String),                    // Streaming text fragment
    Final(String),                    // Complete response text
    Error(ChatError),                 // LLM adapter error
    ToolInvocation {                  // LLM wants to call tool
        tool: String,
        arguments: serde_json::Value,
    },
    Done,                             // Thread exiting
}
```

---

## Persistence Sequence (Detailed)

### User Message (Before Thread Spawn)

1. Generate UUID for session_id
2. Get current timestamp (milliseconds since UNIX epoch)
3. `ExecutionDb::record_execution_with_artifacts()`:
   - id: `"chat_user_{session_id}"`
   - tool_name: `"chat_message"`
   - artifact_type: `"chat_message"`
4. Create `chat_session` graph entity
5. Create `chat_message` graph entity (role="user")
6. Store session_id in `app.current_chat_session_id`
7. Spawn thread

### Assistant Message (After Final Event)

1. Get current timestamp
2. `ExecutionDb::record_execution_with_artifacts()`:
   - id: `"chat_assistant_{session_id}_{timestamp}"`
   - tool_name: `"chat_message"`
   - artifact_type: `"chat_message"`
3. Update `chat_sessions`: `SET end_time=?, message_count=?`
4. Create `chat_message` graph entity (role="assistant")
5. Create `RESPONDED_WITH` edge: user_msg_id -> asst_msg_id
6. Parse content for symbol mentions:
   - Simple regex: `\b[A-Z][a-zA-Z_0-9]*\b`
   - Query codegraph.db for matching symbols
   - Create `ASKED_ABOUT` edges
7. Parse content for file mentions:
   - Regex: `\b[a-z_][a-z0-9_]*\.(rs|toml|json)\b`
   - Create `MENTIONED_FILE` edges

### Compaction (After Assistant, If Triggered)

1. Identify compactable range: `messages[0..messages.len()-10]`
2. Build summary prompt: "Summarize this conversation: {messages}"
3. Call `adapter.generate()` (blocking, synchronous)
4. `ExecutionDb::record_execution_with_artifacts()`:
   - id: `"chat_compaction_{session_id}"`
   - tool_name: `"chat_compaction"`
   - artifact_type: `"chat_summary"`
5. Create `chat_summary` graph entity
6. For each compacted message:
   - Create `COMPACTED_TO` edge: message -> summary
7. Update `chat_sessions`: `SET compacted=1`
8. Update in-memory `chat_messages`: replace with `[Previous conversation summarized]`

---

## Shutdown Behavior During Active Chat

### Case 1: User Types /quit

1. `parse_command()` returns `Command::Quit`
2. `execute_command()` calls `app.quit()` — sets `should_quit=true`
3. `chat_shutdown_flag.store(true, Ordering::SeqCst)`
4. Main loop breaks immediately
5. Cleanup skipped (user intent to exit)
6. Chat thread: checks flag on next `read_line()`, exits early
7. HTTP may abort (acceptable)

### Case 2: User Presses Ctrl+C

1. crossterm `Event::Key` with `Ctrl+C` modifier
2. Same as `/quit`: immediate break
3. `chat_shutdown_flag` set
4. Cleanup skipped

### Case 3: Natural Completion

1. Chat thread sends `Done` event
2. Main thread receives `Done`
3. `app.cleanup_chat_thread()`:
   - `join(thread_handle, Duration::from_millis(100))`
   - If join fails: `detach` (allow thread to finish)
   - Clear handle, receiver, shutdown_flag
4. Ready for next chat

---

## Explicit Non-Goals

- NO async/await runtime (`std::thread` only, `std::mpsc` only)
- NO tokio or other async runtime
- NO multiple concurrent chat sessions
- NO partial chunk persistence (chunks aggregated in memory)
- NO real-time LLM monitoring of filesystem
- NO automatic chat-to-plan conversion
- NO chat history auto-loading on restart (must be explicit)
- NO vector embeddings for chat (not specified)
- NO RAG for chat history (not specified)

---

## VERIFY — Validation Criteria

### Existing Tests That Must Pass (305 tests)

- `tests/chat_lane_isolation_tests.rs` (20 tests) — chat isolated from plan
- `tests/ui_chat_error_isolation_tests.rs` (10 tests) — error routing
- `tests/ui_nlp_mode_tests.rs` (11 tests) — NLP routing
- `tests/execution_tools_tests.rs` (13 tests) — execution DB
- `tests/evidence_queries_tests.rs` (21 tests) — evidence queries
- All other existing tests (~230 tests)

### New Tests Required

#### 1. `tests/ui_chat_threading_tests.rs` (12 tests)

- `test_thinking_indicator_renders_immediately`: verify render <200ms after Enter
- `test_chunk_updates_render_incrementally`: verify 5+ renders during response
- `test_error_event_routes_to_diagnostics`: verify ChatError set
- `test_shutdown_during_chat_aborts_thread`: verify thread exits on flag
- `test_quit_immediate_during_chat`: verify /quit works mid-stream
- `test_ctrl_c_immediate_during_chat`: verify Ctrl+C works mid-stream
- `test_chat_creates_no_plan_objects`: verify no Plan created
- `test_chat_creates_execution_artifacts`: verify 'chat_message' artifacts
- `test_only_one_chat_thread_at_a_time`: verify second chat rejected
- `test_cleanup_after_normal_completion`: verify handle cleaned up
- `test_chat_thread_respects_shutdown_flag`: verify early exit

#### 2. `tests/ui_chat_persistence_tests.rs` (12 tests)

- `test_chat_sessions_table_created`: verify schema exists
- `test_chat_messages_table_created`: verify schema exists
- `test_user_message_persisted_immediately`: verify row exists before thread
- `test_assistant_message_persisted_on_complete`: verify row exists after
- `test_session_end_time_set_on_completion`: verify end_time updated
- `test_message_count_increments`: verify count matches actual
- `test_persisted_messages_can_be_queried`: verify SELECT works
- `test_session_id_format_is_uuid`: verify valid UUID
- `test_chat_persists_across_restart`: verify data in DB after exit
- `test_error_recorded_in_diagnostics`: verify errors logged

#### 3. `tests/ui_chat_graph_tests.rs` (10 tests)

- `test_chat_session_entity_created`: verify kind='chat_session' in graph_entities
- `test_chat_message_entity_created`: verify kind='chat_message' in graph_entities
- `test_responded_with_edge_created`: verify edge between user/assistant
- `test_asked_about_edge_for_symbols`: verify symbol mentions create edges
- `test_mentioned_file_edge_for_files`: verify file mentions create edges
- `test_edge_type_validation_enforces_allowed_types`: verify only valid edges
- `test_chat_graph_survives_restart`: verify graph data persists

#### 4. `tests/ui_chat_compaction_tests.rs` (10 tests)

- `test_compaction_triggers_at_message_count`: verify triggers at 51 messages
- `test_compaction_triggers_at_token_limit`: verify triggers at ~4000 tokens
- `test_compaction_creates_summary_artifact`: verify 'chat_summary' artifact
- `test_compaction_creates_summary_entity`: verify chat_summary entity
- `test_compaction_creates_compacted_to_edges`: verify edges to summary
- `test_compacted_messages_remain_in_db`: verify originals not deleted
- `test_compaction_updates_compacted_flag`: verify sessions.compacted=1
- `test_compaction_includes_original_ids`: verify artifact lists IDs
- `test_compaction_summary_is_coherent`: verify summary captures key info

#### 5. Integration Verification (Manual)

- Run TUI, type "hello", verify "Thinking..." appears within 100ms
- Verify text streams character-by-character (not all at once)
- After response, query: `SELECT * FROM chat_messages`
- Verify graph: `SELECT * FROM graph_entities WHERE kind='chat_session'`
- Type 51 messages, verify compaction triggers
- Restart TUI, verify chat history still queryable

### How We Prove Chat is Responsive

- Test: time from Enter key to first render < 200ms
- Test: at least 5 `render()` calls during 100-token response
- Test: UI accepts keystrokes during chat (Ctrl+C works)
- Manual: observe flowing text, not frozen UI

### How We Prove Chat is Fully Persisted

- Test: SQLite row count equals in-memory count
- Test: `SELECT` returns all messages after completion
- Test: `chat_messages` table has correct role values
- Test: `chat_sessions` has correct start/end times
- Test: Data survives process restart

### How We Prove Compaction Does Not Lose Information

- Test: Original message rows still in `chat_messages` table
- Test: `COMPACTED_TO` edges link originals to summary
- Test: Summary text contains key information from originals
- Test: Compacted flag set on session
- Test: Query for original messages still returns rows

### How We Prove Execution Lane Unaffected

- Test: All `execution_tools` tests pass (unchanged)
- Test: All `evidence_queries` tests pass (unchanged)
- Test: `/plan` workflow still requires approval
- Test: Tool execution still goes through `execution_engine`
- Test: Plan execution logging unchanged
- Test: No test in `execution_tools` needs modification

---

## REPORT — Summary

### Architecture Decision

**SELECTED**: Option A — Fire-and-Forget Chat Thread with Deferred Persistence

**RATIONALE**:
- Chat thread handles only HTTP I/O (isolated responsibility)
- Main thread handles all persistence (thread-safe, single writer)
- Deferred persistence avoids blocking streaming (better UX)
- Session stored immediately, response stored after completion (simple transaction model)
- Compaction at session boundaries (no mid-stream state complexity)
- Graph edges created after SQLite commit (existing best-effort pattern)
- Execution lane completely untouched (zero risk to determinism)

**REJECTED**: Option B — Pump/Step Model
- ureq doesn't support non-blocking I/O
- Would require HTTP library change (outside scope)
- State machine more complex than threading
- No UX benefit over Option A

### Risks and Mitigations

1. **RISK**: Chat thread complexity increases bug surface
   **MITIGATION**: Thread is fire-and-forget, minimal shared state (just AtomicBool)

2. **RISK**: Persistence blocks UI after chat
   **MITIGATION**: SQLite writes are fast (<10ms). Text already rendered, user sees no lag.

3. **RISK**: Compaction loses critical context
   **MITIGATION**: Original messages never deleted from SQLite. Compaction is UI optimization only.

4. **RISK**: Graph edge creation is slow for many messages
   **MITIGATION**: Edges created after SQLite commit. Failure is non-blocking (best-effort).

5. **RISK**: Tool invocation during chat creates state confusion
   **MITIGATION**: Explicit state transition. Chat pauses, plan executes, chat resumes.

6. **RISK**: Session ID collision
   **MITIGATION**: UUID v4 generation. No collisions in practice.

7. **RISK**: Chat thread doesn't respect shutdown
   **MITIGATION**: AtomicBool checked in tight loop. HTTP read may abort (acceptable).

8. **RISK**: Channel overflow
   **MITIGATION**: Unbounded mpsc channel. Chat data is tiny (KB/s). Overflow impossible.

9. **RISK**: Arc<Mutex<>> deadlock
   **MITIGATION**: Only chat_messages is shared. Lock held briefly for append.

10. **RISK**: Compaction summary is poor quality
    **MITIGATION**: Summary uses same LLM as chat. Original messages still queryable.

### GO / NO-GO Criteria

**GO IF ALL**:
- 305 existing tests pass (100% regression coverage)
- 44+ new tests pass (12 threading + 12 persistence + 10 graph + 10 compaction)
- "Thinking..." renders within 200ms of Enter key
- Text streams incrementally (visible character-by-character)
- `/quit` and `Ctrl+C` exit immediately during chat
- SQLite tables created: `chat_sessions`, `chat_messages`
- Graph entities created: `chat_session`, `chat_message`
- Compaction triggers at defined thresholds
- Original messages preserved after compaction
- Zero clippy warnings
- All files <= 300 LOC

**NO-GO IF ANY**:
- Existing test fails (regression)
- New test fails (feature not working)
- UI remains frozen during chat (lag persists)
- Chat not persisted to SQLite
- Graph edges not created
- Compaction loses data
- Chat lane isolation violated (chat creates Plan objects)
- Execution lane behavior changed
- File exceeds 300 LOC

---

## Non-Negotiable Constraints

Reaffirmed from `docs/CONTRACT.md` and project architecture:

- **NO ASYNC** — Synchronous execution only (`std::thread`, not tokio)
- **NO INTELLIGENCE** — We build tools, not AI
- **NO GUESSING** — Every operation grounded in facts
- Tools execute ONLY via CORE
- LLM may invoke tools, but workflow+approval always applies
- `/quit`, `Ctrl+C` must exit immediately
- <= 300 LOC per production file

---

*This is a PLANNING-ONLY proposal. Implementation requires explicit approval.*
