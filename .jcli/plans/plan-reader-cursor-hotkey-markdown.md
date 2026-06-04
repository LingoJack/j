# Reader 修复计划：光标跳回 + 快捷键 + Markdown 渲染

## 问题分析

### 问题 1：输入时光标跳回顶部

**根因**：`MarkdownEditor.tsx` 的 `renderBlocks` 函数在 debounce 300ms 后端解析完成后，会移除所有非活跃 block DOM 再重新创建并通过 `DocumentFragment` 一次性追加回 `host`。由于 `host`（`.md-editor`）自身就是 `overflow-auto` 的滚动容器，所有子节点被移除再重新添加会导致 `scrollTop` 归零。

**修复**：在 `renderBlocks` 的 DOM 操作前后保存/恢复 `host.scrollTop`。

**文件**：`assets/reader/src/editor/MarkdownEditor.tsx`
- 在 `renderBlocks` 函数中，DOM 操作前 `const savedScrollTop = host.scrollTop`
- 在 `host.appendChild(fragment)` 之后 `host.scrollTop = savedScrollTop`

---

### 问题 2：快捷键提示与实际不一致

**现状**：代码逻辑（`Reader.tsx:487`）已经正确使用 `e.altKey`（即 Option/Alt），但 `EmptyState` 帮助面板显示的是 `⇧`（Shift）而不是 `⌥`（Option），导致用户按 Shift 无效。

**修复**：更正 `EmptyState` 中的快捷键提示，从 `⇧` 改为 `⌥`。

**文件**：`assets/reader/src/Reader.tsx`
- EmptyState 里 `⌘ ⇧ ← / →` 改为 `⌘ ⌥ ← / →`

---

### 问题 3：Markdown 渲染应隐藏语法标记

**现状**：`inline-renderer.ts` 在渲染行内元素时，会同时显示 markdown 语法标记（`**`、`*`、`` ` ``、`~~`、`[text](url)`），虽然已通过 `.md-marker` 样式变暗但仍然可见。Heading 前的 `# ` 也是可见的。这不是典型的 Markdown 预览效果。

**修复**：通过 CSS 隐藏所有 markdown 语法标记，让编辑器呈现为富文本渲染效果。同时调整代码块的语言标签，去掉反引号只显示语言名。

**文件及改动**：

1. `assets/reader/src/editor/editor.css`
   - `.md-marker` 添加 `display: none`（隐藏所有行内语法标记和 heading `# ` 前缀）
   - `.md-code-fence-end` 添加 `display: none`（隐藏代码块结尾的 ` ``` `）

2. `assets/reader/src/editor/MarkdownEditor.tsx`
   - `createCodeBlockElement`：语言标签从 `'```' + lang` 改为只显示语言名 `lang`（反引号通过 marker 隐藏）

---

## 实施步骤

1. 修改 `MarkdownEditor.tsx` — 修复 scroll 跳动 + 代码块标签
2. 修改 `editor.css` — 隐藏 markdown 语法标记
3. 修改 `Reader.tsx` — 修正快捷键提示
4. 构建前端 + cargo 检查
