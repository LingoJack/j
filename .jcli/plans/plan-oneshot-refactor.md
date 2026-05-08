# oneshot.rs 拆分优化方案

## 现状分析

`oneshot.rs` 共 1365 行，包含 oneshot 模式（非 TUI 终端交互）的所有逻辑，职责涵盖：

| 职责 | 行数 | 函数/类型 |
|------|------|-----------|
| 工具参数预览 (UI 辅助) | ~60 行 | `extract_tool_desc`, `extract_bash_command`, `make_args_preview` |
| 终端布局工具 | ~12 行 | `term_width`, `box_width` |
| 思考脉冲颜色 | ~27 行 | `thinking_pulse_color` |
| 工具调用/结果打印 | ~50 行 | `print_tool_call_line`, `print_tool_result_line` |
| 思考动画线程 | ~50 行 | `start_thinking_animation`, `stop_thinking_animation` |
| 会话 ID / 持久化 | ~20 行 | `generate_oneshot_session_id`, `persist_messages` |
| 入口 + 参数 | ~90 行 | `ChatArgs`, `handle_chat` |
| 无工具流式输出 | ~75 行 | `run_oneshot_no_tools` |
| Markdown 重绘 | ~18 行 | `redraw_markdown` |
| 交互式工具确认 UI | ~140 行 | `interactive_confirm` |
| Ask 请求处理线程 | ~310 行 | (内联在 `run_oneshot_agent` 中) |
| 流式文本重绘 | ~30 行 | `redraw_streaming_as_markdown` |
| Agent 主循环 | ~630 行 | `run_oneshot_agent` |
| 工具调用处理 | ~90 行 | `handle_tool_call` |
| 格式化时间 | ~8 行 | `format_duration` |
| Hook 触发 | ~18 行 | `fire_session_end` |

**主要问题**：
1. 单文件 1365 行，远超 200-300 行的合理范围
2. 多个职责混杂：UI 渲染、终端动画、交互确认、Agent 循环、工具执行、会话管理
3. `run_oneshot_agent` 函数体 ~630 行，其中 ~310 行是 Ask 线程的内联 UI 绘制闭包
4. Ask 线程中 `draw_multi` / `draw_single` 有大量重复的边框绘制代码
5. `interactive_confirm` 和 Ask 的边框绘制逻辑高度相似但未抽象

## 拆分方案

按照 AGENTS.md 的"弃用 mod.rs，采用 `name.rs` + `name/` 子目录"规范，将 `oneshot.rs` 拆为目录结构：

```
src/command/chat/oneshot.rs      →  src/command/chat/oneshot/mod.rs   (入口 + handle_chat)
                                   src/command/chat/oneshot/
                                       mod.rs           (入口: ChatArgs, handle_chat, re-exports)
                                       display.rs       (终端打印: print_tool_call_line, print_tool_result_line, redraw_markdown, redraw_streaming_as_markdown, thinking_pulse_color)
                                       animation.rs     (思考动画: start_thinking_animation, stop_thinking_animation)
                                       confirm.rs       (交互确认: interactive_confirm + 通用边框绘制)
                                       ask_ui.rs        (Ask 请求处理线程: 单选/多选 UI，复用 confirm 的边框工具)
                                       agent_loop.rs    (Agent 主循环: run_oneshot_agent, run_oneshot_no_tools)
                                       tool_exec.rs     (工具执行: handle_tool_call, format_duration)
                                       session.rs       (会话管理: generate_oneshot_session_id, persist_messages, fire_session_end)
```

### 各文件职责和内容

#### 1. `oneshot/mod.rs` (~50 行)
- `pub struct ChatArgs`
- `pub fn handle_chat()` — 入口分发
- `mod` 声明 + `pub use` re-exports

#### 2. `oneshot/session.rs` (~30 行)
- `generate_oneshot_session_id()`
- `persist_messages()`
- `fire_session_end()`

#### 3. `oneshot/display.rs` (~120 行)
- 常量: `TOOL_ARG_PREVIEW_MAX_CHARS`
- `extract_tool_desc()`, `extract_bash_command()`, `make_args_preview()`
- `term_width()`, `box_width()`
- `thinking_pulse_color()`
- `print_tool_call_line()`, `print_tool_result_line()`
- `redraw_markdown()`, `redraw_streaming_as_markdown()`

#### 4. `oneshot/animation.rs` (~50 行)
- 常量: `ONESHOT_EXIT_TICK_MS`, `ONESHOT_EXIT_SETTLE_MS`
- `start_thinking_animation()`
- `stop_thinking_animation()`

#### 5. `oneshot/confirm.rs` (~150 行)
- `interactive_confirm()` — 重构为使用通用边框绘制
- 新增 `BoxRenderer` 辅助结构体或一组边框绘制工具函数，消除与 `ask_ui.rs` 的代码重复

#### 6. `oneshot/ask_ui.rs` (~200 行)
- Ask 请求处理线程逻辑（从 `run_oneshot_agent` 中提取）
- 单选/多选 UI 绘制（复用 confirm 的边框工具）
- `spawn_ask_handler(ask_rx)` 函数

#### 7. `oneshot/agent_loop.rs` (~350 行)
- `run_oneshot_agent()` — 主循环（调用 `ask_ui::spawn_ask_handler` 代替内联线程）
- `run_oneshot_no_tools()`
- 常量: `ONESHOT_POLL_MS`

#### 8. `oneshot/tool_exec.rs` (~90 行)
- `handle_tool_call()`
- `format_duration()`

### 边框绘制去重

`interactive_confirm` 和 Ask 的 `draw_multi`/`draw_single` 共享同一套 `┌──┐` 直角边框样式。提取公共的边框绘制函数：

```rust
// confirm.rs 或独立的 border.rs
pub(crate) struct BoxDraw<'a> {
    pub width: usize,
    pub title: Option<&'a str>,
}

impl<'a> BoxDraw<'a> {
    pub fn draw_top_border(&self, stdout: &mut io::Stdout) -> io::Result<()> { ... }
    pub fn draw_content_line(&self, stdout: &mut io::Stdout, content: &str) -> io::Result<()> { ... }
    pub fn draw_empty_line(&self, stdout: &mut io::Stdout) -> io::Result<()> { ... }
    pub fn draw_hint_line(&self, stdout: &mut io::Stdout, hint: &str) -> io::Result<()> { ... }
    pub fn draw_bottom_border(&self, stdout: &mut io::Stdout) -> io::Result<()> { ... }
}
```

### 外部影响

- `chat.rs` 中 `pub use oneshot::ChatArgs; pub use oneshot::handle_chat;` **无需修改**（mod.rs 仍导出这些符号）
- 所有 `use crate::command::chat::oneshot::xxx` 的外部引用路径不变

## 实施步骤

1. 创建 `oneshot/` 目录结构
2. 将各职责拆入对应文件
3. 提取公共边框绘制逻辑
4. 将 `run_oneshot_agent` 中的 Ask 线程提取到 `ask_ui.rs`
5. 更新 `mod.rs` 入口和 re-exports
6. 删除原 `oneshot.rs`
7. `cargo fmt` + `cargo clippy -- -D warnings` 验证
