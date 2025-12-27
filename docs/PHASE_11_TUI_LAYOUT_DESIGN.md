# Phase 11: TUI Layout Design (Ratatui)

**Date**: 2025-12-26
**Status**: Design Phase — No Code Written
**Objective**: Design terminal UI layout layer for OdinCode using ratatui

---

## ⚠️ FROZEN INVARIANTS (Non-Negotiable)

**Before any code is written, these invariants are LOCKED:**

### UI Layer Constraints

1. **UI never mutates execution state**
   - TUI is a renderer only — read-only projection of execution memory
   - No write operations to database from UI code
   - No state mutations outside of event handling layer

2. **UI never stores or interprets reasoning**
   - No caching of LLM responses in UI
   - No interpretation or transformation of reasoning_content
   - Messages are views over `execution_log.db`, not chat truth

3. **UI never retries tools**
   - No retry logic in UI layer
   - No "helpful" error recovery
   - Tool execution failures are displayed, not fixed by UI

4. **UI never summarizes without execution IDs**
   - Every displayed message must trace to an execution_id
   - No "summary" that isn't backed by execution memory
   - Timeline-first: UI reflects what actually happened

5. **UI never bypasses grounding gates**
   - No direct tool invocation without execution record
   - No "quick actions" that skip logging
   - All operations must be auditable

### Widget-Specific Invariants

#### Messages Widget (Section 5)

**CRITICAL**: Messages are a **projection of execution memory**, never chat history.

```rust
//! Messages widget: scrollable conversation history
//!
//! # INVARIANT: Read-Only Projection
//!
//! Messages are rendered from execution_log.db only.
//! - No caching of model text
//! - No optimization by storing "current view"
//! - Always read from ground truth (executions table)
//!
//! # What Gets Displayed
//!
//! Each message renders an execution_artifact row:
//! - user_input → chat_user_message artifact
//! - assistant_response → adapter_response artifact
//! - tool_call → adapter_call artifact
//!
//! # What Does NOT Get Displayed
//!
//! - Internal reasoning (never shown, never stored in UI state)
//! - Intermediate prompt expansions (execution artifacts only)
```

#### Status Bar (Section 6)

**CRITICAL**: Must reference execution IDs, not just prose.

```rust
//! Status widget: operation indicator above input
//!
//! # INVARIANT: Execution ID Display
//!
//! Status format MUST include execution_id:
//!
//!     Working: #142 splice_patch | Mode: Mutation
//!            ^^^^ execution ID
//!
//! Why:
//! - Execution ID is ground truth
//! - Prevents UI lying accidentally
//! - Enables timeline lookup
//!
//! # Forbidden Formats
//!
//! ❌ "Working: splice_patch" (no ID)
//! ❌ "Running file read..." (vague)
//!
//! # Required Format
//!
//! ✅ "Working: #142 splice_patch | Mode: Mutation"
//! ✅ "Idle | Mode: Query"
```

**Layout**:
```
Working: #142 splice_patch         Mode: Mutation
^-- execution + tool -----------^ mode --^
```

#### Help Overlay (Section 8)

**CRITICAL**: Must exclude workflow explanation.

```rust
//! Help widget: overlay with keybind reference
//!
//! # INVARIANT: View Controls Only
//!
//! Help CAN explain:
//! - Keys and navigation
//! - View controls (scroll, search, filter)
//! - Terminal interface behavior
//!
//! Help MUST NOT explain:
//! - What tools do (see TOOL_MANUALS.md)
//! - How planning works (see CORE_LOOP.md)
//! - How correctness is enforced (see CONTRACT.md)
//!
//! # Rationale
//!
//! Workflow documentation lives in docs/, not buried in UI.
//! UI help is for "what keys do I press", not "what does this tool do".
//!
//! # Non-Goals (Explicitly Excluded)
//!
//! - Tool descriptions (use `odincode tool --help`)
//! - Agent behavior (see architecture docs)
//! - Execution semantics (see Phase documentation)
```

