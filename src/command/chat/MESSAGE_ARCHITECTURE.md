# Chat 消息三通道架构

## 核心结构

整个 chat 模块只有**一个消息结构体** `ChatMessage`，通过三个独立 `Vec<ChatMessage>` 容器实现 session / context / display 三层分离。

### ChatMessage 定义

`storage/types.rs:69-91`

```rust
pub struct ChatMessage {
    pub role: MessageRole,                      // User | Assistant | Tool | System
    pub content: String,                        // 消息文本（三通道内容可能不同）
    pub tool_calls: Option<Vec<ToolCallItem>>,  // LLM 发起的工具调用
    pub tool_call_id: Option<String>,           // 工具结果对应的 call_id
    pub images: Option<Vec<ImageData>>,         // #[serde(skip)] 不持久化
    pub reasoning_content: Option<String>,      // thinking mode 思考内容
    pub sender_name: Option<String>,            // #[serde(skip_serializing_if=None)] teammate/subagent 运行时标签
}
```

- `images` — 不持久化，多模态 user message 专用
- `sender_name` — 运行时字段，标记 teammate/subagent 来源，UI 据此渲染气泡标签
- `display_type()` (`types.rs:119-132`) — 由 `role + tool_calls` 推导出 `DisplayType` 枚举（User | AssistantText | ToolCallRequest | ToolResult | System），渲染层直接使用

## 三通道存储位置

| 通道 | 字段 | 位置 | 用途 |
|------|------|------|------|
| **Session** | `ChatState.session.messages` | `chat_state.rs:14` | 持久化缓冲，写入 `transcript.jsonl` |
| **Context** | `ChatApp.context_messages` | `chat_app.rs:101` | LLM API 输入，含 XML 标签区分来源 |
| **Display** | `ChatApp.display_messages` | `chat_app.rs:95` | TUI 渲染数据源，干净文本 |

每个通道都有配套的 `read_offset`（`display_read_offset` / `context_read_offset`）用于增量同步检测。

## 双通道设计（Display vs Context）

`stream_poll.rs:19-29` 的注释是核心设计说明：

- **display_messages**：UI 渲染数据源，干净文本 + `sender_name` 字段。`build_message_lines_incremental` 直接读取，不经 session 中转
- **context_messages**：LLM context 数据源，XML 前缀（如 `<Teammate@Frontend>text</Teammate@Frontend>`），`build_api_messages` 直接读取
- **session.messages**：仅持久化用，不经 UI/LLM 直接读取

**主 agent 消息**：两条通道内容相同，通过 `push_both_channels()` / `push_both()` 同时写入。

**Teammate/SubAgent 消息**：两条通道内容不同——
- display 通道：干净文本 + `sender_name` 字段
- context 通道：XML 包裹文本（`<Sender>content</Sender>`），LLM 据此区分消息来源

## 数据流

