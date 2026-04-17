# Teammate System Investigation - Complete Analysis

## Executive Summary

The teammate system in this Rust CLI project implements a sophisticated multi-agent architecture using Tokio threads, Arc/Mutex synchronization, and inter-process message broadcasting. Key findings include:

- **Status Tracking**: Binary only (is_running: Arc<AtomicBool>) - "工作中" (Working) or "空闲" (Idle)
- **Lifecycle Bug**: Session exit doesn't call `manager.stop_all()`, leaving teammates running until idle timeout
- **Idle Detection**: 120 polling rounds (~2 minutes) with no pending work triggers exit
- **Communication**: Broadcast mechanism distributes messages to all teammates' `pending_user_messages` queues
- **UI Rendering**: Status only appears in system prompt, no dedicated UI panel

---

## File Structure Overview

### Core Teammate Files

```
src/command/chat/teammate/
├── mod.rs                 (5 lines)    - Module exports
├── manager.rs             (315 lines)  - TeammateManager, TeammateHandle, global file locks
└── teammate_loop.rs       (330 lines)  - Main agent loop, LLM integration, idle logic
```

### Integration Points

```
src/command/chat/tools/
├── create_teammate.rs     (319 lines)  - Thread spawning, initialization
└── send_message.rs        (106 lines)  - Inter-teammate communication

src/command/chat/app/chat_app.rs        - System prompt integration ({{.teammates}} placeholder)
src/command/chat/handler/chat.rs        - dump_teammates() for /dump command
src/command/chat/handler/tui_loop.rs    - Session exit (missing stop_all() call)
```

---

## Architecture Overview

### TeammateHandle Structure (Lines 70-88 in manager.rs)

```rust
pub struct TeammateHandle {
    pub name: String,
    pub role: String,
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub streaming_content: Arc<Mutex<String>>,
    pub cancel_token: CancellationToken,
    pub is_running: Arc<AtomicBool>,           // PRIMARY STATUS TRACKER
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
    pub system_prompt_snapshot: Arc<Mutex<String>>,
    pub messages_snapshot: Arc<Mutex<Vec<ChatMessage>>>,
}
```

Key methods:
- `running()`: Returns `is_running.load(Ordering::Relaxed)`
- `stop_with_message()`: Sets cancel_token + broadcasts completion message
- `wait_for_completion()`: Joins thread_handle

### TeammateManager Structure (Lines 147-314 in manager.rs)

```rust
pub struct TeammateManager {
    pub teammates: HashMap<String, TeammateHandle>,
    pub main_pending: Arc<Mutex<Vec<ChatMessage>>>,
    pub shared_messages: Arc<Mutex<Vec<ChatMessage>>>,
}
```

Key methods:
- `broadcast(msg, target)`: Injects message to all teammates' pending_user_messages (or specific target)
- `team_summary()`: Generates status for system prompt, format: `"<Name> (<Role>): 工作中/空闲"`
- `cleanup_finished()`: Removes completed teammates from HashMap
- `stop_all()`: **CRITICAL** - Sets cancel token for all teammates
- `stop_teammate()`: Gracefully stops specific teammate

### Thread-Local Context (Lines 12-52 in manager.rs)

```rust
thread_local! {
    static CURRENT_AGENT_NAME: RefCell<String> = RefCell::new("Main".to_string());
    static THREAD_CWD: RefCell<Option<PathBuf>> = RefCell::new(None);
}
```

Used for:
- Identifying sender in SendMessage tool
- Isolating working directories for worktree teammates
- Logging context

### Global File Locks (Lines 54-141 in manager.rs)

```rust
pub struct FileLockGuard {
    path: PathBuf,
}

static GLOBAL_FILE_LOCKS: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
```

Prevents concurrent file edits across teammate threads using RAII pattern.

---

## Teammate Lifecycle

### 1. Creation (create_teammate.rs, lines 83-313)

```
User calls CreateTeammate tool with parameters:
├─ name: String
├─ role: String
├─ prompt: String (initial task)
├─ worktree: bool (optional git isolation)
└─ inherit_permissions: bool (allow_all flag)

Flow:
1. Validate parameters (name not empty, not duplicate)
2. Create worktree if requested (optional)
3. Initialize resources:
   - pending_user_messages: Arc<Mutex<Vec<>>>
   - streaming_content: Arc<Mutex<String>>
   - cancel_token: CancellationToken
   - is_running: Arc<AtomicBool>::new(true)
   - system_prompt_snapshot, messages_snapshot
4. Build sub-registry with disabled tools:
   - CreateTeammate (no recursive spawning)
   - AgentTeam
   - Agent
   - SendMessage enabled
5. Spawn OS thread with run_teammate_loop
6. Register TeammateHandle in TeammateManager
7. Return success message with prompt excerpt
```

