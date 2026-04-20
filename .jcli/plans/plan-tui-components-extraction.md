# TUI 公共组件提取方案

## 目标

从 `chat/ui/`、`todo/ui.rs`、`notebook/ui.rs` 中提取可复用的 UI 组件，减少代码重复，提高维护效率。

---

## 已有公共组件 (`src/tui/components/`)

| 组件 | 功能 | 使用场景 |
|------|------|----------|
| `pointer` | 选中指针 | 所有列表 |
| `label` | 标签/描述 | 配置页 |
| `cursor` | 单行光标 | 输入框 |
| `row` | 行组件 | 配置页 |
| `list` | 列表容器 | 配置页 |
| `tab_bar` | Tab 栏 | 配置页 |
| `hint` | 提示行 | 帮助页/底部栏 |
| `separator` | 分隔线 | 通用 |
| `consts` | 常量 | 通用 |

---

## 待提取组件

### 1. `command_popup.rs` — 命令面板弹窗

**现状**：`chat/ui/chat.rs`、`todo/ui.rs`、`notebook/ui.rs` 各有独立的 `draw_command_popup`，逻辑几乎相同。

**重复代码特征**：
- 计算弹窗宽度/高度
- 位置：主区域底部偏左
- 样式：pointer + key + label 三列布局
- 使用 `Clear` 清除区域

**提取方案**：

```rust
/// 命令面板配置
pub struct CommandPopupConfig<'a> {
    pub title: &'a str,
    pub title_filter: Option<&'a str>,  // 如 "[filter]"
    pub items: &'a [(&'a str, &'a str)], // (key, label)
    pub selected: usize,
    pub theme: &'a Theme,
}

/// 绘制命令面板弹窗
pub fn draw_command_popup(
    f: &mut ratatui::Frame,
    main_area: Rect,
    config: &CommandPopupConfig<'_>,
) {
    // 通用逻辑
}
```

**收益**：消除约 90 行重复代码 × 3 处。

---

### 2. `popup_list.rs` — 通用浮动列表弹窗

**现状**：`chat/ui/chat.rs` 已有 `draw_popup_list`，但仅用于 chat 模块的 5 个弹窗。

**提取方案**：

将现有 `draw_popup_list` + `PopupConfig` 移到公共组件库：

```rust
/// 弹窗列表配置
pub struct PopupConfig {
    pub title: String,
    pub selected: usize,
    pub max_visible: usize,
    pub title_color: Color,
    pub border_color: Color,
    pub bg_color: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
}

/// 通用浮动弹窗列表渲染（输入区上方）
pub fn draw_popup_list(
    f: &mut ratatui::Frame,
    anchor_area: Rect,  // 锚定区域（输入区或主区域）
    items: Vec<ListItem<'static>>,
    item_labels: &[String],
    cfg: &PopupConfig,
    position: PopupPosition,  // Above / BottomLeft
) { ... }

pub enum PopupPosition {
    Above,       // 锚定区域上方（用于 @ 补全等）
    BottomLeft,  // 主区域底部偏左（用于命令面板）
}
```

**收益**：
- chat 模块可继续使用
- todo/notebook 命令面板可复用
- 消除 `draw_command_popup` 重复逻辑

---

### 3. `cursor_wrapped.rs` — 多行折行光标渲染

**现状**：三处独立实现折行光标逻辑：

| 文件 | 函数 |
|------|------|
| `todo/ui.rs` | `build_cursor_wrapped_lines` |
| `notebook/ui.rs` | `build_adding_item` / `build_rename_item` |
| `chat/ui/input.rs` | 内联折行逻辑 |

**重复代码特征**：
```rust
let wrapped = wrap_text(input, width);
for (line_idx, line_str) in wrapped.iter().enumerate() {
    // 计算 cursor_on_this_line
    // 渲染 before + cursor_ch + after
}
```

**提取方案**：

扩展现有 `cursor.rs`：

```rust
use crate::util::text::wrap_text;

/// 多行折行光标渲染结果
pub struct WrappedCursorLines {
    pub lines: Vec<Line<'static>>,
    pub cursor_line_idx: usize,  // 光标所在行（用于终端光标定位）
    pub cursor_col_in_line: usize,
}

/// 构建带折行光标的行列表
pub fn cursor_wrapped_lines(
    input: &str,
    cursor_pos: usize,
    width: usize,
    placeholder: Option<&str>,
    theme: &Theme,
) -> WrappedCursorLines {
    if input.is_empty() && placeholder.is_some() {
        // 显示占位符 + 光标
        return placeholder_line(placeholder.unwrap(), theme);
    }
    // 正常折行 + 光标渲染
}
```

**收益**：消除约 40 行重复逻辑 × 3 处。

---

### 4. `status_input.rs` — 状态栏输入框

**现状**：`todo/ui.rs` 和 `notebook/ui.rs` 的 `render_input_status_bar` 函数完全相同。

**提取方案**：

```rust
/// 状态栏输入框参数
pub struct StatusInputParams<'a> {
    pub label: &'a str,
    pub label_color: Color,
    pub input: &'a str,
    pub cursor_pos: usize,
    pub placeholder: &'a str,
    pub hint: &'a str,
}

/// 在状态栏区域渲染输入框
pub fn draw_status_input(
    f: &mut ratatui::Frame,
    area: Rect,
    params: &StatusInputParams<'_>,
    theme: &Theme,
) {
    // 渲染 + 设置终端光标位置
}
```

**收益**：消除约 60 行重复代码 × 2 处。

---

### 5. `help_page.rs` — 帮助页面框架

**现状**：三处都有类似帮助页面结构：

