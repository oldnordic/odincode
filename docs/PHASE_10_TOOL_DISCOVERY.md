# Phase 10: Progressive Tool Discovery

**Date**: 2025-12-26
**Status**: Phase 10.6 Complete ✅
**Objective**: Implement progressive tool discovery to reduce token usage and improve LLM decision-making

---

## Problem Statement

**Current State**:
- TOOL_WHITELIST contains 20 tools
- All tools loaded into LLM context upfront
- ~8,000 tokens just for tool descriptions (40% of typical context)
- LLM overwhelmed by options → poor tool selection
- No "when NOT to use" guidance

**Desired State**:
- Start with 3-5 core tools (~1,200 tokens)
- Discover specialized tools on-demand based on task context
- Include "when NOT to use" examples for each tool
- 85% token reduction for tool descriptions (Anthropic's measured savings)

---

## Design Principles

1. **Max 300 LOC per file** — OdinCode standard
2. **TDD first** — Tests before implementation
3. **No guessing** — Tool discovery based on explicit rules, not AI inference
4. **Examples teach behavior** — Show when AND when not to use (Claude Code leak insight)
5. **Timeline-first** — Discovery events logged to execution memory

---

## Core vs. Specialized Tools

### Core Tools (Always Loaded)

| Tool | Tokens | When to Use | When NOT to Use |
|------|--------|-------------|-----------------|
| `file_read` | ~150 | Read file contents | When file_search is enough |
| `file_search` | ~200 | Find files by pattern | For reading specific known files |
| `splice_patch` | ~300 | Single symbol replacement | For multi-file changes (use splice_plan) |
| `llm_explain` | ~200 | Understand code behavior | For simple lookups (use file_read) |
| `bash_exec` | ~250 | Run terminal commands | For file operations (use file_* tools) |

**Total**: ~1,100 tokens (vs. ~8,000 for all 20 tools)

### Specialized Tools (Discovered On-Demand)

| Tool | Discovery Trigger | Tokens |
|------|-------------------|--------|
| `splice_plan` | Multi-step refactoring mentioned | ~350 |
| `file_write` | File creation mentioned | ~200 |
| `file_create` | "create new file" in query | ~180 |
| `file_edit` | Line-based edits mentioned | ~220 |
| `file_glob` | Pattern matching needed | ~150 |
| `symbols_in_file` | "functions", "types" mentioned | ~250 |
| `references_to_symbol_name` | "where used", "callers" mentioned | ~200 |
| `references_from_file_to_symbol_name` | "imports", "dependencies" mentioned | ~220 |
| `lsp_check` | "errors", "diagnostics" mentioned | ~180 |
| `git_status` | "git", "changes" mentioned | ~150 |
| `git_diff` | "what changed" mentioned | ~150 |
| `git_log` | "history", "commits" mentioned | ~150 |
| `memory_query` | "previous", "before", "history" mentioned | ~200 |
| `execution_summary` | "what happened", "summary" mentioned | ~180 |
| `wc` | "count", "lines" mentioned | ~100 |
| `approval_granted` | Internal (auto-loaded) | 0 |
| `approval_denied` | Internal (auto-loaded) | 0 |
| `llm_plan` | "plan", "design" mentioned | ~250 |
| `llm_preflight` | Internal (auto-loaded) | 0 |

---

## File Structure

### New Files to Create

```
src/tools/
├── mod.rs                        # Tool registry facade, ~50 LOC
├── core.rs                       # Core tool definitions, ~200 LOC
├── specialized.rs                 # Specialized tool definitions, ~200 LOC
├── discovery.rs                   # Discovery engine, ~250 LOC
├── metadata.rs                    # ToolMetadata struct, ~150 LOC
└── tests/
    └── discovery_tests.rs         # Integration tests, ~300 LOC

docs/
    └── PHASE_10_TOOL_DISCOVERY.md  # This document
```

### Files to Modify

```
src/llm/
├── adapters/
│   └── normalize.rs               # Include only discovered tools in context
└── prompts.rs                     # NEW: System prompt with tool guidance

src/execution_tools/
└── db.rs                          # Log discovery events to execution_log.db
```

---

## Module Breakdown

### 1. `src/tools/metadata.rs` (~150 LOC)

**Purpose**: Tool metadata with examples

**Responsibilities**:
- Define ToolMetadata struct
- Store tool descriptions, examples, and "when not to use"

**API**:
```rust
//! Tool metadata with usage examples
//!
//! # Design Principle
//!
//! Examples teach behavior better than rules (Claude Code leak).
//! Each tool MUST have:
//! - 3-5 "when to use" examples
//! - 2-3 "when NOT to use" examples
//! - Clear reasoning for each

use serde::{Deserialize, Serialize};

/// Complete tool metadata for LLM consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub category: ToolCategory,
    pub description: String,

    /// Examples of proper usage
    pub examples: Vec<ToolExample>,

    /// Examples of when NOT to use this tool
    pub not_examples: Vec<ToolExample>,

    /// Approximate token cost when included in prompt
    pub token_cost: usize,

    /// Whether this tool requires approval (GATED)
    pub gated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolCategory {
    Core,           // Always loaded
    Specialized,    // Discovered on-demand
    Internal,       // System use only, never shown to LLM
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub scenario: String,
    pub command: String,
    pub reasoning: String,
}
```

**Example metadata**:
```rust
// file_read metadata
ToolMetadata {
    name: "file_read".to_string(),
    category: ToolCategory::Core,
    description: "Read the complete contents of a file".to_string(),
    examples: vec![
        ToolExample {
            scenario: "User asks to see a specific file".to_string(),
            command: "file_read src/lib.rs".to_string(),
            reasoning: "Direct file access is the fastest way to view known file".to_string(),
        },
        ToolExample {
            scenario: "Investigating a reported error location".to_string(),
            command: "file_read src/main.rs:100:120".to_string(),
            reasoning: "Reading the specific line range where error occurs".to_string(),
        },
    ],
    not_examples: vec![
        ToolExample {
            scenario: "Finding files matching a pattern".to_string(),
            command: "file_search \"**/*.rs\"".to_string(),
            reasoning: "Use file_search for pattern matching, file_read for specific files".to_string(),
        },
        ToolExample {
            scenario: "Getting an overview of file structure".to_string(),
            command: "ls src/".to_string(),
            reasoning: "Listing directory is faster than reading every file".to_string(),
        },
    ],
    token_cost: 150,
    gated: false,
}
```

---

### 2. `src/tools/core.rs` (~200 LOC)

**Purpose**: Core tool definitions (always loaded)

**Responsibilities**:
- Define core tool set
- Provide metadata for each core tool
- Export for LLM context building

**API**:
```rust
//! Core tools — always loaded into LLM context
//!
//! # Core Tool Criteria
//!
//! A tool is "core" if:
//! - Used in >50% of typical sessions
//! - Required for basic workflow (read → edit → test)
//! - Not discoverable from context alone
//!
//! # Current Core Tools (5)
//!
//! 1. file_read — Reading file contents
//! 2. file_search — Finding files by pattern
//! 3. splice_patch — Single symbol replacement
//! 4. llm_explain — Code understanding
//! 5. bash_exec — Terminal commands

use crate::tools::metadata::{ToolMetadata, ToolCategory, ToolExample};

pub fn core_tools() -> Vec<ToolMetadata> {
    vec![
        file_read_metadata(),
        file_search_metadata(),
        splice_patch_metadata(),
        llm_explain_metadata(),
        bash_exec_metadata(),
    ]
}

fn file_read_metadata() -> ToolMetadata {
    ToolMetadata {
        name: "file_read".to_string(),
        category: ToolCategory::Core,
        description: "Read the complete contents of a file. \
            Use for reading specific known files. \
            Supports line range with file:path:line_start:line_end format.".to_string(),
        examples: vec![
            ToolExample {
                scenario: "Read a specific file".to_string(),
                command: "file_read src/lib.rs".to_string(),
                reasoning: "Direct file access is fastest for known files".to_string(),
            },
            ToolExample {
                scenario: "Read specific lines where error occurs".to_string(),
                command: "file_read src/main.rs:100:120".to_string(),
                reasoning: "Line range focuses on relevant code".to_string(),
            },
        ],
        not_examples: vec![
            ToolExample {
                scenario: "Finding files matching pattern".to_string(),
                command: "file_search \"**/*.test.rs\"".to_string(),
                reasoning: "file_search finds files, file_read reads them".to_string(),
            },
        ],
        token_cost: 150,
        gated: false,
    }
}

// ... other core tool metadata functions
```

---

### 3. `src/tools/specialized.rs` (~200 LOC)

**Purpose**: Specialized tool definitions (discovered on-demand)

**Responsibilities**:
- Define specialized tool set
- Provide metadata for each specialized tool
- Define discovery triggers

**API**:
```rust
//! Specialized tools — discovered on-demand based on context
//!
//! # Discovery Criteria
//!
//! A tool is "specialized" if:
//! - Used in <20% of typical sessions
//! - Triggered by specific keywords in user query
//! - Discoverable from user intent
//!
//! # Discovery Triggers
//!
//! Each specialized tool defines:
//! - Keywords that trigger discovery
//! - Task patterns that indicate need

use crate::tools::metadata::{ToolMetadata, ToolCategory};

/// All specialized tools with discovery triggers
pub fn specialized_tools() -> Vec<SpecializedTool> {
    vec![
        splice_plan_tool(),
        file_write_tool(),
        file_create_tool(),
        file_edit_tool(),
        file_glob_tool(),
        symbols_in_file_tool(),
        // ... etc
    ]
}

/// Specialized tool with discovery rules
pub struct SpecializedTool {
    pub metadata: ToolMetadata,
    pub triggers: Vec<DiscoveryTrigger>,
}

/// Conditions that trigger tool discovery
pub enum DiscoveryTrigger {
    /// Keyword in user message
    Keyword(String),

    /// Keyword in recent tool output
    InOutput(String),

    /// Combination of tools used
    ToolPattern(Vec<String>),
}
```

---

### 4. `src/tools/discovery.rs` (~250 LOC)

**Purpose**: Discovery engine — determine which tools to include

**Responsibilities**:
- Analyze user query and context
- Determine which specialized tools to load
- Return complete tool set for LLM context

**API**:
```rust
//! Progressive tool discovery engine
//!
//! # Discovery Algorithm
//!
//! 1. Start with core tools (always loaded)
//! 2. Analyze user query for trigger keywords
//! 3. Check recent tool outputs for secondary triggers
//! 4. Return core + discovered specialized tools
//!
//! # Grounding
//!
//! All discovery events are logged to execution_log.db

use crate::tools::{core, specialized};
use crate::tools::metadata::ToolMetadata;

pub struct DiscoveryEngine {
    core_tools: Vec<ToolMetadata>,
    specialized_tools: Vec<specialized::SpecializedTool>,
}

impl DiscoveryEngine {
    pub fn new() -> Self {
        Self {
            core_tools: core::core_tools(),
            specialized_tools: specialized::specialized_tools(),
        }
    }

    /// Discover tools based on user query and context
    ///
    /// Returns: (core tools, discovered specialized tools)
    pub fn discover(
        &self,
        user_query: &str,
        recent_outputs: &[String],
    ) -> DiscoveryResult {
        let mut discovered = Vec::new();

        for tool in &self.specialized_tools {
            if self.should_discover(tool, user_query, recent_outputs) {
                discovered.push(tool.metadata.clone());
            }
        }

        DiscoveryResult {
            core: self.core_tools.clone(),
            specialized: discovered,
            total_token_cost: self.calculate_tokens(&self.core_tools, &discovered),
        }
    }

    /// Determine if a specialized tool should be discovered
    fn should_discover(
        &self,
        tool: &specialized::SpecializedTool,
        query: &str,
        outputs: &[String],
    ) -> bool {
        let query_lower = query.to_lowercase();

        for trigger in &tool.triggers {
            match trigger {
                specialized::DiscoveryTrigger::Keyword(keyword) => {
                    if query_lower.contains(&keyword.to_lowercase()) {
                        return true;
                    }
                }
                specialized::DiscoveryTrigger::InOutput(pattern) => {
                    for output in outputs {
                        if output.to_lowercase().contains(&pattern.to_lowercase()) {
                            return true;
                        }
                    }
                }
                specialized::DiscoveryTrigger::ToolPattern(tools) => {
                    // Check if all tools in pattern were recently used
                    // (implementation details...)
                }
            }
        }

        false
    }

    fn calculate_tokens(&self, core: &[ToolMetadata], specialized: &[ToolMetadata]) -> usize {
        core.iter().map(|t| t.token_cost).sum::<usize>()
            + specialized.iter().map(|t| t.token_cost).sum::<usize>()
    }
}

pub struct DiscoveryResult {
    pub core: Vec<ToolMetadata>,
    pub specialized: Vec<ToolMetadata>,
    pub total_token_cost: usize,
}
```

---

### 5. `src/tools/mod.rs` (~50 LOC)

**Purpose**: Facade for tool system

**API**:
```rust
//! Tool system with progressive discovery
//!
//! # Usage
//!
//! ```rust
//! use odincode::tools::DiscoveryEngine;
//!
//! let engine = DiscoveryEngine::new();
//! let result = engine.discover(user_query, recent_outputs);
//!
//! // result.core — always loaded
//! // result.specialized — discovered from context
//! // result.total_token_cost — for logging
//! ```

pub use metadata::{ToolMetadata, ToolCategory, ToolExample};
pub use discovery::{DiscoveryEngine, DiscoveryResult};

pub mod metadata;
pub mod core;
pub mod specialized;
pub mod discovery;
```

---

## Discovery Trigger Examples

| Tool | Trigger Keywords | Trigger Patterns |
|------|-----------------|------------------|
| `splice_plan` | "plan", "refactor", "multi-step" | file_read + file_search used |
| `file_write` | "create file", "write to", "save" | None |
| `file_create` | "new file", "add file" | None |
| `file_edit` | "edit line", "change line" | None |
| `file_glob` | "glob", "pattern", "all files" | None |
| `symbols_in_file` | "functions", "types", "definitions" | None |
| `references_to_symbol_name` | "where used", "callers", "references" | None |
| `references_from_file_to_symbol_name` | "imports", "dependencies", "uses" | None |
| `lsp_check` | "errors", "diagnostics", "check" | None |
| `git_status` | "git status", "changes", "modified" | None |
| `git_diff` | "diff", "what changed", "changes" | None |
| `git_log` | "history", "commits", "blame" | None |
| `memory_query` | "previous", "before", "earlier", "history" | None |
| `execution_summary` | "summary", "what happened", "what did I do" | None |
| `wc` | "count", "lines", "size" | None |

---

## Integration with LLM Adapters

### Before: All Tools Loaded

```rust
// src/llm/adapters/normalize.rs (current)

let all_tools = TOOL_WHITELIST;  // 20 tools
let request = normalize_for_openai_compatible(
    messages_with_tools(all_tools),
    model,
    stream,
);
```

### After: Progressive Discovery

```rust
// src/llm/adapters/normalize.rs (proposed)

use crate::tools::DiscoveryEngine;

let engine = DiscoveryEngine::new();
let discovery = engine.discover(user_query, recent_outputs);

let active_tools = discovery.core
    .into_iter()
    .chain(discovery.specialized)
    .collect::<Vec<_>>();

let request = normalize_for_openai_compatible(
    messages_with_tools(active_tools),
    model,
    stream,
);
```

---

## System Prompt Changes

### New: Tool Usage Guidance

```rust
// src/llm/prompts.rs (NEW FILE)

pub const TOOL_GUIDANCE: &str = r#"
## Tool Selection Guidelines

You have access to a curated set of tools. More tools will be added as needed.

### Core Tools (Always Available)

- **file_read**: Read specific files. Use when you know the file path.
- **file_search**: Find files matching patterns. Use when exploring or searching.
- **splice_patch**: Replace a single symbol. Use for focused changes.
- **llm_explain**: Understand code behavior. Use for "what does this do?" questions.
- **bash_exec**: Run commands. Use for git, tests, building.

### When to Ask for More Tools

If you need functionality not available in core tools, the system will provide additional tools.

### Tool Usage Principles

1. **Be specific**: Use file_read for known paths, file_search for discovery
2. **Start simple**: Try core tools before requesting specialized ones
3. **Read before editing**: Always file_read before splice_patch
4. **Check before assuming**: Use lsp_check to verify errors actually exist

### Common Mistakes to Avoid

- ❌ Don't use file_search when you know the exact path
- ❌ Don't read multiple files when one will suffice
- ❌ Don't run bash commands for file operations (use file_* tools)
- ❌ Don't splice_patch without reading the file first
"#;
```

---

## Execution Memory Logging

### New Discovery Events

When tools are discovered, log to `execution_log.db`:

```sql
-- New table for discovery events
CREATE TABLE discovery_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    user_query_hash TEXT NOT NULL,
    tools_discovered TEXT NOT NULL,  -- JSON array of tool names
    reason TEXT NOT NULL,             -- Why these tools were discovered
    timestamp INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);
