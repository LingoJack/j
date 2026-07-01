# 项目长期记忆

## Markdown 编辑器折行引擎（wrap_engine）关键约束

- `visible_pos` 是"渲染产物 char 序列"的坐标，不是"源码 char 序列"的坐标。任何在渲染产物中不存在的源码字符（block prefix、inline 标记符号）都不应推进 `visible_pos`。
- 折行宽度（`current_width`）和渲染 char 位置（`visible_pos`）是两个独立累加器，不能用同一条件（`ch_width > 0`）同时驱动。block prefix 字符需要：累加 width（保留宽度）+ 不推进 visible_pos（渲染产物中不存在）。
- pulldown-cmark 在 inline 解析（`parse_inline_text`）中也会消费 block prefix（`- `/`# `/`> `），渲染产物中不含前缀字符。
- 折行相关 bug 记录在 `BUGS_TROUBLESHOOTING.md`，改 wrap/render 代码前必读 #2/#4/#5。
