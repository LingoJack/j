# 自研 Markdown 编辑器替代 CodeMirror 6

## 目标

完全移除 CM6 依赖，自研一个 **contenteditable-based** Markdown 编辑器，实现 Typora 风格的即时渲染 + 一体化编辑体验。

核心原则：
- **所见即所得**：表格永远是渲染后的表格，图片永远是图片，代码块永远是高亮的代码块
- **就地编辑**：点击表格单元格直接编辑，点击代码块直接改代码，无需回退源码
- **零 layout 约束**：不再受 CM6 "block decoration 必须直接提供" 等架构限制

## 架构设计

### 核心思路

用一个 `contenteditable` 的 `<div>` 作为编辑容器，内部按 block 级渲染。每个 block 是一个 DOM 节点，根据类型有不同的渲染形态：

| Block 类型 | 渲染形态 | 编辑方式 |
|-----------|---------|---------|
| heading | `<h1>` ~ `<h6>` + hidden `#` markers | 直接编辑文本，`#` 不可见 |
| paragraph | `<p>` | 直接编辑 |
| table | `<table>` + `contenteditable` td/th | 点击单元格直接编辑 |
| code block | `<pre><code>` + Prism 高亮 | 直接编辑代码文本 |
| image | `<img>` widget | 点击选中，编辑 URL 需切到源码 |
| blockquote | `<blockquote>` | 直接编辑 |
| list | `<ul>/<ol>` + `<li>` | 直接编辑 |
| hr | `<hr>` | 不可编辑 |
| inline HTML | 渲染后的 DOM | 不可编辑（保护结构） |

### 一次性渲染 + 增量更新 + 缓存

```
Markdown Source (string)
       ↓ parse (cached by source hash)
Block[] (AST)  ←── Cache: Map<sourceHash, Block[]>
       ↓ render (cached by block identity)
DOM (contenteditable blocks)
       ↓ on edit
Markdown Source (string)  ← serialize back
       ↓ parse (incremental: only re-parse changed regions)
Block[] (AST)
       ↓ diff (keyed by source line range) + patch
DOM (minimal update)
```

### 缓存层设计

#### 1. Parse Cache（语法树缓存）

```typescript
interface ParseCache {
  /** source 的内容 hash → 解析后的 Block[] */
  ast: Map<number, Block[]>
  /** 上一次的 source 和 parse 结果 */
  lastSource: string
  lastBlocks: Block[]
  /** 上一次每个 block 对应的 source line range（用于增量 diff） */
  lastRanges: Array<{ from: number; to: number }>
}
```

- 每次编辑后，对比 source 变化区域，只重新解析受影响的 block
- 如果 source 未变（如 selection 变化、focus 变化），直接复用上次解析结果
- Block 对象带 `startLine` / `endLine` 字段，用于定位变化范围

#### 2. DOM Cache（渲染节点缓存）

```typescript
interface RenderCache {
  /** block identity key → 对应的 DOM 节点 */
  nodes: Map<string, HTMLElement>
  /** 上一次每个 block 的 identity key（用于 diff） */
  lastKeys: string[]
}
```

- 每个 block 根据 type + position 生成 identity key
- diff 时：相同 key 的 block 不重新渲染，只更新内容（如 heading 文本变了）
- 新增的 block 创建新 DOM 节点
- 删除的 block 移除 DOM 节点
- **表格/代码块等重 DOM 操作的 block**：只要 key 不变，完全不重新创建（保留用户编辑状态、焦点、选区）

#### 3. Inline Cache（行内渲染缓存）

```typescript
interface InlineCache {
  /** inline 内容 hash → 渲染后的 DocumentFragment */
  fragments: Map<number, DocumentFragment>
}
```

- 对每个 paragraph/heading 的 inline 内容做 hash
- hash 命中时直接 clone 缓存的 fragment，避免重复正则匹配和 DOM 创建
- 表格单元格内的 inline 渲染也走此缓存

### 增量更新流程

```
用户输入 → input 事件
  ↓
提取完整 source 文本
  ↓
与 cache.lastSource 对比，找到变化行范围 [changeStart, changeEnd]
  ↓
只重新解析 [changeStart, changeEnd] 范围内的 block
  ↓
与 lastBlocks 做 keyed diff：
  - 相同 key → 检查内容是否变化 → 仅更新文本/DOM 属性
  - 新增 key → 创建新 DOM 节点并插入
  - 删除 key → 移除 DOM 节点
  ↓
更新 Parse Cache + DOM Cache
  ↓
通知 onChange + onParsed（debounce）
```