```
用户输入
  │
  ▼
ChatApp::send_message_internal()
  │─ ChatMessage::text(User, text)
  │─ push_both_channels(msg)
  │     ├─ display_messages.push(msg)
  │     └─ context_messages.push(msg)
  │
  ▼
ChatApp::build_api_messages()            ← 读取 context_messages
  │─ select_messages()                   ← 窗口选择（4 阶段优先级，context/window.rs:402）
  │─ 返回 Vec<ChatMessage> (api_messages)
  │
  ▼
spawn_agent_loop(provider, api_messages)
  │─ api_messages 成为 agent 线程本地 `messages: Vec<ChatMessage>`
  │
  ▼  ── agent 线程 ──────────────────────────────────
  │
  │─ drain_pending_user_messages()       ← 注入用户排队消息
  │─ micro_compact(&mut messages)        ← 原地替换旧 tool result 为占位符
  │─ auto_compact(&mut messages)         ← LLM 摘要压缩（超 token 阈值时触发）
  │─ sanitize_messages(&messages)        ← 去除孤立的 tool_call/tool_result
  │─ to_llm_messages(&messages)          ← ChatMessage → llm::Message
  │─ build_request_with_tools()          ← 组装最终 ChatRequest
  │─ LLM API 调用（流式）
  │
  │  [assistant 文本 chunk]:
  │    streaming_content.push_str(chunk)  ← UI 直接读取此字段渲染
  │
  │  [tool_calls 收到]:
  │    messages.push(tool_call_msg)
  │    push_both(display, context, tool_call_msg)
  │    [主线程确认 → 执行工具]
  │    messages.push(tool_result_msg)
  │    push_both(display, context, tool_result_msg)
  │    continue 'round                     ← 用更新后的 messages 继续下一轮
  │
  │  [auto_compact 完成]:
  │    clear_channels(display, context)    ← 旧消息清空
  │    push_compact_tool_messages(...)     ← 推入压缩后的消息
  │
  ▼
StreamMsg::Done → 主线程
  │
  ▼
poll_stream_actions()                    ← 每帧调用（主线程）
  │─ display_messages.len() vs display_read_offset
  │     变化 → msg_lines_cache = None → 触发重渲染
  │─ sync_context_to_session()
  │     context_messages[offset..] → session.messages
  │─ persist_new_messages()
  │     session.messages[persisted_count..safe_cut] → append transcript.jsonl
  │
  ▼
draw_messages()                          ← 渲染阶段
  │─ 读取 display_messages（不读 session.messages）
  │─ build_message_lines_incremental()
  │     按 msg.display_type() 分派到 render_*_msg()
  │─ streaming_content 单独渲染（进行中的回复）
```

## Session 通道 — 持久化

### 写入路径

1. `sync_context_to_session()` (`stream_poll.rs:616-628`)：从 `context_messages[offset..]` 增量复制到 `session.messages`
2. `persist_new_messages()` (`session_mgr.rs:24-64`)：从 `session.messages[persisted_count..]` 找到"安全切点"（所有 `assistant(tool_calls)` 配齐对应 `tool_result`），append 到 `transcript.jsonl`

**安全切点**：避免 JSONL 中出现孤立的 tool_call（没有对应 result）或孤立的 tool_result。半完成的尾部留到下次 persist。

**关键不变量**：`session.messages` **唯一写入路径**是 `sync_context_to_session()`，无其他代码直接 push。

### 读取路径

- `load_session(session_id)` (`storage/session.rs:283-322`)：回放 JSONL，`SessionEvent::Msg` push 消息，`SessionEvent::Clear` 清空，`SessionEvent::Restore` 替换。加载时还会修复孤立的 tool_call/tool_result 对。
- `rebuild_channels_from_session()` (`session_mgr.rs:344-377`)：会话恢复/切换时，从 `session.messages` 重建双通道：
  - context_messages：直接 move（session.messages 存的就是 context 版本，含 XML）
  - display_messages：对有 `sender_name` 的消息做 `strip_agent_xml_tag()` 去除 XML 包裹

## Context 通道 — LLM API 输入

### 填充方式

| 方式 | 函数 | 场景 |
|------|------|------|
| 同步写入 | `push_both_channels()` / `push_both()` | 主 agent 消息 |
| 分异写入 | 各自 push 不同内容 | teammate/subagent 消息 |
| 全量替换 | `sync_context_full()` | auto_compact 或 PostAutoCompact hook 修改后 |
| 清空 | `clear_channels()` | auto_compact 重建前 |

### 从 context_messages 到 API 请求

1. `build_api_messages()` (`system_prompt.rs:132-142`)：读 `context_messages`，调用 `select_messages()`
2. `select_messages()` (`context/window.rs:402-490`)：4 阶段优先级窗口选择
   - 时间保证（最近 N 条必选）
   - 豁免工具保证（特定工具结果必选）
   - 按比例配额
   - 溢出兜底
3. 选出的 `Vec<ChatMessage>` 作为 `api_messages` 传入 `spawn_agent_loop()`
4. agent 线程本地 `messages` 经过 compact → sanitize → to_llm_messages → build_request_with_tools

