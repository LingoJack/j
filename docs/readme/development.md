# 开发指南

> 本文档面向 `j` 的开发者，包含编译、开发流程和扩展指南。

---

## 核心依赖

| 依赖 | 用途 |
|------|------|
| `clap` | 命令行参数解析 |
| `async-openai` | OpenAI API 客户端 |
| `ratatui` + `crossterm` | TUI 界面 |
| `serde` + `serde_json` + `serde_yaml` | 序列化 |
| `tokio` | 异步运行时 |
| `pulldown-cmark` | Markdown 解析 |
| `reqwest` | HTTP 客户端（blocking 模式） |
| `scraper` + `html2md` | HTML 解析与 Markdown 转换 |
| `urlencoding` | URL 编码 |
---

## 添加新命令

### 1. 在 `cli.rs` 添加子命令

```rust
#[derive(Subcommand)]
pub enum SubCmd {
    // 现有命令...
    
    /// 新命令说明
    #[command(alias = "nc")]
    NewCommand {
        /// 参数说明
        #[arg(required = true)]
        param: String,
    },
}
```

### 2. 在 `command/` 下创建 handler

```rust
// src/command/new_command.rs
use crate::config::YamlConfig;

pub fn handle_new_command(param: &str) {
    let config = YamlConfig::load();
    // 实现逻辑...
}
```

### 3. 在 `handler.rs` 添加分发

```rust
pub fn dispatch(cmd: SubCmd, config: &mut YamlConfig) {
    match cmd {
        // 现有分支...
        SubCmd::NewCommand { param } => {
            new_command::handle_new_command(&param);
        }
    }
}
```

### 4. 添加补全规则（可选）

在 `interactive/completer.rs` 中添加 Tab 补全逻辑。

### 5. 更新帮助文本

在 `assets/help.md` 中添加命令说明。

---

## 添加新 Tool

AI 对话模式下，LLM 可以调用 Tool 执行操作（如读取文件、执行命令、网络请求等）。

### 1. 创建 Tool 文件

在 `src/command/chat/tools/` 下创建新文件，实现 `Tool` trait：

```rust
// src/command/chat/tools/my_tool.rs
use super::{Tool, ToolResult};
use serde_json::{json, Value};
use std::sync::{Arc, atomic::AtomicBool};

pub struct MyTool;

impl Tool for MyTool {
    fn name(&self) -> &str {
        "my_tool"
    }

    fn description(&self) -> &str {
        "工具描述，LLM 会根据此描述决定是否调用"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "param1": {
                    "type": "string",
                    "description": "参数说明"
                }
            },
            "required": ["param1"]
        })
    }

    fn execute(&self, arguments: &str, _cancelled: &Arc<AtomicBool>) -> ToolResult {
        // 解析参数并执行
        let args: Value = match serde_json::from_str(arguments) {
            Ok(v) => v,
            Err(e) => return ToolResult {
                output: format!("参数解析失败: {}", e),
                is_error: true,
            }
        };
        
        // 执行逻辑...
        ToolResult {
            output: "执行结果".to_string(),
            is_error: false,
        }
    }

    /// 需要用户确认的工具（如 shell 命令）返回 true
    fn requires_confirmation(&self) -> bool {
        false
    }
}
```

### 2. 注册 Tool

在 `src/command/chat/tools/mod.rs` 中注册：

```rust
mod my_tool;  // 添加模块声明

impl ToolRegistry {
    pub fn new(skills: Vec<crate::command::chat::skill::Skill>) -> Self {
        let mut registry = Self {
            tools: vec![
                // ... 现有工具
                Box::new(my_tool::MyTool),  // 添加新工具
            ],
        };
        // ...
    }
}
```

### 3. 更新文档

在 `assets/help.md` 的内置工具表格中添加说明：

```markdown
| `my_tool` | 功能说明 | 是否需要确认 |
```

---

## 调试技巧

### 启用详细日志

```bash
j log mode verbose
```

然后在代码中使用：

```rust
use crate::util::log::debug_log;
debug_log!(config, "调试信息: {}", value);
```

### 查看配置文件

```bash
cat ~/.jdata/config.yaml


## 文档更新

修改相关文档后，确保：

1. `README.md` 保持精简
2. `docs/*.md` 详细说明
3. `assets/help.md` 用户帮助

---

## 代码风格

- 使用 `rustfmt` 格式化：`cargo fmt`
- 使用 `clippy` 检查：`cargo clippy`
- 遵循 Rust 命名规范
- 新增常量添加到 `constants.rs`
- 使用 `info!`/`error!`/`usage!` 宏输出
