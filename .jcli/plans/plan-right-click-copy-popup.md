# 右键点击消息弹出复制菜单

## 需求
在 Chat 模式下，右键点击消息区域时，弹出一个小型上下文菜单，只有一个"复制"选项。

## 方案设计

### 1. 新增 UI 状态字段（`ui_state.rs`）

在 `UIState` 中新增：

```rust
/// 右键上下文菜单状态
pub context_menu: Option<ContextMenu>,
```

新增 `ContextMenu` 结构体：

```rust
pub struct ContextMenu {
    /// 右键点击时的全局行号（用于定位对应消息）
    pub global_line: usize,
    /// 菜单显示的屏幕坐标 (col, row)
    pub screen_pos: (u16, u16),
}
```

**复制逻辑**：
- 如果当前有鼠标选区（`mouse_selection`），优先复制选区内容
- 如果没有选区，复制点击位置所在消息的完整内容
```

### 2. 鼠标事件处理（`tui_loop.rs`）

在 `dispatch_event()` 的 `Event::Mouse` 分支中新增：

- `MouseEventKind::Down(MouseButton::Right)`:
  1. 检查点击位置是否在消息区域内（复用 `screen_to_text_pos`）
  2. 若在消息区域内，获取全局行号
  3. 设置 `app.ui.context_menu = Some(ContextMenu { global_line, screen_pos })`

- `MouseEventKind::Down(MouseButton::Left)` (已有逻辑修改):
  - 当右键菜单处于激活状态时：
    - 如果点击在菜单区域内 → 执行复制操作并关闭菜单
    - 如果点击在菜单外 → 关闭菜单

### 3. 右键菜单渲染（新增 `ui/context_menu.rs`）

新建 `src/command/chat/ui/context_menu.rs`：

- `draw_context_menu(f, area, app)`:
  - 如果 `app.ui.context_menu` 为 `Some`，在点击位置附近渲染一个小弹窗
  - 弹窗样式：圆角边框，半透明背景色（与现有 popup 风格一致）
  - 单一选项"复制"，点击即执行
  - 弹窗宽度：约 8-10 字符

### 4. 菜单交互（`tui_loop.rs`）

当 `context_menu` 激活时：

| 事件 | 动作 |
|------|------|
| `Enter` | 执行复制操作并关闭菜单 |
| `Esc` | 关闭菜单 |
| 左键点击菜单内 | 执行复制并关闭 |
| 左键点击菜单外 | 关闭菜单 |

### 5. 整合到现有流程

**`draw_chat_ui()`（`chat.rs`）：**
- 在所有弹窗覆盖层之后，渲染右键菜单

**`dispatch_event()`（`tui_loop.rs`）：**
- 在现有鼠标处理分支中增加 `Down(MouseButton::Right)` 处理
- 当 context_menu 激活时拦截左键点击和键盘事件

## 涉及文件

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/command/chat/app/ui_state.rs` | 修改 | 新增 `ContextMenu` 结构体；`UIState` 新增 `context_menu` 字段 |
| `src/command/chat/ui/context_menu.rs` | **新建** | 右键菜单渲染逻辑 |
| `src/command/chat/ui/chat.rs` | 修改 | 在 `draw_chat_ui()` 中调用右键菜单渲染 |
| `src/command/chat/ui/mod.rs` | 修改 | 新增 `context_menu` 模块声明 |
| `src/command/chat/handler/tui_loop.rs` | 修改 | 处理 `Down(MouseButton::Right)` 事件；菜单激活时处理左键点击和 Enter/Esc |

## UI 效果预览

```
╭──────╮
│ 复制 │
╰──────╯
```

- 弹窗出现在右键点击位置附近
- 使用主题色渲染，与现有 popup 风格一致

## 实现步骤

1. 在 `ui_state.rs` 中定义 `ContextMenu` 结构体和 `UIState` 新字段
2. 新建 `ui/context_menu.rs` 实现渲染
3. 在 `tui_loop.rs` 中添加右键事件处理和菜单交互逻辑
4. 在 `draw_chat_ui()` 中集成菜单渲染
5. 编译测试、`cargo fmt`、`cargo clippy`
