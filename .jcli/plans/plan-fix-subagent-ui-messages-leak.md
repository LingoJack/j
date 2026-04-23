# 修复 SubAgent/Teammate 中间消息泄漏到 Main Agent LLM 上下文

## 问题根因

SubAgent 和 Teammate 通过 `push_ui` 向共享的 `ui_messages: Arc<Mutex<Vec<ChatMessage>>>` 推送中间进度消息（`<Name> [调用工具 X]`、`<Name> 文本...`）。UI 线程的 `poll_stream_actions` 增量同步 `ui_messages → session.messages`，而 `build_api_messages()` 从 `session.messages` 读取并经 window 选择后发给 Main Agent LLM。

结果：SubAgent/Teammate 的"仅显示"消息被错误地持久化到 session 并注入 LLM 上下文，浪费 tokens 且可能干扰推理。

## 设计原则

1. **最小改动原则**：不改变 `ChatMessage` 的序列化格式（向后兼容，不破坏已有 session 文件）
2. **单一职责**：消息的"来源/显示性质"信息不应耦合在 `ChatMessage` 结构体上，而应在推送管道层面解决
3. **统一处理**：SubAgent 和 Teammate 都有同样的问题，修复方案应统一

## 修复方案：管道过滤（推荐）

**核心思路**：不在 `ChatMessage` 上加字段，而是在 `ui_messages` 的消费者侧（`poll_stream_actions`）加过滤逻辑，基于消息内容的 `<Name>` 前缀模式跳过 SubAgent/Teammate 的显示消息。

但这依赖字符串模式，脆弱。更好的做法是：

### 方案：引入 `display_only` 字段（用 `#[serde(skip)]` 确保向后兼容）

在 `ChatMessage` 增加 `display_only: bool` 字段，用 `#[serde(skip)]` 标注（同 `images` 字段的处理方式）。这样：
- 序列化/反序列化完全不受影响（老数据读到新结构体时自动为 `false`）
- SubAgent/Teammate 推送的显示消息标记为 `true`
- `poll_stream_actions` 同步到 `session.messages` 时跳过这些消息
- `persist_new_messages` 也不会持久化它们（因为它们根本不会进入 `session.messages`）

### 改动清单

#### 1. `src/command/chat/storage/types.rs` — ChatMessage 增加 display_only 字段

```rust
pub struct ChatMessage {
    pub role: MessageRole,
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip)]
    pub images: Option<Vec<ImageData>>,
    /// 仅 TUI 显示，不同步到 session.messages 也不持久化
    #[serde(skip)]
    #[serde(default)]
    pub display_only: bool,
}
```

更新 `ChatMessage::text()` 和 `ChatMessage::with_images()` 构造函数（`display_only: false`）。

新增 `ChatMessage::display_only(role, content)` 便利构造函数。

更新 `display_type()` 无需变动（`display_only` 不影响渲染逻辑）。

#### 2. `src/command/chat/app/stream_poll.rs` — 同步时跳过 display_only 消息

```rust
// poll_stream_actions 中
for msg in &shared[self.ui_messages_read_offset..] {
    if !msg.display_only {
        self.state.session.messages.push(msg.clone());
    }
}
```

注意：`ui_messages_read_offset` 仍然按 `shared.len()` 推进（即使某些消息被跳过），确保不会重复处理。

#### 3. `src/command/chat/tools/sub_agent.rs` — SubAgent push_ui 标记 display_only

`run_sub_agent_loop` 中的 3 处 `push_ui` 调用改为 `display_only: true`：

- 第 512-515 行：`push_ui(ChatMessage::text(...))` → 使用 `display_only` 构造
- 第 548-551 行：`push_ui(ChatMessage::text(...))` → 使用 `display_only` 构造
- 第 595-597 行：`push_ui(ChatMessage::text(...))` → 使用 `display_only` 构造

抽取一个 `push_display_only` 闭包替代原有的 `push_ui` 闭包。

#### 4. `src/command/chat/teammate/teammate_loop.rs` — Teammate 同样标记 display_only

所有通过 `manager.ui_messages.lock().push(ChatMessage::text(...))` 推送的显示消息：
- 第 229-233 行：文字回复
- 第 369-375 行：工具调用名
- 第 430-433 行：完成通知

全部改为 `display_only: true`。

#### 5. 测试更新

- `storage/types.rs` 中的现有测试如果手动构造 `ChatMessage` 需补充 `display_only: false`
- 新增单元测试：验证 `display_only=true` 的消息不被同步到 `session.messages`
- 新增单元测试：验证 `display_only` 字段不参与序列化/反序列化（向后兼容）

### 不需要改动的文件

- `agent/tool_processor.rs` 中的 `push_ui` 宏 — 主 Agent 的 push_ui 仍然是 `display_only: false`，无需改动
- `agent/agent_loop.rs` — 无需改动
- `agent/window.rs` — 无需改动（这些消息根本不会进入 `session.messages`）
- `agent/compact.rs` — 无需改动
- `agent/message_compression.rs` — 无需改动
- 所有序列化/持久化代码 — `#[serde(skip)]` 确保 `display_only` 不参与序列化

## 实施顺序

1. `storage/types.rs` — 增加 `display_only` 字段 + 便利构造函数
2. 修复编译错误（所有手动构造 `ChatMessage` 的地方补充 `display_only: false`）
3. `app/stream_poll.rs` — 同步时过滤
4. `tools/sub_agent.rs` — 标记 display_only
5. `teammate/teammate_loop.rs` — 标记 display_only
6. `cargo fmt && cargo clippy && cargo test` 验证
