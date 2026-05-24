# 修复 oneshot 模式 markdown 重绘失败问题

## 问题分析

### 现象
`j ai 什么是指数基金`（不带 `--no-render`）时，AI 回复的 markdown 格式（如 `**粗体**`、`# 标题`、列表等）没有被渲染，直接显示了原始 markdown 文本。

### 根因
`run_oneshot_agent`（有工具模式）中，markdown 重绘依赖 `save_cursor_row()` 保存的终端行号。该函数调用 `crossterm::cursor::position()`，其实现方式是向 stdout 发送 DSR（Device Status Report, `ESC[6n`）并等待终端回复光标位置（最多 2 秒超时）。

**问题：`crossterm::cursor::position()` 在以下场景会返回 Err，导致 `content_start_row = None`，整个 markdown 重绘被跳过：**

1. **stdout 不是 TTY**（管道/重定向场景）— DSR 无响应，2 秒超时后返回 Err
2. **某些终端模拟器不正确响应 DSR**（如 VSCode 终端、JetBrains 终端的某些版本）
3. **tmux/screen 会话中** — DSR 响应可能被多路复用层拦截或延迟

关键代码路径 (`run_oneshot_agent`, agent_loop.rs):

```
首次 Chunk 到来 → eprintln!("Sprite") → save_cursor_row() → content_start_row
                                                    ↑
                                          crossterm::cursor::position()
                                          返回 Err → content_start_row = None

StreamMsg::Done → if let Some(row) = content_start_row { redraw... }
                                         ↑
                                   None → 重绘被跳过！
```

### 对比 `run_oneshot_no_tools`
无工具模式使用 `redraw_markdown(raw_lines, cur_col, text)` 基于行数回退，不依赖 DSR。但 `raw_lines` 的计算存在 CJK 字符宽度问题（每个字符 `cur_col += 1`，但 CJK 占 2 列）。

## 修复方案

### 核心思路：不依赖 `cursor::position()` DSR，改用基于行数回退的方法

在 `run_oneshot_agent` 中跟踪流式输出的行数（类似 `run_oneshot_no_tools` 的做法），使用 `redraw_markdown` 基于行数回退重绘。同时修复 CJK 宽度计算问题。

### 具体修改

#### 1. `run_oneshot_agent` (agent_loop.rs)

- **移除 `content_start_row`**：不再使用 `save_cursor_row()` 保存行号
- **新增 `raw_lines` / `cur_col` 跟踪**：在 `StreamMsg::Chunk` 中，当 `!no_render` 时跟踪流式文本的行数和列数（含 CJK 宽度计算）
- **首次 Chunk 不再调用 `save_cursor_row()`**，改为只打印 Sprite 标签
- **`StreamMsg::Done` 和 `StreamMsg::ToolCallRequest` 中**：将 `redraw_streaming_as_markdown(&streaming_content, row)` 替换为 `redraw_markdown(raw_lines, cur_col, &content)`

#### 2. 修复 CJK 宽度计算 (agent_loop.rs)

`run_oneshot_no_tools` 中的 `cur_col += 1` 应改为使用 Unicode 宽度：
```rust
use unicode_width::UnicodeWidthChar;
cur_col += ch.width().unwrap_or(0) as usize;
```

同时 `run_oneshot_agent` 中的行数跟踪也要使用正确的 CJK 宽度。

#### 3. display.rs 清理

- `save_cursor_row()` 和 `redraw_streaming_as_markdown()` 可以标记为 dead code 或移除
- 保留 `redraw_markdown` 和 `redraw_markdown_from_saved`（后者可能还有其他用途）

### 修改文件清单

| 文件 | 修改内容 |
|------|---------|
| `src/command/chat/oneshot/agent_loop.rs` | 移除 `content_start_row`，新增 `raw_lines/cur_col` 跟踪，修复 CJK 宽度，改用 `redraw_markdown` |
| `src/command/chat/oneshot/display.rs` | 标记/移除 `save_cursor_row()` 和 `redraw_streaming_as_markdown()` |

### 风险评估

- **低风险**：`redraw_markdown` 已在 `run_oneshot_no_tools` 中验证可用
- **兼容性**：基于行数回退（`MoveUp`）是标准的 ANSI 序列，所有终端都支持
- **不需要 DSR**：避免了 `cursor::position()` 的所有兼容性问题
