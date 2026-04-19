# Shell 工具 description 在 UI 中的展示

## 背景

`ShellParams` 中有 `description` 字段（`shell.rs:26-28`），注释说明 "displayed in the UI"，但实际上从未在 UI 中被使用。目前标题栏只显示 `🔧 执行 Bash...`，用户无法从标题栏快速了解正在执行什么操作。

## 目标

将 Shell 工具的 `description` 参数（如 "list files"、"build project"）展示在 UI 中：
- **标题栏**：`🔧 执行 Bash...` -> `🔧 执行 Bash: build project...`
- **消息气泡（折叠模式）**：工具名旁边附带 description

## 修改方案

### 1. `ToolCallStatus` 增加 `tool_description` 字段

**文件**: `src/command/chat/app/types.rs`

```rust
pub struct ToolCallStatus {
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: String,
    pub confirm_message: String,
    pub status: ToolExecStatus,
    pub tool_description: Option<String>,  // 新增
}
```

### 2. 构建 `ToolCallStatus` 时提取 `description`

**文件**: `src/command/chat/app/chat_app.rs`

在所有创建 `ToolCallStatus` 的位置（约 3 处），从 `tc.arguments` JSON 中提取 `description` 字段（仅当 `tc.name == "Bash"` 时），赋值给 `tool_description`。

辅助函数：
```rust
/// 从工具调用参数 JSON 中提取 description（仅 Bash 工具有意义）
fn extract_tool_description(tool_name: &str, arguments: &str) -> Option<String> {
    if tool_name != "Bash" {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(arguments)
        .ok()
        .and_then(|v| v.get("description")?.as_str().map(|s| s.to_string()))
}
```

在每处 `ToolCallStatus { ... }` 构造中添加：
```rust
tool_description: extract_tool_description(&tc.name, &tc.arguments),
```

### 3. 标题栏显示 description

**文件**: `src/command/chat/ui/chat.rs`（第 141、147 行）

```rust
// Before:
.map(|tc| format!(" 🔧 执行 {}...", tc.tool_name))
// After:
.map(|tc| {
    if let Some(ref desc) = tc.tool_description {
        format!(" 🔧 执行 {} - {}...", tc.tool_name, desc)
    } else {
        format!(" 🔧 执行 {}...", tc.tool_name)
    }
})
```

同样的修改应用于 "调用" 行。

### 4. 消息气泡中折叠模式显示 description

**文件**: `src/command/chat/render/cache.rs`（`render_tool_call_request_msg` 函数，约第 1461-1478 行）

在折叠模式中，在工具名后面优先显示 description（若存在），替代 raw arguments preview：

```rust
// 折叠模式：图标 + 工具名 + description（若有）或参数预览
let display_preview = extract_description_from_args(&tc.name, &tc.arguments)
    .unwrap_or_else(|| /* 原有的 args_preview 逻辑 */);
```

新增辅助函数 `extract_description_from_args`，从 JSON arguments 中提取 `description` 字段。

## 影响范围

| 文件 | 修改内容 |
|------|---------|
| `app/types.rs` | `ToolCallStatus` 增加 `tool_description: Option<String>` |
| `app/chat_app.rs` | 3 处 `ToolCallStatus` 构造增加字段 + 1 个辅助函数 |
| `ui/chat.rs` | 标题栏 2 处显示逻辑 |
| `render/cache.rs` | 折叠模式气泡显示逻辑 + 辅助函数 |

## 不涉及

- 不修改 `ToolCallItem`（持久化结构，无需存储 description）
- 不修改 `ShellParams` 本身
- 不修改其他工具的参数结构
