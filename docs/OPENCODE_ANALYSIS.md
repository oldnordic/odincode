# OpenCode Analysis: Features, Patterns, and Applicability to OdinCode

**Date**: 2025-12-26
**Source**: https://opencode.ai/, /home/feanor/Projects/opencode
**Purpose**: Identify features from OpenCode that can be leveraged without breaking OdinCode's core principles

---

## Executive Summary

| Category | OpenCode Has | OdinCode Principle | Verdict |
|----------|-------------|-------------------|---------|
| **Tool Registry** | Dynamic loading, plugin system | NO GUESSING (real tools only) | ✅ Adopt pattern |
| **Permission System** | ask/allow/deny per tool | NO GUESSING (explicit control) | ✅ Relevant |
| **Agent Configuration** | File-based (.md) prompts | NO INTELLIGENCE (we're substrate) | ⚠️ Partial |
| **Config Management** | Layered JSONC with env/file refs | Deterministic | ✅ Adopt pattern |
| **Zod Validation** | Runtime type safety | Real tools | ✅ Adopt pattern |
| **UI/Animations** | Terminal TUI with Ink | NO EDITOR UI (console only) | ❌ Out of scope |
| **Multi-session** | Parallel agent instances | Tool substrate | ❌ Different layer |
| **MCP Protocol** | External tool integration | Real tools only | ⚠️ Consider |

---

## OpenCode Architecture Overview

### Project Structure
```
/home/feanor/Projects/opencode/
├── packages/
│   ├── opencode/        # Core CLI (TypeScript/Bun)
│   ├── console/         # Web UI (SolidJS)
│   ├── desktop/         # Tauri desktop app
│   ├── ui/              # Shared UI components
│   └── sdk/             # JavaScript SDK
```

### Key Statistics
- **41,000+** GitHub stars
- **450+** Contributors
- **400,000+** Monthly developers
- **75+** LLM providers supported
- **20+** Packages in Turborepo monorepo

---

## 1. Tool System: Registry Pattern

### What OpenCode Does

```typescript
// packages/opencode/src/tool/registry.ts
export namespace ToolRegistry {
  export const state = Instance.state(async () => {
    const custom = [] as Tool.Info[]

    // Load from .opencode/tool/*.{js,ts}
    for (const dir of await Config.directories()) {
      for await (const match of glob.scan({ cwd: dir })) {
        const mod = await import(match)
        for (const [id, def] of Object.entries<ToolDefinition>(mod)) {
          custom.push(fromPlugin(id, def))
        }
      }
    }

    // Load from plugins
    const plugins = await Plugin.list()
    for (const plugin of plugins) {
      for (const [id, def] of Object.entries(plugin.tool ?? {})) {
        custom.push(fromPlugin(id, def))
      }
    }

    return { custom }
  })

  // Built-in tools
  async function all(): Promise<Tool.Info[]> {
    return [
      InvalidTool, BashTool, ReadTool, GlobTool, GrepTool,
      EditTool, WriteTool, TaskTool, WebFetchTool,
      TodoWriteTool, TodoReadTool, WebSearchTool, CodeSearchTool,
      SkillTool, LspTool, BatchTool,
      ...custom,  // User-defined tools
    ]
  }
}
```

### Tool Definition Pattern

```typescript
// packages/opencode/src/tool/read.ts
export const ReadTool = Tool.define("read", {
  description: DESCRIPTION,
  parameters: z.object({
    filePath: z.string().describe("The path to the file to read"),
    offset: z.coerce.number().describe("Line number to start from (0-based)").optional(),
    limit: z.coerce.number().describe("Number of lines to read (defaults to 2000)").optional(),
  }),

  async execute(params, ctx) {
    // Tool implementation
    return {
      title: path.relative(Instance.worktree, filepath),
      output: formattedContent,
      metadata: { preview: snippet },
    }
  },
})
```

### Applicability to OdinCode

**Verdict**: ✅ **ADOPT** (with adaptation)

**Why it aligns**:
- OdinCode already has TOOL_WHITELIST (20 tools)
- Registry pattern enables extensibility without breaking "real tools only" principle
- Type-safe parameter validation matches our deterministic approach

**Adaptation needed**:
```rust
// OdinCode already has:
// src/execution_tools/tool_registry.rs (Phase 9.x)

// Potential enhancement:
pub struct ToolRegistry {
    built_in: Vec<ToolMetadata>,
    // Future: custom tools from ~/.odincode/tools/*.rs
}

impl ToolRegistry {
    pub fn register(&mut self, tool: ToolMetadata) -> Result<()> {
        // Validate tool has real binary backing
        if !tool.command_exists() {
            return Err(Error::ToolNotFound(tool.command));
        }
        self.built_in.push(tool);
        Ok(())
    }
}
```

---

## 2. Permission System: Granular Control

### What OpenCode Does

```typescript
// packages/opencode/src/permission/index.ts
export namespace Permission {
  // Permission types
  export const Response = z.enum(["once", "always", "reject"])

  // Ask for permission
  export async function ask(input: {
    type: Info["type"]
    title: Info["title"]
    pattern?: Info["pattern"]
    sessionID: Info["sessionID"]
    messageID: Info["messageID"]
    metadata: Info["metadata"]
  }) {
    const { approved } = state()

    // Check if already approved for this pattern
    if (covered(keys, approvedForSession)) return

    // Create permission request
    const info: Info = { /* ... */ }

    // Wait for user response
    return new Promise<void>((resolve, reject) => {
      pending[sessionID][info.id] = { info, resolve, reject }
      Bus.publish(Event.Updated, info)
    })
  }
}
```

### Agent Permission Configuration

```typescript
// packages/opencode/src/agent/agent.ts
const planPermission = {
  edit: "deny",  // Cannot edit files
  bash: {
    "git diff*": "allow",
    "git status*": "allow",
    "find * -delete*": "ask",
    "*": "ask",  // Default to ask for unknown commands
  },
  webfetch: "allow",
  doom_loop: "ask",
  external_directory: "ask",
}
```

### Tool-Level Permission Check

```typescript
// packages/opencode/src/tool/read.ts
async execute(params, ctx) {
  const agent = await Agent.get(ctx.agent)

  // Check external directory permission
  if (!Filesystem.contains(Instance.directory, filepath)) {
    if (agent.permission.external_directory === "ask") {
      await Permission.ask({
        type: "external_directory",
        pattern: [parentDir, path.join(parentDir, "*")],
        sessionID: ctx.sessionID,
        messageID: ctx.messageID,
        callID: ctx.callID,
        title: `Access file outside working directory: ${filepath}`,
        metadata: { filepath, parentDir },
      })
    } else if (agent.permission.external_directory === "deny") {
      throw new Permission.RejectedError(/* ... */)
    }
  }

  // Proceed with tool execution
}
```

### Applicability to OdinCode

**Verdict**: ✅ **HIGHLY RELEVANT** (similar to GATED tools)

**What OdinCode already has** (Phase 9.2):
```rust
// GATED tools require approval
pub const GATED_TOOLS: &[&str] = &["file_write", "file_create"];

// Approval workflow already exists
```

**Enhancement opportunity**:
```rust
// Could add wildcard pattern matching like OpenCode
pub enum Permission {
    Allow,
    Ask,
    Deny,
}

pub struct ToolPermission {
    pub tool: String,
    pub permission: Permission,
    pub pattern: Option<String>,  // e.g., "rm *" -> Ask
}
```

---

## 3. Configuration Management: Layered Loading

### What OpenCode Does

```typescript
// packages/opencode/src/config/config.ts
export namespace Config {
  export const state = Instance.state(async () => {
    let result = await global()

    // 1. Load from global config
    result = mergeConfigWithPlugins(
      result,
      await loadFile(path.join(Global.Path.config, "opencode.jsonc"))
    )

    // 2. Traverse up directory tree
    for (const file of ["opencode.jsonc", "opencode.json"]) {
      const found = await Filesystem.findUp(file, Instance.directory)
      for (const resolved of found.toReversed()) {
        result = mergeConfigWithPlugins(result, await loadFile(resolved))
      }
    }

    // 3. Load from .opencode directories
    for (const dir of directories) {
      result.agent = mergeDeep(result.agent, await loadAgent(dir))
      result.command = mergeDeep(result.command ?? {}, await loadCommand(dir))
      result.plugin.push(...(await loadPlugin(dir)))
    }

    return { config: result, directories }
  })
}
```

### Special Features

**File references**:
```json
{
  "apiKey": "{file:/secrets/openai-key.txt}"
}
```

**Environment variables**:
```json
{
  "apiKey": "{env:OPENAI_API_KEY}"
}
```

**JSONC with comments**:
```jsonc
{
  // My custom model
  "model": "anthropic/claude-sonnet-4",  // Latest model
}
```

### Applicability to OdinCode

**Verdict**: ✅ **ADOPT** (Phase 10+ consideration)

**Why it aligns**:
- Deterministic configuration loading
- Layered merging is predictable
- File/env references useful for secrets

**Current OdinCode state**:
- Hardcoded model/config in CLI
- No external configuration files

**Potential enhancement**:
```rust
// Future: ~/.odincode/config.toml
[odincode]
model = "glm:glm-4.7"
db_root = "~/.local/share/odincode"

[[tools]]
name = "file_read"
enabled = true

[[tools]]
name = "file_write"
permission = "ask"  # GATED
```

---

## 4. Zod Schema Validation: Runtime Type Safety

### What OpenCode Does

```typescript
import z from "zod"

export const Agent = z.object({
  model: z.string().optional(),
  temperature: z.number().optional(),
  top_p: z.number().optional(),
  prompt: z.string().optional(),
  tools: z.record(z.string(), z.boolean()).optional(),
  disable: z.boolean().optional(),
  description: z.string().optional(),
  mode: z.enum(["subagent", "primary", "all"]).optional(),
  color: z.string().regex(/^#[0-9a-fA-F]{6}$/).optional(),
  maxSteps: z.number().int().positive().optional(),
  permission: z.object({
    edit: Permission.optional(),
    bash: z.union([Permission, z.record(z.string(), Permission)]).optional(),
    // ...
  }).optional(),
}).catchall(z.any())
```

### Applicability to OdinCode

**Verdict**: ✅ **ALREADY HAVE** (Rust's type system)

**What OdinCode has**:
- Compile-time type safety via Rust
- `thiserror` for structured errors
- Serde for serialization

**No action needed** — Rust's type system is stronger than Zod.

---

## 5. Event-Driven Architecture: Loose Coupling

### What OpenCode Does

```typescript
// packages/opencode/src/bus/bus-event.ts
export namespace BusEvent {
  export function define<T extends z.ZodType>(name: string, schema: T) {
    return { name, schema }
  }
}

// Usage
export const Event = {
  Updated: BusEvent.define("permission.updated", Info),
  Replied: BusEvent.define("permission.replied", z.object({...})),
}

// Publish
Bus.publish(Event.Updated, info)

// Subscribe (not shown but pattern exists)
```

### Applicability to OdinCode

**Verdict**: ⚠️ **CONSIDER** (different paradigm)

**Why it's different**:
- OpenCode: Multi-session, real-time UI updates
- OdinCode: Single-session, CLI execution, synchronous

**Potential use case**: Execution logging events
```rust
// Could emit events for monitoring
pub enum ExecutionEvent {
    ToolStarted { tool: String, args: String },
    ToolCompleted { tool: String, success: bool },
    PermissionRequested { tool: String },
}

// But: Direct function calls are simpler for our use case
```

**Recommendation**: Skip for now — adds complexity without clear benefit.

---

## 6. Agent Configuration: File-Based Prompts

### What OpenCode Does

```
.opencode/
├── agent/
│   ├── build.md
│   ├── plan.md
│   ├── explore.md
│   └── compaction.md
├── command/
│   └── review.md
└── tool/
    └── custom.ts
```

**Agent file example**:
```markdown
---
description: Fast agent for codebase exploration
mode: subagent
color: #00FF00
maxSteps: 50
tools:
  edit: false
  write: false
---

You are an exploration specialist. Your role is to:
- Search codebases efficiently
- Find files by pattern
- Answer structural questions
```

### Applicability to OdinCode

**Verdict**: ⚠️ **PARTIAL** (we're substrate, not agent)

**Key distinction**:
- OpenCode: Full AI agent with LLM orchestration
- OdinCode: Tool substrate for LLMs to use

**What applies**:
- File-based tool descriptions (already have `EXTERNAL_TOOLS_API.md`)
- Configuration for tool behavior

**What doesn't apply**:
- Agent prompts (that's the LLM's job)
- Multi-agent dispatch

**Current OdinCode approach**:
```rust
// We provide tools, not agents
// src/execution_tools/mod.rs
// src/llm/adapters/ — separate layer

// Prompt management (Phase 9.9):
// Internal control prompts only, not agent personalities
```

---

## 7. What OpenCode Does That Breaks Our Principles

### ❌ Multi-Session / Parallel Agents

**OpenCode**: Run multiple agents simultaneously
```typescript
// Spawn multiple agents in parallel
for (const agent of ["explore", "general", "security"]) {
  spawnAgent(agent, task)
}
```

**OdinCode principle**: We're a tool substrate, not an orchestrator

**Verdict**: ❌ **OUT OF SCOPE**

### ❌ Terminal UI (TUI) with Animations

**OpenCode**: Rich terminal UI using Ink
```typescript
// Animated spinners, progress bars, real-time updates
<Spinner type="dots" />
<ProgressBar progress={0.6} />
```

**OdinCode principle**: NO EDITOR UI — console tool layer only

**Verdict**: ❌ **OUT OF SCOPE** (Phase 0 constraint)

### ❌ Built-in Agent Orchestration

**OpenCode**: Agent spawns subagents automatically
```typescript
// Agent launches Explore agent for codebase navigation
const exploreAgent = await Agent.spawn("explore")
const result = await exploreAgent.run(query)
```

**OdinCode principle**: NO INTELLIGENCE — we don't make decisions

**Verdict**: ❌ **BREAKS CORE PRINCIPLE**

---

## 8. Actionable Recommendations for OdinCode

### HIGH Priority: Adopt Patterns

1. **Tool Registry with Dynamic Loading** (Phase 10+)
   ```rust
   // Allow user-defined tools in ~/.odincode/tools/
   pub struct ToolRegistry {
       built_in: Vec<ToolMetadata>,
       user_defined: Vec<ToolMetadata>,
   }
   ```

2. **Wildcard Permission Patterns** (Phase 9.2.1)
   ```rust
   // Instead of just tool-level gating
   "bash": {
     "rm *": "ask",
     "git *": "allow",
     "*": "ask",
   }
   ```

3. **Configuration File Support** (Phase 10+)
   ```toml
   # ~/.odincode/config.toml
   [odincode]
   model = "glm:glm-4.7"
   db_root = "~/.local/share/odincode"

   [[tools.permission]]
   tool = "file_write"
   level = "ask"
   ```

### MEDIUM Priority: Consider

4. **File-based Tool Definitions**
   ```
   ~/.odincode/tools/
   ├── custom_read.toml
   └── custom_search.toml
   ```

5. **Environment Variable Expansion**
   ```rust
   // Allow {env:VAR_NAME} in config
   let config = config.replace("{env:HOME}", std::env::var("HOME")?);
   ```

### LOW Priority: Defer

6. **Event Bus for Execution Logging**
   - Current: Direct function calls
   - Proposed: Event emission for monitoring
   - Trade-off: Complexity vs. observability

7. **Plugin System**
   - Current: Static tool compilation
   - Proposed: Dynamic WASM plugins
   - Trade-off: Flexibility vs. "real tools only"

---

## 9. Summary Table

| OpenCode Feature | OdinCode Verdict | Rationale |
|------------------|------------------|-----------|
| Tool Registry | ✅ Adopt | Matches TOOL_WHITELIST approach |
| Permission System (ask/allow/deny) | ✅ Relevant | Similar to GATED tools |
| Wildcard Permission Patterns | ✅ Adopt | "rm *" → Ask is useful |
| Configuration Files (JSONC) | ✅ Consider | Better than hardcoded |
| Zod Validation | ✅ Already Have | Rust types stronger |
| Event Bus | ⚠️ Defer | Adds complexity |
| File-based Agent Config | ⚠️ Partial | We're substrate, not agent |
| Multi-session | ❌ Out of Scope | Different layer |
| TUI with Animations | ❌ Out of Scope | Phase 0 constraint |
| Agent Orchestration | ❌ Breaks Principle | NO INTELLIGENCE |
| MCP Protocol | ⚠️ Consider | Could integrate real tools |

---

## 10. Key Takeaways

### What Makes OpenCode Successful

1. **Modularity**: Each component has clear boundaries
2. **Extensibility**: Users can add tools, agents, commands
3. **Type Safety**: Zod validates everything at runtime
4. **Permission Granularity**: Fine-grained control per tool/pattern
5. **Configuration Flexibility**: Layered loading with file/env refs

### What OdinCode Can Learn

1. **Pattern over Implementation**: The registry pattern is sound; we can adapt it
2. **Permissions as Data**: ask/allow/deny is clearer than just GATED
3. **Configuration as Code**: File-based config with validation beats flags
4. **Tool Self-Description**: Each tool defines its own schema

### What to Reject

1. **UI/TUI focus**: We're a substrate, not a UI
2. **Agent spawning**: That's the LLM's job, not ours
3. **Async complexity**: We stay synchronous and deterministic
4. **Multi-session orchestration**: Different architectural layer

---

*Last Updated: 2025-12-26*
*Analysis based on: OpenCode source code at /home/feanor/Projects/opencode, https://opencode.ai/, OdinCode CONTRACT.md*

---

## 11. UPDATE: Tool Result Compaction (NEW - 2025-12-27)

**User Request**: Keep tool call/results spam out of context, enforce LLM to use correct workflow.

### What OpenCode Does

**File**: `packages/opencode/src/session/compaction.ts`

#### Prune Function (Lines 48-88)

```typescript
export async function prune(input: { sessionID: string }) {
    const msgs = await Session.messages({ sessionID: input.sessionID })
    let total = 0
    let pruned = 0

    loop: for (let msgIndex = msgs.length - 1; msgIndex >= 0; msgIndex--) {
        const msg = msgs[msgIndex]
        if (msg.info.role === "user") turns++
        if (turns < 2) continue  // Protect last 2 turns

        for (let partIndex = msg.parts.length - 1; partIndex >= 0; partIndex--) {
            const part = msg.parts[partIndex]
            if (part.type === "tool" && part.state.status === "completed") {
                if (PRUNE_PROTECTED_TOOLS.includes(part.tool)) continue  // "skill" protected

                const estimate = Token.estimate(part.state.output)
                total += estimate
                if (total > PRUNE_PROTECT) {  // 40,000 tokens
                    pruned += estimate
                    toPrune.push(part)
                }
            }
        }
    }

    if (pruned > PRUNE_MINIMUM) {  // 20,000 tokens
        for (const part of toPrune) {
            part.state.time.compacted = Date.now()  // MARK as compacted
        }
    }
}
```

#### Compaction Display (message-v2.ts:517)

```typescript
output: part.state.time.compacted
    ? "[Old tool result content cleared]"
    : part.state.output
```

**When tool results are compacted, they show `[Old tool result content cleared]` instead of full output!**

---

### LSP Auto-Check After Edit

**File**: `packages/opencode/src/tool/edit.ts`

**Lines 141-151**: After EVERY edit, LSP diagnostics are automatically run:

```typescript
await LSP.touchFile(filePath, true)
const diagnostics = await LSP.diagnostics()
const issues = diagnostics[normalizedFilePath] ?? []
if (issues.length > 0) {
    output += `\nThis file has errors, please fix\n<file_diagnostics>...`
}
```

**The edit tool itself returns errors**, forcing the LLM to see and fix them.

---

### Summary Generation

**File**: `packages/opencode/src/session/compaction.ts`

**Lines 90-191**: `process()` function creates an LLM-generated summary:

```typescript
const defaultPrompt =
    "Provide a detailed prompt for continuing our conversation above.
    Focus on information that would be helpful for continuing the conversation,
    including what we did, what we're doing, which files we're working on,
    and what we're going to do next..."
```

**This summary REPLACES all the compacted tool results**, preserving context without spam.

---

### Comparison: OdinCode vs OpenCode

| Feature | OpenCode | OdinCode (Current) |
|---------|----------|-------------------|
| Tool result compaction | ✅ Prunes old results → `[cleared]` | ❌ Full history in context |
| LSP after edit | ✅ Auto-run in edit tool | ❌ Has lsp_check (manual only) |
| Workflow enforcement | ✅ Tool errors if not read first | ❌ Only prompt-based |
| Mode distinction | ✅ Plan vs Build modes | ❌ Explore/Query/Mutation modes |
| Summary generation | ✅ LLM compacts old context | ❌ No compaction mechanism |

---

### What the User Wants

1. **Keep tool call/results spam out of context**
   - OpenCode: Compacts old results → `[Old tool result content cleared]`
   - OpenCode: Generates summary to replace compacted content

2. **Enforce workflow: magellan → splice → LSP → retry**
   - OpenCode: Edit tool auto-runs LSP and returns errors
   - Missing: Magellan check BEFORE edit (what symbols will be affected?)
   - Missing: Auto-retry loop until LSP passes

3. **Store everything in memory so LLM must check**
   - OpenCode: Has compaction + summary
   - Missing: Forced memory_query before edits

---

### Recommendations for OdinCode

1. **Add compaction to FrameStack**:
   ```rust
   pub enum Frame {
       User(String),
       Assistant(String),
       ToolResult {
           tool: String,
           success: bool,
           output: String,
           compacted: bool,  // NEW: Mark old results as compacted
       },
       Summary(String),  // NEW: LLM-generated summary
   }
   ```

2. **In build_messages(), replace compacted output**:
   ```rust
   match frame {
       Frame::ToolResult { compacted: true, .. } => {
           "[Old tool result cleared - use memory_query for details]"
       }
       // ...
   }
   ```

3. **Auto-run LSP in splice_patch**:
   - After applying patch, run `lsp_check` on affected file
   - Return LSP errors in tool output
   - Don't return success until LSP passes

4. **Pre-edit checks enforced in tools**:
   - `splice_patch` checks if symbol was looked up via magellan first
   - Error if `symbols_in_file` not called for the symbol being patched

5. **Summary Frame**:
   - Generate LLM summary to replace compacted tool results
   - "What did we do? What are we working on? What's next?"
