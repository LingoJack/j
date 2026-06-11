# Plan: jstudio-doc-writing

## 用户反馈后的调整

用户明确要求“要大刀阔斧”。因此方案不再是渐进式包装 Markdown 阅读器，而是把 `apps/jstudio` 改造成一个以“文档写作”为核心的一体化本地知识库应用。Markdown 只保留为兼容层/导入导出层，不再主导 UI、命名和核心交互。

## 新定位

JStudio = 本地优先的文档工作台，类似语雀 + Notion 的桌面写作空间：

- 左侧是“知识库 / 空间 / 页面树”，不是文件 Explorer。
- 中间是“页面编辑画布”，不是 Markdown 预览器。
- 内容以块为核心：标题、段落、列表、任务、引用、代码、表格、图片等。
- 顶部强调文档状态、发布/导出/模板，而不是路径和文件操作。
- Markdown 文件系统仍可作为一期持久化，但用户心智必须完全转成“文档”。

## 大刀阔斧的架构方向

### 1. 前端模块重组

新增新的产品模块，逐步废弃 Reader 命名：

```text
apps/jstudio/src/app/studio/
  StudioApp.tsx              # 新主应用，替代 Reader 的产品壳
  studio.css                 # Studio 级布局和视觉变量
  components/
    WorkspaceSidebar.tsx     # 知识库/页面树
    PageTabs.tsx             # 页面标签，弱化文件路径
    PageHeader.tsx           # 封面、图标、标题、状态
    PageCanvas.tsx           # 文档画布
    CommandPalette.tsx       # 命令面板/新建/搜索
    TemplateGallery.tsx      # 模板库
    OutlinePanel.tsx         # 大纲
    StatusBar.tsx            # 字数、保存、同步/本地状态
  editor/
    BlockEditor.tsx          # 新块编辑器外壳
    block-model.ts           # 前端块模型
    markdown-codec.ts        # block <-> markdown 兼容转换
    slash-menu.ts            # / 菜单定义
    bubble-toolbar.tsx       # 选区工具条
```

短期为了控制风险，`main.tsx` 仍可挂载 `StudioApp`，旧 `Reader.tsx` 保留但不作为主入口。旧 `MarkdownEditor.tsx` 的解析/渲染逻辑可抽取为兼容层，而不是继续在旧组件里堆功能。

### 2. 数据模型从文件转页面

新增页面级类型，前端不再直接围绕 `RenderedDoc` 和 `Tab` 设计：

```ts
type PageId = string

interface Workspace {
  rootPath: string
  name: string
  pages: PageTreeNode[]
}

interface PageTreeNode {
  id: PageId
  title: string
  path: string
  children?: PageTreeNode[]
  icon?: string
  cover?: string
  updatedAt?: number
  kind: 'page' | 'group'
}

interface PageDocument {
  id: PageId
  path: string
  title: string
  icon?: string
  cover?: string
  blocks: DocBlock[]
  sourceFormat: 'markdown' | 'jdoc'
  dirty: boolean
}
```

一期 `id` 可以由 path 派生，仍保存为 `.md`；后续可迁移到 `.jstudio/pages/*.json` 或 SQLite。

### 3. 引入真正的块模型

不要继续让 UI block 直接等于 Markdown IR。新增 `DocBlock`：

```ts
type DocBlock =
  | { id: string; type: 'heading'; level: 1 | 2 | 3; text: RichText[] }
  | { id: string; type: 'paragraph'; text: RichText[] }
  | { id: string; type: 'bullet_list'; items: ListItem[] }
  | { id: string; type: 'ordered_list'; items: ListItem[] }
  | { id: string; type: 'todo'; checked: boolean; text: RichText[] }
  | { id: string; type: 'quote'; text: RichText[] }
  | { id: string; type: 'code'; lang?: string; code: string }
  | { id: string; type: 'divider' }
  | { id: string; type: 'table'; rows: RichText[][][] }
```

Markdown parser 只负责打开旧文件时转为 `DocBlock`，保存时再转回 Markdown。这样后续可以加入封面、图标、callout、数据库等非 Markdown 能力。

### 4. 编辑器重写策略

“大刀阔斧”不等于立刻引入超重依赖。建议两条路线二选一：

