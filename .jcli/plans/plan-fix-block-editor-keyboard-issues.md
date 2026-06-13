# 修复 BlockEditor 键盘交互问题（v2）

## 问题总结
1. 上下箭头无法在块之间移动
2. 删除键删不了一些东西
3. 行内代码一旦写了就出不来

## 根因分析

### 问题 1: 上下箭头无法移动
`isCaretOnEdgeLine` 检测逻辑依赖 `getBoundingClientRect()` 比较 caret 和元素的边界，但 contentEditable 中可能存在 `<br>`、内联元素等导致 `elRect` 的高度包含多行，而 `slack` 容差可能不够精确。需要更可靠的检测方法。

### 问题 2: Backspace 删不掉
当 contentEditable 内部含有 `<code>`/`<b>` 等 HTML 标签（由 `handleBlur` 的 Markdown 自动转换产生），光标位置检测 `preCaretRange.toString().length === 0` 可能误判。例如：
- 内容为 `<code>text</code>`，光标在 `<code>` 标签之前时，`preCaretRange.toString()` 可能为空
- 此时会 `preventDefault()` 并触发块合并，而非执行正常删除

### 问题 3: 行内代码出不来
`handleBlur` 不可逆地将 `` `code` `` 转换为 `<code>` HTML 标签。一旦转换：
- 用户无法通过 Backspace 删除 `<code>` 标签本身
- 光标被困在 `<code>` 标签内部
- 虽然代码中已有 `tryEscapeInlineFormat` 和 Backspace 删除 inline 元素的逻辑，但实际执行中这些保护不够完善

## 修复方案

所有修改集中在一个文件 `apps/jstudio/src/components/BlockItem.tsx`。

### 修复 1: 改进 `isCaretOnEdgeLine`

将 caret 位置与元素内**各行**的位置比较，而不是简单地比较 `caretRect.top` 和 `elRect.top`：
- 使用 `getClientRects()` 获取元素内所有行盒矩形
- "向上"判断：caret 所在行与第一行一致时返回 true
- "向下"判断：caret 所在行与最后一行一致时返回 true
- 增大 slack 容差值（从 `lineHeight/2` 到 `lineHeight * 0.75`）

### 修复 2: 修复 Backspace 删除逻辑

重构 Backspace 处理：
- 检测光标是否在**整个元素文本内容的最开头**（不只是 `preCaretRange.toString().length === 0`），还需要确认**内容确实为空或几乎为空**时才触发块合并
- 具体：当 `preCaretRange.toString().length === 0` 且 `el.innerText.trim().length === 0` 时才合并块；否则让浏览器的默认 Backspace 行为执行
- 改进 inline 格式元素的删除逻辑：当光标紧挨 `<code>` 等元素之前时，Backspace 应该选中并删除整个元素

### 修复 3: 修复行内代码"出不来"的问题

核心策略：保持 `handleBlur` 的自动格式化（所见即所得），但大幅改进 inline 格式元素的编辑体验：

1. **改进 `tryEscapeInlineFormat`**：确保右箭头键能可靠地将光标从 `<code>` 内部移出到外面
2. **改进 Backspace 删除 inline 元素的逻辑**（已有的 Case 1）：当前逻辑只在光标在元素末尾时删除整个元素，扩展为：
   - 光标在元素末尾 → 删除整个元素（已有）
   - 光标紧挨在元素之前 → 选中并删除整个元素（新增）
   - 元素内容为空时 → 删除空元素（新增）
3. **改进 `handleBlur`**：格式化时保留光标周围的反引号/星号标记（如果光标正位于未完成的格式标记中，不做转换）

## 修改文件
- `apps/jstudio/src/components/BlockItem.tsx`
