# 修复 SubAgent/Teammate 中间消息泄漏 — 根本性重构方案

## 问题根因

`ui_messages: Arc<Mutex<Vec<ChatMessage>>>` 承担了三个职责但未做区分：
1. **Agent → TUI 显示通道**：SubAgent/Teammate 的中间进度消息（`<Name> [调用工具 X]`）
2. **Agent → session.messages 同步通道**：Main Agent 的正式消息（文本回复、工具调用、工具结果）
3. **Compact 后全量重建**：`clear_ui_messages` + `push_compact_tool_messages` → `StreamCompacted` 全量替换

UI 线程的 `poll_stream_actions` 无差别地将 `ui_messages` 增量同步到 `session.messages`，
导致 SubAgent/Teammate 的"仅显示"消息被持久化并注入 Main Agent 的 LLM 上下文。

## 设计思路：双通道分离

将 `ui_messages` 拆分为两个独立的通道：

| 通道 | 类型 | 写入方 | 消费方 | 内容 |
|------|------|--------|--------|------|
| `messages` | `Arc<Mutex<Vec<ChatMessage>>>` | Main Agent（tool_processor / agent_loop） | UI 线程 → session.messages + 渲染 | 正式消息（文本、工具调用、工具结果、compact 重建） |
| `display_events` | `Arc<Mutex<Vec<DisplayEvent>>>` | SubAgent、Teammate、WorkDone | UI 线程 → 仅渲染 | 进度通知（工具调用名、文本片段、完成标记） |

**核心原则**：
- `messages` 只包含 Main Agent 的正式消息，SubAgent/Teammate 不写入
- `display_events` 只包含显示用事件，不同步到 `session.messages`，不持久化，不进入 LLM 上下文
- 渲染层同时消费 `session.messages` 和 `display_events`，合并渲染

## DisplayEvent 设计

```rust
/// 仅用于 TUI 显示的事件（不持久化、不进入 LLM 上下文）
#[derive(Debug, Clone)]
pub enum DisplayEvent {
    /// Agent 文本输出片段
    AgentText {
        agent_name: String,
        content: String,
    },
    /// Agent 调用了工具（仅名称，无参数/结果）
    ToolCall {
        agent_name: String,
        tool_name: String,
    },
    /// Agent 完成工作
    AgentDone {
        agent_name: String,
    },
}
```

## 改动清单

### 1. 新增 `src/command/chat/storage/display_event.rs` — DisplayEvent 定义

新建文件，定义 `DisplayEvent` 枚举。不序列化，纯运行时类型。

### 2. 修改 `src/command/chat/app/chat_app.rs` — ChatApp 增加 display_events 字段

```rust
pub struct ChatApp {
    // 现有
    pub ui_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub ui_messages_read_offset: usize,
    // 新增
    pub display_events: Arc<Mutex<Vec<DisplayEvent>>>,
    pub display_events_read_offset: usize,
}
```

### 3. 修改 `src/command/chat/tools/derived_shared.rs` — DerivedSharedData 增加 display_events

```rust
pub struct DerivedSharedData {
    // 现有
    pub ui_messages: Arc<Mutex<Vec<ChatMessage>>>,
    // 新增
    pub display_events: Arc<Mutex<Vec<DisplayEvent>>>,
    // ...
}
```

### 4. 修改 `src/command/chat/tools/sub_agent.rs` — SubAgent 推送到 display_events

`run_sub_agent_loop` 中的 3 处 `push_ui` 改为推送到 `display_events`：

- `<name> 文本回复` → `DisplayEvent::AgentText { agent_name, content }`
- `<name> [调用工具 X]` → `DisplayEvent::ToolCall { agent_name, tool_name }`
- `<name> [已完成]` → `DisplayEvent::AgentDone { agent_name }`

### 5. 修改 `src/command/chat/teammate/manager.rs` — TeammateManager 增加 display_events

`broadcast` 方法中推送到 `display_events` 而非 `ui_messages`。

