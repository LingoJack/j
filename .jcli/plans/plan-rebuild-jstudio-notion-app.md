# 重构 apps/jstudio 为本地 Notion 类文档工作台

## 当前状态结论

- `apps/jstudio` 已是 Tauri 2 + React 19 + TypeScript + Vite 项目，并已接入 Tailwind CSS v4（`@tailwindcss/vite`、`tailwindcss`）。
- 现有功能仍偏“Markdown 文件阅读/轻编辑器”：
  - 前端入口：`src/main.tsx` -> `src/app/studio/StudioApp.tsx`。
  - 当前数据模型：`DocBlock` 只覆盖 heading / paragraph / todo / quote / code / divider / bullet / numbered。
  - 存储方式：直接读写 Markdown 文件，Rust 端提供文件读写、列目录、打开目录等命令。
  - 样式有两套遗留：`reader.css` Tailwind token，以及 `studio.css` 大量传统 CSS。
- Rust/Tauri 后端已有可复用基础：文件读写、目录选择、显示文件夹、Markdown 解析等，但要升级为 Notion 类应用，需要新增 workspace / document / block / graph / plugin / sync 等本地优先 API。

## 重构目标

将废弃 reader/studio 项目推倒重做为一个本地优先的文档编写软件，核心能力包括：

1. 文档内 HTML 在线渲染，用于 presentation、交互 demo、嵌入式小工具。
2. 双向链接与知识图谱可视化。
3. 离线优先本地数据存储，默认隐私安全。
4. 块编辑能力：代码块、表格、画板、引用块、图片、内嵌子文档、链接、折叠块等。
5. `/` 快捷唤出命令菜单。
6. 插件系统，允许扩展块类型、命令和面板。
7. 暗黑模式。
8. 跨平台同步的预留与首版基础实现。
9. 技术栈保持 Tailwind CSS v4 + Tauri + React。

## 实施策略

采用“保留壳、重做内核”的方式：

- 保留：Tauri 2 项目配置、Vite/React/Tailwind v4 基础、Rust 文件系统能力中可复用函数。
- 替换：现有 reader/studio UI、Markdown-only block model、传统 `studio.css` 页面结构。
- 新增：本地 workspace 数据层、块编辑器核心、链接索引、图谱视图、HTML 沙箱预览、插件注册中心、主题系统和同步接口。

## 推荐首版数据格式

为兼顾本地可读、离线优先和未来同步，建议 workspace 结构如下：

```text
<workspace>/
  .jstudio/
    workspace.json
    index.json
    graph.json
    plugins/
    attachments/
    sync/
  pages/
    <page-id>.json
```

`pages/<page-id>.json` 存储结构建议：

```ts
interface PageDoc {
  id: string
  title: string
  icon?: string
  createdAt: number
  updatedAt: number
  blocks: BlockNode[]
}

interface BlockNode {
  id: string
  type:
    | 'paragraph'
    | 'heading'
    | 'todo'
    | 'quote'
    | 'code'
    | 'table'
    | 'canvas'
    | 'image'
    | 'html'
    | 'embed'
    | 'link'
    | 'toggle'
    | 'divider'
  props: Record<string, unknown>
  children?: BlockNode[]
}
```

说明：

- 首版优先用 JSON block tree 作为真实数据源，而不是 Markdown；必要时再做 Markdown 导入/导出。
- `[[页面标题]]` 或 `@page-id` 形式用于双链，保存时或后台索引时提取到 `graph.json/index.json`。
- 图片、画板资源进入 `.jstudio/attachments/`，页面只保存相对引用。

## 前端重构模块

建议重建 `src/app/studio` 下的模块结构：

```text
src/app/studio/
  StudioApp.tsx
  model/
    block.ts
    page.ts
    graph.ts
    plugin.ts
    workspace.ts
  store/
    workspace-store.ts
    editor-store.ts
  services/
    studio-api.ts
  editor/
    BlockEditor.tsx
    BlockRenderer.tsx
    SlashMenu.tsx
    block-registry.tsx
    blocks/
      ParagraphBlock.tsx
      HeadingBlock.tsx
      CodeBlock.tsx
      TableBlock.tsx
      CanvasBlock.tsx
      QuoteBlock.tsx
      ImageBlock.tsx
      HtmlBlock.tsx
      EmbedBlock.tsx
      LinkBlock.tsx
      ToggleBlock.tsx
  layout/
    Sidebar.tsx
    Topbar.tsx
    RightPanel.tsx
    StatusBar.tsx
  graph/
    KnowledgeGraph.tsx
    BacklinksPanel.tsx
  plugins/
    PluginManager.tsx
    builtins.ts
  theme/
    theme.ts
```

首版 UI 布局：

- 左侧：workspace / 页面树 / 快速搜索。
- 中间：块编辑器画布。
- 右侧：大纲、反向链接、页面属性、插件面板，可折叠。
- 顶部：页面标题、保存状态、主题切换、同步状态、图谱入口。
- 弹层：`/` 命令菜单、页面链接搜索、插件管理。

## Rust/Tauri 后端重构模块

建议新增/调整 `src-tauri/src`：

```text
src-tauri/src/
  lib.rs
  commands/
    workspace.rs
    page.rs
    graph.rs
    attachment.rs
    plugin.rs
    sync.rs
  services/
    workspace_service.rs
    page_service.rs
    graph_service.rs
    attachment_service.rs
    plugin_service.rs
    sync_service.rs
  models/
    workspace.rs
    page.rs
    graph.rs
    plugin.rs
```

命令首版范围：

