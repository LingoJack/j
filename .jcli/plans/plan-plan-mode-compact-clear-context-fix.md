# Plan: 修复 Compact 和 Plan Mode Clear Context 的消息顺序问题

## 问题分析

### 问题 1: Compact 后 recent_user_messages 在 UI 中的显示顺序不对

**现状** (`agent_loop.rs` 的 `push_compact_tool_messages` 函数，第 22-64 行):

```
push_compact_tool_messages 向 ui_messages 推送顺序:
1. assistant tool_call 消息 (Compact)
2. tool result 消息（摘要内容）
3. recent_user_messages（保留的最近用户消息）
```

**问题**: compact 的 tool result（带摘要的方框）出现在 recent user messages **之前**。用户期望 compact 消息应在恢复的 user messages **下方**。

**期望顺序**:
```
1. [恢复的最近 user messages]  ← 用户能看到自己的原始消息
2. assistant tool_call (Compact)
3. tool result（摘要内容）      ← compact 摘要出现在 user 消息下方
```

同时，`auto_compact` 函数中 `messages`（LLM 上下文）的顺序也需要对应调整：
- 现状: `[summary_user, understood_assistant, ...recent_user_msgs]`
- 期望: `[...recent_user_msgs, summary_user, understood_assistant]` 或保持现状但确保 UI 推送顺序正确

**关键点**: `messages`（LLM API 消息数组）和 `ui_messages`（TUI 渲染用）是两个独立数组。`push_compact_tool_messages` 负责向两者同时推送，顺序需要分别处理。

### 问题 2: Plan Mode 的 "批准并清空上下文" 以 User 角色代替用户发消息

**现状** (`agent_loop.rs` 第 1024-1044 行):

```rust
// Plan 被批准且清空上下文
let preserved_users = compact::extract_user_messages(&messages);
messages.clear();
// 清空 ui_messages
for user_msg in preserved_users {
    messages.push(user_msg.clone());
    push_ui(&ui_messages, user_msg);
}
// ❌ 以 User 角色发送计划消息 - 这等于假装用户说了这段话
let plan_msg = ChatMessage::text(
    MessageRole::User,
    format!("以下计划已获批准，请按计划执行：\n\n{}", plan_content),
);
messages.push(plan_msg.clone());
push_ui(&ui_messages, plan_msg);
```

**问题**:
1. 计划内容以 `MessageRole::User` 发送，等于系统假装用户说了这段话，但用户实际上没有输入
2. 这条消息在 UI 中会渲染为用户消息气泡（You 标签），但实际上不是用户发送的

**期望行为**:
1. 清空上下文（仅保留历史 user messages）
2. 恢复上一条 user message（让 LLM 知道用户的原始意图）
3. 计划内容作为 **ExitPlanMode 的 tool result** 出现（而不是代替用户发消息）

## 修改方案

### 修改 1: `push_compact_tool_messages` - 调整 UI 消息推送顺序

**文件**: `src/command/chat/agent/agent_loop.rs`

将 `push_compact_tool_messages` 函数中 `ui_messages` 的推送顺序改为：
1. 先推送 recent_user_messages（用户消息在上）
2. 再推送 assistant tool_call + tool result（compact 摘要在下）

同时，`messages`（LLM 上下文）中也需要确保 recent_user_messages 在 summary 消息之前：
- 现状 `auto_compact` 中: `[summary_user, understood_assistant, ...recent_user_msgs]`
- 调整为: `[...recent_user_msgs, summary_user, understood_assistant]`

这样 LLM 上下文和 UI 的显示顺序一致。

### 修改 2: `auto_compact` 函数 - 调整 messages 数组中 recent_user_messages 的位置

**文件**: `src/command/chat/agent/compact.rs`

在 `auto_compact` 函数末尾（约第 442-481 行），将 `recent_user` 消息放到 `summary_content` **之前**：

```rust
// 调整前:
messages.push(ChatMessage::text(MessageRole::User, summary_content));  // summary
messages.push(ChatMessage::text(MessageRole::Assistant, "Understood...")); // ack
for msg in recent_user { messages.push(msg); }  // user msgs 在最后

// 调整后:
for msg in recent_user { messages.push(msg); }  // user msgs 在最前面
messages.push(ChatMessage::text(MessageRole::User, summary_content));  // summary
messages.push(ChatMessage::text(MessageRole::Assistant, "Understood...")); // ack
```

