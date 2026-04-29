# Notebook 模块鼠标操作支持计划

## 1. 目标功能

为 notebook TUI 模块添加完整的鼠标交互支持：

| 操作 | 功能 | 适用模式 |
|------|------|----------|
| 左键单击列表项 | 选择对应笔记/目录 | Normal |
| 左键双击列表项 | 进入编辑（文件）或展开/折叠（目录） | Normal |
| 滚轮在列表区 | 切换选择项（上下移动一行） | Normal |
| 滚轮在预览区 | 滚动预览内容（现有功能增强） | Normal/Preview |
| 预览区点击 | 在全屏预览模式下定位滚动位置 | Preview |

## 2. 现状分析

### 已有支持（handler.rs 433-459）
```rust
fn handle_mouse_event(app: &mut NotebookApp, mouse: MouseEvent, frame_area: ratatui::layout::Rect) {
    // 仅处理滚轮，滚动预览区（固定 5 行步长）
    // 缺少：点击选择、双击编辑、列表区/预览区区分
}
```

### 布局结构
```
frame_area
├── chunks[0]: 标题栏 (y: 0-3, height: 3)
├── chunks[1]: 主区域 (height: Min(5))
│   ├── Normal/CommandPopup:
│   │   ├── main_chunks[0]: 笔记列表 (panel_ratio%)
│   │   └── main_chunks[1]: 预览区 (100 - panel_ratio%)
│   ├── Help: 帮助页全屏
│   └── Preview: 预览全屏
├── chunks[2]: 状态栏 (height: 3)
└── chunks[3]: 帮助栏 (height: 1)
```

## 3. 实现方案

### 3.1 数据结构扩展（types.rs）

在 `NotebookApp` 中添加：

```rust
/// 上次鼠标点击时间（用于双击检测）
last_click_time: Option<std::time::Instant>,
/// 上次点击位置（行列，用于双击位置判定）
last_click_pos: Option<(u16, u16)>,
/// 上次点击选中的索引（用于双击索引判定）
last_click_index: Option<usize>,
```

### 3.2 布局区域缓存（handler.rs）

需要缓存各区域的精确位置，用于点击判定：

```rust
/// 鼠标事件处理时需要的布局信息
struct MouseLayoutInfo {
    frame_area: Rect,
    /// 标题栏区域
    title_area: Rect,
    /// 主区域
    main_area: Rect,
    /// 状态栏区域
    status_area: Rect,
    /// 帮助栏区域
    help_area: Rect,
    /// 笔记列表区域（仅在 Normal 模式有效）
    list_area: Option<Rect>,
    /// 预览区域（仅在 Normal 模式有效）
    preview_area: Option<Rect>,
}
```

计算逻辑：
- 标题栏：`y = frame_area.y, height = 3`
- 主区域：`y = frame_area.y + 3, height = frame_area.height - 7`
- 状态栏：`y = frame_area.y + frame_area.height - 4, height = 3`
- 帮助栏：`y = frame_area.y + frame_area.height - 1, height = 1`
- 列表区宽度：`frame_area.width * panel_ratio / 100`
- 预览区宽度：`frame_area.width * (100 - panel_ratio) / 100`

### 3.3 鼠标事件处理重构（handler.rs）

```rust
fn handle_mouse_event(
    app: &mut NotebookApp,
    mouse: MouseEvent,
    layout: MouseLayoutInfo,
    // 用于双击后触发编辑
) -> Option<MouseAction> {
    // 仅处理 Normal 和 Preview 模式
    if !matches!(app.mode, AppMode::Normal | AppMode::Preview) {
        return None;
    }

    match mouse.kind {
        // === 左键按下 ===
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(app, mouse.column, mouse.row, &layout)
        }

        // === 滚轮 ===
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            handle_scroll(app, mouse.column, mouse.row, &layout, mouse.kind)
        }

        _ => None,
    }
}

enum MouseAction {
    /// 需要进入编辑（双击文件条目）
    RequestEdit(String),
    /// 无额外动作
    None,
}
```

### 3.4 左键点击处理

```rust
fn handle_left_click(
    app: &mut NotebookApp,
    col: u16,
    row: u16,
    layout: &MouseLayoutInfo,
) -> Option<MouseAction> {
    // Preview 模式：点击预览区定位滚动
    if app.mode == AppMode::Preview {
        if layout.main_area.contains_position(col, row) {
            // 计算点击位置对应的预览行索引
            let relative_y = row - layout.main_area.y;
            app.preview_scroll = relative_y.saturating_sub(1); // 减去边框
        }
        return None;
    }

    // Normal 模式：点击列表区选择
    if let Some(list_area) = layout.list_area {
        if list_area.contains_position(col, row) {
            let inner_y = row - list_area.y - 1; // 减去顶部边框
            if inner_y >= 0 && inner_y < list_area.height - 2 {
                let index = inner_y as usize;
                if index < app.flat_entries.len() {
                    let now = std::time::Instant::now();
                    
                    // 双击检测：时间 < 500ms 且位置和索引相同
                    let is_double_click = app.last_click_time
                        .map(|t| now.duration_since(t).as_millis() < 500)
                        .unwrap_or(false)
                        && app.last_click_pos == Some((col, row))
                        && app.last_click_index == Some(index);

                    app.state.select(Some(index));
                    app.preview_scroll = 0;
                    app.update_preview();

                    // 记录本次点击
                    app.last_click_time = Some(now);
                    app.last_click_pos = Some((col, row));
                    app.last_click_index = Some(index);

                    // 双击动作
                    if is_double_click {
                        let entry = &app.flat_entries[index];
                        match &entry.kind {
                            FlatEntryKind::File { .. } => {
                                return app.selected_name().map(MouseAction::RequestEdit);
                            }
                            FlatEntryKind::Dir { dir_path, .. } => {
                                // 展开/折叠目录
                                app.expanded_dirs.toggle(dir_path);
                                super::io::save_expanded_dirs(&app.expanded_dirs);
                                app.build_flat_entries();
                                app.update_preview();
                            }
                        }
                    }
                }
            }
        }
    }

    None
}
```