**Layout** (revised):
```
┌─────────────────────────────────────────────┐
│                    HELP                      │
├─────────────────────────────────────────────┤
│ Navigation                                     │
│   Ctrl+P/N    Previous/Next message           │
│   PageUp/Down Scroll up/down full page        │
│                                                 │
│ Input Editing                                   │
│   Ctrl+C      Clear input                      │
│   Ctrl+A/E    Move to start/end of line        │
│                                                 │
│ View Controls                                   │
│   Ctrl+L      Toggle line numbers              │
│   F5           Refresh view                     │
│                                                 │
│ For tool help: odincode tool --help <name>     │
│ For docs: https://github.com/.../docs         │
│                                                 │
│ Press any key to close                         │
└─────────────────────────────────────────────┘
```

---

## Scope

This document covers **layout design only** — visual structure, widgets, and file organization.

**Out of scope** (separate phases):
- Event handling (keyboard, mouse)
- State management
- Backend integration
- Animations

---

## Design Principles

1. **Max 300 LOC per file** — Strict modular design
2. **Separation of concerns** — Layout only, no business logic
3. **TDD first** — Tests before implementation
4. **Integration tests** — Real terminal rendering tests
5. **No drift** — All decisions documented here
6. **Timeline-first** — UI reflects execution memory, never replaces it
7. **Zero authority** — UI cannot decide, retry, or summarize on its own

---

## Screen Layout

```
┌─────────────────────────────────────────────────────────────┐
│ OdinCode v0.x.x                    [Build] [Plan] [Explore] │ ← Header (1 line)
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  User: Help me refactor src/lib.rs                         │
│  Agent: I'll analyze the file structure...                 │
│  [Tool: file_read] src/lib.rs                              │
│     1| pub mod config;                                     │
│     2| pub mod tools;                                      │
│  ...                                                       │
│                                                             │
│  User: What needs refactoring?                             │
│  Agent: Based on the analysis...                           │
│                                                             │ │ ← Messages (scrollable)
│                                                             │
│                                                             │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│ Working: #142 splice_patch         Mode: Mutation           │ │ ← Status (1 line)
├─────────────────────────────────────────────────────────────┤
│ >                                            [Enter] Send │ │ ← Input (2 lines)
│   [Ctrl+X=Menu] [Ctrl+C=Clear] [?=Help]                    │
└─────────────────────────────────────────────────────────────┘
```

---

## File Structure

### New Files to Create

```
src/
└── tui/
    ├── mod.rs                    # Facade, ~50 LOC
    ├── layout/
    │   ├── mod.rs                # Layout exports, ~50 LOC
    │   ├── types.rs              # Shared types, ~100 LOC
    │   ├── header.rs             # Header widget, ~150 LOC
    │   ├── messages.rs           # Message list widget, ~250 LOC
    │   ├── status.rs             # Status bar widget, ~150 LOC
    │   ├── input.rs              # Input field widget, ~200 LOC
    │   └── help.rs               # Help overlay, ~150 LOC
    └── tests/
        └── layout_tests.rs       # Integration tests, ~300 LOC

tests/
└── tui_layout_tests.rs           # Render verification, ~300 LOC

docs/
    └── PHASE_11_TUI_LAYOUT_DESIGN.md   # This document
```

### Files to Modify

```
src/
├── lib.rs                        # Add tui module export
└── main.rs                       # Add --tui flag (Phase 11.2)

Cargo.toml                        # Add ratatui dependency (Phase 11.2)
```

---

## Module Breakdown

### 1. `src/tui/mod.rs` (~50 LOC)

**Purpose**: Facade for TUI layout module

**Responsibilities**:
- Re-export layout widgets
- Provide `Layout` struct for full screen composition

**API**:
```rust
//! Terminal UI layout layer using ratatui
//!
//! # Design Principles
//! - Max 300 LOC per module
//! - Layout only, no business logic
//! - Integration tested with real terminal

pub use layout::{Layout, LayoutConfig};

/// Layout configuration
pub struct LayoutConfig {
    pub show_help: bool,
    pub show_line_numbers: bool,
    pub max_messages: usize,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            show_help: false,
            show_line_numbers: true,
            max_messages: 100,
        }
    }
}
```

