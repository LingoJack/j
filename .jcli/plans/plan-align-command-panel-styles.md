# Plan: align-command-panel-styles

## 目标
将 todo 命令面板的样式与 chat（`c open`）命令面板和 editor 命令面板保持一致。

## 现状分析

### 三个命令面板位置
1. **Chat 命令面板** — `src/command/chat/ui/chat.rs` → `draw_command_popup()` (L1328)
2. **Editor 命令面板** — `src/tui/editor_core/editor.rs` → `render_command_popup()` (L749)
3. **Todo 命令面板** — `src/command/todo/ui.rs` → `draw_command_popup()` (L411)

### 样式对比

| 属性 | Chat (c open) | Editor | Todo (当前) |
|------|--------------|--------|-------------|
| border_color | `t.md_h1` | `accent` (= `self.theme.md_h1`) | `accent` (= `app.theme.md_h1`) |
| title_color | `t.md_h1` | `accent` (= `self.theme.md_h1`) | `accent` (= `app.theme.md_h1`) |
| popup_bg | `t.bg_primary` | `popup_bg` (= `self.theme.bg_primary`) | `popup_bg` (= `app.theme.bg_primary`) |
| highlight_bg | `t.md_h1` | `accent` (= `self.theme.md_h1`) | `highlight_bg` (= `accent` = `app.theme.md_h1`) |
| highlight_fg | `t.bg_primary` | `popup_bg` (= `self.theme.bg_primary`) | `popup_bg` (= `app.theme.bg_primary`) |
| 命令名样式 | `t.label_ai` + BOLD | `text_color` + BOLD (选中) / `text_color` (未选中) | `text_color` + BOLD |
| 分隔符 | ` - ` | 无显式分隔 | 无显式分隔 |

### 关键发现
经过详细对比，**三者实际上已使用相同的核心颜色值**（`md_h1` 作为 accent, `bg_primary` 作为背景）。主要差异在于：

1. **命令名颜色不同**: Chat 用 `t.label_ai`（独特的标签色），Editor 和 Todo 用 `text_color`（普通文本色）
2. **选中项渲染**: Chat/Editor 都有选中态高亮背景+反转前景色，Todo 也有
3. **分隔符**: Chat 在命令名和描述之间有 ` - ` 分隔符显示

实际上这三个面板在核心颜色（border、bg、highlight）上已经一致。如果用户在视觉上看到差异，可能来自于：
- 命令名颜色：Chat 用 `label_ai`，Todo 用 `text_normal`
- 高亮时的文字可见性差异

## 修改计划

### Step 1: 统一 Todo 命令面板的命令名颜色为 `label_ai`
- 文件: `src/command/todo/ui.rs` L468
- 将 `Style::default().fg(text_color).add_modifier(Modifier::BOLD)` 改为使用主题的 `label_ai` 颜色
- 需要引入 `label_ai` 变量: `let label_ai = app.theme.label_ai;`

### Step 2: 验证 Editor 命令面板是否也需要统一
- 如果 Editor 也有相同问题，一并修改

## 风险评估
- 低风险：仅修改颜色值，不影响布局和交互逻辑
