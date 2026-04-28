# Step 4 实施计划：Editor 接入共享层（Inline + 段落）

## 目标

让 editor 的非 Insert 模式渲染走共享层，修复 `**bold**` 在中文/标点/嵌套场景下的不稳定渲染问题。

**范围**：
- Inline 渲染：`**bold**`、`*italic*`、`~~strike~~`、`` `code` ``、链接
- 普通段落渲染
- Heading / List / Blockquote / Rule 渲染

**不在范围内**（后续 Step）：
- 代码块渲染（Step 5）
- 表格渲染（Step 5）
- Insert 模式光标行源码显示（保持现状）
- 续行处理（保持现有 wrap_engine 逻辑）

---

## 关键设计决策

### 1. Editor 不复用 `render_document_wrapped`

Editor 有独特的渲染模式：
- 按 `VisualLine` 逐行渲染（wrap_engine 驱动）
- Insert 模式光标行显示源码
- Normal 模式光标行叠加 cursor block
- 续行（`is_continuation`）特殊处理
- 搜索高亮 / Visual 选区叠加

**结论**：Editor 只复用共享层的**渲染原语**，不复用整文档渲染。

### 2. `EditorMdStyle` 实现 `MdStyle` trait

`EditorTheme` 已有所有 markdown 渲染所需颜色字段，直接实现 `MdStyle` trait 即可。

```rust
// src/tui/editor_core/theme.rs

impl crate::markdown::theme::MdStyle for EditorTheme {
    fn text_normal(&self) -> Color { self.text_normal }
    fn text_dim(&self) -> Color { self.text_dim }
    fn text_bold(&self) -> Color { self.text_bold }
    fn md_h1(&self) -> Color { self.md_h1 }
    fn md_h2(&self) -> Color { self.md_h2 }
    fn md_h3(&self) -> Color { self.md_h3 }
    fn md_h4(&self) -> Color { self.md_h4 }
    fn md_link(&self) -> Color { self.md_link }
    fn md_list_bullet(&self) -> Color { self.md_list_bullet }
    fn md_blockquote_bar(&self) -> Color { self.md_blockquote_bar }
    fn md_blockquote_bg(&self) -> Color { self.md_blockquote_bg }
    fn md_blockquote_text(&self) -> Color { self.md_blockquote_text }
    fn md_inline_code_fg(&self) -> Color { self.md_inline_code_fg }
    fn md_inline_code_bg(&self) -> Color { self.md_inline_code_bg }
    fn code_border(&self) -> Color { self.text_dim }
    fn code_bg(&self) -> Color { self.bg_primary }
    fn table_header(&self) -> Color { self.text_bold }
    fn table_body(&self) -> Color { self.text_normal }
    fn md_heading_sep(&self) -> Color { self.text_dim }
    fn md_rule(&self) -> Color { self.text_dim }
    fn code_syntax_theme(&self) -> Theme {
        // Editor 不使用共享层语法高亮，保留自己的 highlight_fn
        Theme::default()
    }
}
```

### 3. Editor 渲染原语接入点

Editor 的 `render_single_line_with_number` 和 `render_inline` 需要改走共享层：

**旧路径**（editor 手写扫描器）：
```
renderer/inline.rs::render_inline(text) -> Vec<Span>
    - find('*') / find('`') 最小扫描
    - 不处理 CommonMark delimiter rules
```

**新路径**（共享层）：
```
markdown/render/inline.rs::render_inlines(inlines, base_style, theme) -> Vec<Span>
    - 基于 pulldown-cmark 解析的 IR
    - 正确处理嵌套、边界、转义
```

**问题**：Editor 是按视觉行逐行渲染，没有 IR。

### 4. 行级 IR 解析策略

Editor 按 VisualLine 渲染时，每行是独立的文本片段。有两种策略：

**策略 A**：全文预解析 + Block 缓存
- 全文 `parse_markdown(buffer.to_string())` -> ParsedDocument
- 缓存 `blocks` + `line_to_block` 映射
- 渲染某行时：查找该行所属 block，提取对应 inline 内容
- **优点**：IR 完整，后续 Step 5/6 可直接复用
- **缺点**：需要处理"行属于 block 的哪一部分"映射

**策略 B**：行级 mini-parse（仅 inline）
- 每行独立 mini-parse 提取 inline 元素
- 不解析 block 结构（heading/list 等仍用手写检测）
- **优点**：改动最小，风险最低
- **缺点**：block 结构识别仍不稳定，后续还需再改

**决策**：采用**策略 A**。理由：
1. Step 4 目标明确要求新建 `markdown_cache.rs`
2. Block 缓存为 Step 5/6 打基础
3. 行级 mini-parse 无法解决表格内 inline 不渲染的问题

### 5. MarkdownCache 设计

```rust
// src/tui/editor_core/markdown_cache.rs

