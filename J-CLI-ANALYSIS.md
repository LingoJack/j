# J-CLI Codebase Analysis: Compact, Agent & System Prompt

## 1. Compact Summary Prompt Construction (`compact.rs`)

### Location
`src/command/chat/compact.rs` (lines 186-192)

### How It Builds the Compact Summary Prompt

The `auto_compact()` function constructs a **single-turn LLM request** to summarize the conversation:

```rust
let summary_prompt = format!(
    "Summarize this conversation for continuity. Include: \
     1) What was accomplished, 2) Current state, 3) Key decisions made. \
     4) If a skill/workflow was actively being followed, preserve its key steps and current progress so the model can continue following it. \
     Be concise but preserve critical details.\n\n{}",
    truncated
);
```

### Key Focus Areas After Compaction

The compact summary explicitly tells the AI to preserve:

1. **What was accomplished** - Completed work and results
2. **Current state** - The current situation and environment
3. **Key decisions made** - Reasoning and choices that led to current state
4. **Active skill/workflow preservation** - **CRITICAL**: If a skill or workflow was being actively followed, the summary must preserve:
   - Key steps of the workflow
   - Current progress within that workflow
   - Instructions needed to continue following it

### Compaction Workflow (3 Layers)

**Layer 1: Micro Compact** (Zero API cost, in-memory)
- Replaces old tool results (> 800 bytes) with placeholders like `[Previous: used {tool_name}]`
- Keeps recent `keep_recent` (default 10) tool results untouched
- Exempt tools that carry critical workflow: `LoadSkill`, `Task`, `TodoWrite`, `TodoRead`, `Ask`

**Layer 2: Auto Compact** (LLM call)
- Triggered when token count exceeds `token_threshold` (default: 204,800 tokens)
- Calls LLM with summarization prompt
- Replaces entire message history with: `[Conversation compressed. Transcript: {path}]\n\n{summary}`
- Adds assistant acknowledgment: "Understood. I have the context from the summary. Continuing."

**Layer 3: Explicit Compact Tool**
- User/agent can call `Compact` tool to manually trigger Layer 2
- Optional `focus` parameter to specify what to preserve

## 2. Agent Message Construction (`agent.rs`)

### Location
`src/command/chat/agent.rs` (main agent loop: lines 20-492)

### How Messages Are Constructed for LLM

The `run_agent_loop()` function sends requests to the LLM with this message flow:

```rust
// Step 1: System prompt is optional but injected first
build_request_with_tools(
    &provider,
    &messages,
    tools.clone(),
    system_prompt.as_deref(),  // Optional system prompt
)
```

### Message Types in the Loop

**Core Message Roles:**
- `ROLE_USER` ("user") - User input and system notifications
- `ROLE_ASSISTANT` ("assistant") - Model responses (text + optional tool_calls)
- `ROLE_TOOL` ("tool") - Tool execution results

**Special Message Injection Points:**

1. **Pending User Messages** (lines 41-42, 495-509)
   - Drained at loop start with `[User appended]` marker
   - Allows incremental user input during agent execution

2. **Background Task Notifications** (lines 60-79)
   - Injected as `ROLE_USER` messages when background tasks complete
   - Format: `[后台任务完成] task_id={}, command={}, status={}\n结果:\n{}`

3. **Todo Reminder System** (lines 81-99)
   - After 15+ rounds without todo update, injects nag reminder
   - Format: Wraps todo list in `<system-reminder>` tags
   - **NOTE**: NOT current task tracking - only reminds about todo list

4. **Hook: PreLlmRequest** (lines 152-179)
   - Hooks can modify `messages` and `system_prompt` before LLM call
   - Can abort request or inject additional messages

5. **Hook: PostToolExecution** (lines 666-681)
   - Can modify tool result content before adding to messages

### Message Splitting for Better UI

When LLM returns both text + tool_calls (lines 608-626):
```rust
// Text message first (for UI to render text early)
if !assistant_text.is_empty() {
    messages.push(ChatMessage { role: ROLE_ASSISTANT, content: assistant_text, ... })
}
// Then tool_calls message (content empty, only carries tool_calls)
messages.push(ChatMessage { role: ROLE_ASSISTANT, content: String::new(), tool_calls: Some(...), ... })
```

### Important: NO Current Task Tracking

**The agent does NOT include:**
- Current active task in system prompt
- Task progress tracking in message injection
- Task intent signals beyond todo reminders

The task tracking is **separate** (via Task tool and TaskManager), not integrated into LLM message construction.

## 3. System Prompt Construction Logic

