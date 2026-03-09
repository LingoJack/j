# work-copilot (j) — 快捷命令行工具

> 📅 最后更新: 2026-03-08 | 🔖 版本: v12.1.54 | 🖥️ 平台: macOS ARM64

**`j` 是一个快捷命令行工具，核心功能：**

- **别名管理** — 注册 app 路径 / URL / 脚本，`j <alias>` 快速打开
- **日报系统** — 快速写入、查看、搜索日报，自动周数管理
- **待办备忘录** — 内置 TUI 待办管理，支持 markdown checkbox
- **AI 对话** — 内置 TUI AI 对话，多模型、流式输出、工具调用
- **交互模式** — 带 Tab 补全 + 历史建议的 REPL 环境

**重构动机**：启动速度提升 10-100x（JVM ~200-500ms → Rust ~2ms）

---

## 快速上手

```bash
# 注册别名
j set chrome "/Applications/Google Chrome.app"
j set github https://github.com

# 打开
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github URL
j chrome "rust lang"      # 用 Chrome 搜索

# 日报
j report "完成功能开发"    # 写入日报
j check                   # 查看最近 5 行

# 待办
j todo add 买牛奶         # 快速添加待办
j todo                    # 进入 TUI 管理

# AI 对话
j chat                    # 进入 TUI 对话界面
j chat 你好               # 快速提问

# 交互模式
j                         # 进入 REPL（Tab 补全 + 历史建议）
```

---

## 安装

### 一键安装（推荐）

```bash
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh
```

> 零依赖，下载预编译二进制即可使用。

### 从 crates.io 安装

```bash
cargo install j-cli
```

### 从源码编译

```bash
git clone https://github.com/LingoJack/j.git
cd j && cargo install --path .

# 启用完整浏览器自动化（需本地安装 Chrome/Chromium）
cargo install --path . --features browser_cdp
```

> **浏览器自动化 FAQ**
> - 不加 `--features browser_cdp` 时自动使用 Lite 模式（纯 HTTP 模拟，无需 Chrome）
> - CDP 模式需要本地已安装 Chrome/Chromium，程序退出时会自动关闭浏览器进程
> - 从 crates.io 安装：`cargo install j-cli --features browser_cdp`

---

## 核心命令速查

| 类别 | 命令 | 说明 |
|------|------|------|
| **别名** | `j set/rm/rn/mf` | 设置/删除/重命名/修改别名 |
| **分类** | `j note/denote` | 标记/解除分类 |
| **列表** | `j ls` / `j contain` | 列出别名/查找别名 |
| **打开** | `j <alias>` | 打开应用/URL/文件 |
| **日报** | `j report/check/search` | 写入/查看/搜索日报 |
| **待办** | `j todo` | 待办备忘录 |
| **脚本** | `j concat` | 创建脚本 |
| **AI** | `j chat` | AI 对话 |
| **更新** | `j update` | 自更新（仅 GitHub Release 安装） |

---

## 数据目录

```
~/.jdata/
├── config.yaml          # 主配置文件
├── agent/               # AI Agent 数据
│   ├── data/            # 配置、对话历史、归档
│   └── skills/          # 技能目录
├── bin/                 # 内置工具（Markdown 渲染器）
├── report/              # 日报目录
├── scripts/             # 用户脚本
└── todo/                # 待办数据
```

---

## 详细文档

| 文档 | 内容 |
|------|------|
| [架构设计](./docs/readme/architecture.md) | 目录结构、模块说明、数据模型 |
| [命令详解](./docs/readme/commands.md) | 所有命令详细使用说明 |
| [AI 对话](./docs/readme/chat.md) | AI 对话系统完整文档 |
| [设计决策](./docs/readme/design-decisions.md) | 关键设计决策记录 |
| [开发指南](./docs/readme/development.md) | 编译、开发、扩展指南 |

---

## 技术栈

- **clap** — 命令行参数解析
- **rustyline** — 交互模式 REPL
- **ratatui** — TUI 框架
- **async-openai** — OpenAI API 客户端
- **serde** — 序列化框架

---

## 更新 & 卸载

```bash
# 更新
cargo install j-cli

# 卸载
cargo uninstall j-cli
rm -rf ~/.jdata  # （可选）删除数据目录
```

---

## License

MIT
