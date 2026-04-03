# J-CLI Codebase Analysis - Complete Index

This directory contains comprehensive analysis of the j-cli chat system architecture, focusing on:
1. Conversation compaction strategies
2. Agent message construction and LLM communication
3. System prompt building and runtime resolution

## Documents

### 1. **J-CLI-ANALYSIS.md** - Main Overview
**Best for**: Getting the big picture

Contains:
- How compact.rs builds the compact summary prompt
- Key focus areas after compaction (accomplished, state, decisions, workflow)
- How agent.rs constructs messages for the LLM
- System prompt construction flow
- Summary table of all components

**Key Finding**: The compaction summary explicitly preserves active workflow steps and current progress for continuity.

### 2. **SYSTEM-PROMPT-FLOW.md** - Visual System Prompt Flow
**Best for**: Understanding the runtime resolution process

Contains:
- ASCII flow diagram of system prompt construction
- Step-by-step template resolution process
- Placeholder substitution reference table
- File dependencies
- When/when not system prompt is updated

**Key Sections**:
- Step 1: Load Template
- Step 2a: Build Dynamic Summaries (skills, tools, commands)
- Step 2b: Load Persistent Data (style, memory, soul)
- Step 3: String Substitution
- Step 4: Return to Agent Loop

### 3. **COMPACTION-DETAILED.md** - Complete Compaction System
**Best for**: Understanding conversation context management

Contains:
- Layer 1: Micro Compact (in-memory, zero cost)
  - Algorithm and exempt tools philosophy
  - Before/after example
- Layer 2: Auto Compact (LLM summarization)
  - Trigger conditions and process steps
  - Workflow preservation instructions
  - Performance and graceful degradation
- Layer 3: Explicit Compact Tool
  - Tool definition and parameters
  - When to use
- Complete flow diagram
- Configuration options

**Key Finding**: Workflow preservation is explicitly instructed in the compact prompt to maintain multi-step task continuity.

---

## Quick Reference

### System Prompt Building (What Does It Include?)

The system prompt template (`assets/system_prompt_default.md`) contains:

1. **Role Definition**
   - "You are a highly skilled software engineer"
   - "Solve tasks by reading, searching, and editing code using tools"

2. **Context**
   - Current working directory
   - System reminder tag explanation

3. **Working Principles**
   - Rigorous and meticulous
   - Use tools to perceive environment
   - Use Task tool for multi-step tasks ← **Important**
   - Use Ask tool for clarification

4. **Tool Usage Rules**
   - Specific tool for each job (Glob, Grep, Read, Edit, Write)
   - Best practices (parallel execution, etc.)
   - Git safety guidelines

5. **Skill System**
   - Points to skill assets
   - Lists available skills to load

6. **Dynamic Content** (substituted at runtime)
   - `{{.tools}}` → All available tool definitions
   - `{{.skills}}` → All available skills summary
   - `{{.commands}}` → All available commands
   - `{{.current_dir}}` → Working directory
   - `{{.memory}}` → Persistent user information
   - `{{.soul}}` → User personality/instructions
   - `{{.style}}` → Response style preferences

### Agent Message Construction (What Gets Sent to LLM?)

Each LLM request includes:
1. **System Message** - The resolved system prompt (static per session)
2. **Message History** - Previous user/assistant/tool messages
3. **Compacted Tool Results** - Old results replaced with `[Previous: used {tool}]`
4. **Injected Notifications** - Background tasks, todo reminders (via hooks)
5. **Tool Definitions** - Available tools

**Important**: NO current task tracking in messages! Task system is separate.

### Compact Summary (What Gets Preserved?)

When conversation is compressed, the LLM is instructed to include:
1. **What was accomplished** - Completed work and results
2. **Current state** - Current situation and environment
3. **Key decisions made** - Reasoning that led to current state
4. **Active skill/workflow** - ← **CRITICAL**
   - Key steps of the workflow
   - Current progress within it
   - Instructions to continue

---

## Code Locations

### Compact System
| Component | File | Lines |
|-----------|------|-------|
| Micro Compact | `src/command/chat/compact.rs` | 61-127 |
| Auto Compact | `src/command/chat/compact.rs` | 174-246 |
| Compact Tool | `src/command/chat/tools/compact.rs` | 1-45 |
| Agent Loop (compaction calls) | `src/command/chat/agent.rs` | 44-56, 316-320, 399-403, 449-453 |

