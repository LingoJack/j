# 常见 BUG 排查

记录已修复的隐蔽 BUG 与其根因，避免后续在同一处重复踩坑。新增条目按"现象 / 触发条件 / 根因 / 修复 / 教训"五段写。

---

## 1. Markdown 编辑器：鼠标拖选高亮位置偏移、底部拖动看不到高亮

**涉及文件**

- `src/tui/editor_core/editor.rs`：`screen_to_render_pos` / `clamped_render_pos_for_drag` / `RenderMeta`
- `src/tui/editor_core/editor/render.rs`：鼠标选区高亮循环（`mouse_selection`）

### 现象

1. 在编辑器里用鼠标拖选文本，高亮色块和鼠标实际位置错开（看似有"行偏移"）。
2. 文档较长、滚动到中后段后，再拖选基本看不到任何高亮。
3. 拖到视口底部时高亮整段消失。

未滚动 / 文档很短时不易复现，因此早期容易漏掉。`jcli`

### 触发条件

`render_meta.rendered_offset > 0`，也就是当帧 `visual_offset > 0`——只要视口往下滚过一些行就会触发。文档不超过一屏时 `rendered_offset == 0`，BUG 被掩盖。

### 根因

`RenderMeta` 里两个字段的语义没对齐，但消费者按"全局行号"约定使用：

| 字段 | 实际语义 | 旧注释 / 错误理解 |
| --- | --- | --- |
| `map_index` | `rendered_lines` / `vl_map` 的局部下标 | 一度被注释为"全局渲染行号" |
| `rendered_offset` | `rendered_lines[0]` 对应的全局行号 | （正确） |

`MouseSelection.anchor / current` 的约定是**全局渲染行号**——`render.rs` 的高亮循环：

```rust
for (idx, line) in all_visual_lines.iter_mut().enumerate() {
    let gline = visual_offset + idx;        // 全局行号
    if gline < sr || gline > er { continue; }
    ...
}

```

但 `screen_to_render_pos` 的旧实现：

```rust
// ❌ 错：把 map_index（局部下标）当成全局行号
let global_row = content_y + self.render_meta.map_index;
let local_idx = global_row.checked_sub(self.render_meta.rendered_offset)?;

```

两个错刚好在 `rendered_offset == 0`（未滚动）时**互相抵消**，所以本地小 demo 永远跑不出 BUG。一旦 `rendered_offset > 0`：

- 存进 `MouseSelection` 的 sr / er 比真实全局行号小 `rendered_offset`。
- 高亮循环 `gline = visual_offset + idx` 用真实全局行号比对，所有 `gline` 都 `> er`，整段选区被过滤 → 看不到高亮。
- `clamped_render_pos_for_drag`（拖出 area 时的 fallback）有同样的错，所以底部拖动尤其明显。

### 修复

在两个换算函数里把"局部下标"和"全局行号"区分清楚——返回真正的全局行号：

```rust
let local_idx = self.render_meta.map_index + content_y;     // 取行内容
let line = self.render_meta.rendered_lines.get(local_idx)?;
let global_row = self.render_meta.rendered_offset + local_idx; // 给消费者
Some((global_row, char_offset))

```

`copy_mouse_selection_to_clipboard` 里 `local_anchor = sel.anchor.0 - rendered_offset` 的约定也随之自洽：sr/er 是全局行号，相减回到局部下标，索引 `rendered_lines` 取可见正文。

### 教训 / 防回归

- **永远把"局部下标"和"全局行号"显式命名分开**，不要把两个概念塞进同名变量。本项目里命名统一规则：`local_idx` / `global_row`。
- 字段注释必须**只描述实际语义**，不要先写错了再让代码去"按错的注释行事"——这次的 bug 直接源于注释和实现错位。
- 鼠标 / 滚动相关 bug 一定要在"已经滚动"的状态下手动验证，不能只看首屏。
- 修复后保留警示注释（`screen_to_render_pos` / `RenderMeta::map_index` / `MouseSelection`）和这份文档的反向引用，下次有人再来改鼠标坐标换算时第一眼就能看到。

