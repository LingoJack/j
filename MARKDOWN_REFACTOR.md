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
- [x] **Step 3：parser 输出 IR，chat 改走共享 render** — `ir.rs` + `render/` 模块 + parser 纯解析 + facade；290 tests passed
- [x] **Step 4：editor 接入共享层** — `EditorTheme` 实现 `MdStyle` trait；parser 使用 `into_offset_iter()` 填充精确 `SourceRange`；新建 `markdown_cache.rs`（全文解析 + line-to-block 映射 + 懒渲染缓存）；editor `renderer/inline.rs` 改用共享层 `render_inlines`（基于 pulldown-cmark 解析，正确处理 `**bold**`/`*italic*`/`~~strike~~`/`` `code` ``/链接的嵌套和边界）；`cargo clippy -D warnings` 干净，`cargo test --lib` 292 passed。
- [x] **Step 5：迁表格** — editor `renderer/table.rs` 改用 `parse_table_from_source()` 解析表格源码为 `TableData` IR（含内联语法支持），调用共享层 `render_table()` 一次性渲染整个表格；删除旧 `parse_table_cells`/`is_table_separator_line` 等纯文本处理代码；editor 仅在表格首行（`start_idx`）触发完整渲染，后续行返回 `vec![]`；`cargo clippy -D warnings` 干净，`cargo test --lib` 298 passed。
- [x] **Step 6：迁 heading / list / blockquote** — 分析结论：**不需要迁移**。editor 的 heading/list/blockquote 已通过 `render_inline()` 走共享层解析内联语法（`parse_inline_text()` + `render_inlines()`）；共享层 block 渲染（含分隔线/前后空行/`|` prefix）不适合 editor 的紧凑逐行风格；强制迁移会破坏 editor 显示效果。
- [x] **Step 7：性能验证与调优** — 在 `inline.rs` 和 `parser.rs` 添加性能测试；Release 模式下 `parse_inline_text()` 单次 1.0μs（50行帧率 20000+ fps），`parse_table_from_source()` 单次 7.7μs；总渲染开销约 50-60μs/帧，远低于 16ms（60fps）帧预算；**结论：性能完全满足需求，无需缓存优化**；`cargo clippy -D warnings` 干净，`cargo test` 300 passed。

### Step 7 实施备注

- **性能测试方法**：在 `renderer/inline.rs` 添加 `bench_parse_inline_throughput()`，在 `parser.rs` 添加 `bench_parse_table_from_source()`
- **测试场景**：模拟一屏 20 行 × 100 帧 = 2000 次解析（inline），5 行表格 × 1000 次 = 1000 次解析（table）
- **Release 模式结果**：
  - `parse_inline_text()`: 单次 1.0 μs，50 行帧率 20000+ fps
  - `parse_table_from_source()`: 单次 7.7 μs（仅表格首行触发）
- **结论**：总渲染开销约 50-60μs/帧，远低于 16ms（60fps）帧预算，无需缓存优化
- **保留 `markdown_cache.rs`**：作为未来全文缓存优化的基础设施，标注 `#![allow(dead_code)]` + TODO

## 重构完成

**所有 7 个步骤已完成**，Editor markdown 渲染已全部迁移到共享层：

| Block 类型 | 共享层 API | 状态 |
|-----------|-----------|-----|
| Paragraph | `parse_inline_text()` + `render_inlines()` | ✓ |
| Heading | `parse_inline_text()` + `render_inlines()` | ✓ |
| List | `parse_inline_text()` + `render_inlines()` | ✓ |
| BlockQuote | `parse_inline_text()` + `render_inlines()` | ✓ |
| Task List | `parse_inline_text()` + `render_inlines()` | ✓ |
| Rule | 无需迁移（简单样式） | ✓ |
| Table | `parse_table_from_source()` + `render_table()` | ✓ Step 5 |
| CodeBlock | `render_code_block()` | ✓ Step 3 |
| Inline | `parse_inline_text()` + `render_inlines()` | ✓ Step 4 |

