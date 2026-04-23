# Bug: SubAgent 中间消息泄漏到 Main Agent LLM 上下文

## 严重程度
高（上下文污染，浪费 tokens，可能干扰 LLM 推理）

## 现象

调用 `Agent` 工具后，SubAgent 的所有中间消息（工具调用名、文本回复等）会出现在
`/dump-processed` 的 `messages.json` 中，意味着这些消息最终被发送给了 Main Agent 的 LLM。

示例（dump 中的片段）：
```json
{"role": "assistant", "content": "<探索_ExitPlanMode_实现> [调用工具 TodoWrite]"},
{"role": "assistant", "content": "<探索_ExitPlanMode_实现> [调用工具 Bash]"},
{"role": "assistant", "content": "<探索_ExitPlanMode_实现> [调用工具 Grep]"},
{"role": "assistant", "content": "<探索_ExitPlanMode_实现> 现在让我并行读取所有关键文件："},
{"role": "assistant", "content": "<探索_ExitPlanMode_实现> [已完成]"}
```

这些消息不应该出现在 Main Agent 的 LLM 上下文中。

## 根因

SubAgent 的 `push_ui` 推送消息到共享的 `ui_messages: Arc<Mutex<Vec<ChatMessage>>>`，
而 UI 线程的 `poll_stream_actions` 会增量同步 `ui_messages` 到 `session.messages`：

```rust
// stream_poll.rs:19-31
// ★ 从共享消息列表中检测新消息，增量追加到 session.messages
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
```

`ui_messages` 同时承担了两个职责：
1. **TUI 显示通道** — SubAgent 推送显示用消息（`<agent_name> [调用工具 X]`）
2. **持久化数据源** — UI 线程同步到 `session.messages`，后者被 `build_api_messages()` 读取

两者共享同一个 `Arc<Mutex<Vec<ChatMessage>>>`，导致 SubAgent 的"仅显示"消息被错误地
同步到 `session.messages`，最终进入 Main Agent 的 LLM 上下文。

## 涉及文件

| 文件 | 职责 |
|------|------|
| `src/command/chat/tools/sub_agent.rs` | `push_ui` 推送 `<agent_name>` 前缀消息 |
| `src/command/chat/tools/derived_shared.rs` | `DerivedAgentShared` 共享 `ui_messages` Arc |
| `src/command/chat/app/chat_app.rs:239` | `DerivedAgentShared` 和 `ChatApp` 共享同一 `ui_messages` Arc |
| `src/command/chat/app/stream_poll.rs:19-31` | 增量同步 `ui_messages → session.messages` |
| `src/command/chat/app/system_prompt.rs:90-98` | `build_api_messages()` 从 `session.messages` 读取 |
| `src/command/chat/agent/tool_processor.rs:52-56` | `push_ui` 宏定义 |

## 消息流

```
SubAgent 循环
  │
  ├─ push_ui("<name> [调用工具 X]")     ─→ ui_messages (Arc 共享)
  ├─ push_ui("<name> 文本回复...")       ─→ ui_messages (Arc 共享)
  ├─ push_ui("<name> [已完成]")          ─→ ui_messages (Arc 共享)
  │
  │                                       ┌─────────────────────────────┐
  │                                       │ UI 线程 poll_stream_actions │
  │                                       │                             │
  │                                       │ ui_messages ──sync──→       │
  │                                       │   session.messages.push()   │
  │                                       └──────────┬──────────────────┘
  │                                                  │
  │                                      ┌───────────▼──────────────────┐
  │                                      │ build_api_messages()         │
  │                                      │   → window::select_messages  │
  │                                      │   → micro_compact            │
  │                                      │   → sanitize_messages        │
  │                                      │                              │
  │                                      │ = 发给 Main Agent LLM 的上下文 │
  │                                      └──────────────────────────────┘
  │
  └─ return final_text ─→ ToolResult ─→ Main Agent LLM 上下文 (正确路径)
```

## 影响

1. **上下文污染**：SubAgent 的中间消息（工具调用名、进度文本）被注入 Main Agent 的 LLM 上下文，
   这些信息对 Main Agent 毫无价值，反而浪费 tokens 并可能干扰推理。

2. **micro_compact 不生效**：这些 SubAgent 消息以 `role: "assistant"` 形式存在，
   `micro_compact` 只压缩 `role: "tool"` 的消息，所以这些消息永远不会被清理。

3. **window 选择不剔除**：`window::select_messages` 按 Unit 类型（User / AssistantText / ToolGroup）
   分配配额，SubAgent 消息作为 `AssistantText` 占用了 25% 的配额，挤占 Main Agent 自己的回复空间。

## 修复方案

### 方案 A：在 `push_ui` 推送时标记消息为"仅显示"（推荐）

在 `ChatMessage` 中增加一个 `display_only: bool` 字段。SubAgent 推送的消息标记为 `true`。
`poll_stream_actions` 同步时跳过这些消息。

```rust
// storage.rs
pub struct ChatMessage {
    // ... 现有字段
    pub display_only: bool,  // true = 仅 TUI 显示，不同步到 session.messages
}

// stream_poll.rs
for msg in &shared[self.ui_messages_read_offset..] {
    if !msg.display_only {
        self.state.session.messages.push(msg.clone());
    }
}
```

**优点**：改动最小，向后兼容，不影响主 Agent 的 `push_ui` 行为。
**缺点**：`ChatMessage` 增加一个字段，序列化时需要处理。

### 方案 B：SubAgent 使用独立的 `ui_messages`

为 SubAgent 创建独立的 `ui_messages` Arc，不与 Main Agent 共享。
SubAgent 的消息推送到自己的 `ui_messages`，TUI 单独消费。

**优点**：彻底隔离，不存在泄漏风险。
**缺点**：改动较大，TUI 需要同时 poll 两个消息源。

### 方案 C：在 `push_ui` 中使用前缀过滤

SubAgent 推送的消息都有 `<agent_name>` 前缀。在 `poll_stream_actions` 中
按前缀过滤掉这些消息。

**优点**：不需要修改 `ChatMessage` 结构。
**缺点**：隐式约定（依赖命名规范），脆弱且不可靠。

## 推荐

**方案 A**，在 `ChatMessage` 增加 `display_only` 字段。明确、安全、改动最小。
