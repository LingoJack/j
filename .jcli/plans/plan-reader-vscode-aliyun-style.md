# reader-vscode-aliyun-style

## 目标

基于用户反馈，将 Reader UI 从当前偏暖、偏装饰化的风格，调整为：

1. **布局模仿 VS Code**：活动栏、侧边栏、Tab、编辑区、右侧 Outline 更接近 VS Code 的信息架构和空间关系。
2. **视觉参考阿里云文档**：整体更白、更清爽、更轻边框、更少阴影，正文以文档阅读为中心。
3. **底部增加设置入口**：在左下角/底部区域增加设置按钮，用于切换主题。
4. 保留已有改进：语义化 icon、TOC pin 不遮挡正文、文件类型 icon 区分。

## 已确认现状

- Reader 主体在 `assets/reader/src/Reader.tsx`。
- 当前布局是：`44px activity bar + sidebar + splitter + main`。
- 当前 TOC 已改成 pinned 时占位列：正文 + `248px` 右侧栏，不再遮挡。
- 当前视觉 token 在 `assets/reader/src/reader.css`，仍偏米白/赭陶，和阿里云文档的简洁白底风格不一致。
- Markdown 样式在 `assets/reader/src/editor/editor.css`，目前代码块仍是深色 macOS 风，整体比阿里云文档更“设计感”，但不够文档站清爽。
- ActivityBar 在 `assets/reader/src/ActivityBar.tsx`，目前只有 files/toolbox，没有底部设置入口。
- TabBar、FileTree、EditorBar 目前已做过视觉优化，但还不是 VS Code / 阿里云文档方向。

## 设计方向

### 1. VS Code 式布局骨架

保持并强化以下布局：

```text
┌────────────── App Shell ──────────────┐
│ Activity │ Explorer │ Tabs            │
│ Bar      │ Sidebar  │ Breadcrumb/Bar  │ Outline
│          │          │ Editor/Preview  │
│          │          │                 │
│ Settings │          │                 │
└───────────────────────────────────────┘
```

具体调整：

- `Reader.tsx` 根 grid 继续采用：ActivityBar / Sidebar / Splitter / Main。
- Main 内部保持：TabBar → EditorBar → Content。
- TOC pinned 时作为右侧 Outline column；未 pinned 时可以继续浮层，但视觉改为轻量白底 outline。
- ActivityBar 底部增加 Settings 按钮，符合 VS Code 左下角设置入口习惯。

### 2. 阿里云文档式清爽视觉

将主题改成更接近阿里云文档的文档站风格：

- 主背景：`#ffffff` / `#f7f8fa`。
- 侧栏背景：`#f7f8fa` 或 `#fafafa`。
- 边框：`#e5e7eb` 一类浅灰线。
- 主文本：`#1f2937` / `#111827`。
- 次级文本：`#6b7280`。
- 链接 / active：阿里云橙蓝体系可选：
  - 主 accent：`#ff6a00`（阿里云橙）
  - 链接蓝：`#1677ff`
- 减少大阴影、大渐变、拟物卡片。
- EmptyState 去掉重装饰渐变，改成轻量卡片或直接文档入口提示。
- Markdown 正文更像文档站：
  - 行宽约 860-960px。
  - H1/H2 清晰但不夸张。
  - 表格白底、浅灰边框。
  - blockquote 使用浅蓝/浅橙提示条。
  - code block 从“深色 macOS 终端”改为更文档站的浅色代码块，或保留深色但减弱阴影。建议本次改为浅色代码块，和阿里云文档更一致。

### 3. 增加设置入口与主题切换

新增设置入口，既满足当前主题切换，也为后续更多 Reader 设置预留位置：

- 在 `ActivityBar.tsx` 底部放一个 Settings / Gear 按钮。
- 点击后打开轻量 Popover 或 Dialog（建议先实现 popover，复杂度低）。
- Popover 结构按“设置中心”设计，而不是只做一个临时主题按钮：
  - 分组标题：`外观`
  - 当前选项：主题切换
  - 后续可继续追加：字体大小、编辑/预览模式、自动保存、侧栏行为、TOC 默认固定等。
