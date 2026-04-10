# j-cli Project Architecture Exploration Report

**Purpose**: Understanding the j-cli tool system to implement a new "agent team" tool  
**Date**: April 10, 2026  
**Project Root**: `/Users/jacklingo/dev_custom/j/`

---

## 1. Tool Trait Definition and Key Methods

**File**: `/src/command/chat/tools/mod.rs` (lines 76-91)

### Tool Trait Interface
```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;  // JSON Schema for parameters
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;
    fn requires_confirmation(&self) -> bool { false }
    fn confirmation_message(&self, arguments: &str) -> String { ... }
}
```

### Key Data Structures

**ToolResult** (lines 64-72):
```rust
pub struct ToolResult {
    pub output: String,              // Result to return to LLM
    pub is_error: bool,              // Error flag
    pub images: Vec<ImageData>,      // Optional image data for vision
}
```

**ImageData** (lines 56-62):
```rust
pub struct ImageData {
    pub base64: String,              // Base64 encoded image
    pub media_type: String,          // MIME type (e.g., "image/png")
}
```

### Helper Functions
- `schema_to_tool_params<T: JsonSchema>()` - Extracts tool parameters schema
- `parse_tool_args<T>(&str)` - Parses JSON arguments into Rust struct
- `is_dangerous_command()` - Safety filter for shell commands
- `check_blocking_command()` - Detects interactive/blocking commands

---

## 2. Tool Registration System

**File**: `/src/command/chat/tools/mod.rs` (lines 94-309)

### ToolRegistry Structure
```rust
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    pub todo_manager: Arc<TodoManager>,
    pub plan_mode_state: Arc<PlanModeState>,
    pub worktree_state: Arc<WorktreeState>,
}
```

### Registration Process

**Creation** (lines 108-184):
```rust
pub fn new(
    skills: Vec<Skill>,
    ask_tx: mpsc::Sender<AskRequest>,
    background_manager: Arc<BackgroundManager>,
    task_manager: Arc<TaskManager>,
    hook_manager: Arc<Mutex<HookManager>>,
) -> Self
```

**Built-in Tools Registered**:
1. **Shell** - Execute shell commands
2. **File Tools**:
   - Read, Write, Edit, Glob
3. **Search Tools**:
   - Grep, WebFetch, WebSearch
4. **Browser** - Browser automation
5. **Ask** - Interactive user questions
6. **Background** - Task management
7. **Computer Use** - GUI interaction
8. **Plan Mode** - Enter/Exit plan mode
9. **Worktree** - Git worktree isolation
10. **Task/Todo** - Task and todo list management
11. **Hook** - Register hooks
12. **Compact** - Context compaction
13. **LoadSkill** - Load skills (when skills available)

### Key Registry Methods

```rust
pub fn register(&mut self, tool: Box<dyn Tool>)
pub fn get(&self, name: &str) -> Option<&dyn Tool>
pub fn execute(&self, name: &str, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult
pub fn build_tools_summary(&self, disabled: &[String]) -> String
pub fn to_openai_tools_filtered(&self, disabled: &[String]) -> Vec<ChatCompletionTools>
pub fn tool_names(&self) -> Vec<&str>
```

### Plan Mode Integration (lines 201-256)
Tools are filtered through `plan_mode_state`:
- When plan mode active, only whitelisted tools allowed
- Whitelist includes: Read, Glob, Grep, WebFetch, WebSearch, Ask, Compact, TodoRead, TodoWrite
- Write/Edit allowed if targeting plan file

---

## 3. Agent Loop Logic

**File**: `/src/command/chat/agent.rs` (main agent execution)

### Agent Loop Entry Point
```rust
pub async fn run_agent_loop(
    config: AgentLoopConfig,              // Static config
    shared: AgentSharedState,             // Shared runtime state
    mut messages: Vec<ChatMessage>,       // Conversation history
    tools: Vec<ChatCompletionTools>,      // Available tools
    mut system_prompt: Option<String>,
    tx: mpsc::Sender<StreamMsg>,          // → UI (streaming updates)
    tool_result_rx: mpsc::Receiver<ToolResultMsg>,  // ← Tool results
)
```

