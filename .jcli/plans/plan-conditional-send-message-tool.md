# Plan: 条件性注册 SendMessage / WaitForMessage 工具

## 问题

当前 Main Agent 在 `ChatApp::new()` 中无条件注册 `SendMessageTool`，即使没有任何 teammate 存在，LLM 也能看到并调用该工具。`SendMessage` 和 `WaitForMessage` 对 Main Agent 来说应该是条件性的——只有存在活跃 teammate 时才应暴露给 LLM。

用户选择方案 B：每 tool round 动态刷新工具列表，实现 teammate 创建后立即可用这两个工具。

## 架构分析

### Main Agent 的消息流向

- `SendMessageTool.broadcast` → 推入 `context_messages`（完整消息） + `main_agent_inbox`（唤醒信号）
- `context_messages` → UI `poll_stream_actions` drain 到 `session.messages`
- `main_agent_inbox` → agent loop `drain_pending_user_messages` 消费（检测唤醒信号）

### WaitForMessage 的差异

| Agent 类型 | Inbox | 消费模式 | 与谁竞争 |
|-----------|-------|---------|---------|
| Teammate | `pending_user_messages`（每 teammate 独有） | drain | teammate_loop drain（每轮开始一次性，之后不 drain） |
| Main Agent | `context_messages` | **peek**（只读不取） | UI `poll_stream_actions` drain |

Main Agent 的 `WaitForMessageTool` 必须采用 peek 模式，否则会与 UI 的 drain 竞争导致消息丢失。

## 设计方案：Tool trait `is_available` + 每 Tool Round 动态刷新 + Main Agent 专用 WaitForMessage

### 核心改动

1. `Tool` trait 增加 `is_available()` 方法（默认 `true`）
2. `SendMessageTool` 和 `MainWaitForMessageTool` 覆写该方法
3. 将 `Arc<ToolRegistry>` 等传入 `AgentLoopSharedState`
4. `run_main_agent_loop` 每轮动态调用 `to_llm_tools_filtered`
5. 新增 `MainWaitForMessageTool`（peek 模式）注册给 Main Agent
6. 原有 `WaitForMessageTool`（drain 模式）继续给 teammate 使用

### 具体改动（共 8 个文件）

#### 1. `Tool` trait 增加 `is_available`（`tools/definition.rs`）

```rust
pub trait Tool: Send + Sync {
    // ... 现有方法 ...

    /// 工具是否当前可用（默认 true）。
    /// 返回 false 时，该工具不会出现在 LLM 的工具列表和工具摘要中。
    fn is_available(&self) -> bool {
        true
    }
}
```

#### 2. `ToolRegistry` 过滤逻辑增加 `is_available`（`tools/definition.rs`）

`to_llm_tools_filtered`、`build_tools_summary`、`execute` 三处增加过滤。

#### 3. `SendMessageTool` 覆写 `is_available`（`tools/send_message.rs`）

```rust
fn is_available(&self) -> bool {
    self.teammate_manager
        .lock()
        .map(|m| m.has_active_teammates())
        .unwrap_or(false)
}
```

#### 4. `TeammateManager` 增加 `has_active_teammates`（`teammate/manager.rs`）

```rust
pub fn has_active_teammates(&self) -> bool {
    self.teammates.iter().any(|(_, h)| h.running())
}
```

#### 5. 新增 `MainWaitForMessageTool`（`tools/main_wait_for_message.rs`）

采用 peek 模式，从 `context_messages` 读取新消息，使用 `last_seen_len: AtomicUsize` 记录已读位置：

```rust
pub struct MainWaitForMessageTool {
    /// Main Agent 的 context_messages（与 TeammateManager 共享）
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    /// Teammate 管理器（用于 is_available 检查）
    pub teammate_manager: Arc<Mutex<TeammateManager>>,
    /// 已读取位置（peek 模式：只读不取）
    pub last_seen_len: AtomicUsize,
}
```

**为什么不需要 `cancel_token`？**
- Main Agent 的 `cancel_token` 在 `spawn_agent_loop` 中创建，而 `MainWaitForMessageTool` 在 `ChatApp::new()` 时注册，生命周期不匹配
- Main Agent 的 `WaitForMessage` 只需要检查 `cancelled` 参数（已通过 `execute` 传入），以及超时检测

