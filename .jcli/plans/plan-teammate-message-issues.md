# Teammate 消息处理问题排查与修复计划

## 问题分析结果

### 核心问题：`main_agent_inbox` 与 `pending_user_messages` 共享同一存储

**关键代码**：
- `chat_app.rs:198`: `TeammateManager::new(Arc::clone(&pending_user_messages), ...)` — 将 `pending_user_messages` 传给 TeammateManager 作为其 `main_agent_inbox` 字段
- **结果**：`TeammateManager.main_agent_inbox` 和 `ChatApp.state.pending_user_messages` 是同一个 `Arc<Mutex<Vec<ChatMessage>>>`

---

### 问题 1: `<system_reminder>` 消息污染 LLM Context

**完整因果链**：

1. **teammate 发消息时**：`manager.rs:342` 的 `broadcast()` 往 `main_agent_inbox` 推送唤醒信号：
   ```rust
   pending.push(ChatMessage::text(MessageRole::User, "<system_reminder>A teammate has sent..."));
   ```

2. **agent loop 运行期间**：`agent_loop.rs:210` 调用 `drain_pending_user_messages(&mut messages, &pending_user_messages)`

3. **drain 所有 pending**：`tool_processor.rs:57` 把 **所有** pending 消息 append 到 agent 的本地 `messages`：
   ```rust
   messages.append(&mut *pending);  // ← 包括 <system_reminder>
   ```

4. **LLM context 被污染**：`<system_reminder>` 进入 LLM context，AI 可能会在回复中引用它

**时序问题**：
- 如果 main agent **空闲**（`is_loading=false`）：TUI loop Phase 2c 调用 `wake_from_teammate_inbox()` → `pending.clear()` → `<system_reminder>` 被清除，不污染 context
- 如果 main agent **运行中**（`is_loading=true`）：TUI loop 跳过 Phase 2c，下一轮 agent loop 的 `drain_pending_user_messages()` 会把 `<system_reminder>` drain 到 context

**结论**：只有 agent 运行中时 teammate 发消息，才会发生污染。

---

### 问题 2: `</Teammate@xxx>` 闭合标签问题

**现状分析**：

`broadcast()` 分别推送两种消息：
- `context_messages`: XML 包裹 `<Teammate@Counter1> xxx </Teammate@Counter1>`（第 368-371 行）
- `display_messages`: 纯文本 `xxx`（第 374 行）

**UI 渲染来源**：`build_message_lines_incremental()` 直接读取 `display_messages`，理论上不应看到 XML 标签。

**可能原因**：
1. 用户贴出的是 **LLM 回复中引用的内容**（LLM 从 context 中读取并复述了 XML 格式）
2. 或存在某种同步机制将 `context_messages` 复制到了 `display_messages`

---

### 问题 3: SendMessage 的 `to` 定向消息无 UI 标识

**现状**：`broadcast()` 已正确处理 `at_target`：
- context 消息格式: `<FromAgent> @Target text </FromAgent>`
- display 消息格式: 纯文本 `text`，无 `@Target` 标识

**需求**：用户需要看到消息是发给谁的（`all` 或 `@Target`）

---

## 修复方案

### 方案：分离 `main_agent_inbox` 与 `pending_user_messages`

**核心思路**：`main_agent_inbox` 应当只存放唤醒信号，不与 `pending_user_messages` 共享存储。唤醒信号不需要 drain 到 agent 的 messages，只需要触发"有新消息了，去读 context_messages"即可。

#### Step 1: `ChatState` 新增独立的 `main_agent_inbox` 字段

在 `ChatState` 中新增：
```rust
pub main_agent_inbox: Arc<Mutex<Vec<ChatMessage>>>,
```

初始化时创建独立的 Arc，不与 `pending_user_messages` 共享。

#### Step 2: 修改 `ChatApp` 初始化

`chat_app.rs`：
```rust
let main_agent_inbox: Arc<Mutex<Vec<ChatMessage>>> = Arc::new(Mutex::new(Vec::new()));
let teammate_manager = Arc::new(Mutex::new(TeammateManager::new(
    Arc::clone(&main_agent_inbox),  // ← 不再共享 pending_user_messages
    ...
)));
// 同时传给 ChatState
state.main_agent_inbox = main_agent_inbox;
```

#### Step 3: TUI loop 使用 `main_agent_inbox` 检测唤醒

`tui_loop.rs:631-637`：
```rust
// 修改前：
let has_inbox = !safe_lock(&app.state.pending_user_messages, ...).is_empty();

// 修改后：
let has_inbox = !safe_lock(&app.state.main_agent_inbox, ...).is_empty();
```

#### Step 4: `wake_from_teammate_inbox` 使用 `main_agent_inbox`

`message.rs:183`：
```rust
// 修改前：
safe_lock(&self.state.pending_user_messages, "wake_from_inbox::clear");

// 修改后：
safe_lock(&self.state.main_agent_inbox, "wake_from_inbox::clear");
```

#### Step 5: `drain_pending_user_messages` 不再受影响

因为 `<system_reminder>` 不再进入 `pending_user_messages`，`drain_pending_user_messages()` 自然不会 drain 它。用户追加的消息不受影响。

---

### 针对 `to` 定向消息的 UI 标识修复

#### Step 6: `broadcast()` 在 display 消息中保留 `@Target` 标识

`manager.rs:374`：
```rust
// 修改前：
let display_msg = ChatMessage::text(MessageRole::Assistant, text).with_sender(from);

// 修改后：
let display_text = if let Some(target) = at_target {
    format!("@{} {}", target, text)
} else {
    text.to_string()
};
let display_msg = ChatMessage::text(MessageRole::Assistant, &display_text).with_sender(from);
```

**渲染效果**：
```
Teammate@Counter1 @Counter2
╭──────────────────────────────╮
│ @Counter2 7 — 轮到你了，请报 8！ │
╰──────────────────────────────╯
```

---

## 修改文件清单

1. `src/command/chat/storage/mod.rs` — ChatState 新增 `main_agent_inbox` 字段
2. `src/command/chat/app/chat_app.rs` — 初始化时创建独立的 `main_agent_inbox` Arc
3. `src/command/chat/handler/tui_loop.rs:632-633` — 检测 `main_agent_inbox` 而非 `pending_user_messages`
4. `src/command/chat/app/message.rs:183` — `wake_from_teammate_inbox` 使用 `main_agent_inbox`
5. `src/command/chat/teammate/manager.rs:374` — display 消息添加 `@Target` 标识

## 测试验证

修改后需验证：
1. teammate 发消息时（无论 main agent 是否运行中），`<system_reminder>` 不进入 LLM context
2. UI display 消息无 XML 闭合标签
3. `to` 定向消息在 UI 中有 `@Target` 标识
4. main agent 空闲时 teammate 发消息能正确唤醒（`wake_from_teammate_inbox` 走新字段）
5. 用户追加消息正常工作（`pending_user_messages` 独立不受影响）