### Per-Round Flow (lines 67-400+)

1. **Message Management**:
   - Drain pending user messages (from UI input during loop)
   - Micro compact: Replace old tool results
   - Auto compact: LLM summarization if token threshold exceeded

2. **Notifications**:
   - Drain background task completions
   - Inject todo reminders if needed

3. **Pre-LLM Hook** (lines 182-208):
   - Event: `HookEvent::PreLlmRequest`
   - Can modify messages, system_prompt, inject messages
   - Can abort request

4. **LLM Request**:
   - Build request with tools (lines 235-249)
   - Create streaming connection
   - Collect response chunks

5. **Tool Call Extraction** (lines 268-270):
   ```rust
   let mut raw_tool_calls: BTreeMap<u32, (String, String, String)> = BTreeMap::new();
   ```
   Aggregates tool calls by index: (id, name, arguments)

6. **Finish Reason Check**:
   - `FinishReason::ToolCalls` → Process tool calls
   - Other → Exit loop or continue

7. **Tool Call Processing** (process_tool_calls):
   - Iterate through tool calls
   - Send to UI for confirmation (if needed)
   - Receive confirmation/denial
   - Execute tool via registry
   - Collect results

8. **Message Accumulation**:
   - Add assistant message with tool_calls
   - Add tool result messages (role="tool")
   - Next round uses full history

### Configuration Structures

**AgentLoopConfig** (`/src/command/chat/agent_config.rs`, lines 9-21):
```rust
pub struct AgentLoopConfig {
    pub provider: ModelProvider,
    pub max_tool_rounds: usize,           // Default: 10
    pub compact_config: CompactConfig,
    pub hook_manager: HookManager,
    pub cancel_token: CancellationToken,
}
```

**AgentSharedState** (lines 23-37):
```rust
pub struct AgentSharedState {
    pub streaming_content: Arc<Mutex<String>>,    // Agent writes, UI reads
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,  // User input queue
    pub background_manager: Arc<BackgroundManager>,
    pub todo_manager: Arc<TodoManager>,
    pub shared_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub context_tokens: Arc<Mutex<usize>>,        // Token count estimate
}
```

---

## 4. Existing Agent Tool (Sub-agent Spawning)

**File**: `/src/command/chat/tools/agent.rs`

### AgentTool Structure
```rust
pub struct AgentTool {
    pub background_manager: Arc<BackgroundManager>,
    pub provider: Arc<Mutex<ModelProvider>>,
    pub system_prompt: Arc<Mutex<Option<String>>>,
    pub jcli_config: Arc<JcliConfig>,
    pub compact_config: CompactConfig,
    pub hook_manager: Arc<Mutex<HookManager>>,
    pub task_manager: Arc<TaskManager>,
    pub todo_manager: Arc<TodoManager>,
    pub disabled_tools: Arc<Vec<String>>,
}
```

### Parameters
```rust
struct AgentParams {
    prompt: String,              // Task for sub-agent
    description: Option<String>, // Short (3-5 word) description
    run_in_background: bool,     // Foreground vs background execution
}
```

### Sub-agent Execution (lines 89-182)

**Foreground Mode** (synchronous, blocks):
1. Create child registry (excludes "Agent" to prevent recursion)
2. Call `run_headless_agent_loop()` synchronously
3. Return final assistant text as output

**Background Mode** (asynchronous):
1. Register task in background manager: `spawn_command()`
2. Spawn std::thread
3. Return task_id immediately
4. Thread executes `run_headless_agent_loop()`
5. Results written to output buffer
6. Task marked complete and notified to main agent loop

### Headless Agent Loop (lines 196-400)

```rust
fn run_headless_agent_loop(
    provider: ModelProvider,
    system_prompt: Option<String>,
    prompt: String,
    tools: Vec<ChatCompletionTools>,
    registry: Arc<ToolRegistry>,
    jcli_config: Arc<JcliConfig>,
    cancelled: &Arc<AtomicBool>,
) -> String
```

**Key Differences from Main Agent Loop**:
- No streaming (non-stream request)
- No UI interaction
- Uses permission checks (is_denied, is_allowed) instead of user confirmation
- Max 30 rounds (vs configurable main loop)
- Returns final assistant text only
- Respects cancellation token