#### 路线 A：自研轻量 BlockEditor（推荐先做）

- 每个 block 是独立 React 组件，使用 textarea/contenteditable 混合。
- Enter 新建同类/段落 block。
- Backspace 合并或删除空 block。
- `/` 触发插入菜单。
- 左侧 block handle 支持新增/复制/删除，拖拽排序二期做。
- Bubble toolbar 支持加粗、斜体、代码、链接。
- 保存时 `DocBlock[] -> Markdown`。

优点：贴合当前代码，依赖少，能快速做出 Notion/语雀感。
缺点：富文本边界需要自己维护。

#### 路线 B：引入 Tiptap/ProseMirror（二期或如果接受重依赖）

- 用 Tiptap 管编辑器 schema、selection、history、快捷键。
- 自定义 node：callout、todo、table、code、image。
- Markdown 通过 serializer/parser 转换。

优点：编辑器能力成熟。
缺点：改动更深，包体/复杂度高，Tauri 首屏和调试成本更高。

我建议先执行路线 A，做出产品方向；如果后续需求升级，再迁移路线 B。

## 要删除/弱化的旧体验

- 删除或隐藏“Reader / Explorer / 文件 / 复制路径”这些主 UI 文案。
- `ActivityBar` 不再是 VS Code 风文件/工具切换，改为：文档、搜索、模板、工具。
- `FileTree` 改名或替换为 `WorkspaceSidebar`。
- `EditorBar` 改为 `PageHeader` + `StatusBar`。
- `TableOfContents` 改为 `OutlinePanel`，只在文档编辑上下文出现。
- `PlainTextEditor` 和图片查看保留为附件/兼容预览，不作为主工作流。

## 具体实施计划

### Phase 0：入口切换与设计基线

1. 新建 `StudioApp.tsx`，承接 Reader 的初始化、打开目录、打开页面、保存等能力。
2. `main.tsx` 从渲染 `Reader` 改为渲染 `StudioApp`。
3. 新建 `studio.css`，定义文档产品的视觉基线：
   - 背景：更接近语雀/Notion 的浅色纸张感。
   - 左栏：知识库导航，不再 VS Code Explorer。
   - 画布：居中 760-840px，顶部大留白。
   - 字体：正文系统字体，代码 mono。
4. 保留旧 `reader.css` 变量以减少断裂，但新页面优先使用 Studio class。

验收：启动后看到的是 JStudio 文档工作台，而不是 J Reader。

### Phase 1：WorkspaceSidebar 替代 FileTree

1. 新建 `WorkspaceSidebar.tsx`：
   - 顶部显示当前知识库名。
   - CTA：新建页面、打开知识库。
   - 页面树只突出 `.md` 页面和目录分组。
   - 右键菜单改为：新建子页面、重命名、删除、在 Finder 中显示。
2. 新建页面默认创建 `.md`，如果用户没写后缀自动补。
3. 新建页面自动写入模板内容，而不是空文件：

```md
# 未命名文档

开始写作，输入 / 插入内容块。
```

验收：左侧完全是知识库页面树语义，用户不需要理解文件扩展名。

### Phase 2：PageHeader + PageCanvas

1. 新建 `PageHeader.tsx`：
   - 页面图标占位。
   - 大标题输入框，双向绑定第一个 H1。
   - 保存状态、字数、更多菜单。
2. 新建 `PageCanvas.tsx`：
   - 包装新 `BlockEditor`。
   - 空文档显示“输入 / 插入块”。
   - 支持从文件名/H1 提取标题。
3. `PageTabs` 只显示页面标题，不显示完整文件名路径。

验收：打开页面后第一屏就是类 Notion 文档页：icon、title、正文块。

### Phase 3：BlockEditor MVP

1. 新建 `BlockEditor.tsx` 和 `block-model.ts`。
2. 实现 Markdown IR -> DocBlock 转换：
   - heading -> heading
   - paragraph -> paragraph
   - list -> bullet/ordered/todo
   - block_quote -> quote
   - code_block -> code
   - table -> table
   - rule -> divider
3. 实现 DocBlock -> Markdown 保存。
4. 实现基础交互：
   - Enter 新建 block。
   - Backspace 删除空 block 或合并上一段。
   - `/` slash menu 插入块。
   - block hover 显示 `+` 和 handle。
   - `Cmd/Ctrl+S` 保存。
