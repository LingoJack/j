# 计划：将 Help 页面从自制 TUI 迁移到 editor_core

## 背景
当前 `src/command/help/` 使用自制的 TUI 渲染（`HelpApp` + `ui.rs`），存在滚动越界 bug，且功能简陋。
项目已有成熟的 `editor_core` Markdown 编辑器/查看器，具备完整的滚动、折行、Vim 操作、搜索等功能。

## 方案

### 核心思路
将 help 页面的每个 tab 当作一个只读 Markdown 文件，用 `open_markdown_editor_on_terminal` 打开。

但因为 help 有多 tab（10个），需要增加一个 tab 切换层：
- 底部状态栏显示 tab 名称（如 `1:快速上手 2:Chat 3:工具 4:Hook ...`）
- 按 Tab/Shift+Tab 切换 tab
- 数字键 1-9,0 跳转 tab
- `:q` 或 Esc 退出

### 实现步骤

1. **删除 `src/command/help/app.rs` 和 `src/command/help/ui.rs`** — 移除自制 TUI

2. **重写 `src/command/help.rs`** — 改为使用 editor_core：
   - 使用 `open_markdown_editor_on_terminal` 打开，传入 tab 内容
   - 在事件循环中拦截 Tab/Shift+Tab/数字键，切换内容
   - 底部状态栏显示 tab 导航信息
   - 内容为只读（拦截 Insert 模式切换）

3. **保持 `assets.rs` 的 `load_help_tabs()` 不变** — 仍然从 assets/help/*.md 加载

### 关键设计

```
fn run_help_tui() -> io::Result<()>:
    1. 加载 tabs: Vec<HelpTab>
    2. 进入 editor_core 的终端模式
    3. 创建 MarkdownEditor，初始内容为 tabs[0].content
    4. 主循环：
       - render (复用 editor_core 的 render)
       - 渲染额外的 tab 栏（在状态栏上方）
       - 处理按键：
         - Tab/Shift+Tab/1-9,0 → 切换 tab（重建 editor 内容）
         - 其他 → 传给 editor_core 处理
         - q/Esc/Ctrl+Q → 退出
```

### 不需要修改的文件
- `src/assets.rs` — `load_help_tabs()` 保持不变
- `src/tui/editor_core/` — 不需要修改
- `src/tui/editor_markdown.rs` — 不需要修改（help 直接用 editor_core 的 API）
- `assets/help/*.md` — 不需要修改

### 需要删除的文件
- `src/command/help/app.rs`
- `src/command/help/ui.rs`

### 需要修改的文件
- `src/command/help.rs` — 重写主循环
- `src/command/help/mod.rs` — 如果存在的话，移除子模块声明

## 用户操作（迁移后）
| 按键 | 功能 |
|------|------|
| j/↓ | 下移 |
| k/↑ | 上移 |
| Tab/→ | 下一个 tab |
| Shift+Tab/← | 上一个 tab |
| 1-9,0 | 跳转 tab |
| G | 跳到底部 |
| gg | 跳到顶部 |
| / | 搜索 |
| :q/Esc | 退出 |
| Ctrl+Q | 退出 |
