# Plan: migrate-knowledge-app-to-jstudio

## 目标

将 `apps/local-notion-&-knowledge-graph-editor/` 这个 MVP 完整迁移到 `apps/jstudio/`。用户明确表示 `jstudio` 现有代码都可以不要，因此以“替换式迁移”为主：保留 Tauri 外壳与必要构建配置，把前端产品实现替换为 local-notion MVP，并清理旧 Reader/JStudio 代码。

## 当前观察

- 源 MVP 是 Vite + React + Tailwind 项目，核心文件较少：
  - `src/App.tsx`
  - `src/main.tsx`
  - `src/index.css`
  - `src/types.ts`
  - `src/components/*`
  - `src/data/defaultData.ts`
  - `package.json` / `package-lock.json` / `tsconfig.json` / `vite.config.ts`
- 目标 `apps/jstudio/` 是 Tauri v2 + React 项目，已有大量 Reader/Studio 代码与 Rust 后端命令。
- `apps/jstudio/index.html` 当前 root id 是 `reader-root`，而源 MVP 用 `root`。
- `apps/jstudio/vite.config.ts` 包含 Tauri 需要的配置：`base: './'`、`outDir: 'dist'`、dev server `127.0.0.1:1420`、Tailwind/React 插件。
- `apps/jstudio/src-tauri/` 可以继续保留最小 Tauri 启动壳；前端 MVP 目前看起来是本地状态 Demo，不一定需要调用后端命令。

## 实施步骤

1. 备份/确认目标目录结构，不删除构建产物以外的必要 Tauri 配置：
   - 保留 `apps/jstudio/src-tauri/`、`apps/jstudio/vite.config.ts`、`apps/jstudio/index.html`、package 管理文件的 Tauri 依赖部分。
   - 清理或替换 `apps/jstudio/src/` 下旧 Reader/Studio 前端代码。

2. 迁移源 MVP 前端代码到 `apps/jstudio/src/`：
   - 复制 `App.tsx`、`types.ts`、`components/`、`data/`、`index.css`。
   - 将 `src/main.tsx` 调整为 jstudio 的入口，挂载到 `#root` 或同步修改 `index.html`。
   - 保持 Tailwind 入口 `@import "tailwindcss";`。

3. 合并构建配置与依赖：
   - 以 `apps/jstudio/package.json` 为目标，加入源 MVP 所需 dependencies/devDependencies/scripts。
   - 保留 Tauri 脚本（如 dev/build Tauri）和 `@tauri-apps/*` 依赖；删除旧 Reader 相关但 MVP 不需要的依赖（如无引用）。
   - 保留 `apps/jstudio/vite.config.ts` 的 Tauri 生产配置，同时合并源项目 alias `@ -> .`（如源代码依赖）。

4. Tauri 后端最小化：
   - 如果迁移后的前端不调用 Tauri invoke，则把 `src-tauri/src/lib.rs` 简化为只启动 `tauri::Builder::default()`，保留必要 plugin（如 dialog，可选）。
   - 同步精简 `src-tauri/Cargo.toml` 依赖，避免保留旧 markdown/file reader 服务导致编译复杂度和无用代码。
   - 保留 `src-tauri/src/main.rs`、`build.rs`、`tauri.conf.json`、icons、capabilities 等必要文件。

5. 清理旧代码：
   - 删除 `apps/jstudio/src/app/reader`、`src/app/studio`、旧 reader/editor/tool 相关文件。
   - 删除 `src-tauri/src/commands`、`models`、`services`、`markdown`、`renderer.rs` 等不再使用的后端模块（若 lib.rs 已不引用）。
   - 不主动删除 `node_modules`/`dist`/`target`，除非验证或构建过程需要；这些一般由 gitignore 处理。

6. 验证：
   - 在 `apps/jstudio` 执行 `npm install`（如 package-lock 变更或依赖缺失）。
   - 执行 `npm run build` 验证前端 TypeScript/Vite 构建。
   - 执行 `npm run tauri build` 或至少 `cargo check --manifest-path apps/jstudio/src-tauri/Cargo.toml` 验证 Tauri/Rust 后端。
   - 如主仓库有 Makefile 集成，可视情况运行 `make build-jstudio`。

## 风险与注意

- 目标 `apps/jstudio` 是 git submodule，需要注意最终主仓库只记录子模块指针；如果需要提交，应先在子模块内提交/推送，再更新主仓库指针。
- `&` 出现在源目录名中，Shell 命令必须对路径加引号；文件操作优先用工具避免路径转义问题。
- 如果源 MVP 使用浏览器 localStorage 作为数据持久化，迁移到 Tauri 后仍可运行，但桌面端本地文件/工作区能力不会自动具备；本次按“完整迁移 MVP”优先保持产品现状。
- 若源代码依赖 Gemini/API 环境变量，需要确认 `.env.example` 与 Vite env 约定是否要同步到 jstudio。

## 建议验证完成标准

- `apps/jstudio` 启动后展示 local-notion/knowledge-graph MVP，而非旧 Reader/JStudio。
- `npm run build` 通过。
- Tauri 后端 `cargo check` 通过。
- 没有旧 Reader/Studio 前端 import 残留。