### 2. Execution (teammate_loop.rs, lines 44-279)

```
Main loop runs in spawned thread:
├─ Max 200 rounds OR until cancelled
├─ Each round:
│  ├─ Check cancel_token
│  ├─ Drain pending_user_messages (broadcast messages)
│  ├─ Call LLM with system prompt
│  ├─ Handle tool calls if FinishReason::ToolCalls
│  └─ Execute tools with permission checks
└─ Idle detection:
   ├─ If no tool calls:
   │  ├─ Check pending_user_messages
   │  ├─ If empty:
   │  │  ├─ idle_rounds += 1
   │  │  └─ If idle_rounds >= 120 (~2 min): BREAK
   │  │  └─ Else: sleep 1s checking for interrupts
   │  └─ If messages exist: reset idle_rounds
   └─ Always sync messages_snapshot after each round
```

### 3. Communication (send_message.rs, lines 62-105)

```
SendMessage tool:
1. Gets sender name from thread_local CURRENT_AGENT_NAME
2. Calls manager.broadcast(message, optional_target)
3. broadcast() injects ChatMessage to:
   - All teammates' pending_user_messages if no target
   - Specific teammate if @mentioned
4. Message appears in next round's LLM context

Message format in system prompt:
"<SenderName> This is the message content"
```

### 4. Termination (manager.rs, lines 256-294)

**Via Cancellation (Normal)**
- `stop_teammate(name)`: Sets cancel_token.cancel()
- `stop_all()`: Calls stop_token for each teammate
- Thread exits, `is_running.store(false, Ordering::Relaxed)`

**Via Idle Timeout**
- 120 rounds (~2 minutes) with no pending work
- Loop breaks naturally, thread exits
- `is_running` set to false

**Via Cleanup**
- `cleanup_finished()`: Joins completed threads, removes from HashMap
- Worktree cleaned up automatically (lines 239-245 in create_teammate.rs)

---

## Status Tracking

### Current Status Representation

**Binary Status** (is_running: Arc<AtomicBool>)
- `true`: Teammate thread is running
- `false`: Teammate thread has exited

**Status Display in team_summary()** (Lines 235-240 in manager.rs)
```rust
let status = if handle.running() {
    "工作中"  // Working
} else {
    "空闲"    // Idle
};
```

**Where Displayed**
- System prompt only via `{{.teammates}}` placeholder replacement
- No dedicated UI panel or status window
- Visible in `/dump` command via messages_snapshot

### Status Limitations

- **No intermediate states**: Can't distinguish between "working on task", "waiting for input", "error", "stuck"
- **Binary only**: Just running/not-running
- **No error tracking**: Failed tool execution doesn't change status
- **No completion signal**: No way to know if teammate succeeded or abandoned
- **Idle timeout not precise**: 120 polling rounds (~2 min) is approximate, not wall-clock time

---

## System Prompt Integration

### Team Summary Placeholder (chat_app.rs, lines 2119-2140)

```rust
let teammates_summary = self
    .teammate_manager
    .lock()
    .map(|m| m.team_summary())
    .unwrap_or_default();

prompt.replace("{{.teammates}}", &teammates_summary)
```

### System Prompt Template (teammate_loop.rs, lines 282-312)

```
{base_system_prompt}

## Your Identity
你是团队中的 **{name}**，角色: {role}。
你的名字是 `{name}`，在发送消息和被提及时使用这个名字。

{team_summary}

## Communication
- 使用 `SendMessage` 工具与其他 agent 通信
- 收到的广播消息以 `<AgentName>` 前缀出现在对话中
- 用 `@AgentName` 指定消息接收者（消息仍广播给所有人）
- 完成任务后，用 SendMessage 通知 @Main

## Rules
- 专注于你的角色职责，不要越界做其他角色的工作
- 如果需要其他 agent 的配合，通过 SendMessage 沟通
- 如果遇到文件编辑冲突（被其他 agent 锁定），等待后重试
```

---

## Critical Bug: Missing Session Exit Cleanup

### Location
`src/command/chat/handler/tui_loop.rs`, lines 604-645 (Session exit flow)

### Issue
Session exit doesn't call `manager.stop_all()`, leaving teammate threads running:

```rust
// tui_loop.rs - Current (BUGGY)
// ... no stop_all() call ...
```