---

## 2. Markdown 编辑器：折行第二行开头"跳"几个字符 / 少字符

**涉及文件**

- `src/tui/editor_core/wrap_engine.rs`：`VisualLine` 的 `visible_start_char` / `visible_end_char` 字段、`wrap_line_inner`、`compute_visual_line_count`、`char_width_for`
- `src/tui/editor_core/renderer.rs`：`render_non_insert_line` 续行分支与"普通 markdown 行"分支、`extract_span_range`
- `src/tui/editor_core/renderer/inline_width.rs`：`compute_visible_widths`
- `src/tui/editor_core/renderer/inline.rs`：`parse_inline_text`（`pub(super)` 暴露给 inline_width 复用）
- `src/tui/editor_core/editor.rs`：`rebuild_wrap_cache`、`maybe_mark_wrap_dirty_for_cursor`、`last_wrap_cursor_line`

### 现象

引入"按渲染后宽度折行"（标记符号 `**`/`*`/`~~`/```/`[]()` 折算 0 宽）后，**含 markdown inline 标记的长段落**被折行时，第二行开头看起来"跳"了几个字符，或者第一行右边少了一个空格——直观就是渲染产物在折行边界拼不顺。

未折行的行、纯文本行、代码块、表格行不复现。

### 触发条件

同时满足：

1. 非光标行（即"按渲染后宽度"路径，含 Normal 模式所有行 + Insert 模式非光标行）。
2. 该逻辑行被 `wrap_engine` 折成 ≥ 2 段视觉行。
3. 第一段视觉行的源码末尾切到了 markdown 闭合标记附近（典型：`**aaaa bbbb** ` 这种 Strong 闭合后跟空格的位置）；或者切到了未闭合 `**` 内部（`**前面加粗一` 这种）。

### 根因

**两条渲染路径在切分边界 char 数不对齐**：

- vl[0]（折行后第一段）走 `render_single_line_with_number(truncated, ...)`，**把截断的源码片段独立扔给 markdown 解析**。
- vl[1+] 续行走 `render_inline(整行) + extract_span_range(spans, visible_start_char, visible_end_char)`，从**整行**渲染产物里按渲染端 char 索引切片。

pulldown-cmark 在解析独立片段时的边界行为和解析整行不一致：

| truncated 形态 | 独立渲染产物 | 整行渲染前 N char | 差异 |
| --- | --- | --- | --- |
| `**aaaa bbbb** `（尾随单空格） | `aaaa bbbb`（9 char） | `aaaa bbbb `（10 char，含空格） | 少 1 空格**** |
| `normal **bold** ` | `normal bold`（11） | `normal bold `（12） | 少 1 空格**** |
| `**前面加粗一`（未闭合 `**`） | `**前面加粗一`（7） | `前面加粗一`（5） | 多 2 个**** `*`**** |

差异原因：

1. pulldown-cmark 把 Strong 闭合后**紧跟的尾随空白**当作 paragraph 末尾 trim 掉；整行解析时这个空格是后续 Text token 的前导内容，保留。
2. 未闭合的 `**` 在独立解析里退化成 Text 字面量（多出 2 char）；整行解析里被识别为 Strong 包裹。

vl[0] 渲染出 N 个 char，vl[1] 续行从整行渲染产物的 char index 等于 `vl[0].visible_end_char` 开始切——两条流的 char 序列在边界处不连续 → 拼起来出现"丢空格"或"多 `**`"，视觉上就是第二行开头跳了几个字符。

**重要的迷惑点**：wrap_engine 本身**没错**——`visible_start_char` / `visible_end_char` 是按 `compute_visible_widths` 严格累加出来的，所有 vl 的 visible 区间首尾相接、覆盖整行。问题完全出在渲染端"独立解析片段 vs 解析整行"两条路径不可调和。

### 修复

放弃"vl[0] 独立解析 truncated"路径，**只要该行被折成多段，第一段也走和续行完全一致的"整行 inline 渲染 + 按 visible char 索引切片"**——`render_non_insert_line` 末尾的"普通 markdown 行"分支用 `is_wrapped = vl.end_col < line_content.chars().count()` 判断，折行场景走切片路径，单一 vl 时才走 `render_single_line_with_number`。

为支撑这套切片，wrap_engine 必须在每个 VisualLine 上同步维护"渲染端 char 索引"：

- `VisualLine` 新增 `visible_start_char` / `visible_end_char` 字段。
- `wrap_line_inner` 推进 `col`（源码 char）的同时根据 `char_width_for(ch, widths, idx)` 是否 > 0 决定 `visible_pos` 是否推进——标记符号 width = 0 不推进，正文字符（含 CJK，char_width=2 但 char 数 = 1）推进。
- `extract_span_range` 拿到的就是渲染产物 char 索引，和 vl 的 `visible_*_char` 同坐标系。

**配套约束**：

- "按源码宽折行"的行（光标行 / 代码块 / 表格）仍只在 Insert 模式下、且光标 row == 该行时启用。Normal 模式所有行都按渲染宽——和 `renderer.rs:render_visual_line` 里 `is_cursor_line && is_insert_mode` 才走源码渲染的口径严格对齐。
- `editor.rs:rebuild_wrap_cache` 算 `cursor_line` 时也要带 mode 判断；`maybe_mark_wrap_dirty_for_cursor` 同样——Normal 模式下光标在行间移动不能触发 rebuild。

### 教训 / 防回归

- **不要让"同一个逻辑行的不同 vl"走两条不同的 markdown 解析路径**。pulldown-cmark 对"独立片段 vs 完整行"的边界处理永远存在差异（trim 尾随空白、未闭合标记退化），跨路径拼接一定会在某个边界出错。**统一走整行解析 + 切片**，渲染产物的坐标系就只有一个。
- 折行宽度 / 渲染宽度 / char 索引 / 显示列**至少四个坐标系**很容易混。改 wrap / 渲染相关代码前先想清楚每个变量是哪个坐标系——`start_col / end_col` 是源码 char 索引，`visible_start_char / visible_end_char` 是渲染产物 char 索引，`display_width` 是显示列。坐标系混用是这一类 bug 的通用根因。
- "按渲染宽折行" vs "按源码宽折行"必须和**渲染端 Insert/Normal 的 source vs rendered 路径口径完全一致**，否则会出现 Normal 模式下光标移动也触发 rebuild、或者光标行折行规则与显示规则错配的诡异表现。
- 这类 bug 单元测试很难发现——单测 wrap_engine 会说"visible 区间首尾相接、覆盖整行"，单测 inline_width 会说"标记符号是 0 宽"，但没有任何单测覆盖**"vl[0] 独立解析产物 vs 整行解析产物的 char 数是否一致"**。改这块代码时**必须**用真实编辑器手动验证一段含 `**bold**` 的长文本，在不同 wrap_width 下都拉过一遍，看折行边界处是否连贯。
- 折行场景下 heading 图标 / bullet / blockquote 竖条等块级前缀目前是**有意放弃**的，前缀字符以源码形式 fall through 到 inline。如果后续要在折行场景也保留图标，必须让 `compute_visible_widths` 和渲染端的前缀剥离逻辑深度对齐（heading `# ` 渲染成 `◆ ` 同宽、`## ` 渲染成 `◇ ` 少 1 列、`### ` 渲染成 `〈 〉` 多 2 列、`> ` 渲染成 `\| ` 同宽…），并把前缀图标作为 vl[0] 的额外 prefix span 单独画——别再回头去走"vl[0] 独立解析 truncated"那条路。

