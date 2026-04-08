# Plan: 实现自研文本编辑器替代 tui-textarea

## 问题分析

当前问题：
1. `tui-textarea` 不支持自动折行
2. 库的设计理念与需求不符
3. 受限于第三方库的能力

用户反馈：`tui-textarea` 太垃圾，希望完全自己实现

## 解决方案：自研文本编辑器

### 核心设计思路

**完全摆脱 tui-textarea 依赖**，基于 `ratatui` 自己实现一个支持自动折行的 Markdown 编辑器。

### 架构设计

```
┌─────────────────────────────────────────────────────────────┐
│                   MarkdownEditor (新架构)                    │
├─────────────────────────────────────────────────────────────┤
│  TextBuffer                                                 │
│  ├── lines: Vec<String>        // 原始文本行                │
│  ├── cursor: (usize, usize)    // 光标位置 (行, 列)         │
│  ├── selection: Option<Range>  // 选择范围                  │
│  └── history: History          // 撤销/重做栈               │
├─────────────────────────────────────────────────────────────┤
│  WrapEngine                                                 │
│  ├── wrap_enabled: bool        // 是否启用折行              │
│  ├── wrap_width: usize         // 折行宽度                  │
│  └── visual_lines: Vec<VisualLine>  // 视觉行缓存           │
├─────────────────────────────────────────────────────────────┤
│  VimEngine                                                  │
│  ├── mode: Mode               // Normal/Insert/Visual/...   │
│  ├── register: String         // yank 寄存器                │
│  └── handle_input()           // Vim 按键处理               │
├─────────────────────────────────────────────────────────────┤
│  Renderer                                                   │
│  ├── render_line()            // 渲染单行（支持折行）        │
│  ├── render_cursor()          // 渲染光标                   │
│  └── render_status_bar()      // 渲染状态栏                 │
└─────────────────────────────────────────────────────────────┘
```

### 数据结构设计

```rust
/// 视觉行：一个逻辑行可能拆分为多个视觉行
struct VisualLine {
    logical_line: usize,    // 原始行号
    start_col: usize,       // 在原始行中的起始列
    end_col: usize,         // 在原始行中的结束列
    text: String,           // 显示文本
    display_width: usize,   // 显示宽度
}

/// 文本缓冲区
struct TextBuffer {
    lines: Vec<String>,
    cursor: (usize, usize),
    selection_start: Option<(usize, usize)>,
    history: History,
    modified: bool,
}

/// 撤销历史
struct History {
    stack: Vec<TextSnapshot>,
    cursor: usize,
}

/// 折行引擎
struct WrapEngine {
    enabled: bool,
    width: usize,
    cache: Vec<VisualLine>,
    dirty: bool,
}

/// 编辑器主结构
struct MarkdownEditor {
    buffer: TextBuffer,
    wrap: WrapEngine,
    vim: VimEngine,
    search: SearchState,
    theme: Theme,
    viewport: Rect,
    scroll_offset: usize,    // 垂直滚动（视觉行级别）
}
```

### 自动折行实现

```rust
impl WrapEngine {
    /// 将逻辑行拆分为视觉行
    fn wrap_line(&self, line: &str, line_num: usize) -> Vec<VisualLine> {
        if !self.enabled {
            return vec![VisualLine {
                logical_line: line_num,
                start_col: 0,
                end_col: line.chars().count(),
                text: line.to_string(),
                display_width: display_width(line),
            }];
        }

        let mut result = Vec::new();
        let mut current = String::new();
        let mut current_width = 0;
        let mut start_col = 0;
        let mut col = 0;

        for ch in line.chars() {
            let ch_width = if ch.is_ascii() { 1 } else { 2 };
            
            if current_width + ch_width > self.width {
                result.push(VisualLine {
                    logical_line: line_num,
                    start_col,
                    end_col: col,
                    text: current.clone(),
                    display_width: current_width,
                });
                start_col = col;
                current.clear();
                current_width = 0;
            }
            
            current.push(ch);
            current_width += ch_width;
            col += 1;
        }

        if !current.is_empty() {
            result.push(VisualLine {
                logical_line: line_num,
                start_col,
                end_col: col,
                text: current,
                display_width: current_width,
            });
        }

        result
    }

    /// 重建视觉行缓存
    fn rebuild_cache(&mut self, lines: &[String]) {
        self.cache.clear();
        for (i, line) in lines.iter().enumerate() {
            self.cache.extend(self.wrap_line(line, i));
        }
        self.dirty = false;
    }
}
```

### 光标定位

