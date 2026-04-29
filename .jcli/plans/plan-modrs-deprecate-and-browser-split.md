# 执行计划：弃用 mod.rs + 拆分 browser.rs

## 任务概述

根据 AGENT.md 第6条规范，将两个模块从 `mod.rs` 模式改为 `name.rs` + `name/` 子目录模式。

---

## 任务 1：弃用 `markdown/render/mod.rs`

### 当前结构

```
src/markdown/
├── markdown.rs          # 顶层模块入口，声明 `pub mod render;`
└── render/
    ├── mod.rs           # 当前模块入口（需弃用）
    ├── block.rs
    ├── code_block.rs
    ├── inline.rs
    └── table.rs
```

### 目标结构

```
src/markdown/
├── markdown.rs          # 保持不变
├── render.rs            # 新模块入口（原 mod.rs 内容）
└── render/
    ├── block.rs         # 保持不变
    ├── code_block.rs    # 保持不变
    ├── inline.rs        # 保持不变
    └── table.rs         # 保持不变
```

### 执行步骤

1. **删除 `render/mod.rs`**
2. **创建 `render.rs`**，内容来自原 `mod.rs`：
   - 子模块声明：`mod block; mod code_block; pub mod inline; pub mod table;`
   - `RenderContext` 结构体定义
   - `render_document_wrapped()` 公开函数
3. **验证**：外部引用无需修改（`crate::markdown::render::xxx` 路径保持有效）

### 受影响文件

| 文件 | 变更 |
|------|------|
| `src/markdown/render/mod.rs` | 删除 |
| `src/markdown/render.rs` | 新建（原 mod.rs 内容） |

---

## 任务 2：拆分 `browser.rs`

### 当前结构

单文件 `src/command/chat/tools/browser.rs` (1522行)，包含：
- A. imports (行 1-10)
- B. `mod cdp { ... }` (行 12-816, `#[cfg(feature = "browser_cdp")]`)
- C. `mod lite { ... }` (行 820-1288, `#[cfg(not(feature = "browser_cdp"))]`)
- D. `BrowserParams` + `BrowserTool` + `Tool` trait (行 1290-1394)
- E. `exec_browser_cdp()`, `exec_browser_stub()`, `read_headless_config()` (行 1396-1522)

### 目标结构

```
src/command/chat/tools/
├── browser.rs           # BrowserParams + BrowserTool + Tool trait 实现 (~105行)
└── browser/
    ├── cdp.rs           # CDP 模块 (~805行)
    ├── lite.rs          # Lite 模块 (~470行)
    └── dispatch.rs      # exec_browser_cdp + exec_browser_stub + read_headless_config (~125行)
```

### 执行步骤

#### 2.1 创建目录结构

```bash
mkdir -p src/command/chat/tools/browser/
```

#### 2.2 创建 `browser/cdp.rs`

提取原文件行 12-816 的 `mod cdp { ... }` 内容：
- 移除外层 `mod cdp {` 和 `}` 包装
- 保留 `#[cfg(feature = "browser_cdp")]` 条件编译
- 暴露必要函数为 `pub(super)` 或 `pub(crate)` 供 `dispatch.rs` 调用

需要暴露的函数（原为 `pub`，改为 `pub(super)`）：
- `get_runtime()`
- `ensure_browser()`
- `status()`, `start()`, `stop()`, `list_tabs()`, `open_tab()`, `navigate()`
- `screenshot()`, `get_content()`, `click()`, `type_text()`, `press_key()`, `evaluate()`
- `close_tab()`, `snapshot()`, `exec_browser_async()`

#### 2.3 创建 `browser/lite.rs`

提取原文件行 820-1288 的 `mod lite { ... }` 内容：
- 移除外层 `mod lite {` 和 `}` 包装
- 保留 `#[cfg(not(feature = "browser_cdp"))]` 条件编译
- 暴露必要函数为 `pub(super)` 供 `dispatch.rs` 调用

需要暴露的函数：
- `status()`, `start()`, `stop()`, `list_tabs()`, `open_tab()`, `navigate()`
- `snapshot()`, `get_content()`, `screenshot()`, `close_tab()`

#### 2.4 创建 `browser/dispatch.rs`

提取原文件行 1396-1522：
- `exec_browser_cdp()` — CDP 入口（需调用 `cdp::get_runtime()` 和 `cdp::exec_browser_async()`）
- `exec_browser_stub()` — Lite 入口（需调用 `lite::*` 各函数）
- `read_headless_config()` — 配置读取（CDP 专属）

暴露为 `pub(super)` 供 `browser.rs` 的 `execute()` 调用。

#### 2.5 重构 `browser.rs`

保留行 1290-1394（BrowserParams + BrowserTool + Tool trait）：
- 添加子模块声明：
  ```rust
  #[cfg(feature = "browser_cdp")]
  mod cdp;
  #[cfg(not(feature = "browser_cdp"))]
  mod lite;
  mod dispatch;
  ```
- 在 `execute()` 中调用 `dispatch::exec_browser_cdp()` 或 `dispatch::exec_browser_stub()`

#### 2.6 更新可见性

原 `cdp` 和 `lite` 模块内部的 `pub fn` 需改为 `pub(super) fn`：
- `pub` → `pub(super)` （仅对被 dispatch 调用的函数）
- 私有函数保持 `fn` 不变

### 受影响文件

| 文件 | 操作 |
|------|------|
| `src/command/chat/tools/browser.rs` | 重写（仅保留 BrowserParams + BrowserTool + Tool trait + mod 声明） |
| `src/command/chat/tools/browser/cdp.rs` | 新建 |
| `src/command/chat/tools/browser/lite.rs` | 新建 |
| `src/command/chat/tools/browser/dispatch.rs` | 新建 |

### 外部引用（无需修改）

- `src/command/chat/tools/definition.rs:189` — 实例化 `BrowserTool`（路径不变）
- `src/command/chat/tools/tools.rs:41` — 引用 `BrowserTool::NAME`（路径不变）

---

## 执行顺序

1. 先完成 **任务 1**（markdown/render）— 简单，无复杂依赖
2. 再完成 **任务 2**（browser.rs 拆分）— 需处理 feature gate 和可见性

---

## 验证步骤

每个任务完成后执行：

```bash
cargo fmt
cargo clippy -- -D warnings
cargo build --features browser_cdp
cargo build --no-default-features
cargo test
```