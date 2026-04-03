# J-CLI Architecture - Quick Reference Card

## 1️⃣ Compact Summary Prompt Focus

**File**: `src/command/chat/compact.rs:186-192`

The LLM is told to preserve:
```
1) What was accomplished
2) Current state
3) Key decisions made
4) If a skill/workflow was actively followed:
   - Key steps of workflow
   - Current progress within it
   - Instructions to continue
```

**Key Instruction**: *"If a skill/workflow was actively being followed, preserve its key steps and current progress so the model can continue following it."*

---

## 2️⃣ Agent Message Construction

**File**: `src/command/chat/agent.rs:20-492`

### What Gets Sent to LLM

```
CreateChatCompletionRequest {
  system: "[Resolved System Prompt]",  // Static per session
  messages: [
    { role: "user", content: "..." },           // User input
    { role: "assistant", content: "..." },      // Model response
    { role: "tool", content: "[Previous: used {tool_name}]" },  // Compacted
    { role: "tool", content: "..." },           // Recent results (kept)
    // ... background notifications injected as user messages
    // ... todo reminders after 15+ rounds
  ],
  tools: [/* available tools */],
}
```

### Message Injection Points

| Where | What | When | File:Lines |
|-------|------|------|-----------|
| **Pending User Messages** | User appended during loop | Every iteration | 41-42, 495-509 |
| **Background Tasks** | Task completion notifications | When task completes | 60-79 |
| **Todo Reminder** | Current todos in system-reminder | After 15+ rounds | 81-99 |
| **PreLlmRequest Hook** | Custom message injection | Before each LLM call | 152-179 |
| **PostToolExecution Hook** | Modify tool result | After tool runs | 666-681 |

### Important: What's NOT There
- ❌ Current active task
- ❌ Task progress tracking
- ❌ Intent signals beyond todo list
- ❌ Workflow metadata

**Why?** Task system is SEPARATE (Task tool only). System prompt stays clean.

---

## 3️⃣ System Prompt Construction

**File**: `src/command/chat/app.rs:2920-2948`

### Runtime Resolution (4 Steps)

```
┌─ Step 1: Load Template ─────────────────────┐
│ From: ~/.jdata/agent/data/system_prompt.md  │
│ Or:   assets/system_prompt_default.md       │
└─────────────────────────────────────────────┘
                    ↓
┌─ Step 2a: Build Dynamic Summaries ──────────┐
│ skills_summary = list of available skills   │
│ commands_summary = list of available cmds   │
│ tools_summary = list of available tools     │
└─────────────────────────────────────────────┘
                    ↓
┌─ Step 2b: Load Persistent Data ────────────┐
│ style_text = from memory/data/style.md     │
│ memory_text = from memory/data/memory.md   │
│ soul_text = from memory/data/soul.md       │
│ current_dir = os::current_dir()            │
└─────────────────────────────────────────────┘
                    ↓
┌─ Step 3: String Substitution ──────────────┐
│ {{.current_dir}}  → current_dir            │
│ {{.tools}}        → tools_summary          │
│ {{.skills}}       → skills_summary         │
│ {{.commands}}     → commands_summary       │
│ {{.memory}}       → memory_text            │
│ {{.soul}}         → soul_text              │
│ {{.style}}        → style_text             │
│ {{.skill_dir}}    → paths                  │
│ {{.project_skill_dir}} → paths             │
└─────────────────────────────────────────────┘
                    ↓
┌─ Step 4: Inject into LLM Request ──────────┐
│ ChatCompletionRequestSystemMessage {        │
│   content: resolved_prompt                 │
│ }                                          │
└─────────────────────────────────────────────┘
```

### System Prompt Lifecycle

| When | What | Where | Notes |
|------|------|-------|-------|
| **First LLM request** | Resolve once | Background thread | `system_prompt_fn()` closure |
| **Subsequent requests** | Reuse same | In-memory | Static for entire session |
| **On compaction** | NOT re-resolved | N/A | Compact summary is separate |
| **Session end** | Discarded | N/A | New session = new resolution |

---

## 4️⃣ Three-Layer Compaction

**File**: `src/command/chat/compact.rs`

### Layer 1: Micro Compact (FREE)
- **When**: Every agent loop iteration (line 47 in agent.rs)
- **Cost**: O(n), zero API calls
- **What**: Replace old tool results (>800 bytes) with `[Previous: used {tool}]`
- **Keep**: Recent 10 tool results (configurable)
- **Exempt Tools**: LoadSkill, Task, TodoWrite, TodoRead, Ask

### Layer 2: Auto Compact (ONE LLM CALL)
- **When**: `token_count > 204,800` (default threshold)
- **Cost**: ~$0.25 per compaction
- **What**: LLM summarizes entire conversation
- **Saves**: Usually 100K-200K+ tokens (ROI positive)
- **Result**: Replace all messages with `[compressed] + [ack]`
- **Preserves**: Workflow steps, progress, decisions (explicit instruction)