```

**Why log discovery**:
- Audit trail of why tools were available
- Debug "why did LLM have tool X?"
- Learn patterns for better trigger definitions

---

## Testing Strategy

### Unit Tests

```rust
// src/tools/tests/discovery_tests.rs

#[test]
fn test_core_tools_always_loaded() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("", &[]);

    assert!(!result.core.is_empty());
    assert_eq!(result.core.len(), 5);  // file_read, file_search, splice_patch, llm_explain, bash_exec
}

#[test]
fn test_git_tools_discovered_on_git_keyword() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("show me git history", &[]);

    let tool_names: Vec<_> = result.specialized.iter()
        .map(|t| t.name.as_str())
        .collect();

    assert!(tool_names.contains(&"git_log"));
}

#[test]
fn test_no_discovery_for_generic_query() {
    let engine = DiscoveryEngine::new();
    let result = engine.discover("read src/lib.rs", &[]);

    assert!(result.specialized.is_empty());
}

#[test]
fn test_token_cost_reduction() {
    let engine = DiscoveryEngine::new();

    // All tools would be ~8000 tokens
    let generic_result = engine.discover("read file", &[]);

    // Core only should be ~1100 tokens
    assert!(generic_result.total_token_cost < 1500);
}
```

### Integration Tests

```rust
#[test]
fn test_full_discovery_workflow() {
    // 1. User query
    let query = "refactor the error handling in src/main.rs";

    // 2. Discover tools
    let engine = DiscoveryEngine::new();
    let result = engine.discover(query, &[]);

    // 3. Verify expected tools discovered
    assert!(result.specialized.iter().any(|t| t.name == "splice_plan"));

    // 4. Verify LLM context includes discovered tools
    let context = build_llm_context(&result);
    assert!(context.contains("splice_plan"));
}
```

---

## Implementation Order

1. **Phase 10.1**: ✅ Tool metadata structure (`metadata.rs`) — COMPLETE
2. **Phase 10.2**: ✅ Core tool definitions (`core.rs`) — COMPLETE
3. **Phase 10.3**: ✅ Specialized tool definitions (`specialized.rs`) — COMPLETE
4. **Phase 10.4**: ✅ Discovery engine (`discovery.rs`) — COMPLETE
5. **Phase 10.5**: ✅ System prompt with tool guidance (`prompts.rs`) — COMPLETE
6. **Phase 10.6**: ✅ LLM adapter integration — COMPLETE
7. **Phase 10.7**: ✅ Discovery event logging — COMPLETE
8. **Phase 10.8**: ✅ Integration tests — COMPLETE

Each phase:
1. Write failing test
2. Implement module
3. Verify test passes
4. Update this doc with changes

---

## Phase 10.1 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `src/tools/mod.rs` | 50 | Module facade, exports, MAX_TOOLS_AT_ONCE constant |
| `src/tools/metadata.rs` | 370 | ToolMetadata, ToolCategory, ToolExample, DiscoveryTrigger, SpecializedTool, DiscoveryResult |
| `tests/phase_10_1_tool_metadata_tests.rs` | 308 | TDD tests (15 tests) |

**Tests**: 15/15 passing (100%)
- Serialization tests (5)
- Validation tests (2)
- Visibility tests (3)
- Builder tests (2)
- Discovery result tests (3)

**Key Types Implemented**:
- `ToolMetadata`: name, category, description, examples, not_examples, token_cost, gated
- `ToolCategory`: Core, Specialized, Internal
- `ToolExample`: scenario, command, reasoning
- `DiscoveryTrigger`: Keyword, InOutput, ToolPattern
- `SpecializedTool`: metadata + triggers (with builder pattern)
- `DiscoveryResult`: core + specialized + total_token_cost

---

## Phase 10.2 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `src/tools/core.rs` | 271 | Core tool definitions with metadata |
| `tests/phase_10_2_core_tools_tests.rs` | 175 | TDD tests (17 tests) |

**Tests**: 17/17 passing (100%)
- Core tools count (1 test)
- Category validation (1 test)
- Validity checks (1 test)
- Visibility checks (1 test)
- Individual tool presence (5 tests)
- Example naming conventions (5 tests)
- Token cost validation (1 test)
- Example completeness (2 tests)
- Internal module tests (3 tests)

**Core Tools Defined**:
1. `file_read` — Read file contents (100 base tokens, 3 examples, 2 not-examples)
2. `file_search` — Find files by pattern (100 base tokens, 3 examples, 2 not-examples)
3. `splice_patch` — Single symbol replacement (150 base tokens, 2 examples, 2 not-examples)
4. `bash_exec` — Terminal commands (150 base tokens, 3 examples, 3 not-examples)
5. `display_text` — Show LLM text responses (50 base tokens, 2 examples, 1 not-example)

**Total Token Cost**: ~550 base + examples = < 1500 estimated

**Note**: Uses `display_text` instead of `llm_explain` (actual tool in TOOL_WHITELIST at src/llm/router.rs:32)

---

## Phase 10.3 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `src/tools/specialized.rs` | 194 | Specialized tool definitions with discovery triggers |
| `tests/phase_10_3_specialized_tools_tests.rs` | 202 | TDD tests (29 tests) |

**Files Modified**:
| File | Changes |
|------|---------|
| `src/tools/mod.rs` | Added `pub mod specialized` and `pub use specialized::specialized_tools` |

**Tests**: 29/29 passing (100%)
- Tool count and category validation (2 tests)
- Validity and visibility checks (3 tests)
- Individual tool presence tests (15 tests)
- Discovery trigger tests (4 tests)
- Example validation tests (3 tests)
- Discovery behavior tests (2 tests)

**15 Specialized Tools Defined**:
| Tool | Triggers | Token Cost |
|------|----------|------------|
| file_write | write, save | 100 |
| file_create | create new file, new file, add file | 80 |
| file_glob | glob, pattern, all files | 80 |
| file_edit | edit line, change line, replace lines | 100 |
| splice_plan | multi-step, refactor plan, plan | 120 |
| symbols_in_file | functions, types, symbols, definitions | 100 |
| references_to_symbol_name | where used, callers, references, usages | 80 |
| references_from_file_to_symbol_name | imports, dependencies, uses, depends on | 80 |
| lsp_check | error, diagnostic, check, compile | 80 |
| memory_query | previous, before, history, past, earlier | 80 |
| execution_summary | summary, statistics, stats, what happened | 80 |
| git_status | git status, git | 70 |
| git_diff | diff, what changed, git, changes | 70 |
| git_log | history, commits, log, git | 70 |
| wc | count, lines, size, loc | 50 |

**Total Token Cost**: ~1200 base + examples (estimated)

**Discovery Mechanism**: Each tool uses `SpecializedTool::new()` builder pattern with `.with_trigger()` for keyword discovery. The `should_discover()` method (from metadata.rs) checks if query keywords match triggers.

**Next**: Phase 10.4 — Discovery Engine

---

## Phase 10.4 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `src/tools/discovery.rs` | 95 | Discovery engine with `DiscoveryEngine::discover()` |
| `tests/phase_10_4_discovery_tests.rs` | 230 | TDD tests (21 tests) |

**Files Modified**:
| File | Changes |
|------|---------|
| `src/tools/mod.rs` | Added `pub mod discovery` and `pub use discovery::DiscoveryEngine` |
| `src/tools/specialized.rs` | Added `InOutput("error")` trigger to `lsp_check` |

**Tests**: 21/21 passing (100%)
- Basic structure tests (2 tests)
- Core tools always included (2 tests)
- Keyword discovery tests (5 tests)
- InOutput trigger tests (2 tests)
- Token cost calculation tests (3 tests)
- DiscoveryResult methods tests (2 tests)
- Multiple tools discovery tests (2 tests)
- Trigger integration tests (3 tests)

**Discovery Engine API**:
```rust
pub struct DiscoveryEngine {
    core_tools: Vec<ToolMetadata>,
    specialized_tools: Vec<SpecializedTool>,
}

