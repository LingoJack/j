# 修复 Teammate/SubAgent UI 标签 + sender_name 字段

## 方案

两管齐下：
1. **ChatMessage 加 `sender_name: Option<String>` 字段** — 渲染层直接读此字段显示气泡标签名，不用解析 content
2. **context_messages 中 content 用 `<Name>text</Name>` XML 包裹** — LLM 能清晰看到消息来源
3. **display_messages 中 content 保持纯文本** — UI 渲染层靠 `sender_name` 字段显示名字，content 不含前缀

## 改动清单

### 1. `src/command/chat/storage/types.rs` — ChatMessage 加字段
- 加 `pub sender_name: Option<String>` 字段
- `#[serde(skip_serializing_if = "Option::is_none")]` 不持久化到 session
- `ChatMessage::text()` 初始化为 `None`
- 加 helper `pub fn with_sender(mut self, name: impl Into<String>) -> Self`

### 2. `src/command/chat/teammate/teammate_loop.rs` — teammate 消息设 sender_name
- 文本回复：`msg.sender_name = Some(format!("Teammate@{}", name))`，content 不含 `<Name>` 前缀
- 工具调用广播：同上
- 完成广播：同上
- **context_messages**: content 用 `<Teammate@Name>text</Teammate@Name>` 包裹
- **display_messages**: content 不包裹，纯文本

### 3. `src/command/chat/tools/sub_agent.rs` — subagent 消息设 sender_name
- 文本回复：`msg.sender_name = Some(format!("SubAgent@{}", agent_name))`
- 工具调用广播：同上
- 完成广播：同上
- **context_messages**: content 用 `<SubAgent@Name>text</SubAgent@Name>` 包裹
- **display_messages**: content 不包裹

### 4. `src/command/chat/render/cache.rs` — 渲染层用 sender_name
- `render_assistant_msg()`: 优先从 `msg.sender_name` 读标签名
  - 有 sender_name → 用它作为气泡标签（如 "Teammate@Frontend"）
  - 无 sender_name → 显示 "Sprite"（主 agent）
- 移除 `parse_agent_prefix` 对 content 的 `<Name>` 前缀解析（不再需要）
- `is_teammate` 判断改为 `msg.sender_name.is_some()`

### 5. `src/command/chat/context/message_compress.rs` — 保持兼容
- `extract_agent_source()` 等函数仍从 context_messages 的 `<Name>` 格式中提取
- 不需要改动，因为 context_messages 仍用 XML 包裹

### 6. `src/command/chat/ui/title_bar.rs` — 流式显示标签
- streaming 标签仍为 "Sprite"，无改动

## 数据流示意

```
Teammate "Frontend" 说 "Hello"

display_messages:  sender_name=Some("Teammate@Frontend"), content="Hello"
                    → UI 显示标签 "Teammate@Frontend"，正文 "Hello"

context_messages:  sender_name=None, content="<Teammate@Frontend>Hello</Teammate@Frontend>"
                    → LLM 看到 XML 包裹，清晰知道来源
```

## 注意事项

- `sender_name` 用 `skip_serializing_if` 不持久化到 session 文件（session 恢复时不需要）
- 现有 `ChatMessage::text()` 的所有调用点自动得到 `sender_name: None`，无需逐一修改
- `parse_agent_prefix` 可保留作为 fallback，用于处理老 session 中无 sender_name 的消息
