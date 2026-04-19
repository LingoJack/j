# Plan: Compact 事件 UI 感知

## 问题

当前 `auto_compact`（Layer 2 被动 / Layer 3 主动 / tool_call_id 恢复）执行时，UI 完全无感知：
- 标题栏没有"正在压缩"状态提示
- 压缩完成后没有 toast 通知
- 用户无法区分"LLM 思考中"和"上下文正在压缩"

## 修改方案

### 1. `StreamMsg` 新增 `Compacting` 变体

**文件**: `src/command/chat/app/types.rs`

在 `StreamMsg` 枚举中增加：
```rust
/// 上下文正在压缩（auto_compact 执行中）
Compacting,
```

### 2. `Action` 新增 `StreamCompacting` 变体

**文件**: `src/command/chat/app/action.rs`

在流式生命周期分组（`StreamRetrying` 之后）增加：
```rust
/// 上下文正在压缩（auto_compact 执行中）
StreamCompacting,
```

### 3. `auto_compact` 返回 `CompactResult`

**文件**: `src/command/chat/agent/compact.rs`

修改 `auto_compact` 函数返回值：
```rust
pub struct CompactResult {
    /// 压缩前的消息数
    pub messages_before: usize,
}

// 返回类型: Result<CompactResult, String>
```

在函数开头记录 `messages.len()`，压缩完成后返回 `CompactResult { messages_before }`。

### 4. agent_loop 在所有 `auto_compact` 调用前后发送 StreamMsg

**文件**: `src/command/chat/agent/agent_loop.rs`

共 4 处调用点，每处统一模式：

```rust
// 压缩前：通知 UI
let _ = tx.send(StreamMsg::Compacting);

// 执行压缩
match compact::auto_compact(...).await {
    Ok(result) => {
        // 压缩成功：通过 push_ui 向 UI 消息列表推送一条 system 消息
        let compact_msg = ChatMessage {
            role: "system".to_string(),
            content: format!("📦 上下文已压缩 ({} 条消息已归档)", result.messages_before),
            ..Default::default()
        };
        push_ui(&ui_messages, compact_msg);
    }
    Err(e) => {
        // 已有错误处理逻辑，不变
    }
}
```

4 处调用点：
1. L199: Layer 2 token 阈值触发
2. L542: tool_call_id 不一致恢复
3. L733: Layer 3 CompactTool - fallback 非流式路径
4. L898: Layer 3 CompactTool - 流式路径

### 5. stream_poll 处理 `StreamMsg::Compacting`

**文件**: `src/command/chat/app/stream_poll.rs`

在 `match msg` 分支中增加：
```rust
StreamMsg::Compacting => {
    actions.push(Action::StreamCompacting);
}
```

### 6. chat_app.update 处理 `Action::StreamCompacting`

**文件**: `src/command/chat/app/chat_app.rs`

在 `StreamRetrying` 分支之后增加：
```rust
Action::StreamCompacting => {
    self.state.retry_hint = Some("📦 压缩上下文中...".to_string());
}
```

复用 `retry_hint` 机制——标题栏已有渲染 `retry_hint` 的逻辑，无需修改 `ui/chat.rs`。

### 7. 压缩完成后清除 retry_hint

`auto_compact` 完成后，agent_loop 会 `continue 'round`，后续流程（收到 chunk 或完成）会：
- `StreamMsg::Chunk` → `Action::StreamChunk`：`chat_app.update` 中 `StreamChunk` 不修改 `retry_hint`
- `StreamMsg::Done` → `Action::StreamDone`：`finish_loading` 中会清除 `retry_hint`

但存在一个 gap：如果 auto_compact 后 LLM 直接返回文本回复（chunk），在收到第一个 chunk 时不会清除 `retry_hint`。需要在 `Action::StreamChunk` 处理中清除 `retry_hint`：

```rust
Action::StreamChunk => {
    // 清除压缩提示（如果有）
    self.state.retry_hint = None;
    // ... 原有逻辑
}
```

## 涉及文件清单

| 文件 | 改动 |
|------|------|
| `app/types.rs` | `StreamMsg` 增加 `Compacting` 变体 |
| `app/action.rs` | `Action` 增加 `StreamCompacting` 变体 |
| `agent/compact.rs` | `auto_compact` 返回 `CompactResult` |
| `agent/agent_loop.rs` | 4 处 `auto_compact` 调用前后发通知 + push_ui |
| `app/stream_poll.rs` | `match StreamMsg::Compacting` 分支 |
| `app/chat_app.rs` | 处理 `Action::StreamCompacting`；`StreamChunk` 中清除 retry_hint |

## 不改动的部分

- `ui_state.rs` — 不新增字段，复用 `retry_hint`
- `ui/chat.rs` — 不修改渲染逻辑，复用已有 loading/retry_hint 显示
- `ui/hint.rs` — 不修改