**agent 线程本地的 `messages: Vec<ChatMessage>`** 是请求期间的权威副本，可被 compact/hook/drain 原地修改。

## Display 通道 — TUI 渲染

### 填充方式

与 context 通道同步（通过 `push_both` / `push_both_channels`），但 teammate/subagent 消息内容为干净文本（无 XML 包裹）。

### 渲染流程

1. `poll_stream_actions()` 检测 `display_messages.len()` 变化 → 置 `msg_lines_cache = None`
2. `build_message_lines_incremental()` (`render/cache.rs:74-454`)：直接读取 `display_messages`
3. 按 `msg.display_type()` 分派：
   - `User` → `render_user_msg()`（右对齐气泡）
   - `AssistantText` → `render_assistant_msg()`（左对齐气泡，Markdown 渲染）
   - `ToolCallRequest` → `render_tool_call_request_msg()`（工具名 + 参数）
   - `ToolResult` → `render_tool_result_msg()`（边框结果）
   - `System` → 灰色缩进文本
4. **流式内容**单独渲染：`streaming_content: Arc<Mutex<String>>`（进行中的 assistant 回复），不经 display_messages

**关键**：渲染层**从不读 `session.messages`**，只读 `display_messages` + `streaming_content`。

## 关键转换函数

| 函数 | 文件 | 输入 → 输出 | 用途 |
|------|------|------------|------|
| `ChatMessage::display_type()` | `types.rs:119` | `&self` → `DisplayType` | role+tool_calls → UI 语义类型 |
| `select_messages()` | `context/window.rs:402` | `&[ChatMessage]` → `Vec<ChatMessage>` | 优先级窗口选择 |
| `micro_compact()` | `context/compact.rs:228` | `&mut [ChatMessage]` → in-place | 旧 tool result 替换为占位符 |
| `auto_compact()` | `context/compact.rs:337` | `&mut Vec<ChatMessage>` → `CompactResult` | LLM 摘要压缩 |
| `sanitize_messages()` | `agent/api.rs:124` | `&[ChatMessage]` → `Vec<ChatMessage>` | 去孤立 tool_call/result |
| `to_llm_messages()` | `agent/api.rs:17` | `&[ChatMessage]` → `Vec<llm::Message>` | 转换为 API 格式 |
| `push_both()` | `agent/tool_processor.rs:68` | 双通道 + msg → side-effect | 同时推入 display + context |
| `push_both_channels()` | `app/session_mgr.rs:389` | self + msg → side-effect | ChatApp 级双通道推送 |
| `sync_context_to_session()` | `app/stream_poll.rs:616` | 读 context_messages → 写 session | 增量同步持久化 |
| `rebuild_channels_from_session()` | `app/session_mgr.rs:344` | session.messages → 双通道 | 会话恢复/切换 |
| `clear_channels()` | `app/session_mgr.rs:380` | 双通道清空 | 新建会话 / compact 重建 |
| `strip_agent_xml_tag()` | `app/session_mgr.rs:397` | sender + content → 干净文本 | 去除 `<Sender>...</Sender>` |
| `compress_other_agent_toolcalls()` | `context/message_compress.rs:75` | `&[ChatMessage]` → `Vec<ChatMessage>` | 压缩 teammate 工具调用广播 |

## 设计要点

1. **一结构体三容器**：`ChatMessage` 统一结构，三层角色由所在 `Vec` 和内容差异（有无 XML 包裹）区分
2. **双通道分异**：teammate/subagent 消息在 display 和 context 中内容不同——display 干净文本，context XML 包裹
3. **session 是持久化缓冲**：不被 UI/LLM 直接读取，唯一写入路径是 `sync_context_to_session()`，唯一消费方是 `persist_new_messages()`
4. **agent 线程本地 messages 是请求期权威**：从 `build_api_messages()` 拷贝而来，被 compact/hook/drain 原地修改
5. **offset 增量同步**：`display_read_offset` / `context_read_offset` 防止重复消费
6. **安全切点持久化**：`persist_new_messages()` 只在 tool_call 配齐 tool_result 处切分，保证 JSONL 完整性
