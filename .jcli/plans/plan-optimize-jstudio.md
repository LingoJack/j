# apps/jstudio 优化重构计划

## 目标
对 `apps/jstudio` 做一轮可落地的工程优化，优先提升可维护性、编辑器性能、缓存生命周期和工具扩展能力，同时保持现有功能行为不变。

## 范围
- 前端 React/TypeScript：`apps/jstudio/src/**`
- 必要时检查 Tauri Rust 后端：`apps/jstudio/src-tauri/src/**`
- 不改用户可见交互语义，除非是明确的性能/稳定性修复。

## 实施步骤

### 1. 建立共享工具注册表
- 新增或调整工具定义模块，例如 `src/toolRegistry.tsx`。
- 将工具 tab 的 title、icon、description、tone、component 统一收口。
- 让 `Toolbox`、`Reader`、`TabBar` 尽量从同一份 registry 读取工具元信息。
- 目标：新增工具时不再需要分散修改多处常量/switch。

### 2. 优化编辑器缓存层
- 给 `RenderCache` 增加容量上限和更明确的淘汰策略。
- 给 `InlineCache` 增加容量上限，并避免纯 hash 命中导致潜在冲突：缓存条目保存 `text` 或 `length + text` 校验。
- 暴露必要的 `size` / `clear` / `retain` 行为，便于调试和生命周期管理。
- 保持现有渲染 API 尽量兼容，避免一次性大改编辑器主体。

### 3. 优化 `MarkdownEditor.tsx` 热路径
- 检查 `renderDocument()`、`resetInlineCache()`、`replaceChildren()` 的调用链。
- 在不做大规模虚拟化的前提下，优先减少不必要的缓存全量清理。
- 将明显重复的 source 行拆分、block key 构造、渲染辅助逻辑抽小。
- 如果局部 DOM diff 风险过高，本轮先做“低风险缓存与生命周期优化”，避免破坏编辑体验。

### 4. 拆分/整理 `Reader.tsx`
- 抽出低风险纯工具：路径工具、localStorage 读写、tab 资源清理/迁移。
- 将 `TOOL_TITLES` 替换为工具 registry 查询。
- 将重复的 ref 桶删除/迁移操作封装为小 helper，降低 rename/delete/close 维护成本。
- 优化 tab 更新中的重复扫描：优先做局部 helper，不强行把数组状态改成 Map，避免影响 UI 顺序和大量调用点。

### 5. 优化 `FileTree.tsx`
- 优化 `toggleDir` 的闭包依赖，避免依赖整个 `nodes` 导致 handler 频繁变化。
- 抽出 `upsertNode`/`createLoadingNode` 等节点状态更新 helper。
- 对树节点渲染组件进行 `memo` 可行性检查；若现有递归函数结构改动过大，本轮先做状态更新和 handler 稳定性优化。

### 6. 验证
- 运行前端类型检查/构建：根据 `package.json` 脚本执行，例如 `npm run build` 或项目实际脚本。
- 如果涉及 Rust 后端，运行 `cargo fmt`、`cargo clippy -- -D warnings`，至少限定在 jstudio tauri crate 或工作区可用命令。
- 检查关键手动场景：打开文件、切 tab、保存、关闭 dirty tab、打开工具、文件树新建/重命名/删除、图片预览。

## 风险控制
- 不一次性引入虚拟列表或全量局部 DOM diff，这类改动风险较高。
- 优先小步重构，保持现有 props/API 兼容。
- 每个阶段后运行构建/类型检查，发现问题及时回退局部改动。

## 预期产出
- 更统一的工具注册机制。
- 有容量控制、冲突校验的编辑器缓存。
- `Reader.tsx` 中重复资源管理逻辑减少。
- `FileTree.tsx` 状态更新更稳定。
- 构建/检查通过。
