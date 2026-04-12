# Todo 命令面板功能完善计划

## 背景

`j todo` 进入 TUI 后，按 `/` 打开命令面板（CommandPopup）。当前命令面板只有 5 个选项，未覆盖所有 TUI 快捷键功能。

## 现状对比

| 功能 | 快捷键 | 命令面板 | 状态 |
|------|--------|----------|------|
| 切换过滤 | `f` | `filter` | 已覆盖 |
| 添加 | `a` | `add` | 已覆盖 |
| 删除 | `d` | `delete` | 已覆盖 |
| 保存 | `s` | `save` | 已覆盖 |
| 帮助 | `?` | `help` | 已覆盖 |
| **编辑** | `e` | - | **缺失** |
| **切换完成** | `空格/Enter` | - | **缺失** |
| **复制到剪贴板** | `y` | - | **缺失** |
| **上移排序** | `K` | - | **缺失** |
| **下移排序** | `J` | - | **缺失** |
| **退出** | `q/Esc` | - | **缺失** |

## 改动范围

仅涉及 `src/command/todo/app.rs` 一个文件，两处改动：

### 1. 扩展 `CMD_POPUP_ITEMS` 常量

```rust
pub const CMD_POPUP_ITEMS: &[(&str, &str)] = &[
    ("toggle", "切换完成"),
    ("edit", "编辑"),
    ("add", "添加"),
    ("delete", "删除"),
    ("copy", "复制"),
    ("filter", "切换过滤"),
    ("moveup", "上移排序"),
    ("movedown", "下移排序"),
    ("save", "保存"),
    ("quit", "退出"),
    ("help", "帮助"),
];
```

新增 6 项，按使用频率排序。

### 2. 扩展 `handle_command_popup_mode` 中 Enter 匹配分支

在 `KeyCode::Enter` 的 match 中新增 6 个 case：

- `"toggle"` → 调用 `app.toggle_done()`（切换完成后会自动进入 ConfirmReport 模式，此时需要 `return` 避免 mode 被覆盖为 Normal）
- `"edit"` → 进入 Editing 模式（复制现有内容到 input），同快捷键 `e` 的逻辑，`return`
- `"copy"` → 调用 `copy_to_clipboard`，同快捷键 `y` 的逻辑
- `"moveup"` → 调用 `app.move_item_up()`
- `"movedown"` → 调用 `app.move_item_down()`
- `"quit"` → 检查 dirty 状态后退出（return true 或设 message）

对于 `toggle` 和 `edit`，执行后不应将 mode 重置为 Normal（因为 toggle 会进入 ConfirmReport，edit 会进入 Editing），需要 `return` 提前退出。其他命令执行后保持原有的 `app.mode = AppMode::Normal` 逻辑。

## 文件修改清单

| 文件 | 改动类型 | 说明 |
|------|----------|------|
| `src/command/todo/app.rs` | 修改 | 扩展 CMD_POPUP_ITEMS + handle_command_popup_mode Enter 分支 |
