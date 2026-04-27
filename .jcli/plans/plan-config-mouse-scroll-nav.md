# 优化鼠标滚轮在不同模式下的导航行为

## 问题分析

当前 `tui_loop.rs:192-204` 中，鼠标滚轮事件（`ScrollUp`/`ScrollDown`）**硬编码**为 `Action::Scroll`（消息列表滚动），无论当前处于什么模式。这导致在 Config、SelectModel、SelectTheme、ArchiveList 等模式下，鼠标滚轮只会滚动背后的消息列表，而不会导航当前界面的选项列表，体验很差。

## 方案

在 `dispatch_event` 的鼠标滚轮处理中，根据当前 `app.ui.mode` 分发不同的 Action：

| 模式 | 滚轮行为 | 对应 Action |
|------|---------|------------|
| `Chat` | 滚动消息 | `Action::Scroll` (不变) |
| `Config` (非编辑态) | 移动配置字段指针 | `Action::ConfigNavigate` |
| `Config` (编辑中/子Tab) | 保持 `Scroll` | 不变 |
| `SelectModel` | 移动模型选择指针 | `Action::ModelSelectNavigate` |
| `SelectTheme` | 移动主题选择指针 | `Action::ThemeSelectNavigate` |
| `ArchiveList` | 移动归档列表指针 | `Action::ArchiveListNavigate` |
| `Browse` | 滚动浏览消息 | `Action::Scroll` (不变) |
| 其他模式 | 保持 `Scroll` | 不变 |

## 修改文件

### `src/command/chat/handler/tui_loop.rs`

**仅修改一处**：`dispatch_event` 函数中的 `Event::Mouse` 分支（约 192-204 行）。

将：
```rust
Event::Mouse(mouse) if *mouse_capture_enabled => match mouse.kind {
    MouseEventKind::ScrollUp => {
        app.update(Action::Scroll(CursorDirection::Up));
        *needs_redraw = true;
        false
    }
    MouseEventKind::ScrollDown => {
        app.update(Action::Scroll(CursorDirection::Down));
        *needs_redraw = true;
        false
    }
    _ => false,
},
```

改为：
```rust
Event::Mouse(mouse) if *mouse_capture_enabled => match mouse.kind {
    MouseEventKind::ScrollUp => {
        let action = mouse_scroll_action(&app, CursorDirection::Up);
        app.update(action);
        *needs_redraw = true;
        false
    }
    MouseEventKind::ScrollDown => {
        let action = mouse_scroll_action(&app, CursorDirection::Down);
        app.update(action);
        *needs_redraw = true;
        false
    }
    _ => false,
},
```

并新增一个辅助函数：
```rust
/// 根据当前 ChatMode 将鼠标滚轮事件路由到对应的导航 Action
fn mouse_scroll_action(app: &ChatApp, dir: CursorDirection) -> Action {
    match app.ui.mode {
        ChatMode::Config if !app.ui.config_editing => Action::ConfigNavigate(dir),
        ChatMode::SelectModel => Action::ModelSelectNavigate(dir),
        ChatMode::SelectTheme => Action::ThemeSelectNavigate(dir),
        ChatMode::ArchiveList => Action::ArchiveListNavigate(dir),
        _ => Action::Scroll(dir),
    }
}
```

**不新增文件、不新增 Action、不新增 update 方法**，仅修改一处事件分发逻辑。

## 影响范围

- 修改量极小：1 个文件，新增 1 个辅助函数 + 修改 2 行调用
- 不影响任何键盘操作逻辑
- 所有现有 Action 和 update 方法都已存在，无需新增
- 向后兼容
