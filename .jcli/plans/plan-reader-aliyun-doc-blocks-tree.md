# reader-aliyun-doc-blocks-tree

## 目标

用户进一步指出 Reader 与阿里云帮助文档页面在以下区域差异仍较大：

1. Markdown 代码块样式。
2. 引用块样式。
3. 列表样式。
4. 内联代码样式。
5. 左侧目录 / 文件树样式。

本次目标不是泛泛“变清爽”，而是更贴近阿里云帮助中心文档站的可读性和控件气质：白底、浅灰分割、浅色代码容器、橙色强调但不过度、列表层级清晰、文件树更像文档目录而不是 IDE 深色资源管理器。

## 当前状态

已确认：

- Tailwind CSS v4 正在使用。
- Markdown 主样式在 `assets/reader/src/editor/editor.css`。
- 全局 prose 兼容样式在 `assets/reader/src/reader.css` 的 `@layer components`。
- 文件树在 `assets/reader/src/FileTree.tsx`。
- 当前 `editor.css` 中：
  - `.md-code-wrap` 有浅色代码块，但仍有顶部栏 `::before`，像编辑器组件，不像阿里云文档的纯代码卡片。
  - `.md-code-lang` 在右上角，存在强 UI 感。
  - `.md-blockquote` 使用圆角、accent 背景和阴影，和阿里云常见的轻提示条/浅灰引用块差异较大。
  - `.md-list-item` 使用 flex，自定义间距较重，容易不像传统文档列表。
  - 内联代码使用紫色，阿里云文档更偏红/橙文字 + 浅暖灰背景。
- 当前 `reader.css` 中还保留旧 `.seeyue-codeblock` 深色 macOS 三圆点样式，需清理或改成同一阿里云文档风格，避免不同渲染路径样式不一致。
- 文件树当前 active 行是 `bg-seeyue-accent-strong`，过重；并且有右侧绿色指示条、目录连接线偏深，和清爽文档目录差异大。

## 实施方案

### 1. Markdown 代码块：改为阿里云文档式浅色代码容器

调整 `assets/reader/src/editor/editor.css`：

- `.md-code-wrap`
  - 背景使用 `#f7f8fa` / `var(--color-seeyue-code-bg)`。
  - 边框为 `#e5e7eb`。
  - 圆角减小到 4-6px。
  - 去掉 macOS/编辑器感顶部栏：移除或隐藏 `::before`。
  - margin 建议：`16px 0`，不要太卡片化。
- `.md-code-lang`
  - 改成非常弱的右上角文字，或者放在代码块内顶部 8px；不做 header bar。
  - 字号 12px，颜色浅灰，uppercase 可保留但不抢眼。
- `.md-code-pre`
  - padding 更接近文档站：`16px 20px`。
  - 行高 1.6-1.65。
  - 背景透明。
- `.md-code-content`
  - 字号 `13px` 或 `13.5px`。
  - 文本颜色 `#1f2937`。
- 语法高亮颜色保留丰富度，但略收敛为文档站配色，避免 VS Code 彩虹感过强。

同时调整 `assets/reader/src/reader.css` 里的 `.seeyue-codeblock`：

- 删除深色 macOS 三圆点视觉。
- 与 `.md-code-wrap` 保持同一浅色代码块样式。
- 这样自研编辑器和普通 prose 渲染路径风格一致。

### 2. 内联代码：阿里云文档式浅背景 + 橙红文字

调整：

- `editor.css` `.md-editor code`
- `reader.css` `.seeyue-prose :not(pre) > code`

目标：

- 背景：浅暖灰/浅橙灰，例如 `#fff4ec` 或 `rgba(255,106,0,0.08)`。
- 文字：`#d4380d` / `#c2410c`，不要紫色。
- 边框可选：1px solid `rgba(255,106,0,0.12)`。
- 圆角 3-4px。
- padding：`0 4px` 或 `1px 4px`，不要太厚。
- 字号 `0.9em`。

### 3. 引用块：从“强调卡片”改为文档提示/引用条

调整 `.md-blockquote` 和 `reader.css .seeyue-prose blockquote`：

- 背景：`#f7f8fa` 或 `rgba(22,119,255,0.04)`。
- 左边框：3px solid `#d8dde6` 或轻橙色 `rgba(255,106,0,0.45)`。
- 去掉阴影。
- 圆角减小到 4px。
- padding：`10px 16px`。
- 文本颜色接近正文，不要过淡；`#4b5563`。
- blockquote 内段落的 margin 收紧。

### 4. 列表：恢复文档站自然列表节奏

调整 `editor.css`：

- `.md-list`
  - `padding-left` 增加到 22-24px，`margin: 8px 0 16px`。
- `.md-list-item`
  - 减少 flex UI 感，保持自然文本列表：行高 1.8，margin 4px 0。
  - 若当前 DOM 必须 flex，则把 gap 降低，并确保 marker 与文字对齐。
- 嵌套列表 margin 缩小但层级清晰。
- task checkbox 更像文档复选框：尺寸 14px，accent 橙/蓝，margin-top 对齐首行。

如有 marker 元素可单独调整 marker 颜色为 `#6b7280`，active 不做卡片感。

### 5. 文件树 / 目录树：从 IDE 资源管理器改成阿里云文档目录风

调整 `assets/reader/src/FileTree.tsx`：

- 外层：
  - 背景保持 `#f7f8fa` 或 `#fafafa`。
  - 去掉内阴影。
  - 右边框轻灰即可。
- Header：
  - 不用粗 underline tab；改成小标题 + 操作按钮。
  - 标题可用 `EXPLORER` / `文件`，颜色中性。
- 搜索框：
  - 白底、浅灰边框、4px 圆角、聚焦橙色描边。
- 树节点：
  - 行高 28px 左右。
  - hover：`rgba(0,0,0,0.04)`。
  - active：不要橙色实底；改成浅橙背景 `rgba(255,106,0,0.08)` + 左侧 3px 橙色竖线或文字橙色。
  - 去掉右侧绿色指示条。
  - 目录连接线颜色调浅到 `#e5e7eb`。
  - 文件/目录 icon 保留但颜色弱化，active 时文字橙色。
- 面包屑分隔符从 `/` 可改为 chevron 或保持但更浅。

### 6. 验证

修改后运行：

- `npm run format:check`
- `npm run build`
- `cargo fmt`
- `cargo clippy -- -D warnings`

如需要也可运行 `npm run lint`，但项目仍有既有 lint 问题；重点确认本次没有 TS/build 错误。

## 预计改动文件

- `assets/reader/src/editor/editor.css`
- `assets/reader/src/reader.css`
- `assets/reader/src/FileTree.tsx`
- `assets/reader/dist/**`（build 生成）

## 注意事项

- 不改变 Markdown 编辑器 DOM / 保存逻辑，仅调整样式。
- 不再引入新的主题或依赖。
- Warm 主题仍应可用；默认 Aliyun Light 样式优先对齐用户给的阿里云文档参考。
