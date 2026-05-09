# j-agent 提取重构文档

## 1. 背景

j-cli 是一个 macOS CLI 生产力工具，核心功能之一是 AI 对话（chat）。在重构之前，chat 子系统全部代码（约 80,000 行）都在 `src/` 下，深度耦合 ratatui/crossterm TUI 框架。

**问题**：团队计划开发 GUI 版本（Tauri/SwiftUI），但所有聊天引擎逻辑和终端渲染混在一起，无法复用。

**目标**：将聊天引擎核心逻辑提取为独立 crate `j-agent`，不依赖任何 TUI 框架，让 CLI 和 GUI 共同使用。

## 2. 架构

```
重构前:
┌─────────────────────────────────────────────┐
│                  j-cli                       │
│  ┌─────────────────────────────────────┐    │
│  │ chat 引擎（LLM、Agent、工具、权限）    │    │
│  │ + TUI 渲染（ratatui/crossterm）      │    │
│  │ + 资源嵌入（rust-embed）              │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘

重构后:
┌─────────────────────────────────────────────┐
│  j-agent (纯逻辑层)                         │
│  ├── agent/       Agent 循环、SubAgent、Teammate │
│  ├── llm/         LLM 客户端、SSE 流、类型定义   │
│  ├── tools/       18 个工具实现                │
│  ├── context/     消息窗口、compact、plan       │
│  ├── infra/       Hook 系统、Skill、Sandbox     │
│  ├── permission/  .jcli 规则匹配、权限队列       │
│  ├── storage/     AgentConfig、Session 持久化   │
│  ├── teammate/    Teammate 管理器、循环          │
│  ├── template/    系统提示词模板 (include_str!)  │
│  ├── message_types/ 消息类型定义               │
│  └── util/        通用工具函数                  │
└─────────────────────────────────────────────┘
          ↑ 依赖（无 ratatui）
          │
┌─────────┴───────────────────────────────────┐
│  j-cli (TUI 层)                              │
│  ├── command/chat/handler/  键盘事件处理       │
│  ├── command/chat/render/   渲染缓存          │
│  ├── command/chat/ui/       UI 绘制          │
│  ├── command/chat/input.rs  输入线程          │
│  ├── command/chat/oneshot/  单次对话模式       │
│  ├── command/chat/remote/   远程 WS 模式      │
│  ├── assets/                rust-embed 资源   │
│  ├── markdown/              Markdown 渲染     │
│  └── tui/                   TUI 编辑器组件    │
└─────────────────────────────────────────────┘
```

## 3. 代码统计

| 指标 | j-agent | j-cli |
|------|---------|-------|
| .rs 文件数 | 118 | 190 |
| 代码行数 | ~31,000 | ~49,000 |
| 依赖 ratatui | 否 | 是 |
| 依赖 crossterm | 否 | 是 |

## 4. j-agent 模块清单

