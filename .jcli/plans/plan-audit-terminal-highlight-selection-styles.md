# Audit terminal highlight selection styles

## 背景

用户指出 `assets/themes/terminal.json` 下，选中/高亮项的背景应保持当前自然效果：背景使用 `reverse`，前景使用 `reset`，不要把背景或 `ThemeColor::apply_fg()` 的 reverse 语义改成真正的前景反转。实际要修的是：选中行内的右侧说明文字、次要信息、tag 等子文本，不应继续使用 dim/config_dim/text_dim，而应在选中态跟随该行的高亮前景色。

当前已确认 chat 输入区 popup 的问题点：
- `src/command/chat/ui/popup.rs`
  - slash command popup：选中行的 `cmd.description()` 之前固定 `t.text_dim`。
  - custom command popup：选中行的 `desc` 之前固定 `t.text_dim`。
  - 这类应保留已做方向：选中时用 `t.popup_highlight_fg.apply_fg(Style::default())`，未选中仍用 `t.text_dim`。

必须保留/回退的语义：
- `assets/themes/terminal.json`
  - `popup_highlight_bg`: `reverse`
  - `popup_highlight_fg`: `reset`
- `src/theme.rs`
  - `ThemeColor::apply_fg(ThemeColor::Reverse)` 仍应返回 `style.fg(Color::Reset)`，不要添加 `Modifier::REVERSED`。

## terminal 主题相关高亮配置

`assets/themes/terminal.json` 中与选中/高亮相关的关键字段：
- `model_sel_highlight_bg`: `reverse`
- `model_sel_highlight_fg`: `reset`
- `popup_highlight_bg`: `reverse`
- `popup_highlight_fg`: `reset`
- `config_label_selected`: `reset`
- `tab_active_bg`: `reverse`
- `tab_active_fg`: `reset`

## 初步审计发现

### 1. 已定位且应修复：Chat 输入 popup

文件：`src/command/chat/ui/popup.rs`

现状/风险：
- List 级别 highlight_style 使用 `popup_highlight_bg` + `popup_highlight_fg`。
- 但行内的说明 Span 若显式设置为 `t.text_dim`，会覆盖选中行高亮前景，导致 terminal 主题下选中项右侧说明文字仍是 dim。

计划：
- 保留：选中时说明文字使用 `t.popup_highlight_fg.apply_fg(Style::default())`。
- 未选中时继续 `Style::default().fg(t.text_dim)`。

### 2. 必须修复：共享命令面板组件（Todo / Notebook popup 颜色问题）

文件：`src/tui/components/command_popup.rs`

影响范围：
- Todo 命令面板：`src/command/todo/ui.rs` 调用 `render_command_popup()`。
- Notebook 命令面板/路径补全：`src/command/notebook/ui.rs` 调用 `render_command_popup()`。
- 其他复用 `CommandPopupConfig` 的命令 popup。

当前代码现状：
- 弹窗颜色是局部硬编码组合：
  - border/title: `t.md_h1`
  - popup bg: `t.bg_primary`
  - pointer selected: `accent`
  - key: 固定 `t.label_ai`
  - label: 固定 `t.text_dim`
  - list highlight: `Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)`
- 这导致 Todo / Notebook popup 在 terminal 主题下存在明显颜色问题：选中行虽然被 reversed，但 key/label/pointer 等 Span 自己显式设置了前景色，尤其 label 固定 dim，会覆盖或破坏选中态前景一致性。

计划：
- 共享组件集中修，不分别改 Todo/Notebook 调用方。
- 将选中行所有文字 Span 的前景统一为选中前景，不再保留 label 的 dim：
  - selected label 使用 `t.config_label_selected` 或等价 reset 前景。
  - selected key/pointer 也避免继续固定 `label_ai`/`accent`。
  - unselected label 继续 `t.text_dim`。
  - unselected key 可继续强调色，保持非选中可读性。
- 背景/反转效果保持现有命令面板自然效果，优先不改成硬编码背景色。
- 如果实现中需要更精确表达，可为共享 command popup 引入局部 helper：`command_popup_text_style(is_selected, selected_fg, normal_fg)`，但不改外部 API。

### 3. 应修复：编辑器命令 popup

文件：`src/tui/editor_core/editor/render.rs`

位置：命令 popup 渲染处，约 621 行附近。

现状/风险：
- `name_style` 在选中时加粗。
- `desc_style` 固定为 `Style::default().fg(dim_color)`。
- list highlight_style 使用 `.bg(accent).fg(popup_bg)`，这不完全跟 terminal 主题的 `reverse/reset` 体系一致；尤其右侧 desc 固定 dim。

计划：
- 至少修复 desc_style：选中时使用和选中 name 一致的前景或 highlight foreground，未选中继续 dim。
- 该编辑器组件使用独立 theme gallery，不一定直接走 `assets/themes/terminal.json`，需谨慎最小改动，避免改变非 terminal 主题视觉。

### 4. 应修复：主题选择 popup

文件：
- `src/command/help/ui.rs` 主题选择弹窗，约 381 行附近。
- `src/tui/editor_core/editor/render.rs` 主题选择弹窗，约 670 行附近。

现状/风险：
- list highlight_style 用 `.bg(accent).fg(popup_bg)`，并非 terminal 主题字段。
- 行内 name_style 在选中时 `.fg(text_color).bold()`，可能覆盖 list highlight 前景。

计划：
- 如果这些弹窗属于 terminal 主题下可见 UI，统一让选中行内所有 Span 的前景与 highlight 前景一致。
- 但这里没有“右侧说明文字”问题，只有选中行主文字可能覆盖 List highlight。优先级低于带说明文字的列表。