### Location
Primary: `src/command/chat/app.rs` (lines 2920-2948)
Template: `assets/system_prompt_default.md`
Loading: `src/command/chat/storage.rs` (lines 351-...)

### How System Prompt is Built

**Step 1: Load Template**
```rust
let template = load_system_prompt()?;  // Loads from ~/.jdata/agent/data/system_prompt.md
```

**Step 2: Gather Runtime Information**
```rust
let skills_summary = skill::build_skills_summary(&loaded_skills, &disabled_skills);
let commands_summary = command::build_commands_summary(&loaded_commands, &disabled_commands);
let tools_summary = tool_registry.build_tools_summary(&disabled_tools);
let style_text = load_style().unwrap_or_else(|| "（未设置）".to_string());
let memory_text = load_memory().unwrap_or_default();
let soul_text = load_soul().unwrap_or_default();
let current_dir = std::env::current_dir()...;
```

**Step 3: String Substitution**
```rust
let resolved = template
    .replace("{{.current_dir}}", &current_dir)
    .replace("{{.skills}}", &skills_summary)
    .replace("{{.skill_dir}}", &skill_dir)
    .replace("{{.project_skill_dir}}", &project_skill_dir)
    .replace("{{.commands}}", &commands_summary)
    .replace("{{.tools}}", &tools_summary)
    .replace("{{.style}}", &style_text)
    .replace("{{.memory}}", &memory_text)
    .replace("{{.soul}}", &soul_text);
```

### Default System Prompt Template

**File:** `assets/system_prompt_default.md`

**Key Sections:**

1. **Role Definition**
   ```
   You are a highly skilled software engineer. You solve the user's tasks 
   by reading, searching, and editing code using the available tools.
   ```

2. **Working Context**
   - Current working directory
   - System reminder tags explanation

3. **Working Principles**
   - Rigorous and meticulous
   - Use tools to perceive environment
   - Be honest about unknowns
   - Use Ask tool for clarification
   - **Use Task tool for complex multi-step tasks** (explicit recommendation!)
   - Use Markdown for image rendering

4. **Tool Usage Rules**
   - Specific tool for each job (Glob for file search, Grep for content, etc.)
   - Best practices (parallel execution, read before edit, etc.)
   - Git safety guidelines

5. **Skill System**
   - Points to skill assets location
   - Lists available skills to load

6. **Response Language**
   ```
   请使用中文回复
   ```
   (Use Chinese for responses)

7. **Available Tools Section**
   - `{{.tools}}` placeholder filled at runtime with tool definitions
   - Includes all enabled tools

8. **Memory/Soul Integration**
   - `{{.memory}}` - Persistent user information
   - `{{.soul}}` - User personality/style guidance
   - `{{.style}}` - Response style preferences

### Runtime System Prompt Resolution Flow

```
1. AgentHandle::spawn() called with system_prompt_fn closure
   ↓
2. Background thread executes system_prompt_fn()
   ↓
3. Loads template from storage
4. Builds dynamic summaries (skills, tools, commands)
5. Loads persistent files (memory, soul, style)
6. String substitution on template
   ↓
7. Returns Option<String>
   ↓
8. build_request_with_tools() injects into API request as system message
```

### Where System Prompt is Used

```rust
// In API request building (api.rs line 124):
if let Some(sys) = system_prompt {
    let trimmed = sys.trim();
    if !trimmed.is_empty() && let Ok(msg) = ChatCompletionRequestSystemMessageArgs::default()
        .content(trimmed)
        .build() { ... }
}

// In request preparation (app.rs line 2970):
let system_prompt_fn: Box<dyn FnOnce() -> Option<String> + Send> = Box::new(move || { ... });
```

### System Prompt is NOT Used For:

- Task tracking (Task tool is separate)
- Current operation intent (no "currently working on X" injection)
- Conversation compression context (compact tool handles this separately)
- Tool call guidance beyond what's in the template

## Summary Table

| Aspect | Details |
|--------|---------|
| **Compact Summary Focus** | What accomplished, current state, key decisions, active workflow progress |
| **Agent Message Construction** | No built-in task tracking; uses hooks, notifications, and todo reminders |
| **System Prompt Template** | Loaded once per session, substituted with runtime info (skills, tools, memory, soul) |
| **Current Task Tracking** | Explicitly NOT in system prompt; separate Task tool only |
| **Message Injection Points** | Pending user messages, background notifications, todo reminders, hooks |
| **Compaction Strategy** | 3-layer: micro (in-memory), auto (LLM summary), explicit (user-triggered) |