---

## 3. Markdown 编辑器：滚动到尾部时内容显示不全、底部提前出现 `~`

**涉及文件**

- `src/tui/editor_core/editor/render.rs`：`render()` 中视口同步、`render_start` / `render_end` 计算、`expand_render_end_to_cover_visual_range`
- `src/tui/editor_core/editor.rs`：EOF 渲染回归测试

### 现象

编辑器打开较长文档并定位到文件尾部时，底部内容没有完整显示；实际还有文本行应出现在视口内，但屏幕底部提前出现 Vim 风格的 `~` 占位行。

### 触发条件

常见于以下组合：

1. 文档超过一屏。
2. 打开时光标策略为 EOF，或其它逻辑在本帧 render 内把 `scroll_offset` 同步到文件尾部附近。
3. 末尾附近包含多视觉行内容（长段落折行、表格 / 代码块等块级渲染），固定逻辑行缓冲无法覆盖完整视口。

### 根因

旧渲染流程先用**进入 render 时的旧 `scroll_offset`** 计算 `start_logical` / `end_logical`，构建 `all_visual_lines`；随后才根据光标位置把 `scroll_offset` 同步到 EOF。

这会造成两个问题：

1. 当光标追踪改变了 `scroll_offset` 时，当帧已经构建好的 `all_visual_lines` 仍对应旧视口，不一定覆盖新视口所需的全局视觉行。
2. `render_end = end_logical + 3` 只是固定经验缓冲，不能保证覆盖 `scroll_offset + content_height`。遇到末尾长折行或块级渲染时，局部渲染数组长度不足，`visible_end_local` 被 `all_visual_lines.len()` 截断，后续行就被 `~` 填充。

