<div align="center"></div>

# jcli

**AI 驱动的命令行工作台**

别名打开 · Agent 工作台 · 日报周报 · 待办备忘 · Markdown 预览 · 脚本工作流

[![Rust](https://img.shields.io/badge/Rust-1.93%2B-orange?logo=rust)](https://www.rust-lang.org/) [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT) [![Version](https://img.shields.io/badge/version-12.10.7-green.svg)](https://github.com/LingoJack/jcli)

[在线文档](https://lingojack.github.io/j/) · [快速开始](#-快速开始) · [功能一览](#-功能一览) · [安装](#-安装) · [GUI 版本](https://github.com/LingoJack/j-gui.git)

如果你更偏好图形界面，可以试试 [j-gui](https://github.com/LingoJack/j-gui) —— 基于 Tauri 的 GUI 版本（研发中），提供可视化操作体验。当前 j-cli 的 TUI 界面也支持部分鼠标操作（点击、滚动等），在终端中即可获得接近 GUI 的交互体验。

---

### 

## Agent 工作台

内置终端 Agent，支持多模型、流式输出、多步推理、工具调用、权限确认，适合直接在仓库里分析、修改、执行任务

## 别名打开

`j set chrome "/Applications/Google Chrome.app"` 一次注册后，`j chrome`、`j chrome github`、`j vscode ./src`、`j deploy` 都能直接作为统一入口使用

### 日报周报

`j report "完成功能开发"` 快速记录，支持整篇 Markdown 编辑、自动周数管理，以及 `reportctl push/pull` 做 Git 同步

### 待办备忘

`j todo` 进入全屏 TUI 管理，支持 Markdown checkbox、筛选排序、长内容预览，完成事项时还能顺手写入日报

### Markdown 终端预览

`j md`、`j reportctl open` 等入口可直接进入终端 Markdown 编辑器，边写边看渲染效果，适合日报、笔记和 AGENTS.md

<summary><strong>更多功能</strong></summary>

<summary><strong>更多功能</strong></summary>

- **Skill 技能系统** — 可扩展的 AI 技能，按需加载，支持 `@skill` 触发
- **Hook 系统** — 三级 Hook（用户/项目/Session），灵活扩展 AI 行为
- **浏览器自动化** — Lite 模式（零依赖）和 CDP 模式（完整浏览器控制），AI 可直接操作网页
- **移动端远程控制** — 扫码连接手机，远程操作 AI 对话
- **交互模式** — `j` 回车即进入 REPL，Tab 补全 + 历史建议 + Shell 穿透

---

## 截图

<br>

<br>

<p><em>图 6：别名列表视图。统一管理应用、脚本和路径入口，配合 `j &lt;alias&gt;` 形成很轻的个人命令工作流。</em></p>

### 

### 安装

**macOS / Linux：**

```bash
curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh

```

**Windows (PowerShell)：**

```powershell
irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex

```

**从 crates.io 安装：**

```bash
cargo install j-cli

```

<p><em>图 4：Agent 多轮工作流。回复内容、代码片段、Grep/Read/Bash 结果都在同一终端会话里连续展开，适合代码审查和问题排查。</em></p>

<p><br></p>

**从源码编译：**

```bash
git clone https://github.com/LingoJack/jcli.git
cd j && cargo install --path .

```

**完整版（CDP 浏览器自动化，需本地安装 Chrome）：**

```bash
cargo install j-cli --features browser_cdp

```

### 上手使用

```bash
# 注册别名
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"
j set github https://github.com

# 一键打开
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github
j chrome "rust lang"      # 用 Chrome 搜索
j vscode ./src            # 用 VSCode 打开目录

# 日报
j report "完成功能开发"    # 写入日报
j check                   # 查看最近 10 行

# 待办
j todo add 买牛奶         # 快速添加
j todo                    # TUI 管理

# AI 对话
j chat                    # 进入 TUI 对话
j chat "你好"             # 快速提问

# 交互模式（Tab 补全 + 历史建议）
j

```

---

### 安装

| 命令 | 说明 |
| --- | --- |
| `j <alias>` | 打开应用/文件/URL |
| `j set <alias> <path>` | 注册别名 |
| `j rm <alias>` | 删除别名 |
| `j ls` | 列出别名 |
| `j report <content>` | 写入日报 |
| `j check [N]` | 查看日报 |
| `j todo` | 待办管理 (TUI) |
| `j chat` | AI 对话 (TUI) |
| `j script <name> "<cmd>"` | 创建脚本 |
| `j` | 进入交互模式 |

> 完整命令文档请访问 [在线文档](https://lingojack.github.io/j/)
> 

---

## 技术栈

| 技术 | 用途 |
| --- | --- |
| Rust[](https://www.rust-lang.org/) | 核心语言 |
| clap[](https://github.com/clap-rs/clap) | 命令行参数解析 |
| ratatui[](https://github.com/ratatui/ratatui) | TUI 框架 |
| rustyline[](https://github.com/kkawakam/rustyline) | REPL 交互 |
| async-openai[](https://github.com/64bit/async-openai) | OpenAI API 客户端 |
| serde[](https://github.com/serde-rs/serde) | 序列化框架 |

---

## GUI 版本

如果你更偏好图形界面操作，[j-gui](https://github.com/LingoJack/jcli-gui) 提供了基于 Tauri 的桌面客户端（**目前正在研发中**），支持别名管理、日报周报、AI 对话等核心功能，适合不想在终端中操作的场景。

<summary>其他安装方式</summary>

---

## License
