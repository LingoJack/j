# Markdown 渲染管线统一重构

## 背景

项目里目前有两套独立的 markdown 渲染管线，互不复用，editor 这套不稳定。

| 维度 | Editor 渲染 | Chat 渲染 |
|---|---|---|
| 入口 | `MarkdownRenderer::render_visual_line(vl, ...)` | `markdown_to_lines(md, w, theme) -> Vec<Line>` |
| Parser | 手写最小扫描器（`find('*')`、`split('|')`） | `pulldown-cmark`（CommonMark / GFM 扩展） |
| 驱动方式 | 按 `VisualLine` 逐行渲染 | 整文档一次性 |
| 上下文 | 光标、搜索高亮、insert/normal 模式 | 无状态 |
| 位置 | `src/tui/editor_core/renderer/*` | `src/markdown/*` |
| 已共享 | `highlight_code_line`、`EditorTheme::from(&Theme)` |

### Editor 侧的根因 bug

1. **表格单元格按纯文本处理**（`renderer/table.rs`）
   `parse_table_cells -> wrap_text -> Span::styled(...)`，单元格里的 `**bold**`、`` `code` ``、链接、删除线全部失效。

2. **`split('|')` 切分**
   不处理转义 `\|`、code span 内的 `|`，cell 边界不可信。

3. **inline parser 是 `find('*') / find('`')` 的最小扫描**
   不处理 CommonMark delimiter rules、不支持多反引号 code span、不处理嵌套与转义、对中文标点边界判错。这是 `**xx**` 不稳定的直接原因。

## 目标

抽出顶层 `src/markdown/` 共享层，让 chat 和 editor 共用同一个 parser + IR + block/inline 渲染原语，最终删除 editor 里手写的 markdown 语法识别逻辑。

这里的“共享”是：

- 共享 `pulldown-cmark` parser
- 共享 markdown IR
- 共享 block / inline 渲染原语

这里的“非共享”是：

- chat 的最终 width-aware 文档渲染
- editor 的 `wrap_engine`、光标叠加、insert 模式源码显示、搜索高亮

这条边界必须守住，否则 editor 会和现有 `wrap_engine` 发生双重 wrap / continuation 语义冲突。

## 总体设计

```
src/markdown/
├── mod.rs
├── ir.rs              # Block / Inline / TableCell / SourceRange
├── parser.rs          # md -> ParsedDocument { blocks, line_index, ... }
├── preprocess.rs      # normalize_terminal_text / table separator fix / quote fix
├── theme.rs           # MdStyle trait
├── render/
│   ├── mod.rs
│   ├── inline.rs      # Inline -> Vec<Span>
│   ├── block.rs       # Block -> RenderedBlock / Vec<Line>
│   ├── code_block.rs
│   ├── table.rs
│   ├── heading.rs
│   ├── list.rs
│   └── blockquote.rs
└── block_index.rs     # source_line -> block idx / block ranges
```

## 关键抽象

### 1. `MdStyle` trait

共享层不直接依赖 `Theme` 或 `EditorTheme`，而是抽象为 `MdStyle` trait。

原因：

- chat 和 editor 的背景策略不同
- chat 的代码块 / 表格 / blockquote 是 bubble 场景
- editor 有 `bg_primary`、cursor overlay、insert 模式源码显示
- editor 还有代码块右边框按最长行对齐的特殊行为

建议 trait 只暴露“markdown 渲染语义需要的样式”，不要把整个 theme object 透传进共享层。

### 2. Block IR 携带源位置信息

parser 使用 `Parser::into_offset_iter()` 记录 block 的 byte offset，再映射到源码行号。

最低需要：

```rust
struct SourceRange {
    start_line: usize,
    end_line: usize,
}

struct Block {
    source: SourceRange,
    kind: BlockKind,
}
```

注意：`source line -> block` 只能回答“这行源码属于哪个 block”，**不能**回答“这行最终显示第几行”。后者必须由渲染层另建索引。

### 3. Inline 也是 IR

表格单元格、列表项、段落、blockquote 内部都统一表示成 `Vec<Inline>`。

这样：

- 表格里的 `**bold**`、`` `code` ``、链接天然可用
- CommonMark delimiter rules 全部由 `pulldown-cmark` 负责
- editor / chat 不再维护两套 inline 识别逻辑

## Chat 与 Editor 的职责边界

### Chat

chat 继续走“整文档 width-aware 渲染”：

```rust
parse(md) -> ParsedDocument
render_document_wrapped(&doc, &ChatMdStyle, width) -> Vec<Line>
```

chat 这一侧允许共享层决定：