- `open_workspace(path?)`：打开或初始化 workspace。
- `list_pages(workspace)`：列页面树。
- `get_page(page_id)` / `save_page(page)` / `create_page(parent_id?)` / `delete_page(page_id)`。
- `rebuild_graph()` / `get_graph()` / `get_backlinks(page_id)`。
- `import_attachment(path)`：复制图片等资源到 workspace。
- `list_plugins()` / `set_plugin_enabled(id, enabled)`。
- `get_sync_status()` / `export_sync_snapshot()`：首版先做同步预留与快照导出。

## 功能分期

### P0：可运行骨架

- 清理旧 Reader/Studio 代码路径，统一入口为新 `StudioApp`。
- Tailwind v4 作为主要样式方案，尽量移除传统 `studio.css` 大块样式。
- 建立 workspace 初始化、页面创建、页面保存、页面树浏览。
- 基础暗黑模式：`data-theme="dark"` + Tailwind token。

### P1：块编辑 MVP

实现可稳定编辑和持久化的基础块：

- paragraph / heading / todo / quote / divider。
- code block：语言字段、代码编辑、复制按钮。
- table block：基础行列编辑。
- image block：导入本地图片到 attachments。
- link block：普通 URL 和页面链接。
- toggle block：折叠块，children 内嵌子块。
- `/` SlashMenu：输入 `/` 唤出，选择块类型转换或插入。

### P2：HTML 在线渲染与交互

- 新增 `html` block：编辑源码 + 预览双模式。
- 使用 sandbox iframe 渲染，默认禁用危险能力；本地模式下通过用户确认开启脚本。
- 提供 presentation 模板块：HTML slides、交互 demo、可嵌入小组件。
- 支持从 HTML block 复制/导出为单文件 HTML。

安全建议：

- iframe 使用 `sandbox`。
- 默认只允许 `allow-scripts`，不允许 `allow-same-origin`，除非用户在设置里打开。
- 外链资源加载给出提示或开关。

### P3：双链与知识图谱

- 在保存页面时提取 `[[page title]]`、`@page-id`、URL link 等引用。
- 建立 `graph.json`：nodes/pages + edges/links。
- 右侧面板显示当前页 backlinks 和 outgoing links。
- 图谱视图首版用 SVG/Canvas 自实现力导向简化版，避免引入过重依赖；后续可换 d3/force-graph。
- 支持点击节点打开页面。

### P4：画板、内嵌子文档、插件系统

- Canvas block：首版可做轻量白板 JSON 数据（矩形、文本、线条），后续再增强手绘。
- Embed subdocument：block 引用另一个 page_id，可内联预览/跳转。
- Plugin API：
  - 注册 block type。
  - 注册 slash command。
  - 注册右侧 panel。
  - 注册导入/导出器。
- 内置插件示例：字数统计、Mermaid/HTML presentation、简单看板。

### P5：同步能力

首版不建议直接做云服务后端，先做跨平台同步接口：

- workspace 使用纯文件目录，天然支持 iCloud Drive / OneDrive / Dropbox / Syncthing。
- 每个 page 独立 JSON 文件，降低冲突范围。
- `.jstudio/sync/` 保存设备 ID、版本向量、最近同步状态。
- 提供“导出同步快照/导入快照”。
- 后续可扩展 WebDAV/Git/自建服务同步插件。

## 关键技术取舍

1. 编辑器不建议首版引入重型 ProseMirror/Lexical，除非要做接近 Notion 的复杂富文本体验；当前需求可先以自研 block editor + contenteditable/textarea 混合实现。
2. 数据源从 Markdown 转为 JSON block tree，Markdown 作为导入导出格式，避免表格、画板、HTML、子文档等信息丢失。
3. HTML 渲染必须沙箱化，避免本地隐私风险。
4. 插件系统首版先做“内置插件注册协议”，再开放加载 workspace `.jstudio/plugins` 中的插件清单。
5. 同步首版以文件夹级同步兼容为主，不在第一阶段引入远端账号系统。

## 验证计划

- 前端：`npm run build`。
- Tauri/Rust：`cargo check` 或 `npm run tauri build`。
- 根仓库要求：必要时运行 `make fmt`、`make lint`、`make test`。
- 手动验收：
  - 新建 workspace、新建页面、保存后重启仍存在。
  - 插入所有基础块并保存/重载不丢失。
  - `/` 菜单能搜索和插入块。
  - HTML block 能预览且被 iframe 隔离。
  - 双链能产生 backlinks 和图谱边。
  - 暗黑模式切换正常。

## 需要用户确认的点

1. 是否同意把真实文档存储从 Markdown 文件改为 `.jstudio/pages/*.json` 的 block tree？如果不同意，可保留 Markdown 为主存储，但画板、HTML、折叠块、子文档会需要额外 frontmatter/sidecar 文件。
2. 首版同步是否接受“文件夹级同步兼容 + 快照导入导出”，云端账号/服务后续插件化？
3. HTML block 是否默认允许执行脚本？推荐默认沙箱预览，用户按块授权开启脚本。

## 预计实施顺序

1. 搭建新数据模型与 Tauri workspace/page 命令。
2. 替换前端应用壳和 Tailwind 主题。
3. 实现基础页面树和页面 CRUD。
4. 实现 block editor 与 `/` 菜单。
5. 添加高级块：代码、表格、图片、HTML、toggle、embed、canvas。
6. 实现链接索引、backlinks、图谱视图。
7. 实现插件注册中心和内置插件示例。
8. 添加暗黑模式、同步状态、构建验证。
