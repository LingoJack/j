# Step 3 实施计划：Parser 输出 IR，Chat 改走共享 Render

## 目标

将当前的"一体化"parser（解析+渲染同时进行）拆分为：
1. **IR 层**：`parse_markdown(md) -> ParsedDocument`（纯解析，生成中间表示）
2. **Render 层**：`render_document_wrapped(&doc, &dyn MdStyle, width) -> Vec<Line>`（基于 IR 渲染）
3. **Facade**：`markdown_to_lines(...)` 保持签名不变，内部调用 parse + render

硬约束：`parser/tests.rs` 全部通过。

---

## IR 设计（src/markdown/ir.rs）

```rust
/// 源码位置范围
#[derive(Debug, Clone, PartialEq)]
pub struct SourceRange {
    pub start_line: usize,  // 0-based
    pub end_line: usize,    // 0-based, inclusive
}

/// 解析后的文档
#[derive(Debug, Clone, Default)]
pub struct ParsedDocument {
    pub blocks: Vec<Block>,
    /// 源码行号 -> block 索引的映射（用于 editor 侧快速定位）
    /// Step 3 先不强制填充，可为空 Vec
    pub line_to_block: Vec<Option<usize>>,
}

/// Block 级元素
#[derive(Debug, Clone)]
pub struct Block {
    pub source: SourceRange,
    pub kind: BlockKind,
}

#[derive(Debug, Clone)]
pub enum BlockKind {
    Paragraph(Vec<Inline>),
    Heading { level: u8, content: Vec<Inline> },
    CodeBlock { lang: String, code: String },
    Table(TableData),
    List(ListData),
    BlockQuote(Vec<Block>),
    Rule,
}

/// 表格数据
#[derive(Debug, Clone)]
pub struct TableData {
    pub alignments: Vec<pulldown_cmark::Alignment>,
    /// rows[row_idx][col_idx] = cell inlines
    pub rows: Vec<Vec<Vec<Inline>>>,
}

/// 列表数据
#[derive(Debug, Clone)]
pub struct ListData {
    pub ordered: bool,
    pub start_index: Option<u64>,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub checked: Option<bool>,
    pub content: Vec<Inline>,
}

/// Inline 级元素
#[derive(Debug, Clone)]
pub enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Strikethrough(Vec<Inline>),
    Code(String),
    Link { text: Vec<Inline>, url: String },
    SoftBreak,
    HardBreak,
}
```

**设计决策**：
- 不包含 `Image` block（当前渲染器未实现独立图片 block，图片通过 image_cache 异步加载后替换行）
- `SourceRange` Step 3 先用 0..0 填充，后续 Step 按需实现精确映射
- `Alignment` 直接 re-export `pulldown_cmark::Alignment`，避免冗余定义

---

## 模块结构

```
src/markdown/
├── mod.rs              # pub mod ir; pub mod parser; pub mod render; ...
├── ir.rs               # IR 类型定义（新建）
├── parser.rs           # 纯解析：md -> ParsedDocument（重构）
├── parser/
│   ├── table.rs        # 表格解析辅助（重构）
│   ├── text.rs         # 预处理（保留）
│   └── tests.rs        # 测试（保持不变）
├── render/
│   ├── mod.rs          # render_document_wrapped 入口（新建）
│   ├── inline.rs       # Inline -> Span 渲染（新建）
│   ├── block.rs        # Block -> Vec<Line> 渲染（新建）
│   ├── table.rs        # 表格渲染（从 parser/table.rs 迁移渲染逻辑）
│   └── code_block.rs   # 代码块渲染（从 parser.rs 提取）
├── highlight.rs        # 语法高亮（保持不变）
├── image_cache.rs      # 图片缓存（保持不变）
├── image_loader.rs     # 图片加载（保持不变）
└── theme.rs            # MdStyle trait（保持不变）
```

---

## 实施步骤

### Phase 1：IR 定义 + 编译验证

1. 新建 `src/markdown/ir.rs`，定义所有 IR 类型
2. 在 `src/markdown/mod.rs` 中添加 `pub mod ir;`
3. `cargo check` 编译通过，零行为变化

### Phase 2：新建 Render 模块（并行开发）

> 关键策略：先写 render 模块，不改 parser。Parser 侧先写一个临时的 IR 构造函数，让 render 模块可以编译。

1. 新建 `src/markdown/render/mod.rs`
   - `render_document_wrapped(doc: &ParsedDocument, theme: &dyn MdStyle, width: usize) -> Vec<Line<'static>>`
