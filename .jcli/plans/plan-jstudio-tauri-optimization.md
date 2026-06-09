# JStudio Tauri 应用极致优化计划

## 项目现状概要

JStudio 是一个基于 **Tauri 2.x + React + TypeScript + Tailwind CSS v4** 的桌面 Markdown 编辑器 / 文件阅读器。

- **前端**：React 19, Vite 6, Tailwind CSS v4, 无路由库（单页）
- **后端**：Rust, Tauri 2.x, pulldown-cmark 用于 Markdown 解析
- **架构**：VS Code 风格布局（活动栏 + 侧栏 + TabBar + 编辑区 + TOC）
- **编辑器**：自研混合 Markdown 编辑器（渲染态/源码态切换，非 Milkdown/ProseMirror）

---

## 优化优先级排序（按铁律 §9 执行）

### 第一轮：Rust 后端架构重构（🔴 P0）

#### 1.1 引入 thiserror 统一错误处理
**问题**：当前 `lib.rs` 手写 `ReaderError` enum + `impl Error`，但 Cargo.toml 已有 `thiserror` 依赖标记为可用。Tauri 命令返回 `Result<T, String>`，丢失了结构化错误信息。

**方案**：
- 使用 `thiserror` 派生 `ReaderError`（已声明为依赖）
- 创建 `src-tauri/src/error.rs` 统一错误类型
- Tauri 命令层保持 `Result<T, String>` 序列化兼容（Tauri 2 的 command 必须返回可序列化类型），但内部统一走 `ReaderError`

**涉及文件**：
- `src-tauri/src/error.rs`（新建）
- `src-tauri/src/lib.rs`

#### 1.2 Rust 后端目录结构规范化
**问题**：所有业务逻辑（14 个 Tauri 命令 + 所有 helper 函数 + 所有 model 定义）堆叠在一个 `lib.rs`（455 行）文件中。

**方案**：按规范拆分为 `commands/`, `services/`, `models/` 结构：
```
src-tauri/src/
├── commands/          # Tauri 命令薄层
│   ├── mod.rs
│   ├── file.rs        # read_file, save_file, create_file/dir
│   ├── dir.rs         # list_dir, open_dir
│   ├── asset.rs       # read_asset
│   └── app.rs         # get_initial, quit_reader, parse_markdown
├── services/          # 业务逻辑
│   ├── mod.rs
│   ├── file_service.rs
│   └── dir_service.rs
├── models/            # 数据结构
│   ├── mod.rs
│   └── requests.rs    # SaveReq, CreateReq, PathReq, RenameReq
├── error.rs           # ReaderError
├── markdown/          # (保持不变)
├── renderer.rs        # (保持不变)
├── main.rs
└── lib.rs             # 只注册命令
```

**涉及文件**：多个新建 + lib.rs 瘦身

#### 1.3 移除 `run()` 中的 `.expect()`
**问题**：`lib.rs:474` 使用 `.expect("error while running jstudio")`，违反 AGENTS.md 规则。

**方案**：改为 `std::process::exit(1)` 或更优雅的错误处理。

---

### 第二轮：前端架构规范化（🟠 P1）

#### 2.1 前端目录结构重组
**问题**：所有 `.tsx` 文件扁平放在 `src/` 根目录（21 个文件），没有按功能/页面组织。

**方案**：按规范重组：
```
src/
├── app/                       # 主入口
│   └── reader/
│       ├── index.tsx           # Reader 主组件
│       ├── components/         # Reader 私有组件
│       │   ├── EditorBar.tsx
│       │   ├── EmptyState.tsx
│       │   └── ToolHost.tsx
│       └── hooks/
│           └── useDirtyTitle.ts
├── components/
│   ├── ui/                    # 原子 UI 组件
│   │   ├── Toast.tsx
│   │   ├── Splitter.tsx
│   │   ├── DialogButton.tsx
│   │   ├── ConfirmDialog.tsx
│   │   ├── CloseConfirmDialog.tsx
│   │   ├── QuitConfirmDialog.tsx
│   │   └── PromptDialog.tsx
│   └── business/              # 业务组件
│       ├── ActivityBar.tsx
│       ├── FileTree.tsx
│       ├── TabBar.tsx
│       ├── Toolbox.tsx
│       ├── TableOfContents.tsx
│       ├── ImageViewer.tsx
│       ├── PlainTextEditor.tsx
│       ├── DiffTool.tsx
│       └── JsonTool.tsx
├── editor/
│   ├── MarkdownEditor.tsx
│   ├── MarkdownIR.tsx
│   ├── code-highlight.ts
│   ├── inline-renderer.ts
│   ├── cache.ts
│   └── editor.css
├── services/                  # IPC 封装层
│   └── index.ts               # api.ts 重命名
├── types/
│   └── index.ts
├── utils/
│   ├── slug.ts
│   ├── toc.ts
│   ├── fileIconKind.ts
│   ├── codeLanguage.ts
│   └── assetUrl.ts
├── styles/
│   └── reader.css
├── Icon.tsx                   # 图标组件放根目录
└── main.tsx
```

#### 2.2 提取 `services/` 层，封装 IPC 调用
**问题**：`api.ts` 已经是良好的 IPC 封装，但文件名不符合规范命名。

**方案**：将 `api.ts` 移动到 `services/index.ts`，确保组件通过 services 层调用 invoke。

#### 2.3 Reader.tsx 拆分
**问题**：`Reader.tsx` 有 1105 行，超过 250 行上限。包含 `EditorBar`, `EmptyState`, `WelcomeSection`, `WelcomeAction`, `ToolHost`, `useDirtyTitle`, `breadcrumb` 等内部组件和函数。

