# j-cli Architecture - Quick Reference

## Key Files for Agent Team Implementation

### Tool System Foundation
- **Tool Trait Definition**: `/src/command/chat/tools/mod.rs` (lines 76-91)
- **Tool Registry**: `/src/command/chat/tools/mod.rs` (lines 94-309)
- **Registration Example**: See how tools are registered in `ToolRegistry::new()` (lines 124-175)

### Agent Loop & Sub-agent Pattern
- **Main Agent Loop**: `/src/command/chat/agent.rs` (core loop logic, 67-400+)
- **Existing Agent Tool**: `/src/command/chat/tools/agent.rs` (template for sub-agents)
  - Foreground mode: Lines 164-181
  - Background mode: Lines 123-162
  - Headless loop: Lines 196-400
- **Agent Config**: `/src/command/chat/agent_config.rs` (config structures)

### Shared State Management
- **Background Manager**: `/src/command/chat/tools/background.rs` (task tracking)
- **Task Manager**: `/src/command/chat/tools/task/task_manager.rs` (persistent storage)
- **Worktree State Pattern**: `/src/command/chat/tools/worktree.rs` (stateful tool example)

### Data Structures
- **ChatMessage**: `/src/command/chat/storage.rs` (lines 106-121)
- **ToolCallItem**: `/src/command/chat/storage.rs` (lines 88-94)
- **StreamMsg Types**: `/src/command/chat/app/types.rs` (lines 6-18)
- **ToolResultMsg**: `/src/command/chat/app/types.rs` (lines 44-52)

### Permissions & Security
- **Permission System**: `/src/command/chat/permission.rs`
- **Used in sub-agents**: `/src/command/chat/tools/agent.rs` (lines 327-366)

---

## Tool Implementation Checklist

```rust
// 1. Define parameters with schemars
#[derive(Deserialize, JsonSchema)]
struct Params { ... }

// 2. Create tool struct
pub struct MyTool {
    // Fields for dependencies
}

// 3. Implement Tool trait
impl Tool for MyTool {
    fn name(&self) -> &str { "MyTool" }
    fn description(&self) -> &str { "..." }
    fn parameters_schema(&self) -> Value {
        schema_to_tool_params::<Params>()
    }
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        let params: Params = parse_tool_args(arguments)?;
        // ... implementation
        ToolResult { output: "...", is_error: false, images: vec![] }
    }
    fn requires_confirmation(&self) -> bool { false }
}

// 4. Register in ToolRegistry::new()
// In /src/command/chat/tools/mod.rs around line 124
registry.register(Box::new(MyTool { ... }));
```

---

## Sub-agent Spawning Pattern

```rust
// Foreground (synchronous, blocks)
let result = run_headless_agent_loop(
    provider, system_prompt, prompt, tools, registry, jcli_config, cancelled
);
// Returns String directly

// Background (asynchronous)
let (task_id, output_buffer) = background_manager.spawn_command(...);
std::thread::spawn(move || {
    let result = run_headless_agent_loop(...);
    {
        let mut buf = safe_lock(&output_buffer, "label");
        buf.push_str(&result);
    }
    bg_manager.complete_task(&task_id, "completed", result);
});
```

---

## State Sharing Pattern

```rust
// Define shared state (similar to WorktreeState)
pub struct MyState {
    state: Mutex<MyStateInner>,
}

impl MyState {
    pub fn new() -> Self {
        Self { state: Mutex::new(Default::default()) }
    }
    
    pub fn get_state(&self) -> Option<MyStateInner> {
        self.state.lock().ok()?.clone()
    }
    
    pub fn update_state(&self, updater: impl Fn(&mut MyStateInner)) {
        if let Ok(mut guard) = self.state.lock() {
            updater(&mut guard);
        }
    }
}

// Register in ToolRegistry.new()
let state = Arc::new(MyState::new());
// Pass to tools that need it
registry.register(Box::new(Tool1 { state: Arc::clone(&state) }));
registry.register(Box::new(Tool2 { state: Arc::clone(&state) }));
```

---

## Key Constants & Defaults

From `/src/command/chat/constants.rs` and other locations:
- **Max Tool Rounds**: 10 (configurable, subagent: 30)
- **Sub-agent Max Rounds**: 30
- **Default Context Tokens**: 20 max history messages
- **Plan Mode Whitelist**: Read, Glob, Grep, WebFetch, WebSearch, Ask, Compact, TodoRead, TodoWrite
- **Background Task Default Timeout**: 30,000ms

---

## Token-Efficient Patterns

1. **Compact Micro**: Replaces old tool results each round
2. **Compact Auto**: LLM summarization when token threshold exceeded
3. **Message History**: Limited to configurable max messages (default 20)
4. **Context Estimation**: `compact::estimate_tokens(&messages)`

---

## Important Mutex Helper

From `/src/util/safe_lock`:
```rust
pub fn safe_lock<T>(mutex: &Arc<Mutex<T>>, context: &str) -> MutexGuard<T>
```
Use this instead of `.lock().unwrap()` for better error handling!

---

## Sub-agent Headless Loop Key Differences

| Aspect | Main Agent Loop | Sub-agent Loop |
|--------|-----------------|----------------|
| Streaming | ✓ Streaming | ✗ Non-streaming |
| UI Interaction | ✓ Yes (StreamMsg) | ✗ No UI |
| Confirmation | User confirmation | Permission rules |
| Max Rounds | Configurable (~10) | 30 |
| Returns | Streams to UI | Final text |
| Cancellation | Via token | Via AtomicBool |

---

## For Agent Team Implementation

### Proposed Structure
```rust
pub struct AgentTeamTool {
    background_manager: Arc<BackgroundManager>,
    provider: Arc<Mutex<ModelProvider>>,
    system_prompt: Arc<Mutex<Option<String>>>,
    jcli_config: Arc<JcliConfig>,
    compact_config: CompactConfig,
    hook_manager: Arc<Mutex<HookManager>>,
    task_manager: Arc<TaskManager>,
    todo_manager: Arc<TodoManager>,
    disabled_tools: Arc<Vec<String>>,
    team_state: Arc<AgentTeamState>,  // New: track team members & status
}

pub struct AgentTeamState {
    inner: Mutex<AgentTeamInner>,
}

struct AgentTeamInner {
    team_members: Vec<TeamMember>,
    assignments: HashMap<String, Vec<TaskAssignment>>,
    coordination: Arc<Mutex<TeamCoordination>>,
}
```

### Key Methods Needed
1. `spawn_team_agents()` - Launch multiple sub-agents with role filtering
2. `collect_results()` - Gather outputs from all agents
3. `coordinate_team()` - Sync between agents (if needed)
4. `filter_tools_by_role()` - Different tools per role

---