2. 新建 `src/markdown/render/inline.rs`
   - `render_inlines(inlines: &[Inline], base_style: Style, theme: &dyn MdStyle) -> Vec<Span<'static>>`
   - 从 `parser/text.rs` 迁移 `split_text_with_urls` 逻辑，处理 `Inline::Text` 中的 URL 拆分
   - 递归处理 `Inline::Strong`、`Inline::Emphasis` 等
   - `Inline::Code` → 带 bg 色的 Span
   - `Inline::Link` → 带 link 颜色的 Span
3. 新建 `src/markdown/render/block.rs`
   - `render_block(block: &Block, ctx: &mut RenderContext) -> Vec<Line<'static>>`
   - `RenderContext { width, theme }`（简洁结构，避免引用可变状态）
   - 从 parser.rs 提取各 block 的渲染逻辑：
     - heading：prefix ◆/◇/▸/► + underline
     - list：bullet • / number + indent
     - blockquote：│ prefix bar + dim text
     - rule：─── 分隔线
     - paragraph：auto-wrap with inline styles
4. 新建 `src/markdown/render/code_block.rs`
   - 从 parser.rs 提取代码块渲染逻辑（边框 ┌─┐、highlight 调用、└─┘）
   - 调用 `highlight::highlight_code_line`
5. 新建 `src/markdown/render/table.rs`
   - 从 `parser/table.rs` 迁移渲染逻辑（边框绘制、cell wrap、截断）
   - `wrap_cell_styled` 改为接收 `&[Inline]` 而非 `&str`
   - `cell_to_pieces` 不再需要（IR 已区分 text/code），但需实现 inline 渲染 + wrap
   - 保留 `display_width` 计算逻辑，但改为遍历 Inline 计算

### Phase 3：Parser 重构

1. 重写 `parser.rs` 中的 `ParserState`：
   - 移除渲染相关字段（lines, current_spans, style_stack, theme）
   - 添加 IR 累积字段：blocks, current_inlines, inline_stack
   - 保留预处理逻辑
2. 实现 `parse_markdown(md: &str) -> ParsedDocument`：
   - 预处理（normalize_terminal_text, 中文引号 ZWSP, 表格分隔行修复）
   - pulldown_cmark::Parser::new_ext 解析
   - Event 匹配 → 累积 IR 节点
   - 使用 inline 栈处理嵌套（Strong > Emphasis > Code 等）
3. 重写 `parser/table.rs`：
   - `handle_table_cell_end` 改为收集 `Vec<Inline>` 而非 String
   - `handle_code_in_table` 改为 push `Inline::Code(text)`
   - 移除 `cell_to_pieces`（render 层不再需要字符串级反引号匹配）
   - 移除 `display_width_cell`（render 层直接遍历 Inline 计算宽度）
   - 保留 `wrap_cell_styled` 的核心算法（字符级 wrap），但改为基于 Inline

### Phase 4：Facade 连接

1. 修改 `markdown_to_lines`：
   ```rust
   pub fn markdown_to_lines(md: &str, max_width: usize, theme: &dyn MdStyle) -> Vec<Line<'static>> {
       let doc = parse_markdown(md, max_width);
       render::render_document_wrapped(&doc, theme, max_width)
   }
   ```
2. `parse_markdown` 接收 `max_width` 仅用于表格分隔行修复预处理（当前逻辑）
3. 运行 `cargo test --lib markdown::parser::tests` — 全部通过

### Phase 5：清理

1. 移除 `parser/text.rs` 中迁移到 render 的函数（保留预处理函数）
2. 移除 `parser/table.rs` 中的渲染函数（`cell_to_pieces`, `display_width_cell`）
3. 运行 `cargo fmt` + `cargo clippy -- -D warnings`

---

## 关键技术细节

### Inline 栈处理

```rust
struct ParseContext {
    blocks: Vec<Block>,
    /// 当前正在构建的 block 级 inline 容器
    current_inlines: Vec<Inline>,
    /// 嵌套栈：每层是 (container_index, tag_type)
    /// 用于处理 **bold _italic_** 等嵌套
    inline_stack: Vec<InlineContainer>,
}

enum InlineContainer {
    Strong,
    Emphasis,
    Strikethrough,
    Link { url: String },
}
```