---

### 2. `src/tui/layout/mod.rs` (~50 LOC)

**Purpose**: Export all layout widgets

**Responsibilities**:
- Re-export header, messages, status, input, help
- Module organization only

**API**:
```rust
//! Layout widgets for OdinCode TUI
//!
//! Organized by screen region:
//! - header: Top bar with title and agent selector
//! - messages: Scrollable conversation history
//! - status: Current operation indicator
//! - input: User input field
//! - help: Overlay help screen

pub use types::{Rect, Size};
pub use header::{Header, HeaderProps};
pub use messages::{Messages, MessagesProps, Message};
pub use status::{Status, StatusProps};
pub use input::{Input, InputProps};
pub use help::{Help, HelpProps};

pub use layout::Layout;
```

---

### 3. `src/tui/layout/types.rs` (~100 LOC)

**Purpose**: Shared types across layout modules

**Responsibilities**:
- Common structs used by multiple widgets
- No rendering logic

**API**:
```rust
//! Shared types for layout widgets

use ratatui::layout::Rect as RatatuiRect;

/// Screen region alias
pub type Rect = RatatuiRect;

/// Terminal size
#[derive(Debug, Clone, Copy)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

/// Message role
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
    Error,
}

/// A single message in the conversation
#[derive(Debug, Clone)]
pub struct Message {
    pub execution_id: Option<u64>,  // Ground truth: traces to executions table
    pub role: MessageRole,
    pub content: String,
    pub timestamp: Option<u64>,
    pub tool_name: Option<String>,
    pub tool_status: Option<ToolStatus>,
}

/// Tool execution status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Success,
    Failed,
}

/// Agent mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentMode {
    Build,
    Plan,
    Explore,
}

/// Current operation state
#[derive(Debug, Clone)]
pub struct OperationState {
    pub execution_id: Option<u64>,  // Ground truth requirement
    pub current_tool: Option<String>,
    pub mode: AgentMode,
    pub working: bool,
}
```

---

### 4. `src/tui/layout/header.rs` (~150 LOC)

**Purpose**: Top header bar widget

**Responsibilities**:
- Render title and version
- Render agent selector (Build/Plan/Explore)
- Handle layout of header elements

**API**:
```rust
//! Header widget: top bar with title and agent selector

use ratatui::{Frame, layout::Rect};
use crate::tui::layout::types::AgentMode;

pub struct Header {
    props: HeaderProps,
}

pub struct HeaderProps {
    pub title: String,
    pub version: String,
    pub current_agent: AgentMode,
    pub available_agents: Vec<AgentMode>,
}

impl Header {
    pub fn new(props: HeaderProps) -> Self {
        Self { props }
    }

    /// Render header in the given area
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Implementation:
        // - Left: "OdinCode v0.x.x"
        // - Right: "[Build] [Plan] [Explore]" with current highlighted
        // - Border line below
    }

    /// Calculate required height
    pub fn height() -> u16 {
        1
    }
}
```

**Layout**:
```
OdinCode v0.1.0                              [Build] [Plan] [Explore]
^-- left aligned ---------------------------^ right aligned --^
```

---

### 5. `src/tui/layout/messages.rs` (~250 LOC)

**Purpose**: Scrollable message history widget

**Responsibilities**:
- Render conversation messages
- Handle scrolling (viewport management)
- Format tool output blocks
- Line wrapping for long content

