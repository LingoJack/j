# J-CLI Chat Command - Complete Architecture Analysis

## 1. Overview

The **chat command** is a sophisticated AI conversation module within the `j` CLI tool. It provides an interactive TUI (Terminal User Interface) for multi-model AI conversations with support for:

- **Multi-model support** (OpenAI, DeepSeek, custom API endpoints)
- **Streaming responses** with markdown rendering
- **Tool execution** (shell commands, file operations, web tools, etc.)
- **Agent mode** for autonomous multi-step reasoning
- **Session management** with persistent chat history
- **Skill system** for extensible AI capabilities
- **Permission-based security** for tool execution
- **Remote control** via mobile connection
- **Image/vision support** for multimodal models

**Code Size**: ~26,000 lines of Rust across 66 files in `src/command/chat/`

---

## 2. Directory Structure

```
src/command/chat/
├── mod.rs                          # Main entry point, oneshot mode
├── command.rs                      # Custom command system (markdown-based)
├── app.rs                          # UI state, modes, message types (4800+ lines)
├── agent.rs                        # Agent loop with tool calling (3200+ lines)
├── api.rs                          # OpenAI API client integration
├── storage.rs                      # Data structures and persistence
├── permission.rs                   # Tool execution permissions (.jcli/)
├── skill.rs                        # Skill system management
├── sandbox.rs                      # Sandboxing utilities
├── hook.rs                         # Event hooks (before/after tool execution)
├── autocomplete.rs                 # CLI input autocomplete
├── archive.rs                      # Session archiving
├── compact.rs                      # Token optimization (micro/auto compact)
├── render_cache.rs                 # Message rendering cache
├── theme.rs                        # TUI color themes
├── ui_helpers.rs                   # UI rendering helpers
├── input_thread.rs                 # Background input thread
├── constants.rs                    # Message role constants
│
├── handler/                        # Event handlers for different modes
│   ├── mod.rs                      # Handler dispatch
│   ├── chat.rs                     # Chat mode handler
│   ├── browse.rs                   # Message browsing mode
│   ├── archive.rs                  # Archive management mode
│   ├── config.rs                   # Configuration UI
│   ├── tool_confirm.rs             # Tool execution confirmation
│   └── tui_loop.rs                 # Main TUI event loop (1800+ lines)
│
├── tools/                          # Tool implementations (3000+ lines)
│   ├── mod.rs                      # Tool registry & trait
│   ├── shell.rs                    # Shell/bash execution
│   ├── file/                       # File operations
│   │   ├── read.rs                 # File reading
│   │   ├── write.rs                # File writing
│   │   ├── edit.rs                 # File editing
│   │   └── glob.rs                 # File globbing
│   ├── grep.rs                     # File searching (ripgrep)
│   ├── web_fetch.rs                # HTTP fetching
│   ├── web_search.rs               # Web search
│   ├── browser.rs                  # Browser automation (CDP mode)
│   ├── ask.rs                      # Interactive questions
│   ├── computer_use.rs             # Screen capture & mouse/keyboard
│   ├── skill.rs                    # Skill loading
│   ├── background.rs               # Background task execution
│   ├── classification.rs           # Content classification
│   ├── task/                       # Task management
│   │   ├── mod.rs
│   │   ├── entity.rs
│   │   ├── task_manager.rs
│   │   └── task_tool.rs
│   └── todo/                       # Todo list management
│       ├── mod.rs
│       ├── entity.rs
│       ├── todo_manager.rs
│       ├── todo_read_tool.rs
│       └── todo_write_tool.rs
│
├── ui/                             # UI components
│   ├── mod.rs
│   ├── chat.rs                     # Main chat interface
│   ├── archive.rs                  # Archive UI
│   └── config.rs                   # Configuration UI
│
├── markdown/                       # Markdown processing
│   ├── mod.rs
│   ├── parser.rs                   # Markdown parsing
│   ├── highlight.rs                # Syntax highlighting
│   ├── image_cache.rs              # Image caching
│   └── image_loader.rs             # Image loading
│
├── remote/                         # Remote control features
│   ├── mod.rs
│   ├── protocol.rs                 # Communication protocol
│   ├── bridge.rs                   # Remote bridge
│   ├── crypto.rs                   # Encryption
│   └── server.rs                   # Remote server
```