### 修改 3: `push_compact_tool_messages` - 与 auto_compact 保持一致的 UI 顺序

**文件**: `src/command/chat/agent/agent_loop.rs`

```rust
// 调整前:
// 1. assistant tool_call
// 2. tool result
// 3. recent_user_messages

// 调整后:
// 1. recent_user_messages（先推送到 UI）
// 2. assistant tool_call
// 3. tool result（摘要）
```

### 修改 4: Plan Mode Clear Context - 将计划内容作为 Tool Result 而非 User 消息

**文件**: `src/command/chat/agent/agent_loop.rs`

两处 plan clear context 逻辑（流式路径约第 1024-1044 行，非流式路径约第 847-870 行）需要修改：

```rust
// 调整前:
for user_msg in preserved_users {
    messages.push(user_msg.clone());
    push_ui(&ui_messages, user_msg);
}
let plan_msg = ChatMessage::text(
    MessageRole::User,  // ❌ 假装是用户消息
    format!("以下计划已获批准，请按计划执行：\n\n{}", plan_content),
);
messages.push(plan_msg.clone());
push_ui(&ui_messages, plan_msg);

// 调整后:
// 1. 恢复 user messages
for user_msg in &preserved_users {
    messages.push(user_msg.clone());
    push_ui(&ui_messages, user_msg.clone());
}
// 2. 计划内容作为 assistant 消息 + tool result 形式注入
//    模拟 ExitPlanMode 工具被调用的效果
let tool_call_id = "plan_exit_approved";
let tool_call_item = ToolCallItem {
    id: tool_call_id.to_string(),
    name: "ExitPlanMode".to_string(),
    arguments: r#"{"approved":true,"clear_context":true}"#.to_string(),
};
let tool_call_msg = ChatMessage {
    role: MessageRole::Assistant,
    content: String::new(),
    tool_calls: Some(vec![tool_call_item]),
    tool_call_id: None,
    images: None,
};
messages.push(tool_call_msg.clone());
push_ui(&ui_messages, tool_call_msg);

let result_content = format!(
    "Plan approved with context clear! Exited plan mode.\n\n{}\n\n请按以上计划继续执行。",
    plan_content
);
let tool_msg = ChatMessage {
    role: MessageRole::Tool,
    content: result_content,
    tool_calls: None,
    tool_call_id: Some(tool_call_id.to_string()),
    images: None,
};
messages.push(tool_msg.clone());
push_ui(&ui_messages, tool_msg);
```

## 涉及文件

| 文件 | 修改类型 | 说明 |
|------|---------|------|
| `src/command/chat/agent/agent_loop.rs` | 修改 | `push_compact_tool_messages` 顺序调整 + plan clear context 逻辑修改 |
| `src/command/chat/agent/compact.rs` | 修改 | `auto_compact` 中 recent_user_messages 位置调整 |

## 测试要点

1. **Compact 顺序**: 触发 auto_compact 后，确认 UI 中 recent user messages 在 compact 摘要框上方
2. **Compact 后 LLM 行为**: 确认 compact 后 LLM 能正确理解上下文（recent user + summary 的顺序不影响 LLM 的理解能力）
3. **Plan Mode Clear**: 进入 plan mode → 写计划 → 选择"批准并清空上下文" → 确认计划内容以 tool result 形式出现而非 user 消息
4. **Plan Mode Clear 后 UI**: 确认历史 user messages 被保留，ExitPlanMode tool result 出现在 user messages 下方
5. **Plan Mode Clear 后 LLM 行为**: 确认 LLM 能正确理解保留的 user messages + 计划 tool result 并开始执行

## 风险评估

- **低风险**: `push_compact_tool_messages` 顺序调整仅影响 UI 渲染，不影响 LLM 调用
- **中风险**: `auto_compact` 中 messages 顺序调整可能影响 LLM 对上下文的理解（summary 在 user msgs 之后可能让 LLM 更关注 summary 而忽略 user msgs）。但考虑到 summary 本身已包含 user 的意图摘要，且 recent_user_msgs 已在 summary 之前，LLM 应能正确处理
- **中风险**: Plan clear context 改为 tool result 形式后，LLM 会看到一个 ExitPlanMode tool result，而不是直接收到"请执行计划"的 user 消息。需要确认 LLM 能识别这个 tool result 并开始执行计划