### 6. 修改 `src/command/chat/teammate/teammate_loop.rs` — Teammate 推送到 display_events

3 处直接 `ui_messages.lock().push()` 改为推送到 `display_events`。

### 7. 修改 `src/command/chat/tools/work_done.rs` — WorkDone 推送到 display_events

### 8. 修改 `src/command/chat/app/stream_poll.rs` — 分离消费逻辑

```rust
// 现有：从 ui_messages 增量同步到 session.messages（不变）
{
    let shared = safe_lock(&self.ui_messages, "poll::shared_msgs");
    let new_count = shared.len();
    if new_count > self.ui_messages_read_offset {
        for msg in &shared[self.ui_messages_read_offset..] {
            self.state.session.messages.push(msg.clone());
        }
        self.ui_messages_read_offset = new_count;
    }
}

// 新增：从 display_events 增量消费（仅渲染，不同步到 session.messages）
{
    let events = safe_lock(&self.display_events, "poll::display_events");
    let new_count = events.len();
    if new_count > self.display_events_read_offset {
        // 将 DisplayEvent 转换为 ChatMessage 推入一个专用的渲染列表
        // 或者直接在渲染层消费 display_events
        self.display_events_read_offset = new_count;
    }
}
```

### 9. 修改 `src/command/chat/render/cache.rs` — 渲染层消费 display_events

渲染层在遍历 `session.messages` 之后，追加渲染 `display_events` 中的新事件。

需要设计 `DisplayEvent → 渲染行` 的映射逻辑，复用现有的气泡样式但用不同颜色/缩进区分。

### 10. 修改 `src/command/chat/app/chat_app.rs` — StreamCompacted 路径

`StreamCompacted` 全量替换时，`display_events` 也需要清空（compact 后旧事件已过时）。

### 11. 删除 `src/command/chat/agent/message_compression.rs` 中的 SubAgent 消息压缩逻辑

`compress_other_agent_toolcalls` 函数原本用于压缩 SubAgent 的 `<Name> [调用工具 X]` 消息。
重构后这些消息不再进入 `session.messages`，此函数可以简化或删除。

但注意：Teammate 的广播消息（通过 `pending_user_messages` 接收的 `<Name> message`）仍然
会进入 Teammate 自己的 `messages`，这个压缩逻辑对 Teammate 仍然有用。
所以保留此函数，但注释说明它只用于 Teammate 内部消息压缩。

## 实施顺序

1. 新建 `storage/display_event.rs` — 定义 `DisplayEvent`
2. 修改 `storage/mod.rs` — 导出 `DisplayEvent`
3. 修改 `app/chat_app.rs` — 增加 `display_events` 字段 + 初始化
4. 修改 `tools/derived_shared.rs` — `DerivedSharedData` 增加 `display_events` + 传递
5. 修改 `tools/sub_agent.rs` — SubAgent 推送到 `display_events`
6. 修改 `teammate/manager.rs` — `TeammateManager` 增加 `display_events`
7. 修改 `teammate/teammate_loop.rs` — Teammate 推送到 `display_events`
8. 修改 `tools/work_done.rs` — WorkDone 推送到 `display_events`
9. 修改 `app/stream_poll.rs` — 分离消费逻辑
10. 修改 `render/cache.rs` — 渲染层消费 `display_events`
11. 修改 `app/chat_app.rs` — StreamCompacted 清空 `display_events`
12. `cargo fmt && cargo clippy && cargo test` 验证

## 不需要改动的文件

- `agent/tool_processor.rs` — Main Agent 的 `push_ui` 仍然推送到 `ui_messages`，不变
- `agent/agent_loop.rs` — 不变
- `agent/window.rs` — 不变（`session.messages` 中不再有 SubAgent 消息）
- `agent/compact.rs` — 不变
- `storage/types.rs` — `ChatMessage` 结构不变，向后兼容
- 所有序列化/持久化代码 — 不变
