# j-cli Project Exploration - Complete Index

**Date**: April 10, 2026  
**Project**: j-cli at `/Users/jacklingo/dev_custom/j/`  
**Purpose**: Understand architecture for implementing "agent team" tool

---

## 📄 Documents Generated

### 1. **ARCHITECTURE_EXPLORATION.md** (720 lines)
Comprehensive deep-dive covering:
- Tool trait definition and key methods
- Tool registration system
- Agent loop logic (main flow)
- Existing agent tool (sub-agent spawning)
- Background task management
- Task management system
- Skill system loading
- Key data structures
- Cross-thread communication patterns
- Tool confirmation flows
- Worktree tool pattern
- Complete file structure overview
- Implementation patterns for new tools
- Agent team tool considerations

**Read This For**: Complete understanding of all systems involved

### 2. **QUICK_REFERENCE.md** (203 lines)
Quick lookup guide including:
- Key files with line numbers
- Tool implementation checklist
- Sub-agent spawning pattern code
- State sharing pattern template
- Key constants and defaults
- Token efficiency patterns
- Safe mutex helper usage
- Sub-agent vs main loop comparison table
- Proposed agent team structure

**Read This For**: Quick lookups during implementation

---

## 🔑 Critical Files to Reference

### Tool System (Foundations)
```
/src/command/chat/tools/mod.rs           ← Tool trait, registry, helpers
/src/command/chat/tools/agent.rs         ← Sub-agent template (most similar)
/src/command/chat/tools/background.rs    ← Background task management
/src/command/chat/tools/worktree.rs      ← Stateful tool example
```

### Agent Execution
```
/src/command/chat/agent.rs               ← Main agent loop (streaming)
/src/command/chat/agent_config.rs        ← Configuration structures
/src/command/chat/app/types.rs           ← Message types for communication
/src/command/chat/app/tool_executor.rs   ← Tool confirmation & execution
```

### Data & Storage
```
/src/command/chat/storage.rs             ← ChatMessage, ToolCallItem, ModelProvider
/src/command/chat/tools/task/task_manager.rs  ← Persistent task storage
```

### Support Systems
```
/src/command/chat/skill.rs               ← Skill loading
/src/command/chat/permission.rs          ← Permission checking
/src/command/chat/hook.rs                ← Hook system
/src/util/safe_lock.rs                   ← Safe mutex wrapper
```

---

## 🎯 Key Insights for Agent Team Tool

### 1. Tool Trait is Simple & Powerful
```rust
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters_schema(&self) -> Value;
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult;
    fn requires_confirmation(&self) -> bool { false }
    fn confirmation_message(&self, arguments: &str) -> String { ... }
}
```
- ✓ Single execute method (async via tokio)
- ✓ Cancellation support via AtomicBool
- ✓ Optional confirmation mechanism
- ✓ Schema-driven parameters

### 2. Sub-agent Spawning Pattern Already Exists
The `AgentTool` (/src/command/chat/tools/agent.rs) shows:
- ✓ How to spawn single sub-agents
- ✓ Both foreground (blocking) and background (async) modes
- ✓ Permission checking without UI confirmation
- ✓ Tool filtering (disabled_tools)
- ✓ Result aggregation

**For agent team**: Extend this to spawn multiple agents in parallel with role-based tool filtering.

### 3. State Sharing Pattern is Established
Examples:
- `BackgroundManager` - Task tracking
- `PlanModeState` - Mode flag + file path
- `WorktreeState` - Session info
- `TodoManager` - Todo list

**For agent team**: Use `Arc<Mutex<AgentTeamState>>` for member tracking, assignments, coordination.

### 4. Background Task Management is Sophisticated
```rust
pub fn spawn_command(&self) -> (String, Arc<Mutex<String>>)
```
- Returns task_id + shared output buffer
- Worker thread writes to buffer real-time
- Agent loop polls drain_notifications()
- Main loop injects completion notifications

**For agent team**: Can reuse for parallel sub-agent execution with real-time monitoring.

### 5. Tool Confirmation is Optional
- `requires_confirmation()` → bool
- Only calls like shell need it
- File/read operations skip
- Sub-agents use permission rules instead

**For agent team**: Each role can have different confirmation requirements.

---

## 📊 Architecture Patterns Used

### Pattern 1: Trait-Based Abstraction
- All tools implement same trait
- Registry holds `Vec<Box<dyn Tool>>`
- LLM tools generated from registry
- New tools added via registration

### Pattern 2: Arc<Mutex<T>> for Shared State
- Used for all cross-thread data
- Mutex protects from races
- Arc enables cloning across threads
- Common in Rust async

### Pattern 3: Channel-Based Communication
- `mpsc::Sender<StreamMsg>` - Agent → UI
- `mpsc::Receiver<ToolResultMsg>` - UI → Agent
- `mpsc::SyncSender` for blocking operations
- Decouples threads cleanly

### Pattern 4: Permission Checks Instead of User Confirmation
- Headless sub-agents can't ask users
- Use `jcli_config.is_denied()` / `is_allowed()`
- Rules-based allow/deny lists
- Supports auto-approval for known tasks

### Pattern 5: Output Buffering for Real-Time Progress
```rust
pub output_buffer: Arc<Mutex<String>>
```
- Caller (UI/main) provides shared buffer
- Worker writes incrementally
- Caller reads for progress updates
- Avoids batching results

---

## 🚀 Quick Start for Agent Team Implementation

