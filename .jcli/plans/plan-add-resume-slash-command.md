# Plan: 添加 `/resume` 斜杠命令以切换到历史会话

## 目标
在 chat TUI 的斜杠命令中新增 `/resume`，用户输入 `/resume` 后进入一个会话列表选择界面，可以直接切换到以前的某个 session。

## 现有架构分析

### 斜杠命令流程
1. **定义**: `src/command/chat/autocomplete.rs` — `SlashCommand` enum 定义所有斜杠命令变体
2. **执行**: `src/command/chat/handler/chat.rs` — `execute_slash_command()` 函数处理每个命令
3. **渲染**: `src/command/chat/ui/chat.rs` — 渲染斜杠命令弹窗

### 现有会话恢复机制
- Config 界面的 Session Tab 已有完整的 session 列表/恢复/删除功能
- `Action::LoadSessionList` / `Action::SessionListNavigate` / `Action::RestoreSession` 已实现
- `UIState` 中已有 `session_list`, `session_list_index`, `session_restore_confirm` 字段
- `ChatMode` 已有会话切换逻辑

## 实现方案

由于 `/resume` 需要展示一个会话列表供用户选择，最简方案是复用已有的 Session 列表逻辑：`/resume` 命令执行时，加载 session 列表并直接进入 Config 界面的 Session Tab。

### 修改文件清单

#### 1. `src/command/chat/autocomplete.rs`
- 在 `SlashCommand` enum 中新增 `Resume` 变体
- 在 `display_label()` 中返回 `"/resume"`
- 在 `description()` 中返回 `"恢复历史会话"`
- 在 `all()` 中加入 `SlashCommand::Resume`

#### 2. `src/command/chat/handler/chat.rs`
- 在 `execute_slash_command()` 的 match 中新增 `SlashCommand::Resume` 分支
- 逻辑：调用 `Action::LoadSessionList` 加载 session 列表，然后进入 Config 界面的 Session Tab（`ConfigTab::Session`）
- 需要设置 `app.ui.config_tab = ConfigTab::Session`，然后进入 `ChatMode::Config`

### 具体步骤

**Step 1**: `autocomplete.rs` — 添加 `Resume` 变体到 `SlashCommand`
```rust
pub enum SlashCommand {
    // ...existing...
    /// 恢复历史会话
    Resume,
}
```
在 `display_label()`, `description()`, `all()` 中分别添加对应分支。

**Step 2**: `handler/chat.rs` — 在 `execute_slash_command()` 中添加处理逻辑
```rust
SlashCommand::Resume => {
    app.update(Action::LoadSessionList);
    app.ui.config_tab = ConfigTab::Session;
    app.ui.config_scroll_offset = 0;
    app.update(Action::EnterMode(ChatMode::Config));
}
```

### 用户交互流程
1. 用户输入 `/` → 弹出斜杠命令列表
2. 选择 `/resume`（描述："恢复历史会话"）
3. 自动进入 Config 界面的 Session Tab
4. 用户可上下导航选择历史会话，Enter 恢复，`d` 删除，`n` 新建，Esc 返回

### 影响范围
- 仅修改 2 个文件
- 完全复用现有 Config Session Tab 的 UI 和交互逻辑
- 不需要新增 Action、ChatMode 或 UIState 字段