- 段落换行
- 表格列宽压缩
- 代码块边框
- heading / list / blockquote 的最终输出

### Editor

editor **不复用** chat 的最终文档渲染函数。

原因：

- editor 已经有 `wrap_engine`
- editor 的 continuation 语义依赖 `VisualLine`
- insert 模式当前行显示源码，不是渲染结果
- cursor / search / visual selection 都是在 editor 自己的视觉行模型上叠加

所以 editor 只复用：

- parser
- IR
- inline renderer
- block renderer 原语

editor 的共享层入口应是下面这种级别，而不是 `render_document_wrapped(...)`：

```rust
render_block_primary_rows(&block, &EditorMdStyle, ctx) -> RenderedBlock
```

其中 `RenderedBlock` 需要同时包含：

- 最终 `Vec<Line>`
- 这些渲染行和源码行之间的映射

## Editor 缓存模型

第一版不要只存 `line_to_block`，这不够。

因为：

- 一个源码行可能渲染成多行
- 一个 block 的渲染行数也不等于源码行数
- 表格、代码块、heading 分隔线、blockquote 空行都会打破 `sub_idx = logical_line - start_line`

建议缓存结构至少是：

```rust
struct MarkdownCache {
    revision: u64,
    blocks: Vec<Block>,
    line_to_block: Vec<Option<usize>>,
    block_rendered: Vec<Option<RenderedBlock>>,
    block_dirty: Vec<bool>,
}

struct RenderedBlock {
    lines: Vec<Line<'static>>,
    source_rows: Vec<RenderedRowRef>,
}

struct RenderedRowRef {
    source_line: Option<usize>,
}
```

如果 editor 需要根据逻辑行回查“该行主显示内容”，还需要额外维护：

```rust
line_to_render_rows: Vec<Vec<(block_idx, rendered_row_idx)>>
```

这才足够支持：

- 非当前行按共享层渲染
- 当前逻辑行查到对应的主渲染行
- 后续对光标 / 搜索 / visual selection 做覆盖

### 为什么用 `revision`，不用 `doc_hash`

`doc_hash` 每次都要重新扫描全文，本身就是 O(n)。

editor 里的文本修改都是自己驱动的，直接维护单调递增 `revision: u64` 更合适：

- 编辑一次 `revision += 1`
- cache 命中判断简单
- 大文件下不会多做一次全文 hash

## 失效与刷新策略

第一版先定一个保守但正确的策略：

- **全文 parse**
- **按 block 懒渲染**
- **dirty block 重渲染**
- **Insert 模式延迟刷新**

### dirty 策略

- `TextBuffer` 修改后，editor 标记受影响行范围
- `mark_dirty(line_range)` 找出覆盖的 block
- 再连带标记上下相邻一个 block，处理 fence / list / blockquote 边界漂移

### Insert 模式刷新

这是性能最关键的一条：

- 当前行在 Insert 模式本来就显示源码
- 所以没必要每次按键都同步全文 reparse + rerender

第一版就应该带上：

- 80-150ms debounce，或
- 退出 Insert 模式 / 停止输入后再刷新 markdown cache

否则大文件场景下，“一次打开慢一点”还不是最大问题，“每次输入都卡下一帧”才是最大问题。

### 先不要做的事

第一版不要把“从最近空行开始局部 parse”写进正式实施目标。

原因：

- CommonMark / GFM block 边界不适合靠空行启发式切片
- fence/list/blockquote 很容易误切
- 实现复杂度明显上升，但第一阶段未必需要

如果后面性能数据证明全文 parse 仍然不够，再评估更细粒度的局部 parse。

## 推进步骤

每一步都要保证 `src/markdown/parser/tests.rs` 全过，作为不退化的硬约束。

### Step 1：模块落位（零行为变化）

- 把现有 markdown 模块统一放到 `src/markdown/`
- 保留 `markdown_to_lines(...)` 兼容 facade
- chat 调用方尽量不改行为，只改 import / 模块路径

**目标**：`cargo build` + `cargo test` 全过，零行为变化。

### Step 2：抽 `MdStyle` trait

- 把 parser / render 中直接依赖 `Theme` 的地方抽掉
- chat 侧实现 `ChatMdStyle`
- 行为保持完全一致

### Step 3：parser 输出 IR，chat 改走共享 render

- `parse(md) -> ParsedDocument`
- `render_document_wrapped(&doc, &ChatMdStyle, width) -> Vec<Line>`
- `markdown_to_lines(...)` 保留为 facade

这是最大的一步重构，但验证面最完整，因为 chat 已经有现成测试基线。

### Step 4：editor 接入共享层（先接 inline + 段落）