impl DiscoveryEngine {
    pub fn new() -> Self { ... }
    pub fn discover(&self, user_query: &str, recent_outputs: &[String]) -> DiscoveryResult { ... }
}
```

**Discovery Algorithm**:
1. Start with all 5 core tools (always loaded)
2. For each specialized tool, check `should_discover(query, outputs)`
3. Include tool if any trigger matches:
   - `Keyword(k)` — query contains `k` (case-insensitive)
   - `InOutput(p)` — any recent output contains `p` (case-insensitive)
   - `ToolPattern(...)` — placeholder for future pattern matching
4. Return `DiscoveryResult` with core + discovered specialized + total token cost

**Token Cost Calculation**:
- Sums `token_cost` field from all included tools
- Core tools: ~550 tokens
- Specialized tools: vary by discovery

**Example Discovery**:
| Query | Discovered Tools | Total Cost |
|-------|------------------|------------|
| `"write file"` | file_write | ~650 |
| `"check git status"` | git_status | ~620 |
| `"error in code"` | lsp_check | ~630 |
| `"write and check git"` | file_write, git_status | ~720 |

**Total Phase 10 Tests**: 82/82 passing
- Phase 10.1: 15 tests
- Phase 10.2: 17 tests
- Phase 10.3: 29 tests
- Phase 10.4: 21 tests

**Next**: Phase 10.5 — System Prompt Generation

---

## Phase 10.5 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `src/tools/prompts.rs` | 144 | System prompt generation for tool descriptions |
| `tests/phase_10_5_prompts_tests.rs` | 220 | TDD tests (14 tests) |

**Files Modified**:
| File | Changes |
|------|---------|
| `src/tools/mod.rs` | Added `pub mod prompts` and re-exports |

**Tests**: 14/14 passing (100%)
- Basic formatting tests (3 tests)
- Discovery result formatting tests (3 tests)
- System prompt generation tests (3 tests)
- Gated tool marking test (1 test)
- Empty discovery handling test (1 test)
- Structured output consistency test (1 test)
- Specialized tools section tests (2 tests)

**Prompt Generation API**:
```rust
// Format individual tools
pub fn format_tool(tool: &ToolMetadata) -> String
pub fn format_tools(tools: &[ToolMetadata]) -> String

