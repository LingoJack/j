# Chat Command - Quick Reference Guide

## File Organization

```
chat/
├─ CORE LOGIC
│  ├─ mod.rs               Entry point + oneshot mode logic
│  ├─ app.rs              UI state machine (4800 lines)
│  ├─ agent.rs            Agent loop (3200 lines)
│  └─ api.rs              LLM API integration
│
├─ DATA & CONFIG
│  ├─ storage.rs          Persistent data structures
│  ├─ permission.rs       Security (.jcli/permissions.yaml)
│  ├─ skill.rs            Extensible prompts
│  ├─ command.rs          Custom commands
│  └─ hook.rs             Event hooks
│
├─ TOOLS (AGENTS CAN INVOKE)
│  ├─ tools/mod.rs        Tool registry & trait
│  ├─ tools/shell.rs      Execute bash commands
│  ├─ tools/file/*        File read/write/edit/glob
│  ├─ tools/grep.rs       Full-text search
│  ├─ tools/web_*         HTTP fetch & web search
│  ├─ tools/browser.rs    Browser automation
│  ├─ tools/ask.rs        Interactive questions
│  ├─ tools/background.rs Background task execution
│  ├─ tools/task/         Task management
│  └─ tools/todo/         Todo list management
│
├─ UI RENDERING
│  ├─ ui/chat.rs          Main chat interface
│  ├─ ui/archive.rs       Archive UI
│  ├─ ui/config.rs        Settings UI
│  ├─ markdown/           Markdown parsing & rendering
│  ├─ theme.rs            Color schemes
│  └─ render_cache.rs     Performance optimization
│
├─ EVENT HANDLING
│  ├─ handler/chat.rs     Chat mode events
│  ├─ handler/browse.rs   Message browsing
│  ├─ handler/archive.rs  Archive management
│  ├─ handler/config.rs   Configuration
│  ├─ handler/tui_loop.rs Main event loop (1800 lines)
│  └─ handler/mod.rs      Handler dispatch
│
└─ UTILITIES
   ├─ remote/             Mobile remote control
   ├─ compact.rs          Token optimization
   ├─ autocomplete.rs     CLI input suggestions
   ├─ archive.rs          Session archiving
   ├─ sandbox.rs          Execution sandboxing
   ├─ ui_helpers.rs       UI rendering helpers
   ├─ input_thread.rs     Background input
   └─ constants.rs        Message role constants
```

---

## Execution Paths

### 1. TUI Mode (Interactive)
```
j chat                       # No args → TUI
  ↓
handle_chat() → run_chat_tui()
  ↓
Initialize UI + Spawn tokio runtime
  ↓
Main event loop:
  - User types in input
  - Press Enter to send
  - Agent loop processes asynchronously
  - UI renders streamed responses
  - User can browse, archive, configure
```

### 2. Oneshot Mode (Quick)
```
j chat "your message"        # With args → oneshot
  ↓
handle_chat() → run_oneshot_agent()
  ↓
Stream response to stdout
  ↓
Print session ID
```

### 3. Continue Mode
```
j chat --continue "follow up"
  ↓
Load latest session
  ↓
Append new message
  ↓
Continue conversation
```

---

## Key Data Flow

### User Message → Response

```
User Input
    ↓
UIState.input updated
    ↓
Added to pending_user_messages queue
    ↓
Agent loop drains queue:
  - Merges into messages vector
  - Calls LLM API with streaming
  - Streams chunks via StreamMsg::Chunk
    ↓
TUI renders chunk in real-time
    ↓
If tool calls → User confirmation prompt
    ↓
Tool execution (synchronous)
    ↓
Tool result added to messages
    ↓
Next agent round OR stop
```

### Tool Execution Flow

```
LLM decides to call tool
    ↓
Parse tool call (name + JSON args)
    ↓
Check permission (.jcli/permissions.yaml)
    ├─ In deny list → reject
    ├─ In allow list → execute
    └─ Else → prompt user
    ↓
If needs confirmation:
  - Interactive menu (↑↓ navigate, Enter confirm)
  - Timeout can auto-execute
    ↓
ToolRegistry.execute(name, args)
    ├─ Find tool by name
    ├─ Deserialize JSON args
    ├─ Call tool.execute()
    └─ Capture output + error flag
    ↓
Return ToolResult (output, is_error, images)
    ↓
Add to messages as role="tool"
    ↓
Next agent round
```

---

## Message Structure

```rust
ChatMessage {
  role: "user" | "assistant" | "tool" | "system",
  content: String,
  tool_calls: Option<Vec<ToolCallItem>>,  // LLM → tools
  tool_call_id: Option<String>,            // tool → LLM
  images: Option<Vec<ImageData>>,         // multimodal
}
```

### Role Flow
```
User writes → role="user"
             ↓
LLM responds → role="assistant"
             ↓
Tool called → role="tool" + tool_call_id references assistant's tool_calls
             ↓
Next round → tool result merged, next response, etc.
```

---

## Configuration Files

### `~/.jdata/agent/data/agent_config.json`
```json
{
  "providers": [{
    "name": "GPT-4o",
    "api_base": "https://api.openai.com/v1",
    "api_key": "sk-...",
    "model": "gpt-4o",
    "supports_vision": true
  }],
  "active_index": 0,
  "tools_enabled": true,
  "max_tool_rounds": 100,
  "compact": {
    "enabled": true,
    "token_threshold": 20000,
    "keep_recent": 5
  }
}
```

### `.jcli/permissions.yaml` (Project-level)
```yaml
permissions:
  allow_all: false
  allow:
    - 'Bash:{"command":"ls.*"}'
  deny:
    - 'Bash:{"command":"rm.*"}'
```

