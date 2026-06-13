# jstudio Sandbox 简化和智能化

## 背景

`apps/jstudio` 里的"代码沙盒"块（`html-render`）目前是整个编辑器里**最重、最啰嗦**的一块 UI：

- 顶部有一个标题 "代码沙盒" + 4 个顶层 tab（`预览 / HTML / CSS / JS`）—— 4 选 1
- 预览区上方又有 3 个子 tab（`代码 / 网页 / 文件`）—— 还要再选 1 次
- 切到 "网页" 才出 URL 输入框，切到 "文件" 才出上传按钮
- 状态里既有持久化的 `sandboxPreviewMode` / `sandboxPreviewUrl` / `sandboxPreviewFileName`，又有只在内存里活的 `sandboxPreviewFileSrc`，还有 `sandboxPreviewLoadKey`、`sandboxWebError` 等次要状态

用户已经明确说：
- "sandbox 的样式可以更简洁一点"
- "你可以自己判断是 html 还是文件的，没必要要用户选"

## 目标

- **去掉两层 tab**。砍掉顶层 `预览/HTML/CSS/JS`，去掉中间层 `代码/网页/文件`。
- **自动判断预览源**：用户给出什么就渲染什么，不需要点 tab 切换。
- **样式更轻**：把厚重的标题栏和"代码沙盒"四个字挪走，腾出空间给预览本身。
- **状态更少**：删掉 `sandboxPreviewMode`、相关的 props、相关字段。

## 设计

### 1. 永远只展示一个布局：预览 + 代码

不再有顶层 tab。每个 sandbox 块固定为：

```
+-------------------------------------------------------------+
|  [Preset v]  [Reload]                                [theme] |  <- 极简工具栏
+-------------------------------------------------------------+
|  [ URL 输入框...  ]  [File]  [Clear]                        |  <- 源控制条
+-------------------------------------------------------------+
|                                                             |
|                  iframe 预览                                |
|                                                             |
+-------------------------------------------------------------+
|  HTML | CSS | JS                                            |  <- 代码子 tab
|  +-----------------------------------------------------+    |
|  | code editor                                        |    |
|  +-----------------------------------------------------+    |
+-------------------------------------------------------------+
```

### 2. 预览源自动判断

按优先级**自动选择** iframe 的 `src` / `srcDoc`：

```ts
const previewSource = useMemo(() => {
  // 1. 有文件就显示文件（HTML/PDF/图片都可以直接塞进 iframe）
  if (sandboxFileSrc) return { kind: "file", src: sandboxFileSrc, name: sandboxFileName };
  // 2. 有合法 URL 就显示远程网页
  const normalized = normalizeSandboxUrl(sandboxUrl);
  if (normalized) return { kind: "url", src: normalized };
  // 3. 都没有就显示用户写的代码
  return { kind: "code", srcDoc: sandboxDebouncedSrcDoc };
}, [sandboxFileSrc, sandboxFileName, sandboxUrl, sandboxDebouncedSrcDoc]);
```

- 三个输入源（文件 / URL / HTML/CSS/JS 代码）**独立存活**，不会互相覆盖。
- 用户拖入一个 PDF → 立刻显示 PDF；点 "Clear" → 回到代码预览。
- 用户在 URL 框里输入 `wikipedia.org` → 立刻显示网页；清空 URL → 回到代码。
- 用户写代码 → 默认就是代码预览。
- 任意时刻，**主预览框只有一个**；右上角徽标告诉用户当前显示的是 "Code / Web / File" 哪一种。

### 3. 状态重塑

| 旧 state | 新 state | 说明 |
|---------|---------|------|
| `sandboxPreviewMode` | 删 | 改成从其他 state 派生 |
| `sandboxPreviewUrl` | `sandboxUrl` | 重命名 |
| `sandboxPreviewFileSrc` | `sandboxFileSrc` | 重命名 |
| `sandboxPreviewFileName` | `sandboxFileName` | 重命名 |
| `sandboxPreviewLoadKey` | 删 | URL 加载失败时改用 `key={sandboxUrl}` 触发重载 |
| `sandboxWebError` | 改成派生 | 不再需要专门 state，错误时用 `<iframe onError>` + ref 监听 |
| `sandboxTab` | `sandboxCodeTab` | 范围缩小：只取 `html` / `css` / `js`，没有 "preview" 和 "split" |

持久化字段也跟着收窄：

```diff
// types.ts
- sandboxPreviewMode?: 'html' | 'url' | 'file';
- sandboxPreviewUrl?: string;
- sandboxPreviewFileName?: string;
+ sandboxUrl?: string;        // URL 输入框的当前值
+ sandboxFileName?: string;   // 仅有文件名需要持久化（dataURL 不存）
```