// Format discovery results
pub fn format_discovery_result(result: &DiscoveryResult) -> String

// Generate system prompts
pub fn system_prompt(result: &DiscoveryResult) -> String
pub fn system_prompt_with_metadata(result: &DiscoveryResult) -> (String, PromptMetadata)

// Utilities
pub fn estimate_tokens(text: &str) -> usize
```

**Prompt Structure**:
1. **Tool Selection Guidelines** — Principles and common mistakes
2. **Core Tools Section** — Always-present core tools with examples
3. **Specialized Tools Section** — Only included when tools discovered

**Example Generated Prompt**:
```markdown
## Tool Selection Guidelines

You have access to a curated set of tools...

### Tool Usage Principles

1. **Be specific**: Use `file_read` for known paths...
2. **Start simple**: Try core tools first...
...

## Available Tools

### Core Tools

**file_read**
Read the complete contents of a file

**When to use:**
- *Read specific file*: `file_read src/lib.rs`
  - Direct file access when path is known
...

### Specialized Tools

**file_write** ⚠️ *GATED: Requires approval*
Atomically write contents to a file
...
```

**Token Estimation**: Simple heuristic of ~4 characters per token (consistent with metadata.rs)

**Total Phase 10 Tests**: 96/96 passing
- Phase 10.1: 15 tests
- Phase 10.2: 17 tests
- Phase 10.3: 29 tests
- Phase 10.4: 21 tests
- Phase 10.5: 14 tests

**Next**: Phase 10.6 — LLM Adapter Integration

---

## Phase 10.6 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `src/llm/discovery.rs` | 96 | LLM integration for progressive tool discovery |
| `tests/phase_10_6_integration_tests.rs` | 180 | TDD tests (14 tests) |

**Files Modified**:
| File | Changes |
|------|---------|
| `src/llm/mod.rs` | Added `pub mod discovery` and re-exports |

**Tests**: 14/14 passing (100%)
- Discovery context tests (2 tests)
- Chat discovery tests (4 tests)
- Plan discovery tests (3 tests)
- Integration with DiscoveryEngine (2 tests)
- Token cost tracking (1 test)
- Whitelist validation (2 tests)

**LLM Integration API**:
```rust
// Context for discovery
pub struct ToolDiscoveryContext {
    pub user_query: String,
    pub recent_outputs: Vec<String>,
    pub discovered_tools: Vec<String>,
}

