# 开发者快速上手指南

> 本文档帮助新开发者在 10 分钟内搭建开发环境、理解项目结构、并完成第一次构建运行。

---

## 前置条件

| 依赖 | 最低版本 | 安装方式 |
|------|---------|---------|
| **Rust toolchain** | 2024 edition | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| **macOS ARM64** | M1+ | 本项目目标平台 |
| **Git** | 2.x | `brew install git` 或 Xcode 自带 |
| **make**（可选） | — | macOS 自带 |

验证环境：

```bash
rustc --version   # 确认 Rust 已安装
cargo --version   # 确认 Cargo 可用
```

---

## 1. 克隆与首次构建

```bash
# 克隆仓库
git clone https://github.com/LingoJack/j.git
cd j

# Debug 构建（首次约 1-2 分钟，后续增量编译很快）
cargo build

# 验证构建成功
cargo run -- version
```

> **注意**：项目使用 `rsproxy-sparse`（中国 crates.io 镜像），如果你在海外网络环境，可修改 `.cargo/config.toml` 中的 registry 配置。

---

## 2. 运行与体验

```bash
# 直接运行（进入交互模式 REPL）
cargo run

# 运行具体命令
cargo run -- help               # 查看帮助
cargo run -- set test-alias https://example.com   # 设置一个测试别名
cargo run -- ls                 # 列出已有别名
cargo run -- chat               # 进入 AI 对话 TUI（需先配置 API Key）

# 安装到系统（Release 构建 + 复制到 /usr/local/bin/j）
make install
j version                       # 验证安装
```

---

## 3. 项目结构速览

```
j/
├── Cargo.toml                 # 项目配置、依赖声明、patch 声明
├── Makefile                   # 开发快捷命令
├── CLAUDE.md                  # Claude Code 指导文件
├── README.md                  # 用户文档（修改功能时同步更新）
├── assets/                    # 编译时嵌入的资源文件
│   ├── help.md                #   帮助文档（j help 输出）
│   └── version.md             #   版本信息模板
├── docs/readme/               # 详细文档
├── patches/                   # 本地 patch 的第三方 crate（tui-textarea）
├── .github/workflows/         # CI/CD（tag 触发 Release 构建）
└── src/
    ├── main.rs                # 入口：无参数→REPL，有参数→clap 解析
    ├── cli.rs                 # clap derive 定义所有子命令（SubCmd 枚举）
    ├── constants.rs           # 全局常量（版本号、section 名、分类等）
    ├── assets.rs              # rust-embed 资源嵌入
    ├── config/
    │   └── yaml_config.rs     # ~/.jdata/config.yaml 的 serde 结构体 + 读写
    ├── command/               # ⭐ 命令处理层（核心开发区域）
    │   ├── handler.rs         #   dispatch(SubCmd) 主分发路由
    │   ├── open.rs            #   别名打开（核心：路径/URL/脚本/App 识别）
    │   ├── alias.rs           #   set/remove/rename/modify
    │   ├── report.rs          #   日报系统
    │   ├── chat/              #   AI 对话 TUI（最复杂的模块）
    │   ├── todo/              #   待办备忘录 TUI
    │   └── ...                #   其他命令
    ├── interactive/           # 交互模式（REPL + Tab 补全）
    ├── tui/                   # 全屏编辑器（ratatui + patched tui-textarea）
    └── util/                  # 工具函数（日志宏、Markdown 渲染、模糊匹配）
```

---

## 4. 核心架构理解

### 命令执行流程

```
用户输入
  │
  ├─ 无参数 ──────────────→ interactive::run_interactive()  (REPL 循环)
  │
  └─ 有参数 ──→ Cli::try_parse()
                  │
                  ├─ 解析成功 + 有子命令 ──→ command::handler::dispatch(SubCmd)
                  │                              │
                  │                              ├─ SubCmd::Set { .. }    → alias::handle_set()
                  │                              ├─ SubCmd::Report { .. } → report::handle_report()
                  │                              ├─ SubCmd::Chat { .. }   → chat::handle_chat()
                  │                              └─ ...
                  │
                  ├─ 解析成功 + 无子命令 ──→ open::handle_open(args)  (别名打开)
                  │
                  └─ 解析失败 ─────────────→ open::handle_open(args)  (fallback)
```

### 关键设计点

