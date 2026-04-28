# Chat UI 鼠标字符级选择实现方案

## 目标

让 Chat UI 消息区支持鼠标拖拽选中文字，松开后自动复制到剪贴板，体验与 Markdown 编辑器一致。

## 现有架构分析

### 渲染数据结构

```
MsgLinesCache
├── per_msg_lines: Vec<PerMsgCache>  — 每条历史消息的渲染缓存
│   └── PerMsgCache { lines: Vec<Line<'static>>, msg_index, content_len }
├── streaming_lines: Vec<Line<'static>> — 流式内容渲染行
├── msg_start_lines: Vec<(usize, usize)> — 每条消息的起始全局行号
├── total_line_count: usize — 总行数
└── history_line_count: usize — 历史消息总行数
```

### Line 结构（ratatui）

```rust
Line { spans: Vec<Span<'static>> }
Span { content: String, style: Style }
```

### 渲染方式

`render_text_pass` 遍历可见行（`start..end`），每行用 `Paragraph::new(line)` 渲染到 `Rect::new(inner.x, y, inner.width, 1)`。

### 坐标体系

- `scroll_offset: u16` — 滚动偏移（全局行号）
- `inner: Rect` — 消息区域（不含边框）
- 屏幕 y → 全局行号：`global_line = scroll_offset + (screen_y - inner.y)`
- 全局行号 → Line：通过 `get_line_at(cached, global_line, history_total)` 获取
- 屏幕 x → 字符偏移：需累加 spans 的显示宽度（CJK 宽字符占 2 列）

---

## 实现方案

### 1. UI 状态新增字段（`ui_state.rs`）

```rust
/// 鼠标选区状态
pub struct MouseSelection {
    /// 选区起点（全局行号，行内字符偏移）
    pub anchor: (usize, usize),
    /// 选区当前位置（全局行号，行内字符偏移）
    pub current: (usize, usize),
    /// 是否正在进行拖拽选择
    pub active: bool,
}

// UIState 中新增：
pub mouse_selection: Option<MouseSelection>,
```

### 2. 坐标映射函数（新增于 `ui/chat.rs`）

```rust
/// 将屏幕坐标转换为 (全局行号, 行内字符偏移)
/// 返回 None 表示点击在消息区域外或空白区域
fn screen_to_text_pos(
    screen_x: u16,
    screen_y: u16,
    inner: Rect,
    scroll_offset: u16,
    cached: &MsgLinesCache,
    history_total: usize,
) -> Option<(usize, usize)> {
    // 1. 计算全局行号
    let local_y = screen_y.saturating_sub(inner.y);
    if local_y >= inner.height {
        return None;
    }
    let global_line = scroll_offset as usize + local_y as usize;
    if global_line >= cached.total_line_count {
        return None;
    }

    // 2. 获取该行的 Line
    let line = get_line_at(cached, global_line, history_total)?;

    // 3. 计算行内字符偏移（考虑 CJK 宽字符）
    let local_x = screen_x.saturating_sub(inner.x) as usize;
    let char_offset = spans_to_char_offset(&line.spans, local_x);

    Some((global_line, char_offset))
}

/// 根据 spans 和屏幕 x 坐标计算字符偏移
fn spans_to_char_offset(spans: &[Span<'static>], screen_col: usize) -> usize {
    let mut acc_width = 0usize;
    let mut char_offset = 0usize;

    for span in spans {
        for ch in span.content.chars() {
            let w = char_width(ch);
            if acc_width >= screen_col {
                return char_offset;
            }
            acc_width += w;
            char_offset += 1;
        }
    }
    char_offset
}
```

### 3. 鼠标事件处理（`tui_loop.rs`）

在 `dispatch_event` 中添加鼠标事件分支：

```rust
Event::Mouse(mouse) if *mouse_capture_enabled => {
    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // 开始选择：设置 anchor 和 current
            if let Some((gline, coff)) = screen_to_text_pos(...) {
                app.ui.mouse_selection = Some(MouseSelection {
                    anchor: (gline, coff),
                    current: (gline, coff),
                    active: true,
                });
            }
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            // 拖拽：更新 current
            if let Some(ref sel) = app.ui.mouse_selection {
                if let Some((gline, coff)) = screen_to_text_pos(...) {
                    app.ui.mouse_selection = Some(MouseSelection {
                        anchor: sel.anchor,
                        current: (gline, coff),
                        active: true,
                    });
                    *needs_redraw = true;
                }
            }
        }
        MouseEventKind::Up(MouseButton::Left) => {
            // 松开：提取文本并复制到剪贴板
            if let Some(ref sel) = app.ui.mouse_selection {
                let text = extract_selection_text(app, sel.anchor, sel.current);
                if !text.is_empty() {
                    copy_to_clipboard(&text);
                    app.show_toast("已复制到剪贴板", false);
                }
                app.ui.mouse_selection = None;
                *needs_redraw = true;
            }
        }
        MouseEventKind::ScrollUp / ScrollDown => { ... } // 保持现有滚动逻辑
        _ => {}
    }
}
```

