# 实现 Thinking/Reasoning 内容的流式实时显示 + 修复绿点脉冲动画

## 问题

1. **绿点脉冲动画不工作**：当 LLM 在 thinking 阶段（`streaming_content` 为空，`is_loading` 为 true），`streaming_len` 始终为 0，缓存命中判断通过，跳过重绘。`thinking_pulse_color()` 虽然基于系统时间计算不同颜色，但缓存阻止了实际重绘。
2. **Reasoning 内容不可见**：`reasoning_content` 只积累到局部变量 `assistant_reasoning`，不写入 UI 可见的缓冲区。

## 修复方案

### 改动 1: 缓存命中判断 — loading 状态下始终重绘（核心修复）

**文件**: `ui/chat.rs`（第 531-541 行）

当 `is_loading` 为 true 时，缓存永远不命中，确保脉冲动画和 reasoning 内容实时更新。

```rust
let cache_hit = if !app.state.is_loading {
    // 非 loading 状态：正常缓存判断
    if let Some(ref cache) = app.ui.msg_lines_cache {
        cache.msg_count == msg_count
            && cache.last_msg_len == last_msg_len
            && cache.streaming_len == streaming_len
            && cache.bubble_max_width == bubble_max_width
            && cache.browse_index == current_browse_index
            && cache.tool_confirm_idx == current_tool_confirm_idx
    } else {
        false
    }
} else {
    // loading 状态：始终重绘（脉冲动画 + reasoning 内容需要实时更新）
    false
};
```

**性能考虑**：
- loading 状态下 tui_loop 已有节流机制（150ms/200bytes），不会过度重绘
- 非 loading 状态下缓存逻辑完全不变
- loading 状态下虽然顶层缓存不命中，但 P0 增量缓存（第 108-124 行）仍然有效——历史消息的 `content_len` 和 `is_selected` 没变，直接复用旧缓存行（零拷贝）
- 实际重新渲染的只有 streaming 区域（绿点 + reasoning 内容），开销极小

### 改动 2: 添加 `streaming_reasoning_content` 字段

**文件**: `app/chat_state.rs`（第 16 行后）
```rust
pub streaming_reasoning_content: Arc<Mutex<String>>,
```

**文件**: `app/chat_app.rs`（第 491 行后）
```rust
streaming_reasoning_content: Arc::new(Mutex::new(String::new())),
```

**文件**: `agent/config.rs`（`AgentLoopSharedState` 第 30 行后）
```rust
pub streaming_reasoning_content: Arc<Mutex<String>>,
```

### 改动 3: 传递 `streaming_reasoning_content` 到 agent loop

**文件**: `app/message.rs`（第 212 行附近 + 第 290-301 行）

clone `streaming_reasoning_content` 并传入 `AgentLoopSharedState`。

### 改动 4: Agent Loop 写入 reasoning 缓冲区

**文件**: `agent/agent_loop.rs`（第 585-590 行）

```rust
if let Some(ref reasoning) = choice.delta.reasoning_content {
    assistant_reasoning.push_str(reasoning);
    // 写入 UI 可见的流式缓冲区
    {
        let mut reason_buf = safe_lock(&streaming_reasoning_content, "agent::stream_reasoning");
        reason_buf.push_str(reasoning);
    }
    let _ = tx.send(StreamMsg::Chunk);
}
```

同时在所有清空 `streaming_content` 的位置（第 330、680、739、812 行）同步清空 `streaming_reasoning_content`。

### 改动 5: 渲染 thinking 区块

**文件**: `render/cache.rs`（第 277-289 行，`streaming_text == "◍"` 分支）

当 `streaming_reasoning_content` 非空时，在 ◍ 绿点下方渲染 reasoning 内容：

```rust
if streaming_text == "◍" {
    let pulse_color = thinking_pulse_color(t);
    let indicator_line = Line::from(Span::styled("◍", Style::default().fg(pulse_color)));
    let bubble_line = wrap_md_line_in_bubble(...);
    streaming_lines.push(bubble_line);

    // 如果有 reasoning 内容，在绿点下方渲染
    let reasoning_str = safe_lock(&app.state.streaming_reasoning_content, "render::reasoning").clone();
    if !reasoning_str.is_empty() {
        // Thinking 标签
        streaming_lines.push(Line::from(Span::styled(
            "  Thinking...",
            Style::default().fg(t.text_muted).add_modifier(Modifier::ITALIC),
        )));
        // Reasoning 内容（灰色文本）
        let wrapped = wrap_text(&reasoning_str, bubble_max_width.saturating_sub(4));
        for line in wrapped {
            streaming_lines.push(wrap_md_line_in_bubble(
                Line::from(Span::styled(line, Style::default().fg(t.text_muted))),
                bubble_bg, pad_left_w, pad_right_w, bubble_total_w,
            ));
        }
    }

    // 下边距
    ...
}
```

### 改动 6: 流式结束清空 reasoning 缓冲区

**文件**: `agent/tool_processor.rs`（`flush_streaming_as_message` 函数）

添加 `streaming_reasoning_content` 参数，函数末尾清空。更新 `agent_loop.rs` 中 3 个调用点。

### 改动 7: 历史消息渲染 reasoning_content

**文件**: `render/cache.rs`（`build_message_lines_incremental` 中 `AssistantText` 和 `ToolCallRequest` 分支）

如果消息有 `reasoning_content`，在内容上方渲染一个 thinking 区块。

## 改动文件清单

| 文件 | 改动 |
|------|------|
| `ui/chat.rs` | loading 状态下缓存永远不命中 |
| `app/chat_state.rs` | 添加 `streaming_reasoning_content` 字段 |
| `app/chat_app.rs` | 初始化新字段 |
| `agent/config.rs` | `AgentLoopSharedState` 添加字段 |
| `app/message.rs` | clone 并传递 `streaming_reasoning_content` |
| `agent/agent_loop.rs` | 写入 reasoning 缓冲区 + 同步清空 |
| `agent/tool_processor.rs` | `flush_streaming_as_message` 添加参数并清空 |
| `render/cache.rs` | 渲染 thinking 区块 + 历史消息 reasoning |
