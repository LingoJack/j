# Slash Command 输入交互重构方案

## 问题分析

### 当前实现的问题

#### 1. Chat 模块 (`src/command/chat/ui/`)
- `draw_slash_popup` 浮动在输入区域上方
- 用户输入 `/` 触发命令后，弹窗遮挡输入框
- **核心问题**：用户继续输入筛选关键词时，看不到自己输入的内容
- 同样的问题存在于 `@popup`、`file_popup`、`skill_popup`、`command_popup`

#### 2. Notebook 模块 (`src/command/notebook/ui.rs`)
- `draw_command_popup` 浮动在主区域底部
- 状态栏显示筛选状态，帮助栏显示操作提示
- **核心问题**：用户输入筛选词时，filter 隐藏在 `cmd_popup_filter` 变量中，界面只显示标题栏的 `[筛选: xxx]`
- 没有可见的输入光标，看不到正在输入的内容

#### 3. Todo 模块 (`src/command/todo/ui.rs`)
- 同 Notebook，命令面板浮动在列表区上方
- **核心问题**：筛选输入不可见，用户看不到自己输入的关键词
- 状态栏显示模式，帮助栏显示操作提示，但看不到输入过程

---

## 解决方案

### 核心思路：将"底部提示栏"转变为"输入区"

用户的需求很明确：当触发 slash command 时，**底部应该变成一个可见的输入区**，用户可以在那里看到自己的输入内容，上方显示筛选结果列表。

### 设计原则

1. **输入可见性**：用户始终能看到自己正在输入的内容和光标位置
2. **列表实时更新**：筛选列表随输入实时更新
3. **统一交互体验**：三个模块采用一致的交互模式
4. **最小代码改动**：复用现有的输入渲染逻辑

---

## 详细实现方案

### Phase 1: Chat 模块改造

#### 1.1 UI 布局调整

当 `slash_popup_active` 或其他补全弹窗激活时：
- 输入区（`chunks[2]`）保持不变，继续显示用户输入
- 弹窗位置改为**输入区上方**，而不是覆盖输入区
- 弹窗高度动态计算，最大不超过消息区可用空间

#### 1.2 弹窗定位改造

修改 `draw_slash_popup`、`draw_at_popup` 等函数：
```rust
// 当前：弹窗浮动在 input_area 上，遮挡输入框
// 改为：弹窗浮动在 input_area 上方（消息区底部）

// 计算弹窗位置：
// popup_area.y = input_area.y - popup_height（向上偏移）
// popup_area.x = input_area.x + prompt_width（对齐输入内容）
```

#### 1.3 输入区显示优化

当补全弹窗激活时，输入区显示：
- `/search` 显示 `/search` + 光标（用户能看到自己输入的关键词）
- 弹窗标题显示筛选状态，但输入内容在输入区可见

### Phase 2: Notebook 模块改造

#### 2.1 布局重构

当前布局：
```
[标题栏]      Constraint::Length(3)
[主区域]      Constraint::Min(5)     ← 命令弹窗浮动在此
[状态栏]      Constraint::Length(3)
[帮助栏]      Constraint::Length(1)
```

改造后布局（CommandPopup 模式）：
```
[标题栏]      Constraint::Length(3)
[主区域]      Constraint::Min(5)     ← 筛选结果列表（原有内容隐藏）
[输入区]      Constraint::Length(3) ← 替代原状态栏，显示输入+光标
[帮助栏]      Constraint::Length(1) ← 操作提示
```

#### 2.2 状态栏转变为输入区

`render_status_bar` 在 `AppMode::CommandPopup` 时：
- 不再显示"命令面板 + 筛选: xxx"
- 改为显示实际的输入框：`> sea|rch`（带光标）
- 复用 Chat 的输入渲染逻辑

#### 2.3 主区域显示筛选列表

`render_list` 在 `CommandPopup` 模式时：
- 不显示笔记列表
- 显示命令筛选结果列表
- 取消浮动弹窗，直接渲染在主区域

### Phase 3: Todo 模块改造

与 Notebook 类似的改造方案：

#### 3.1 布局重构

CommandPopup 模式时：
```
[标题栏]      Constraint::Length(3)
[命令列表]    Constraint::Min(5)     ← 筛选结果列表
[输入区]      Constraint::Length(3) ← 替代原状态栏
[帮助栏]      Constraint::Length(1)
```

#### 3.2 状态栏改造

`render_status_bar` 在 `AppMode::CommandPopup` 时显示输入框，而非静态文本。

---

## 文件改动清单

### Chat 模块

| 文件 | 改动内容 |
|------|----------|
| `src/command/chat/ui/chat.rs` | 修改弹窗定位逻辑，改为输入区上方浮动 |
| `src/command/chat/ui/input.rs` | 无需改动，输入区已正确显示 |
| `src/command/chat/ui/hint.rs` | 可能需要调整提示内容 |

