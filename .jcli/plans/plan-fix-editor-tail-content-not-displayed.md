# 计划：修复 Markdown 编辑器尾部内容显示不全

## 问题判断

根据已读代码，问题集中在 `src/tui/editor_core/editor/render.rs` 的渲染窗口计算：

1. `wrap_engine.visual_line_count()` 代表全局视觉行总数。
2. `render()` 先根据 `scroll_offset` / `content_height` 推导 `start_logical`、`end_logical`，然后只渲染 `render_start..render_end` 这段逻辑行。
3. 当前 `render_end` 写死为：
   ```rust
   let render_end = (end_logical + 3).min(line_count).max(cursor_row + 1);
   ```
   这里的 `+ 3` 是经验缓冲。对于以下情况可能不足：
   - 视口底部附近存在折行很多的长段落；
   - 表格 / 代码块等块级渲染高度与逻辑行跨度不线性对应；
   - `scroll_offset` 接近 EOF，但 `render_start` 前推后，`all_visual_lines` 覆盖不到 `scroll_offset + content_height` 所需的全部局部行。
4. 之后 `visible_end_local = min(visible_start_local + content_height, all_visual_lines.len())`，如果 `all_visual_lines` 本身没收集够，就会提前结束，剩余行被 `~` 填充，表现为“尾部内容显示不全”。
5. EOF 兜底目前只在 `render_end >= line_count && all_visual_lines.len() > content_height` 时把窗口底锚定到 `all_visual_lines` 末尾；但它不能修复“渲染范围没覆盖到当前 scroll_offset 对应的全局视觉行”这一类问题，而且 `render_meta.rendered_offset` 仍是 `visual_offset`，若窗口被底锚修正，鼠标映射也依赖 `map_index` 配合。

## 修复思路

核心原则：不要依赖固定 `+ 3` 的逻辑行缓冲；渲染范围应覆盖目标可见的全局视觉行区间。

### 1. 在 render.rs 中改造渲染范围扩展

在计算 `render_start` 后，基于目标可见视觉区间动态推进 `render_end`：

- 目标视觉结束行：`target_visual_end = scroll_offset + content_height`（半开区间右端）。
- 初始 `render_end` 可保留当前估算，但之后循环检查：
  - 当前 `render_end` 对应的全局视觉偏移：`self.wrap.visual_offset_of(render_end)`；
  - 只要它还小于 `target_visual_end`，说明 `render_start..render_end` 不足以覆盖视口底部，就继续向后扩展；
  - 每次可按小步或指数步扩展，并限制到 `line_count`，避免死循环。

建议提取私有辅助函数，避免 `render()` 变得更复杂，例如：

```rust
fn expand_render_end_to_cover_visual_range(
    &self,
    mut render_end: usize,
    target_visual_end: usize,
    line_count: usize,
) -> usize
```

或在 `render.rs` 内部局部实现简单循环。

### 2. 保留表格首行前推逻辑

`render_start` 落在表格续行时前推到表格首行的逻辑是必要的，应保留：

```rust
if let Some((tbl_start, _)) = self.wrap.table_block_for_line(render_start) {
    render_start = render_start.min(tbl_start);
}
```

同时应在 `render_start` 最终确定后再计算 `visual_offset`。

### 3. 调整 EOF 兜底条件（如仍需要）

动态扩展 `render_end` 后，EOF 兜底应只作为最后保险。可以保留现有逻辑，但重点验证：

- 当 `all_visual_lines.len() == content_height` 时无需底锚；
- 当 `all_visual_lines.len() < content_height` 时本来就只能显示全部内容并填充 `~`；
- 当 `all_visual_lines.len() > content_height` 且已到 EOF 时底锚仍合理。

如果动态扩展后该兜底不再触发普通尾部缺行场景，可保留注释说明。

### 4. 增加回归测试

在 `src/tui/editor_core/editor.rs` 的测试模块中补充一个不依赖真实终端的测试，验证渲染窗口覆盖逻辑。可使用 `ratatui::backend::TestBackend` / `Terminal` 进行一次 render，然后检查 buffer 内容。

建议至少覆盖：

1. 长文档滚动到尾部，最后几行都能进入渲染 buffer，而不是被 `~` 提前替代。
2. 末尾包含长折行段落时，滚动到靠近 EOF 仍能显示完整尾部视觉行。

如果项目已有 ratatui 测试 backend 使用案例，优先沿用现有风格；否则写最小测试。

### 5. 更新 BUGS_TROUBLESHOOTING.md

新增第 3 条，按现有格式记录：

- 现象：编辑器尾部内容显示不全，底部提前出现 `~`。
- 触发条件：文档较长 / 末尾有多视觉行内容 / 滚动接近 EOF。
- 根因：`render_end = end_logical + 3` 固定缓冲无法保证渲染范围覆盖 `scroll_offset + content_height`。
- 修复：动态扩展 `render_end` 直到覆盖目标视觉行区间或到 EOF。
- 教训：渲染窗口要按视觉坐标闭包校验，不要用固定逻辑行数量猜测。

## 验证

实施后执行：

1. `cargo fmt`
2. `cargo test editor_core` 或相关精确测试
3. `cargo clippy -- -D warnings`（若耗时较长也应运行，符合项目要求）

## 预期修改文件

- `src/tui/editor_core/editor/render.rs`
- `src/tui/editor_core/editor.rs`（仅测试，如测试更适合放这里）
- `BUGS_TROUBLESHOOTING.md`