### 4. 选区文本提取（新增函数）

```rust
/// 根据 anchor 和 current 提取选区纯文本
fn extract_selection_text(
    app: &ChatApp,
    anchor: (usize, usize),
    current: (usize, usize),
) -> String {
    // 确保 start <= end
    let ((sr, sc), (er, ec)) = if anchor.0 < current.0 
        || (anchor.0 == current.0 && anchor.1 <= current.1) {
        (anchor, current)
    } else {
        (current, anchor)
    };

    let cached = app.ui.msg_lines_cache.as_ref()?;
    let history_total = cached.history_line_count;

    let mut result = String::new();
    for gline in sr..=er {
        let line = get_line_at(cached, gline, history_total)?;
        let line_text: String = line.spans.iter().map(|s| s.content.as_str()).collect();
        
        // 计算该行的截取范围
        let start_col = if gline == sr { sc } else { 0 };
        let end_col = if gline == er { ec } else { line_text.chars().count() };
        
        let chars: Vec<char> = line_text.chars().collect();
        if start_col < end_col && start_col < chars.len() {
            let slice: String = chars[start_col..end_col.min(chars.len())].iter().collect();
            result.push_str(&slice);
            if gline < er {
                result.push('\n');
            }
        }
    }
    result
}
```

### 5. 渲染选区高亮（修改 `render_text_pass`）

```rust
fn render_text_pass(f: &mut Frame, params: &TextPassParams, selection: Option<&MouseSelection>) {
    for (i, line_idx) in (params.start..params.end).enumerate() {
        let line = get_line_at(params.cached, line_idx, params.history_total)?;
        let y = params.inner.y + i as u16;
        let line_area = Rect::new(params.inner.x, y, params.inner.width, 1);

        // 检查该行是否在选区内
        let (sel_start, sel_end) = if let Some(ref sel) = selection {
            compute_line_selection_range(line_idx, sel.anchor, sel.current)
        } else {
            (0, 0) // 无选区
        };

        if sel_start < sel_end {
            // 有选区：重建 spans，选中部分高亮
            let highlighted_spans = rebuild_spans_with_selection(
                &line.spans,
                sel_start,
                sel_end,
                params.msg_area_bg.fg.unwrap_or(Color::White),
                Color::DarkGray, // 选中背景色
            );
            let p = Paragraph::new(Line::from(highlighted_spans)).style(params.msg_area_bg);
            f.render_widget(p, line_area);
        } else {
            // 无选区：正常渲染
            let p = Paragraph::new(line.clone()).style(params.msg_area_bg);
            f.render_widget(p, line_area);
        }
    }
}

/// 计算某全局行与选区的交集字符范围
fn compute_line_selection_range(
    line_idx: usize,
    anchor: (usize, usize),
    current: (usize, usize),
) -> (usize, usize) {
    let ((sr, sc), (er, ec)) = if anchor.0 < current.0 
        || (anchor.0 == current.0 && anchor.1 <= current.1) {
        (anchor, current)
    } else {
        (current, anchor)
    };

    if line_idx < sr || line_idx > er {
        return (0, 0); // 无交集
    }

    let start = if line_idx == sr { sc } else { 0 };
    let end = if line_idx == er { ec } else { usize::MAX }; // MAX 表示到行尾
    
    (start, end)
}
```

### 6. 复用 Markdown 编辑器的 span 分割逻辑

`rebuild_spans_with_selection` 可以直接复用 `editor.rs` 中刚实现的函数（或提取为公共模块）：

```rust
// 从 editor.rs 复制或提取到公共模块
fn rebuild_spans_with_selection(
    spans: &[Span<'static>],
    char_start: usize,
    char_end: usize,
    sel_fg: Color,
    sel_bg: Color,
) -> Vec<Span<'static>> { ... }
```

---

## 修改文件清单

| 文件 | 修改内容 |
|------|----------|
| `ui_state.rs` | 新增 `MouseSelection` 结构体和 `mouse_selection` 字段 |
| `tui_loop.rs` | 鼠标事件处理（Down/Drag/Up） |
| `ui/chat.rs` | `screen_to_text_pos`、`render_text_pass` 选区高亮、`extract_selection_text` |
| `render/cache/clipboard.rs` | 已有，无需修改 |
| `render/cache.rs` 或新建公共模块 | 提取 `rebuild_spans_with_selection` |

---

## 测试要点

1. 单行内选部分文字 → 只有选中部分高亮，松开后复制正确
2. 跨多行选择 → 起始/结束行部分高亮，中间行整行高亮，复制包含换行
3. 点击空白区域 → 不触发选择
4. 滚动后选择 → 坐标映射正确
5. 流式输出时选择 → 可选中已渲染的内容
6. CJK 字符 → 宽字符坐标映射正确（点击字符中间也能定位）

---

## 工作量估计

约 3-4 小时，核心难点是：
1. 坐标映射（特别是 CJK 宽字符）
2. span 分割逻辑复用/提取
3. 与现有滚动逻辑的协调