### Phase 1: Define Structures
```rust
// agent_team.rs or extend agent.rs
pub struct AgentTeamTool {
    // All the dependencies from AgentTool
    // PLUS:
    team_state: Arc<AgentTeamState>,
}

pub struct AgentTeamState {
    inner: Mutex<AgentTeamInner>,
}

#[derive(Clone)]
struct TeamMember {
    id: String,
    role: String,            // "researcher", "implementer", "reviewer"
    disabled_tools: Vec<String>,
    status: MemberStatus,
}

enum MemberStatus {
    Idle,
    Working,
    Complete(String),        // Final output
}
```

### Phase 2: Implement Tool Trait
```rust
impl Tool for AgentTeamTool {
    fn name(&self) -> &str { "AgentTeam" }
    fn description(&self) -> &str { /* describe team feature */ }
    fn parameters_schema(&self) -> Value {
        // Team definition + main task
    }
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
        // Parse team definition
        // Spawn multiple sub-agents
        // Collect results
        // Aggregate output
    }
}
```

### Phase 3: Spawn Sub-agents with Role Filtering
```rust
// Reuse run_headless_agent_loop but:
// 1. Call it multiple times (once per team member)
// 2. Filter tools per role using disabled_tools
// 3. Spawn as background tasks or parallel threads
// 4. Collect outputs
```

### Phase 4: Register in ToolRegistry
```rust
// In /src/command/chat/tools/mod.rs, ToolRegistry::new()
registry.register(Box::new(AgentTeamTool { ... }));
```

---

## ⚠️ Key Considerations

### Threading & Concurrency
- ✓ Use `safe_lock()` helper from `/src/util/safe_lock.rs`
- ✓ Avoid nested locks (deadlock risk)
- ✓ Use Arc<Atomic*> for simple flags
- ✓ Mutex for complex state

### Performance
- ✓ Each sub-agent = new tokio runtime (or share one)
- ✓ Multiple LLM API calls in parallel
- ✓ Token usage compounds per agent
- ✓ Consider queue throttling

### Cancellation
- ✓ AtomicBool is checked, not guaranteed
- ✓ Sub-agents respect it
- ✓ Background threads may continue briefly
- ✓ Main loop can still timeout

### Context Management
- ✓ Sub-agents build fresh message histories
- ✓ Each has independent token budget
- ✓ Results aggregated in main agent context
- ✓ Watch total token consumption

### Testing
- ✓ All tools should be testable
- ✓ Mock shared state for unit tests
- ✓ Integration tests with real agent loop
- ✓ Test both foreground and background modes

---

## 📚 Reading Order Recommendation

1. **Start with**: QUICK_REFERENCE.md (3 min)
   - Get file locations and basic patterns

2. **Read**: /src/command/chat/tools/agent.rs (30 min)
   - Understand existing sub-agent implementation
   - See foreground vs background spawning
   - Study permission checking

3. **Read**: /src/command/chat/agent.rs (30 min, focus on lines 67-400)
   - Understand main agent loop flow
   - See tool call processing
   - Learn message handling

4. **Skim**: ARCHITECTURE_EXPLORATION.md (20 min)
   - Full context on all systems
   - Use as reference for specifics

5. **Reference**: /src/command/chat/tools/worktree.rs (10 min)
   - See state management pattern
   - Learn Mutex protection approach

---

## 🔍 Specific Code Locations

### Where to find examples of:

| Need | Location | Lines |
|------|----------|-------|
| Tool implementation | `/src/command/chat/tools/file/read.rs` | full file |
| Parameter parsing | `/src/command/chat/tools/mod.rs` | 44-51 |
| Schema generation | `/src/command/chat/tools/mod.rs` | 33-42 |
| Sub-agent spawning | `/src/command/chat/tools/agent.rs` | 123-182 |
| Headless agent loop | `/src/command/chat/tools/agent.rs` | 196-400 |
| Permission checks | `/src/command/chat/tools/agent.rs` | 327-366 |
| Background tasks | `/src/command/chat/tools/background.rs` | 64-87 |
| State sharing | `/src/command/chat/tools/worktree.rs` | 25-55 |
| Tool registry | `/src/command/chat/tools/mod.rs` | 108-184 |
| Main loop | `/src/command/chat/agent.rs` | 30-400+ |

---

## ✅ Implementation Checklist

- [ ] Define `AgentTeamTool` struct with all dependencies
- [ ] Create `AgentTeamState` for tracking team members
- [ ] Define parameter struct with schemars
- [ ] Implement Tool trait
  - [ ] name()
  - [ ] description()
  - [ ] parameters_schema()
  - [ ] execute()
  - [ ] requires_confirmation() (false)
- [ ] Parse team definition from parameters
- [ ] Implement role-based tool filtering
- [ ] Spawn sub-agents (parallel or sequential)
  - [ ] Each gets filtered tool registry
  - [ ] Reuse run_headless_agent_loop()
- [ ] Collect and aggregate results
- [ ] Handle cancellation token
- [ ] Register in ToolRegistry::new()
- [ ] Test foreground mode (blocking)
- [ ] Test background mode (async)
- [ ] Test tool filtering per role
- [ ] Test result aggregation
- [ ] Document in tool description

---

## 📝 Next Steps

1. Review ARCHITECTURE_EXPLORATION.md section 4 (Agent Tool) - 15 min
2. Study run_headless_agent_loop() implementation - 20 min
3. Design AgentTeamTool parameters and output format
4. Implement structure and Tool trait
5. Implement team spawning logic
6. Add unit tests
7. Integration test with main agent loop

---