### 修复

1. 在构建渲染范围前，先根据光标位置同步并 clamp 最终 `scroll_offset`。
2. 基于最终 `scroll_offset` 重新计算视口对应的逻辑行范围。
3. 新增 `expand_render_end_to_cover_visual_range`，按视觉坐标动态扩展 `render_end`，直到 `wrap.visual_offset_of(render_end)` 覆盖 `scroll_offset + content_height`，或已经到 EOF。
4. 保留表格续行前推到表格首行的逻辑，保证块级渲染从正确的逻辑行触发。
5. 增加 `render_tail_content_uses_final_scroll_offset` 回归测试，验证 EOF 光标触发的首帧渲染会把最后一行纳入渲染缓存。

### 教训 / 防回归

- 渲染窗口必须以**最终视口状态**为准。只要 render 内会改 `scroll_offset`，就不能先构建可见内容再同步滚动。
- 不要用固定逻辑行数量猜测视觉范围覆盖。折行、表格、代码块都会让“逻辑行数量”和“屏幕行数量”失去稳定比例。
- 判断“是否覆盖视口”要回到视觉坐标：目标是覆盖 `[scroll_offset, scroll_offset + content_height)`，而不是覆盖“多几行源码”。

---

## 4. Markdown 编辑器：渲染态每个块尾部内容被裁掉，源码态正常

**涉及文件**

- `src/tui/editor_core/renderer.rs`：`compute_line_visible_widths()`、`block_prefix_source_widths()`
- `src/tui/editor_core/editor/render.rs`：block cache 与 wrap cache 的更新顺序

### 现象

Markdown 渲染态下，heading / list / blockquote 等块级内容在行尾或块尾会少一截；切回源码态显示正常。

### 根因

渲染态和源码态的字符宽度口径混用了：

1. `compute_visible_widths()` 通过 `pulldown-cmark` 计算 inline 标记的可见宽度，会把 `**bold**`、链接 URL 等 inline 标记记为 0 宽。
2. 但 `pulldown-cmark` 同时也会把 `# `、`> `、`- `、`1. ` 等块级前缀作为 Markdown block syntax 吞掉。
3. `wrap_engine` 因此低估了块级行的源码占宽；renderer 实际渲染时又会绘制 heading/list/blockquote 的块级前缀或按源码片段渲染。
4. 最终 wrap 认为这一行还能容纳更多尾部字符，但 TUI 实际绘制宽度已经超出可用区域，终端从右侧裁剪，表现为“每个渲染块尾部少内容”。