Event 处理：
- `Event::Start(Tag::Strong)` → push Strong 到 inline_stack，开始新的子容器
- `Event::End(TagEnd::Strong)` → pop 栈，收集子 inlines 为 `Inline::Strong(children)`，push 到父容器
- `Event::Text(s)` → push `Inline::Text(s)` 到当前容器
- `Event::Code(s)` → push `Inline::Code(s)` 到当前容器
- `Event::SoftBreak` → push `Inline::SoftBreak`
- `Event::Start(Tag::Link { dest_url, .. })` → push Link 到 inline_stack
- `Event::End(TagEnd::Link)` → pop 栈，收集为 `Inline::Link { text, url }`

### 表格单元格 Inline 收集

当前：`handle_code_in_table` 将 code 文本加反引号拼入 String
新方案：
- `handle_table_cell_start()` → `current_cell_inlines.clear()`
- `handle_code_in_table(text)` → `current_cell_inlines.push(Inline::Code(text.to_string()))`
- 表格内的 `Event::Text` → `current_cell_inlines.push(Inline::Text(text.to_string()))`
- `handle_table_cell_end()` → `current_row.push(current_cell_inlines.clone())`

### 表格渲染宽度计算

当前 `display_width_cell` 通过反引号配对计算字符串显示宽度。
新方案：遍历 `&[Inline]` 计算宽度：
```rust
fn inline_display_width(inlines: &[Inline]) -> usize {
    inlines.iter().map(|i| match i {
        Inline::Text(s) | Inline::Code(s) => display_width(s),
        Inline::Strong(children) | Inline::Emphasis(children) => inline_display_width(children),
        Inline::SoftBreak | Inline::HardBreak => 0,
        Inline::Link { text, .. } => inline_display_width(text),
        Inline::Strikethrough(children) => inline_display_width(children),
    }).sum()
}
```

### 表格 cell wrap（基于 Inline）

当前 `wrap_cell_styled` 在字符串级别 wrap，用 `cell_to_pieces` 拆分反引号对。
新方案：
1. 先调用 `render_inlines(cell_inlines, base_style, theme)` 得到 `Vec<Span>`
2. 将这些 Span 按显示宽度 wrap 成多行
3. 保留截断逻辑（char_width 逐字符累积）

---

## 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `src/markdown/mod.rs` | 修改 | 添加 `pub mod ir;` 和 `pub mod render;` |
| `src/markdown/ir.rs` | **新建** | IR 类型定义 |
| `src/markdown/parser.rs` | **重构** | 改为纯解析，输出 ParsedDocument |
| `src/markdown/parser/table.rs` | **重构** | 累积 TableData IR，移除渲染函数 |
| `src/markdown/parser/text.rs` | 修改 | 保留预处理，移除 URL 拆分（迁到 render） |
| `src/markdown/parser/tests.rs` | **保持** | 测试不变，验证 facade 兼容 |
| `src/markdown/render/mod.rs` | **新建** | render_document_wrapped 入口 |
| `src/markdown/render/inline.rs` | **新建** | Inline -> Span 渲染 |
| `src/markdown/render/block.rs` | **新建** | Block -> Vec<Line> 渲染 |
| `src/markdown/render/table.rs` | **新建** | 表格渲染（从 parser/table.rs 迁移） |
| `src/markdown/render/code_block.rs` | **新建** | 代码块渲染 |
| `src/markdown/highlight.rs` | 保持 | 语法高亮不变 |
| `src/markdown/theme.rs` | 保持 | MdStyle trait 不变 |

---

## 测试策略

1. **硬约束**：`parser/tests.rs` 全部通过（21 个测试）
2. 每完成一个 Phase 都运行 `cargo test --lib markdown::parser::tests`
3. Phase 5 后运行完整 `cargo clippy -- -D warnings`

---

## 风险与缓解

1. **表格 cell wrap 逻辑复杂**：当前 `wrap_cell_styled` 约 50 行，改为基于 Span 的 wrap 需要保持截断精度
   - 缓解：先保留 `wrap_cell_styled` 的核心算法，仅将输入从 `&str` 改为 `Vec<Span>`

2. **Inline 嵌套栈处理**：需要正确处理 Strong > Emphasis > Code 等嵌套
   - 缓解：先处理 2 层嵌套（最常见场景），后续迭代补充

3. **预处理与 IR 的一致性**：预处理改变了文本，IR 中的文本应是预处理后的
   - 缓解：先对预处理后的文本生成 IR，不做 offset 映射
