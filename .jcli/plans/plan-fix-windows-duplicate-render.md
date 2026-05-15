# 修复 Windows 端 Config 界面重复渲染问题

## 问题描述
Windows 端 Config 界面（Model tab）出现重复渲染现象，macOS 上正常。
用户截图显示每个配置字段（显示名称、API Base、API Key、模型名称、支持视觉）出现了两次。

## 根因分析

### 已排除的原因
1. **KeyEventKind::Release 过滤**：`dispatch_event` 第 291 行已正确过滤，只处理 `Press | Repeat`
2. **terminal.draw 双重调用**：Phase 3 和 Phase 4 有互斥逻辑（`needs_redraw = false`），不会同时渲染
3. **CONFIG_FIELDS 重复遍历**：`draw_tab_model_list` 只遍历一次 `CONFIG_FIELDS`
4. **ItemList 重复 push**：每个字段只 push 一次

### 最可能的原因：Windows 上 crossterm 差异缓冲区不清理旧内容

ratatui 0.29.0 的 `Terminal::draw` 使用差异缓冲区策略：只更新变化的 cell。
在 macOS 上，crossterm 使用 Unix PTY，差异缓冲区工作正常。
在 Windows 上，crossterm 使用 WinAPI 或 VT100 转义序列，**差异缓冲区在某些场景下可能不清理旧 cell 内容**。

具体表现：
- 当 Config 界面重新渲染时（例如滚动或切换字段），旧的 cell 内容可能没有被清除
- 这导致新旧内容叠加，看起来像是"重复渲染"

### 验证方式

需要在 Windows 上添加调试日志，记录 `terminal.draw` 的调用频率和参数。

## 修复方案

### 方案 A：在 `draw_config_screen` 中添加 `Clear` widget（推荐）

在 `draw_config_screen` 的三段布局和回退模式中，在渲染 `Paragraph` 之前先渲染 `Clear` widget，确保旧内容被清除。

具体修改：
1. **`config.rs` 中的三段布局**（header + list 模式）：
   - 在 `f.render_widget(Paragraph::from(header_lines), header_area)` 之前，先 `f.render_widget(Clear, header_area)`
   - 在 `f.render_widget(list_paragraph, list_area)` 之前，先 `f.render_widget(Clear, list_area)`

2. **`config.rs` 中的回退模式**（单 Paragraph 模式）：
   - 在 `f.render_widget(full_paragraph, area)` 之前，先 `f.render_widget(Clear, area)`

### 方案 B：在 `draw_chat_ui` 的整体背景中添加 `Clear`

在 `draw_chat_ui` 的第 33-34 行（全屏背景）之前，先渲染 `Clear` widget：
```rust
f.render_widget(ratatui::widgets::Clear, size);
let bg = Block::default().style(Style::default().bg(app.ui.theme.bg_primary));
f.render_widget(bg, size);
```

### 方案选择

推荐 **方案 A**，因为：
- 更精准地清除需要更新的区域
- 不影响其他模式的渲染性能
- `Clear` widget 是 ratatui 官方推荐的方式

如果方案 A 不够，可以升级到方案 B 作为兜底。

## 涉及文件
- `src/command/chat/ui/config.rs`（主要修改）
- `src/command/chat/ui/chat.rs`（可能需要兜底修改）