**Permission Checks** (lines 327-366):
- Check deny rules: `jcli_config.is_denied()`
- Check confirmation requirements + allow rules: `jcli_config.is_allowed()`
- Returns "Tool denied" message for denied tools

---

## 5. Background Task Management

**File**: `/src/command/chat/tools/background.rs`

### BackgroundManager Structure
```rust
pub struct BackgroundManager {
    pub(super) tasks: Mutex<HashMap<String, BgTask>>,
    notifications: Mutex<Vec<BgNotification>>,
    next_id: Mutex<u64>,
}
```

### BgTask Structure
```rust
pub(super) struct BgTask {
    pub task_id: String,
    pub command: String,
    pub status: String,              // "running" | "completed" | "error" | "timeout"
    pub output_buffer: Arc<Mutex<String>>,  // Shared buffer for real-time output
    pub result: Option<String>,
}
```

### BgNotification Structure
```rust
pub struct BgNotification {
    pub task_id: String,
    pub command: String,
    pub status: String,
    pub result: String,
}
```

### Key Methods

```rust
pub fn spawn_command(&self, command: &str, _cwd: Option<String>, _timeout_secs: u64) 
    -> (String, Arc<Mutex<String>>)
    // Returns: (task_id, shared output_buffer)

pub fn complete_task(&self, task_id: &str, status: &str, result: String)
    // Marks task complete and adds notification

pub fn drain_notifications(&self) -> Vec<BgNotification>
    // Agent loop calls each round to get completed tasks

pub fn get_task_status(&self, task_id: &str) -> Option<Value>
    // Query current status + intermediate output

pub fn is_running(&self, task_id: &str) -> bool
    // Check if task still executing
```

### Real-time Output Pattern

1. Background task spawner gets `(task_id, output_buffer)`
2. Worker thread writes to `output_buffer` during execution
3. TaskOutputTool polls `output_buffer` for intermediate output
4. Agent loop polls `drain_notifications()` for completion

---

## 6. Task Management System

**Files**: 
- `/src/command/chat/tools/task/mod.rs`
- `/src/command/chat/tools/task/task_manager.rs`
- `/src/command/chat/tools/task/entity.rs`

### AgentTask Entity
```rust
pub struct AgentTask {
    pub task_id: u64,
    pub title: String,
    pub description: String,
    pub status: String,              // "pending" | "in_progress" | "completed" | "deleted"
    pub blocked_by: Vec<u64>,        // Task IDs blocking this one
    pub owner: String,
    pub task_doc_paths: Vec<String>,
}
```

### TaskManager Structure
```rust
pub struct TaskManager {
    tasks_dir: PathBuf,              // .jcli/tasks/ or ~/.jdata/agent/data/tasks/
    write_lock: Mutex<()>,
}
```

### Key Methods
```rust
pub fn create_task(
    &self,
    title: &str,
    description: &str,
    blocked_by: Vec<u64>,
    task_doc_paths: Vec<String>,
) -> Result<AgentTask, String>

pub fn list_ready_tasks(&self) -> Vec<AgentTask>  // Pending + no blockers

pub fn get_task(&self, id: u64) -> Result<AgentTask, String>

pub fn list_tasks(&self) -> Vec<AgentTask>

pub fn update_task_status(&self, id: u64, status: &str) -> Result<(), String>
```

### Persistence
- Tasks stored as `task_{id}.json` files
- Auto-increment ID based on max existing file ID
- Write lock prevents concurrent modifications

---

## 7. Skill System

**Files**:
- `/src/command/chat/skill.rs` (loading & parsing)
- `/src/command/chat/tools/skill.rs` (LoadSkillTool)

### Skill Structure
```rust
pub struct Skill {
    pub frontmatter: SkillFrontmatter,  // YAML frontmatter with name, description
    pub body: String,                   // Markdown content after frontmatter
    pub dir_path: PathBuf,              // Skill directory path
    pub source: SkillSource,            // User or Project level
}

pub enum SkillSource {
    User,       // ~/.jdata/agent/skills/
    Project,    // .jcli/skills/ (project-level)
}
```