### Layer 3: Explicit Compact (USER/AGENT TRIGGERED)
- **How**: Call `Compact` tool with optional `focus` parameter
- **Triggers**: Layer 2 (auto_compact)
- **When**: User decides conversation is too long

### Exempt Tools Philosophy

These tools are preserved even if large because they carry **state/instructions**:

| Tool | Reason |
|------|--------|
| **LoadSkill** | Workflow instructions for continuation |
| **Task** | Task definitions and tracking state |
| **TodoWrite** | Todo state management |
| **TodoRead** | Todo state management |
| **Ask** | Dialog context for decisions |

---

## 5️⃣ Compaction Prompt (Most Important!)

**The exact instruction** (compact.rs:186-192):

```
"Summarize this conversation for continuity. Include:
 1) What was accomplished,
 2) Current state,
 3) Key decisions made.
 4) If a skill/workflow was actively being followed, preserve its 
    key steps and current progress so the model can continue 
    following it.
 Be concise but preserve critical details."
```

**This is CRITICAL** because it ensures:
- Workflow continuity across compression
- Multi-step task preservation
- Progress recovery after summarization
- No loss of workflow instructions

---

## 6️⃣ Configuration

**File**: `~/.jdata/agent/config.yaml`

```yaml
[compact]
enabled = true              # Enable/disable all compaction
token_threshold = 204800    # Trigger auto_compact at this token count
keep_recent = 10            # Keep this many recent tool results in micro_compact
```

### Default Thresholds

| Setting | Default | Meaning |
|---------|---------|---------|
| `token_threshold` | 204,800 | Auto-compact at ~200K tokens |
| `keep_recent` | 10 | Keep last 10 tool results |
| `MICRO_COMPACT_BYTES` | 800 | Replace if tool result > 800 bytes |

---

## 7️⃣ Debugging Commands

### Check if Compaction Happened
```bash
tail -f ~/.jdata/logs/agent_loop.log | grep -E "compact|Transcript"
```

### View Current System Prompt
```bash
cat ~/.jdata/agent/data/system_prompt.md | head -50
```

### Find All Transcripts
```bash
ls -lh ~/.jdata/agent/transcripts/
```

### Monitor Agent Loop
```bash
tail -f ~/.jdata/logs/agent_loop.log | grep -E "Round|compact|tool"
```

---

## 8️⃣ Code Map for Key Operations

| Operation | Main File | Lines | Helper Files |
|-----------|-----------|-------|--------------|
| **Resolve system prompt** | app.rs | 2920-2948 | storage.rs:351+ |
| **Build LLM request** | api.rs | 117-149 | agent.rs:181-192 |
| **Run agent loop** | agent.rs | 20-492 | compact.rs, hooks.rs |
| **Micro compact** | compact.rs | 61-127 | agent.rs:44-56 |
| **Auto compact** | compact.rs | 174-246 | agent.rs:48-56 |
| **Trigger compact** | tools/compact.rs | 1-45 | agent.rs:606 |
| **Message injection** | agent.rs | 41-99 | (inline) |
| **Hook execution** | agent.rs | 152-179, 666-681 | hook.rs |

---

## 9️⃣ Design Philosophy Summary

```
┌─ WORKFLOW PRESERVATION ─────────────────┐
│ Compaction explicitly preserves workflow │
│ steps and progress for continuity       │
└─────────────────────────────────────────┘

┌─ GRACEFUL DEGRADATION ──────────────────┐
│ Failures don't crash: auto_compact fails │
│ → continue with original messages        │
└─────────────────────────────────────────┘

┌─ LAZY BACKGROUND RESOLUTION ────────────┐
│ System prompt resolved in background    │
│ thread → doesn't block UI                │
└─────────────────────────────────────────┘

┌─ MULTI-LAYER OPTIMIZATION ─────────────┐
│ Layer 1: Free (micro)                  │
│ Layer 2: Powerful (LLM)                │
│ Layer 3: User control (explicit)       │
└─────────────────────────────────────────┘

┌─ NO BUILT-IN TASK TRACKING ─────────────┐
│ System prompt stays clean               │
│ Task system is SEPARATE mechanism       │
│ Prevents hallucination                  │
└─────────────────────────────────────────┘
```

---

## 🔟 One-Minute Summary

**System Prompt**: Built once per session from template + runtime data. Static after first request.

**Messages to LLM**: System prompt + history + compacted tool results + injected notifications.

**Compaction Strategy**: 
- Layer 1 (Free): Replace old tool results with placeholders
- Layer 2 (Powerful): LLM summarizes when exceeding token threshold  
- Layer 3 (Manual): User/agent can trigger explicitly

**Key Design**: Workflow preservation is CRITICAL - compaction prompt explicitly preserves skill/workflow steps and progress.

**No Task Tracking in Messages**: Task system is separate. System prompt stays clean for better results.

---

**📚 For details, see**: ANALYSIS-INDEX.md
**🏃 For quick ref**: This file
**📊 For flow diagram**: SYSTEM-PROMPT-FLOW.md
**🔍 For deep dive**: COMPACTION-DETAILED.md

