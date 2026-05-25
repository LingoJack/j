# 修复编辑器鼠标滚动闪烁/卡死问题

## 问题描述
1. 鼠标滚动到底部后，页面一直闪烁
2. 中途鼠标点击后，滚动不了了

## 根因分析

### 根因 1（核心）：鼠标点击没有解除 `scroll_locked`

**事件链**：
1. 用户滚轮滚动 → `scroll_locked = true`（正确，允许用户浏览远离光标的内容）
2. 用户点击某一行 → 光标被移到该位置，但 `scroll_locked` **仍然是 `true`**
3. 点击使该行变为光标行 → Insert 模式下光标行显示源码，Normal 模式下叠加光标块
4. 光标行的渲染高度可能不同于渲染后行的高度（源码可能只有1行，而渲染后的 Markdown 可能产出多行）
5. `rendered_line_count` 发生变化
6. 下一帧 `render()` 中，因为 `scroll_locked = true`，视口不跟随光标，但 `visible_start` 的计算依赖于 `scroll_offset` 和 `visual_map`
7. 由于渲染行数变化，`visual_map` 的映射也变了，导致 `visible_start` 抖动
8. 每帧渲染出不同的内容 → 闪烁

**为什么闪烁而不只是"不跟随"**：
- 光标行切换为源码模式时，渲染行数减少 → `all_visual_lines` 变短
- `scroll_offset` 基于旧的较长列表计算，现在可能超出 `visual_map` 范围
- fallback 分支 `all_visual_lines.len().saturating_sub(content_height)` 计算出新的 `visible_start`
- 下一帧如果渲染行数又变回来（因为某些状态变化），`visible_start` 又跳回去
- 这就形成了闪烁循环

**对比键盘输入**：`handle_input()` 第一行就是 `self.viewport.scroll_locked = false;`，所以键盘操作后视口会自动跟随光标，没有这个问题。

### 根因 2（次要）：滚动到底部边界的稳定性

`scroll_viewport_down()` 使用 `render_meta.rendered_line_count` 计算 `max_offset`，但这个值是上一帧渲染的结果。当光标行切换导致渲染行数变化时，`max_offset` 可能不准。

## 修复方案

### 修复 1：鼠标点击/拖拽时解除 `scroll_locked`

在 `handle_mouse()` 方法中：
- `Down(Left)`：添加 `self.viewport.scroll_locked = false;`
- `Drag(Left)`：添加 `self.viewport.scroll_locked = false;`

这样点击后视口会自动跟随光标，不会出现光标在视口外导致的渲染抖动。

### 修复 2：渲染时 clamp `scroll_offset`

在 `render()` 方法中，在用 `scroll_offset` 计算 `visible_start` 之前，将其 clamp 到 `[0, all_visual_lines.len().saturating_sub(content_height)]` 范围内，防止超出边界。

## 修改文件

1. `src/tui/editor_core/editor.rs` - `handle_mouse()` 方法：在点击和拖拽时解除 `scroll_locked`
2. `src/tui/editor_core/editor/render.rs` - `render()` 方法：clamp `scroll_offset`
