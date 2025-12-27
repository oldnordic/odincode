# Phase 9.3 Notes — Observability + UX Hardening

## STEP 0 Findings

### Current Scroll Logic (src/ui/view.rs:352-359)
```rust
// Scroll to bottom (show most recent)
let visible_lines = (area.height as usize).saturating_sub(2);
let scroll_start = if lines.len() > visible_lines {
    lines.len() - visible_lines
} else {
    0
};
```
- **Always scrolls to bottom** (no user scroll state tracking)
- No concept of "autoscroll enabled/disabled"
- User cannot scroll up to see history

### ToolStatus Display (src/ui/state.rs:103-128)
```rust
ToolStatus {
    tool: String,
    step: usize,
    start_timestamp: u64,
}
```
- Already shows elapsed time: `Running {tool} (step {step})... {elapsed}s`
- No token counter (would show "tokens: n/a")

### execution_log.db Schema
**executions table:**
- id, tool_name, arguments_json, timestamp, success, exit_code, duration_ms, error_message

**execution_artifacts table:**
- id, execution_id, artifact_type, content_json
- Includes: `approval_granted`, `approval_denied` (Phase 9.2)

## Implementation Plan

### A) Autoscroll State
- Add `chat_scroll_offset: usize` to App (default 0 = bottom)
- Add `autoscroll_enabled: bool` to App (default true)
- Modify `render_chat_transcript` to use scroll_offset
- Add key handlers for scroll up/down/PageUp/PageDown/Home/End

### B) Token/Time Counters
- Time: Already working in ToolStatus display
- Tokens: Add "tokens: n/a" to ToolStatus (adapters don't expose usage)

### C) Trace Viewer
- New module: `src/ui/trace.rs` (≤300 LOC)
- Query: `query_last_loop_trace(conn, limit) -> Vec<TraceRow>`
- TraceRow includes: tool_name, scope, timestamp, success, affected_path
- Toggle with 'L' key
- Render as overlay panel
