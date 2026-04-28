# 修复 Visual 模式选区高亮——整行变色改为精确字符级高亮

## 问题分析

`editor.rs` 第 875~907 行的 Visual 模式选区高亮逻辑存在两个核心问题：

### 问题 1：判断粒度是"整行"而非"字符"

当前的 `in_selection` 判断只确定某个**视觉行**是否与选区有交集，但一旦判定有交集，就把**整行的所有 span** 都改为选中色：

```rust
// 第 902~905 行——问题核心
if in_selection && let Some(line) = all_visual_lines.get_mut(idx) {
    for span in line.spans.iter_mut() {
        span.style = span.style.patch(Style::default().fg(sel_fg).bg(sel_bg));
    }
}
```

这意味着即使只选了一行中的 3 个字符，整行都会变色。

### 问题 2：行号 span 也被选中色覆盖

行号（如 `  12  `）是渲染出的第一个 span，它也位于 `line.spans` 中，被一并修改了颜色。

## 修复方案

将"整行覆盖"改为**精确到字符的选区高亮**，思路与搜索高亮（`search.highlight_line`）或光标叠加（`overlay_cursor_on_spans`）类似：

### 核心思路

1. **仍然用 `in_selection` 判断视觉行是否与选区有交集**（快速跳过无关行）
2. 对于有交集的视觉行，计算该视觉行内需要高亮的**字符范围** `[highlight_start, highlight_end)`
3. 遍历 spans，将落在该范围内的字符分割出来并应用选中样式
4. **跳过行号 span**（前 `line_num_chars` 个字符）

### 计算视觉行内的选区字符范围

对于视觉行 `vl`（对应逻辑行 `meta.logical_line`，字符范围 `[vl.start_col, vl.end_col)`）：

- 如果该视觉行的逻辑行完全在选区中间（`sr < logical_line < er`），则整行高亮
- 如果该视觉行恰好是选区起始行或结束行，则需与 `[sc, ec)` 求交集：
  - `hl_start = max(vl.start_col, sc)`
  - `hl_end = min(vl.end_col, ec)`
- 转为视觉行内**字符偏移**：`char_start = hl_start - vl.start_col`, `char_end = hl_end - vl.start_col`

### 新增辅助函数 `apply_selection_to_spans`

在 `MarkdownEditor` 的 `render` 方法中，将整行覆盖逻辑替换为：

```rust
/// 对 spans 的指定字符范围应用选区高亮。
/// `char_start` / `char_end` 是跳过行号之后的字符偏移（0-based, exclusive end）。
fn apply_selection_to_spans(
    spans: &mut Vec<Span<'static>>,
    line_num_chars: usize,
    char_start: usize,
    char_end: usize,
    sel_fg: Color,
    sel_bg: Color,
) {
    // 将 spans 按 line_num_chars 分割：
    // - line_num 范围内的 spans 不动
    // - line_num 之后、落入 [char_start, char_end) 的 spans 切分并染色
    let mut chars_seen = 0usize;
    let mut span_idx = 0;

    // 跳过行号 span
    while span_idx < spans.len() && chars_seen < line_num_chars {
        let len = spans[span_idx].content.chars().count();
        chars_seen += len;
        span_idx += 1;
    }

    // 现在开始处理内容 span
    // char_start/char_end 是相对于内容起始的偏移
    // 但由于我们只关心落入选中范围的部分，用原地 split span 的方式
    // ...（具体实现见代码）
}
```

实际上更简洁的方式是：复用已有的 `overlay_cursor_on_spans` / `extract_span_range` 的模式——**重建 spans 列表**。

### 最终实现策略

采用**重建 spans** 的方式（而非原地修改），更简洁且不易出错：

```rust
// 伪代码
for (idx, meta) in all_vl_meta.iter().enumerate() {
    let (hl_start, hl_end) = compute_highlight_range(meta, sr, sc, er, ec);
    if hl_start >= hl_end { continue; } // 无交集

    // 将 hl_start/hl_end 转为视觉行内字符偏移
    let local_start = hl_start.saturating_sub(meta.start_col);
    let local_end = hl_end.saturating_sub(meta.start_col);

    // 重建该行的 spans：行号部分不变 + 内容部分按 [local_start, local_end) 切分染色
    if let Some(line) = all_visual_lines.get_mut(idx) {
        line.spans = rebuild_spans_with_selection(
            &line.spans, line_num_chars, local_start, local_end, sel_fg, sel_bg
        );
    }
}
```

## 修改文件

仅修改 `src/tui/editor_core/editor.rs`：

1. 在 `render()` 方法中，将第 887~907 行的"整行覆盖"逻辑替换为精确字符范围高亮
2. 新增辅助函数 `rebuild_spans_with_selection()`，处理 span 分割和样式应用

## 测试要点

- 单行内选部分文字 → 只有选中部分变色
- 单行内选全部文字 → 整行变色（不包括行号）
- 跨多行选择 → 起始行和结束行部分变色，中间行整行变色
- 行号不变色
- 折行场景：选区跨越视觉折行边界时的正确性
