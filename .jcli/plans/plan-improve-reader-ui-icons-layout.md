# improve-reader-ui-icons-layout

## 目标

优化 `j read` Reader UI，重点解决三类问题：

1. 图标语义混乱、标识度不足：文件、复制、目录、工具、导航等图标需要有明确区分。
2. 右侧目录（TOC）固定后遮挡正文：固定态应参与布局或给内容留出稳定空间，不能永久覆盖内容。
3. 整体视觉质感不足：统一设计语言，提升层次、留白、状态反馈和阅读舒适度。

## 已确认现状

- Rust 入口在 `src/command/read.rs`，主要负责启动 reader server 和浏览器；本次改动主要集中在 `assets/reader/src`。
- Reader 主布局在 `assets/reader/src/Reader.tsx`：当前 grid 为 `44px ${sidebarWidth}px 5px 1fr`，TOC 渲染在主内容区内部。
- TOC 组件在 `assets/reader/src/TableOfContents.tsx`：
  - 未固定时使用 absolute hover 展开。
  - 固定时仍返回 `TocPanel`，而 `TocPanel` 的基础 class 仍是 `absolute right-0 top-0 ...`，因此会覆盖内容。
- 图标集中在 `assets/reader/src/Icon.tsx`，已存在文件类型、复制、目录、工具等 SVG，但部分使用场景和视觉区分仍可强化。
- 文件树在 `assets/reader/src/FileTree.tsx`，Tab 在 `assets/reader/src/TabBar.tsx`，顶部编辑器栏在 `Reader.tsx` 的 `EditorBar`。
- 样式入口为 `assets/reader/src/reader.css` 和 `assets/reader/src/editor/editor.css`，Tailwind v4 utilities + CSS 变量。
- 前端构建命令在 `assets/reader/package.json`：`npm run build`，另有 `npm run lint`、`npm run format:check`。

## 实施方案

### 1. 右侧 TOC 固定态改为不遮挡正文

- 在 `Reader.tsx` 中将主内容区域从“内容 + absolute TOC”改为可根据 `tocPinned` 动态分栏：
  - 未固定：保持现有 hover/focus 浮层体验，TOC 作为 overlay 小胶囊入口，不占空间。
  - 固定：使用 `grid` 或 `flex` 让编辑区与 TOC 成为两列，右侧 TOC 占固定宽度（建议 248px 左右），中间内容区域自动缩窄。
- 给固定态 TOC 使用 `relative/sticky` 样式，而不是 `absolute`。
- `TableOfContents.tsx` 调整为支持 `mode` / `pinned` 下的不同 panel class：
  - pinned：`relative w-[248px] h-full max-h-none ...`，边界明确，滚动在 TOC 内部。
  - floating：继续使用 `absolute right-0 top-0 ...`。
- 保留 localStorage 的 `jreader.tocPinned` 行为。
- 小屏幕或空间不足时可降级：固定态仍显示为右侧窄列，但设置合理 `minmax(0,1fr)`，避免正文被挤爆。

### 2. 图标体系语义化和视觉差异化

- 在 `Icon.tsx` 中补充/替换更明确的图标：
  - 复制路径：使用 `Copy` 双矩形图标，并可新增 `ClipboardCopy` 或 `CopyPath`（剪贴板+路径线）强化“复制路径”含义。
  - 普通文件：避免与复制相似，使用单页折角 `FileGeneric`，复制保持双页/剪贴板。
  - 文件树入口：继续 `Files`，但与复制保持明显差异。
  - TOC：`ListTree` 可保留，但固定/取消固定建议统一迁入 Icon.tsx，避免组件内自定义散落。
  - 可补充 `PanelRight`, `PanelRightClose`, `Pin`, `PinOff`，让导航动作更直观。
- 审核使用点：`FileTree.tsx`、`TabBar.tsx`、`ActivityBar.tsx`、`Reader.tsx/EditorBar`、`TableOfContents.tsx`。
- 强化文件类型图标差异：Markdown、文本、代码、图片、通用文件在 path 与颜色上都区分，不只靠颜色。
- 为按钮补齐 `aria-label`，避免只有 title。

### 3. 整体 UI 视觉优化

- 统一视觉方向：保留当前米白暖色底，但降低“浑浊感”，提升清爽度和层次。
- 调整设计 token：
  - 背景/侧栏/面板层级更克制，减少大面积深阴影。
  - accent 保持赭陶色，但 hover/active 使用更细腻的 soft background。
  - border 使用更透明、更一致的层级。
- 布局细节：
  - 左侧 FileTree 去掉过重的 `shadow-[0_0_12px_rgba(0,0,0,0.3)]`，改为轻边界/轻内阴影。
  - TabBar active 状态更像文档标签：细顶/底强调线 + 背景过渡。
  - EditorBar 增强路径层次和按钮 affordance，复制路径与保存按钮更易识别。
  - EmptyState 改为更雅致的卡片/插画式提示，避免廉价渐变。
- Markdown 阅读区：
  - `.md-editor` 根据 TOC 固定态自然缩窄后仍居中，保持最大行宽。
  - heading、blockquote、code block、table 的间距和圆角统一。

### 4. 构建产物与验证

- 修改源码后，在 `assets/reader` 下运行：
  - `npm run format:check`（或必要时先 `npm run format`）
  - `npm run lint`
  - `npm run build`
- 因 `dist/` 会被 Rust embed 打包，确认 `npm run build` 更新 `assets/reader/dist`。
- 回到仓库根目录运行：
  - `cargo fmt`
  - `cargo clippy -- -D warnings`
- 如 Rust 未改动，仍建议跑 clippy 以满足项目约束。

## 预计改动文件

- `assets/reader/src/Icon.tsx`
- `assets/reader/src/TableOfContents.tsx`
- `assets/reader/src/Reader.tsx`
- `assets/reader/src/FileTree.tsx`
- `assets/reader/src/TabBar.tsx`
- `assets/reader/src/ActivityBar.tsx`（如需微调活动栏视觉）
- `assets/reader/src/reader.css`
- `assets/reader/src/editor/editor.css`
- `assets/reader/dist/**`（由 build 生成）

## 风险与注意事项

- TOC 改为固定列后，必须确保编辑器滚动、TOC active heading observer 和 smooth scroll 不受影响。
- 不在 TUI 相关 Rust 代码中增加 stdout/stderr 输出。
- 避免对 Markdown 编辑核心逻辑做大改，本次主要是布局和视觉层。
- SVG icon 继续使用 currentColor，保持主题可控。