---

## 3. Core Data Structures

### 3.1 Agent Configuration (`storage.rs`)

```rust
pub struct AgentConfig {
    pub providers: Vec<ModelProvider>,              // Multiple LLM providers
    pub active_index: usize,                        // Current provider
    pub system_prompt: Option<String>,              // System prompt template
    pub stream_mode: bool,                          // Streaming enabled
    pub max_history_messages: usize,                // History window (default 20)
    pub theme: ThemeName,                           // UI theme
    pub tools_enabled: bool,                        // Tool calling enabled
    pub max_tool_rounds: usize,                     // Max agent iterations
    pub style: Option<String>,                      // Response style
    pub tool_confirm_timeout: u64,                  // Tool confirmation timeout
    pub disabled_tools: Vec<String>,                // Disabled tools
    pub disabled_skills: Vec<String>,               // Disabled skills
    pub disabled_commands: Vec<String>,             // Disabled commands
    pub compact: CompactConfig,                     // Token optimization
    pub auto_restore_session: bool,                 // Auto-restore on startup
}

pub struct ModelProvider {
    pub name: String,                               // Display name (e.g., "GPT-4o")
    pub api_base: String,                           // API endpoint
    pub api_key: String,                            // API key
    pub model: String,                              // Model name
    pub supports_vision: bool,                      // Multimodal support
}
```

### 3.2 Chat Messages (`storage.rs`)

```rust
pub struct ChatMessage {
    pub role: String,                               // "user" / "assistant" / "tool" / "system"
    pub content: String,                            // Message text
    pub tool_calls: Option<Vec<ToolCallItem>>,      // LLM-generated tool calls
    pub tool_call_id: Option<String>,               // For tool responses
    pub images: Option<Vec<ImageData>>,             // For vision models
}

pub struct ToolCallItem {
    pub id: String,                                 // Unique call ID
    pub name: String,                               // Tool name
    pub arguments: String,                          // JSON arguments
}
```

### 3.3 UI State (`app.rs`)

```rust
pub struct UIState {
    pub input: String,                              // User input buffer
    pub cursor_pos: usize,                          // Cursor position
    pub mode: ChatMode,                             // Chat/Browse/Archive/Config
    pub scroll_offset: u16,                         // Message list scroll
    pub auto_scroll: bool,                          // Auto-scroll to bottom
    pub browse_msg_index: usize,                    // Selected message index
    pub browse_scroll_offset: u16,                  // Message detail scroll
    pub model_list_state: ListState,                // Model selector state
    pub toast: Option<(String, bool, Instant)>,     // Notification message
    pub msg_lines_cache: Option<MsgLinesCache>,     // Rendered lines cache
}

pub enum ChatMode {
    Chat,                                           // Normal chat input
    Browse,                                         // Browse messages
    Archive,                                        // Archive management
    Config,                                         // Configuration
}
```

### 3.4 Tool System (`tools/mod.rs`)

```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;                      // JSON Schema
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;
    fn requires_confirmation(&self) -> bool { false }
    fn confirmation_message(&self, arguments: &str) -> String
}

pub struct ToolResult {
    pub output: String,                             // Result text
    pub is_error: bool,                             // Error flag
    pub images: Vec<ImageData>,                     // Optional images
}
```

---

## 4. Main Execution Flows

### 4.1 Entry Point (`mod.rs: handle_chat()`)

Two modes:
1. **TUI Mode** (interactive): Called when no arguments, remote flag, or no provider configured
2. **Oneshot Mode** (quick): `j chat "message"` - send message and get response

```
handle_chat()
├─ If remote or no args → run_chat_tui()      [TUI event loop]
└─ Else → run_oneshot_agent()                 [Simple streaming]
```

