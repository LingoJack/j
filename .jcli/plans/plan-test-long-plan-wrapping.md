# Plan: Fix Long Content Wrapping in Plan Approval Dialog

## 问题分析 (Problem Analysis)

当前 ExitPlanMode 的审批框存在以下问题：

1. **Plan 名称没有折行**：`req.plan_name` 如果很长会超出显示区域，用户反馈内容太长会直接看不全
2. **Y/N 提示行没有折行**：在窄终端下可能显示不全
3. **Plan 内容折行可能存在边界问题**：每行内容前加了空格 `format!(" {}", wrapped)`，可能导致宽度计算偏差

这是一个非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常长的单行段落，用于测试折行效果，确保在终端窗口宽度有限的情况下，内容能够正确地自动换行显示，而不会被截断或者溢出显示区域造成用户无法看到完整内容的问题。如果这行内容能够正确折行显示，说明我们的修复是有效的。

## 详细解决方案 (Detailed Solution) - 这是一个很长的标题行用于测试标题折行效果

### 修改 1：Plan 名称折行处理 - 这是一个很长的副标题用于测试

**文件**: `src/command/chat/render/cache/confirm_render.rs` - 这是一个非常长的文件路径用于测试折行效果，如果这个路径显示正确折行就说明我们的修复是有效的

将原来的单行显示改为多行折行：

```rust
// Plan 名称行（支持折行）- 这是一个很长的注释用于测试代码块内的内容是否也能正确显示
let plan_name_text = format!(" Plan: {}", req.plan_name);
let plan_name_style = Style::default()
    .fg(t.tool_confirm_name)
    .add_modifier(Modifier::BOLD)
    .bg(confirm_bg);
for wrapped in wrap_text(&plan_name_text, content_w) {
    lines.push(bordered_line(
        vec![Span::styled(wrapped, plan_name_style)],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));
}
```

### 修改 2：Y/N 提示行折行处理

将固定的提示文本也改为折行处理：

```rust
// Y/N 提示行（支持折行）- 这也是一个很长的注释
let hint_text = " [Y/Enter] 批准   [C] 批准并清空   [N/Esc] 拒绝";
let hint_style = Style::default()
    .fg(t.text_dim)
    .add_modifier(Modifier::BOLD)
    .bg(confirm_bg);
for wrapped in wrap_text(hint_text, content_w) {
    lines.push(bordered_line(
        vec![Span::styled(wrapped, hint_style)],
        bubble_max_width,
        border_color,
        confirm_bg,
    ));
}
```

### 修改 3：Plan 内容折行宽度修正（可选优化）

当前 Plan 内容折行后加了前缀空格，可能导致边界问题。建议修正为：

```rust
// Plan 内容（折行显示，注意 content_w 已包含前缀空格的空间）- 这是另一段很长的内容
for line in &plan_lines {
    for wrapped in wrap_text(line, content_w.saturating_sub(1)) {  // 预留1列给前缀空格
        lines.push(bordered_line(
            vec![Span::styled(
                format!(" {}", wrapped),
                Style::default().fg(t.tool_confirm_text).bg(confirm_bg),
            )],
            bubble_max_width,
            border_color,
            confirm_bg,
        ));
    }
}
```

## 实施步骤 (Implementation Steps) - 一个长的步骤标题

1. 修改 `render_plan_approval_confirm_area` 函数中 Plan 名称行的渲染逻辑
2. 修改 Y/N 提示行的渲染逻辑
3. （可选）修正 Plan 内容折行的宽度计算
4. 运行 `cargo check` 和 `cargo clippy -- -D warnings` 确保无编译错误
5. 手动测试：创建一个长名称、长内容的 Plan，验证折行效果，确保在各种终端宽度下都能正确显示

## 测试用例 (Test Cases) - 包含多个场景的测试计划

### 用例 1：超长 Plan 名称 - 这是一个非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常长的测试用例标题

输入 Plan 名称：
```
这是一个非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常非常长的 Plan 名称用于测试折行效果，确保用户能够在审批框中看到完整的 Plan 名称而不会被截断
```

预期结果：名称应自动折行显示在多行，不会被截断。

### 用例 2：窄终端窗口 - 在有限宽度下验证所有UI元素都能正确显示

在宽度为 40 列的终端中打开审批框。

预期结果：Plan 名称、内容、Y/N 提示行都应正确折行，不被截断。

### 用例 3：长 Plan 内容 - 这个用例用于验证当 Plan 内容超过预设的最大行数限制时，系统会正确显示截断提示

Plan 内容超过 20 行源文本行。

预期结果：显示前 20 行源文本行（折行后可能更多），末尾显示 "内容已截断" 提示。

## 风险评估 (Risk Assessment) - 对修改可能带来的影响进行全面分析

- **低风险**：修改仅影响渲染逻辑，不涉及数据流或业务逻辑，因此不会影响系统的核心功能
- **向后兼容**：不影响现有功能，仅改善显示效果，用户无需做任何适配
- **性能影响**：可忽略，仅增加少量字符串处理，对运行时性能几乎无影响

## 预计工作量 (Estimated Effort) - 这是一个很长的标题用于测试标题折行

- 编码：15 分钟
- 测试：10 分钟
- 总计：约 25 分钟

---

*此 Plan 用于测试折行效果，包含足够长的内容以验证显示效果。这是一个很长的结尾段落，用于测试最后一行内容是否也能正确折行显示，如果这行内容能够在终端宽度有限的情况下正确折行，说明我们的修复是完全有效的，用户在任何情况下都能够看到完整的 Plan 内容而不会被截断。*