```
j-agent/src/
├── lib.rs              # 公开模块入口
├── agent.rs            # AgentLoop 入口
├── agent/              # Agent 循环核心
│   ├── agent_loop.rs   # 主循环（发送消息→接收流→处理工具调用）
│   ├── api.rs          # OpenAI SDK 封装（async-openai）
│   ├── config.rs       # AgentConfig 类型
│   ├── retry.rs        # 重试策略
│   ├── thread_identity.rs  # 线程身份标记
│   └── tool_processor.rs   # 工具调用分发与结果收集
├── agent_md.rs         # AGENTS.md 加载与解析
├── chat_error.rs       # ChatError 枚举
├── constants.rs        # 全局常量
├── context/            # 上下文窗口管理
│   ├── compact.rs      # micro_compact + auto_compact
│   ├── message_compress.rs  # 消息压缩
│   ├── plan_state.rs   # Plan 状态
│   ├── policy.rs       # 上下文策略
│   └── window.rs       # 消息窗口
├── crypto.rs           # 远程协议加密
├── infra/              # 基础设施
│   ├── hook.rs         # 4 级 Hook 系统（14 事件类型）
│   ├── skill.rs        # Skill 加载与执行
│   ├── archive.rs      # 会话归档
│   ├── command.rs      # Shell 命令执行
│   └── sandbox.rs      # 沙箱环境
├── llm/                # LLM 客户端
│   ├── client.rs       # HTTP 客户端 + 流式响应
│   ├── error.rs        # LlmError
│   ├── stream.rs       # SSE 流解析
│   └── types.rs        # ChatRequest/Response/StreamChunk 等
├── message_types.rs    # 消息类型定义
├── permission/         # 权限系统
│   ├── queue.rs        # 权限确认队列
│   └── rules.rs        # .jcli 规则匹配
├── protocol.rs         # 远程 WebSocket 协议
├── storage/            # 持久化
│   ├── config.rs       # AgentConfig 读写
│   ├── persist.rs      # JSON 持久化
│   ├── session.rs      # Session 管理
│   └── types.rs        # 存储类型
├── teammate/           # Teammate 系统
│   ├── manager.rs      # Teammate 生命周期管理
│   └── teammate_loop.rs # Teammate 独立循环
├── template.rs         # 系统提示词模板（include_str!）
├── theme_name.rs       # 主题名称枚举
├── tools/              # 工具实现（18 个）
│   ├── mod.rs          # Tool trait + ToolRegistry
│   ├── definition.rs   # JSON Schema → tool 参数定义
│   ├── shell.rs        # Shell 工具（Bash 执行）
│   ├── file/           # Read/Write/Edit/Glob
│   ├── grep.rs         # 搜索工具（ripgrep 风格）
│   ├── browser/        # 浏览器自动化（CDP + Lite）
│   ├── web_fetch.rs    # Web 抓取
│   ├── web_search.rs   # Web 搜索
│   ├── ask.rs          # 交互式提问
│   ├── background.rs   # 后台任务
│   ├── hook.rs         # Hook 注册工具
│   ├── task/           # 任务管理（Create/List/Get/Update）
│   ├── todo/           # 待办管理
│   ├── sub_agent.rs    # 子 Agent
│   ├── teammate_tool.rs # Teammate 工具
│   ├── computer_use/   # 屏幕操作（macOS）
│   ├── skill.rs        # 技能加载
│   ├── plan.rs         # Plan 工具
│   ├── compact_tool.rs # 上下文压缩触发
│   └── ...             # send_message, work_done, worktree 等
└── util/               # 通用工具
    ├── html_extract.rs # HTML→Markdown 提取
    ├── path_utils.rs   # 路径工具
    ├── shell_safety.rs # Shell 安全检查
    ├── sync.rs         # 文件锁
    ├── log.rs          # 日志宏
    └── text.rs         # 文本处理
```

## 5. j-cli 如何使用 j-agent

j-cli 通过 re-export 方式使用 j-agent 的模块，保持向后兼容：

```rust
// src/command/chat.rs
pub use j_agent::agent;
pub use j_agent::context;
pub use j_agent::infra;
pub use j_agent::permission;
pub use j_agent::storage;
pub use j_agent::teammate;
pub use j_agent::tools;
pub use j_agent::chat_error as error;
pub use j_agent::constants;
```

j-cli 子模块中通过 `crate::command::chat::tools::xxx` 或直接 `j_agent::tools::xxx` 访问。

## 6. 关键设计决策

### 6.1 资源加载方式差异

| | j-agent | j-cli |
|--|---------|-------|
| 方式 | `include_str!` 编译时嵌入 | `rust-embed` (RustEmbed) |
| 优点 | 零依赖，cargo test 友好 | 支持运行时遍历、按需加载 |
| 缺点 | 每个文件需手动声明 | 需要额外依赖 |

j-agent 用 `include_str!` 读取系统提示词等模板文件，因为 j-agent 不需要运行时动态发现资源。

### 6.2 Hook 帮助内容注入

j-agent 的 `RegisterHookTool` 需要显示帮助文档，但 j-agent 没有 `rust-embed`。

**方案**：`OnceLock<String>` + setter 函数：

```rust
// j-agent/src/tools/hook.rs
static HOOK_HELP_CONTENT: OnceLock<String> = OnceLock::new();

pub fn set_hook_help_content(content: String) {
    let _ = HOOK_HELP_CONTENT.set(content);
}
```