### Loading
- User skills: `~/.jdata/agent/skills/`
- Project skills: `.jcli/skills/` (overrides user skills with same name)
- Each skill is a directory with `SKILL.md` file

### LoadSkillTool
```rust
pub struct LoadSkillTool {
    pub skills: Vec<Skill>,
}
```

- Registered only when skills are available
- Allows agent to load full skill content into context
- Supports `$ARGUMENTS` placeholder substitution

---

## 8. Key Data Structures for Messages

**File**: `/src/command/chat/storage.rs`

### ChatMessage
```rust
pub struct ChatMessage {
    pub role: String,                           // "user" | "assistant" | "tool" | "system"
    pub content: String,                        // Message text
    pub tool_calls: Option<Vec<ToolCallItem>>,  // LLM tool calls (assistant only)
    pub tool_call_id: Option<String>,           // Tool result ID (tool role only)
    pub images: Option<Vec<ImageData>>,         // Multi-modal images (not persisted)
}
```

### ToolCallItem
```rust
pub struct ToolCallItem {
    pub id: String,           // Unique call ID from LLM
    pub name: String,         // Tool name
    pub arguments: String,    // JSON string of parameters
}
```

### ModelProvider
```rust
pub struct ModelProvider {
    pub name: String,                 // Display name
    pub api_base: String,             // API URL
    pub api_key: String,              // API key
    pub model: String,                // Model name
    pub supports_vision: bool,        // Multi-modal support
}
```

---

## 9. Cross-Thread Communication

**File**: `/src/command/chat/app/types.rs`

### StreamMsg (Agent → UI)
```rust
pub enum StreamMsg {
    Chunk,                              // Streaming text chunk
    ToolCallRequest(Vec<ToolCallItem>),  // Tool calls detected
    Done,                               // Stream complete
    Error(String),                      // Error occurred
    Cancelled,                          // User cancelled
}
```

### ToolResultMsg (UI → Agent)
```rust
pub struct ToolResultMsg {
    pub tool_call_id: String,
    pub result: String,
    pub is_error: bool,
    pub images: Vec<ImageData>,        // Vision models
}
```

### ToolCallStatus (Runtime tool state)
```rust
pub struct ToolCallStatus {
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub confirm_message: String,
    pub status: ToolExecStatus,        // PendingConfirm | Executing | Done | Rejected | Failed
}
```

---

## 10. Tool Confirmation Flow

**File**: `/src/command/chat/app/tool_executor.rs`

### Confirmation Process
1. Tool defines `requires_confirmation() -> bool`
2. Tool provides `confirmation_message(&str) -> String`
3. TUI displays prompt with Accept/Reject options
4. User input triggers tool execution or rejection
5. Result sent via `tool_result_tx: mpsc::SyncSender<ToolResultMsg>`

### Example: ShellTool
```rust
pub struct ShellTool {
    pub manager: Arc<BackgroundManager>,
}

impl Tool for ShellTool {
    fn requires_confirmation(&self) -> bool {
        true  // Always ask for confirmation
    }
    
    fn confirmation_message(&self, arguments: &str) -> String {
        format!("Execute shell command: {}", ...)
    }
    
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        // Implementation
    }
}
```

---

## 11. Worktree Tool Pattern

**File**: `/src/command/chat/tools/worktree.rs`

### Worktree State Sharing
```rust
pub struct WorktreeState {
    session: Mutex<Option<WorktreeSession>>,
}

pub struct WorktreeSession {
    pub original_cwd: PathBuf,
    pub worktree_path: PathBuf,
    pub branch: String,
    pub original_head_commit: Option<String>,
}
```

### Pattern for Stateful Tools
- Create shared State struct protected by Mutex
- Register in ToolRegistry during initialization
- Pass to tools needing state
- Tools query/update state via methods
- State survives across tool invocations

---

## 12. File Structure Overview

