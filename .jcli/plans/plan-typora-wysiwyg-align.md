# jstudio 编辑器对标 Typora WYSIWYG 方案

## 问题根源

当前 jstudio 采用 **"点击切换控件"** 模式：
- 渲染态：`<table>`/`<pre><code>` — 富文本 DOM，紧凑的排版
- 编辑态：`<textarea>` — 独立控件，自带 padding/border/行高，与渲染态排版参数不一致

这导致编辑态和渲染态在尺寸上有明显跳动，尤其是表格和代码块。

## Typora 的核心思路

Typora 采用 **"透明 overlay"** 技术，核心原则是：

> **编辑态和渲染态使用同一套 CSS 参数，编辑控件覆盖在渲染 DOM 上，完全透明，用户看到的是渲染效果，但实际输入在透明控件中。**

具体实现：
1. **代码块**：渲染态的 `<pre><code>`（语法高亮）始终显示；编辑态用透明 `<textarea>` 覆盖，textarea 与 pre/code 使用完全相同的 font-size/line-height/padding/font-family
2. **表格**：每个 `<td>/<th>` 是 `contenteditable` 的富文本容器，直接在渲染 cell 内编辑，无需切换控件
3. **行内元素**：段落、heading、blockquote 等同样用 `contenteditable` 或透明 overlay

## 改造方案

### 一、代码块：透明 textarea overlay

**当前实现**：
- 渲染态：`createCodeBlockElement()` → `<div class="md-code-wrap">` → `<pre class="md-code-pre">` → `<code class="md-code-content">` (语法高亮 span)
- 编辑态：`createCodeBlockEditor()` → `<textarea class="md-code-source-input">` (替换整个结构)

**改造方案**：

```
渲染态（始终存在）:
<div class="md-code-wrap">
  <input class="md-code-lang-input" /> ← 语言输入，始终显示
  <pre class="md-code-pre">
    <code class="md-code-content"> ← 语法高亮渲染结果（textarea 在其上方）
      <span class="token keyword">fn</span>
      <span class="token function">main</span>
      ...
    </code>
  </pre>
</div>

编辑态（透明 overlay）:
<div class="md-code-wrap md-code-editing"> ← 添加 editing 状态标记
  <input class="md-code-lang-input" /> ← 语言输入（可见）
  <pre class="md-code-pre">
    <code class="md-code-content"> ← 语法高亮渲染（作为背景）
    </code>
    <textarea class="md-code-overlay-input"> ← 新增：透明 textarea overlay
      fn main() { ... }                  ← 原始代码文本（无高亮，透明显示）
    </textarea>
  </pre>
</div>
```

**关键 CSS**：
```css
/* 透明 overlay textarea */
.md-code-overlay-input {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  /* 完全继承 pre/code 的排版参数 */
  font-family: var(--font-mono);
  font-size: 13px;
  line-height: 1.65;
  padding: 16px 20px; /* 与 .md-code-pre 一致 */
  margin: 0;
  border: none;
  background: transparent;
  color: transparent; /* 文字透明，用户看到的是下面的语法高亮 */
  caret-color: var(--color-seeyue-accent); /* 光标可见 */
  resize: none;
  outline: none;
  white-space: pre;
  overflow-x: auto;
  z-index: 1; /* 在 code 之上 */
}

.md-code-overlay-input::selection {
  background: rgba(204, 120, 92, 0.25);
  color: transparent;
}
```

**编辑流程**：
1. 点击代码块 → 在原渲染 DOM 上创建透明 textarea overlay
2. 用户输入 → textarea 值实时变化
3. 实时解析 → 语法高亮 DOM（下面的 `<code>`）同步更新
4. 失焦 → 移除 textarea overlay，保留渲染 DOM

### 二、表格：contenteditable cell

**当前实现**：
- 渲染态：`<table>` → `<th>/<td>` → 直接渲染 Inline 内容
- 编辑态：`<table>` → `<th>/<td>` → `<textarea>` (替换 cell 内容)

**改造方案**：

```
渲染态（可编辑状态）:
<div class="md-table-wrap md-table-editing">
  <table class="md-table">
    <thead>
      <tr>
        <th contenteditable="true" class="md-cell-editing">
          Header Text          ← 直接编辑渲染内容
        </th>
      </tr>
    </thead>
    <tbody>
      <tr>
        <td contenteditable="true" class="md-cell-editing">
          <strong>bold</strong> and <em>italic</em> ← 支持富文本编辑
        </td>
      </tr>
    </tbody>
  </table>
</div>
```