use crate::markdown::ir::{Block, BlockKind, Inline, ParsedDocument};

/// 渲染后的 block 结果
pub struct RenderedBlock {
    /// 渲染输出的行（不含行号）
    pub lines: Vec<Line<'static>>,
    /// 每个渲染行对应的源码行号（用于光标定位）
    pub source_lines: Vec<usize>,
}

/// Markdown 解析与渲染缓存
pub struct MarkdownCache {
    /// 缓存版本号（buffer 修改时递增）
    revision: u64,
    /// 解析后的文档结构
    doc: Option<ParsedDocument>,
    /// 源码行 -> block 索引映射
    line_to_block: Vec<Option<usize>>,
    /// 已渲染的 block（按需填充）
    rendered_blocks: Vec<Option<RenderedBlock>>,
}

impl MarkdownCache {
    pub fn new() -> Self { ... }
    
    /// 判断缓存是否需要重建
    pub fn needs_rebuild(&self, buffer_revision: u64) -> bool {
        self.revision != buffer_revision
    }
    
    /// 重建缓存（全文解析）
    pub fn rebuild(&mut self, text: &str, width: usize) {
        self.doc = Some(parse_markdown(text, width));
        // 构建 line_to_block 映射...
        self.revision += 1;
    }
    
    /// 获取指定源码行所属的 block 索引
    pub fn get_block_for_line(&self, line_idx: usize) -> Option<usize> {
        self.line_to_block.get(line_idx).copied().flatten()
    }
    
    /// 获取指定 block 的渲染结果（懒渲染）
    pub fn render_block(&mut self, block_idx: usize, theme: &dyn MdStyle, width: usize) -> Option<&RenderedBlock> {
        // 如果未渲染，调用共享层 render_block
        ...
    }
}
```

---

## 实施步骤

### Phase 1：EditorTheme 实现 MdStyle trait

1. 在 `src/tui/editor_core/theme.rs` 中添加：
   ```rust
   impl crate::markdown::theme::MdStyle for EditorTheme { ... }
   ```
2. `cargo check` 编译通过

### Phase 2：新建 markdown_cache.rs

1. 创建 `src/tui/editor_core/markdown_cache.rs`
2. 定义 `RenderedBlock` 和 `MarkdownCache`
3. 实现 `rebuild`（全文解析 + line_to_block 映射）
4. 实现 `get_block_for_line`（源码行 -> block 查询）
5. 实现 `render_block`（调用共享层渲染，带缓存）

**line_to_block 映射算法**：
- 遍历 doc.blocks，根据每个 block 的 `source.start_line..end_line` 填充映射
- 当前 IR 的 `SourceRange` 暂用 `0..0`，需要改进 parser 提供精确行号

**精确行号方案**：
- 在 parser 中使用 `pulldown_cmark::Parser::into_offset_iter()` 获取 byte offset
- 将 byte offset 映射到源码行号（需要源码的行起始位置表）
- Step 4 先用近似方案：根据 block 内容推断行范围

### Phase 3：改写 renderer/line.rs

将 `render_single_line_with_number` 改为使用共享层：

**旧逻辑**：
```rust
fn render_single_line_with_number(&self, line: &str, line_idx: usize, max_width: usize) -> Line {
    // 手写检测 heading / list / blockquote / rule
    // 手写 render_inline 处理 **bold** / `code` 等
}
```

**新逻辑**：
```rust
fn render_single_line_with_number(&self, line_idx: usize, cache: &mut MarkdownCache) -> Option<Line> {
    // 1. 查找该行所属的 block
    let block_idx = cache.get_block_for_line(line_idx)?;
    
    // 2. 获取 block 渲染结果
    let rendered = cache.render_block(block_idx, &self.theme, self.width)?;
    
    // 3. 从 rendered.lines 中提取该源码行对应的渲染行
    //    （这需要 RenderedBlock.source_lines 映射）
    let rendered_line_idx = rendered.source_lines.iter().position(|&l| l == line_idx)?;
    let base_line = rendered.lines.get(rendered_line_idx)?.clone();
    
    // 4. 添加行号前缀
    let line_num = self.format_line_number(line_idx);
    let mut spans = vec![Span::styled(line_num, self.style(Color::DarkGray))];
    spans.extend(base_line.spans);
    
    Some(Line::from(spans))
}
```

**问题**：一个源码行可能渲染成多行（如表格），或者多行源码渲染成一行（如 blockquote）。

**解决**：Phase 3 先只处理简单 block（paragraph/heading/list/rule），这些 block 的源码行和渲染行大致 1:1。表格/代码块在 Step 5 处理。

### Phase 4：改写 renderer.rs 调用路径

1. 在 `MarkdownRenderer` 中添加 `MarkdownCache` 字段
2. 在 `render_non_insert_line` 中：
   - 非续行：调用 `render_single_line_with_number(line_idx, &mut cache)`
   - 续行：保持现有逻辑（显示源码片段）
3. 在 `render_visual_line` 中：
   - Insert 模式：保持现有逻辑（显示源码 + 光标）
   - Normal 模式非光标行：走共享层
   - Normal 模式光标行：走共享层 + overlay cursor

### Phase 5：删除 renderer/inline.rs

1. 确认所有 `render_inline` 调用已替换为共享层
2. 删除 `src/tui/editor_core/renderer/inline.rs`
3. `cargo clippy -- -D warnings` 干净

---

## 精确行号映射方案（Phase 2 关键细节）

当前 IR 的 `SourceRange` 使用 `0..0`，需要改进。

### 方案：基于 offset_iter 的精确映射

```rust
// parser.rs 改进