### Notebook 模块

| 文件 | 改动内容 |
|------|----------|
| `src/command/notebook/ui.rs` | 大改：`draw_ui`布局、`render_list`、`render_status_bar`、删除`draw_command_popup` |
| `src/command/notebook/app.rs` | 小改：可能需要调整 `handle_command_popup_mode` 的按键处理 |

### Todo 模块

| 文件 | 改动内容 |
|------|----------|
| `src/command/todo/ui.rs` | 大改：`draw_ui`布局、`render_list`、`render_status_bar`、删除`draw_command_popup` |
| `src/command/todo/app.rs` | 小改：可能需要调整 `handle_command_popup_mode` 的按键处理 |

---

## 实现步骤

### Step 1: Chat 模块弹窗位置调整

**优先级：高** - 这是用户最常用的功能

1. 修改 `draw_slash_popup` 的定位计算
2. 弹窗 y 坐标从 `input_area.bottom()` 改为 `input_area.y.saturating_sub(popup_height)`
3. 确保弹窗不超出屏幕顶部

### Step 2: Notebook 模块重构

**优先级：中**

1. 修改 `draw_ui` 在 `CommandPopup` 模式下的布局约束
2. 新增 `render_command_list` 函数渲染筛选结果
3. 修改 `render_status_bar` 显示输入框样式
4. 删除 `draw_command_popup` 浮动弹窗函数

### Step 3: Todo 模块重构

**优先级：中**

与 Notebook 类似：
1. 修改布局
2. 状态栏改为输入区
3. 列表区显示命令筛选结果

### Step 4: 测试验证

- 测试 `/search`、`/help` 等命令的筛选体验
- 验证光标可见性
- 验证实时筛选响应

---

## 技术要点

### 弹窗定位计算

```rust
// Chat 模块：弹窗浮动在输入区上方
fn compute_popup_position(input_area: Rect, popup_height: u16, msg_area: Rect) -> Rect {
    let y = input_area.y.saturating_sub(popup_height).max(msg_area.y);
    let x = input_area.x + 3; // 对齐提示符之后
    let width = popup_width.min(input_area.width);
    Rect::new(x, y, width, popup_height)
}
```

### 输入区光标渲染

复用现有的光标渲染逻辑，关键是：
1. 显示 `prompt`（如 `>` 或 `/`）
2. 显示用户输入内容
3. 在正确位置渲染光标块

### 状态栏输入区转换

```rust
// Notebook/Todo: 状态栏在 CommandPopup 时显示输入
fn render_status_bar_as_input(f: &mut Frame, area: Rect, app: &App) {
    let input_text = &app.cmd_popup_filter;
    let cursor_pos = app.cmd_popup_filter.chars().count();
    // 渲染类似 Chat 输入框的样式
    draw_inline_input(f, area, input_text, cursor_pos, " / ");
}
```

---

## 预期效果

### 改造前

```
┌─────────────────────────┐
│ [消息区]                │
│                         │
│ ┌─────────────────┐     │ ← 弹窗遮挡输入区
│ │ /search  搜索    │     │
│ │ /help    帮助    │     │
│ └─────────────────┘     │
│ > /se█                  │ ← 用户看不到输入
├─────────────────────────┤
│ @ / 命令 Ctrl+M ...     │ ← 提示栏
└─────────────────────────┘
```

### 改造后

```
┌─────────────────────────┐
│ [消息区]                │
│                         │
│ ┌─────────────────┐     │ ← 弹窗在输入区上方
│ │ /search  搜索    │     │
│ │ /help    帮助    │     │
│ └─────────────────┘     │
├─────────────────────────┤
│ > /se█rch               │ ← 输入区可见
├─────────────────────────┤
│ ↑↓ 选择 Enter 确认 ...  │ ← 提示栏
└─────────────────────────┘
```

### Notebook/Todo 改造后

```
┌─────────────────────────┐
│ 📝 待办                  │
├─────────────────────────┤
│ ❯ toggle   切换完成      │ ← 命令列表在主区域
│   edit     编辑          │
│   add      添加          │
│   delete   删除          │
├─────────────────────────┤
│ / edi█                   │ ← 输入区（原状态栏）
├─────────────────────────┤
│ ↑↓ 选择 Enter 确认 Esc   │ ← 帮助栏
└─────────────────────────┘
```

---

## 风险评估

1. **改动范围大**：涉及三个核心模块的 UI 层
2. **交互一致性**：需要确保三个模块的体验一致
3. **回归风险**：需要全面测试各模式的切换

建议分阶段实施，先改 Chat（用户最常用），验证后再改 Notebook 和 Todo。