### 4.2 TUI Mode (`handler/tui_loop.rs`)

**Event Loop**:
- Runs async agent loop in background thread
- Main thread handles user input and renders UI
- Cross-thread communication via channels (`StreamMsg`, `ToolResultMsg`)

```
run_chat_tui()
├─ Initialize TUI components
├─ Spawn agent_loop task (tokio)
├─ Spawn input thread (stdin)
└─ Main loop:
   ├─ Handle keyboard events (arrow keys, Enter, etc.)
   ├─ Process agent stream messages
   ├─ Handle tool execution confirmations
   ├─ Render UI (messages, input box, status)
   └─ Update state
```

**Modes**:
- **Chat Mode**: Type messages, press Enter to send
- **Browse Mode**: Arrow keys to navigate messages, view details
- **Archive Mode**: Manage saved sessions
- **Config Mode**: Configure providers, settings, theme

### 4.3 Agent Loop (`agent.rs: run_agent_loop()`)

**Multi-turn reasoning with tool calling**:

```
for each round (max_tool_rounds):
  1. Drain pending user messages from queue
  2. Micro-compact: merge old tool results
  3. Auto-compact: LLM-summarize if tokens > threshold
  4. Inject background task notifications
  5. Inject todo list reminders (every 15 turns)
  
  6. API call with streaming:
     - Send messages + tools + system prompt
     - Stream response, collect tool calls
  
  7. If no tool calls:
     - Save session and return
  
  8. For each tool call:
     - Check permissions (.jcli/permissions.yaml)
     - Prompt user confirmation (if needed)
     - Execute tool
     - Collect result
  
  9. Append assistant + tool results to messages
  
  10. Continue to next round
```

### 4.4 Oneshot Mode (`mod.rs: run_oneshot_agent()`)

Simpler version for quick queries:
- No async/tokio
- Single API call (no loops)
- Interactive tool confirmation (crossterm)
- No background tasks
- Simple streaming output

---

## 5. Tool System

### 5.1 Built-in Tools

| Tool | Category | Purpose |
|------|----------|---------|
| **Shell** | Execution | Run bash/shell commands |
| **ReadFile** | File I/O | Read file contents |
| **WriteFile** | File I/O | Write/create files |
| **EditFile** | File I/O | Edit file ranges |
| **Glob** | File I/O | Pattern matching on files |
| **Grep** | Search | Full-text search with ripgrep |
| **WebFetch** | Web | HTTP fetch + markdown conversion |
| **WebSearch** | Web | Web search integration |
| **Browser** | Automation | Chromium CDP mode |
| **ComputerUse** | Automation | Screen capture, mouse/keyboard |
| **Ask** | Interaction | Interactive questions with options |
| **Background** | Tasks | Run commands in background |
| **TaskOutput** | Tasks | Get background task results |
| **Todo** (read/write) | Productivity | Manage todo lists |
| **Task** | Productivity | Manage task items |
| **Skill** | Extension | Load and invoke skills |
| **Hook** | Extension | Execute hook scripts |

### 5.2 Tool Registry (`tools/mod.rs`)

```rust
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    pub todo_manager: Arc<TodoManager>,
}

impl ToolRegistry {
    pub fn new(
        skills: Vec<Skill>,
        ask_tx: mpsc::Sender<AskRequest>,
        background_manager: Arc<BackgroundManager>,
        task_manager: Arc<TaskManager>,
        hook_manager: Arc<Mutex<HookManager>>,
    ) -> Self { ... }
    
    pub fn to_openai_tools_filtered(&self, disabled: &[String]) -> Vec<ChatCompletionTools> { ... }
    pub fn execute(&self, name: &str, args: &str, cancelled: &Arc<AtomicBool>) -> ToolResult { ... }
    pub fn build_tools_summary(&self, disabled: &[String]) -> String { ... }
}
```

---

## 6. Permission System

### 6.1 Permission Configuration (`.jcli/permissions.yaml`)

