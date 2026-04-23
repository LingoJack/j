# 解耦 UI 消息与 LLM 上下文

## 一、核心观点

**需求合理，路径不合理。**

- Main Agent 看到 SubAgent/Teammate 的缩略动作 → **合理需求**
- 通过 `ui_messages` 实现 → **不合理路径**（UI 通道不应承担 context 注入职责）

## 二、目标架构

### 2.1 核心原则

1. **`display_messages`（原 ui_messages）**：仅用于 UI 显示，任何消息都**不**从这里进入 LLM context
2. **`context_messages`**：LLM context 的唯一数据源，只有**显式调用**才写入此通道

### 2.2 消息流

```
┌──────────────────────────────────────────────────────────────────────────┐
│                         Main Agent (agent_loop)                          │
│                                                                          │
│   messages (本地 Vec)                                                    │
│        │                                                                 │
│        ├─► LLM API（真实 context）                                       │
│        │                                                                 │
│        ├─► push_display(display_messages, msg)   → UI 显示              │
│        │                                                                 │
│        └─► push_context(context_messages, msg)   → session.messages     │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────┐
│                    SubAgent / Teammate (独立线程)                         │
│                                                                          │
│   本地 messages（独立 context）                                          │
│        │                                                                 │
│        ├─► push_display(display_messages, "<Name> 状态")                        │
│        │      → 仅 UI 显示（不进入任何 agent context）                   │
│        │                                                                 │
│        └─► push_context(context_messages, "<Name> 缩略动作")            │
│               → 显式注入 Main Agent context（session.messages）         │
│               → Main Agent 能看到子代理的缩略进度                        │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘

┌──────────────────────────────────────────────────────────────────────────┐
│                          TUI 线程 (poll_stream_actions)                  │
│                                                                          │
│   context_messages ──增量同步──► session.messages ──┬─► Main Agent ctx  │
│                                                     └─► UI 渲染        │
│                                                                          │
│   display_messages ──读取──► UI 渲染（可不同样式）                       │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

### 2.3 与当前架构的关键区别

| 维度 | 当前 | 新架构 |
|------|------|--------|
| SubAgent 缩略消息的注入方式 | 通过 `push_ui` → `ui_messages` → 无差别同步到 `session.messages` | 通过 `push_context` → `context_messages` → **显式**同步到 `session.messages` |
| SubAgent 纯状态消息（如 `[已完成]`）| 也通过 `push_ui` → 进入 context | 仅通过 `push_display` → 不进入 context |
| `ui_messages` / `display_messages` 的职责 | 同时承担 UI + context | **仅 UI** |
| context 同步的显式性 | 隐式（TUI 线程无差别同步） | 显式（只有 `push_context` 写入的消息才进入） |

---

## 三、详细设计

### 3.1 推送函数设计

```rust
// tool_processor.rs（或 context/message_channel.rs）

/// 向 display 通道推送消息（仅 UI 显示）
/// SubAgent/Teammate 的所有状态消息、Main Agent 的流式回复都走这里
pub fn push_display(display: &Arc<Mutex<Vec<ChatMessage>>>, msg: ChatMessage) { ... }

/// 向 context 通道推送消息（进入 Main Agent LLM context）
/// 只有需要进入 Main Agent context 的消息才走这里
pub fn push_context(context: &Arc<Mutex<Vec<ChatMessage>>>, msg: ChatMessage) { ... }

/// Main Agent 对话消息同时推送两个通道
pub fn push_main_agent_msg(
    display: &Arc<Mutex<Vec<ChatMessage>>>,
    context: &Arc<Mutex<Vec<ChatMessage>>>,
    msg: ChatMessage,
) {
    push_display(display, msg.clone());
    push_context(context, msg);
}
```

### 3.2 各调用点的推送策略

**原则**：所有需要 Main Agent 感知的消息都通过 `push_context` 显式注入，而非通过 UI 通道隐式泄漏。

| 来源 | 消息内容 | display | context | 说明 |
|------|----------|---------|---------|------|
| Main Agent | assistant text reply | ✓ | ✓ | 对话历史 |
| Main Agent | tool_call request | ✓ | ✓ | 对话历史 |
| Main Agent | tool result | ✓ | ✓ | 对话历史 |
| Main Agent | compact 后 summary | ✓ | ✓ | 替换 context |
| SubAgent | `<Name> 文本回复` | ✓ | ✓ | Main Agent 需看到 |
| SubAgent | `<Name> [调用工具 X]` | ✓ | ✓ | Main Agent 需看到 |
| SubAgent | `<Name> [已完成]` | ✓ | ✓ | Main Agent 需看到 |
| Teammate | `<Name> 文本回复` | ✓ | ✓ | Main Agent 需看到 |
| Teammate | `<Name> [调用工具 X]` | ✓ | ✓ | Main Agent 需看到 |
| Teammate | `<Name> [已完成工作]` | ✓ | ✓ | Main Agent 需看到 |

**关键区别**：与当前架构相比，消息**内容不变**，但**注入路径**从隐式变为显式。
SubAgent/Teammate 的消息通过 `push_context` **主动选择**注入 Main Agent context，
而非通过 `push_ui` → 无差别同步"被动泄漏"。

### 3.3 ChatApp / AgentLoopSharedState 结构

```rust
pub struct AgentLoopSharedState {
    // ...
    