impl ToolDiscoveryContext {
    pub fn new(user_query: impl Into<String>) -> Self;
    pub fn with_recent_output(self, output: impl Into<String>) -> Self;
    pub fn with_recent_outputs(self, outputs: Vec<String>) -> Self;
}

// Discovery functions
pub fn discover_tools_for_chat(context: &ToolDiscoveryContext) -> Vec<String>;
pub fn discover_tools_for_plan(context: &ToolDiscoveryContext) -> (Vec<String>, String);
```

**Usage Examples**:
```rust
use odincode::llm::{ToolDiscoveryContext, discover_tools_for_chat, discover_tools_for_plan};

// Chat mode — get just tool names
let context = ToolDiscoveryContext::new("write a file");
let tools = discover_tools_for_chat(&context);
// tools = ["bash_exec", "display_text", "file_read", "file_search",
//          "file_write", "splice_patch", ...]

// Plan mode — get tools + system prompt
let context = ToolDiscoveryContext::new("write a file")
    .with_recent_output("error: something went wrong");
let (tools, prompt) = discover_tools_for_plan(&context);
// prompt includes tool descriptions, examples, and guidelines
```

**Integration Points**:
1. Uses `DiscoveryEngine` from `tools` module
2. Leverages `system_prompt()` from `tools::prompts` for plan mode
3. Returns sorted tool names for deterministic behavior
4. Validates all discovered tools are in `TOOL_WHITELIST`

---

## Phase 10.7 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `src/execution_tools/discovery_log.rs` | 127 | Discovery event logging functions |
| `tests/phase_10_7_logging_tests.rs` | 150 | TDD tests (8 tests) |

**Files Modified**:
| File | Changes |
|------|---------|
| `src/execution_tools/mod.rs` | Added `pub mod discovery_log` and re-exports |
| `src/execution_tools/db.rs` | Added `discovery_events` table + indexes to schema |

**Tests**: 8/8 passing (100%)
- Schema tests (2 tests)
- Logging tests (4 tests)
- Query tests (2 tests)

**Schema Changes**:
```sql
-- New table
CREATE TABLE discovery_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    user_query_hash TEXT NOT NULL,
    tools_discovered TEXT NOT NULL,
    reason TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

