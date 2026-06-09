# 优化 apps/jstudio 前端页面计划

## 目标

对 `apps/jstudio/` 的 Reader 前端做一轮偏用户体验的优化，重点不只是视觉样式，还包括“不符合逻辑”的交互修正、减少阻塞弹窗、补充常用入口和空状态引导。

## 已确认现状

技术栈与入口：

- React + TypeScript + Vite，入口在 `src/main.tsx`，主组件为 `src/Reader.tsx`。
- 样式主要使用 Tailwind CSS v4 utilities，设计令牌集中在 `src/reader.css`。
- 主要页面结构：
  - `ActivityBar.tsx`：左侧活动栏，文件/工具箱/设置。
  - `FileTree.tsx`：文件树、目录切换、新建、重命名、删除、右键菜单。
  - `TabBar.tsx`：顶部 Tab 栏。
  - `Reader.tsx`：主状态、快捷键、编辑器/工具路由、空状态、Toast。
  - `Toolbox.tsx`、`DiffTool.tsx`、`JsonTool.tsx`：工具面板和工具 Tab。

## 发现的问题与优化点

### 1. 文件树删除仍使用 `window.confirm` / `window.alert`

当前 `FileTree.tsx` 的删除确认和部分错误提示仍使用浏览器原生弹窗：

- 与项目已有的 `Toast`、`PromptDialog` 视觉体系不一致。
- 原生弹窗阻塞页面，体验突兀。
- 删除危险操作可读性弱，无法展示更细粒度说明。

计划：新增/复用一个自定义确认弹窗组件，用于删除文件/文件夹；错误通过父级 `Toast` 或 `FileTree` 内部非阻塞提示展示。

### 2. 文件树过滤逻辑只过滤当前已展开目录的直接子项

当前过滤框提示为“过滤当前目录”，实际递归节点中每层只对本层 entries 做 `includes`。用户可能误以为是全局搜索。

计划：

- 保持轻量过滤，不改成后端全局搜索，避免范围扩大。
- 文案改得更明确，例如“过滤已展开项”。
- 过滤时增加清晰的结果/无结果提示，避免用户不知道为什么看不到未展开目录内文件。

### 3. 空状态缺少可点击主操作

`EmptyState` 只展示说明文字和快捷键，用户仍需要自己去左侧文件树或工具栏操作。

计划：

- 让空状态支持 `onOpenRoot`、`onSelectActivity`、`onOpenTool` 等主操作。
- 增加卡片式快捷入口：打开目录、打开工具箱、打开 JSON 查看器、打开文本 Diff。
- 仍保留快捷键提示。

### 4. Tab 栏缺少常见右键/批量关闭体验

当前 Tab 只有点击切换、关闭按钮、中键关闭。打开多个文件后，缺少：

- 关闭其他 Tab。
- 关闭右侧 Tab。
- 复制路径。
- 在访达中显示。

计划：增加 Tab 右键菜单，复用已有 `showInFolder`、`copyPath`、`requestCloseTab` 逻辑；对 dirty Tab 仍走现有确认流程，不绕过安全机制。

### 5. 设置菜单可扩展但目前偏少

当前只有主题切换。可以增加更直接的体验项：

- TOC 默认固定/取消固定入口，和右侧 TOC 按钮呼应。
- 重置侧栏宽度。
- 显示快捷键说明。

计划：优先加入“重置侧栏宽度”和“快捷键提示”，TOC 设置视代码耦合程度决定是否加入。

### 6. 文件树右键菜单定位可能溢出视口

当前 context menu 直接用鼠标坐标 `left/top`，靠近窗口右下角时可能超出屏幕。

计划：增加菜单定位修正，按菜单估算宽高或渲染后测量，确保不溢出视口。

### 7. `useDirtyTitle` 的 `pagehide` 监听清理不完整

`Reader.tsx` 中 `beforeunload` listener 会清理，但 `pagehide` 使用匿名函数注册，effect cleanup 只移除了 beforeunload，没有移除 pagehide。虽然影响较小，但逻辑不完整。

计划：提取具名 `pagehide` handler，并在 cleanup 中一起移除。

## 实施步骤

1. **补充通用 UX 组件**
   - 新增或复用确认弹窗，用于删除确认。
   - 如果需要，新增轻量 `ContextMenu`/`MenuButton` 抽象，避免 TabBar/FileTree 重复太多样式。

2. **优化 FileTree**
   - 删除确认改自定义弹窗。
   - 错误提示改为非阻塞方式，必要时通过 `onNotify` 回调交给 `Reader` 的 Toast。
   - 过滤框文案和无结果提示优化。
   - 右键菜单加入视口边界处理。

3. **优化 Reader 空状态与主操作入口**
   - `EmptyState` 改为接收回调。
   - 添加“打开目录”“打开工具箱”“文本 Diff”“JSON 查看器”等按钮。
   - 保持当前快捷键说明。

4. **增强 TabBar**
   - 扩展 props：复制路径、在访达中显示、关闭其他/右侧等。
   - 增加右键菜单。
   - 批量关闭时逐个调用 `requestCloseTab`，dirty Tab 仍弹现有确认；如果实现复杂，先只做单 Tab 的复制路径/访达显示/关闭。

5. **设置菜单小增强**
   - 增加“重置侧栏宽度”。
   - 增加快捷键提示入口或在 EmptyState 中强化快捷键说明。

6. **修复逻辑小问题**
   - 修复 `useDirtyTitle` 中 `pagehide` listener cleanup。
   - 检查按钮可访问性：`type="button"`、`aria-label`、键盘操作。

7. **验证**
   - 在 `apps/jstudio` 下运行前端格式化/构建检查（优先 `npm run build`，如 package scripts 支持再运行 lint）。
   - 回到项目根按要求运行必要的格式化/检查：涉及 Rust 代码时运行 `cargo fmt`、`cargo clippy -- -D warnings`；若只改前端 TS/CSS，则至少运行前端构建。

## 非目标

- 不改后端 API 语义。
- 不引入大型 UI 组件库。
- 不做全局文件内容搜索（只优化当前过滤体验），除非后续明确需要。
- 不大规模重写编辑器核心，避免影响 Milkdown/CodeMirror 性能路径。

## 风险与控制

- Tab 批量关闭涉及 dirty 文件安全：必须复用现有关闭确认，不直接丢弃。
- FileTree 状态刷新涉及路径迁移：只做 UI 层改造，不改后端路径逻辑。
- 空状态增加操作入口时，需要避免在未初始化 `treeRoot` 时触发无效打开。