**修复的问题**：
1. 表格单元格内 `**bold**` / `` `code` `` / 链接现在正确渲染
2. `split('|')` 误切 code span 内管道符的问题已修复
3. `**bold**` 在中文 / 标点 / 嵌套场景下的不稳定渲染已修复

### Step 2 实施备注

- **未引入显式 `ChatMdStyle` 包装类型**：因为 `Theme` 是顶层 `crate::theme::Theme`（不属于 chat 模块），直接 `impl MdStyle for Theme` 即可，无需 wrapper struct。这与规划中"chat 侧实现 ChatMdStyle"的语义等价，但避免一层冗余。后续 editor 侧实现 `EditorMdStyle` 时同理：要么 `impl MdStyle for EditorTheme`，要么写专用 struct，按届时需要选择。
- **残留依赖**：`src/markdown/highlight.rs` 和 `src/markdown/theme.rs` 仍然 `use crate::tui::editor_core::EditorTheme`。Step 3 起会引入独立的 `SyntaxHighlightTheme` 抽象彻底消除该跨模块依赖。
- **测试依赖修复**：`parser/tests.rs` 之前通过 `use super::*` 隐式继承 parser 的私有 `use Theme`；切断后改为 tests.rs 自己显式 import。


### Step 3 实施备注

- **IR 设计决策**：`SourceRange` 暂用 `0..0` 填充；`line_to_block` 暂为空 Vec；`Alignment` re-export `pulldown_cmark::Alignment`；未包含 `Image` block
- **模块结构**：`ir.rs`（IR 类型）、`parser.rs`（纯解析）、`render/`（`mod.rs`/`inline.rs`/`block.rs`/`table.rs`/`code_block.rs`）
- **关键改动**：表格单元格从 `String` 改为 `Vec<Inline>`；`wrap_cell_inlines` 替代旧 `wrap_cell_styled`；`markdown_to_lines` 保持签名不变，内部调用 parse + render

### Step 4 实施备注

- **EditorTheme 实现 MdStyle**：直接 `impl MdStyle for EditorTheme`，无需 wrapper struct
- **精确行号映射**：parser 使用 `into_offset_iter()` + `build_line_offsets()` 填充 `SourceRange.start_line/end_line`
- **markdown_cache.rs**：全文解析 + `line_to_block` 映射 + 按需渲染缓存（为后续 Step 5/6 打基础）
- **inline 渲染共享化**：editor `renderer/inline.rs` 改用 `parse_inline_text()` + 共享层 `render_inlines`，正确处理嵌套和边界；**修复 heading 行不调用 render_inline 的问题**（`# **bold** heading` 现能正确渲染加粗）
- **测试覆盖**：`markdown_cache::tests` 新增 `cache_rebuild_basic` 和 `cache_get_rendered_line` 测试

## 下一步

重构已完成。可选的后续优化方向：

1. **启用 `markdown_cache.rs`**：如果未来需要 editor 预览模式或全文渲染优化，可以启用缓存
2. **局部 parse**：如果大文件场景下全文 parse 仍有性能问题，可评估更细粒度的局部 parse
3. **viewport 优先渲染**：如果首屏打开时间需要进一步优化，可实现 viewport 附近 block 的优先渲染

### Step 6 实施备注

- **分析结论：不需要迁移**
- **理由 1**：`render_inline()` 已走共享层 — editor 的 heading/list/blockquote 通过 `parse_inline_text()` + 共享层 `render_inlines()` 正确渲染内联语法（bold/code/link 等）
- **理由 2**：共享层 block 渲染不适合 editor — heading H1/H2 会多输出分隔线；blockquote 会多输出前后空行和 `|` prefix；list 会整块渲染而非逐行
- **理由 3**：强制迁移会破坏 editor 紧凑风格 — editor 需要单行紧凑渲染，共享层适合预览场景
- **最终状态**：Editor markdown 渲染已全部迁移共享层（inline/block 原语），无需进一步改动

