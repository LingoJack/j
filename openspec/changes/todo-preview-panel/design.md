## Context

### 当前状态

Todo TUI（`src/command/todo.rs`）使用 ratatui 框架渲染全屏待办管理界面。列表项通过 `truncate_to_width()` 函数截断过长内容，确保单行显示一致性。但用户无法在列表中查看完整内容，需要进入编辑模式（按 `e`）才能查看或修改。

### 参考实现

日报编辑器（`src/tui/editor.rs`）已实现预览区功能：
- 使用 `count_wrapped_lines_unicode()` 计算换行后的行数
- 使用 `display_width_of()` 计算字符串显示宽度（unicode_width）
- 预览区支持 `Paragraph::wrap()` 自动换行
- 支持 Alt+↑/↓ 滚动预览区内容
- 动态显示滚动进度 `[X/Y行]`

### 约束

- 预览区不能占用过多空间，保证列表区域仍有足够的可视行数
- 滚动逻辑需要与主列表滚动独立，互不干扰
- 性能要求：预览区渲染不应明显影响 TUI 响应速度

## Goals / Non-Goals

**Goals:**

1. 在 Todo TUI 中新增预览区面板，当选中项内容超出列表显示宽度时自动显示
2. 预览区支持文本换行显示，完整展示待办内容
3. 预览区支持 Alt+↑/↓ 滚动查看长文本
4. 布局动态调整：无长内容时预览区隐藏，有长内容时列表与预览区合理分配空间
5. 预览区标题显示当前项索引、创建时间，帮助用户定位

**Non-Goals:**

1. 不在预览区中直接编辑内容（编辑仍需按 `e` 进入编辑模式）
2. 不支持预览区内搜索或高亮（这是纯展示功能）
3. 不改变现有快捷键行为（Alt+↑/↓ 目前未使用，可安全复用）

## Decisions

### 1. 预览区显示触发条件

**决策**：当选中项内容的显示宽度超过列表可用宽度时，显示预览区。

**实现方式**：
- 列表可用宽度 = 终端宽度 - 行号区宽度 - checkbox 区宽度 - 日期区宽度 - 边框宽度
- 使用 `display_width()` 函数计算内容显示宽度（已存在于 `todo.rs`）
- 如果 `content_width > available_width`，则显示预览区

**替代方案**：
- 始终显示预览区 → 浪费屏幕空间，短内容无意义
- 手动开关预览区 → 增加操作复杂度，不如自动触发直观

### 2. 布局比例

**决策**：预览区显示时，列表区占 55%，预览区占 40%，状态栏和帮助栏保持不变。

**实现方式**：
```rust
let constraints = if needs_preview {
    vec![
        Constraint::Percentage(55),  // 列表区
        Constraint::Min(5),          // 预览区
        Constraint::Length(3),       // 状态栏
        Constraint::Length(2),       // 帮助栏
    ]
} else {
    vec![
        Constraint::Min(5),          // 列表区
        Constraint::Length(3),       // 状态栏
        Constraint::Length(2),       // 帮助栏
    ]
};
```

**替代方案**：
- 固定高度预览区 → 不适应不同长度的内容
- 列表区 60%+预览区 35% → 预览区空间略小，长文本滚动频繁

### 3. 滚动状态管理

**决策**：在 `TodoApp` 结构体中新增 `preview_scroll` 字段，独立跟踪预览区滚动偏移。

**实现方式**：
```rust
struct TodoApp {
    // ... 现有字段 ...
    preview_scroll: u16,      // 预览区滚动偏移
    last_preview_index: Option<usize>, // 上一次预览的选中项索引，用于重置滚动
}
```

**滚动重置逻辑**：
- 切换选中项时，`preview_scroll` 重置为 0
- 通过 `last_preview_index` 检测选中项变化

**替代方案**：
- 不重置滚动 → 切换到新项时可能显示中间内容，体验混乱
- 使用 Vec 存储每项滚动位置 → 内存开销大，无必要

### 4. 预览区内容渲染

**决策**：使用 ratatui 的 `Paragraph` 组件 + `Wrap` 选项实现自动换行。

**实现方式**：
```rust
let preview = Paragraph::new(content)
    .block(preview_block)
    .wrap(Wrap { trim: false })
    .scroll((preview_scroll, 0));
```

**标题显示**：
- 格式：`📖 第 N 项预览 [YYYY-MM-DD]`
- 长文本时显示滚动进度：`📖 第 N 项预览 [X/Y行] Alt+↓/↑滚动`

### 5. 快捷键绑定

**决策**：复用 Alt+↑/↓ 滚动预览区（与日报编辑器一致）。

**实现位置**：在 TUI 主循环的事件处理中，检测 `KeyModifiers::ALT` + `KeyCode::Up/Down`。

**不影响现有功能**：
- 当前 Todo TUI 未使用 Alt 组合键，无冲突
- 与日报编辑器行为一致，降低用户学习成本

## Risks / Trade-offs

### 风险 1：预览区占用空间导致列表行数减少

**风险**：长内容时预览区显示，列表可视行数减少，可能影响浏览效率。

**缓解措施**：
- 预览区使用 `Constraint::Min(5)` 最小高度限制，保证至少 5 行可见
- 用户可通过 `e` 进入编辑模式查看完整内容，预览区作为辅助功能

### 风险 2：频繁切换选中项时预览区滚动重置可能造成困惑

**风险**：用户在预览区滚动后，切换到下一项时滚动位置重置为顶部。

**缓解措施**：
- 这是合理的设计，新选中项应从顶部开始显示
- 状态栏提示当前滚动进度，帮助用户理解位置

### 风险 3：中文等多字节字符的显示宽度计算

**风险**：`display_width()` 函数假设非 ASCII 字符宽度为 2，但某些 Unicode 字符宽度可能不同。

**缓解措施**：
- 复用现有 `display_width()` 函数，已在 `todo.rs` 中验证正确性
- 使用 `unicode_width` crate（已在 `editor.rs` 中使用）确保准确性