pub fn parse_markdown_with_lines(md: &str, max_width: usize) -> ParsedDocument {
    // 预处理：构建行起始位置表
    let line_offsets: Vec<usize> = md.lines()
        .scan(0, |state, line| {
            let start = *state;
            *state += line.len() + 1; // +1 for \n
            Some(start)
        })
        .collect();
    
    // 使用 offset_iter 获取每个 event 的 byte range
    let parser = pulldown_cmark::Parser::new_ext(md, options);
    for (event, range) in parser.into_offset_iter() {
        // 将 range 映射到行号
        let start_line = byte_to_line(range.start, &line_offsets);
        let end_line = byte_to_line(range.end - 1, &line_offsets); // end 是 exclusive
        
        // 在创建 Block 时填充 SourceRange
        ...
    }
}

fn byte_to_line(byte: usize, line_offsets: &[usize]) -> usize {
    line_offsets.iter().position(|&offset| offset > byte).unwrap_or(line_offsets.len()).saturating_sub(1)
}
```

---

## 续行处理策略

续行（`is_continuation = vl.start_col > 0`）是折行后的第二行及之后。Editor 当前处理：
- 续行无法独立渲染 Markdown（标记可能跨边界）
- 续行显示源码片段
- 代码块续行保持边框样式
- 表格续行跳过渲染（完整表格由 VL1 渲染）

**Step 4 策略**：
- 续行**不走共享层**，保持现有逻辑
- 只有 VL1（非续行）走共享层渲染

---

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/tui/editor_core/theme.rs` | 修改 | 添加 `impl MdStyle for EditorTheme` |
| `src/tui/editor_core/markdown_cache.rs` | **新建** | Markdown 解析与渲染缓存 |
| `src/tui/editor_core/renderer.rs` | 修改 | 添加 cache 字段，改写 render_non_insert_line |
| `src/tui/editor_core/renderer/line.rs` | **重构** | 使用共享层渲染原语 |
| `src/tui/editor_core/renderer/inline.rs` | **删除** | 移除手写 inline 扫描器 |
| `src/markdown/parser.rs` | 修改 | 改进 SourceRange 精确行号填充 |

---

## 测试策略

1. **硬约束**：`parser/tests.rs` 全部通过（21 个测试）
2. 新增 editor 回归测试：
   - 普通段落里的 `**bold**`、`*italic*`、`~~strike~~`
   - 中文 + 引号边界：`**中文**`、`**"引号"**`
   - 嵌套场景：`**bold *italic* bold**`
   - 行内代码：`` `code` ``、`` ``double backtick`` ``

---

## 风险与缓解

1. **SourceRange 精确行号复杂**：需要 offset_iter + 行位置映射
   - 缓解：Phase 2 可先用近似方案（根据 block 内容推断），后续迭代改进

2. **续行与共享层语义冲突**：续行是折行片段，不是完整 Markdown 行
   - 缓解：续行保持现有逻辑，只让 VL1 走共享层

3. **表格/代码块在 Step 4 范围外**：这些 block 渲染结果可能不是 1:1 行映射
   - 缓解：Phase 3 只处理简单 block，表格/代码块保持现有逻辑，Step 5 再迁