---
name: jcli-dev-guide
description: j-cli (j) 项目开发者入门指南。当开发者需要了解 jcli 项目结构、各模块职责、开发流程，或者需要完成常见开发任务（添加新命令、新工具、修改 Chat 模块、调整 Hook/Permission 系统等）时使用此 skill。适用场景：新成员上手、代码导航、功能开发 checklist 查询。
---

# jcli 开发者入门指南

## 项目概览

**j-cli** (`j`) 是用 Rust 编写的 CLI 生产力工具，二进制名为 `j`。
- Rust 2024 edition，最低 rustc 1.93.1
- 发布于 crates.io，包名 `j-cli`
- 所有 UI 文字使用中文

## 入口流程

```
main.rs
  ├─ 无参数 → interactive::run_interactive()  (REPL 模式)
  └─ 有参数 → cli::Cli (clap 解析)
               ├─ 解析成功 → SubCmd::into_handler().execute()
               └─ 解析失败 → command::open::handle_open()  (别名打开逻辑)
```

## 核心模块速查

| 模块 | 路径 | 职责 |
|------|------|------|
| CLI 定义 | `src/cli.rs` | clap 命令/子命令定义，`SubCmd` 枚举 |
| 命令分发 | `src/command/handler.rs` | `CommandHandler` trait + `command_handlers!` 宏 |
| 常量 | `src/constants.rs` | 路径、版本、数据目录等全局常量 |
| 配置 | `src/config/yaml_config.rs` | `YamlConfig`，对应 `~/.jdata/config.yaml` |
| 别名管理 | `src/command/alias.rs` | set/remove/rename/modify 别名 |
| 分类标记 | `src/command/category.rs` | note/denote 分类 |
| 列表 | `src/command/list.rs` | `ls` 命令 |
| 别名打开 | `src/command/open.rs` | 打开 app/URL/编辑器/浏览器 |
| 日报系统 | `src/command/report.rs` | report/reportctl/check/search |
| 脚本 | `src/command/script.rs` | `j concat` 脚本创建执行 |
| 系统命令 | `src/command/system.rs` | contain/change/clear/help 等 |
| 计时器 | `src/command/time.rs` | countdown 倒计时 |
| 自更新 | `src/command/update.rs` | self-update |
| 待办 TUI | `src/command/todo/` | ratatui 待办界面 |
| AI Chat | `src/command/chat/` | 最大子系统，见下方详述 |
| REPL | `src/interactive/` | rustyline + tab 补全 + shell 模式 |
| TUI 编辑器 | `src/tui/` | 共享的 TUI 文本编辑器 widget |
| 工具宏 | `src/util/` | 日志宏、Markdown 渲染、模糊搜索、HTML 提取 |

## Chat 模块详解 (`src/command/chat/`)

这是最大的子系统，详细说明见 [references/chat-module.md](references/chat-module.md)。

| 文件/目录 | 职责 |
|-----------|------|
| `agent.rs` | Agent 循环：发消息→LLM→处理工具调用→循环 |
| `api.rs` | OpenAI 兼容 API 客户端（streaming via async-openai）|
| `tools/` | 20+ 内置工具，每个工具独立文件 |
| `compact.rs` | 3 层 context 压缩（micro→auto→Compact tool）|
| `permission.rs` | `.jcli/` 文件权限系统（allow/deny 规则）|
| `hook.rs` | 3 层 Hook 系统（user→project→session）|
| `skill.rs` | 从 `~/.jdata/agent/skills/` 加载 skill |
| `storage.rs` | 聊天历史持久化、agent config 加载 |
| `handler/` | TUI 事件循环、聊天逻辑、配置 UI、工具确认弹窗 |
| `ui/` | UI 渲染（聊天视图、配置视图、归档列表）|
| `app/` | App 状态机（action/types/ui_state/agent_handle）|
| `archive.rs` | 对话归档/恢复 |
| `theme.rs` | 颜色主题（dark/light/dracula/gruvbox/monokai/nord）|
| `render_cache.rs` | 渲染消息缓存 |
| `markdown/` | Markdown 解析与语法高亮 |
| `autocomplete.rs` | `@skill` 自动补全 |

## 关键模式

### CommandHandler 模式（添加新命令的核心）
```rust
// 1. 在 src/cli.rs SubCmd 枚举添加变体
// 2. 在 src/command/handler.rs 用宏注册：
command_handlers! {
    MyCmd { param: String } => |self, config| {
        crate::command::my_module::handle_my_cmd(&self.param, config);
    },
}
// 3. 在 SubCmd::into_handler() 中添加 match 分支
// 4. 在 src/command/mod.rs dispatch() 中添加路由（如有需要）
```

### Tool trait（添加新 AI 工具的核心）
```rust
impl Tool for MyTool {
    fn name(&self) -> &str { "MyTool" }
    fn description(&self) -> &str { "..." }
    fn parameters_schema(&self) -> Value { schema_to_tool_params::<MyArgs>() }
    fn execute(&self, arguments: &str, cancelled: &Arc<AtomicBool>) -> ToolResult { ... }
    fn requires_confirmation(&self) -> bool { true } // 需要用户确认时
}
// 在 ToolRegistry::new() 中注册
```

### 日志宏
```rust
info!("普通信息");           // 总是打印
error!("错误信息");          // 总是打印（红色）
usage!("用法提示");          // 总是打印
debug_log!(config, "调试"); // 仅 verbose 模式打印
// Agent 文件日志（写入 ~/.jdata/agent/logs/）：
write_info_log("msg");
write_error_log("msg");
```

## 数据目录结构

```
~/.jdata/
├── config.yaml              # 主配置（别名、日报路径等）
└── agent/
    ├── data/agent_config.json  # AI 配置（模型、API key、provider 等）
    ├── logs/                   # Agent 运行日志
    ├── sessions/               # 聊天历史
    └── skills/                 # 用户 skill 文件
```

## 开发 Checklist

常见开发任务的快速 checklist，见 [references/dev-checklist.md](references/dev-checklist.md)。
