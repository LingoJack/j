# 计划：所有 tab 页面支持 `[` / `]` 切换左右 tab

## 需求分析
在 jcli 所有含 tab 切换功能的 TUI 界面中，**非输入模式**下添加 `[` / `]` 键支持：
- `[` → 切换到上一个 tab（左移）
- `]` → 切换到下一个 tab（右移）

## 涉及的模块和修改

### 1. Chat Config 模式 — `src/command/chat/handler/config.rs`
**现状**：`Left/Right` 已映射到 `ConfigSwitchTab`，但 `[`/`]` 未映射。
**修改**：在所有 `ConfigTab::*` 分支的 `match key.code` 中添加 `KeyCode::Char('[')` 和 `KeyCode::Char(']')`，分别映射到 `ConfigSwitchTab(CursorDirection::Up)` 和 `ConfigSwitchTab(CursorDirection::Down)`。

涉及分支（共 7 处，需在每个分支添加）：
- `ConfigTab::Model`
- `ConfigTab::Global`（需注意 `compact_exempt_sublist` 子模式不处理）
- `ConfigTab::Tools`
- `ConfigTab::Skills`
- `ConfigTab::Hooks`
- `ConfigTab::Commands`
- `ConfigTab::Teammates`
- `ConfigTab::Archive`（非确认还原模式）
- `ConfigTab::Session`（非确认恢复模式）

**优化方案**：与其在每个分支中重复添加，不如在 `config.rs` 的 `handle_config_mode` 函数入口处（`config_editing` 检查之后、`config_tab` match 之前）统一拦截 `[` / `]`，减少代码重复。

### 2. Help 模式 — `src/command/help.rs`
**现状**：`Left/Right`/`h/l` 已映射到 `prev_tab/next_tab`，但 `[`/`]` 未映射。
**修改**：在 `handle_normal_key` 函数的 `match key.code` 中添加：
- `KeyCode::Char('[')` → `app.prev_tab()`
- `KeyCode::Char(']')` → `app.next_tab()`

### 3. Notebook 模式 — `src/command/notebook/app.rs`
**现状**：`[` / `]` 已用于面板比例调整（`handle_normal_mode`）。
**不需要修改**：Notebook 没有 tab 切换功能，`[`/`]` 已有其他用途。

### 4. Todo 模式 — `src/command/todo/app.rs`
**不需要修改**：Todo 没有 tab 切换功能。

## 具体修改内容

### 文件 1：`src/command/chat/handler/config.rs`
在 `handle_config_mode` 函数中，在 `if app.ui.config_editing { ... return; }` 之后、`let action = match app.ui.config_tab {` 之前，添加统一拦截：

```rust
// 统一拦截 [ / ] 切换 tab（非编辑模式下）
if !app.ui.config_editing {
    match key.code {
        KeyCode::Char('[') => {
            app.update(Action::ConfigSwitchTab(CursorDirection::Up));
            return;
        }
        KeyCode::Char(']') => {
            app.update(Action::ConfigSwitchTab(CursorDirection::Down));
            return;
        }
        _ => {}
    }
}
```

但这样会在每个 config_tab 分支中仍然保留 `Left/Right` 的映射，不会冲突。不过需要确认 `compact_exempt_sublist` 模式下也支持（它没有 tab 切换，不需要）。

实际上更好的做法是：只在 config_tab match 之前拦截 `[`/`]`，各分支已有的 `Left/Right` 不受影响。注意 `compact_exempt_sublist` 模式没有 tab 切换，不应拦截。因此在 `compact_exempt_sublist` 检查之后添加。

### 文件 2：`src/command/help.rs`
在 `handle_normal_key` 函数中添加两行：
```rust
KeyCode::Char('[') => app.prev_tab(),
KeyCode::Char(']') => app.next_tab(),
```
放在 Tab 切换区域（`Right/Left/h/l` 附近）。