### 5. 应修复：Chat 配置页可选行 secondary/tag/desc

共享文件：`src/tui/components/row.rs`

问题函数：
- `selectable_row(primary, secondary, selected, theme)`
  - primary 选中时用 `config_label_selected`。
  - secondary 始终 `config_dim`。
- `toggle_list_item(ctx)`
  - name 选中时用 `config_label_selected`。
  - desc/tag 始终 `config_dim`。

影响范围：
- `src/command/chat/ui/config/archive.rs`
  - 归档列表：名称 + `secondary`。
- `src/command/chat/ui/config/session.rs`
  - 会话列表：标题预览 + `secondary`。
- `src/command/chat/ui/config/hooks.rs`
  - hooks 列表经 `toggle_list_item`。
- 其他直接复用 `selectable_row`/`toggle_list_item` 的配置列表。

计划：
- 在共享函数内修复：当 `selected` 为 true 时，secondary/desc/tag 使用 `theme.config_label_selected` 或同一选中前景；未选中仍使用 `theme.config_dim`。
- 这样 terminal 主题下从 dim 变为 reset，符合用户“文字颜色”诉求。
- 不引入背景变化。

### 6. 应修复：Chat 配置页 Commands/Skills 列表描述折行

文件：
- `src/command/chat/ui/config/commands.rs`
- `src/command/chat/ui/config/skills.rs`

现状/风险：
- 名称行选中时使用 `config_label_selected`。
- tag 使用 `config_dim`。
- 描述折行 `desc_style` 固定 `config_dim`，且这些描述行通过 `push_raw` 加入，不一定与选中字段索引一一对应。

计划：
- 对选中的 command/skill：名称行中的 tag 使用 `config_label_selected`。
- 对紧随其后的描述行：如果对应 item selected，则描述折行也使用 `config_label_selected`；未选中继续 `config_dim`。
- 注意不要污染 `field_line_indices`，仍可用 `push_raw`。

### 7. 应修复：Chat 配置页 Teammates 列表多列次要信息

文件：`src/command/chat/ui/config/teammates.rs`

现状/风险：
- selected 时 pointer/name 使用 `config_label_selected`。
- role/status/detail 等多列字段仍大量使用 `config_dim` 或状态色。

计划：
- 对纯次要文本列（role/description/metadata）在 selected 时使用 `config_label_selected`。
- 对状态色字段需要谨慎：状态颜色有语义，不一定应覆盖；可只处理明显的 dim 文本列，保留 error/running/completed 等状态色。

### 8. 可能不需要修复：归档独立列表

文件：`src/command/chat/ui/archive.rs`

现状：
- 每个 ListItem 是单个 Span，非选中 `text_dim`，选中 `model_sel_highlight_fg` + bold。
- List highlight_style 也使用 `model_sel_highlight_bg/fg`。

结论：
- 没有“同一选中行里右侧说明仍 dim”的问题，暂不改。

### 9. 已纳入本次修复：Todo / Notebook 命令 popup

文件：
- `src/command/todo/ui.rs`
- `src/command/notebook/ui.rs`
- 共享实现：`src/tui/components/command_popup.rs`

现状：
- Todo / Notebook 的命令 popup 复用共享 `render_command_popup()`。
- 颜色问题不是调用方传参造成，而是共享组件内部：选中行内 key/label/pointer 显式前景色与 `REVERSED` highlight 叠加不一致。

结论：
- 本次必须修。
- 主要改共享组件即可，调用方通常无需改。

## 实施计划

1. 建立一个小型样式辅助逻辑，优先放在已有共享组件附近，避免重复 if selected：
   - 对 config 行：`selected_dim_style(selected, theme)`，selected 时返回 `config_label_selected`，否则 `config_dim`。
   - 对 popup 行：局部使用 `if is_selected { highlight_fg } else { dim }`。

2. 修复明确问题点：
   - `src/command/chat/ui/popup.rs`
   - `src/tui/components/row.rs`
   - `src/command/chat/ui/config/commands.rs`
   - `src/command/chat/ui/config/skills.rs`
   - `src/command/chat/ui/config/teammates.rs` 中纯 dim 次要列
   - `src/tui/components/command_popup.rs`：重点修 Todo / Notebook popup 选中行颜色，selected 行内 pointer/key/label 统一选中前景，label 不再 dim
   - `src/tui/editor_core/editor/render.rs` 命令 popup desc

3. 保持不变：
   - `assets/themes/terminal.json` 的 `popup_highlight_bg: reverse` / `popup_highlight_fg: reset`。
   - `model_sel_highlight_bg/fg` 的 reverse/reset。
   - `ThemeColor::apply_fg(Reverse)` 继续映射为 `Color::Reset`。
   - 不改背景色，不把 dim 文本改成背景 reverse。

4. 验证：
   - `cargo fmt`
   - `cargo clippy -- -D warnings`
   - 重点人工检查 terminal 主题下：
     - `/` popup、`@` popup、自定义命令 popup。
     - Chat 配置 Commands/Skills/Archive/Session/Hooks/Teammates 列表。
     - Todo/Notebook 命令面板如复用共享组件则跟随修复。

## 需要用户确认

本计划建议只修“选中行内显式 dim/config_dim/text_dim 的说明/次要文字”，不改 terminal 主题的背景 reverse/reset 机制，也不扩大到所有非 terminal 体系的硬编码 cyan/black 主列表。