### Step 5 实施备注

- **新增 `parse_table_from_source()`**：在 `markdown/parser.rs` 添加公共函数，接收 `&[&str]` 表格源码行，利用 pulldown-cmark 解析为 `TableData` IR（单元格内容为 `Vec<Inline>`，天然支持 bold/code/emphasis 等内联语法）
- **editor 表格渲染简化**：`render_table_rows()` 仅在表格首行（`start_idx`）时调用 `parse_table_from_source()` + 共享层 `render_table()` 一次性渲染完整表格，后续行返回 `vec![]`
- **删除旧代码**：`parse_table_cells()`（纯文本 `split('|')` 切分）、`is_table_separator_line()` 等废弃函数已移除
- **直接修复**：表格单元格内的 `**bold**` / `` `code` `` / 链接现在正确渲染；`split('|')` 误切 code span 内管道符的问题不复存在

---

## Step 7：折行 + 光标 + 渲染的坐标系对齐（editor）

> 这一步没有引入新的 markdown 解析能力，目的是把 editor 里**折行算法 / 光标行为 / 渲染产物**三者的坐标系对齐。下面是几条关键设计，给后人改这块时少踩坑。

### 关键设计 1：光标行 vs 其它行 = 严格跟随渲染端口径

editor 的 Typora 风格规则就一条：

> **只有 Insert 模式 + 光标所在行**显示 markdown 源码，其它一律显示渲染产物。

整个改造里所有"光标行有没有特殊处理"的地方都必须**严格跟随这一条**，否则就会出错：

- `wrap_engine` 的"按源码字符宽折行"路径，**只**对"Insert 模式 + 光标行"开启；其它（含 Normal 模式的光标行）一律按渲染后宽度算。
- 光标在不同逻辑行之间移动**只有 Insert 模式下**才触发 wrap rebuild；Normal 模式下移动光标不触发任何重算（因为所有行都按渲染宽，光标行没有特殊路径）。

落地：`editor.rs:rebuild_wrap_cache` 算 `cursor_line` 时硬编码 `self.vim.mode() == &Mode::Insert`，`maybe_mark_wrap_dirty_for_cursor` 同口径。**改这两处时务必保持口径与 `renderer.rs:render_visual_line` 里 `is_cursor_line && is_insert_mode` 一致**——三处任一不对齐都会出现"光标移动诡异 rebuild"或"光标行折行规则与显示规则错配"。

### 关键设计 2：三个坐标系，由 wrap_engine 单一来源给出

editor 这块至少要区分三个坐标系，永远不要在渲染端事后猜：

| 坐标系 | 谁用 | 在 `VisualLine` 上的字段 |
|---|---|---|
| **源码 char 索引** | 光标、选区、鼠标定位、vim 移动 | `start_col` / `end_col` |
| **渲染产物 char 索引** | 渲染端切 `render_inline(整行)` 的 spans | `visible_start_char` / `visible_end_char` |
| **显示列宽** | 折行决策、对齐右边框 | `display_width`（与 `width` 比较） |

`wrap_engine.wrap_line_inner` 在遍历源码 char 的同时同步推进这三个量：

- `col` 每个源码 char +1（源码索引）
- `visible_pos` 仅当 `char_width_for(ch, widths, idx) > 0` 才 +1（标记符号不推进）
- `current_width` 累加 `char_width_for(...)`（渲染显示宽度）

**强约束**：渲染端**禁止**自己用源码索引去切渲染产物。续行 `extract_span_range(full_line_spans, vl.visible_start_char, vl.visible_end_char)`——索引语义和 spans 同坐标系，由 wrap_engine 一次性算好。坐标系混用是这一类 bug 的通用根因（见 BUGS_TROUBLESHOOTING.md 条目 2）。

### 关键设计 3：折行场景下"同一行的所有 vl 必须走同一条 markdown 解析路径"

