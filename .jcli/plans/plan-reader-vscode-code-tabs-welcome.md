# reader-vscode-code-tabs-welcome

## 目标

继续把 Reader 往 VS Code + 清爽文档站方向推进，重点解决用户指出的三类体验问题：

1. **代码块高亮不够丰富**
   - Markdown 渲染出来的 fenced code block 需要更丰富、接近 VS Code / GitHub 文档的语法色。
   - 原文件如果是代码文件，不能再只是普通 textarea；需要至少有文件级代码预览/编辑高亮体验。

2. **顶部栏行为对齐 VS Code**
   - Tab 关闭逻辑需要更像 VS Code：未保存文件用 dirty dot，关闭按钮 hover 才明显；关闭当前 tab 后激活相邻 tab；关闭未保存 tab 走确认；快捷键语义保持 `Cmd/Ctrl+W`。
   - 保存逻辑需要更像 VS Code：`Cmd/Ctrl+S` 保存当前文件；dirty 状态在 tab 和标题上体现；保存按钮不应该像普通“提交按钮”，而应是 editor/title action。

3. **空白启动页对齐 VS Code Welcome / Empty Editor 风格**
   - 打开了目录但还没选文件时，空白页应像 VS Code 的空编辑器提示：简洁、靠左/居中偏左、快捷操作列表、弱装饰，不要卡片式大插画。

## 当前已确认现状

- 前端使用 Tailwind CSS v4：`@tailwindcss/vite` + `@import 'tailwindcss'` + `@theme`。
- `assets/reader/src/editor/code-highlight.ts` 当前是手写轻量 tokenizer，仅有：
  - `kw`、`str`、`num`、`cmt`、`type`、`punct`
  - 语言覆盖有限，且没有 function/property/operator/tag/attr/regexp/builtin/constant 等 token。
- `package.json` 已有 `refractor: ^5.0.0`，但当前没有使用。可以优先使用 refractor 做更稳定的 Prism 系语法高亮。
- Markdown code block 渲染入口：
  - `MarkdownEditor.tsx` 的 `createCodeBlockElement`
  - 调用 `renderHighlightedCode(code, lang)`。
- 普通文件编辑入口：
  - `PlainTextEditor.tsx` 当前只有 uncontrolled `<textarea>`，无法做语法高亮。
- TabBar：`assets/reader/src/TabBar.tsx`。
- 顶部 EditorBar 和 EmptyState 在 `assets/reader/src/Reader.tsx` 底部。

## 实施方案

### 1. 高亮体系升级

#### 1.1 Markdown code block 使用 refractor 优先高亮

修改 `assets/reader/src/editor/code-highlight.ts`：

- 使用 `refractor/core`，按需注册常见语言，避免全量体积过大。
- 建议注册：
  - `markup/html/xml`
  - `css`
  - `javascript`
  - `typescript`
  - `jsx` / `tsx`（如 refractor 支持对应组件）
  - `json`
  - `bash/shell`
  - `rust`
  - `go`
  - `python`
  - `sql`
  - `yaml`
  - `toml`
  - `markdown`
- `normalizeLang` 做别名映射：`js -> javascript`、`ts -> typescript`、`tsx -> tsx/typescript`、`html -> markup`、`shell/zsh -> bash`、`rs -> rust`、`yml -> yaml` 等。
- `renderHighlightedCode`：
  - 优先 `refractor.highlight(code, lang)`。
  - 将 refractor AST 转成 DOM fragment。
  - 保留当前手写 tokenizer 作为 fallback，避免未知语言直接无高亮。
- CSS 中适配 Prism token class：
  - `.token.comment`
  - `.token.prolog`
  - `.token.doctype`
  - `.token.punctuation`
  - `.token.property`
  - `.token.tag`
  - `.token.boolean`
  - `.token.number`
  - `.token.constant`
  - `.token.symbol`
  - `.token.deleted`
  - `.token.selector`
  - `.token.attr-name`
  - `.token.string`
  - `.token.char`
  - `.token.builtin`
  - `.token.inserted`
  - `.token.operator`
  - `.token.entity`
  - `.token.url`
  - `.token.atrule`
  - `.token.attr-value`
  - `.token.keyword`
  - `.token.function`
  - `.token.class-name`
  - `.token.regex`
  - `.token.important`
  - `.token.variable`
- 同时保留 `.hl-*` fallback 样式。

#### 1.2 代码文件也使用高亮编辑/预览层

目标是“原来是文件”的代码也能有丰富高亮。考虑当前 `PlainTextEditor` 是 textarea，真正编辑器级高亮需要 overlay 架构；本次做一个轻量可靠版本：

