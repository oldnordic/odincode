# Claude Code System Prompts: Engineering Analysis

**Source**: [Piebald-AI/claude-code-system-prompts](https://github.com/Piebald-AI/claude-code-system-prompts)
**Version Analyzed**: Claude Code v2.0.76 (December 22, 2025)
**Documented**: 2025-12-26
**Context**: OdinCode internal tool architecture

---

## Executive Summary

In December 2024, the complete system prompts for Claude Code were leaked/exposed via the Piebald-AI repository. This repository contains **40+ modular prompt fragments** totaling approximately **15,000-20,000 tokens** that dynamically compose Claude Code's behavior.

This is a **production-grade prompt architecture** that demonstrates how Anthropic builds agentic systems. The patterns apply universally—not just to Claude Code, but to any tool-using AI system including OdinCode's internal tools (`file_read`, `splice_patch`, `memory_query`, `lsp_check`, etc.).

**Key Insight**: Claude Code doesn't have ONE system prompt. It has **40+ fragments** that are:
- Conditionally assembled based on mode/context
- Token-budgeted and tracked
- Versioned independently
- Purpose-built for specific responsibilities

---

## Architecture: Four Layers

### Layer 1: Main System Prompt (~2,981 tokens)

The "constitution" that defines core behavior:

```markdown
You are an interactive CLI tool that helps users with software engineering tasks.

# Tone and style
- Only use emojis if the user explicitly requests it
- Short and concise responses for CLI
- Output text to communicate; never use tools for communication
- NEVER create files unless absolutely necessary
- ALWAYS prefer editing existing files to creating new ones

# Professional objectivity
Prioritize technical accuracy over validation. Avoid excessive praise.
Focus on facts and problem-solving.

# Planning without timelines
Provide concrete steps without time estimates. Never say "this will take 2-3 weeks."
```

**Key patterns for OdinCode**:
1. **Tone constraints**: Emoji ban, concise CLI output, no unnecessary file creation
2. **Objectivity over sycophancy**: Don't validate false beliefs; disagree when necessary
3. **No timelines**: Focus on WHAT, not WHEN
4. **Edit over create**: Prefer modifying existing files

### Layer 2: Agent Prompts (~3,000+ tokens total)

Specialized sub-agents with distinct behaviors:

| Agent | Tokens | Purpose | Key Constraint |
|-------|--------|---------|----------------|
| **Explore** | 516 | Codebase navigation | READ-ONLY: no Write/Edit/Create |
| **Plan** | 633 | Implementation design | READ-ONLY: output plans, no execution |
| **Task** | 294 | General subagent spawning | Specialized agent dispatch |
| **Security Review** | 2,610 | Vulnerability analysis | Focus on exploitable flaws |
| **Conversation Summary** | 1,121 | Context compaction | Preserve technical details |
| **User Sentiment** | 205 | Frustration detection | Detect user anger |

**Explore Agent Pattern** (applicable to OdinCode):
```markdown
=== CRITICAL: READ-ONLY MODE - NO FILE MODIFICATIONS ===
STRICTLY PROHIBITED from:
- Creating new files
- Modifying existing files
- Deleting files
- Using redirect operators (>, >>, |)
- Running commands that change system state

Your role is EXCLUSIVELY to search and analyze existing code.
```

**Decision for OdinCode**: Our PromptMode system already implements similar constraints:
- Query Mode → aggregate tools only
- Explore Mode → read/query tools, bounded search
- Mutation Mode → edit tools with validation
- Presentation Mode → no tools

### Layer 3: Tool Descriptions (~8,000+ tokens total)

Each tool has its own "mini system prompt":

| Tool | Tokens | Notable Features |
|------|--------|------------------|
| **TodoWrite** | 2,167 | Extensive examples, when/NOT to use |
| **Bash** | 1,074 | Git workflow, quoting rules, security notes |
| **Task** | 1,214 | Subagent dispatch guidance |
| **ReadFile** | 439 | File reading patterns |
| **Edit** | 278 | Must read before edit requirement |

**Bash Tool Pattern** (applies to our `bash_exec`):
```markdown
IMPORTANT: This tool is for terminal operations (git, npm, docker).
DO NOT use it for file operations — use specialized tools.

Before executing:
1. Directory Verification: ls before mkdir
2. Quote file paths with spaces
3. Use '&&' for sequential commands
4. Run independent commands in parallel (single message)

Avoid: find, grep, cat, head, tail, sed, awk, echo
Use specialized tools instead.
```

**TodoWrite Tool Pattern** (inspiring for task management):
```markdown
## When to Use This Tool
1. Complex multi-step tasks (3+ steps)
2. User provides multiple tasks
3. After receiving new instructions
4. Mark in_progress BEFORE starting
5. Mark completed IMMEDIATELY after finishing

## When NOT to Use
1. Single, straightforward task
2. Trivial task (< 3 steps)
3. Purely conversational
4. Informational request

## Task States
- pending: Not started
- in_progress: Exactly ONE at a time
- completed: FULLY accomplished

IMPORTANT: Task descriptions need TWO forms:
- content: "Run tests" (imperative)
- activeForm: "Running tests" (present continuous)
```

**Decision for OdinCode**: Adopt TodoWrite's two-form pattern for our task tracking if/when we implement it.

### Layer 4: System Reminders (~1,800+ tokens total)

Mode-specific overrides injected into context:

| Reminder | Tokens | Purpose |
|----------|--------|---------|
| **Plan Mode Active** | 1,211 | Multi-phase planning workflow |
| **Plan Mode Re-Entry** | 236 | Resume previous planning session |
| **Plan Mode for Subagents** | 310 | Simplified planning for agents |

**Plan Mode Pattern** (5-phase workflow):
```markdown
### Phase 1: Initial Understanding
- Launch up to 3 Explore agents IN PARALLEL
- Use 1 agent for isolated tasks
- Use multiple agents for uncertain scope

### Phase 2: Design
- Launch Plan agent(s) for implementation approach
- Default: 1 agent for most tasks
- Multiple agents for complex refactors

### Phase 3: Review
- Read critical files
- Align with user intent
- Ask clarifying questions

### Phase 4: Final Plan
- Write to plan file (only editable file)
- Concise but actionable

### Phase 5: Call ExitPlanMode
- Always call when done planning
- End turn with question or ExitPlanMode
```

---

## Key Patterns for OdinCode

### 1. Modular Prompt Composition

Instead of one giant prompt, compose from fragments:

```rust
// Pseudo-code for OdinCode
struct PromptComposer {
    base: String,              // Main system prompt (~3000 tokens)
    mode: Option<PromptMode>,   // Query/Explore/Mutation/Presentation
    tools: Vec<ToolMetadata>,   // Tool descriptions (~8000 tokens total)
    agents: Vec<AgentPrompt>,   // Sub-agent prompts (~3000 tokens)
    reminders: Vec<Reminder>,   // Mode-specific overrides (~2000 tokens)
}

impl PromptComposer {
    fn build(&self, mode: PromptMode) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}",
            self.base,
            self.mode_prompt(mode),
            self.tool_prompt(mode.allowed_tools()),
            self.agent_prompt(),
            self.reminder_prompt(mode)
        )
    }
}
```

### 2. READ-ONLY Enforcement

Both Explore and Plan agents are **strictly prohibited** from modifications:

**For OdinCode Explore/Query modes**:
```markdown
=== CRITICAL: READ-ONLY MODE ===
PROHIBITED:
- splice_patch, splice_plan
- file_write, file_create, file_edit
- git_commit, git_push
- npm install, pip install
- Any command with --force or --yes

ALLOWED:
- file_search, file_glob
- symbols_in_file, references_to_symbol_name
- lsp_check (read-only diagnostics)
- git log, git diff, git status
```

### 3. Tool Governance Through Examples

TodoWrite (2,167 tokens!) uses extensive examples to teach behavior:

**Pattern**: Show when to use AND when NOT to use

```markdown
## When NOT to Use TodoList

Example: User asks "How do I print Hello World?"
Reasoning: Single trivial task, one step, no tracking needed

Example: User asks "What does git status do?"
Reasoning: Informational, no coding task
```

**For OdinCode**: Each tool should have:
- 3-5 examples of proper usage
- 2-3 examples of when NOT to use
- Clear reasoning for each

### 4. Conversation Summarization Strategy

The summarization prompt (1,121 tokens) specifies exact structure:

```markdown
<analysis>
[Chronological analysis of conversation]
</analysis>

<summary>
1. Primary Request and Intent:
   [Detailed description]

2. Key Technical Concepts:
   - [Concept 1]
   - [Concept 2]

3. Files and Code Sections:
   - [File Name]
     - Why important
     - Changes made
     - Code snippets

4. Errors and fixes:
   - [Error]: How fixed

5. Problem Solving:
   [Solved problems]

6. All user messages:
   - [Message 1]
   - [Message 2]

7. Pending Tasks:
   - [Task 1]

8. Current Work:
   [What was being worked on]

9. Optional Next Step:
   [Direct quote from recent work]
</summary>
```

**For OdinCode**: Our FrameStack already handles conversation history. This structure could enhance our compaction strategy.

### 5. User Sentiment Analysis

Lightweight agent (205 tokens) that detects:
1. User frustration (repeated corrections, negative language)
2. PR creation requests (explicit "send/submit/push PR" language)

```markdown
Output:
<frustrated>true/false</frustrated>
<pr_request>true/false</pr_request>
```

**For OdinCode**: Could add similar lightweight analysis to detect when user is stuck and needs human intervention.

---

## Token Budget Breakdown

| Component | Tokens | % of Total |
|-----------|--------|------------|
| Main System Prompt | 2,981 | ~15% |
| Agent Prompts | ~3,000 | ~15% |
| Tool Descriptions | ~8,000 | ~40% |
| System Reminders | ~1,800 | ~9% |
| Utilities (summary, etc.) | ~4,000 | ~20% |
| **Total** | **~20,000** | **100%** |

**Key insight**: 40% of tokens are tool descriptions. This validates our earlier decision (from ENGINEERING_DECISIONS.md) to implement progressive tool discovery.

---

## Security Patterns

### Bash Tool Security

```markdown
IMPORTANT: You must NEVER generate or guess URLs for the user
unless you are confident that the URLs are for helping the user with programming.

- Be careful not to introduce security vulnerabilities
- If you notice insecure code, immediately fix it
- Avoid OWASP Top 10: injection, XSS, etc.
```

### Command Injection Prevention

Separate agent (835 tokens) for detecting:
- Command prefixes
- Command injection attempts
- Dangerous flag combinations (`--force`, `--yes`)

### Git Commit Rules

Separate instruction (1,615 tokens) covering:
- Conventional commit format
- Branch naming conventions
- PR description templates
- NEVER use `--no-verify` to skip hooks

---

## Phase-Specific Behaviors

### Plan Mode Workflow

The most sophisticated reminder (1,211 tokens) defines a **5-phase process**:

1. **Understanding**: Explore agents in parallel
2. **Design**: Plan agents for implementation
3. **Review**: Critical files, user alignment
4. **Final Plan**: Write to plan file
5. **Exit**: Call ExitPlanMode tool

**Critical insight**: Plan mode explicitly forbids ALL edits except the plan file itself. This prevents premature execution during planning.

### Explore Agent Constraints

```markdown
NOTE: You are meant to be a FAST agent.
- Make efficient use of tools
- Spawn multiple parallel tool calls for grepping/reading
- Return output quickly
```

This is the **"thoroughness" parameter** pattern we should adopt for our magellan queries.

---

## Implementation Roadmap for OdinCode

### Phase 1: Adopt Patterns (Immediate)

1. **Two-form task descriptions**
   ```rust
   pub struct Task {
       pub content: String,      // "Run tests"
       pub active_form: String,  // "Running tests"
   }
   ```

2. **READ-ONLY enforcement for Explore/Query modes**
   - Already partially implemented via PromptMode
   - Add explicit "CRITICAL: READ-ONLY" sections to internal prompts

3. **Edit-over-create preference in system prompt**
   - Add to main system prompt
   - "NEVER create files unless absolutely necessary"

### Phase 2: Expand Examples (Short-term)

1. **Tool usage examples** for each tool
   - 3-5 proper usage examples
   - 2-3 "when NOT to use" examples
   - Reasoning for each

2. **Security guidance** in tool descriptions
   - Command injection warnings
   - OWASP Top 10 reminders
   - Git commit best practices

### Phase 3: Utility Agents (Medium-term)

1. **Conversation summarization**
   - Enhance FrameStack compaction
   - Use structured summary format

2. **User sentiment detection**
   - Lightweight frustration detection
   - Trigger for human intervention

3. **Bash output summarization**
   - Detect when output needs summarization
   - Filter noise from signal

### Phase 4: Advanced Planning (Long-term)

1. **Multi-phase planning workflow**
   - Explore → Design → Review → Final Plan → Execute
   - Parallel agent spawning
   - Critical files identification

2. **Sub-agent architecture**
   - Explore agent (read-only codebase navigation)
   - Plan agent (implementation design)
   - Security review agent (vulnerability analysis)

---

## Key Takeaways

1. **Prompts are modular, not monolithic**
   - 40+ fragments, not one giant string
   - Each fragment has single responsibility
   - Versioned independently

2. **READ-ONLY modes are strictly enforced**
   - Explore/Plan agents cannot modify files
   - Explicit prohibitions, not soft suggestions
   - Tool access filtered by mode

3. **Examples teach behavior better than rules**
   - TodoWrite: 2,167 tokens of examples
   - Show when AND when not to use
   - Include reasoning

4. **Planning is separate from execution**
   - 5-phase workflow
   - Only plan file is editable during Plan mode
   - Explicit ExitPlanMode transition

5. **Token budgets are intentional**
   - Tool descriptions: 40% (prime target for optimization)
   - Each prompt tracked and versioned
   - Progressive loading for efficiency

---

## References

- [Piebald-AI/claude-code-system-prompts](https://github.com/Piebald-AI/claude-code-system-prompts)
- [tweakcc](https://github.com/Piebald-AI/tweakcc) - Prompt customization tool
- Claude Code v2.0.76 (December 22, 2025)
- OdinCode Phase 9.9: Internal Control Prompts
- docs/ENGINEERING_DECISIONS.md: Advanced tool orchestration patterns
