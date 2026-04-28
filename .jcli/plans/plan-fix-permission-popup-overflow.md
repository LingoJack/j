# 修复权限弹窗溢出问题

## 问题分析

用户报告 Bash 命令很长时，权限弹窗右侧 `│` 边框被挤出，导致没对齐。

### 根本原因

`render_tool_confirm_content` 函数中有多处手动构建 `Line::from(vec![...])` 的代码：

1. **行 461-486: 工具名行** — 当 `tool_name` 很长时会溢出
2. **行 488-516: 确认信息行** — 当 `confirm_message`（bash 命令）很长时会溢出  
3. **行 663-680: 选项行** — 选项文本很长时可能溢出

这些手动构建的行计算 `fill` 宽度时，如果内容超出 `content_w`，`fill` 会变成 0，导致：
- 右侧填充宽度不够
- 右边框 ` │` 被内容挤出可见区域

### 对比 ask 弹窗

`render_ask_questions` 函数几乎全部使用 `bordered_line()` 来渲染内容行，这个函数内置了**溢出钳制**：
- 逐 span 逐字符截断
- 确保内容不超出目标宽度
- 保证右边框始终对齐

### 已修复的内容

在之前的对话中，已修复：
1. 顶/底边框添加背景色
2. `draw_messages` 滚动逻辑覆盖 `AgentPermConfirm` 和 `PlanApprovalConfirm`

## 修复方案

将 `render_tool_confirm_content` 中所有手动构建的行改为使用 `bordered_line()`：

### 1. 工具名行（行 461-486）

**修改前**：
```rust
let text_content = format!("{}{}", label, name);
let fill = content_w.saturating_sub(display_width(&text_content));
lines.push(Line::from(vec![
    Span::styled("  │ ", ...),
    Span::styled(" ", ...),
    Span::styled(label, ...),
    Span::styled(name.clone(), ...),
    Span::styled(" ".repeat(fill.saturating_sub(1)), ...),
    Span::styled(" │", ...),
]));
```

**修改后**：
```rust
lines.push(bordered_line(
    vec![
        Span::styled(" ", Style::default().bg(confirm_bg)),
        Span::styled(label, Style::default().fg(t.tool_confirm_label).bg(confirm_bg)),
        Span::styled(name.clone(), Style::default().fg(t.tool_confirm_name).bg(confirm_bg).add_modifier(Modifier::BOLD)),
    ],
    bubble_max_width,
    border_color,
    confirm_bg,
));
```

### 2. 确认信息行（行 488-516）

**修改前**：手动构建，计算 fill
**修改后**：使用 `bordered_line()` + 前缀空格

### 3. 选项行（行 663-680）

**修改前**：手动构建，计算 fill
**修改后**：使用 `bordered_line()`

### 4. 保持不变的部分

- 空行和分隔行：固定宽度计算，不会溢出
- 输入模式的折行处理：已使用 `bordered_line()`

## 需要修改的文件

- `src/command/chat/render/cache/confirm_render.rs`：`render_tool_confirm_content` 函数

## 验证

1. 运行 `cargo clippy -- -D warnings` 确保无警告
2. 运行 `cargo fmt -- --check` 确保格式正确
3. 测试长 bash 命令的显示，确认边框对齐