**API**:
```rust
//! Messages widget: scrollable conversation history

use ratatui::{Frame, layout::Rect};
use crate::tui::layout::types::{Message, MessageRole};

pub struct Messages {
    props: MessagesProps,
    state: MessagesState,
}

pub struct MessagesProps {
    pub messages: Vec<Message>,
    pub scroll_offset: usize,
}

pub struct MessagesState {
    viewport_start: usize,
    line_count: usize,
}

impl Messages {
    pub fn new(props: MessagesProps) -> Self {
        let state = MessagesState {
            viewport_start: props.scroll_offset,
            line_count: 0,
        };
        Self { props, state }
    }

    /// Render messages in the given area
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        // Implementation:
        // - Calculate visible lines based on area height
        // - Render messages from viewport_start
        // - Handle wrapping for long lines
        // - Style by role (User=green, Assistant=blue, Tool=yellow, Error=red)
    }

    /// Scroll up by n lines
    pub fn scroll_up(&mut self, n: usize) {
        self.state.viewport_start = self.state.viewport_start.saturating_sub(n);
    }

    /// Scroll down by n lines
    pub fn scroll_down(&mut self, n: usize) {
        let max_start = self.state.line_count.saturating_sub(self.state.viewport_lines());
        self.state.viewport_start = (self.state.viewport_start + n).min(max_start);
    }

    /// Scroll to bottom
    pub fn scroll_to_bottom(&mut self) {
        self.state.viewport_start = self.state.line_count.saturating_sub(self.state.viewport_lines());
    }

    fn viewport_lines(&self) -> usize {
        // Return current viewport height in lines
        0  // placeholder
    }
}
```

**Message Format**:
```
User: Help me refactor src/lib.rs

Assistant: I'll analyze the file structure...

[Tool: file_read] src/lib.rs
   1| pub mod config;
   2| pub mod tools;
   3| ...
[Tool: file_read] Completed in 23ms

User: What needs refactoring?
```

---

### 6. `src/tui/layout/status.rs` (~150 LOC)

**Purpose**: Status bar widget (above input)

**Responsibilities**:
- Display current operation
- Show active agent mode
- Working indicator

**API**:
```rust
//! Status widget: operation indicator above input

use ratatui::{Frame, layout::Rect};
use crate::tui::layout::types::{OperationState, AgentMode};

pub struct Status {
    props: StatusProps,
}

pub struct StatusProps {
    pub operation: OperationState,
}

impl Status {
    pub fn new(props: StatusProps) -> Self {
        Self { props }
    }

    /// Render status bar in the given area
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Implementation:
        // - Left: "Working: tool_name" or "Idle"
        // - Right: "Mode: Mutation" or "Mode: Query"
        // - Border line below
    }

    /// Calculate required height
    pub fn height() -> u16 {
        1
    }
}
```

**Layout**:
```
Working: #142 splice_patch         Mode: Mutation
^-- left (operation) -----------------^ right (mode) --^
```

---

### 7. `src/tui/layout/input.rs` (~200 LOC)

**Purpose**: Input field widget

**Responsibilities**:
- Render current input text
- Show cursor position
- Display keybind hints

**API**:
```rust
//! Input widget: user input field with keybind hints

use ratatui::{Frame, layout::Rect};

pub struct Input {
    props: InputProps,
}

pub struct InputProps {
    pub prompt: String,
    pub text: String,
    pub cursor_position: usize,
    pub keybind_hints: Vec<KeybindHint>,
}

pub struct KeybindHint {
    pub key: String,
    pub action: String,
}

impl Input {
    pub fn new(props: InputProps) -> Self {
        Self { props }
    }

    /// Render input field in the given area (2 lines)
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Line 1: "> input text here [Enter] Send"
        // Line 2: "[Ctrl+X=Menu] [Ctrl+C=Clear] [?=Help]"
    }

    /// Calculate required height
    pub fn height() -> u16 {
        2
    }
}
```

**Layout**:
```
> Read src/lib.rs and find issues              [Enter] Send
  [Ctrl+X=Menu] [Ctrl+C=Clear] [Ctrl+W=Write] [?=Help]
^-- input line ----------------------------^ hints line --^
```

---

### 8. `src/tui/layout/help.rs` (~150 LOC)

**Purpose**: Help overlay widget

**Responsibilities**:
- Render keybind reference
- Show as centered overlay