| 设计决策 | 说明 |
|---------|------|
| **clap 解析失败 = 别名查找** | `j chrome` 不是合法子命令，但会被路由到 `open::handle_open`，在配置中查找 `chrome` 别名 |
| **配置即数据库** | 所有别名和设置存储在 `~/.jdata/config.yaml`，通过 `YamlConfig` 结构体读写 |
| **`config` 贯穿全局** | `&mut YamlConfig` 作为参数传递给所有命令 handler |
| **资源编译时嵌入** | `help.md`/`version.md` 等通过 `rust-embed` 嵌入二进制，零运行时 IO |
| **patched tui-textarea** | 本地 patch 在 `patches/` 目录，通过 `Cargo.toml` 的 `[patch.crates-io]` 引用 |

---

## 5. 常用开发命令

```bash
# ============ 构建 & 运行 ============
cargo build                    # Debug 构建
cargo build --release          # Release 构建
cargo run -- <args>            # 运行并传参
make install                   # Release 构建 + 安装到 /usr/local/bin/j

# ============ 代码质量 ============
cargo fmt                      # 格式化代码
cargo clippy                   # Lint 检查
cargo test                     # 运行测试
cargo check                    # 快速检查（不生成二进制）
make pre-commit                # fmt + clippy + test 一键检查

# ============ 功能特性 ============
cargo build --features browser_cdp   # 启用 CDP 浏览器自动化（需本地 Chrome）

# ============ 其他 ============
make help                      # 查看所有 Makefile 命令
make deps                      # 查看依赖树
make doc                       # 生成 Rust 文档
cargo watch -x run             # 文件变化时自动重新运行（需 cargo-watch）
```

---

## 6. 添加新命令（实战教程）

以添加一个 `hello` 命令为例，完整走一遍流程：

### Step 1: 定义子命令 — `src/cli.rs`

在 `SubCmd` 枚举中添加：

```rust
#[derive(Subcommand, Debug)]
pub enum SubCmd {
    // ... 现有命令 ...

    /// 打招呼（示例命令）
    #[command(alias = "hi")]
    Hello {
        /// 你的名字
        name: Option<String>,
    },
}
```

### Step 2: 实现 handler — `src/command/hello.rs`

```rust
use crate::config::YamlConfig;
use crate::util::log::info;

pub fn handle_hello(name: Option<String>, config: &YamlConfig) {
    let name = name.unwrap_or_else(|| "World".to_string());
    info!("Hello, {}! 👋", name);
    debug_log!(config, "hello command executed with name: {}", name);
}
```

### Step 3: 注册模块 — `src/command/mod.rs`

```rust
pub mod hello;  // 添加模块声明
```

### Step 4: 添加分发路由 — `src/command/handler.rs`

```rust
pub fn dispatch(cmd: SubCmd, config: &mut YamlConfig) {
    match cmd {
        // ... 现有分支 ...
        SubCmd::Hello { name } => {
            hello::handle_hello(name, config);
        }
    }
}
```

### Step 5:（可选）添加 Tab 补全 — `src/interactive/completer.rs`

如果命令有特殊的补全需求，在 completer 中添加相应逻辑。

### Step 6: 更新文档

- `assets/help.md` — 添加命令说明（编译时嵌入，`j help` 会显示）
- `README.md` — 如果是用户可见的功能，更新命令速查表

### 验证：

```bash
cargo run -- hello
# 输出: Hello, World! 👋

cargo run -- hello Jack
# 输出: Hello, Jack! 👋

cargo run -- hi Jack
# 输出: Hello, Jack! 👋  （别名也能用）
```

---

## 7. 添加新 AI Tool（扩展教程）

如果需要在 `j chat` 中让 AI 调用新工具：

### Step 1: 创建 Tool 文件 — `src/command/chat/tools/my_tool.rs`

```rust
use super::{Tool, ToolResult};
use serde_json::{json, Value};
use std::sync::{Arc, atomic::AtomicBool};

pub struct MyTool;

impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }

    fn description(&self) -> &str {
        "工具描述（LLM 根据此描述决定是否调用）"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "param1": { "type": "string", "description": "参数说明" }
            },
            "required": ["param1"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        let args: Value = serde_json::from_str(arguments).unwrap();
        let param1 = args["param1"].as_str().unwrap_or_default();
        ToolResult {
            output: format!("执行结果: {}", param1),
            is_error: false,
        }
    }

    fn requires_confirmation(&self) -> bool { false }
}
```

### Step 2: 注册 — `src/command/chat/tools/mod.rs`

```rust
mod my_tool;

// 在 ToolRegistry::new() 的 tools vec 中添加：
Box::new(my_tool::MyTool),
```

### Step 3: 更新 `assets/help.md` 中的工具表格

---

## 8. 调试技巧

### 开启详细日志

```bash
j log mode verbose             # 开启 verbose 模式
# 之后运行任何命令都会输出 debug_log! 的内容
j log mode concise             # 恢复简洁模式
```

