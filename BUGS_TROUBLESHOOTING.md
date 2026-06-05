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
