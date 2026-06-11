# Plan: migrate-local-notion-to-jstudio

## 目标

把 `apps/local-notion-&-knowledge-graph-editor/` 中的 OmniNote/本地知识库编辑器迁移到 `apps/jstudio/`，并补齐/对齐 Tauri 应用结构，使 jstudio 成为可开发、可构建的桌面应用。

## 已观察到的现状

- `apps/local-notion-&-knowledge-graph-editor/` 是 React + Vite + Tailwind v4 应用，核心文件在 `src/`：
  - `App.tsx`：主应用状态、文档 CRUD、主题、导入导出等。
  - `components/*`：文档列表、块编辑器、块渲染、本地附件面板、目录。
  - `data/defaultData.ts`：默认文档数据。
  - `types.ts`：核心类型。
- 原应用 `vite.config.ts` 已按 Tauri 生产资源做了关键配置：`base: './'`、`outDir: 'dist'`、`server.port=1420`、`chunkSizeWarningLimit=2000`、tldraw vendor chunk。
- `apps/jstudio/` 当前看起来只有前端脚手架文件（`package.json`、`vite.config.ts`、`index.html`、tsconfig/eslint/prettier 等），未发现 `src/` 或 `src-tauri/`。
- `apps/jstudio` 是主仓库的 git submodule；实现变更需要在子模块内修改，最后主仓库只更新子模块指针。

## 实施步骤

1. **确认 jstudio 基线与依赖**
   - 检查 `apps/jstudio/package.json` 的 scripts、dependencies、devDependencies。
   - 与源应用 `package.json` 对齐 React、Tailwind、lucide-react、tldraw、textarea-caret、Tauri CLI/API 等依赖。
   - 保留 jstudio 现有 lint/prettier/tsconfig 约定，避免无必要重写配置。

2. **迁移前端源代码**
   - 将源应用 `src/` 迁移到 `apps/jstudio/src/`。
   - 将源应用 `index.html` 的标题和入口保持/更新为 jstudio/OmniNote 合理名称。
   - 对齐 `vite.config.ts`：保留 jstudio 当前插件与别名，同时加入 Tauri 必需配置：`base: './'`、固定 dev server、build outDir、vendor chunks、chunk warning limit。
   - 如 jstudio tsconfig include/path alias 当前只适合根目录，调整为适配 `src` 应用代码。

3. **补齐 Tauri 工程骨架**
   - 在 `apps/jstudio/src-tauri/` 创建标准 Tauri v2 结构：`Cargo.toml`、`tauri.conf.json`、`src/main.rs`、`src/lib.rs`、必要图标/能力配置（若缺失则使用最小可构建配置）。
   - 配置 `beforeDevCommand` / `beforeBuildCommand` 调用前端 npm scripts，`devUrl` 指向 Vite 端口，`frontendDist` 指向 `../dist`。
   - 应用标识使用稳定值（例如 `com.jacklingo.jstudio`），窗口标题使用 `JStudio / OmniNote`。
   - 首阶段不强行把 LocalStorage 改成 Rust 文件存储；先保持现有离线数据模型，保证迁移后功能等价运行。后续如需要可再扩展 Tauri command 持久化到 app data 目录。

4. **修复迁移后的类型与 lint 问题**
   - 处理可能的 TypeScript 问题，例如 `textarea-caret` 类型声明缺失、React import 风格、未使用变量、`any`、`confirm/window` 调用等。
   - 对 tldraw CSS、Tailwind v4 `@import "tailwindcss"`、Vite alias 做构建验证。

5. **验证**
   - 在 `apps/jstudio` 运行 `npm install`（如 lockfile 需要更新）。
   - 运行前端构建：`npm run build`。
   - 如 Tauri CLI/环境可用，运行 `npm run tauri build` 或至少 `cargo check`（在 `src-tauri`）。
   - 在主仓库层面检查 git 状态，说明子模块变更与主仓库 submodule 指针状态。

## 风险与取舍

- `apps/jstudio` 是子模块：如果需要提交/推送，必须先处理子模块仓库，再更新主仓库指针；本次默认只改工作区，不自动 commit/push。
- Tauri 图标资源如果从零生成可能较繁琐；优先使用 Tauri 默认/最小资源，保证可运行，再按需美化。
- 源应用目前主要使用浏览器 `localStorage` 保存文档与附件；迁移为 Tauri 并不等同于自动物理文件持久化。若要真正落盘到 app data/SQLite，需要额外设计 Rust command/API 与前端适配。