### 在代码中添加调试输出

```rust
use crate::util::log::{info, error, debug_log};

info!("普通信息");              // 始终输出
error!("错误信息");             // 始终输出（红色）
debug_log!(config, "调试: {}", value);  // 仅 verbose 模式下输出
```

### 查看/编辑配置

```bash
cat ~/.jdata/config.yaml       # 查看当前配置
```

### 常见问题排查

| 问题 | 排查方向 |
|------|---------|
| 构建失败找不到依赖 | 检查 `.cargo/config.toml` 中的 registry 镜像是否可达 |
| 命令不生效 | 确认 `handler.rs` 中的 dispatch 分支已添加 |
| Tab 补全不工作 | 检查 `completer.rs` 中是否注册了新命令 |
| TUI 显示异常 | 确认终端支持 256 色，尝试不同终端（iTerm2 / Alacritty） |
| `j <alias>` 无反应 | `j log mode verbose` 后重试，查看 `open.rs` 的日志输出 |

---

## 9. 数据目录说明

所有用户数据存储在 `~/.jdata/`（可通过 `J_DATA_PATH` 环境变量覆盖）：

```
~/.jdata/
├── config.yaml              # 主配置（别名、分类、设置）
├── history.txt              # 交互模式命令历史
├── agent/
│   ├── data/
│   │   ├── agent_config.json    # AI 模型配置（API Key 等）
│   │   ├── chat_session.json    # 当前对话
│   │   ├── archives/            # 归档对话
│   │   ├── system_prompt.md     # 系统提示词
│   │   ├── memory.md            # AI 记忆
│   │   └── soul.md              # AI 人格设定
│   ├── logs/                    # AI 日志
│   └── skills/                  # 技能目录
├── report/
│   ├── week_report.md           # 周报文件
│   ├── settings.json            # 日报配置
│   └── todo.json                # 待办数据
└── scripts/                     # 用户创建的脚本
```

> **开发提示**：开发时可以设置 `J_DATA_PATH=/tmp/jdata-dev` 来使用独立的测试数据目录，避免影响日常使用的数据。

---

## 10. 提交与发布

### 日常开发提交

```bash
make fmt                       # 先格式化
cargo test                     # 跑测试
git add <files>
git commit -m "feat: 添加 xxx 功能"
git push
```

### 发布新版本

```bash
# 方式 1: 发布到 crates.io + GitHub Release
make publish                   # 自动递增版本号 → 构建 → commit → tag → push → cargo publish

# 方式 2: 仅触发 GitHub Release（CI 自动构建二进制）
make bump-version              # 递增版本号
git add . && git commit -m "chore: bump version"
make tag                       # 创建 tag 并推送（触发 GitHub Actions）

# 发布前检查
make publish-check             # dry-run，不实际发布
```

---

## 11. 核心技术栈速查

| 技术 | 用途 | 关键文件 |
|------|------|---------|
| **clap** (derive) | 命令行解析 | `src/cli.rs` |
| **serde** + serde_yaml | 配置序列化 | `src/config/yaml_config.rs` |
| **ratatui** + crossterm | TUI 框架 | `src/tui/`, `src/command/todo/`, `src/command/chat/` |
| **rustyline** | REPL 交互 | `src/interactive/` |
| **async-openai** + tokio | AI 对话（流式） | `src/command/chat/api.rs` |
| **rust-embed** | 资源嵌入 | `src/assets.rs` |
| **pulldown-cmark** | Markdown 解析 | `src/command/chat/markdown/` |
| **reqwest** | HTTP 请求 | 多处使用 |

---

## 快速参考卡片

```
┌─────────────────────────────────────────────────────┐
│  j-cli 开发速查                                       │
├─────────────────────────────────────────────────────┤
│  构建:  cargo build / make install                   │
│  运行:  cargo run -- <cmd>                           │
│  测试:  cargo test                                   │
│  检查:  make pre-commit (fmt + lint + test)          │
│                                                     │
│  新命令: cli.rs → handler.rs → command/xxx.rs        │
│  新工具: chat/tools/xxx.rs → tools/mod.rs            │
│  文档:   assets/help.md + README.md                  │
│                                                     │
│  数据:  ~/.jdata/     配置: config.yaml              │
│  日志:  j log mode verbose                           │
│  测试数据: J_DATA_PATH=/tmp/jdata-dev cargo run      │
└─────────────────────────────────────────────────────┘
```

---

> 更多详细文档参见：[架构设计](./architecture.md) | [命令详解](./commands.md) | [AI 对话](./chat.md) | [设计决策](./design-decisions.md) | [开发指南](./development.md)