旧字段仍然**保留**在 `Block.properties` 的类型里以兼容老文档的读取，但在新逻辑里不再写入。

### 4. 拖拽支持

预览区域加上 `onDragOver` / `onDrop`：

- 拖入文件 → 立刻读取为 dataURL，喂给 `sandboxFileSrc`。
- 拖入 URL（从浏览器标签页拖入会得到 text/uri-list）→ 写入 `sandboxUrl`。
- 拖入过程中的可视反馈：在预览区加一层高亮遮罩 "Drop a file or URL here"。

### 5. 工具栏更轻

旧：

```
[代码沙盒]                          [预览][HTML][CSS][JS]
```

新：

```
[Preset v] [Reload]                                            [theme]
```

- "Preset" 仍保留，点击展开两个预设（依旧只列当前 `SANDBOX_PRESETS` 的 2 个）。
- "Reload" 按钮触发 `setRunIndicator(v => v + 1)`，重新编译 srcDoc 和重载文件 / URL 的 iframe。
- 主题切换挪到最右。
- 不再有任何顶层 tab。

## 实现要点

### 状态

- 把 `BlockItem` 内部的 7 个 sandbox 相关 `useState` 收敛为 8 个：`sandboxHtml` / `sandboxCss` / `sandboxJs` / `sandboxTheme` / `sandboxUrl` / `sandboxFileSrc` / `sandboxFileName` / `sandboxCodeTab` / `runIndicator`。其中 URL 和 FileName 是**双向绑定到 `block.properties`**。
- `useEffect` 初始化时只读 `sandboxUrl` / `sandboxFileName`（不读 `sandboxPreviewMode`）。

### 渲染顺序

```jsx
{previewSource.kind === "file" && <FileBadge>{previewSource.name}</FileBadge>}
{previewSource.kind === "url"  && <UrlBadge>{previewSource.src}</UrlBadge>}
{previewSource.kind === "code" && <CodeBadge>Code preview</CodeBadge>}

<iframe
  title={`Sandbox ${block.id}`}
  src={previewSource.kind === "code" ? undefined : previewSource.src}
  srcDoc={previewSource.kind === "code" ? previewSource.srcDoc : undefined}
  sandbox="allow-scripts allow-modals allow-forms allow-popups"
  className="w-full h-[380px] border-none bg-white dark:bg-slate-900"
/>
```

URL 模式下 `key={previewSource.src + runIndicator}` 来强制刷新。

### 拖拽

```jsx
<div
  onDragOver={(e) => { e.preventDefault(); setIsDragging(true); }}
  onDragLeave={() => setIsDragging(false)}
  onDrop={handleSandboxDrop}
  className={isDragging ? "ring-2 ring-indigo-500" : ""}
>
  <iframe ... />
</div>
```

`handleSandboxDrop`：

- `e.dataTransfer.files[0]` → 走 `handleSandboxFileSelect`
- `e.dataTransfer.types.includes("text/uri-list")` → `e.dataTransfer.getData("text/uri-list")` 写 `sandboxUrl`

### 旧 props / 旧 state 全清掉

`handleSandboxChange` 不再接收 `previewModeVal` / `previewUrlVal` / `previewFileNameVal`。改为：

```ts
const handleSandboxCodeChange = (html, css, js) => { ... };
const handleSandboxUrlChange  = (url) => { ... };
const handleSandboxFileSelect = (file) => { ... };
```

干净分离。

## 涉及文件

- `apps/jstudio/src/components/BlockItem.tsx` —— sandbox 整个分支（约 270 行）
- `apps/jstudio/src/types.ts` —— `Block.properties` 类型收窄（保留旧字段名作为可读可不读，老文档不破）

## 验证

1. `npm run lint && npm run build` 必须通过。
2. 手动测试：
   - 默认打开一个 sandbox 块，看到预览 + 代码同时存在。
   - 在 HTML 文本里写 `<h1>hi</h1>` → 预览实时出现 `<h1>`。
   - 在 URL 框里输入 `https://example.com` → 预览切到远端网页；点 "Clear" → 回到代码预览。
   - 点 `File` 选择 `paper.pdf` → 预览显示 PDF；点 "Clear" → 回到代码预览。
   - 把一个图片文件从 Finder 拖到预览区 → 预览显示图片。
   - 刷新（Reload 按钮）→ 预览强制重新加载。
   - 切 dark/light 主题 → 预览内 iframe 的代码部分主题随之改变。
3. 确认旧文档打开后仍能正常显示（兼容读旧 `sandboxPreviewMode` 字段：新逻辑把它当成 hint 写回新 `sandboxUrl` / `sandboxFileName` / 默认 code）。