**peek 模式核心逻辑**：
```rust
fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult {
    // 轮询循环中：
    // 1. 读取 context_messages.len()
    // 2. 比较 last_seen_len，取增量切片
    // 3. 用 from/keyword 过滤增量消息
    // 4. 不匹配的消息更新 last_seen_len（跳过）
    // 5. 匹配的消息返回给 LLM
    // 注意：不 drain，UI 的 poll_stream_actions 照常消费
}
```

注意：工具名称仍为 `WaitForMessage`，与 teammate 版本一致（LLM 不区分实现差异）。

#### 6. `AgentLoopSharedState` 增加 3 个字段（`agent/config.rs`）

```rust
pub struct AgentLoopSharedState {
    // ... 现有字段 ...
    pub tool_registry: Arc<ToolRegistry>,
    pub disabled_tools: Vec<String>,
    pub tools_enabled: bool,
}
```

#### 7. `spawn_agent_loop` + `MainAgentHandle::spawn`（`app/message.rs` + `app/agent_handle.rs`）

移除 tools snapshot，传递 `Arc<ToolRegistry>` 和相关字段到 `AgentLoopSharedState`。

#### 8. `run_main_agent_loop` 每轮动态获取 tools（`agent/agent_loop.rs`）

每轮 `'round` 开始时调用 `to_llm_tools_filtered`。

### 注册逻辑

**Main Agent（`ChatApp::new()`）**：
```rust
tool_registry.register(Box::new(MainWaitForMessageTool {
    context_messages: Arc::clone(&context_messages),
    teammate_manager: Arc::clone(&teammate_manager),
    last_seen_len: AtomicUsize::new(0),
}));
```

**Teammate（`TeammateTool::execute`）**：
```rust
child_registry.register(Box::new(WaitForMessageTool {
    pending_user_messages: Arc::clone(&pending_user_messages),
    cancel_token: cancel_token.clone(),
}));
```

### 影响范围

| 文件 | 改动 |
|------|------|
| `tools/definition.rs` | `Tool` trait 增加 `is_available`；`ToolRegistry` 三处增加过滤 |
| `tools/send_message.rs` | 覆写 `is_available` |
| `tools/main_wait_for_message.rs` | **新增**：Main Agent 专用 WaitForMessage（peek 模式） |
| `tools/wait_for_message.rs` | 无改动（teammate 版本） |
| `teammate/manager.rs` | 增加 `has_active_teammates` 方法 |
| `agent/config.rs` | `AgentLoopSharedState` 增加 3 个字段 |
| `app/message.rs` | `spawn_agent_loop` 传递新字段，移除 tools snapshot |
| `app/agent_handle.rs` | `MainAgentHandle::spawn` 移除 `tools` 参数 |
| `agent/agent_loop.rs` | `run_main_agent_loop` 每轮动态获取 tools |

### 不需要改动的部分

- **TeammateTool**：始终可用（用于创建第一个 teammate）
- **oneshot.rs**：独立的 oneshot 模式，不涉及 teammate

### 安全性分析：Main Agent wait 时 teammate 权限请求

**问题**：Main Agent 执行 `WaitForMessage`（阻塞等待消息）时，teammate 触发需要审批的权限请求，会有问题吗？

**答案：没有问题！**

**原因**：
- Main Agent 的工具执行在**独立 worker 线程**中（`tool_executor.rs:139`：`std::thread::spawn`）
- TUI 线程继续正常运行，轮询 `PermissionQueue.pop_pending()`
- Teammate 的权限请求通过 `PermissionQueue.request_blocking()` 发给 TUI
- 三条线程独立运行：Main Agent worker、Teammate worker、TUI 主循环
- `WaitForMessageTool.execute` 只阻塞 **worker 线程**，不阻塞 **TUI 线程**

**流程示意**：
```
[Main Agent Worker]      [Teammate Worker]           [TUI Main Loop]
    |                          |                           |
WaitForMessage          需要执行 Bash               poll PermissionQueue
(阻塞轮询)              request_blocking            pop_pending
    |                   → 推入队列 →                  展示审批对话框
    |                          |                     用户点击 Allow/Reject
    |                   wait_for_decision                |
    |                          |                   resolve(approved)
    |                   ← 被唤醒 ←                     |
(继续等待消息)          执行/跳过 Bash                   |
```

**结论**：Main Agent 可以安全使用 `WaitForMessage`，不会阻塞权限审批流程。