另外，`render()` 里旧顺序是先 `rebuild_wrap_cache()` 再更新 renderer 的 `block_cache`。如果块级缓存过期，wrap 高度 / 宽度判断也可能基于旧块信息计算。

### 修复

1. `render()` 中先调用 `renderer.ensure_cache_valid()`，再根据当前 block cache 重建 wrap cache。
2. 新增 `block_prefix_source_widths()`：对 heading/list/blockquote 这类块级前缀行，不再使用 `pulldown-cmark` 的 inline 宽度图，而是按源码字符宽度保留前缀宽度。
3. 普通段落仍继续使用 `inline_width::compute_visible_widths()`，保证 `**bold**`、inline code、link URL 等 inline 标记仍按渲染态隐藏宽度处理。
4. 增加 `block_prefix_widths_keep_source_prefix_visible` 和 `regular_inline_widths_still_hide_inline_markers` 测试，防止块级前缀被再次误当成 0 宽，同时保证普通 inline 标记逻辑不回退。

### 教训 / 防回归

- 宽度计算必须区分 block syntax 和 inline syntax。不能直接把 parser 的“可见文本”结果无条件当成 editor 的折行宽度图。
- 源码坐标、渲染坐标、终端 cell 宽度三者必须有明确边界；一旦混用，就会出现源码态正常、渲染态尾部裁剪的 bug。
- wrap cache 依赖 block cache 时，必须先更新 block cache，再重建 wrap cache。

---

## 5. Markdown 编辑器：宽度小折行吞字符（block prefix 行续行少字符）

**涉及文件**

- `src/tui/editor_core/renderer.rs`：`block_prefix_source_widths()`、`compute_line_visible_widths()`、`count_inline_render_chars()`
- `src/tui/editor_core/wrap_engine.rs`：`wrap_line_inner()` 的 `visible_pos` 推进逻辑、`VisualLine.visible_start_char / visible_end_char`
- `src/tui/editor_core/renderer.rs`：`render_non_insert_line()` 折行路径 `extract_span_range(&full_line_spans, vl.visible_start_char, vl.visible_end_char)`

### 现象

在窄终端宽度下，含 Markdown 块级前缀（`- ` / `# ` / `> ` / `1. `）的长段落被折行时，续行开头会"吞"掉几个字符——少显示渲染产物开头的 N 个字符，N 等于前缀在源码中的字符数。

未折行的行、纯文本行、代码块、表格行不复现。普通段落（`**bold**` 等 inline 标记）也不复现。

### 触发条件

同时满足：

1. 非光标行（按渲染后宽度折行路径）。
2. 该行是 block prefix 行（heading / list / blockquote / ordered list）。
3. 该行被 `wrap_engine` 折成 ≥ 2 段视觉行。
4. 终端宽度足够小，让前缀 + 部分正文超过 wrap_width 触发折行。

### 根因

`block_prefix_source_widths`（BUG #4 修复引入）给 heading/list/blockquote 前缀字符（`-`、`#`、`>`、空格）返回 `char_width(ch)`（>0），让 wrap_engine 把这些前缀字符当作"渲染产物中的 char"推进了 `visible_pos`。

但 `render_inline(line_content)` 走整行 inline 解析时，pulldown-cmark 把 `- `/`# `/`> ` 等 block syntax **当作 block 事件消费掉了**，渲染产物（`parse_inline_text` 输出的 `Vec<Inline>`）中**不含前缀字符**。

结果：

- `vl.visible_end_char` 比渲染产物实际长度多算前缀字符数（如 `- ` 多算 2，`### ` 多算 4）。
- 续行 `extract_span_range(spans, vl.visible_start_char, vl.visible_end_char)` 切片时，`visible_start_char` 比渲染产物真实起点偏后 N 个字符 → 续行开头少显示 N 个字符 → "吞字符"。

