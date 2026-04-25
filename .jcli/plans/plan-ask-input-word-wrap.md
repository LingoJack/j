# Ask 自由输入区、权限确认框、Plan 审批框 文字折行支持

## 折行边界说明

折行以**审核框边框内的内容宽度**为边界（即 `content_w = bubble_max_width - 6`，6 为左右边框 `│` + padding 的列开销）。对于带前缀（如 `" ❯ ✏ "`）的行，折行可用宽度 = `content_w - prefix_display_width`。每折一行都通过 `bordered_line` 渲染，保证边框完整。

## 问题分析

在 `render/cache.rs` 中有三个区域的文本内容没有支持自动折行换行，当内容超出一行时会被 `bordered_line` 截断丢失：

### 1. Ask 自由输入区（第 1101~1138 行）
当 `tool_interact_typing == true` 时，用户输入的文本 `before + cursor + after` 作为整行一次性传给 `bordered_line`，不做折行处理。`bordered_line` 会将溢出内容截断丢弃。

**修复方案**：将输入文本按可用宽度折行，光标所在行高亮光标字符，其余行为普通文本续行。具体逻辑：
- 计算前缀 `" ❯ ✏ "` 占据的宽度
- 计算续行缩进宽度（与前缀对齐）
- 将 `input` 全文本按可用宽度折行
- 定位光标所在折行及其行内偏移
- 光标行渲染 `before` + 光标字符（块状样式）+ `after`
- 光标前/后的续行只渲染普通文本

### 2. 权限确认框标题行（第 1380~1392 行）
`render_agent_perm_confirm_area` 中标题行 `req.title()` 直接传给 `bordered_line`，如果标题过长会被截断。

**修复方案**：对 `title` 使用 `wrap_text` 折行后逐行渲染。

### 3. Plan 审批框标题行（第 1473~1485 行）
`render_plan_approval_confirm_area` 中标题行 `format!(" Plan 审批请求 [{}] ", req.agent_name)` 直接传给 `bordered_line`，如果 agent_name 过长会被截断。

**修复方案**：对标题使用 `wrap_text` 折行后逐行渲染。

### 4. 工具确认模式自由输入（第 1308~1325 行）
`render_tool_confirm_content` 中 `app.ui.tool_interact_input` 直接拼成单行传给 `bordered_line`，也会被截断。

**修复方案**：与 Ask 自由输入类似，对输入文本折行后渲染。

## 修改文件

- `src/command/chat/render/cache.rs`

## 具体修改点

### 修改点 A：Ask 自由输入折行（第 1101~1138 行）
将当前单行渲染改为多行折行渲染：
1. 计算前缀宽度（`" ❯ ✏ "` 的 display_width）和续行缩进
2. 将输入文本按 `content_w - prefix_w - 2` 的可用宽度调用 `wrap_text`
3. 定位光标所在折行
4. 光标行渲染块状光标，续行渲染普通文本
5. 每行都用 `bordered_line` 包裹

### 修改点 B：权限确认框标题折行（第 1380~1392 行）
将标题用 `wrap_text(content_w)` 折行后逐行渲染（参考 Ask header 的处理方式）。

### 修改点 C：Plan 审批框标题折行（第 1473~1485 行）
将标题用 `wrap_text(content_w)` 折行后逐行渲染。

### 修改点 D：工具确认自由输入折行（第 1308~1325 行）
对 `tool_interact_input` 做折行渲染。