### Core Paths
```
/src/command/chat/
├── agent.rs                    # Main agent loop
├── agent_config.rs             # Config structures
├── api.rs                       # OpenAI API calls
├── skill.rs                     # Skill loading
├── storage.rs                   # Data structures
├── hook.rs                      # Hook system
├── permission.rs               # Permission checks
├── tools/
│   ├── mod.rs                  # Tool trait + registry
│   ├── agent.rs                # Sub-agent tool
│   ├── background.rs           # Background task management
│   ├── shell.rs                # Shell command tool
│   ├── file/
│   │   ├── mod.rs
│   │   ├── read.rs             # Read tool
│   │   ├── write.rs            # Write tool
│   │   ├── edit.rs             # Edit tool
│   │   └── glob.rs             # Glob tool
│   ├── grep.rs                 # Grep tool
│   ├── skill.rs                # LoadSkill tool
│   ├── plan.rs                 # Plan mode tools
│   ├── task/
│   │   ├── mod.rs
│   │   ├── task_manager.rs
│   │   ├── entity.rs
│   │   └── task_tool.rs
│   ├── todo/
│   │   ├── mod.rs
│   │   ├── todo_manager.rs
│   │   ├── entity.rs
│   │   ├── todo_read_tool.rs
│   │   └── todo_write_tool.rs
│   ├── worktree.rs             # Worktree isolation
│   ├── computer_use/           # GUI interaction
│   ├── web_fetch.rs
│   ├── web_search.rs
│   ├── browser.rs
│   ├── ask.rs
│   ├── hook.rs
│   └── classification.rs
└── app/
    ├── mod.rs
    ├── types.rs                # Message types
    ├── tool_executor.rs        # Tool execution state
    ├── chat_app.rs
    └── ...
```

---

## 13. Implementation Pattern for New Tools

### Basic Structure
```rust
// 1. Define parameter struct
#[derive(Deserialize, JsonSchema)]
struct MyToolParams {
    param1: String,
    #[serde(default)]
    param2: Option<i32>,
}

// 2. Implement Tool trait
pub struct MyTool {
    // Dependencies as needed
}

impl Tool for MyTool {
    fn name(&self) -> &str { "MyTool" }
    
    fn description(&self) -> &str { "..." }
    
    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<MyToolParams>()
    }
    
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: MyToolParams = parse_tool_args(arguments)?;
        // Implementation
        ToolResult {
            output: "result".to_string(),
            is_error: false,
            images: vec![],
        }
    }
    
    fn requires_confirmation(&self) -> bool {
        false  // or true if user approval needed
    }
}

// 3. Register in ToolRegistry::new()
registry.register(Box::new(MyTool { ... }));
```

---

## 14. Agent Team Tool Considerations

### For Implementation

1. **State Management**:
   - Create `AgentTeamState` struct (similar to WorktreeState)
   - Store team members, assignments, coordination state
   - Use Mutex for thread-safe access

2. **Sub-agent Spawning**:
   - Similar to AgentTool but multiple instances
   - Can run in background or foreground
   - Each gets subset of tools
   - Coordinate via shared state

3. **Tool Filtering**:
   - Use `disabled_tools` pattern for each team member role
   - Different roles get different tool subsets
   - Prevent certain operations per role

4. **Communication**:
   - Agent team may need internal messaging
   - Results from sub-agents aggregated
   - Synchronization via Arc<Mutex<>> shared state

5. **Persistence** (Optional):
   - Similar to TaskManager pattern
   - Store team definitions, assignments
   - Recover team state across sessions

6. **Complexity Trade-offs**:
   - Foreground: synchronous, no threading complexity, blocks main loop
   - Background: asynchronous, uses thread pool, needs coordination
   - Hybrid: spawns background sub-agents but waits for key milestones

---

## Summary

The j-cli project uses a **modular, trait-based tool system** with:
- **Tool trait** for uniform interface (execute, requires_confirmation, parameters_schema)
- **ToolRegistry** for central management and filtering
- **Agent loop** that handles multi-round tool calling with streaming
- **Shared state** via Arc<Mutex<>> for cross-thread coordination
- **Background task management** for async operations
- **Permission/Hook system** for security and extensibility

The existing **AgentTool** demonstrates sub-agent spawning patterns, which can be extended for "agent team" with:
- Multiple sub-agents with role-based tool filtering
- Shared team state management
- Coordination through notifications or shared buffers
- Both foreground (wait-for-results) and background (fire-and-forget) modes

