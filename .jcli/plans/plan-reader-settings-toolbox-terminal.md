# Plan: reader-settings-toolbox-terminal

## 背景与目标

本次仅针对当前浏览器版 Reader UI 做三个小改动；`Command + J` PTY 终端暂不在当前 Reader 内实现，后续放到 jcli GUI / Tauri 计划中处理。

1. 「设置」不应只是主题设置，需要更接近 VS Code 的设置菜单形态，并支持点击外部/按 Esc 关闭。
2. 工具箱 tab 已经由左侧 ActivityBar 表达，不需要在工具箱侧栏顶部重复显示“工具箱”说明。
3. 文件/编辑区滚动时，垂直滚动条应位于编辑区域最右侧，而不是贴着正文内容列。

## 计划步骤

### 1. 调整设置入口与弹层行为

目标文件：

- `assets/reader/src/ActivityBar.tsx`
- 可能涉及 `assets/reader/src/Icon.tsx`

改动：

- 将当前设置弹层从“主题设置面板”改成更通用的设置菜单。
- 设置按钮点击后弹出菜单，菜单内包含至少：
  - `颜色主题`：展示当前可用主题选项。
  - 只放已能真实工作的入口；暂不做假入口，避免误导。
- 主题切换仍保留 `Aliyun Light` / `Seeyue Warm`。
- 增加点击外部关闭逻辑：
  - 使用 wrapper ref。
  - 监听 document `pointerdown`。
  - 点击设置菜单外部时关闭。
- 增加 `Esc` 关闭逻辑：
  - 监听 document `keydown`。
  - `Escape` 时关闭。
- 保持 ActivityBar 仍只负责侧栏切换与设置入口，不引入复杂业务状态。

### 2. 删除工具箱顶部冗余标题栏

目标文件：

- `assets/reader/src/Toolbox.tsx`

改动：

- 删除顶部“工具箱”标题栏及 icon。
- 工具列表直接从顶部开始渲染。
- 调整容器 padding，避免列表贴边。
- 保留工具项名称与说明，例如“文本 Diff”“JSON 查看器”。

### 3. 将文件/编辑区滚动条放到最右侧

目标文件：

- `assets/reader/src/Reader.tsx`
- `assets/reader/src/editor/MarkdownEditor.tsx`
- `assets/reader/src/PlainTextEditor.tsx`
- `assets/reader/src/reader.css` / `assets/reader/src/editor/editor.css`

改动方向：

- 检查当前 Markdown 编辑器、纯文本编辑器、代码编辑器的滚动容器。
- 让外层编辑区域负责纵向滚动，内层正文只负责最大宽度和排版。
- Markdown 正文继续居中并保持最大宽度，但 scrollbar 应贴在中央编辑 pane 的最右边。
- 纯文本/代码 textarea 继续 `w-full h-full`，确保自身 scrollbar 在 pane 最右边。
- 如需保留正文两侧留白，使用 padding 而不是让窄容器承担滚动。

### 4. 构建与检查

完成后运行：

```bash
npm run build
cargo fmt
cargo clippy -- -D warnings
```

并检查：

- 前端 dist hash 文件变化。
- TypeScript 构建错误。
- Rust clippy warning。

## 暂不做项

- 当前浏览器 Reader 暂不实现 WebSocket PTY terminal。
- `Command + J` 终端放入后续 `jcli GUI / Tauri` 计划中，通过 Tauri IPC / Channel + PTY 实现。