    /// 仅 UI 显示的消息通道
    pub display_messages: Arc<Mutex<Vec<ChatMessage>>>,
    
    /// Main Agent LLM context 同步通道
    /// poll_stream_actions 从此读取并同步到 session.messages
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    
    // 删除 ui_messages
}

pub struct ChatApp {
    // ...
    
    /// display 通道（UI 显示）
    pub display_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub display_read_offset: usize,
    
    /// context 通道（LLM context 同步）
    pub context_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub context_read_offset: usize,
    
    // 删除 ui_messages, ui_messages_read_offset
}
```

### 3.4 poll_stream_actions 变更

```rust
fn poll_stream_actions(&mut self) -> Vec<Action> {
    // 1. 从 context_messages 增量同步到 session.messages
    {
        let shared = safe_lock(&self.context_messages, "poll_context");
        for msg in &shared[self.context_read_offset..] {
            self.state.session.messages.push(msg.clone());
        }
        self.context_read_offset = shared.len();
    }
    
    // 2. display_messages 仅用于 UI 渲染，不同步到 session.messages
    // UI 渲染时合并 session.messages + display_messages（增量部分）
    
    // ...
}
```

### 3.5 UI 渲染变更

UI 需要合并两个来源的消息进行渲染：

```rust
// 方案：按时间戳合并（display 和 context 消息交错显示）
// 或更简单：display 消息以不同样式渲染（如灰色/淡色）
```

具体实现待确认，但关键是：
- `session.messages` 只包含 context 消息
- `display_messages` 的增量部分用于 UI 渲染

---

## 四、ContextScope 枚举（保留）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextScope {
    #[default]
    UiAndMainAgentContext,
    Ui,
    SubagentContext,
    DerivedAgentContext,
    TeammateAgentContext,
}
```

`ContextScope` 主要用于：
1. **Session 恢复**：从 session.jsonl 恢复时识别消息来源
2. **UI 渲染**：按 scope 区分样式（如 SubagentContext 的消息用淡色）
3. **审计**：追踪消息属于哪个 agent

---

## 五、实现步骤

### Phase 1：扩展 ChatMessage（添加 ContextScope）
1. `storage/types.rs` 添加 `ContextScope` 枚举 + 字段

### Phase 2：引入双通道
1. `ChatApp` + `AgentLoopSharedState` 添加 `display_messages` / `context_messages`
2. 删除 `ui_messages`

### Phase 3：重构推送函数
1. `tool_processor.rs` 添加 `push_display` / `push_context` / `push_main_agent_msg`
2. 添加 `clear_display` / `sync_context_full`

### Phase 4：重构 agent_loop
1. Main Agent 消息使用 `push_main_agent_msg`
2. compact 后使用 `clear_display` + `sync_context_full`

### Phase 5：重构 SubAgent
1. 缩略消息使用 `push_context`（显式注入）
2. 状态消息使用 `push_display`（仅 UI）
3. `[已完成]` 仅 `push_display`

### Phase 6：重构 Teammate
1. 同 SubAgent 逻辑

### Phase 7：重构 poll_stream_actions
1. 从 `context_messages` 同步到 `session.messages`
2. 从 `display_messages` 读取用于 UI

### Phase 8：重构 UI 渲染
1. 合并 `session.messages` + `display_messages`
2. 按 `ContextScope` 区分样式

### Phase 9：验证
1. `cargo build` + `cargo clippy` + `cargo test`

---

## 六、影响范围

| 文件 | 变更 |
|------|------|
| `storage/types.rs` | 添加 `ContextScope` |
| `app/chat_app.rs` | 双通道 |
| `app/ui_state.rs` | display 缓存 |
| `app/stream_poll.rs` | 从双通道分别读取 |
| `agent/config.rs` | 双通道 |
| `agent/agent_loop.rs` | push_main_agent_msg |
| `agent/tool_processor.rs` | 新推送函数 |
| `tools/sub_agent.rs` | push_display + push_context |
| `teammate/teammate_loop.rs` | push_display + push_context |
| `ui/chat.rs` | 合并渲染 |
| `handler/chat.rs` | 初始化双通道 |
| `teammate/manager.rs` | 双通道 |
| `tools/derived_shared.rs` | 双通道（如有） |