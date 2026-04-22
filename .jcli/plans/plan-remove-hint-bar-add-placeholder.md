# 计划：移除底部 Hint Bar，改为输入区底部 Placeholder

## 需求理解

用户希望：
1. 移除底部独立的 hint bar（提示栏），节省界面空间
2. 将输入区底部区域扩大，占据原来 hint bar 的位置（输入区从 5 行变为 6 行）
3. 将原来的 hint bar 内容以 placeholder 形式显示在输入区最后一行
4. placeholder 的行为：只有当用户输入的文字到达最后一行时才取消显示

## 当前架构分析

### 现有布局结构（`chat.rs`）
```
chunks[0]: 标题栏 (动态高度)
chunks[1]: 消息区 (Min(5))
chunks[2]: 输入区 (Length(5))  ← 固定 5 行
chunks[3]: 提示栏 (Length(1))  ← 固定 1 行，用于显示快捷键提示
```

### Hint Bar 内容（`hint.rs::draw_hint_bar`）
根据 `ChatMode` 显示不同的快捷键提示：
- **Chat 模式**: `@ 引用 | / 命令 | Ctrl+M 选中模式 | Ctrl+O 工具详情 | Tab Bypass | ?/F1 帮助`
- **SelectModel**: `↑↓/jk 移动 | Enter 确认 | Esc 取消`
- **Browse**: `↑↓/jk 跳转 | Tab 角色 | y/Enter 复制 | ...`
- 其他模式各有不同提示

### 输入区（`input.rs::draw_input`）
- 当前已有 placeholder 逻辑（仅在输入为空时显示在第一行）
- 支持多行折行、光标跟踪、@mention 高亮

## 技术实现方案

### 核心思路

将 hint bar 从独立区域合并到输入区，作为输入区最后一行的"底部 placeholder"。当输入内容折行到最后一行时，placeholder 自动消失。

### 关键判断逻辑

判断"文字是否到达最后一行"：
- 计算输入文本在给定 wrap_width 下的折行数量
- 如果折行数量 >= 输入区内部可用高度，则隐藏 placeholder
- 注意：需要减去提示符占用的空间（第一行有 `> ` 或 `bypass > ` 前缀）

### 实现步骤

#### 1. 修改 `chat.rs` 布局
- 删除 `chunks[3]`（提示栏）的约束
- 将输入区高度从 `Length(5)` 改为 `Length(6)`
- 删除 `draw_hint_bar` 调用

```rust
// 修改前
.constraints([
    Constraint::Length(title_height), // 标题栏
    Constraint::Min(5),               // 消息区
    Constraint::Length(5),            // 输入区
    Constraint::Length(1),            // 操作提示栏
])

// 修改后
.constraints([
    Constraint::Length(title_height), // 标题栏
    Constraint::Min(5),               // 消息区
    Constraint::Length(6),            // 输入区（合并原 hint bar 空间）
])
```

#### 2. 修改 `input.rs::draw_input`
- 添加"底部 hint placeholder"渲染逻辑
- 将原 hint 内容转换为 placeholder 格式
- 根据折行状态决定是否显示

核心逻辑：
```rust
// 1. 计算当前输入内容的折行数
let wrapped_line_count = wrapped_lines.len();

// 2. 输入区内部可用高度（减去边框）
let inner_height = area.height.saturating_sub(2) as usize;  // 现为 4

// 3. 判断是否显示底部 placeholder
// - 输入为空或折行未填满时显示
// - 折行数 < inner_height 时显示
let show_bottom_placeholder = is_empty || wrapped_line_count < inner_height;

// 4. 在最后一行渲染 hint placeholder
if show_bottom_placeholder && !is_browse_mode {
    // 渲染 hint 文本作为 placeholder（暗色）
}
```

#### 3. 提取 hint 内容生成函数
- 从 `hint.rs` 中提取一个函数 `get_hints_for_mode(mode: ChatMode, app: &ChatApp) -> Vec<(&str, &str)>`
- 该函数返回当前模式下的 hint 键值对
- 用于生成底部 placeholder 文本

#### 4. 处理特殊情况

**Browse 模式**：
- Browse 模式当前使用输入区显示过滤状态，不显示常规 hint
- 保持现有 Browse 模式逻辑不变

**非 Chat 模式（SelectModel、Config 等）**：
- 这些模式下输入区通常不可见或不活跃
- hint 仍然显示，但可能需要不同的处理策略
- 实际上在非 Chat 模式下，输入区仍然渲染，但用户焦点在其他地方
- 需要在这些模式下也显示相应的 hint placeholder

**Loading 状态**：
- Loading 时 Chat 模式的 hint 为空（当前设计）
- 保持这个逻辑：loading 时不显示底部 hint placeholder

#### 5. Placeholder 文本格式化

将 hint 键值对格式化为单行文本：
```rust
// 输入: [("@", "引用"), ("/", "命令"), ("Ctrl+M", "选中模式")]
// 输出: "@ 引用 │ / 命令 │ Ctrl+M 选中模式"
fn format_hints_as_placeholder(hints: &[(&str, &str)]) -> String {
    hints.iter()
        .map(|(k, v)| format!("{} {}", k, v))
        .collect::<Vec<_>>()
        .join(" │ ")
}
```

如果文本过长超出可用宽度，需要截断。

#### 6. 处理滚动情况

当输入内容超出可见区域（用户已滚动）：
- 不显示 placeholder（因为最后一行已被实际内容占用）

判断滚动：
```rust
let line_scroll = compute_line_scroll(...);
// 如果 line_scroll > 0，说明有滚动，不显示 placeholder
```

### 文件变更清单

| 文件 | 变更内容 |
|------|---------|
| `src/command/chat/ui/chat.rs` | 修改布局约束，删除 hint bar 渲染调用 |
| `src/command/chat/ui/input.rs` | 添加底部 hint placeholder 渲染逻辑 |
| `src/command/chat/ui/hint.rs` | 添加 `get_hints_for_mode` 函数供 input.rs 调用 |

### 边界情况

1. **窗口高度过小**：如果窗口不足以显示 6 行输入区，需要限制最小高度
2. **Hint 文本过长**：如果 hint placeholder 超出一行宽度，截断末尾添加 `…`
3. **Browse 模式**：保持现有过滤显示逻辑，不显示 hint placeholder
4. **弹窗激活状态**：当 @popup/slash_popup 等激活时，可能需要隐藏 placeholder

### 测试要点

1. Chat 模式：输入为空时显示底部 hint placeholder
2. Chat 模式：输入一行文字时，hint placeholder 仍在最后一行显示
3. Chat 模式：输入多行文字填满输入区时，hint placeholder 消失
4. Chat 模式：输入溢出产生滚动时，hint placeholder 消失
5. Browse 模式：显示过滤状态而非 hint placeholder
6. 其他模式（SelectModel、Config 等）：显示相应模式的 hint placeholder
7. Loading 状态：不显示 hint placeholder（符合当前设计）

## 注意事项

- `help/ui.rs` 中有独立的 `draw_hint_bar` 实现，用于帮助界面，本次变更不影响
- `hint.rs::draw_toast` 保持不变，Toast 弹窗功能继续正常工作
- placeholder 样式使用 `t.text_dim` 颜色，与现有 placeholder 风格一致