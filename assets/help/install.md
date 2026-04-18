---
name: 安装
order: 9
---

## 安装 & 更新

### 一键安装（推荐）
```bash
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh
```

### 从 crates.io 安装
```bash
cargo install j-cli
# CDP 版本：cargo install j-cli --features browser_cdp
```

### 更新
```bash
j update               # 自动检测安装来源并更新
j update --check       # 仅检查是否有新版本
```

## 卸载

```bash
# 使用安装脚本卸载（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# 或通过 cargo 卸载
cargo uninstall j-cli

# （可选）删除数据目录
rm -rf ~/.jdata
```

> 卸载命令只会删除二进制文件，用户数据（`~/.jdata/`）会保留。

## 使用技巧

- 不带参数运行 `j` 进入**交互模式**，支持 Tab 补全和历史建议
- 交互模式下按 `Ctrl+Q` 快速退出（等同于 `exit` 命令或 `Ctrl+D`）
- 交互模式下用 `!` 前缀执行 shell 命令（如 `!ls -la`），自动注入别名环境变量
- 交互模式下输入 `!`（不带命令）进入交互式 shell 模式（提示符变为绿色 `shell >`），cd 等状态延续，输入 `exit` 或按 `Ctrl+D` 返回 copilot
- 路径含空格时用引号包裹：`j set app "/Applications/My App.app"`
- URL 会自动识别并归类到 `inner_url`，无需手动指定 section
- CLI 工具（如 rg、fzf）注册后可直接在终端执行并支持管道
- 脚本需要后台运行时，使用 `-w` 标志在新窗口中执行（如 `j deploy -w`）
- 启用 shell Tab 补全：`eval "$(j completion zsh)"` 加入 `.zshrc`
- AI 对话中输入 `/` 唤起斜杠命令面板，快速执行常用操作
- AI 对话中输入 `@` 唤起补全弹窗，引用技能、命令或文件
- 使用 `j md` 管理笔记，支持子目录、Markdown 编辑和实时预览