| 文件 | 函数 |
|------|------|
| `chat/ui/help.rs` | `draw_help` |
| `todo/ui.rs` | `render_help` |
| `notebook/ui.rs` | `render_help` |

**重复特征**：
- 标题行（图标 + 文字）
- 空行
- 快捷键行（key + desc 格式）
- 外框 Block

**提取方案**：

```rust
/// 帮助页面配置
pub struct HelpPageConfig<'a> {
    pub title: &'a str,
    pub title_icon: Option<&'a str>,
    pub shortcuts: &'a [(&'a str, &'a str)],
    pub footer_lines: Option<Vec<Line<'a>>>,
    pub theme: &'a Theme,
}

/// 绘制帮助页面
pub fn draw_help_page(
    f: &mut ratatui::Frame,
    area: Rect,
    config: &HelpPageConfig<'_>,
) {
    let mut lines = vec![
        Line::from(""),
        section_header(icon, title, theme),
        Line::from(""),
    ];
    for (key, desc) in shortcuts {
        lines.push(help_key_row(key, desc, KEY_WIDTH, theme));
    }
    if let Some(footer) = footer_lines {
        lines.extend(footer);
    }
    // 渲染 Block + Paragraph
}
```

**收益**：消除约 50 行重复代码 × 3 处。

---

### 6. `confirm_dialog.rs` — 确认对话框

**现状**：三处都有确认对话框：

| 文件 | 场景 |
|------|------|
| `todo/ui.rs` | `ConfirmDelete` / `ConfirmReport` |
| `notebook/ui.rs` | `ConfirmDelete` |
| `chat/ui/chat.rs` | `ArchiveConfirm` |

**提取方案**：

```rust
/// 确认对话框类型
pub enum ConfirmStyle {
    Danger,   // 红色边框（删除等危险操作）
    Warning,  // 黄色边框（警告）
    Normal,   // 默认颜色
}

/// 确认对话框参数
pub struct ConfirmParams<'a> {
    pub title: &'a str,
    pub title_icon: Option<&'a str>,
    pub message: &'a str,
    pub hint: &'a str,  // "y 确认 | n 取消"
    pub style: ConfirmStyle,
    pub theme: &'a Theme,
}

/// 绘制确认对话框
pub fn draw_confirm_dialog(
    f: &mut ratatui::Frame,
    area: Rect,
    params: &ConfirmParams<'_>,
) { ... }
```

**收益**：消除约 30 行重复代码 × 多处。

---

## 组件优先级

按重复程度和收益排序：

| 优先级 | 组件 | 重复度 | 收益 |
|--------|------|--------|------|
| **P0** | `command_popup.rs` | 高（3处完全相同） | 高 |
| **P0** | `status_input.rs` | 高（2处完全相同） | 中 |
| **P1** | `cursor_wrapped.rs` | 中（逻辑相同，细节不同） | 高 |
| **P1** | `popup_list.rs` | 低（已有，需扩展位置参数） | 中 |
| **P2** | `help_page.rs` | 中（结构相同，内容不同） | 中 |
| **P2** | `confirm_dialog.rs` | 低（样式略有不同） | 低 |

---

## 实施步骤

### Phase 1：高优先级组件（P0）

1. **创建 `command_popup.rs`**
   - 定义 `CommandPopupConfig`
   - 实现 `draw_command_popup`
   - 更新 `mod.rs` 导出

2. **创建 `status_input.rs`**
   - 定义 `StatusInputParams`
   - 实现 `draw_status_input`
   - 更新 `mod.rs` 导出

3. **重构 todo/notebook/chat**
   - 替换原有 `draw_command_popup` 调用
   - 替换原有 `render_input_status_bar` 调用

### Phase 2：中优先级组件（P1）

4. **扩展 `cursor.rs`**
   - 添加 `cursor_wrapped_lines`
   - 添加 `WrappedCursorLines` 结构体

5. **扩展 `popup_list.rs`**
   - 从 chat 移到公共库
   - 添加 `PopupPosition` 参数

### Phase 3：低优先级组件（P2）

6. **创建 `help_page.rs`**
   - 定义 `HelpPageConfig`
   - 实现 `draw_help_page`

7. **创建 `confirm_dialog.rs`**
   - 定义 `ConfirmParams` / `ConfirmStyle`
   - 实现 `draw_confirm_dialog`

---

## 文件结构变更

```
src/tui/components/
├── mod.rs           # 更新导出
├── consts.rs        # 不变
├── cursor.rs        # 添加多行折行支持
├── label.rs         # 不变
├── list.rs          # 不变
├── pointer.rs       # 不变
├── row.rs           # 不变
├── separator.rs     # 不变
├── tab_bar.rs       # 不变
├── hint.rs          # 不变
├── command_popup.rs # 新增
├── popup_list.rs    # 新增（从 chat 移入）
├── status_input.rs  # 新增
├── help_page.rs     # 新增
└── confirm_dialog.rs # 新增
```

---

## 测试策略

每个新组件需要：
1. 单元测试（配置构建、参数验证）
2. 视觉测试（在 todo/chat/notebook 中验证效果一致）
3. 边界测试（空列表、超长文本、极端宽度）

---

## 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 样式细节差异 | 各模块配色略有不同 | 使用 Theme 参数，支持自定义 |
| 折行逻辑差异 | chat 有图片占位等特殊处理 | 保留扩展点，渐进迁移 |
| 回滚复杂度 | 多文件同时修改 | 分阶段提交，每阶段独立验证 |

---

## 预期收益

- 消除约 300+ 行重复代码
- 新增 UI 功能只需一处实现
- 统一视觉风格，降低维护成本
- 为后续模块（如 settings）提供组件基础