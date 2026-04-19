# 修复表格内行内代码（`code`）样式渲染

## 问题分析

在 `parser.rs` 中，pulldown-cmark 解析表格时：

1. **单元格内容收集为纯字符串**：`Event::Text` 和 `Event::Code` 在 `in_table` 时直接追加到 `current_cell: String`（第 386-387 行、第 264-267 行）
2. **表格渲染为统一 Span**：每个单元格作为一个整体 Span 输出（第 681 行），使用单一的 `table_style` 或 `header_style`
3. **结果**：表格中 `` `code` `` 只是以原始反引号文本显示，没有任何高亮/背景色样式

### 关键代码路径

```
Event::Code(text) → if in_table { current_cell.push('`'); current_cell.push_str(&text); current_cell.push('`'); }
                    ↓
Event::End(TagEnd::Table) → 渲染时 cell_text 作为一个整体字符串 → Span::styled(formatted, style)
```

## 修复方案

在表格渲染阶段（`Event::End(TagEnd::Table)`），将单元格文本中的 `` `...` `` 片段拆分为多个 Span，对行内代码部分应用 `md_inline_code_fg` / `md_inline_code_bg` 样式。

### 修改文件

**`src/command/chat/markdown/parser.rs`**

1. **新增辅助函数 `split_cell_spans`**：解析单元格文本，将 `` `code` `` 部分拆分为独立 Span，行内代码使用 `md_inline_code_fg` / `md_inline_code_bg` 样式，其余文本使用传入的基础样式。

2. **修改表格渲染逻辑**（第 644-682 行）：将原来每个单元格渲染为单个 `Span::styled(formatted, style)` 改为使用 `split_cell_spans` 生成多个 Span，每个 Span 携带各自的样式（普通文本 vs 行内代码）。

3. **更新 `display_width` 计算**：由于行内代码中的反引号不再需要显示（或保留为可见标记），需要确保 `col_widths` 计算与实际显示宽度一致。

### 具体实现

#### `split_cell_spans` 函数

```rust
/// 将单元格文本拆分为 Span 列表，`` `code` `` 片段应用行内代码样式
fn split_cell_spans(cell_text: &str, base_style: Style, code_style: Style) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = cell_text;
    while !remaining.is_empty() {
        if let Some(start) = remaining.find('`') {
            // 反引号之前的普通文本
            if start > 0 {
                spans.push(Span::styled(remaining[..start].to_string(), base_style));
            }
            // 找到配对的反引号
            let after_tick = &remaining[start + 1..];
            if let Some(end) = after_tick.find('`') {
                let code_text = &after_tick[..end];
                spans.push(Span::styled(code_text.to_string(), code_style));
                remaining = &after_tick[end + 1..];
            } else {
                // 无配对反引号，当作普通文本
                spans.push(Span::styled(remaining[start..].to_string(), base_style));
                break;
            }
        } else {
            spans.push(Span::styled(remaining.to_string(), base_style));
            break;
        }
    }
    spans
}
```

#### 修改单元格渲染（第 646-682 行）

原来：
```rust
let formatted = match align { ... };  // " " + cell_line + fill + " "
row_spans.push(Span::styled(formatted, style));
```

改为：
```rust
let (left_pad, right_pad) = match align {
    Center => (" ".repeat(left), " ".repeat(right)),
    Right => (" ".repeat(fill), String::new()),
    _ => (String::new(), " ".repeat(fill)),
};
// 左填充
row_spans.push(Span::styled(format!(" {}", left_pad), style));
// 单元格内容（拆分行内代码）
let cell_spans = split_cell_spans(cell_line, style, code_style);
row_spans.extend(cell_spans);
// 右填充
row_spans.push(Span::styled(format!("{} ", right_pad), style));
```

#### 注意事项

- `col_widths` 计算仍然基于 `display_width(cell_text)`，这与实际渲染宽度一致（反引号字符在 cell_text 中仍被计入，实际渲染中不显示反引号，但 cell_text 中保留它们用于解析；需要确保 `display_width` 计算的是去除反引号后的宽度，或者调整策略：在 cell_text 中保留反引号但在计算宽度时去除）

  **更简洁的做法**：将行内代码样式直接应用到包含反引号的文本上（即显示 `` `code` `` 并加样式），这样 `display_width` 不需要修改，只是反引号作为可见分隔符保留，同时内容有背景色高亮。这与普通段落中行内代码的处理一致（普通段落中行内代码前后有空格和背景色区分）。

  实际上看 parser.rs 第 263-297 行，表格中 `Event::Code` 在 current_cell 中保存了带反引号的文本。因此：
  - `col_widths` 基于 `display_width(cell_text)` 计算，**包含反引号字符的宽度**
  - 渲染时显示反引号 + 内容，应用 `md_inline_code_bg` 背景色

  这是最安全的方案，不需要改变宽度计算逻辑。