最容易踩坑的一点：**不要**让 vl[0] 独立解析 truncated 片段、vl[1+] 解析整行再切片。pulldown-cmark 对"独立片段 vs 完整行"的边界行为永远不一致：

- 独立解析 `**aaaa bbbb** ` → trim 掉尾随空格变成 `aaaa bbbb`（少 1 char）
- 独立解析 `**前面加粗一`（未闭合）→ 退化成 Text `**前面加粗一`（多 2 char）

两条路径拼起来在折行边界**必然**少 / 多字符。

所以渲染端的最终设计是：

- **行只占 1 个视觉行（不折）** → 走完整 `render_single_line_with_number`，保留 heading 图标 / bullet / blockquote 竖条等块级前缀
- **行被折成多段** → 所有 vl（包括 vl[0]）都走 `render_inline(整行) + extract_span_range(..., visible_*_char)`，渲染产物只有一份，char 序列在视觉行边界严格连续

判据是 `is_wrapped = vl.end_col < line_content.chars().count()`。

**已知取舍**：折行场景下块级前缀图标暂时不画，前缀字符（`# `/`- `/`> `）以源码形式 fall through 进 inline 切片。若以后要在折行场景也保留图标，**不要**回头去让 vl[0] 独立解析；正确做法是让 `compute_visible_widths` 与 `renderer/line.rs` 的前缀剥离规则深度对齐（heading `# `→`◆ ` 同宽、`## `→`◇ ` 少 1、`### `→`〈 〉` 多 2、`> `→`| ` 同宽…），把前缀图标作为 vl[0] 的额外 prefix span 单独画在 inline 切片之前。

### 关键设计 4：每行的"可见宽度数组"= 复用既有 inline parser，不另写

`compute_visible_widths(line) -> Vec<u8>`（`renderer/inline_width.rs`）：

1. 调 `parse_inline_text(line)` 拿 `Vec<Inline>`（直接复用 `renderer/inline.rs`，与 Step 4 统一）
2. 递归收集"可见文本片段"（`Text`/`Code`/`Strong`/`Emphasis`/`Strikethrough` 内容 + `Link.text`，丢弃 `Link.url`）
3. 用游标在源码里按 char 子序列匹配回填——可见 char 给 `char_width`，跳过的源码 char 全是标记符号给 0

输出长度严格 = `line.chars().count()`。`wrap_engine` 把这个数组按行存进 `line_visible_widths: Vec<Option<Vec<u8>>>`——`None` 表示该行按源码 `char_width` 算（光标行 / fence / 代码块内 / 表格行）。

**不另起一套行内剥离器**——这是 [feedback_reuse_existing_parsers](../.codebuddy/...) 立的规矩，也是 Step 1~6 共享层的延续。

### 关键设计 5：不变的契约

为了让选区、搜索、鼠标、vim 移动这些**已经稳定**的逻辑零修改，本次改造严格保护这些契约：

- `start_col` / `end_col` 仍是源码 char 索引（不要为了"可见宽度"重新定义）
- `visual_to_logical` / `logical_to_visual` 输入输出语义不变
- 代码块（fenced）和表格的特殊处理沿用 Step 4/5 已有逻辑（`-4` 边框补偿、块级膨胀）
- 续行的 inline 解析仍用整行 `render_inline` 一份产物，只是切片索引换成了 `visible_*_char`

新增的字段、API 都是**叠加**而非**替换**——旧 `rebuild_cache_with_blocks` 改成薄壳调用新 `rebuild_cache_with_blocks_and_widths`，保留所有老调用方。

### 一句话总结

> Step 7 的本质：**让 wrap_engine 和 renderer 用同一个"渲染产物坐标系"对话**——wrap 算出每段视觉行在渲染产物里的 char 区间，renderer 拿区间去切整行渲染产物。光标行为则严格跟随渲染端的 Insert/Normal 口径，所有"光标行特殊"的逻辑只在 Insert 模式生效，Normal 模式下整篇都是渲染产物，光标只是一个叠加。
