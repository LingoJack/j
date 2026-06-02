# Reader CSS → Tailwind CSS v4 全量迁移方案

## 目标

将 `assets/reader/src/reader.css` 中**所有**原生 CSS 规则迁移为 Tailwind CSS v4 utilities，最终 reader.css 仅保留 `@import 'tailwindcss'`、`@theme` 和 `@keyframes`（或 Tailwind 原生动画），不含任何 `.seeyue-*` 组件类或全局样式规则。

## 迁移策略

### 1. 全局基线 → Tailwind `@layer base`

全局 html/body/selection/scrollbar/textarea 样式通过 Tailwind v4 的 `@layer base` 内联，不再写原生 CSS 规则。

### 2. 动画 → `@theme` 注册

在 `@theme` 中注册 3 个自定义动画，让 Tailwind 自动生成 `animate-seeyue-*` 类：

```css
@theme {
  --animate-seeyue-fade-in: seeyue-fade-in 0.18s ease;
  --animate-seeyue-scale-in: seeyue-scale-in 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  --animate-seeyue-slide-in: seeyue-slide-in 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}
```

### 3. Markdown prose → Tailwind `@apply` 或组件内联

`.seeyue-prose` 及其子选择器（blockquote、table、code、hr、link）：
- 简单样式直接写在对应 React 组件的 `className` 上
- 深层嵌套选择器（如 `blockquote`、`table tr:nth-child`）使用 Tailwind 的 `@utility` + `@apply` 或 `@layer components` 组合

### 4. 伪元素 → Tailwind variant

所有 `::before`/`::after` 伪元素用 Tailwind variant：
- `before:content-['']` `before:absolute` `before:bg-seeyue-*` 等
- 例如 `.seeyue-tab[data-active='true']::after`（底部下划线）→ `after:content-[''] after:absolute ... data-[active=true]:after:bg-seeyue-accent-strong`

### 5. 父子联动 → `group`/`group-*` 变体

如 `.seeyue-tree-row:hover .seeyue-tree-caret`：
- 父元素加 `group`
- 子元素用 `group-hover:text-seeyue-success`

### 6. 复杂 data 属性 → `data-[name=value]:` 变体

如 `data-[tone=primary]:bg-seeyue-accent-strong`、`data-[op=add]:bg-[rgba(163,190,140,0.14)]`

## 实施步骤

### Step 1: 更新 reader.css 骨架

将 reader.css 精简为：

```css
@import 'tailwindcss';

@theme {
  /* 现有颜色/字体变量保持不变 */

  /* 新增：动画 */
  --animate-seeyue-fade-in: seeyue-fade-in 0.18s ease;
  --animate-seeyue-scale-in: seeyue-scale-in 0.18s cubic-bezier(0.16, 1, 0.3, 1);
  --animate-seeyue-slide-in: seeyue-slide-in 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

/* 全局基线移到 @layer base */
@layer base {
  html, body, #reader-root { height: 100%; }
  body { margin: 0; ... }
  ::selection { ... }
  ::-webkit-scrollbar { ... }
  textarea.seeyue-textarea { ... }
}

/* keyframes 保留（Tailwind v4 暂不支持在 @theme 内定义 keyframes） */
@keyframes seeyue-fade-in { ... }
@keyframes seeyue-scale-in { ... }
@keyframes seeyue-slide-in { ... }
```

### Step 2: 组件文件逐个迁移 className

每个 TSX 文件中的 `className="seeyue-*"` 替换为对应的 Tailwind utility 字符串。

#### 涉及文件清单：

| # | 文件 | 主要 CSS 类 | 迁移要点 |
|---|------|------------|---------|
| 1 | `Reader.tsx` | `seeyue-editor-bar`, `seeyue-icon-btn`, `seeyue-btn`, `seeyue-empty`, breadcrumb/status-pill | editor bar 内 `.breadcrumb`/`.crumb`/`.status-pill` 也需内联 |
| 2 | `TabBar.tsx` | `seeyue-tabbar`, `seeyue-tabbar-empty`, `seeyue-tabbar-empty-hint`, `seeyue-tabbar-quit`, `seeyue-tab-pill`, `tab-name`, `tab-dirty`, `tab-close`, `tab-dirty-mark` | tab-pill 内子元素全部内联；`::after` 底线用 `after:` variant |
| 3 | `FileTree.tsx` | `seeyue-sidebar-shell`, `seeyue-search-box`, `seeyue-tree-row`, `seeyue-tree-caret`, `seeyue-tree-icon`, `seeyue-tree-label`, `seeyue-tree-action`, `seeyue-tree-branch` | tree-row 加 `group`；子元素用 `group-hover:`/`group-focus-within:`；tree-branch 的 `::before` 层级线用 `before:` variant |
| 4 | `ActivityBar.tsx` | `seeyue-activity-bar`, `seeyue-activity-item` | activity-item `::before` 左侧高亮条用 `before:` variant |
| 5 | `TableOfContents.tsx` | `seeyue-toc-shell`, `seeyue-toc-list`, `seeyue-toc-rail`, `head`, `title`, `row` | head 内 `.title::after` 下划线用 `after:` variant |
| 6 | `Toast.tsx` | `seeyue-toast`, `seeyue-toast-icon`, `seeyue-toast-msg`, `seeyue-toast-close` | data-tone 变体控制 border-left 颜色 |
| 7 | `Splitter.tsx` | `seeyue-splitter`, `grip` | `::before` 线条 + grip 用 `before:` variant |
| 8 | `Toolbox.tsx` | `seeyue-toolbox-row`, `seeyue-toolbox-icon`, `seeyue-toolbox-text`, `name`, `desc` | group hover 控制 icon brightness |
| 9 | `DiffTool.tsx` | `seeyue-diff-tool`, `seeyue-diff-toolbar`, `seeyue-diff-inputs`, `pane`, `pane-head`, `dot`, `seeyue-diff-result`, `result-head`, `result-empty`, `result-grid`, `diff-row`, `cell`, `lineno`, `marker`, `text` | data-op 变体控制行背景色 |
| 10 | `JsonTool.tsx` | `seeyue-json-tool`, `seeyue-json-toolbar`, `seeyue-json-body`, `seeyue-json-input`, `seeyue-json-tree`, `pane-head`, `hint`, `tree-scroll`, `json-empty`, `json-error`, `json-node`, `json-row`, `caret`, `json-key`, `json-colon`, `json-bracket`, `json-summary`, `json-close-row`, `json-children`, `json-leaf`, `json-leaf-*`, `json-leaf-input`, `seeyue-json-toast` | 量最大，需仔细处理每一行 |
| 11 | `ImageViewer.tsx` | `seeyue-image-viewer`, `image-stage`, `image-error`, `image-statusbar`, `filename`, `sep`, `mime` | image-stage 棋盘格背景用 arbitrary value |
| 12 | `CloseConfirmDialog.tsx` | `seeyue-modal-mask`, `seeyue-modal`, `seeyue-modal-actions`, `seeyue-btn` | modal 内 h3/p 样式也需内联 |
| 13 | `QuitConfirmDialog.tsx` | 同上 | |
| 14 | `PromptDialog.tsx` | 同上 + `seeyue-modal-error`, `input[type='text']` | input 样式用 Tailwind utilities |
| 15 | `PlainTextEditor.tsx` | `seeyue-textarea` | textarea 全局基线已在 `@layer base` 处理 |
| 16 | `MarkdownIR.tsx` | 无 CSS 类 | 不需改动 |

