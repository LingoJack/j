# AGENTS/CLAUDE

## 项目概述

jcli 是一个 AI 驱动的命令行工作台，包含别名管理、Agent 工作台、日报周报、待办备忘、Markdown 预览等功能。项目为个人使用，主要开发环境为 Windows。

## 开发命令

项目使用 Makefile 管理构建流程，常用命令：

```bash
# 构建发布版本
make release

# 从本地构建安装到系统
make install

# 格式化代码（必须通过）
make fmt

# Lint 检查（必须通过，CI 强制）
make lint

# 运行测试
make test

# 提交前检查（fmt + lint + test）
make pre-commit

# 递增版本号（同步 j-agent 和安装脚本）
make bump-version

# 发布到 crates.io
make publish
```

**完整命令列表**：运行 `make help` 查看。

### 本地开发快捷命令（仅个人环境）

`.cargo/config.toml` 定义了本地别名，不影响其他开发者：

```bash
# 构建并安装（开发首选）
cargo jb
# jb = "install --path . --features browser_cdp"
```

## 子模块

### jstudio（Tauri Reader 应用）

`apps/jstudio` 是一个 Tauri 桌面应用，提供文件阅读器功能（`j read <path>`）。

```bash
# 初始化子模块
make init-jstudio

# 开发模式（启动 Tauri dev）
make dev-jstudio

# 构建 Tauri 应用
make build-jstudio

# 安装到系统（macOS 安装到 /Applications）
make install-jstudio

# 提交子模块变更（非 AI）
make commit-jstudio

# 推送子模块（AI 生成 commit message）
make push-jstudio

# 查看子模块状态
make status-jstudio
```

子模块管理注意事项：
- 主仓库只保存 submodule 指针（特定 commit）
- 修改 jstudio 后需先 push 子模块，再更新主仓库指针
- `make push-jstudio` 会自动同步主仓库指针

## 架构概览

项目采用 **库 + 二进制分离** 架构：

- **`j-agent/`** — 核心引擎库（无 TUI 依赖，可被 CLI 和 GUI 共用）
  - `llm/` — LLM API 客户端（流式响应、错误处理）
  - `agent/` — Agent 主循环、工具处理、重试机制
  - `tools/` — 工具定义（Bash、Read、Write、Edit、Grep、Glob、Browser、Computer Use 等）
  - `context/` — 对话上下文管理、消息压缩、窗口策略
  - `permission/` — 工具执行权限确认队列
  - `infra/` — Hook 系统、Skill 系统、沙箱
  - `storage/` — 会话持久化、配置存储
  - `teammate/` — 多 Agent 协作

- **`src/`** — CLI/TUI 层（依赖 ratatui/crossterm）
  - `command/` — 命令实现（alias/chat/todo/report/notebook 等）
  - `command/chat/` — Chat TUI 界面（最大模块）
    - `app/` — 应用状态（ChatApp、Action、Session 等）
    - `handler/` — 事件处理（键盘、鼠标、WebSocket）
    - `render/` — UI 渲染（消息气泡、工具调用展示）
    - `input/` — 输入线程、自动补全
    - `oneshot/` — 非交互式快速提问模式
    - `remote/` — 手机远程控制 WebSocket 服务
  - `tui/` — TUI 组件（编辑器、选择器、Markdown 渲染）
  - `config/` — YAML 配置文件管理
  - `interactive/` — REPL 交互模式
  - `markdown/` — Markdown 解析与渲染
  - `util/` — 工具函数（日志、颜色适配、文件操作）

- **`apps/jstudio/`** — Tauri 桌面应用（submodule）
  - 文件阅读器 GUI，通过 `j read <path>` 调用
  - 与 j-cli 共用 j-agent 核心库

## 代码规范（来自 AGENTS.md）

### 格式与 Lint

- `cargo fmt` 必须通过
- `cargo clippy -- -D warnings` 必须无告警
- 命名：`PascalCase`（类型/Trait）、`snake_case`（函数/变量）、`SCREAMING_SNAKE_CASE`（常量）

### 内存与性能

- 避免非必要 `.clone()`，优先所有权转移或借用
- 参数优先切片（`&str`, `&[T]`）而非包装类型
- 集合预分配：已知大小时用 `with_capacity`

### 错误处理

- 非 test 代码避免 `unwrap()`/`expect()`，用 `?` 传播
- 错误类型：手写 enum + `impl std::error::Error` + `From` 转换

### 模块组织

- 弃用 `mod.rs`：采用 `name.rs` + `name/` 子目录
- 路径简化：禁止长路径引用（如 `a::b::c::Type`），就近 `use`
- 语义化分文件：按职责拆分，单一文件不堆叠不相关功能

### TUI 输出规范

- **禁止在 TUI 模式用 `println!`/`eprintln!`/`crate::info!`/`crate::error!`**
- 后台线程日志必须用 `crate::util::log::write_info_log()` 写文件

### 锁与死锁防范

- `std::sync::Mutex`/`RwLock` **不可重入**，同一线程二次 `lock()` 会永久阻塞
- **trait 方法保持纯查询**：`Tool::description()` 等 trait 方法内禁止 `lock()` 任何 Mutex
- **临界区不出锁**：持 `MutexGuard` 时不调用 trait 方法、虚函数、用户回调
- **持锁透传副作用 → 改 clone**：先 `guard.clone()` 再 `drop(guard)` 再调用下层

### 文档

- 公共 API 和核心类型必须有 `///` 文档注释
- `unsafe` 块上方必须有 `// SAFETY:` 注释

## 关键模块入口

| 入口 | 说明 |
|------|------|
| `src/main.rs` | CLI 入口，clap 解析 + 命令分发 |
| `src/lib.rs` | 库入口，重导出 YamlConfig 和 llm |
| `src/cli.rs` | SubCmd 枚举定义（所有命令） |
| `src/command.rs` | 命令分发函数 |
| `j-agent/src/lib.rs` | j-agent 库入口 |
| `j-agent/src/agent.rs` | Agent 主循环入口 |
| `j-agent/src/tools.rs` | Tool trait 和工具注册 |
| `src/command/chat.rs` | Chat 命令入口 |
| `src/command/chat/app/chat_app.rs` | ChatApp 核心状态 |