### 文件结构

```
src/editor/
  ├── MarkdownEditor.tsx      # 主编辑器 React 组件
  ├── parser.ts               # Markdown → Block[] 解析器（基于 Lezer）
  ├── serializer.ts           # Block[] → Markdown 序列化器
  ├── renderer.tsx            # Block[] → DOM 渲染器
  ├── inline-renderer.ts      # inline markdown (bold/italic/code/link) → DOM
  ├── table-handler.ts        # 表格编辑交互（Tab 跳转、blur 同步等）
  ├── code-handler.ts         # 代码块编辑交互（语法高亮、语言切换）
  ├── cursor.ts               # 光标/选区管理
  ├── cache.ts                # Parse Cache + DOM Cache + Inline Cache
  ├── editor.css              # 编辑器样式（Tailwind + 少量自定义）
```

## 实施步骤

### Phase 1：基础框架（editor shell + parser + heading/paragraph）

1. **创建 `MarkdownEditor.tsx`** — 主组件，管理 `source` ↔ DOM 双向同步
2. **创建 `parser.ts`** — 用 Lezer markdown parser 把源码解析成 Block AST（复用现有 `@lezer/markdown` 依赖，不引入新包）
3. **创建 `renderer.tsx`** — 把 Block AST 渲染成 DOM 节点
4. 实现 heading + paragraph 的渲染和编辑
5. 验证：能在 heading/paragraph 中自由输入文字，双向同步

### Phase 2：inline 渲染（bold/italic/code/link/image/strikethrough）

1. **创建 `inline-renderer.ts`** — 把 inline markdown 节点渲染成 DOM
2. 实现 marker 字符的隐藏/显示（类似当前 livePreview 的逻辑，但基于 DOM 而非 CM6 decoration）
3. 验证：`**bold**` 显示为 **bold**，但编辑时能看到 `**`

### Phase 3：block 级元素（table/code block/blockquote/list/hr）

1. **表格**：`<table>` 渲染 + `contenteditable` 单元格 + blur 回写
2. **代码块**：`<pre><code>` + refractor 高亮 + 语言标识
3. **引用块**：`<blockquote>` 渲染
4. **列表**：`<ul>/<ol>/<li>` 渲染 + checkbox 支持
5. **分割线**：`<hr>` 渲染
6. 验证：所有 block 类型都能渲染和编辑

### Phase 4：图片 + HTML widget

1. **图片**：`<img>` widget 渲染，URL 解析复用现有 `resolveAssetUrl`
2. **HTML block**：innerHTML 渲染 + sanitize
3. 验证：README.md 完整渲染正确

### Phase 5：清理

1. 删除 `src/cm6/` 目录及所有 CM6 相关依赖
2. 更新 `package.json` 移除 CM6 包
3. 更新 `CodemirrorEditor.tsx` → 替换为新编辑器
4. 构建验证 + 视觉验证

## 关键技术决策

### Q: 用 Lezer 还是自写 parser？
**A: 用 Lezer**。已经有 `@lezer/markdown` 依赖，它是增量 parser，性能好，能识别所有 GFM 语法节点（Table, FencedCode, Image, HTMLBlock 等）。自写 parser 工作量巨大且容易出 bug。

### Q: 为什么不用 ProseMirror / TipTap / Slate？
**A: 过度抽象。** 这些框架本质上是"自己实现一个 contenteditable"但加了大量的 schema/view/state 抽象层。我们要的是 Typora 体验——直接操作 DOM 就够了。当需要更精细的光标管理时可以局部引入。

### Q: 如何实现"即时渲染"？
**A: 输入时 debounce 解析 + 增量 DOM 更新。** 具体流程：
1. 用户输入 → `input` 事件触发
2. 从 `contenteditable` DOM 提取纯文本 → 重新解析 → diff AST
3. 只更新变化的 block DOM 节点（不影响未修改的 block，保持其焦点/选区）

### Q: 表格编辑具体怎么做？
**A: 每个 td/th 都是 `contenteditable="true"`。** 用户点击单元格直接编辑。blur 时将整个表格 DOM 序列化回 markdown 并更新 source。Tab/Shift+Tab 跳转单元格。

## 与 Reader.tsx 的契约

新编辑器组件接口与 `CodemirrorEditor` 完全一致：

```typescript
interface Props {
  path: string
  baseDir: string | null
  initialSource: string
  onChange: (path: string, source: string) => void
  onParsed: (path: string, doc: ParsedDocument) => void
  onSave: () => void | Promise<void>
}
```

替换时只需改 import 路径。