-- New indexes
CREATE INDEX idx_discovery_session ON discovery_events(session_id);
CREATE INDEX idx_discovery_timestamp ON discovery_events(timestamp);
CREATE INDEX idx_discovery_query_hash ON discovery_events(user_query_hash);
```

**API**:
```rust
// Discovery event record
pub struct DiscoveryEvent {
    pub id: i64,
    pub session_id: String,
    pub user_query_hash: String,
    pub tools_discovered: Vec<String>,
    pub reason: String,
    pub timestamp: i64,
}

// Logging functions
pub fn log_discovery_event(
    db: &ExecutionDb,
    session_id: &str,
    context: &ToolDiscoveryContext,
    tools_discovered: &[String],
    reason: &str,
) -> Result<(), Error>;

pub fn query_discovery_events(
    db: &ExecutionDb,
    session_id: &str,
) -> Result<Vec<DiscoveryEvent>, Error>;
```

**Usage Examples**:
```rust
use odincode::execution_tools::{log_discovery_event, query_discovery_events};
use odincode::llm::discovery::{ToolDiscoveryContext, discover_tools_for_chat};

// Log a discovery event
let context = ToolDiscoveryContext::new("write a file");
let tools = discover_tools_for_chat(&context);
log_discovery_event(&db, "session_123", &context, &tools, "keyword: write")?;