- 新建 `editor_core/markdown_cache.rs`
- `render_non_insert_line` 改为查 markdown cache
- 先只迁 inline + 普通段落
- 删除 `editor_core/renderer/inline.rs`

**直接修复**：`**xx**` 在中文 / 标点 / 嵌套场景下的不稳定渲染。

### Step 5：迁表格

- editor 表格改走共享 table renderer
- 删除 `editor_core/renderer/table.rs`

**直接修复**：

- 表格里 `**bold**` / `` `code` `` / 链接不渲染
- `split('|')` 切坏 cell

### Step 6：迁 heading / list / blockquote

- editor 端 `renderer/line.rs` 的 markdown 路径逐步走共享层
- 保留 editor 独有的 cursor / insert / continuation 覆盖逻辑

### Step 7：性能验证与调优

重点看两件事：

- 大文件首屏打开时间
- 连续输入时按键到下一帧的延迟

必要时再加：

- block_rendered 淘汰策略
- viewport 附近 block 的优先渲染
- 更细粒度的 dirty 策略

## 风险与边界

### 1. Insert 模式保持现状

当前光标行在 Insert 模式继续显示源码，不走共享层。

共享层不承担“源码和渲染半混合显示”的职责。

### 2. `wrap_engine` 不动

editor 的折行继续由 `wrap_engine` 决定。

共享层不要接管 editor 普通段落的最终换行，否则会和 `VisualLine` / continuation 冲突。

### 3. 代码块右边框对齐是 editor 独有行为

editor 的 `calculate_code_block_max_width` 仍然需要保留，或以 hook 形式挂在 `EditorMdStyle` / editor adapter 上。

chat 不需要这套行为，不能强塞进共享层默认语义。

### 4. 搜索高亮 / visual selection / cursor overlay 仍由 editor 后处理

共享层只负责“基础 markdown 呈现”。

editor 自己继续负责：

- cursor block
- search highlight
- visual mode selection overlay

### 5. 测试基线必须前移

不只要保 `parser/tests.rs`，还应给 editor 新增至少这几类回归测试：

- 普通段落里的 `**bold**`
- 表格 cell 里的 inline code / strong / link
- 中文 + 引号边界
- 代码块和表格在窄宽度下的渲染稳定性

## 当前进度

- [x] 方案确认
- [x] Step 1：模块落位 / 搬目录
- [x] **Step 2：抽 `MdStyle` trait** — 新增 `src/markdown/theme.rs` 定义 trait + `impl MdStyle for crate::theme::Theme`；parser/parser-text/parser-table 改为通过 trait method 取色；`markdown_to_lines` 签名改为 `theme: &dyn MdStyle`，5 处调用方零改动（`&Theme` 自动 unsizing coerce）；parser.rs 已彻底切断对 `chat::Theme` 的直接引用；`cargo clippy --lib --bins -D warnings` 干净，`cargo test --lib` 290 passed。
- [ ] **Step 3：parser 输出 IR**（下一步）
- [ ] Step 4：editor 接入（inline + 段落）
- [ ] Step 5：迁表格
- [ ] Step 6：迁 heading / list / blockquote
- [ ] Step 7：性能验证与调优

### Step 2 实施备注

- **未引入显式 `ChatMdStyle` 包装类型**：因为 `Theme` 是顶层 `crate::theme::Theme`（不属于 chat 模块），直接 `impl MdStyle for Theme` 即可，无需 wrapper struct。这与规划中"chat 侧实现 ChatMdStyle"的语义等价，但避免一层冗余。后续 editor 侧实现 `EditorMdStyle` 时同理：要么 `impl MdStyle for EditorTheme`，要么写专用 struct，按届时需要选择。
- **残留依赖**：`src/markdown/highlight.rs` 和 `src/markdown/theme.rs` 仍然 `use crate::tui::editor_core::EditorTheme`。Step 3 起会引入独立的 `SyntaxHighlightTheme` 抽象彻底消除该跨模块依赖。
- **测试依赖修复**：`parser/tests.rs` 之前通过 `use super::*` 隐式继承 parser 的私有 `use Theme`；切断后改为 tests.rs 自己显式 import。

## 当前建议

下一步优先做 **Step 2：抽 `MdStyle` trait**，但在开始前先把 editor 侧的目标接口定死：

- editor 不直接调用 `render_document_wrapped`
- editor 需要 `RenderedBlock + source row mapping`
- Insert 模式必须带 debounce / idle refresh

这三个点不先定死，后面很容易又回到“共享了一半，结果为了兼容 editor 再补一套特殊逻辑”的状态。