### `~/.jdata/agent/data/system_prompt.md`
- Template with placeholders: `{{.tools}}`, `{{.skills}}`, etc.
- Resolved at runtime with tool list, skill contents, current dir, etc.

---

## Built-in Tools Reference

| Tool | Signature | Confirmation Required |
|------|-----------|----------------------|
| **Bash** | `{"command": "..."}` | Yes |
| **Read** | `{"file_path": "..."}` | No |
| **Write** | `{"file_path": "...", "content": "..."}` | No |
| **Edit** | `{"file_path": "...", "changes": [...]}` | No |
| **Glob** | `{"pattern": "..."}` | No |
| **Grep** | `{"pattern": "...", "path": "..."}` | No |
| **WebFetch** | `{"url": "..."}` | No |
| **WebSearch** | `{"query": "..."}` | No |
| **Browser** | `{"url": "...", "method": "..."}` | Yes |
| **Ask** | `{"questions": [...]}` | No (interactive) |
| **Background** | `{"command": "..."}` | Yes |
| **Task** | `{"action": "create\|get\|list\|update"}` | No |
| **Todo** | `{"action": "create\|list\|update"}` | No |

---

## UI Modes

### Chat Mode
- Type messages
- Tab key: autocomplete (skills, commands, files)
- Enter: send message
- Esc: clear input
- ↑↓: scroll history
- Tab then ↑: browse suggestions
- Ctrl+C: interrupt stream

### Browse Mode
- ↑↓: navigate messages
- Tab: expand/collapse message
- q: back to chat
- j/k: vim keys (up/down)

### Archive Mode
- ↑↓: select session
- Enter: load session
- d: delete session
- q: back to chat

### Config Mode
- Tab: navigate sections
- ↑↓: select provider/setting
- Enter: edit
- j/k: vim keys

---

## Key Concepts

### Agent Loop Rounds
```
for round in 0..max_tool_rounds {
  - Receive LLM response (streaming)
  - If finish_reason == "tool_calls":
    - Execute each tool
    - Append results
    - Continue loop
  - Else:
    - Save session
    - Exit loop
}
```

### Token Management
1. **Micro-compact**: Merge old tool results before each round
2. **Auto-compact**: LLM summarizes if token count > threshold
3. **History window**: Only send recent N messages to API

### Rendering Cache
- Pre-rendered message lines stored in `msg_lines_cache`
- Updated incrementally on new messages
- Invalidated on config/theme change

### Streaming Optimization
- Chunk size adaptation based on terminal width
- Throttled rendering (skip if too frequent)
- Cancellation token support (Ctrl+C)

---

## Extension Points

### Add Custom Tool
```rust
// 1. Create in tools/ directory
pub struct MyTool;

// 2. Implement Tool trait
impl Tool for MyTool {
    fn name(&self) -> &str { "MyTool" }
    fn description(&self) -> &str { "..." }
    fn parameters_schema(&self) -> Value { /* JSON Schema */ }
    fn execute(&self, args: &str, cancelled: &Arc<AtomicBool>) -> ToolResult { ... }
    fn requires_confirmation(&self) -> bool { true }
}

// 3. Register in ToolRegistry::new()
Box::new(MyTool { ... })
```

### Add Skill
```markdown
# ~/.jdata/agent/skills/my-skill.md or .jcli/skills/my-skill.md

---
name: "my-skill"
description: "What this skill does"
---

You are an expert in...
When asked to X, you should...
```

### Add Hook
```markdown
# ~/.jdata/agent/hooks/my-hook.md or .jcli/hooks/my-hook.md

---
name: "my-hook"
event: "tool_after"
---

Executed after tool: {tool_name}
Arguments: {arguments}
Result: {result}
```

### Add Permission Rule
```yaml
# .jcli/permissions.yaml
permissions:
  allow:
    - 'MyTool:{"param":"value"}'
```

---

## Common Patterns

### Oneshot with Tools
```rust
// In mod.rs
run_oneshot_agent(provider, agent_config, message, prior_messages, session_id)
  → Creates ToolRegistry
  → For each round:
    → API call with tools
    → Interactive confirmation (crossterm)
    → Execute tool (sync)
    → Collect result
    → Continue or break
```

### Tool Confirmation UI
```rust
// Interactive menu with arrow keys
let options = ["Allow", "Deny", "Always Allow"];
let choice = interactive_confirm(&tool_desc, &options, 0);

// Returns Some(index) or None
```

### Message Streaming
```rust
// In agent loop
loop {
  chunk = stream.next().await?;
  assistant_text.push_str(chunk);
  tx.send(StreamMsg::Chunk)?;  // Signal to UI
}
```

### Session Persistence
```rust
// Append-only pattern
append_session_event(session_id, &SessionEvent::Msg(message));
append_session_event(session_id, &SessionEvent::ToolCall(call));
```

---

## Debugging Tips

### Enable Logging
```rust
write_info_log("module_name", "message");
write_error_log("module_name", "error");
// Stored in: ~/.jdata/agent/logs/{info,error}.log
```

### Session ID
- Printed when chat completes
- Format: `{timestamp_us:x}-{pid:x}`
- Use to load conversation: `j chat --session <ID>`

### Check Configuration
```bash
cat ~/.jdata/agent/data/agent_config.json
cat .jcli/permissions.yaml
ls -la ~/.jdata/agent/data/skills/
ls -la .jcli/hooks/
```

### Monitor Streams
- TUI shows toast notifications
- Tool execution details logged
- Stream cache preserved for review in browse mode