### System Prompt
| Component | File | Lines |
|-----------|------|-------|
| System Prompt Resolution | `src/command/chat/app.rs` | 2920-2948 |
| System Prompt Template | `assets/system_prompt_default.md` | All |
| System Prompt Loading | `src/command/chat/storage.rs` | 351+ |
| Default Template Assets | `src/assets.rs` | 59-62 |

### Message Construction
| Component | File | Lines |
|-----------|------|-------|
| Agent Loop (main) | `src/command/chat/agent.rs` | 20-492 |
| Build Request | `src/command/chat/api.rs` | 117-149 |
| Message Injection | `src/command/chat/agent.rs` | 41-99 |
| Hook System | `src/command/chat/agent.rs` | 152-179, 666-681 |

---

## Key Design Decisions

### 1. Three-Layer Compaction Strategy
- **Layer 1**: Micro (fast, local)
- **Layer 2**: Auto (when needed, costs ~$0.25)
- **Layer 3**: Explicit (user control)

**Why?** Provides flexibility: free optimization most of the time, powerful LLM summarization when needed.

### 2. Workflow Preservation as First-Class Concern
- Explicit instruction in compact prompt
- Exempt tools preserved (LoadSkill, Task, etc.)
- Designed for long multi-step workflows

**Why?** Users may spend hours following a skill/workflow; losing progress is unacceptable.

### 3. No Built-In Task Tracking in LLM Messages
- System prompt doesn't include "currently working on X"
- Task tool is separate from message system
- Micromanagement via hooks and tool results

**Why?** Keeps prompt clean, task tracking via explicit Task tool, avoids hallucination.

### 4. Graceful Degradation Everywhere
- Auto compact fails? Continue with originals
- Micro compact always succeeds (no I/O)
- System prompt loading optional

**Why?** Robustness: tool failures shouldn't crash the conversation.

### 5. Lazy System Prompt Resolution
- Built in background thread as closure
- Only resolved once per session
- Static after first LLM request

**Why?** Performance: no blocking on UI thread, no re-resolving after every message.

---

## Testing the System

### Check Compaction Happening
```bash
# Watch logs
tail -f ~/.jdata/logs/agent_loop.log

# Look for:
grep "micro_compact" ~/.jdata/logs/agent_loop.log
grep "auto_compact" ~/.jdata/logs/agent_loop.log
grep "Transcript saved" ~/.jdata/logs/agent_loop.log
```

### Check System Prompt
```bash
# View current system prompt
cat ~/.jdata/agent/data/system_prompt.md

# View what gets substituted
grep "{{" ~/.jdata/agent/data/system_prompt.md
```

### Manual Compaction
```bash
# During chat session, user can trigger:
/Compact

# Or agent detects and calls autonomously when:
# - Token count > 204,800 (default)
# - User explicitly calls Compact tool
```

### Debug Configuration
```bash
# Edit config to control thresholds
~/.jdata/agent/config.yaml

[compact]
enabled = true
token_threshold = 204800  # Adjust to trigger compaction sooner/later
keep_recent = 10          # Adjust to keep more/fewer recent tool results
```

---

## Summary: How It All Works Together

1. **User starts chat** (`j chat`)
2. **System Prompt** is resolved in background:
   - Load template
   - Build skills/tools/commands summaries
   - Load memory/soul/style
   - Substitute all placeholders
3. **Agent Loop** starts sending requests to LLM:
   - Each turn: check token count, run micro_compact if needed
   - If threshold exceeded: auto_compact (summarize conversation)
   - Inject any pending messages, background notifications, or reminders
   - Build request with system prompt + messages + tools
   - Send to API
   - Process response (tools or text)
4. **Long Workflows** are preserved because:
   - Compact prompt explicitly preserves workflow steps
   - Exempt tools keep critical state (LoadSkill, Task, etc.)
   - After compaction: model acknowledges context and continues
5. **User can manually** trigger compaction with Compact tool when needed

---

## Related Files for Deep Dives

- **Skill System**: See `src/command/chat/skill.rs`
- **Task Tool**: See `src/command/chat/tools/task/`
- **Todo System**: See `src/command/chat/tools/todo/`
- **Hook System**: See `src/command/chat/hook.rs`
- **Tool Registry**: See `src/command/chat/tools/mod.rs`
- **Storage/Loading**: See `src/command/chat/storage.rs`

