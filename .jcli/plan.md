# Plan: Markdown 编辑器代码块渲染优化

## 问题分析

用户提出两个问题：
1. **自动补全**：输入 ``` 后回车应该自动补全闭合的 ```
2. **右侧边框**：代码块内容行右边缺少闭合的竖线渲染

## 当前实现分析

### 文件结构
- `src/tui/editor_markdown.rs` - Markdown 编辑器核心，负责编辑和渲染
- `src/command/chat/markdown/parser.rs` - Markdown 解析器（用于聊天消息渲染）
- `src/command/chat/markdown/highlight.rs` - 代码语法高亮

### 现有代码块渲染逻辑（editor_markdown.rs）

1. `render_code_fence_line` (行691-738)
   - 处理 ``` 围栏行的渲染
   - 开始围栏显示 `┌─ lang ────`
   - 结束围栏显示 `└──────`

2. `render_code_block_line` (行740-773)
   - 处理代码块内容行的渲染
   - 只渲染了左侧竖线 `│` + 代码内容
   - **缺失右侧竖线**

3. 输入处理在 `handle_insert_mode` (行254-262)
   - 当前只是简单传递给 textarea
   - **没有自动补全逻辑**

## 实现方案

### 任务1：代码块右侧边框渲染

**修改文件**：`src/tui/editor_markdown.rs`

**修改位置**：`render_code_block_line` 方法

**方案**：
1. 计算当前行的显示宽度
2. 在代码内容右侧填充空格到统一宽度
3. 添加右侧竖线 `│`

**伪代码**：
```rust
fn render_code_block_line(&self, line: &str, line_idx: usize, lines: &[String]) -> Line<'static> {
    // 计算代码块最大宽度（基于视口宽度或内容最大宽度）
    let max_code_width = self.calculate_code_block_width(lines);
    
    // 渲染左侧边框 + 代码内容
    let mut spans = vec![
        Span::styled(line_num, ...),
        Span::styled("│ ", ...),  // 左侧竖线
    ];
    spans.extend(highlighted_spans);  // 高亮代码
    
    // 计算填充宽度并添加右侧边框
    let content_width = display_width(&visible_line);
    let fill_width = max_code_width.saturating_sub(content_width);
    spans.push(Span::styled(" ".repeat(fill_width), ...));
    spans.push(Span::styled(" │", ...));  // 右侧竖线
    
    Line::from(spans)
}
```

### 任务2：``` 自动补全

**修改文件**：`src/tui/editor_markdown.rs`

**修改位置**：主事件循环中的 Insert 模式处理

**方案**：
1. 在 Insert 模式下检测回车键
2. 检查当前行是否是 ``` 开头（代码块开始）
3. 如果是，自动插入闭合的 ``` 并将光标移动到中间

**伪代码**：
```rust
// 在 Insert 模式下处理回车
if mode == Mode::Insert && input.key == Key::Enter {
    let (cursor_row, cursor_col) = textarea.cursor();
    if let Some(current_line) = textarea.lines().get(cursor_row) {
        let trimmed = current_line.trim();
        // 检测是否是 ``` 开头（可能有语言标识）
        if trimmed.starts_with("```") {
            // 检查是否已经有闭合（避免重复补全）
            if !has_closing_fence(cursor_row, textarea.lines()) {
                // 插入闭合 ```
                textarea.insert_newline();
                textarea.insert_newline();
                textarea.insert_str("```");
                // 将光标移到空行（两个 ``` 之间）
                textarea.move_cursor(CursorMove::Up);
                continue;
            }
        }
    }
}
// 正常回车处理
textarea.input(input);
```

## 实现步骤

### Step 1: 添加代码块宽度计算辅助函数
- 新增 `calculate_code_block_width` 方法
- 遍历代码块所有行，计算最大宽度

### Step 2: 修改 render_code_block_line 方法
- 添加右侧填充和竖线渲染
- 保持与左侧竖线样式一致

### Step 3: 实现自动补全逻辑
- 新增 `handle_code_fence_autocomplete` 函数
- 在 Insert 模式的回车处理中调用

### Step 4: 处理边界情况
- 代码块内已有闭合围栏时不重复补全
- 光标位置正确处理
- 撤销历史正确记录

## 预期效果

1. **右侧边框**：
```
┌─ rust ──────────────
│ fn main() {        │
│     println!();    │
│ }                  │
└────────────────────┘
```

2. **自动补全**：
   - 输入 ` ```rust` 后按回车
   - 自动生成：
     ```
     ```rust
     
     ```
     ```
   - 光标定位在中间空行，方便输入代码
