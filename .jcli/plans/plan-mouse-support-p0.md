# Markdown 编辑器 P0：鼠标操作支持

## 目标

为 Markdown 编辑器添加鼠标操作支持，实现：
1. **鼠标点击定位光标** — 左键单击将光标移动到点击位置
2. **鼠标拖拽选择文本** — 左键拖拽进入 Visual 模式并选择文本
3. **鼠标滚轮滚动** — ScrollUp/ScrollDown 调整 scroll_offset

## 当前状态分析

### 事件循环（`editor.rs:1041-1083`）
- 只处理 `Event::Key`，完全忽略 `Event::Mouse`
- 循环在 `open_markdown_editor_on_terminal` 函数中
- 调用前已经执行了 `EnableMouseCapture`（在 notebook handler 和 chat tui_loop 中）

### 关键映射方法（`wrap_engine.rs`）
- `visual_to_logical(visual_row) -> (usize, usize)` — 视觉行 → (逻辑行, 起始列)
- `visual_to_logical_line(visual_row) -> usize` — 视觉行 → 逻辑行号（二分查找 O(log n)）
- `logical_to_visual(logical_line, logical_col) -> usize` — 逻辑位置 → 视觉行索引
- `get_visual_line(visual_row) -> Option<&VisualLine>` — 获取视觉行详情
- `get_cached_lines(logical_line) -> &[VisualLine]` — 获取逻辑行所有视觉行
- `visual_offset_of(logical_line) -> usize` — 获取逻辑行的视觉偏移

### 渲染布局（`editor.rs:610-623`）
- 内容区域 = `area` 减去边框（上下各1行，左右各1列）
- `content_height = area.height - 3`（上边框1 + 下边框1 + 状态栏1）
- `content_width = area.width - 2`（左右边框各1）
- 行号宽度：`line_num_width = 6`（显示时）或 `0`（隐藏时）
- 折行宽度：`wrap_width = content_width - line_num_width`
- 内容起始屏幕坐标：`x = area.x + 1`（左边框后），`y = area.y + 1`（上边框后）

### Vim Visual 模式（`vim.rs`）
- 已有 `Mode::Visual` 和 `visual_start: (usize, usize)` 字段
- `handle_visual_mode` 支持方向键/hjkl 移动光标扩展选区
- `y` 键可 yank 选中文本

## 实现方案

### 1. 新增 `MouseAction` 类型和 `handle_mouse` 方法

在 `editor.rs` 中新增鼠标处理方法，而非在 vim 层处理（因为鼠标是独立于 Vim 模式的输入通道）。

```rust
/// 鼠标动作
#[derive(Debug)]
pub enum MouseAction {
    /// 无操作
    None,
    /// 左键按下：移动光标到点击位置
    Click,
    /// 左键拖拽：进入 Visual 模式并更新选区
    Drag,
    /// 滚轮滚动
    Scroll,
}
```

### 2. 核心方法：`screen_to_logical`

将屏幕坐标 (screen_x, screen_y) 转换为逻辑位置 (logical_row, logical_col)。

**坐标转换步骤：**

```
screen_x, screen_y (crossterm 坐标)
  ↓ 减去边框偏移
content_x = screen_x - area.x - 1  (减去左边框)
content_y = screen_y - area.y - 1  (减去上边框)
  ↓ 检查是否在有效区域
  ↓ 加上 scroll_offset 得到视觉行号
visual_row = content_y + scroll_offset
  ↓ 使用 wrap_engine.visual_to_logical 映射
(logical_row, start_col) = wrap_engine.visual_to_logical(visual_row)
  ↓ 减去行号区域得到内容列
content_col = content_x - line_num_width
  ↓ 在该逻辑行的视觉行中精确定位
```

**精确列定位（考虑折行）：**

1. 获取 `logical_row` 的所有视觉行 `vlines`
2. 找到 `visual_row` 对应的子视觉行 `vl`
3. `local_col = vl.start_col + content_col`（简单估算）
4. 但需要考虑宽字符（CJK），通过遍历 `vl.text` 的字符精确计算

**宽字符处理：**

```rust
fn screen_col_to_logical_col(text: &str, screen_col: usize) -> usize {
    let mut acc_width = 0;
    for (i, ch) in text.chars().enumerate() {
        if acc_width >= screen_col {
            return i; // 返回字符偏移
        }
        acc_width += char_width(ch);
    }
    text.chars().count() // 超出范围则返回行尾
}
```

### 3. `handle_mouse` 方法

在 `MarkdownEditor` 上新增：

