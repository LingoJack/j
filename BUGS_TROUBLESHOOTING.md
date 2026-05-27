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

未滚动 / 文档很短时不易复现，因此早期容易漏掉。

### 触发条件

`render_meta.rendered_offset > 0`，也就是当帧 `visual_offset > 0`——只要视口往下滚过一些行就会触发。文档不超过一屏时 `rendered_offset == 0`，BUG 被掩盖。

### 根因

`RenderMeta` 里两个字段的语义没对齐，但消费者按"全局行号"约定使用：

| 字段                | 实际语义                              | 旧注释 / 错误理解          |
| ------------------- | ------------------------------------- | -------------------------- |
| `map_index`         | `rendered_lines` / `vl_map` 的局部下标 | 一度被注释为"全局渲染行号" |
| `rendered_offset`   | `rendered_lines[0]` 对应的全局行号    | （正确）                   |

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