**关键 CSS**：
```css
/* 编辑态 cell 保持渲染态样式 */
.md-cell-editing {
  outline: none;
  /* 完全继承渲染态 cell 的 padding/font/line-height */
}

.md-cell-editing:focus {
  background: var(--color-seeyue-elevated);
  box-shadow: inset 0 0 0 2px var(--color-seeyue-accent);
}
```

**编辑流程**：
1. 点击表格 → 给所有 `<th>/<td>` 添加 `contenteditable="true"`
2. 点击 cell → focus 进入 cell，直接编辑渲染内容
3. cell 内容变化 → 实时同步到 Markdown 源码
4. 表格整体失焦 → 移除 contenteditable，保留渲染 DOM

**注意**：contenteditable 的 cell 内容是富文本（Inline 渲染结果），需要：
- 从 cell.innerHTML 逆向提取 Markdown 文本
- 或者在 cell 内用透明 overlay 显示 Markdown 源码（更复杂但更准确）

### 三、通用元素：透明 overlay 或 contenteditable

对于段落、heading、blockquote 等：
- Typora 用 contenteditable 的富文本编辑
- 但这需要逆向从 DOM 提取 Markdown，复杂度高
- jstudio 当前用 textarea 切换，相对简单

**建议**：先改造代码块和表格（用户感知最明显），其他元素保持当前 textarea 模式但统一排版参数。

## 实施步骤

### Step 1：统一 CSS 排版参数

在 `editor.css` 中建立一套统一的排版变量：

```css
@theme {
  /* 编辑态/渲染态统一排版 */
  --seeyue-code-font-size: 13px;
  --seeyue-code-line-height: 1.65;
  --seeyue-code-padding-x: 20px;
  --seeyue-code-padding-y: 16px;
  
  --seeyue-table-font-size: 13.5px;
  --seeyue-table-cell-padding: 14px;
  --seeyue-table-cell-py: 8px; /* py-2 = 8px */
}
```

### Step 2：改造代码块编辑器

修改 `MarkdownEditor.tsx` 的 `createCodeBlockEditor()`：

1. 不创建新的 `<div>`，而是复用渲染态的 `md-code-wrap`
2. 在 `<pre>` 内追加透明 textarea overlay
3. textarea 的 CSS 参数与 `.md-code-content` 完全一致
4. 监听 textarea input → 实时更新下面的语法高亮 `<code>`
5. 失焦 → 移除 textarea overlay

### Step 3：改造表格编辑器

修改 `MarkdownEditor.tsx` 的 `createTableBlockEditor()`：

1. 复用渲染态的 `<table>` DOM
2. 给每个 `<th>/<td>` 设置 `contenteditable="true"`
3. 监听 cell input → 实时同步到 Markdown 源码
4. 实现从 cell 富文本逆向生成 Markdown 的逻辑

### Step 4：实现实时语法高亮更新

代码块编辑时，textarea 内容变化 → 触发语法高亮重新渲染 → 更新下面的 `<code>` DOM。

需要：
- 扩展 `renderHighlightedCode()` 支持增量更新
- 或每次重新渲染整个 `<code>`（简单但有性能开销）

### Step 5：处理边界情况

- Tab 键在 textarea 中插入制表符（不跳转焦点）
- 多行代码的行号对齐
- 语言输入框的焦点管理
- 表格 cell 的键盘导航（Tab 移动到下一个 cell）

## 风险与权衡

| 方面 | 当前方案 | Typora 方案 |
|---|---|---|
| 实现复杂度 | 低（textarea 切换） | 高（overlay + 实时同步） |
| WYSIWYG 效果 | 有尺寸跳动 | 完美对齐 |
| 性能 | 好（切换时重绘） | 需优化（实时高亮） |
| 富文本编辑 | 不支持（纯文本） | 支持（contenteditable） |
| Markdown 准确性 | 高（直接编辑源码） | 需逆向提取（可能有误差） |

**建议**：优先改造代码块（用户感知最明显），表格可暂时用简化方案（contenteditable cell 只编辑纯文本，不支持富文本）。

## 预期效果

改造后：
- 代码块：点击进入编辑，渲染态和编辑态尺寸完全一致，用户看到的是语法高亮效果，但实际在透明 textarea 中输入源码
- 表格：点击 cell 直接编辑，cell 尺寸不变，编辑内容直接显示
- 其他元素：保持当前行为，但逐步统一排版参数