```yaml
permissions:
  allow_all: false                  # Allow all tools without confirmation
  allow:                            # Rules to skip confirmation
    - 'Bash:{"command":"ls.*"}'    # Regex matching
    - 'Read:{"path":"/public/*"}'
  deny:                             # Rules to always deny
    - 'Bash:{"command":"rm.*"}'
    - 'Write:{"path":"/system/*"}'
```

### 6.2 Permission Checking

```rust
pub fn is_allowed(&self, tool_name: &str, arguments: &str) -> bool {
    // deny list is checked first
    if self.is_denied(tool_name, arguments) {
        return false;
    }
    
    // then allow list or allow_all
    self.permissions.allow_all || self.is_in_allow_list(tool_name, arguments)
}
```

---

## 7. Storage & Persistence

### 7.1 Data Directories

```
~/.jdata/agent/
├── data/
│   ├── agent_config.json           # Configuration (JSON)
│   ├── system_prompt.md            # System prompt template
│   ├── memory.md                   # Long-term memory
│   ├── soul.md                     # Personality/soul config
│   ├── style.md                    # Response style
│   └── sessions/                   # Chat session archives
│       ├── {session_id}/
│       │   ├── messages.json       # Serialized messages
│       │   └── metadata.json       # Session metadata
└── logs/
    ├── info.log
    └── error.log
```

### 7.2 Session Management

- **Oneshot Session ID**: `{timestamp_micros:x}-{pid:x}`
- **Session Events**: Append-only log pattern
- **Auto-restore**: Load last session on startup (if enabled)

```rust
pub fn load_session(session_id: &str) -> ChatSession { ... }
pub fn append_session_event(session_id: &str, event: &SessionEvent) { ... }
pub fn list_sessions() -> Vec<String> { ... }
pub fn delete_session(session_id: &str) { ... }
```

---

## 8. Key Features

### 8.1 Streaming & Rendering

- **Streaming**: SSE from OpenAI API
- **Real-time**: Display chunks as they arrive
- **Markdown Rendering**: Full-featured markdown with:
  - Syntax highlighting for code blocks
  - Table formatting
  - Image display (with caching)
  - Link handling
- **Throttling**: Render optimization for high-frequency updates

### 8.2 Token Management

**Micro-compact**: Summarize old tool results:
```rust
pub fn micro_compact(messages: &mut Vec<ChatMessage>, keep_recent: usize) {
    // Keep most recent K tool results, merge older ones
}
```

**Auto-compact**: LLM-based summarization:
```rust
pub async fn auto_compact(
    messages: &mut Vec<ChatMessage>,
    provider: &ModelProvider,
) -> Result<()> {
    // Call LLM to summarize conversation history
}
```

### 8.3 Hook System

**Three levels** of hooks:
1. **User level**: `~/.jdata/agent/hooks/`
2. **Project level**: `.jcli/hooks/`
3. **Session level**: In-memory hooks

```rust
pub enum HookEvent {
    ToolBefore(String, String),      // Before tool execution
    ToolAfter(String, String, String), // After tool execution
    AgentRound(u32),                  // After each agent round
    MessageReceived(String),          // After receiving message
}
```

### 8.4 Skill System

```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub body: String,                 // Markdown content
    pub source: SkillSource,           // User or Project
}

// Skills are injected into system prompt:
// "Here are available skills you can use:\n<skill_body>"
```

### 8.5 Custom Commands

Similar to skills but with different structure:
```rust
pub struct CustomCommand {
    pub frontmatter: CommandFrontmatter, // name, description
    pub body: String,                    // Prompt text
    pub source: CommandSource,           // User or Project
}

// Usage: @command:name in prompts
```

---

## 9. Implementation Highlights

### 9.1 Multimodal Support

```rust
// Vision models can receive images
pub struct ChatMessage {
    pub images: Option<Vec<ImageData>>,
}

// Images loaded from:
// - URL (WebFetch tool)
// - File (ReadFile tool with image detection)
// - Computer use (screenshot tool)
```

### 9.2 Background Tasks

