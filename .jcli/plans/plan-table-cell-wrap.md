# 计划：Chat 消息表格渲染支持单元格内折行

## 问题分析

### 当前 Chat 消息中的表格渲染（`parser.rs`）
在 `Event::End(TagEnd::Table)` 处（第 564-719 行），表格渲染逻辑：
- 每个 table row 只输出**一行**文本
- 当单元格内容超过列宽 `col_widths[i]` 时，**截断**内容（逐字符截取到列宽）
- 没有 `wrap_text` 调用

关键代码（第 634-670 行）：
```rust
let text = if cell_w > *cw {
    // 截断！超宽部分直接丢弃
    let mut t = String::new();
    let mut w = 0;
    for ch in cell_text.chars() {
        let chw = char_width(ch);
        if w + chw > *cw { break; }
        t.push(ch);
        w += chw;
    }
    // ...
} else {
    // 正常对齐填充
};
```

### editor_core 中的表格渲染（`renderer.rs`）
在 `render_table_rows` 方法（第 927-1069 行），已有完整的单元格折行支持：
- 对每个单元格调用 `wrap_text(cell_text, col_width)` → 得到 `Vec<String>`
- 取所有单元格的最大折行行数 `max_rows`
- 每个折行子行输出一行渲染结果，不足行数的单元格用空行填充
- 空单元格显示空内容

### 目标
让 Chat 消息的表格渲染也支持单元格内折行，与 editor_core 行为一致。

## 实施方案

### 修改文件
`src/command/chat/markdown/parser.rs`

### 修改范围
`Event::End(TagEnd::Table)` 处理块（第 564-719 行），替换其中的行渲染逻辑。

### 具体改动

1. **在列宽计算后、渲染前**，对每个单元格调用 `wrap_text` 进行折行
2. **计算每行数据行的最大折行数** `max_rows`
3. **将每个 table row 渲染为 `max_rows` 行**，不足的单元格用空字符串填充
4. **保持现有的对齐逻辑**（Left/Center/Right），在每个折行子行上应用
5. **边框渲染保持不变**（顶边框 ┌─┬─┐、中间分隔线 ├─┼─┤、底边框 └─┴─┘）

### 详细步骤

将第 628-698 行的渲染循环替换为：

```rust
for (row_idx, row) in table_rows.iter().enumerate() {
    // 对每个单元格进行折行
    let wrapped_cells: Vec<Vec<String>> = col_widths
        .iter()
        .enumerate()
        .map(|(i, cw)| {
            let cell_text = row.get(i).map(|s| s.as_str()).unwrap_or("");
            wrap_text(cell_text, *cw)
        })
        .collect();

    // 计算最大折行行数
    let max_rows = wrapped_cells.iter().map(|r| r.len()).max().unwrap_or(1);

    // 渲染每个折行子行
    for sub_row in 0..max_rows {
        let mut row_spans: Vec<Span> = Vec::new();
        row_spans.push(Span::styled("│", border_style));
        for (i, cw) in col_widths.iter().enumerate() {
            let cell_line = wrapped_cells
                .get(i)
                .and_then(|lines| lines.get(sub_row))
                .map(|s| s.as_str())
                .unwrap_or("");
            let cell_line_w = display_width(cell_line);
            let fill = cw.saturating_sub(cell_line_w);
            let align = table_alignments
                .get(i)
                .copied()
                .unwrap_or(pulldown_cmark::Alignment::None);
            let formatted = match align {
                pulldown_cmark::Alignment::Center => {
                    let left = fill / 2;
                    let right = fill - left;
                    format!(" {}{}{} ", " ".repeat(left), cell_line, " ".repeat(right))
                }
                pulldown_cmark::Alignment::Right => {
                    format!(" {}{} ", " ".repeat(fill), cell_line)
                }
                _ => format!(" {}{} ", cell_line, " ".repeat(fill)),
            };
            let style = if row_idx == 0 { header_style } else { table_style };
            row_spans.push(Span::styled(formatted, style));
            row_spans.push(Span::styled("│", border_style));
        }
        if table_right_pad > 0 {
            row_spans.push(Span::raw(" ".repeat(table_right_pad)));
        }
        lines.push(Line::from(row_spans));
    }

    // 行间分隔线（非最后一行时）
    if row_idx < table_rows.len() - 1 {
        let mut sep = String::from("├");
        for (i, cw) in col_widths.iter().enumerate() {
            sep.push_str(&"─".repeat(cw + 2));
            if i < num_cols - 1 {
                sep.push('┼');
            }
        }
        sep.push('┤');
        let mut sep_spans = vec![Span::styled(sep, border_style)];
        if table_right_pad > 0 {
            sep_spans.push(Span::raw(" ".repeat(table_right_pad)));
        }
        lines.push(Line::from(sep_spans));
    }
}
```

### 不变部分
- 列宽计算逻辑（第 569-601 行）保持不变
- `col_widths` 缩放/限制逻辑保持不变
- 顶边框和底边框渲染保持不变
- `table_style`、`header_style`、`border_style` 定义保持不变

### 注意事项
- `wrap_text` 已在文件顶部通过 `use crate::util::text::{display_width, wrap_text};` 引入
- `char_width` 不再需要在此处 import（删除了逐字符截断逻辑）
- 折行后的子行不需要在内部加行间分隔线（如 ├─┼─┤），只在不同的 table row 之间加