```rust
pub fn handle_mouse(
    &mut self,
    mouse: crossterm::event::MouseEvent,
    area: ratatui::layout::Rect,
) -> MouseAction {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => { ... }
        MouseEventKind::Drag(MouseButton::Left) => { ... }
        MouseEventKind::Up(MouseButton::Left) => { ... }
        MouseEventKind::ScrollUp => { ... }
        MouseEventKind::ScrollDown => { ... }
        _ => MouseAction::None,
    }
}
```

#### 3.1 左键按下（Click）

1. 调用 `screen_to_logical` 得到 `(row, col)`
2. 如果当前是 Visual 模式以外的模式，退出到 Normal 模式
3. `buffer.set_cursor(row, col)`
4. 记录点击位置 `mouse_anchor = Some((row, col))`（用于后续拖拽）
5. 返回 `MouseAction::Click`

#### 3.2 左键拖拽（Drag）

1. 调用 `screen_to_logical` 得到 `(row, col)`
2. 如果 `mouse_anchor.is_some()` 且不是 Visual 模式：
   - 进入 Visual 模式：`vim.set_mode(Mode::Visual)`
   - 设置 `vim.visual_start = mouse_anchor.unwrap()`
3. 更新光标位置：`buffer.set_cursor(row, col)`
4. 返回 `MouseAction::Drag`

#### 3.3 左键释放（Up）

1. 清除 `mouse_anchor = None`
2. 返回 `MouseAction::None`

#### 3.4 滚轮上（ScrollUp）

1. `scroll_offset = scroll_offset.saturating_sub(3)`（每次滚动3行，符合常见编辑器体验）
2. 确保 `scroll_offset >= 0`
3. 不移动光标位置（仅调整视口）
4. 返回 `MouseAction::Scroll`

#### 3.5 滚轮下（ScrollDown）

1. `scroll_offset += 3`
2. 确保不超过最大视觉行数：`scroll_offset = min(scroll_offset, total_visual_lines.saturating_sub(viewport_height))`
3. 返回 `MouseAction::Scroll`

### 4. 新增字段

在 `MarkdownEditor` 结构体中新增：

```rust
/// 鼠标拖拽锚点（按下时的位置）
mouse_anchor: Option<(usize, usize)>,
```

初始化为 `None`。

### 5. 修改事件循环

在 `editor.rs` 的 `open_markdown_editor_on_terminal` 函数中，当前只处理 `Event::Key`：

```rust
// 当前代码 (行 1052)
if let Event::Key(key) = evt {
```

修改为同时处理鼠标事件：

```rust
match evt {
    Event::Key(key) => {
        // ... 现有的键盘处理逻辑不变
    }
    Event::Mouse(mouse) => {
        editor.handle_mouse(mouse, area);
    }
    _ => {}
}
```

### 6. 滚动后确保光标可见

在滚轮滚动场景中，只调整 `scroll_offset`，不移动光标。但如果光标落在可视区域外，在下次 `render()` 调用时 `update_scroll_from_visual` 会自动将视口调整回光标位置。

**问题**：这会导致滚轮滚动后视口立刻跳回。

**解决方案**：在 `MarkdownEditor` 中新增 `scroll_lock` 标志：

```rust
/// 滚轮滚动锁定：防止 render() 自动将视口拉回到光标位置
scroll_locked: bool,
```

- 滚轮滚动时设置 `scroll_locked = true`
- 键盘输入（按键）时重置 `scroll_locked = false`
- `render()` 中：当 `scroll_locked == true` 时，跳过 `update_scroll_from_visual`

### 7. 需要暴露的 Vim 方法

`visual_start` 字段目前是私有的，需要添加 setter：

```rust
/// 设置 Visual 模式的选区起点
pub fn set_visual_start(&mut self, pos: (usize, usize)) {
    self.visual_start = pos;
}
```

## 修改文件清单

| 文件 | 修改内容 |
|------|---------|
| `src/tui/editor_core/editor.rs` | 新增 `MouseAction`、`handle_mouse`、`screen_to_logical`、`mouse_anchor`/`scroll_locked` 字段、修改事件循环、修改 `render` 中的滚动逻辑 |
| `src/tui/editor_core/vim.rs` | 新增 `set_visual_start` 公共方法 |

## 实现顺序

1. 在 `vim.rs` 中添加 `set_visual_start` 方法
2. 在 `editor.rs` 结构体中添加 `mouse_anchor` 和 `scroll_locked` 字段
3. 实现 `screen_to_logical` 私有方法
4. 实现 `handle_mouse` 公共方法
5. 修改 `render` 方法，支持 `scroll_locked` 时跳过自动滚动
6. 修改事件循环，添加 `Event::Mouse` 分支
7. 测试验证
