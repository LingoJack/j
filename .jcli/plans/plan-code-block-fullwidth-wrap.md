# Plan: Code Block Fullwidth + Wrap

## 目标

代码块渲染统一改为：
1. **上下加空行** — 代码块前后各一个空行，增加呼吸感
2. **到屏幕边缘闭合** — 围栏宽度撑满可用宽度（content_width），而非按代码内容最大宽度
3. **内容自动折行** — 超出可用宽度的代码行自动折行，而非截断

## 涉及的渲染器

| 渲染器 | 文件 | 用途 |
|--------|------|------|
| IR 渲染器 | `src/markdown/render/code_block.rs` | Help 页面、Chat 消息预览 |
| Editor 渲染器 | `src/tui/editor_core/renderer/code_block.rs` | Notebook/Markdown 编辑器 |

## 方案

### IR 渲染器 (`markdown/render/code_block.rs`)

**现状**：围栏宽度 = max_content_width + padding，不传 content_width
**改为**：围栏宽度 = content_width（撑满），代码行按 content_width - 4（左右各1 padding + 左右边框）折行

具体修改：
- `render_code_block()` 接收 `_content_width` 参数（目前忽略），改为使用它作为围栏总宽度
- 去掉 `max_content_width` 计算
- 代码内容宽度 = content_width - 4（│ + 空格 + 内容 + 空格 + │）
- 用 `wrap_text()` 对代码行折行
- 前后各插入一个空行

### Editor 渲染器 (`editor_core/renderer/code_block.rs`)

**现状**：围栏宽度 = max_content_width + padding，按逻辑行渲染（不折行）
**改为**：围栏宽度 = 编辑器可视宽度（传入），代码行按可视宽度折行

具体修改：
- `render_code_fence_line()`: 围栏宽度改为 wrap_width（已有的参数）
- `render_code_block_line()`: 长行内容自动折行（已有 visual line 折行机制，续行保持 │ 边框）
- `calculate_code_block_max_width()`: 不再需要，删除
- 围栏行前后各一行空行（由上层调用处处理，或由围栏渲染自身处理）

### 补充说明

- Editor 的折行已有 `wrap_width` 参数和 visual line 机制。代码块内容行的续行已经在 `renderer.rs` 的 is_continuation 分支中处理（保持 │ 边框），只需确保围栏宽度用 wrap_width 即可。
- IR 渲染器需要手动实现折行（用 `wrap_text()`）。
- 空行在代码块前后各一个，不加边框，仅占一行空白。

## 实施步骤

1. 修改 IR `render_code_block()`：撑满 content_width + 折行 + 前后空行
2. 修改 Editor `render_code_fence_line()`：围栏撑满 wrap_width
3. 删除 Editor `calculate_code_block_max_width()`
4. 修改 Editor `render_code_block_line()` 及续行逻辑：使用 wrap_width
5. cargo check + clippy
