# jstudio 编辑器键盘交互修复

## 背景

`apps/jstudio`（OmniNote Tauri 应用）的 Block 编辑器存在三个键盘交互问题：

1. **上下无法移动**：上下方向键无法在文档标题、块与块之间正确移动光标。
2. **删除也删不了一些东西**：自动格式化后的内联元素（行内代码、Wiki 链接、加粗）无法通过 Backspace 干净地删除；空块按 Backspace 也不一定删得掉。
3. **行内代码一旦写了就出不来**：在文本块里输入 ``` `code` ``` 后失焦会被自动转成 `<code>code</code>`，再点回来光标容易困在 `<code>` 内，没有键盘快捷方式可以"逃出"。

## 根因分析

### 1. 上下移动失效
- `BlockEditor.tsx:199-205` 文档标题 `<input>` 没有 `data-block-editable="true"`，也没有 keydown 处理器。从标题按 ↓ 完全无响应。
- `BlockItem.tsx:500-553` `isCaretOnEdgeLine` 对 contentEditable 走的是 `getClientRects()` 路径，依赖极小数（3px 阈值），对空块（`<br>`/`<div><br></div>`）和小尺寸元素不稳定。
- 对 textarea（callout / code / toggle）：当前用 `value.indexOf("\n")` 判定首尾行，逻辑基本可用，但同样没有暴露机制让标题 ↔ 块 互通。

### 2. 删除失效
- `BlockItem.tsx:625-647` 对 contentEditable 仅判断"光标前文本长度为 0"才删块。但当内容已格式化为 `<code>code</code>` 时，光标放在 `</code>` 之后时 `preCaretRange.toString()` 是 `"code"`（长度 4），不满足条件，Backspace 只能逐字符删 `<code>` 内的字符，删不掉整个内联元素，更不会触发"合并到上一块"。
- 没有"清空整个块内容"的快速键，也没暴露"合并到上一行"的入口。

### 3. 行内代码困人
- `BlockItem.tsx:654-678` `handleBlur` 用正则把 ``` `foo` ``` 转成 `<code class="...">foo</code>`，但写入 `rawText` 后下次进入时光标位置由浏览器决定，常常落在 `<code>` 内部。
- 没有任何 ArrowRight / ArrowLeft 处理器来"跳出"内联元素。删除 `<code>` 也只能逐字符。

## 修复方案

### A. 让标题与块之间可上下移动
- `BlockEditor.tsx`
  - 给标题 `<input>` 增加 `data-block-editable="true"` 和 `id="title-input"`，并通过 `useRef` 拿到 DOM 引用。
  - 加一个 `onKeyDown`：
    - `ArrowDown`（在标题末尾）→ 找到第一个 block 的 `[data-block-editable="true"]` 并 focus。
    - `Enter`（标题非空）→ 自动插入并 focus 到第一个新文本块。

### B. 强化块的上下移动
- `BlockItem.tsx`
  - 重写 `isCaretOnEdgeLine`：
    - contentEditable 走 `Range.getBoundingClientRect()` + 容器 `getBoundingClientRect()` 比较 top/bottom（更稳定，不依赖 `getClientRects()` 枚举）。
    - 兼容空块（用 zero-width 探针 `Range#insertNode` 取坐标后撤销）。
    - 多行容器以"光标到容器顶/底的距离 ≤ 一行行高的一半"为边线判据。
  - 改 `moveFocusToSiblingBlock` 允许从"块之后"再走一步到标题（向上方向 + 第一个块的兄弟是标题时）。具体做法：在 BlockEditor 暴露 `focusBlock(blockId)` 回调，块向上若无兄弟则回调上层跳标题。
  - textarea 路径保持，但同样支持上一条规则的回退。

### C. Backspace 强化
- `BlockItem.tsx`
  - 对 contentEditable，新增"光标紧贴在前一个内联元素之后"判定：当前一个 sibling 是 `<code>` / `<a>` / `<b>` 且光标在它之后 → Backspace 一次删掉整段内联元素（保留其后可能存在的内容）。
  - 对 contentEditable，若整个块只含一个内联元素且光标在它之前 → Backspace 触发 `onDeleteBlock("")` 合并/删除（与现有"空块"行为一致）。
  - 新增快捷键 `Cmd/Ctrl+Backspace` → 无论光标位置，直接调用 `onDeleteBlock("")` 合并到上一块（与 Notion / 飞书一致）。
  - textarea（code/callout/toggle）：保持现状，行为正确；额外在内容为空时 Backspace 也走 `onDeleteBlock`。

### D. 行内代码能"逃出"
- `BlockItem.tsx`
  - 在 `handleKeyDown` 里增加 `ArrowRight` 处理：
    - 若当前 `Range` 在一个内联元素（`<code>` / `<a>` / `<b>`）之内，且 `Range.collapsed` 且 `Range.endOffset === node.childNodes.length`（即在该内联元素末尾）→ 把光标移到该内联元素的父节点中、紧跟其后。
  - 同理 `ArrowLeft`：光标在该内联元素最前面 → 移到父节点、紧靠其前。
  - 这样无论用户怎么点，都能用方向键干净地进出 `<code>`。

### E. 自动格式化的容错
- `BlockItem.tsx`
  - `handleBlur` 中，把 `**...**` / ``` `...` ``` / `[[...]]` 替换前先判断"是否会被错误地吃掉有效文本"：当原始 `rawText` 里同时存在裸字符 `<` `>` 时不强行替换（避免误伤）。原正则保持。
  - 如果格式化产生 HTML 变化，**保存前** 把光标位置（用 `Range` 序列化/offset）记下来；保存后用 `setRawText(formatted)` 触发的 `useEffect` 重新设置光标到等价的文本位置。这一步是体验关键：避免用户写完代码后失焦，再点回来时光标"乱飞"。

## 涉及文件

- `apps/jstudio/src/components/BlockEditor.tsx`：标题 ref / keydown / 暴露 `focusBlock`。
- `apps/jstudio/src/components/BlockItem.tsx`：所有键盘逻辑与 `ContentEditableBlock` 微调（重写上下行判定、新增 Backspace 与方向键逃出、cursor 位置保留）。

## 验证方式

1. `cd apps/jstudio && npm run lint`（`tsc --noEmit`）必须通过。
2. 手动测试用例：
   - 标题末尾按 ↓ → 焦点跳到第一个 block 末尾。
   - 第一个 block 头部按 ↑ → 焦点跳到标题末尾。
   - 空 block 按 Backspace → 块被合并/删除。
   - 输入 ``` `foo` ``` 失焦 → 变成 `<code>foo</code>`。再点进 `<code>` 内任意位置，按 → 一次能跳到 `<code>` 之外；按 Backspace 一次能删掉整个 `<code>`。
   - 在 heading-1 块按 ↑/↓ 仍能正常跨块跳转。
   - 代码块（textarea）多行场景下，行内 ↑/↓ 正常工作，行首 ↑/行末 ↓ 跨块。

## 风险点

- contentEditable 的光标位置恢复是兼容性最差的部分，需要在保存后用 `requestAnimationFrame` 异步还原，否则 React 重渲染会覆盖光标。
- `BlockEditor` 暴露的 `focusBlock` 要在 `blocks-container` 用 `useRef` 收集所有 `[data-block-id]` DOM 节点，避免依赖 querySelector 拿不到刚插入的块。
