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
```

### 查看 Agent 配置

```bash
cat ~/.jdata/agent/data/agent_config.json
```

---

## 测试

### 运行测试

```bash
cargo test
```

### 手动测试清单

- [ ] `j set/rm/rename/mf` 别名 CRUD
- [ ] `j <alias>` 打开应用/URL
- [ ] `j report/check/search` 日报系统
- [ ] `j todo` 待办管理
- [ ] `j chat` AI 对话
- [ ] 交互模式 Tab 补全
- [ ] 交互模式 `!` shell 命令

---

## 发布流程

### 1. 更新版本号

```bash
# 编辑 constants.rs 中的 VERSION
# 编辑 Cargo.toml 中的 version
```

### 2. 编译 Release

```bash
cargo build --release
```

### 3. 测试

```bash
./target/release/j version
./target/release/j help
```

### 4. 发布到 crates.io

```bash
cargo publish
```

### 5. 创建 GitHub Release

```bash
git tag v1.0.0
git push origin v1.0.0
# 在 GitHub 上创建 Release，上传二进制
```

---

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