- 当前先实现主题选项：
  - `Aliyun Light`：默认，新清爽文档风。
  - `Seeyue Warm`：保留当前暖色主题，避免用户想回退。
- 使用 `localStorage` 持久化：`jreader.theme`。
- 在 `Reader.tsx` 管理主题状态，并在根容器加 `data-theme="aliyun" | "warm"`。
- 在 `reader.css` 中用 CSS 变量 + `[data-theme='warm']` 覆盖实现主题切换。
- 默认主题建议设为 `aliyun`，响应用户现在的偏好。

实现方式：

- 新增类型：`type ReaderTheme = 'aliyun' | 'warm'`。
- 新增常量：`THEME_LS_KEY = 'jreader.theme'`。
- `Reader.tsx`：
  - 初始化读取 localStorage。
  - `setTheme` 时写 localStorage。
  - 根容器加 `data-theme={theme}`。
  - 传给 `ActivityBar`：`theme`, `onThemeChange`。
- `ActivityBar.tsx`：
  - 增加 Settings icon（如果 `Icon.tsx` 没有就新增 `Settings`）。
  - 内部管理 popover open 状态。
  - 底部使用 `mt-auto`。
- `reader.css`：
  - 默认 token 改为 Aliyun Light。
  - `[data-theme='warm']` 覆盖为当前暖色 token。

### 4. 组件具体调整

#### `ActivityBar.tsx`

- 改为 VS Code 风：窄栏、纯色/浅灰背景、按钮更方正。
- 顶部 files/toolbox。
- 底部 settings。
- settings popover 向右上弹出，避免超出底部。

#### `FileTree.tsx`

- Explorer 风格：
  - Header 改为 `EXPLORER` / `文件` 小标题。
  - 搜索框白底浅边框。
  - hover 为浅灰，active 为浅橙/浅蓝背景 + 左侧细条。
  - 减少阴影和渐变。

#### `TabBar.tsx`

- 更像 VS Code：
  - tab 高度 35-36px。
  - active tab 白底，顶部/底部 accent 细线。
  - inactive tab 浅灰底。
  - close icon hover 更克制。

#### `EditorBar` in `Reader.tsx`

- 更像 VS Code breadcrumb + 文档工具栏：
  - 白底或极浅灰。
  - 底边框。
  - 保存/复制按钮为 icon button，hover 浅灰。

#### `TableOfContents.tsx`

- pinned：右侧 Outline 栏，白底/浅灰，标题可叫 `OUTLINE` 或 `目录`。
- active heading 用左侧橙色细线 + 轻背景。
- floating：白底轻边框，不使用厚阴影。

#### `editor/editor.css`

- 主体改阿里云文档风：
  - `.md-editor` max-width 920px，padding 更文档化。
  - heading 使用文档站节奏。
  - inline code 使用浅灰底 + 红/橙色文字。
  - code block 改浅色块：浅灰背景、浅边框、无 macOS 三点或减弱。
  - table 更清晰：thead 浅灰背景、边框统一。

## 验证

修改后运行：

- `npm run format:check`
- `npm run lint`
- `npm run build`
- `cargo fmt`
- `cargo clippy -- -D warnings`

注意：之前 `npm run lint` 有既有 lint 项，需要区分是否由本次改动引入；本次应尽量不新增 lint warning。

## 预计改动文件

- `assets/reader/src/Reader.tsx`
- `assets/reader/src/ActivityBar.tsx`
- `assets/reader/src/Icon.tsx`
- `assets/reader/src/FileTree.tsx`
- `assets/reader/src/TabBar.tsx`
- `assets/reader/src/TableOfContents.tsx`
- `assets/reader/src/reader.css`
- `assets/reader/src/editor/editor.css`
- `assets/reader/dist/**`（build 生成）

## 风险

- 主题切换如果用 Tailwind class 写死颜色，会导致切换不完整；尽量依赖 CSS variables。
- TOC fixed column 需继续保持不遮挡正文。
- 不改变 Markdown 编辑器核心逻辑，只改布局和样式。