### Consequence
- Teammates continue running after user exits chat session
- Threads eventually exit after ~2 minutes (idle timeout)
- Multiple concurrent chat sessions accumulate orphaned teammate threads
- System resources leak (threads, memory, LLM tokens)

### Fix
```rust
// Add before session cleanup:
if let Ok(mut manager) = self.teammate_manager.lock() {
    manager.stop_all();  // Gracefully cancel all teammates
    
    // Optional: wait for completion
    for (_, handle) in &manager.teammates {
        let _ = handle.wait_for_completion(Duration::from_secs(5));
    }
}
```

---

## Message Flow Diagram

```
┌─────────────┐
│  Teammate A │
└──────┬──────┘
       │
       │ pending_user_messages queue
       │
       ▼
┌──────────────────────────┐
│  TeammateManager         │
│  ├─ teammates HashMap    │
│  ├─ shared_messages      │
│  └─ main_pending         │
└──────────────────────────┘
       ▲
       │ broadcast(message)
       │ ├─ injects to all pending_user_messages
       │ └─ displays via shared_messages to TUI
       │
   ┌───┴───┐
   │       │
┌──┴──┐ ┌─┴───┐
│ B   │ │ C   │
└─────┘ └─────┘
```

---

## Idle Detection Logic

### Constants (teammate_loop.rs, line 62)
```rust
let max_consecutive_idle = 120;  // polling rounds
// Each round includes:
// - LLM call
// - Tool execution
// - 1s sleep during idle check
// ≈ ~2 minutes total
```

### Algorithm
```
for round in 0..max_rounds {
    if !pending_user_messages.is_empty() {
        idle_rounds = 0
        continue  // Process messages
    }
    
    choice = call_llm(...)
    
    if no_tool_calls {
        if pending_user_messages.is_empty() {
            idle_rounds += 1
            if idle_rounds >= 120 {
                break  // Exit loop
            }
            sleep_1s_with_interrupt_checks()
        } else {
            idle_rounds = 0
            continue  // Process new messages
        }
    } else {
        idle_rounds = 0
        execute_tools()  // Continue working
    }
}
```

---

## Thread Isolation Mechanisms

### 1. Thread-Local Agent Identity
```rust
set_current_agent_name(&teammate_name);  // In thread spawn
let sender = current_agent_name();       // In SendMessage tool
```

### 2. Thread-Local Working Directory (Worktree)
```rust
if let Some((ref wt_path, _)) = worktree_info {
    set_thread_cwd(wt_path);  // In thread spawn
}
```

### 3. Global File Locks
```rust
struct FileLockGuard { path: PathBuf }
static GLOBAL_FILE_LOCKS: Mutex<HashSet<PathBuf>>

// Before file edit:
acquire_lock(&path)  // Blocks other teammates
// After edit:
lock dropped  // RAII auto-releases
```

### 4. Separate Sub-Registry
Each teammate gets disabled tools:
- CreateTeammate (no recursive spawning)
- AgentTeam
- Agent
- SendMessage: ENABLED (only inter-teammate comms tool)

---

## Key Data Structures

### Message Snapshots
Used by `/dump` command to export teammate conversation state:
```rust
system_prompt_snapshot: Arc<Mutex<String>>
messages_snapshot: Arc<Mutex<Vec<ChatMessage>>>

// Updated each round (teammate_loop.rs, lines 91-96, 119, 261)
sync_messages(&messages) {
    if let Ok(mut snap) = messages_snapshot.lock() {
        *snap = messages.clone();
    }
}
```

### Pending User Messages Queue
Core communication mechanism:
```rust
pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>

// Each round (teammate_loop.rs, lines 111-116):
drain_broadcast_messages(&mut messages, &pending_user_messages) {
    if let Ok(mut pending) = pending.lock() {
        if !pending.is_empty() {
            messages.append(&mut *pending);
            return true  // had_new_messages
        }
    }
}
```

---

## Worktree Isolation

### Creation (create_teammate.rs, lines 122-145)
```rust
let worktree_info: Option<(PathBuf, String)> = if params.worktree {
    crate::command::chat::tools::worktree::create_agent_worktree(&name)?
}

// Creates: .jcli/worktrees/worktree-agent-{name}/
// Branch: worktree-agent-{name}
```

### Activation (create_teammate.rs, lines 202-213)
```rust
if let Some((ref wt_path, _)) = worktree_info {
    crate::command::chat::teammate::set_thread_cwd(wt_path);
}
```

