# Plan: Split parser.rs (1518 lines)

## Current Structure

`src/command/chat/markdown/parser.rs` 有 1518 行，核心是一个 848 行的巨型函数 `markdown_to_lines`。

**文件结构**：
1. `markdown_to_lines` (L11-858) — 公共函数，pulldown-cmark 事件循环
   - 预处理 (L22-36) — 修复中文引号加粗
   - 解析器初始化 (L38-63) — 15+ 个局部状态变量
   - 事件循环 (L72-842) — giant match：
     - Heading (L74-147)
     - Strong/Emphasis/Strikethrough (L148-168)
     - Link (L169-198)
     - CodeBlock (L199-266)
     - InlineCode (L267-302)
     - List/Item/TaskListMarker (L303-350)
     - Paragraph (L351-361)
     - BlockQuote (L362-384)
     - **Text (L385-538)** — 最复杂，含 URL 拆分 + 自动换行
     - SoftBreak/HardBreak (L540-553)
     - Rule (L554-560)
     - **Table (L561-812)** — 最大块，约 250 行
     - Image (L813-839)
   - 最终刷新 (L844-858)
2. 私有辅助函数 (L860-1026)：
   - `cell_to_pieces` (L862-886)
   - `wrap_cell_styled` (L890-942)
   - `display_width_cell` (L945-965)
   - `split_text_with_urls` (L968-1026)
3. 测试 (L1028-1518) — 约 490 行

**外部引用**：通过 `markdown.rs` 的 `pub use parser::markdown_to_lines` 导出，外部统一用 `crate::command::chat::markdown::markdown_to_lines`。

## Splitting Strategy

引入 `ParserState` 结构体封装事件循环的共享状态，将 Table 和 Text 这两个最大逻辑块提取为 `ParserState` 的方法。

```
parser/
├── mod.rs       # markdown_to_lines() + ParserState 定义 + 简单事件处理 + 主循环
├── table.rs     # ParserState 的 table 事件方法 + 3 个 cell 辅助函数
├── text.rs      # ParserState 的 text 事件方法 + split_text_with_urls
├── tests.rs     # 所有测试
```

### 详细分配

#### 1. `mod.rs` (~450 lines)

**内容**：
- imports
- `pub fn markdown_to_lines()` — 预处理 + 解析器初始化 + 主循环
- `ParserState` 结构体定义（pub(crate) 字段）：
  ```rust
  pub(crate) struct ParserState<'a> {
      pub(crate) lines: Vec<Line<'static>>,
      pub(crate) current_spans: Vec<Span<'static>>,
      pub(crate) style_stack: Vec<Style>,
      pub(crate) in_code_block: bool,
      pub(crate) code_block_content: String,
      pub(crate) code_block_lang: String,
      pub(crate) list_depth: usize,
      pub(crate) ordered_index: Option<u64>,
      pub(crate) heading_level: Option<u8>,
      pub(crate) in_blockquote: bool,
      pub(crate) link_url: Option<String>,
      pub(crate) image_url: Option<String>,
      pub(crate) image_alt: String,
      pub(crate) content_width: usize,
      pub(crate) theme: &'a Theme,
      pub(crate) base_style: Style,
  }
  ```
- `impl ParserState` — `new()`, `flush_line()`, 简单事件处理方法
- 简单事件直接在主循环 match 中处理（Heading, Strong, Emphasis, CodeBlock, List, Paragraph, BlockQuote, Image 等各自 10-40 行）
- Table/Text 事件委托给子模块方法

#### 2. `table.rs` (~350 lines)

**内容**：
- `impl ParserState` 的 table 相关方法：
  - `handle_table_start()` — Event::Start(Tag::Table)
  - `handle_table_end()` — Event::End(TagEnd::Table)，含完整表格渲染逻辑（列宽计算、边框、单元格截断等）
  - `handle_table_head_start/end()`, `handle_table_row_start/end()`, `handle_table_cell_start/end()`
  - `handle_code_in_table()` — Event::Code 在 in_table=true 时的逻辑
- 私有函数：`cell_to_pieces`, `wrap_cell_styled`, `display_width_cell`

#### 3. `text.rs` (~200 lines)

**内容**：
- `impl ParserState` 的 text 事件方法：
  - `handle_text_event()` — 处理 Event::Text，含图片alt累积、代码块累积、表格单元格累积、普通文本的 URL 拆分 + 自动换行
- `split_text_with_urls` 函数

#### 4. `tests.rs` (~490 lines)

**内容**：所有测试原样迁移

## ParserState 方法在子模块中的实现

Rust 允许在同一个 crate 的不同文件中为同一类型添加 `impl` 块。`table.rs` 和 `text.rs` 中可以有 `impl ParserState<'_> { ... }` 块，只要 `ParserState` 定义在 `mod.rs` 中且字段为 `pub(crate)`。

## 实施步骤

1. `mkdir src/command/chat/markdown/parser/`
2. 移动 `parser.rs` → `parser/mod.rs`
3. 在 `parser/mod.rs` 中引入 `ParserState` 结构体，重构 `markdown_to_lines` 使用 `ParserState`
4. 提取 table 逻辑到 `parser/table.rs`
5. 提取 text 逻辑到 `parser/text.rs`
6. 提取测试到 `parser/tests.rs`
7. `cargo fmt && cargo clippy -- -D warnings` 验证
8. `cargo test` 确认所有测试通过

## 预估行数

| 文件 | 行数 |
|------|------|
| mod.rs | ~450 |
| table.rs | ~350 |
| text.rs | ~200 |
| tests.rs | ~490 |
| **总计** | **~1490** |