**方案**：
- `EditorBar` → `app/reader/components/EditorBar.tsx`
- `EmptyState` + `WelcomeSection` + `WelcomeAction` → `app/reader/components/EmptyState.tsx`
- `ToolHost` → `app/reader/components/ToolHost.tsx`
- `useDirtyTitle` → `app/reader/hooks/useDirtyTitle.ts`
- helper 函数 (`docToTab`, `ingestDoc`, `breadcrumb`, `filenameFromPath`, `isSameOrChildPath`, `rebasePath`) → `utils/path.ts` + `utils/doc.ts`

---

### 第三轮：性能优化（🟠 P1）

#### 3.1 Rust: Tauri 命令异步化
**问题**：所有 Tauri 命令都是同步函数 (`fn`)，在 Tauri 主线程上执行阻塞 I/O（`std::fs::read`, `std::fs::write`, `std::fs::read_dir` 等），会阻塞 IPC 线程池。

**方案**：将所有涉及 I/O 的命令改为 `async fn`，内部用 `tokio::task::spawn_blocking` 包裹同步文件操作。需要添加 `tokio` 依赖。

**涉及文件**：
- `src-tauri/Cargo.toml` — 添加 `tokio` 依赖
- 所有 Tauri 命令函数

#### 3.2 前端: Markdown 编辑器 parse debounce 优化
**问题**：当前 `MarkdownEditor.tsx` 的 `scheduleParse` 使用 250ms 固定 debounce。对于大文件可能过于频繁触发 IPC。

**方案**：
- 自适应 debounce：小文档（<500 行）250ms，中等（500-2000 行）500ms，大文档（>2000 行）800ms
- 增量解析提示：如果 source 与上次差异只有最后几行变化，可以提示后端跳过未变 block

#### 3.3 前端: MarkdownEditor DOM 操作优化
**问题**：`renderDocument()` 在每次 `activeRange` 变化时调用 `host.replaceChildren()` 重建整个 DOM 树。虽然编辑器的 block 级 DOM 重建是必要的，但可以优化：
- 每次 parseAndRender 也全量重建 DOM（即使只有一小部分 block 变化）

**方案**：
- 增量 DOM 更新：对比新旧 blocks，只更新变化的部分
- 或：保持当前全量重建策略，但使用 `requestAnimationFrame` 批量更新避免布局抖动

#### 3.4 图片加载优化：使用 Tauri Asset Protocol
**问题**：`read_asset` 命令通过 IPC 将整个图片二进制数据以 `Vec<u8>` 序列化为 JSON 传给前端。对于大图片（最大 128MB），这会导致：
- JSON 序列化/反序列化开销极大
- 内存峰值 = 文件大小 × 3（Rust buffer + JSON string + JS Uint8Array）
- 阻塞 IPC 线程

**方案**：`tauri.conf.json` 已启用 `assetProtocol`，前端可直接通过 `convertFileSrc()` 构建本地文件 URL 加载图片，完全绕过 IPC。

**涉及文件**：
- `src/ImageViewer.tsx` — 改用 asset protocol URL
- `src-tauri/src/lib.rs` — `read_asset` 可标记为 deprecated 或保留作兼容

---

### 第四轮：代码质量 & 可维护性（🟡 P2）

#### 4.1 移除 `read_asset` IPC 中的大文件传输
**方案**：已在 3.4 覆盖。

#### 4.2 统一 constants 管理
**问题**：魔法值散落各处。例如：
- `MAX_TABS = 32` 在 `Reader.tsx`
- `SIDEBAR_DEFAULT/MIN/MAX` 在 `Reader.tsx`
- `MAX_FILE_SIZE`, `MAX_ASSET_SIZE` 在 `lib.rs`
- localStorage keys 散落各处

**方案**：前端创建 `constants/` 目录集中管理；Rust 侧保持现有 `const` 但加文档注释。

#### 4.3 TypeScript 严格模式验证
**问题**：需验证 `tsconfig.app.json` 是否开启 `strict: true`。

**方案**：检查并确保严格模式开启，修复所有类型问题。

---

### 第五轮：交互与 UX 细节（🟢 P3）

#### 5.1 窗口最小尺寸规范化
**问题**：`tauri.conf.json` 设 `minWidth: 900, minHeight: 600`，规范要求 `800×600`。

**方案**：确认 900px 是有意为之（考虑到侧栏 + 编辑区 + TOC 的三栏布局），保持不变但加注释说明原因。

#### 5.2 ESC 关闭行为一致性
**问题**：需要确保所有弹窗、下拉菜单、抽屉都支持 ESC 关闭。

**方案**：审计所有 modal 组件，确认 ESC 行为。

#### 5.3 动效降级
**问题**：需检测 `prefers-reduced-motion` 并降级动效。

**方案**：在 CSS 中添加 `@media (prefers-reduced-motion: reduce)` 规则。

---

## 实施路线

每轮改动不超过 **3 个不相关文件**，保持聚焦：

1. **Round 1** — Rust 错误处理统一（error.rs + lib.rs 重构）
2. **Round 2** — Rust 目录结构拆分（commands/services/models）
3. **Round 3** — 前端 services 层 + constants 规范化
4. **Round 4** — 前端 Reader.tsx 拆分
5. **Round 5** — Rust async 命令化
6. **Round 6** — 图片 Asset Protocol 优化
7. **Round 7** — MarkdownEditor DOM 增量更新
8. **Round 8** — UX 细节打磨

---

## 风险评估

- **Rust 目录重构**：低风险，纯文件搬迁，逻辑不变
- **async 命令化**：中等风险，需引入 tokio，需全面测试
- **前端目录重组**：低风险，但 import 路径全面变更，需确保构建通过
- **Reader.tsx 拆分**：低风险，纯组件提取
- **Asset Protocol 切换**：低风险，tauri.conf.json 已配置好