### Cleanup (create_teammate.rs, lines 239-245)
```rust
if let Some((ref wt_path, ref branch)) = worktree_info {
    crate::command::chat::tools::worktree::remove_agent_worktree(wt_path, branch);
}
```

---

## Current Gaps and Limitations

1. **No Rich Status States**
   - Can't track: working-on-task, waiting-for-approval, error, stuck, completed-success, completed-failure
   - Only: running vs. not-running

2. **No Error Tracking**
   - Tool execution errors don't propagate to status
   - No error count, no error history

3. **No Completion Signal**
   - Teammates don't explicitly indicate "I'm done"
   - Must wait for idle timeout or manual stop

4. **No UI Panel**
   - Status only in system prompt text
   - No dedicated team status dashboard
   - No per-teammate status indicators

5. **Idle Timeout is Approximate**
   - 120 polling rounds ≠ precise wall-clock time
   - Varies based on LLM latency and tool execution time
   - No configurable timeout per teammate

6. **Session Exit Bug**
   - tui_loop.rs doesn't call manager.stop_all()
   - Teammates run until idle timeout
   - Potential resource leak in long-running sessions

7. **No Task Queuing**
   - Teammates must be explicitly told to do work
   - No job queue system
   - No work prioritization

8. **No Coordination Primitives**
   - No mutexes/semaphores between teammates
   - Only broadcast messaging (all-or-nothing)
   - No request-response or rendezvous patterns

---

## Integration Points Summary

| Component | File | Lines | Purpose |
|-----------|------|-------|---------|
| Manager | teammate/manager.rs | 147-314 | Core management, status tracking |
| Loop | teammate/teammate_loop.rs | 44-279 | Main agent loop, LLM integration |
| Create | tools/create_teammate.rs | 83-313 | Spawning and initialization |
| Message | tools/send_message.rs | 62-105 | Inter-teammate communication |
| Chat App | app/chat_app.rs | 2119-2140 | System prompt {{.teammates}} integration |
| Dump | handler/chat.rs | 918-952 | Export teammate snapshots |
| Exit | handler/tui_loop.rs | 604-645 | **MISSING stop_all() call** |

---

## Testing Scenarios

### Scenario 1: Basic Teammate Creation and Work
```
1. User creates Frontend teammate with prompt to create React app
2. Teammate spawns, reads initial prompt from pending_user_messages
3. Teammate calls LLM, gets tool calls (file create, etc.)
4. Teammate executes tools, collects results
5. Loop continues until idle timeout (~2 min)
6. Teammate exits, is_running set to false
```

### Scenario 2: Inter-Teammate Communication
```
1. Frontend teammate completes, calls SendMessage:
   "Done with UI, @Backend please integrate API"
2. SendMessage injects to Backend's pending_user_messages
3. Backend reads message in next loop iteration
4. Backend processes based on message content
```

### Scenario 3: Session Exit with Active Teammates
```
1. User exits chat (current session)
2. **BUG**: manager.stop_all() not called
3. Teammates continue running
4. After ~2 minutes of idle, teammates exit
5. Orphaned threads cleaned up
```

### Scenario 4: Worktree Isolation
```
1. User creates Backend teammate with worktree=true
2. Teammate gets dedicated git worktree: .jcli/worktrees/worktree-agent-Backend/
3. All file operations happen in worktree branch
4. Multiple teammates can edit overlapping files without conflicts
5. On teammate exit, worktree cleaned up automatically
```

---

## Recommendations for Next Steps

### High Priority
1. **Fix Session Exit Bug**: Add `manager.stop_all()` to tui_loop.rs session cleanup
2. **Test with Multiple Teammates**: Verify no resource leaks, proper cleanup

### Medium Priority
3. **Add Richer Status States**: Extend is_running to enum: Running, Idle, Completed, Error, Cancelled
4. **Create UI Status Panel**: Dedicated space showing all teammates with status
5. **Add Error Tracking**: Collect tool execution errors, expose via status

### Low Priority
6. **Configurable Idle Timeout**: Per-teammate or global setting
7. **Task Queuing System**: Formal job queue for teammates
8. **Coordination Primitives**: Mutexes, semaphores, barriers between teammates

---

## References

- Summary text ("{{.teammates}}"): teammate_loop.rs lines 282-312, manager.rs lines 226-246
- Status tracking: manager.rs lines 235-240
- Broadcast mechanism: manager.rs lines 170-224
- Idle detection: teammate_loop.rs lines 166-202
- Thread spawning: create_teammate.rs lines 198-263
- Session exit: tui_loop.rs lines 604-645
