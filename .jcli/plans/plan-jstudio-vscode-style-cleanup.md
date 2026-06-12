# jstudio VS Code 风格清爽化计划

## 背景
用户希望 `apps/jstudio/` 的视觉不是只改「设置」弹窗，而是整体参考 VS Code 的清爽、克制、工具型产品风格。

当前已确认：
- jstudio 是 React + Tailwind + Vite/Tauri 应用。
- 主要 UI 文件包括：
  - `src/App.tsx`：整体框架、顶部栏、编辑区布局。
  - `src/components/DocumentList.tsx`：左侧文档列表与底部设置。
  - `src/components/LocalFolder.tsx`：右侧本地文件抽屉。
  - `src/components/ArticleOutline.tsx`：右侧文章大纲。
  - `src/components/BlockEditor.tsx`、`BlockItem.tsx`：编辑器和各种块样式。
- `../vscode/` 当前工作区只看到 `.git`，没有检出源码文件，无法直接读取 VS Code CSS/TS 实现；但可按 VS Code 的通用桌面 UI 语言执行：平面、低圆角、细分割线、状态色克制、无大阴影/动效/胶囊化。

## 当前问题
整体 UI 存在较多偏“花哨/卡片化/胶囊化”的样式：
- 大量 `rounded-xl/rounded-2xl`、`shadow-xl/2xl`、`backdrop-blur`、半透明玻璃效果。
- 交互反馈包含 `active:scale-*`、`hover:-translate-y-*`、`animate-pulse` 等偏动效化效果。
- 状态色较多：indigo、rose、amber、emerald 等在工具 UI 中出现频繁。
- 本地文件抽屉偏卡片列表，不像 VS Code Explorer 的紧凑列表。
- 顶栏和编辑器外壳有较强阴影、玻璃背景，不够 VS Code 式克制。

## 目标风格
以 VS Code 风格为方向：
- 背景层级清晰：`activity/sidebar/editor/panel` 通过背景和 1px border 区分。
- 控件矩形/低圆角：常用 `rounded-sm` 或 `rounded`，避免胶囊和大圆角。
- 菜单/列表平面化：hover 为浅背景，active 用左侧 2px 色条或浅选中底色。
- 减少阴影和玻璃效果：除 popover/浮层外，不使用大阴影。
- 减少动效：保留颜色过渡，移除 scale、translate、pulse 等。
- 颜色更克制：主色只用于选中/焦点，危险操作只在 hover 或图标上体现。
- 密度更高：侧边栏、文件列表、大纲更紧凑。

## 实施步骤

### 1. 全局框架：`App.tsx`
- 顶层背景改为更平的 VS Code 工作台色：浅色 `#f3f3f3/#ffffff`，暗色 `#1e1e1e/#181818`。
- 去掉外层 `shadow-2xl`、`backdrop-blur-xl`、`animate-in`。
- 顶部栏高度保持或微调为 35~40px，样式改为纯色背景 + 底部分割线。
- 顶栏按钮改为低圆角、hover 平面背景，去掉 `active:scale-*`。
- 应用名区域去掉 Sparkles 强强调，使用普通文字/文件图标。

### 2. 左侧文档列表：`DocumentList.tsx`
- 侧边栏背景调整为 VS Code sidebar 风格。
- 搜索框改为紧凑矩形输入框：低圆角、无 focus 大光圈，仅 border/focus outline。
- 文档项从大圆角胶囊改为紧凑列表行：`h-7/8`、小 padding、`rounded-sm` 或无圆角。
- active 状态改为浅背景 + 左侧 2px accent bar。
- section header 使用 VS Code Explorer 式 uppercase/小字/紧凑间距。
- 底部按钮保持图标工具栏风格，设置 popover 已改过，但可继续统一阴影/圆角。

### 3. 右侧本地文件面板：`LocalFolder.tsx`
- 抽屉从“浮动玻璃卡片”改成右侧 panel：低圆角或无圆角、细 border、无大阴影。
- 文件夹选择从三张卡片改为类似 VS Code Explorer 的树/列表行。
- 上传 dropzone 从大卡片改为紧凑 dashed row/toolbar 区。
- 文件项从大圆角卡片改为列表行，图片预览保持但弱化边框和阴影。
- 操作按钮改为 inline toolbar，小图标/文本按钮，移除 hover 上浮、pulse。

### 4. 大纲面板：`ArticleOutline.tsx`
- 改成 VS Code Outline view：平面 panel header + 紧凑 list rows。
- 当前项选中与 hover 统一使用低对比背景，不使用卡片阴影。

### 5. 编辑器区域：`BlockEditor.tsx` / `BlockItem.tsx`
- 编辑器背景、标题、元信息区域更接近 VS Code editor：简洁、留白适中。
- 块 hover 背景降低存在感，去掉大圆角。
- callout/code/table/html-render 等块减少大圆角和阴影：`rounded`、`border`、平面背景。
- Slash command menu 若存在，统一成 VS Code command palette/menu 风格：矩形 popover、列表行选中态。
- inline code/wiki link 保留功能，但减弱胶囊感，改低圆角/下划线或轻背景。

### 6. 统一清理
- 搜索并替换高风险视觉类：`rounded-2xl`、大量 `rounded-xl`、`shadow-2xl`、`shadow-xl`、`backdrop-blur-*`、`active:scale-*`、`hover:-translate-y-*`、`animate-pulse`。
- 保留必要浮层阴影，但降低到 `shadow-lg` 或 `shadow-md`。
- 保证暗色模式仍可读。

### 7. 验证
- 运行 `npm run build` 验证 TypeScript 和 Vite 构建。
- 如有 lint/format 脚本则运行对应命令；否则仅 build。

## 风险与注意
- 改动主要是 className 样式，不改业务逻辑。
- `BlockItem.tsx` 文件较大，需分段精准修改，避免误动沙盒示例内容里的 Tailwind 字符串。
- 重点改应用 UI 外壳和组件 className，不批量替换用户示例 HTML 字符串里的设计样式。