// Query events for a session
let events = query_discovery_events(&db, "session_123")?;
// events sorted by timestamp (ascending)
```

**Purpose**:
- Audit trail of which tools were available during LLM interactions
- Debug "why did LLM have tool X?" situations
- Learn patterns for better trigger definitions
- Track query-to-discovery mappings via hash

---

## Phase 10.8 Completion Report

**Completed**: 2025-12-26

**Files Created**:
| File | LOC | Purpose |
|------|-----|---------|
| `tests/phase_10_8_integration_tests.rs` | 360 | End-to-end integration tests (14 tests) |

**Tests**: 14/14 passing (100%)
- End-to-end workflow tests (4 tests)
- Logging integration tests (2 tests)
- Whitelist validation tests (2 tests)
- Prompt quality tests (2 tests)
- Output-based discovery tests (1 test)
- Token cost tracking tests (2 tests)
- Progressive discovery tests (1 test)

**Test Coverage**:
```
test_end_to_end_discovery_workflow          ✅
test_end_to_end_chat_mode_workflow          ✅
test_end_to_end_plan_mode_workflow          ✅
test_progressive_discovery_workflow         ✅
test_token_cost_accuracy_across_workflow    ✅
test_discovery_logging_workflow             ✅
test_multi_query_session_workflow           ✅
test_all_discovered_tools_in_whitelist      ✅
test_chat_mode_validates_whitelist          ✅
test_prompt_includes_all_required_sections  ✅
test_prompt_formatting_consistency          ✅
test_output_based_discovery_workflow        ✅
test_discovery_context_tracks_token_cost    ✅
test_core_tools_token_cost_is_minimum       ✅
```

**Integration Validated**:
1. Full discovery workflow: User query → DiscoveryEngine → ToolDiscoveryContext
2. Chat mode integration: discover_tools_for_chat → tool names
3. Plan mode integration: discover_tools_for_plan → (tools + system prompt)
4. Logging integration: log_discovery_event → query_discovery_events
5. Whitelist validation: All discovered tools in TOOL_WHITELIST
6. Token cost accuracy: Core tools cost matches expected minimum
7. Progressive discovery: Generic queries discover fewer tools than specific ones
8. Output-based discovery: InOutput triggers work correctly
9. Session-based workflows: Multiple queries logged correctly
10. Prompt quality: System prompts include required sections

**Total Phase 10 Tests**: 132/132 passing
- Phase 10.1: 15 tests
- Phase 10.2: 17 tests
- Phase 10.3: 29 tests
- Phase 10.4: 21 tests
- Phase 10.5: 14 tests
- Phase 10.6: 14 tests
- Phase 10.7: 8 tests
- Phase 10.8: 14 tests

**Next**: Phase 10 COMPLETE 🎉


---

## Open Questions

1. **Should user be able to override discovery?** (e.g., `--tools all` flag)
2. **How to handle recursive discovery?** (discovered tools triggering more discovery)
3. **Should discovery be session-scoped or per-query?**
4. **How to visualize discovery in TUI?** (show available vs. discovered tools)

---

## Expected Impact

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Tool description tokens | ~8,000 | ~1,100 | 86% reduction |
| Initial context size | ~20,000 | ~13,000 | 35% reduction |
| Tool selection quality | Poor (overwhelmed) | Better (focused) | Qualitative |
| Discovery latency | 0ms | ~10ms | Acceptable |

---

## Design Decisions Log

| Date | Decision | Rationale |
|------|----------|-----------|
| 2025-12-26 | Core = 5 tools only | Most common use cases |
| 2025-12-26 | Keyword-based discovery | Simple, predictable |
| 2025-12-26 | Log discovery events | Audit trail + learning |
| 2025-12-26 | Examples teach behavior | Claude Code leak insight |
| 2025-12-26 | "When NOT to use" required | Prevents tool misuse |

---

*Last Updated: 2025-12-26*
*Status: Phase 10.3 Complete ✅*