**API**:
```rust
//! Help widget: overlay with keybind reference

use ratatui::{Frame, layout::Rect};

pub struct Help {
    props: HelpProps,
}

pub struct HelpProps {
    pub sections: Vec<HelpSection>,
}

pub struct HelpSection {
    pub title: String,
    pub items: Vec<(String, String)>,  // (key, action)
}

impl Help {
    pub fn new(props: HelpProps) -> Self {
        Self { props }
    }

    /// Render help as centered overlay
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        // Implementation:
        // - Draw bordered box in center of screen
        // - Sections: Navigation, Editing, Tools, Other
        // - Dim background behind overlay
    }

    /// Calculate overlay size (80% width, 80% height)
    pub fn overlay_size(screen: Rect) -> Rect {
        Rect {
            x: screen.width * 10 / 100,
            y: screen.height * 10 / 100,
            width: screen.width * 80 / 100,
            height: screen.height * 80 / 100,
        }
    }
}
```

**Layout**:
```
┌─────────────────────────────────────────────┐
│                    HELP                      │
├─────────────────────────────────────────────┤
│ Navigation                                     │
│   Ctrl+P/N    Previous/Next message           │
│   Ctrl+U/D    Scroll up/down half page        │
│   PageUp/Down Scroll up/down full page        │
│                                                 │
│ Editing                                         │
│   Ctrl+C      Clear input                      │
│   Ctrl+A/E    Move to start/end of line        │
│                                                 │
│ Tools                                           │
│   Ctrl+X      Open tool menu                   │
│   Ctrl+W      Write current file               │
│                                                 │
│ Press any key to close                         │
└─────────────────────────────────────────────┘
```

---

### 9. `src/tui/layout/layout.rs` (~200 LOC)

**Purpose**: Compose full screen layout

**Responsibilities**:
- Calculate regions for each widget
- Delegate rendering to sub-widgets
- Handle layout constraints

**API**:
```rust
//! Full screen layout composition

use ratatui::Frame;
use super::{Header, Messages, Status, Input, Help};
use crate::tui::{LayoutConfig, types::Rect};

pub struct Layout {
    config: LayoutConfig,
    header: Header,
    messages: Messages,
    status: Status,
    input: Input,
    help: Help,
}

impl Layout {
    pub fn new(config: LayoutConfig) -> Self {
        // Initialize all widgets with default props
        Self {
            config,
            header: Header::new(/* ... */),
            messages: Messages::new(/* ... */),
            status: Status::new(/* ... */),
            input: Input::new(/* ... */),
            help: Help::new(/* ... */),
        }
    }

    /// Render full screen layout
    pub fn render(&mut self, frame: &mut Frame) {
        let size = frame.size();

        // Calculate regions
        let regions = self.calculate_regions(size);

        // Render each widget in its region
        self.header.render(frame, regions.header);
        self.messages.render(frame, regions.messages);
        self.status.render(frame, regions.status);
        self.input.render(frame, regions.input);

        // Render help overlay if enabled
        if self.config.show_help {
            let help_area = Help::overlay_size(size);
            self.help.render(frame, help_area);
        }
    }

    /// Calculate screen regions for each widget
    fn calculate_regions(&self, size: Rect) -> ScreenRegions {
        let header_height = Header::height();
        let status_height = Status::height();
        let input_height = Input::height();

        // Messages gets remaining space
        let messages_height = size.height
            .saturating_sub(header_height)
            .saturating_sub(status_height)
            .saturating_sub(input_height);

        ScreenRegions {
            header: Rect {
                x: 0,
                y: 0,
                width: size.width,
                height: header_height,
            },
            messages: Rect {
                x: 0,
                y: header_height,
                width: size.width,
                height: messages_height,
            },
            status: Rect {
                x: 0,
                y: header_height + messages_height,
                width: size.width,
                height: status_height,
            },
            input: Rect {
                x: 0,
                y: header_height + messages_height + status_height,
                width: size.width,
                height: input_height,
            },
        }
    }
}

struct ScreenRegions {
    header: Rect,
    messages: Rect,
    status: Rect,
    input: Rect,
}
```

