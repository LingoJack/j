# 开发指南

> 本文档面向 `j` 的开发者，包含编译、开发流程和扩展指南。

---

## 编译运行

### Debug 编译

```bash
cargo build
```

### Release 编译

```bash
cargo build --release
# 二进制在 target/release/j，~17MB（内嵌 ask 渲染引擎）
```

### 运行

```bash
cargo run             # 进入交互模式
cargo run -- help     # 快捷模式执行 help
cargo run -- set chrome /Applications/Google\ Chrome.app
```

---

## 技术栈

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }   # 命令行参数解析
rustyline = "17.0.2"                               # 交互模式 REPL
serde = { version = "1", features = ["derive"] }   # 序列化框架
serde_yaml = "0.9"                                 # YAML 配置读写
serde_json = "1"                                   # JSON 处理
chrono = "0.4"                                     # 日期时间
colored = "3"                                      # 终端彩色输出
dirs = "6"                                         # 跨平台用户目录
ratatui = "0.29.0"                                 # TUI 框架
crossterm = "0.28.0"                               # 终端原始模式
tui-textarea = "0.7"                               # 多行文本编辑
async-openai = "0.33"                              # OpenAI API 客户端
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
pulldown-cmark = "0.13.0"                          # Markdown 解析
whisper-rs = "0.15"                                # Whisper.cpp Rust 绑定
cpal = "0.17"                                      # 跨平台音频捕获
```

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
- [ ] `j voice` 语音转文字
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