j-cli 在 TUI 启动时注入：

```rust
// j-cli src/command/chat/handler/tui_loop.rs
if let Some(asset) = crate::assets::Assets::get("help/hook.md") {
    let content = String::from_utf8_lossy(&asset.data).into_owned();
    j_agent::tools::hook::set_hook_help_content(content);
}
```

### 6.3 browser_cdp Feature Gate

浏览器 CDP 模式依赖 `chromiumoxide`（大依赖），通过 feature gate 控制：

```toml
# j-agent/Cargo.toml
[features]
browser_cdp = ["dep:chromiumoxide"]

# j-cli/Cargo.toml
[features]
browser_cdp = ["dep:chromiumoxide", "j-agent/browser_cdp"]
```

j-cli 的 feature 传递到 j-agent。

### 6.4 删除的 j-cli 本地模块

以下模块在提取后变成了死代码，已从 j-cli 删除：

| 模块 | 行数 | 原因 |
|------|------|------|
| src/llm/ (4 文件) | ~600 | j-agent::llm 已提供 |
| src/util/html_extract.rs | ~200 | j-agent::util 已提供 |
| src/util/path_utils.rs | ~100 | j-agent::util 已提供 |
| src/util/shell_safety.rs | ~800 | j-agent::util 已提供 |
| assets/template.rs 两个函数 | ~20 | j-agent::template 已提供 |

## 7. GUI 如何使用 j-agent

### Cargo.toml 依赖

```toml
# Git 依赖（推荐，GUI 在独立仓库）
j-agent = { git = "https://github.com/LingoJack/jcli.git", branch = "main" }

# 本地 path 依赖（开发调试用）
j-agent = { path = "../j/j-agent" }
```

### 使用示例

```rust
use j_agent::llm::LlmClient;
use j_agent::tools::ToolRegistry;
use j_agent::agent::AgentLoop;
use j_agent::infra::hook::HookManager;

// 1. 创建 LLM 客户端
let client = LlmClient::new(endpoint, api_key, model);

// 2. 注册工具
let mut registry = ToolRegistry::new();
// registry.register(ShellTool::new());
// registry.register(ReadTool::new());
// ... 或加载内置工具集

// 3. 初始化 hook 帮助内容（如果使用 RegisterHookTool）
j_agent::tools::hook::set_hook_help_content(hook_help_text);

// 4. 运行 Agent
let mut agent = AgentLoop::new(client, registry, config);
let response = agent.run(user_message).await?;
```

### GUI 渲染完全自由

j-agent 不依赖任何 UI 框架，GUI 可以自由选择：
- Tauri (Web)
- iced (Rust native)
- SwiftUI (macOS native)
- egui (游戏风格)
- 自定义渲染

## 8. 提交历史

```
95ed824 fix: 移除3个空的 mod tests 声明
fb45097 rename: j-cli-core → j-agent
aa1b992 fix: 清理所有 clippy 警告，0 error 0 warning
dab632e fix: clippy + browser_cdp 编译修复
88b1b09 feat: j-cli 从 j-agent re-export 模块，消除代码重复
162c129 feat: j-agent 编译通过，无 ratatui 依赖
c771b70 feat: 搬运 chat 引擎核心模块到 j-agent
bfef4e9 feat: 添加 j-agent 核心常量
72d924d feat: 创建 j-agent workspace crate
```

## 9. 验证状态

- ✅ `cargo clippy -p j-agent -- -D warnings`: 0 error, 0 warning
- ✅ `cargo clippy -p j-cli -- -D warnings`: 0 error, 0 warning
- ✅ `cargo clippy -p j-agent --features browser_cdp -- -D warnings`: 0 error, 0 warning
- ✅ `cargo test --all`: 358 passed, 0 failed
- ✅ j-agent 无 ratatui/crossterm 依赖

## 10. 待办

- [ ] GUI 项目脚手架（Tauri？）
- [ ] j-agent 公开 API 审查（部分函数可能需要更友好的高层封装）
- [ ] j-agent 发布到 crates.io（可选）
- [ ] CI/CD 跨 crate 测试
