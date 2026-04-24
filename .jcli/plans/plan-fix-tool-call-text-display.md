# 修复 Tool Call 消息中的文本内容被清空的问题

## 问题现象

当 LLM 同时返回文本内容 + tool call 时（例如先输出解释文字再调用工具），文本消息只在流式阶段短暂显示，随后从 UI 上"消失"，只保留 tool call 信息。

## 根因分析

### 数据流

1. **流式阶段**：`assistant_text` 通过 `streaming_content` 在 UI 中正常显示。
2. **Tool call 到来**：`process_tool_calls()` 将 `assistant_text` + `tool_calls` 合并为一条 `ChatMessage`，同时清空 `streaming_content`。
3. **渲染判断**：`display_type()` 根据 `tool_calls.is_some()` 返回 `ToolCallRequest`。
4. **渲染执行**：`ToolCallRequest` 分支只渲染 `tool_calls`，完全忽略 `m.content`。

### 关键代码位置

| 文件 | 行号 | 问题 |
|------|------|------|
| `storage/types.rs` | 109-114 | `display_type()` 只看 `tool_calls`，忽略 `content` |
| `render/cache.rs` | 142-152 | `ToolCallRequest` 分支只渲染工具调用信息 |

## 修复方案

### 方案 A：在渲染 `ToolCallRequest` 时同时渲染文本内容（推荐）

**优点**：
- 最小改动
- 符合语义：一条 assistant 消息确实可以同时包含文本和 tool call
- 保持消息结构不变，不影响持久化和其他逻辑

**改动**：
1. `render/cache.rs`：在 `ToolCallRequest` 分支中，先渲染 `content`（如果有），再渲染 `tool_calls`

```rust
DisplayType::ToolCallRequest => {
    // 先渲染文本内容（如果有）
    if !m.content.is_empty() {
        render_assistant_msg(&m.content, is_selected, bubble_max_width, &mut tmp_lines, t);
    }
    // 再渲染工具调用
    if let Some(ref tool_calls) = m.tool_calls {
        render_tool_call_request_msg(tool_calls, bubble_max_width, &mut tmp_lines, t, expand);
    }
}
```

### 方案 B：新增 `ToolCallRequestWithText` 类型

**优点**：
- 类型更精确，便于未来扩展

**缺点**：
- 改动较大，需要修改 `DisplayType` enum 和多处判断逻辑
- 可能影响其他依赖 `display_type()` 的逻辑

### 方案 C：拆分为两条消息

**优点**：
- 消息结构更清晰

**缺点**：
- 破坏了"一条 assistant 消息可以同时有文本和 tool call"的 API 语义
- 需要修改 `process_tool_calls` 的消息构建逻辑
- 可能影响消息持久化和 LLM 上下文重建

---

## 推荐方案 A

改动范围：
- `src/command/chat/render/cache.rs` 第 142-152 行

影响评估：
- 只影响渲染逻辑，不影响消息存储、持久化、LLM 上下文
- 已有的 tool call 消息如果 `content` 为空（大多数情况），行为不变
- 有文本内容的 tool call 消息将正确显示文本 + 工具调用