5. 先支持纯文本 rich text，inline 加粗/链接可在 Phase 4 加。

验收：不进入 Markdown 源码 textarea，也能完成常见文档写作。

### Phase 4：写作效率能力

1. Slash Menu 完整化：标题、列表、任务、引用、代码、表格、分割线。
2. Bubble Toolbar：加粗、斜体、代码、链接。
3. 模板库：会议记录、项目方案、PRD、日报、周报、技术方案。
4. 命令面板：新建页面、搜索页面、打开最近、切换模板。

验收：用户可以像语雀/Notion 一样用快捷命令写文档。

### Phase 5：本地知识库元数据

新增 `.jstudio/metadata.json`（在用户打开的知识库根目录下）：

```json
{
  "version": 1,
  "workspace": { "name": "我的知识库" },
  "pages": {
    "/abs/path/doc.md": {
      "title": "文档标题",
      "icon": "doc",
      "cover": null,
      "pinned": false,
      "updatedAt": 1730000000000
    }
  }
}
```

后端新增命令：

- `read_workspace_metadata(root)`
- `write_workspace_metadata(root, patch)`
- `create_page(parent, title, template)`
- `move_page(path, new_parent, order?)`

验收：JStudio 开始拥有自己的知识库体验，而不只是文件浏览器外壳。

## 文件级改动清单

第一轮实际落地建议涉及：

- `apps/jstudio/src/main.tsx`
  - 挂载 `StudioApp`。
- `apps/jstudio/src/app/studio/StudioApp.tsx`
  - 新应用壳。
- `apps/jstudio/src/app/studio/studio.css`
  - 新视觉系统。
- `apps/jstudio/src/app/studio/components/WorkspaceSidebar.tsx`
  - 替代 FileTree 的文档树。
- `apps/jstudio/src/app/studio/components/PageHeader.tsx`
  - 页面头部。
- `apps/jstudio/src/app/studio/components/PageCanvas.tsx`
  - 文档画布。
- `apps/jstudio/src/app/studio/editor/BlockEditor.tsx`
  - 块编辑器 MVP。
- `apps/jstudio/src/app/studio/editor/block-model.ts`
  - 块模型和转换。
- `apps/jstudio/src/app/studio/editor/markdown-codec.ts`
  - Markdown 兼容读写。
- `apps/jstudio/src/services/index.ts`
  - 增加 create page/template 组合能力，或继续复用 read/save/create。
- `apps/jstudio/src-tauri/src/services/file_service.rs`
  - 后续支持 metadata，第一轮可不动。

## 第一轮交付范围（建议这次就做）

为了真正体现“大刀阔斧”，第一轮不要只改文案，直接做出新壳和新编辑体验 MVP：

1. 新建 StudioApp 并切换入口。
2. 新建 WorkspaceSidebar，替代主界面的 FileTree。
3. 新建 PageHeader/PageCanvas，替代 EditorBar + 旧 MarkdownEditor 直出。
4. 新建轻量 BlockEditor MVP：paragraph、heading、todo、quote、code、divider。
5. 实现 block <-> Markdown 的基础转换和保存。
6. 保留旧 Reader/MarkdownEditor 作为 fallback，不在主入口展示。
7. 跑 `npm run build` 或项目已有 jstudio 构建命令验证。

## 风险

- 自研 BlockEditor 会触及光标、输入法、撤销栈等复杂问题；MVP 应保持简单：每个 block 一个 textarea，避免 contenteditable 的复杂坑。
- 第一次切换入口可能遗漏旧快捷键/关闭确认逻辑；需优先保留保存、打开目录、打开文件、关闭窗口这些基础能力。
- Markdown 复杂语法往返会有损耗；一期可以接受，后续通过 `.jstudio` 原生格式解决。

## 结论

按用户要求，应从“改造阅读器”升级为“重做 JStudio 产品壳 + 新建块编辑器 MVP”。旧 Markdown 能力作为兼容层复用，不再围绕它做产品设计。第一轮就应该让应用打开后完全不像 Markdown Reader，而像一个本地语雀/Notion。
