# 重构计划：Message Role 字符串 → 枚举类型

## 背景

当前 `ChatMessage.role` 是 `String` 类型，存在以下问题：
1. **类型安全**：拼写错误无法在编译期发现
2. **性能**：每次处理都需要字符串比较
3. **语义不清晰**：渲染层需要 `role` + `tool_calls.is_some()` 双重判断

## 目标

1. 将 `role: String` 改为 `role: MessageRole` 枚举
2. 新增 `DisplayType` 枚举用于渲染层直接映射
3. 保持向后兼容（老 JSON 数据反序列化正常）

## 涉及文件

### 核心修改

| 文件 | 修改内容 |
|------|----------|
| `storage/types.rs` | 定义 `MessageRole` 和 `DisplayType` 枚举，修改 `ChatMessage` |
| `constants.rs` | 删除 `ROLE_*` 字符串常量，或改为枚举关联常量 |
| `render/cache.rs` | 使用 `display_type()` 替代字符串匹配 |
| `agent/api.rs` | 使用 `MessageRole` 枚举匹配 |
| `agent/window.rs` | 使用 `MessageRole` 枚举匹配 |
| `agent/compact.rs` | 使用 `MessageRole` 枚举匹配 |
| `agent/tool_processor.rs` | 使用 `MessageRole` 枚举创建消息 |
| `agent/agent_loop.rs` | 使用 `MessageRole` 枚举创建消息 |

### 次要修改

| 文件 | 修改内容 |
|------|----------|
| `app/chat_app.rs` | 创建消息时使用枚举 |
| `app/browse.rs` | 过滤消息时使用枚举 |
| `app/session_mgr.rs` | 消息判断使用枚举 |
| `storage/session.rs` | 消息判断使用枚举 |
| `remote/protocol.rs` | `SyncMessage.role` 保持字符串（外部协议） |
| `tools/derived_shared.rs` | 创建消息时使用枚举 |
| `tools/sub_agent.rs` | 创建消息时使用枚举 |
| `teammate/teammate_loop.rs` | 创建消息时使用枚举 |
| `oneshot.rs` | 创建消息时使用枚举 |

## 详细设计

### 1. 新增枚举定义 (`storage/types.rs`)

```rust
/// 消息角色（API 层 + 存储层共用）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
    System,
}

impl MessageRole {
    /// 返回对应的字符串表示（用于日志、外部协议等）
    pub const fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::Tool => "tool",
            MessageRole::System => "system",
        }
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// 显示类型（渲染层专用，面向 UI 语义细分）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayType {
    /// 用户消息（右对齐气泡）
    User,
    /// AI 文本回复（左对齐气泡 + Markdown）
    AssistantText,
    /// 工具调用请求（折叠/展开参数）
    ToolCallRequest,
    /// 工具执行结果（带状态图标 + 摘要）
    ToolResult,
    /// 系统消息（灰色缩进）
    System,
}
```

### 2. 修改 `ChatMessage` 结构

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,  // 改为枚举
    #[serde(default)]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallItem>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip)]
    pub images: Option<Vec<ImageData>>,
}

impl ChatMessage {
    /// 创建普通文本消息
    pub fn text(role: MessageRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            images: None,
        }
    }

    /// 推断显示类型（渲染层入口）
    pub fn display_type(&self) -> DisplayType {
        match self.role {
            MessageRole::User => DisplayType::User,
            MessageRole::System => DisplayType::System,
            MessageRole::Assistant => {
                if self.tool_calls.is_some() {
                    DisplayType::ToolCallRequest
                } else {
                    DisplayType::AssistantText
                }
            }
            MessageRole::Tool => DisplayType::ToolResult,
        }
    }
}
```

### 3. 渲染层改造 (`render/cache.rs`)

```rust
// Before:
match m.role.as_str() {
    ROLE_USER => { ... }
    ROLE_ASSISTANT => { ... }
    ROLE_TOOL => { ... }
    ROLE_SYSTEM => { ... }
    _ => {}
}

// After:
match m.display_type() {
    DisplayType::User => { ... }
    DisplayType::AssistantText => { ... }
    DisplayType::ToolCallRequest => { ... }
    DisplayType::ToolResult => { ... }
    DisplayType::System => { ... }
}
```

### 4. API 层改造 (`agent/api.rs`)

```rust
// Before:
match msg.role.as_str() {
    ROLE_SYSTEM => ...
    ROLE_USER => ...
    ROLE_ASSISTANT => ...
    ROLE_TOOL => ...
    _ => None,
}

// After:
match msg.role {
    MessageRole::System => ...
    MessageRole::User => ...
    MessageRole::Assistant => ...
    MessageRole::Tool => ...
}
```

### 5. 向后兼容

`serde` 的 `#[serde(rename_all = "lowercase")]` 会自动处理：
- 序列化：`MessageRole::User` → `"user"`
- 反序列化：`"user"` → `MessageRole::User`

老 JSON 文件无需迁移，直接兼容。

## 执行顺序

1. **Phase 1**：定义枚举
   - 在 `storage/types.rs` 添加 `MessageRole` 和 `DisplayType`
   - 添加 `FromStr` 实现以兼容手动解析场景

2. **Phase 2**：修改核心结构
   - 修改 `ChatMessage.role` 类型
   - 更新 `ChatMessage::text()` 等构造函数

3. **Phase 3**：修改使用点（按模块逐个）
   - `constants.rs` - 删除 `ROLE_*` 常量
   - `render/cache.rs` - 使用 `display_type()`
   - `agent/*.rs` - 使用枚举匹配
   - `app/*.rs` - 使用枚举创建/匹配
   - `tools/*.rs` - 使用枚举创建消息

4. **Phase 4**：清理
   - 删除不再使用的 `ROLE_*` 常量
   - 运行 `cargo clippy` 确保无告警
   - 运行测试确保功能正常

## 风险点

1. **遗漏的字符串比较**：部分代码可能直接使用 `"user"` 等字符串字面量
   - 缓解：全局搜索 `== "user"` 等模式

2. **外部协议**：`remote/protocol.rs` 的 `SyncMessage.role` 保持 `String` 类型
   - 原因：WebSocket 协议对外暴露，保持字符串更灵活

3. **宏或代码生成**：如有宏生成相关代码，需单独处理

## 测试策略

1. 编译测试：`cargo build` 无错误
2. 静态检查：`cargo clippy` 无告警
3. 单元测试：现有测试应全部通过
4. 手动验证：启动 chat 界面，检查消息渲染正常
