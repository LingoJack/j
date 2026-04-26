# 拆分 cache.rs (2981 行) 方案

## 背景

`cache.rs` 是项目最大的源文件（2981 行），包含增量缓存引擎、气泡布局、消息渲染、确认区域渲染、工具调用/结果渲染、动画效果等多种职责。

## 当前模块结构

```
src/command/chat/render.rs      → pub mod cache; pub mod helpers; pub mod theme;
src/command/chat/render/cache.rs → 2981 行（单文件）
```

## 外部引用（re-export 路径）

```rust
// chat.rs
pub use render::cache as render_cache;

// ui/chat.rs
use super::super::render_cache::build_message_lines_incremental;

// chat_app.rs (两处)
use crate::command::chat::render::cache::copy_to_clipboard;
```

所有外部引用只涉及 3 个公开符号：`build_message_lines_incremental`、`copy_to_clipboard`、以及子模块路径 `render::cache`。**拆分后不需要改外部引用。**

## 拆分方案

将 `src/command/chat/render/cache.rs` → `src/command/chat/render/cache/` 目录模块。

### 文件划分

| 新文件 | 包含函数 | 预估行数 |
|--------|---------|---------|
| `cache/mod.rs` | 常量 + `RenderContext` + `ContentContext` + `find_stable_boundary` + `build_message_lines_incremental` + re-export pub use | ~500 |
| `cache/bubble.rs` | `wrap_md_line_in_bubble`, `wrap_md_line_in_bubble_with_margin`, `bordered_line` | ~100 |
| `cache/msg_render.rs` | `render_user_msg`, `render_assistant_msg`, `render_thinking_block`, `parse_agent_prefix`, `agent_name_color` | ~250 |
| `cache/confirm_render.rs` | `render_tool_confirm_area`, `render_ask_questions`, `render_tool_confirm_content`, `render_agent_perm_confirm_area`, `render_plan_approval_confirm_area` | ~850 |
| `cache/tool_call_render.rs` | `render_tool_call_request_msg`, `render_json_params_enhanced`, `extract_tool_description_from_args`, `TeammateCallArgs` + `extract_teammate_args` + `render_teammate_call_request_expanded`, `BashArgs` + `extract_bash_args` + `render_bash_call_request_expanded`, `AgentCallArgs` + `extract_agent_args` + `render_agent_call_request_expanded`, `render_exit_plan_mode_request`, `render_bash_call_request_expanded` | ~600 |
| `cache/tool_result_render.rs` | `render_tool_result_msg`, `render_diff_content`, `render_agent_result_nested`, `render_bash_result`, `render_todo_result`, `parse_tool_label` | ~460 |
| `cache/animation.rs` | `current_tick`, `thinking_pulse_color`, `comet_gradient_line` | ~100 |
| `cache/clipboard.rs` | `copy_to_clipboard` | ~40 |

### 可见性设计

- **pub 符号**（`build_message_lines_incremental`, `wrap_md_line_in_bubble`, `render_user_msg`, `render_assistant_msg`, `render_tool_call_request_msg`, `render_tool_result_msg`, `copy_to_clipboard`, `find_stable_boundary`, `RenderContext`, `ContentContext`）保持 `pub`，在 `mod.rs` 中 `pub use` re-export。
- **私有函数**（`bordered_line`, `render_tool_confirm_area` 等）降为 `pub(crate)` 以便子文件间调用，或保持在同文件内。
- 由于 Rust 模块可见性规则，子文件中的私有函数对 `mod.rs` 不可见。因此子文件中需要跨文件调用的函数统一标记为 `pub(crate)`。

### 调用关系

```
mod.rs
  ├── bubble.rs        ← 被 mod.rs, msg_render.rs, tool_call_render.rs, confirm_render.rs 调用
  ├── msg_render.rs    ← 被 mod.rs 调用
  ├── confirm_render.rs ← 被 mod.rs 调用
  ├── tool_call_render.rs ← 被 mod.rs 调用
  ├── tool_result_render.rs ← 被 mod.rs 调用
  ├── animation.rs     ← 被 mod.rs 调用
  └── clipboard.rs     ← 被 mod.rs re-export，外部直接调用
```

### 不变的引用路径

外部代码通过 `crate::command::chat::render::cache::xxx` 或 `super::super::render_cache::xxx` 引用。由于 `cache.rs` → `cache/mod.rs`，路径完全不变。

## 执行步骤

1. 创建 `cache/` 目录
2. 创建 `cache/bubble.rs` — 提取气泡布局函数
3. 创建 `cache/msg_render.rs` — 提取消息渲染函数
4. 创建 `cache/confirm_render.rs` — 提取确认区域渲染函数
5. 创建 `cache/tool_call_render.rs` — 提取工具调用渲染函数
6. 创建 `cache/tool_result_render.rs` — 提取工具结果渲染函数
7. 创建 `cache/animation.rs` — 提取动画函数
8. 创建 `cache/clipboard.rs` — 提取剪贴板函数
9. 将 `cache.rs` 改写为 `cache/mod.rs` — 保留常量/结构体/增量引擎 + re-export
10. 运行 `cargo fmt` + `cargo clippy -- -D warnings` 验证
