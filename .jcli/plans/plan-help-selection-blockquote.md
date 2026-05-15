# Help UI 改进计划：文字选中复制 + Blockquote 样式对齐

## 目标

1. **支持鼠标选中复制文字**：在右侧内容区支持鼠标拖拽选择文字，松开后高亮选区，支持 Ctrl+C 复制
2. **Blockquote 样式对齐 Thinking block**：引用块的背景色改为 `bg_primary`（与整体背景融合），竖线样式保持一致

## 现状分析

### Thinking block 渲染 (`msg_render.rs:59-129`)
```rust
// 引用块配色（背景使用 bg_primary，与整体融合）
let bar_color = theme.md_blockquote_bar;
let text_color = theme.md_blockquote_text;
let bg_color = theme.bg_primary;  // ← 关键：背景色是主背景色

// 每行结构：[空格缩进] + [竖线|] + [内容]
lines.push(Line::from(vec![
    Span::styled("  ", Style::default()),
    Span::styled("| ", bar_style),
    Span::styled(content, text_style),
]));
```

### 当前 Blockquote 渲染 (`markdown/render/block.rs:199-232`)
```rust
// 使用 md_blockquote_bg 作为背景（独立背景色，与主背景不同）
let bar_style = Style::default()
    .fg(ctx.theme.md_blockquote_bar())
    .bg(ctx.theme.md_blockquote_bg())  // ← 问题：独立的背景色
```

### Chat UI 鼠标选区实现
- `MouseSelection` 结构体：存储 anchor/current 坐标
- `screen_to_text_pos()`：屏幕坐标 → 全局行号 + 字符偏移
- `rebuild_spans_with_selection()`：对渲染 spans 应用选区高亮
- `copy_selection_to_clipboard()`：复制到剪贴板

## 实施方案

### 1. Blockquote 样式对齐

**改动文件**：`src/markdown/render/block.rs`

修改 `render_blockquote` 函数：
```rust
fn render_blockquote(blocks: &[Block], ctx: &RenderContext) -> Vec<Line<'static>> {
    // 背景色使用 bg_primary（与 thinking block 一致）
    let bg_color = ctx.theme.bg_primary();
    let bar_style = Style::default()
        .fg(ctx.theme.md_blockquote_bar())
        .bg(bg_color)
        .add_modifier(Modifier::BOLD);
    let text_style = Style::default()
        .fg(ctx.theme.md_blockquote_text())
        .bg(bg_color);

    // 添加前导空行
    lines.push(Line::from(""));

    for block in blocks {
        for inner_line in render_block(block, ctx) {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default().bg(bg_color)),
                Span::styled("| ", bar_style),
                // 内容 spans，统一设置 bg_primary 背景
                ...inner_line.spans.map(|s| s.style.bg(bg_color)),
            ]));
        }
    }

    // 后导空行
    lines.push(Line::from(""));
}
```

### 2. 鼠标选区支持

**改动文件**：
- `src/command/help/app.rs`：添加 `MouseSelection` 状态
- `src/command/help.rs`：添加鼠标拖拽选区处理
- `src/command/help/ui.rs`：渲染选区高亮 + 内容区 inner rect 缓存

#### 2.1 状态模型

```rust
pub struct HelpApp {
    // ... existing fields ...
    /// 鼠标选区状态（拖拽选择文字）
    pub mouse_selection: Option<MouseSelection>,
    /// 内容区 inner rect（用于坐标映射）
    pub content_inner: Option<Rect>,
}

#[derive(Clone, Debug)]
pub struct MouseSelection {
    /// 选区起点（全局行号，行内字符偏移）
    pub anchor: (usize, usize),
    /// 选区当前位置
    pub current: (usize, usize),
}
```

#### 2.2 鼠标事件处理

```rust
// 在 handle_mouse_event 中添加内容区选区逻辑：
MouseEventKind::Down(MouseButton::Left) => {
    // 点击内容区开始选区
    if in_content_area(col, row) {
        let pos = screen_to_content_pos(col, row, content_inner, scroll_offset, lines);
        if let Some(p) = pos {
            app.mouse_selection = Some(MouseSelection { anchor: p, current: p });
        }
    }
}
MouseEventKind::Drag(MouseButton::Left) => {
    // 拖拽更新选区
    if let Some(sel) = &mut app.mouse_selection {
        let pos = screen_to_content_pos(...);
        if let Some(p) = pos {
            sel.current = p;
        }
    }
}
MouseEventKind::Up(MouseButton::Left) => {
    // 松开时选区完成，不高亮（保持选区供复制）
}
```

#### 2.3 Ctrl+C 复制

```rust
KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    if let Some(sel) = &app.mouse_selection {
        copy_help_selection_to_clipboard(app);
    }
}
```

#### 2.4 选区高亮渲染

```rust
// render_content 中：
if let Some(sel) = &app.mouse_selection {
    let (sel_start, sel_end) = compute_line_selection_range(line_idx, sel.anchor, sel.current);
    if sel_start < sel_end {
        let highlighted = rebuild_spans_with_selection(
            &line.spans, 0, sel_start, sel_end,
            Color::White, Color::DarkGray,
        );
        // 渲染高亮行
    }
}
```

## 实施步骤

1. **修改 blockquote 渲染**：背景色改为 `bg_primary`
2. **添加 `MouseSelection` 状态**：在 `HelpApp` 中
3. **添加鼠标选区事件处理**：点击/拖拽/释放
4. **实现选区高亮渲染**：复用 `selection.rs` 工具
5. **实现 Ctrl+C 复制**：提取选区文本并写入剪贴板
6. **编译测试**：`cargo check` + `cargo clippy`

## 文件变更清单

| 文件 | 变更 |
|------|------|
| `src/markdown/render/block.rs` | `render_blockquote` 背景色改为 `bg_primary` |
| `src/command/help/app.rs` | 添加 `MouseSelection` / `content_inner` |
| `src/command/help.rs` | 添加鼠标选区事件处理 + Ctrl+C 复制 |
| `src/command/help/ui.rs` | 渲染选区高亮 + 缓存 `content_inner` |