### 3.5 滚轮处理改进

```rust
fn handle_scroll(
    app: &mut NotebookApp,
    col: u16,
    row: u16,
    layout: &MouseLayoutInfo,
    kind: MouseEventKind,
) -> Option<MouseAction> {
    let direction = match kind {
        MouseEventKind::ScrollUp => -1,
        MouseEventKind::ScrollDown => 1,
        _ => return None,
    };

    // Preview 模式：仅滚动预览
    if app.mode == AppMode::Preview {
        app.preview_scroll = if direction < 0 {
            app.preview_scroll.saturating_sub(3)
        } else {
            app.preview_scroll.saturating_add(3)
        };
        return None;
    }

    // Normal 模式：根据鼠标位置区分列表区/预览区
    if let Some(list_area) = layout.list_area {
        if let Some(preview_area) = layout.preview_area {
            if list_area.contains_position(col, row) {
                // 列表区：切换选择项
                if direction < 0 {
                    app.move_up();
                } else {
                    app.move_down();
                }
            } else if preview_area.contains_position(col, row) {
                // 预览区：滚动预览内容
                app.preview_scroll = if direction < 0 {
                    app.preview_scroll.saturating_sub(3)
                } else {
                    app.preview_scroll.saturating_add(3)
                };
            }
        }
    }

    None
}
```

### 3.6 TUI Loop 集成（handler.rs）

**现有代码（410-420 行）**：
```rust
Event::Mouse(mouse) => {
    handle_mouse_event(&mut app, mouse, terminal.get_frame().area());
    while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
        if let Ok(Event::Mouse(m)) = event::read() {
            handle_mouse_event(&mut app, m, terminal.get_frame().area());
        }
    }
}
```

**改造方案**：
```rust
Event::Mouse(mouse) => {
    let frame_area = terminal.get_frame().area();
    let layout = compute_mouse_layout(frame_area, &app);
    let action = handle_mouse_event(&mut app, mouse, layout);
    
    // 处理双击编辑请求
    if let Some(MouseAction::RequestEdit(title)) = action {
        let needs_reload = edit_note_on_terminal(&title, &mut terminal);
        if needs_reload {
            app.reload();
        } else {
            app.update_preview();
        }
        // 清空事件队列
        while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
            let _ = event::read();
        }
    }
    
    // 消费后续鼠标事件（防止拖拽产生的冗余事件）
    while event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
        if let Ok(Event::Mouse(m)) = event::read() {
            let _ = handle_mouse_event(&mut app, m, layout);
        }
    }
}

fn compute_mouse_layout(frame_area: Rect, app: &NotebookApp) -> MouseLayoutInfo {
    let title_area = Rect {
        x: frame_area.x,
        y: frame_area.y,
        width: frame_area.width,
        height: 3,
    };
    let main_area = Rect {
        x: frame_area.x,
        y: frame_area.y + 3,
        width: frame_area.width,
        height: frame_area.height.saturating_sub(7),
    };
    let status_area = Rect {
        x: frame_area.x,
        y: frame_area.y + frame_area.height.saturating_sub(4),
        width: frame_area.width,
        height: 3,
    };
    let help_area = Rect {
        x: frame_area.x,
        y: frame_area.y + frame_area.height.saturating_sub(1),
        width: frame_area.width,
        height: 1,
    };

    // Normal 模式下计算列表/预览区域
    let (list_area, preview_area) = if matches!(app.mode, AppMode::Normal | AppMode::CommandPopup) {
        let list_width = frame_area.width * app.panel_ratio / 100;
        let preview_width = frame_area.width - list_width;
        (
            Some(Rect { x: frame_area.x, y: main_area.y, width: list_width, height: main_area.height }),
            Some(Rect { x: frame_area.x + list_width, y: main_area.y, width: preview_width, height: main_area.height }),
        )
    } else {
        (None, None)
    };

    MouseLayoutInfo {
        frame_area,
        title_area,
        main_area,
        status_area,
        help_area,
        list_area,
        preview_area,
    }
}
```

## 4. 文件修改清单

| 文件 | 修改内容 |
|------|----------|
| `types.rs` | 添加 `last_click_time`, `last_click_pos`, `last_click_index` 字段 |
| `handler.rs` | 重构 `handle_mouse_event`，添加 `MouseLayoutInfo`、`MouseAction`、`handle_left_click`、`handle_scroll`、`compute_mouse_layout` |
| `ui.rs` | 无需修改（布局信息在事件处理时动态计算） |

## 5. 测试要点

1. **单击选择**：点击列表任意位置，选中对应笔记
2. **双击编辑**：快速双击文件条目，进入 Markdown 编辑器
3. **双击目录**：快速双击目录条目，展开/折叠切换
4. **滚轮列表区**：鼠标在列表区滚动，切换选中项
5. **滚轮预览区**：鼠标在预览区滚动，滚动预览内容
6. **Preview 模式点击**：全屏预览时点击，定位到对应行
7. **Preview 模式滚轮**：全屏预览时滚动，滚动内容

## 6. 遵循项目规范

- 使用 `snake_case` 命名
- 通过 `cargo fmt` 格式化
- 通过 `cargo clippy -- -D warnings` 检查
- 避免 `unwrap()`，使用 `Option`/`Result` 处理
- 类型定义与 impl 块在同一文件相邻
- 添加必要的文档注释