```rust
pub struct BackgroundTask {
    pub task_id: String,
    pub command: String,
    pub status: String,               // running / completed / failed
    pub result: String,
}

pub struct BackgroundManager {
    tasks: Arc<Mutex<HashMap<String, BackgroundTask>>>,
}
```

### 9.3 Interactive Confirmations

```rust
// Tool execution confirmation with options
let options = ["Allow", "Deny", "Always Allow"];
let choice = interactive_confirm(&tool_msg, &options, 0);

// Ask tool with multi-select/single-select
pub struct AskQuestion {
    pub question: String,
    pub options: Vec<AskOption>,
    pub multi_select: bool,
}
```

### 9.4 Remote Control

```rust
// QR code generation + websocket server
// Mobile can control chat via:
// - Send messages
// - Confirm tool execution
// - View responses
```

---

## 10. Configuration

### 10.1 Agent Config File (`~/.jdata/agent/data/agent_config.json`)

```json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-...",
      "model": "gpt-4o",
      "supports_vision": true
    }
  ],
  "active_index": 0,
  "stream_mode": true,
  "max_history_messages": 20,
  "theme": "dark",
  "tools_enabled": true,
  "max_tool_rounds": 100,
  "tool_confirm_timeout": 0,
  "disabled_tools": [],
  "compact": {
    "enabled": true,
    "token_threshold": 20000,
    "keep_recent": 5
  }
}
```

---

## 11. Message Flow Diagrams

### 11.1 TUI Chat Flow

```
┌─────────────────────────────────────────────┐
│         User Input (Terminal)               │
│  Type message + Press Enter                 │
└────────────────┬────────────────────────────┘
                 │
                 ▼
    ┌─────────────────────────┐
    │  UI State Updated       │
    │  (input buffer cleared) │
    └────────────┬────────────┘
                 │
                 ▼
    ┌──────────────────────────────┐
    │  Queued for Agent Loop       │
    │  (pending_user_messages)     │
    └────────────┬─────────────────┘
                 │
                 ▼
    ┌──────────────────────────────┐
    │  Agent Loop (async)          │
    │  - Drain pending messages    │
    │  - Call OpenAI with stream   │
    │  - Parse tool calls          │
    └────────────┬─────────────────┘
                 │
        ┌────────┴────────┐
        │                 │
        ▼                 ▼
   Text Stream      Tool Calls
        │                 │
        ▼                 ▼
   ┌─────────────┐   ┌──────────────┐
   │ StreamMsg:: │   │StreamMsg::   │
   │ Chunk       │   │ToolCallReq   │
   └─────┬───────┘   └──────┬───────┘
         │                  │
         ▼                  ▼
    ┌─────────────────────────────────┐
    │ Main Thread TUI Loop            │
    │ - Render messages + status      │
    │ - Show tool confirmation prompt │
    └────────────┬────────────────────┘
                 │
                 ▼
    ┌──────────────────────────────┐
    │ Wait for Tool Confirmation   │
    │ or Auto-execute if timeout   │
    └────────────┬─────────────────┘
                 │
      ┌──────────┴──────────┐
      │                     │
      ▼                     ▼
   Execute              Send Result
   Tool                 Back to Agent
      │                     │
      └──────────┬──────────┘
                 │
                 ▼
        ┌─────────────────────────┐
        │  Next Agent Round       │
        │  or Complete            │
        └─────────────────────────┘
```

### 11.2 Oneshot Mode Flow

```
j chat "message"
    │
    ▼
generate_oneshot_session_id()
    │
    ▼
Load prior messages (if --continue)
    │
    ▼
run_oneshot_agent()
    │
    ├─ Create ToolRegistry
    ├─ Resolve system prompt
    └─ For each tool round:
       ├─ API stream call
       ├─ Interactive confirmation (crossterm)
       ├─ Execute tool (synchronous)
       ├─ Append results
       └─ Continue or break
    │
    ▼
Persist session
Print session ID
```

---

## 12. Performance Optimizations