---

### 10. `src/tui/tests/layout_tests.rs` (~300 LOC)

**Purpose**: Integration tests for layout rendering

**Responsibilities**:
- Test widget rendering
- Verify layout calculations
- Regression tests for visual output

**Test Strategy**:
```rust
//! Integration tests for TUI layout

use ratatui::{backend::TestBackend, Terminal};
use crate::tui::layout::Layout;

#[test]
fn test_header_renders() {
    // Create terminal with test backend
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();

    // Render header
    let mut layout = Layout::new(Default::default());
    terminal.draw(|f| layout.render(f)).unwrap();

    // Verify output contains expected text
    let buffer = terminal.backend().buffer();
    assert!(buffer.content.contains("OdinCode"));
}

#[test]
fn test_messages_scrolling() {
    // Test scroll_up and scroll_down behavior
}

#[test]
fn test_layout_regions_no_overlap() {
    // Verify calculated regions don't overlap
}

#[test]
fn test_input_cursor_position() {
    // Test cursor rendering
}

#[test]
fn test_help_overlay_centered() {
    // Verify help overlay positioning
}
```

---

## Dependencies

### Cargo.toml additions (Phase 11.2)

```toml
[dependencies]
# TUI
ratatui = "0.29"
crossterm = "0.28"

# For integration tests
[dev-dependencies]
ratatui = { version = "0.29", features = ["termwiz"] }
```

---

## Implementation Order

1. **Phase 11.1**: Layout types (`types.rs`)
2. **Phase 11.2**: Header widget (`header.rs`)
3. **Phase 11.3**: Messages widget (`messages.rs`)
4. **Phase 11.4**: Status widget (`status.rs`)
5. **Phase 11.5**: Input widget (`input.rs`)
6. **Phase 11.6**: Help overlay (`help.rs`)
7. **Phase 11.7**: Layout composition (`layout.rs`)
8. **Phase 11.8**: Integration tests (`layout_tests.rs`)

Each phase:
1. Write failing test
2. Implement widget
3. Verify test passes
4. Update this doc with any changes

---

## Open Questions

1. **Color scheme**: Default colors? Dark/light mode support?
2. **Line wrapping algorithm**: Word boundary or character boundary?
3. **Message truncation**: How to handle extremely long tool outputs?
4. **Terminal size limits**: Minimum viable dimensions? (assume 80x24)

---

## Design Decisions Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-12-26 | Max 300 LOC per file | OdinCode standard |
| 2025-12-26 | Layout only, no events | Separation of concerns |
| 2025-12-26 | ratatui over alternatives | Rust-native, active维护 |
| 2025-12-26 | **FROZEN: UI read-only** | Timeline-first doctrine |
| 2025-12-26 | **FROZEN: execution_id required** | Ground truth requirement |
| 2025-12-26 | **FROZEN: No workflow in help** | Docs belong in docs/ |
| 2025-12-26 | **FROZEN: Messages are DB projection** | No context caching |

---

## Implementation Status

| Phase | File | Status | Date |
|-------|------|--------|------|
| 11.0 | Design (this document) | ✅ COMPLETE | 2025-12-26 |
| 11.1 | types.rs | ⏸️ NOT STARTED | - |
| 11.2 | header.rs | ⏸️ NOT STARTED | - |
| 11.3 | messages.rs | ⏸️ NOT STARTED | - |
| 11.4 | status.rs | ⏸️ NOT STARTED | - |
| 11.5 | input.rs | ⏸️ NOT STARTED | - |
| 11.6 | help.rs | ⏸️ NOT STARTED | - |
| 11.7 | layout.rs | ⏸️ NOT STARTED | - |
| 11.8 | layout_tests.rs | ⏸️ NOT STARTED | - |

---

*Last Updated: 2025-12-26*
*Status: Design Frozen — Awaiting Implementation*