这是 BUG #2 和 BUG #4 修复的**冲突点**：

- BUG #2 修复要求折行行统一走"整行 inline 渲染 + 按 visible char 索引切片"，保证 vl 之间 char 序列连续。
- BUG #4 修复要求 block prefix 行的前缀字符按 `char_width` 算（保留宽度），避免终端裁剪。
- 两者在 block prefix 行折行场景下冲突：前缀字符的 width > 0（BUG #4）让 `visible_pos` 推进，但渲染产物中不含前缀（BUG #2 路径）→ 坐标系错位。

BUG #2 修复文档里"前缀字符以源码形式 fall through 到 inline"的说法**不准确**——pulldown-cmark 实际消费了前缀，没有 fall through。

### 修复

让 wrap_engine 知道每行的"前缀字符数"（`prefix_chars`），推进 `visible_pos` 时**跳过前缀字符**：

1. `block_prefix_source_widths` 改返回 `Option<(Vec<u8>, usize)>`：`(per_char_widths, prefix_chars)`。
   - 前缀字符的 width 仍为 `char_width`（保留 BUG #4 修复，折行宽度判断正确）。
   - `prefix_chars` = 源码字符数 - 渲染产物字符数（通过 `count_inline_render_chars` 调 `parse_inline_text` 计算）。
2. `compute_line_visible_widths` 改返回 `Vec<Option<(Vec<u8>, usize)>>`，普通段落返回 `(widths, 0)`。
3. `WrapEngine::line_visible_widths` 字段类型改为 `Vec<Option<(Vec<u8>, usize)>>`。
4. `wrap_line_inner` 循环中：`if ch_width > 0 && idx >= prefix_chars { visible_pos += 1; }`——前缀字符不推进 `visible_pos`，但仍累加 width 到 `current_width`。

这样 `visible_start_char` / `visible_end_char` 严格对应渲染产物坐标，续行 `extract_span_range` 切片不再错位。

### 教训 / 防回归

- **`visible_pos` 是"渲染产物 char 序列"的坐标，不是"源码 char 序列"的坐标**。任何在渲染产物中不存在的源码字符（block prefix、inline 标记符号）都不应推进 `visible_pos`。BUG #4 修复让前缀字符的 width > 0（保留折行宽度），但没同步让 `visible_pos` 跳过它们——这就是 bug 的根因。
- **折行宽度（`current_width`）和渲染 char 位置（`visible_pos`）是两个独立的累加器**，不能用同一个条件（`ch_width > 0`）同时驱动。前缀字符需要：累加 width（保留宽度）+ 不推进 visible_pos（渲染产物中不存在）。这两个语义必须分离。
- BUG #2 的"统一走整行解析 + 切片"修复**只对 inline 标记成立**（`**`/`` ` ``/`[]()` 在渲染产物中不存在 → width=0 → 不推进 visible_pos，自动对齐）。block prefix 字符的 width > 0（BUG #4 要求），需要额外机制（`prefix_chars`）让 `visible_pos` 跳过。
- pulldown-cmark 对 `- xxx` / `# xxx` / `> xxx` 等 block prefix 行，**在 inline-only 解析（`parse_inline_text`）中也会消费前缀**（发出 List/Item/Heading/BlockQuote 事件，但 inline 产物不含前缀字符）。这与"block prefix 在源码中可见、在 inline 渲染产物中不可见"的直觉一致，但与 BUG #2 文档里"前缀以源码形式 fall through 到 inline"的描述矛盾——已修正该描述。
- 这类 bug 单元测试难发现：单测 `block_prefix_source_widths` 会说"widths 之和 = display_width（正确）"，单测 `wrap_engine` 会说"visible 区间首尾相接（在源码坐标系下）"。只有**对比 visible_end_char 与渲染产物实际长度**才能发现错位。改这块代码时必须用"渲染产物坐标"作为参照系验证。
