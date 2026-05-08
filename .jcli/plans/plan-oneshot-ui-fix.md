# oneshot UI 问题修复计划

## 问题分析

### 1. Ctrl+C 无法打断 Tool Call 执行
**根本原因**: 在 `agent_loop.rs` 中，Ctrl+C handler 只设置了 `cancel_token.cancel()`（第 256-259 行），但工具执行用的是独立的 `cancelled: Arc<AtomicBool>` 标志（第 282 行创建，第 363 行传给 `handle_tool_call`）。这个标志从未被 Ctrl+C 触发。

**影响**: 用户在工具执行期间按 Ctrl+C 无效，只能等待工具执行完成或超时。

### 2. 确认框边框不闭合
**根本原因**: `confirm.rs` 中边框宽度计算不一致：
- `draw_top_border`: 用 `bw.saturating_sub(2)` 计算 inner_w，再减 `title_text.chars().count() + 2`
- `draw_bottom_border`: 用 `bw.saturating_sub(3)` 作为 dash 宽度

两者计算逻辑不同，导致顶底边框不对齐。

### 3. Sprite 文字回复没有缩进
**根本原因**: `agent_loop.rs` 第 313 行 `print!("{}", delta)` 直接输出文本，没有像 TUI 模式那样给 AI 回复加上缩进。

### 4. 整体布局间距过窄，缺乏层次感
**问题点**:
- 工具调用轮次标题前后缺少空行分隔
- AI 标签 "Sprite" 后的文字紧贴标签
- 各元素之间没有适当的视觉分隔

---

## 修复方案

### 修复 1: Ctrl+C 打断工具执行

**文件**: `src/command/chat/oneshot/agent_loop.rs`

**改动**:
1. 将 `cancelled` 标志 clone 一份给 Ctrl+C handler
2. 在 handler 中同时触发 `cancel_token.cancel()` 和 `cancelled.store(true, Ordering::Relaxed)`

```rust
// 第 282 行附近
let cancelled = Arc::new(AtomicBool::new(false));
let cancelled_for_ctrlc = Arc::clone(&cancelled);

// Ctrl+C handler（第 256-259 行）
let cancel_for_ctrlc = cancel_token.clone();
let _ = ctrlc::set_handler(move || {
    cancel_for_ctrlc.cancel();
    cancelled_for_ctrlc.store(true, Ordering::Relaxed);
});
```

### 修复 2: 边框宽度统一

**文件**: `src/command/chat/oneshot/confirm.rs`

**改动**:
1. 统一顶底边框宽度计算逻辑
2. 确保总宽度等于 `bw`

```rust
// draw_top_border: 边框总宽度 = 1(┌) + dash_left + title + dash_right + 1(┐)
// 其中 dash_left 至少 1 个，dash_right 填满剩余
pub(crate) fn draw_top_border(stdout: &mut io::Stdout, bw: usize, title: &str) -> io::Result<()> {
    let title_len = title.chars().count();
    // 边框总宽度 = bw，去掉两个角字符后剩余 bw-2
    let remaining = bw.saturating_sub(2);
    // title 前后各至少一个 dash
    let dash_before = 1;
    let dash_after = remaining.saturating_sub(title_len + dash_before);
    let dash_tail = "─".repeat(dash_after);
    
    writeln!(
        stdout,
        "  {}{}{}{}{}\r",
        "┌".yellow().bold(),
        "─".repeat(dash_before).yellow(),
        title.white().bold(),
        "─".yellow(),
        format!("{}{}", dash_tail, "┐").yellow().bold(),
    )
}

// draw_bottom_border: 宽度同样为 bw
pub(crate) fn draw_bottom_border(stdout: &mut io::Stdout, bw: usize) -> io::Result<()> {
    writeln!(
        stdout,
        "  {}{}{}\r",
        "└".yellow().bold(),
        "─".repeat(bw.saturating_sub(2)).yellow(),
        "┘".yellow().bold(),
    )?;
    stdout.flush()
}
```

### 修复 3 & 4: 文本缩进和间距优化

**文件**: `src/command/chat/oneshot/agent_loop.rs`

**改动**:
1. AI 回复文本增加缩进（2 空格）
2. 工具调用轮次前后增加空行分隔
3. 调整各元素的间距

```rust
// 第 302-326 行，Chunk 处理
// 首次输出时，Sprite 标签和内容之间加空行
if first_content {
    let theme = Theme::terminal();
    eprintln!(
        "  {}",
        crate::util::color_adapt::apply_fg("Sprite", theme.label_ai).bold()
    );
    eprintln!();  // 新增空行分隔
    first_content = false;
}

// 流式文本输出时加缩进
let delta = &content[last_streaming_len..];
print!("  {}", delta);  // 增加缩进

// 第 350-366 行，ToolCallRequest 处理
round += 1;

// 轮次标题前后加空行
eprintln!();  // 新增前置空行
eprintln!("  {} R{} · {} 工具", "⚙".dimmed(), round, items.len());
eprintln!();  // 新增后置空行

// 逐个确认 + 执行
for item in items.iter() {
    ...
}

// 第 392-394 行，Done 处理
if round > 0 {
    eprintln!();  // 保留后置空行
}
eprintln!("{} {}", "会话 ID:".dimmed(), session_id.dimmed());
```

---

## 修改文件清单

| 文件 | 改动类型 | 改动点 |
|------|----------|--------|
| `src/command/chat/oneshot/agent_loop.rs` | Bug Fix + Style | 1. Ctrl+C handler 增加 cancelled 触发<br>2. 流式文本缩进<br>3. 间距优化 |
| `src/command/chat/oneshot/confirm.rs` | Bug Fix | 边框宽度计算统一 |

---

## 验证方法

1. **Ctrl+C 打断测试**: 启动 oneshot 模式，触发一个长时间运行的工具（如 `Bash sleep 10`），按 Ctrl+C 应立即中断并显示 "已中断" 消息

2. **边框闭合测试**: 触发工具确认弹窗，目测顶底边框是否对齐闭合

3. **间距层次测试**: 运行一个多轮工具调用的对话，观察：
   - Sprite 标签后是否有空行
   - 回复文本是否有缩进
   - 工具轮次标题前后是否有分隔空行

---

## 注意事项

1. 所有改动遵循项目规范：`cargo fmt` + `cargo clippy -- -D warnings`
2. 避免在 TUI 模式下使用 `println!/eprintln!`，但 oneshot 模式是纯终端输出，可以使用
3. 边框计算需考虑 Unicode 字符宽度（使用 `.chars().count()` 而非 `.len()`）