### Step 3: Markdown 渲染 prose 样式处理

`.seeyue-prose` 样式用于 Markdown 渲染结果（通过 Milkdown/ProseMirror），有两种方案：

**方案 A（推荐）：保留在 `@layer components`**

极少的 prose 排版样式保留在 reader.css 的 `@layer components` 块中，这是 Tailwind 官方推荐的自定义组件层，不算"原生 CSS"，因为 Tailwind v4 会正确处理层叠优先级。

**方案 B：用 `@utility` 逐一注册**

用 Tailwind v4 的 `@utility` 语法注册 prose 子选择器为具名 utility。

### Step 4: 删除所有 `.seeyue-*` 组件 CSS 规则

迁移完成后，reader.css 中不再有任何 `.seeyue-*` 规则。

### Step 5: 构建验证

```bash
cd assets/reader && npm run build
```

## 复杂场景处理方案

| 场景 | Tailwind 解决方案 |
|------|------------------|
| `.seeyue-tree-branch::before` 层级线 | `className="relative"` + `before:content-[''] before:absolute before:top-0 before:bottom-0 before:w-px before:bg-seeyue-border-dim"` |
| `.seeyue-tab[data-active='true']::after` 底线 | `className="relative ... after:content-[''] after:absolute after:left-0 after:right-0 after:bottom-[-1px] after:h-[3px] after:rounded-[1.5px] after:bg-seeyue-accent-strong after:transition-all after:duration-400 data-[active=true]:after:content-['']"` |
| `.seeyue-activity-item[data-active='true']::before` 左侧条 | `className="relative ... before:content-[''] before:absolute before:-left-1 before:top-1.5 before:bottom-1.5 before:w-0.5 before:rounded-sm before:bg-seeyue-accent-strong data-[active=true]:before:content-['']"` |
| `.seeyue-splitter::before` + `:hover::before` | `className="relative ... before:content-[''] before:absolute before:top-0 before:bottom-0 before:left-0.5 before:w-px before:bg-seeyue-border before:transition-colors before:duration-150 hover:before:bg-seeyue-accent active:before:bg-seeyue-accent"` |
| `.seeyue-empty .glyph` 渐变背景 | `bg-[linear-gradient(135deg,rgba(94,129,172,0.12),rgba(163,190,140,0.08))]` |
| `.seeyue-image-viewer .image-stage` 棋盘格 | 保留为 arbitrary value 或提取到 `@layer components` |
| `.seeyue-codeblock::before` 三圆点 | `before:content-[''] before:block before:h-7 before:bg-seeyue-panel before:border-b before:border-seeyue-code-border before:bg-[radial-gradient(...)]` |
| `.seeyue-prose :not(pre) > code` 行内 code | 在 prose 容器上用 `not-pre:*` 或在 `@layer components` 中处理 |
| `.seeyue-prose table tbody tr:nth-child(even)` | 在 `@layer components` 中用 `[&_tbody_tr:nth-child(even)_td]:bg-[rgba(255,255,255,0.025)]` 或保留 prose 块 |
| `.diff-row .cell[data-op='del']` | `data-[op=del]:bg-[rgba(191,97,106,0.14)]` |
| `.json-leaf-string` | `text-seeyue-success`（直接写在 className 里） |

## 预期产出

- **reader.css**: 从 ~1579 行 → ~80 行（`@import` + `@theme` + `@layer base` + `@layer components`(prose only) + `@keyframes`）
- **所有 TSX 文件**: `className` 从引用 CSS 类名变为直接使用 Tailwind utilities
- **视觉效果**: 完全一致
