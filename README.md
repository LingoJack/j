<div align="center">

# j

**AI 驱动的命令行工作台**

别名管理 · 日报周报 · 待办备忘 · AI 对话 · 脚本管理 · 浏览器自动化

[![Rust](https://img.shields.io/badge/Rust-1.93%2B-orange?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)
[![Version](https://img.shields.io/badge/version-12.10.7-green.svg)](https://github.com/LingoJack/j)

[在线文档](https://lingojack.github.io/j/) · [快速开始](#-快速开始) · [功能一览](#-功能一览) · [安装](#-安装)

</div>

---

## ✨ 功能一览

<table>
<tr>
<td width="50%">

### 🤖 AI 对话
内置 TUI 对话界面，多模型支持、流式输出、Markdown 渲染、工具调用、Agent 自主推理

</td>
<td width="50%">

### 🏷️ 别名管理
`j set chrome "/Applications/Google Chrome.app"` 一键注册，`j chrome` 秒开应用、URL、脚本

</td>
</tr>
<tr>
<td>

### 📝 日报周报
`j report "完成功能开发"` 快速记录，自动周数管理，支持 Git 同步与团队周报

</td>
<td>

### ✅ 待办备忘
`j todo` 进入全屏 TUI 管理，支持 Markdown checkbox，完成时可联动写入日报

</td>
</tr>
<tr>
<td>

### 🔧 脚本系统
`j script deploy "npm run build"` 创建脚本，别名环境变量自动注入，支持新窗口执行

</td>
<td>

### 🌐 浏览器自动化
Lite 模式（零依赖）和 CDP 模式（完整浏览器控制），AI 可直接操作网页

</td>
</tr>
</table>

<details>
<summary><strong>更多功能</strong></summary>

- **Skill 技能系统** — 可扩展的 AI 技能，按需加载，支持 `@skill` 触发
- **Hook 系统** — 三级 Hook（用户/项目/Session），灵活扩展 AI 行为
- **移动端远程控制** — 扫码连接手机，远程操作 AI 对话
- **交互模式** — `j` 回车即进入 REPL，Tab 补全 + 历史建议 + Shell 穿透

</details>

---

## 📸 截图

<div align="center">
<img src="https://raw.githubusercontent.com/LingoJack/j/main/web/public/pics/jcli-ai/1.png" width="80%" alt="AI Chat Interface" />
<p><em>AI 对话界面 — 流式输出、Markdown 渲染、工具调用</em></p>
</div>

<div align="center">
<img src="https://raw.githubusercontent.com/LingoJack/j/main/web/public/pics/jcli-ai/2.png" width="80%" alt="AI Chat Interface 2" />
</div>

<div align="center">
<img src="https://raw.githubusercontent.com/LingoJack/j/main/web/public/pics/jcli-ai/3.png" width="80%" alt="AI Chat Interface 3" />
</div>

---

## 🚀 快速开始

### 安装

**macOS / Linux：**

```bash
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh
```

**Windows (PowerShell)：**

```powershell
irm https://raw.githubusercontent.com/LingoJack/j/main/install.ps1 | iex
```

**从 crates.io 安装：**

```bash
cargo install j-cli
```

<details>
<summary>其他安装方式</summary>

**从源码编译：**

```bash
git clone https://github.com/LingoJack/j.git
cd j && cargo install --path .
```

**完整版（CDP 浏览器自动化，需本地安装 Chrome）：**

```bash
cargo install j-cli --features browser_cdp
```

</details>

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

## 📖 核心命令速览

| 命令 | 说明 |
|------|------|
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

---

## 🛠️ 技术栈

| 技术 | 用途 |
|------|------|
| [Rust](https://www.rust-lang.org/) | 核心语言 |
| [clap](https://github.com/clap-rs/clap) | 命令行参数解析 |
| [ratatui](https://github.com/ratatui/ratatui) | TUI 框架 |
| [rustyline](https://github.com/kkawakam/rustyline) | REPL 交互 |
| [async-openai](https://github.com/64bit/async-openai) | OpenAI API 客户端 |
| [serde](https://github.com/serde-rs/serde) | 序列化框架 |

---

## 📄 License

[MIT](https://opensource.org/licenses/MIT)