```rust
impl MarkdownEditor {
    /// 逻辑位置 -> 视觉位置
    fn logical_to_visual(&self, logical: (usize, usize)) -> usize {
        let mut visual_row = 0;
        for vl in &self.wrap.cache {
            if vl.logical_line == logical.0 && vl.start_col <= logical.1 && logical.1 < vl.end_col {
                return visual_row;
            }
            visual_row += 1;
        }
        visual_row.saturating_sub(1)
    }

    /// 视觉位置 -> 逻辑位置
    fn visual_to_logical(&self, visual_row: usize) -> (usize, usize) {
        if let Some(vl) = self.wrap.cache.get(visual_row) {
            (vl.logical_line, vl.start_col)
        } else {
            (0, 0)
        }
    }
}
```

### 光标移动逻辑

```rust
impl MarkdownEditor {
    fn move_cursor_up(&mut self) {
        let visual_row = self.logical_to_visual(self.buffer.cursor);
        if visual_row > 0 {
            let target_visual = visual_row - 1;
            // 尝试保持列位置
            let current_col = self.buffer.cursor.1;
            let vl = &self.wrap.cache[target_visual];
            let new_col = current_col.min(vl.end_col.saturating_sub(1)).max(vl.start_col);
            self.buffer.cursor = (vl.logical_line, new_col);
        }
    }

    fn move_cursor_down(&mut self) {
        let visual_row = self.logical_to_visual(self.buffer.cursor);
        if visual_row < self.wrap.cache.len() - 1 {
            let target_visual = visual_row + 1;
            let current_col = self.buffer.cursor.1;
            let vl = &self.wrap.cache[target_visual];
            let new_col = current_col.min(vl.end_col.saturating_sub(1)).max(vl.start_col);
            self.buffer.cursor = (vl.logical_line, new_col);
        }
    }
}
```

### 与现有代码的关系

现有 `editor_markdown.rs` 已经实现的功能可以复用：
- ✅ Vim 模式状态机（可直接复用）
- ✅ 搜索状态和高亮（可直接复用）
- ✅ Markdown 渲染逻辑（需要适配折行）
- ✅ 状态栏渲染（可直接复用）

需要移除的依赖：
- ❌ `tui_textarea::TextArea`
- ❌ `tui_textarea::CursorMove`
- ❌ `tui_textarea::Input`/`Key`

### 实施步骤

#### 阶段一：基础结构（1-2小时）
1. 创建 `text_buffer.rs` - 文本缓冲区
2. 创建 `wrap_engine.rs` - 折行引擎
3. 创建 `editor_core.rs` - 编辑器核心

#### 阶段二：Vim 引擎迁移（1-2小时）
1. 迁移现有 Vim 模式逻辑
2. 移除对 TextArea 的依赖
3. 实现基于视觉行的光标移动

#### 阶段三：渲染适配（1小时）
1. 修改 `render_line()` 支持折行
2. 实现视觉行滚动
3. 光标渲染适配

#### 阶段四：编辑操作（1小时）
1. 实现字符插入/删除
2. 实现行操作（插入、删除、合并）
3. 实现撤销/重做

#### 阶段五：搜索功能（30分钟）
1. 迁移搜索逻辑
2. 搜索高亮适配折行

#### 阶段六：测试和优化（30分钟）
1. 测试各种边界情况
2. 性能优化
3. 快捷键测试

### 文件结构

```
src/tui/
├── editor_markdown.rs      # 主编辑器入口（保留，大幅简化）
├── editor_core/
│   ├── mod.rs              # 模块导出
│   ├── text_buffer.rs      # 文本缓冲区
│   ├── wrap_engine.rs      # 折行引擎
│   ├── history.rs          # 撤销/重做
│   ├── vim_engine.rs       # Vim 模式处理
│   └── search.rs           # 搜索功能
└── renderer/
    ├── mod.rs              # 模块导出
    ├── line_renderer.rs    # 行渲染（支持折行）
    └── markdown_styling.rs # Markdown 样式
```

### 优势

1. **完全自主控制**：不受第三方库限制
2. **原生折行支持**：从底层设计就支持自动折行
3. **针对 Markdown 优化**：可以针对 Markdown 场景做特殊优化
4. **代码更简洁**：移除大量适配代码
5. **更易维护**：完全理解自己的代码

### 风险和注意事项

1. **工作量大**：预计需要 5-7 小时开发时间
2. **测试需求**：需要全面测试各种边界情况
3. **中文支持**：需要确保双宽字符处理正确
4. **性能考量**：大文件可能需要优化折行缓存

## 用户确认

请确认：
1. 是否采用此方案（自研编辑器替代 tui-textarea）？
2. 是否需要分阶段实施，先实现基础功能再逐步完善？
3. 是否有其他特殊需求（如只读模式、多光标等）？