- 新增或改造 `PlainTextEditor.tsx` 为 `CodeTextEditor` 风格：
  - 对代码文件（通过扩展名判断）渲染：
    - 背景层 `<pre><code>`：使用 `renderHighlightedCode(value, lang)` 生成高亮。
    - 前景层 `<textarea>`：透明文字或半透明 caret 层，负责真实输入。
  - 对非代码文本保持普通 textarea，避免 Markdown/plain text 输入出现奇怪体验。
- 需要解决 overlay 同步：
  - `valueRef` / 内部 state 保存当前文本。
  - `onInput` 更新 highlighter DOM，同时回调外层 `onChange`。
  - `onScroll` 同步 textarea 与 pre 的 scrollTop/scrollLeft。
- 语言判断：新增 `detectCodeLanguage(path)` 或在 `fileIconKind.ts` 附近新增工具，覆盖常见扩展名。
- 如果 overlay 复杂度过高，可先实现只读高亮背景 + textarea 输入透明层；重点保证可编辑、可保存、可滚动。

### 2. 顶部栏 / Tab 行为对齐 VS Code

#### 2.1 TabBar 视觉和交互

- Dirty 状态：
  - VS Code 样式是 dirty dot 替代或靠近 close icon。
  - tab 未 hover 时 dirty 文件显示实心圆点；hover 时 close icon 出现。
- Close button：
  - inactive tab close icon 默认弱化/隐藏，hover tab 时显示。
  - active tab close icon可见但克制。
- Close 后激活逻辑：
  - 当前 `forceCloseTab` 需要确认是否关闭后选择相邻 tab。若已经实现但不够 VS Code，调整为：
    - 关闭 active tab：优先激活右侧 tab，否则左侧 tab，否则无 active。
    - 关闭非 active tab：active 不变。
- Middle click 可选：如当前没有，增加 tab `onMouseDown` middle button close。

#### 2.2 保存逻辑和顶部栏

- `Cmd/Ctrl+S`：确认当前已有全局监听；如没有或语义不完整，补齐。
- `EditorBar` 改为 VS Code editor title actions：
  - 左侧 breadcrumb 更像 VS Code：`folder > file`，分隔符用 chevron 而不是 `/`。
  - 右侧 actions：复制路径、保存。
  - 保存按钮：
    - clean 时禁用或弱化，title 显示“无更改”。
    - dirty 时高亮，title 显示“保存”。
    - saving 时显示 spinner 或“保存中”。
- 保存成功后 dirty 清除，失败显示状态；现有 `saveTab` 状态机制保留。
- 未保存关闭：继续使用 CloseConfirmDialog，但文案可向 VS Code 对齐（保存 / 不保存 / 取消）。

### 3. VS Code 风格空白启动页

替换 `EmptyState`：

- 视觉：无大卡片、无大渐变插画。
- 布局：在编辑区中央偏左或居中，像 VS Code Empty Editor：
  - 标题：`J Reader`
  - 副标题：`选择左侧文件开始阅读或编辑`
  - 分组：`开始` / `快捷键`
  - 快捷项：
    - `从 Explorer 打开文件`
    - `新建文件`（如果有可触发 action 就接入；否则仅提示左侧 + 按钮后续可接）
    - `切换文件栏 Cmd/Ctrl+1`
    - `保存 Cmd/Ctrl+S`
    - `关闭编辑器 Cmd/Ctrl+W`
- 风格：文字列表 + 小 icon + kbd，和 VS Code welcome 一样信息密度高、装饰低。

### 4. 验证

修改后运行：

- `npm run format:check`
- `npm run build`
- `cargo fmt`
- `cargo clippy -- -D warnings`

`npm run lint` 可运行，但当前项目已有既有 lint 问题；需要记录是否新增 lint 问题。若本次修改文件能顺手规避 lint，则尽量修掉本次新增项。

## 预计改动文件

- `assets/reader/src/editor/code-highlight.ts`
- `assets/reader/src/editor/editor.css`
- `assets/reader/src/PlainTextEditor.tsx`
- `assets/reader/src/TabBar.tsx`
- `assets/reader/src/Reader.tsx`
- 可能新增：`assets/reader/src/codeLanguage.ts`
- `assets/reader/dist/**`（build 生成）

## 风险与注意事项

- `refractor` ESM 语言注册路径要和 v5 API 对齐，需通过 build 验证。
- contenteditable code block 内使用 refractor span 后，编辑时可能产生嵌套 span。当前保存通过 `textContent` 取值，理论上可行；但输入后的实时高亮可能不应在 active code block 中频繁重建 DOM，否则会影响光标。因此 Markdown code block 继续只在 block 创建/patch 时高亮，输入过程中保持可编辑稳定。
- PlainTextEditor overlay 高亮要确保 textarea 输入、选择、滚动、保存不受影响；如果透明文字影响可用性，可改成 textarea 正常文字 + 背景层只在只读/失焦时展示。