### 12.1 Rendering Cache
- **Message lines cache**: Pre-render complex messages
- **Throttling**: Skip renders if update frequency too high
- **Incremental**: Only render changed portions

### 12.2 Memory Management
- **Micro-compact**: Keep memory low
- **Auto-compact**: Summarize old conversations
- **Token estimation**: Prevent API overages
- **History window**: Keep only recent N messages

### 12.3 Streaming Optimization
- **Chunked rendering**: Process stream in batches
- **Terminal width aware**: Adjust line wrapping
- **Cancellation token**: Stop stream on Ctrl+C

---

## 13. Error Handling

### 13.1 Error Types

```rust
- API errors: Network, auth, rate limit, timeout
- Tool errors: Execution failure, permission denied
- UI errors: Invalid input, state corruption
- Storage errors: File I/O, JSON parsing
```

### 13.2 Error Recovery

- **Auto-retry**: Network errors with backoff
- **Fallback**: Degraded mode if API fails
- **User feedback**: Toast notifications
- **Logging**: Dual logs (info + error)

---

## 14. Extension Points

### 14.1 Adding New Tools

1. Implement `Tool` trait
2. Add to `ToolRegistry::new()`
3. Define JSON schema for parameters
4. Handle execution + errors

```rust
pub struct MyTool;
impl Tool for MyTool {
    fn name(&self) -> &str { "MyTool" }
    fn description(&self) -> &str { "..." }
    fn parameters_schema(&self) -> Value { json!({ ... }) }
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult { ... }
    fn requires_confirmation(&self) -> bool { true }
}
```

### 14.2 Adding Hooks

Create `.jcli/hooks/HOOKNAME.md` or `~/.jdata/agent/hooks/HOOKNAME.md`:

```markdown
---
name: "my-hook"
event: "tool_after"
---

Executed after tool call: {tool_name}
Arguments: {arguments}
Result: {result}
```

### 14.3 Adding Skills

Create `.jcli/skills/SKILLNAME.md` or `~/.jdata/agent/skills/SKILLNAME.md`:

```markdown
---
name: "code-review"
description: "Review code for quality"
---

You are an expert code reviewer...
```

---

## 15. Testing & Debugging

### 15.1 Logging

```rust
write_info_log("module_name", &format!("message: {}", value));
write_error_log("module_name", &format!("error: {}", error));
```

Logs stored in: `~/.jdata/agent/logs/{info,error}.log`

### 15.2 Debug Output

- TUI shows toast notifications for status
- Session ID printed on completion
- Tool execution details logged
- Stream content cached for review

---

## 16. Summary Table

| Aspect | Detail |
|--------|--------|
| **Language** | Rust |
| **LOC** | ~26,000 |
| **Files** | 66 |
| **Core Pattern** | Event-driven TUI + async agent loop |
| **Concurrency** | Tokio (async), threads (input), channels (IPC) |
| **API** | OpenAI-compatible (rest/streaming) |
| **Database** | File-based (JSON/YAML) |
| **UI Framework** | Ratatui (TUI) |
| **Key Dependencies** | tokio, async-openai, serde, crossterm, ratatui |
| **Extensions** | Tools, skills, hooks, commands |
| **Security** | Permission system, capability-based |
| **Persistence** | Session history, config, logs |

---

## 17. Key Modules Responsibility

| Module | Responsibility |
|--------|-----------------|
| `mod.rs` | Entry point, oneshot mode |
| `app.rs` | UI state machine, message types |
| `agent.rs` | Multi-turn reasoning loop |
| `api.rs` | LLM API integration |
| `storage.rs` | Data persistence, config |
| `tools/mod.rs` | Tool registration & execution |
| `handler/` | Event handlers for different modes |
| `ui/` | Ratatui components |
| `permission.rs` | Security & ACL |
| `skill.rs` | Extensible prompts |
| `command.rs` | Custom command system |
| `compact.rs` | Token optimization |
| `markdown/` | Content rendering |
| `remote/` | Mobile remote control |
| `hook.rs` | Extension points |

