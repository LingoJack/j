# work-copilot (j) — 快捷命令行工具 🚀

> 一条命令打开一切，高效管理日常工作流

---

## 🚀 快速上手

```bash
# 注册应用别名
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# 注册 URL 别名（自动识别为 inner_url）
j set github https://github.com

# 标记分类（标记后支持组合打开）
j note chrome browser
j note vscode editor

# 一键打开
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github 对应的 URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录

# 写日报 & 查看
j report "完成功能开发"    # 写入今日日报
j check                   # 查看最近 5 行
j check 20                # 查看最近 20 行

# 进入交互模式（带 Tab 补全 + 历史建议）
j
```

---

## 📁 数据目录

所有数据统一存储在 `~/.jdata/` 下（可通过环境变量 `J_DATA_PATH` 自定义）：

```
~/.jdata/
├── config.yaml          # 主配置文件（别名、分类、设置等）
├── history.txt          # 交互模式命令历史
├── scripts/             # j concat 创建的脚本
├── todo/                # 待办备忘录目录
│   └── todo.json        # 待办数据（JSON 格式）
└── report/              # 日报目录
    ├── week_report.md   # 周报文件
    ├── settings.json    # 日报配置（周数、日期）
    └── .git/            # git 仓库（配置远程仓库后生成）
```

### 配置文件结构 (`config.yaml`)

| Section | 说明 | 示例 |
|---------|------|------|
| `path` | 本地应用/文件路径 | `chrome: /Applications/Google Chrome.app` |
| `inner_url` | URL 链接 | `github: https://github.com` |
| `outer_url` | 需 VPN 的外网 URL | `docs: https://internal.example.com` |
| `browser` | 浏览器列表（值引用 path 中的 key） | `chrome: chrome` |
| `editor` | 编辑器列表（同上） | `vscode: vscode` |
| `vpn` | VPN 应用 | |
| `script` | 已注册的脚本 | `deploy: ~/.jdata/scripts/deploy.sh` |
| `report` | 日报系统配置 | `git_repo: https://github.com/xxx/report` |
| `setting` | 全局设置 | `search-engine: bing` |
| `log` | 日志设置 | `mode: concise` |

---

## 📦 别名管理

| 命令 | 说明 |
|------|------|
| `j set <alias> <path>` | 设置别名（路径自动归类到 path，URL 归类到 inner_url） |
| `j rm <alias>` | 删除别名（同时清理关联的分类标记） |
| `j rename <alias> <new>` | 重命名别名（同步更新所有分类引用） |
| `j mf <alias> <new_path>` | 修改别名指向的路径 |

## 🏷️ 分类标记

| 命令 | 说明 |
|------|------|
| `j note <alias> <category>` | 标记别名分类 |
| `j denote <alias> <category>` | 解除别名分类 |

可用分类: `browser`, `editor`, `vpn`, `outer_url`, `script`

> 标记为 browser 后可以用 `j <browser> <url>` 打开链接或搜索
> 标记为 editor 后可以用 `j <editor> <file>` 打开文件

## 📋 列表 & 查找

| 命令 | 说明 |
|------|------|
| `j ls` | 列出常用别名（path/url/browser/editor 等） |
| `j ls all` | 列出所有 section 下的别名 |
| `j ls <section>` | 列出指定 section（如 `j ls path`） |
| `j contain <alias>` | 在所有分类中查找别名 |
| `j contain <alias> <sections>` | 在指定分类中查找（逗号分隔） |

## 🚀 打开

| 命令 | 说明 |
|------|------|
| `j <alias>` | 打开应用/文件/URL |
| `j <browser> <url_alias>` | 用浏览器打开 URL |
| `j <browser> <text>` | 用浏览器搜索（默认 Bing，可配置） |
| `j <editor> <file>` | 用编辑器打开文件 |

> **智能识别**：CLI 可执行文件在当前终端执行（支持管道），GUI 应用(.app)用系统打开

## 📝 日报系统

| 命令 | 说明 |
|------|------|
| `j report <content>` | 写入日报（自动追加日期前缀） |
| `j reportctl new [date]` | 开启新的一周（周数+1） |
| `j reportctl sync [date]` | 同步周数和日期 |
| `j reportctl push [msg]` | 推送周报到远程 git 仓库 |
| `j reportctl pull` | 从远程 git 仓库拉取周报 |
| `j reportctl set-url [url]` | 设置/查看 git 仓库地址 |
| `j reportctl open` | 用内置 TUI 编辑器打开日报文件全文编辑 |
| `j check [N]` | 查看日报最近 N 行（默认 5） |
| `j search <N/all> <kw>` | 在日报中搜索关键字 |
| `j search <N/all> <kw> -f` | 模糊搜索（大小写不敏感） |

> 日报默认路径: `~/.jdata/report/week_report.md`
> 自定义路径: `j change report week_report <path>`
> 配置远程仓库: `j reportctl set-url <repo_url>`

## 📋 待办备忘录

| 命令 | 说明 |
|------|------|
| `j todo` | 进入 TUI 待办管理界面（全屏交互） |
| `j td` | 同上（别名） |
| `j todo 买牛奶` | 快速添加一条待办 |

### TUI 界面快捷键

| 按键 | 功能 |
|------|------|
| `n` / `↓` / `j` | 向下移动 |
| `N` / `↑` / `k` | 向上移动 |
| `空格` / `回车` | 切换完成状态 `[x]` / `[ ]` |
| `a` | 添加新待办 |
| `e` | 编辑选中待办 |
| `d` | 删除待办（需确认） |
| `y` | 复制选中待办到系统剪切板 |
| `f` | 过滤切换（全部 / 未完成 / 已完成） |
| `J` / `K` | 调整待办顺序（下移 / 上移） |
| `s` | 手动保存 |
| `?` | 查看完整帮助 |
| `q` | 退出（有未保存修改时需先保存或用 `q!` 强制退出） |
| `q!` | 强制退出（丢弃未保存的修改） |

### 输入/编辑模式快捷键

| 按键 | 功能 |
|------|------|
| `←` / `→` | 移动光标 |
| `Home` / `End` | 跳到行首 / 行尾 |
| `Backspace` | 删除光标前字符 |
| `Delete` | 删除光标处字符 |
| `Enter` | 确认提交 |
| `Esc` | 取消 |

> 数据存储路径: `~/.jdata/todo/todo.json`

## 📜 脚本 & ⏳ 倒计时

| 命令 | 说明 |
|------|------|
| `j concat <name> "<content>"` | 创建脚本并注册为别名（保存到 `~/.jdata/scripts/`） |
| `j concat <name>` | 脚本已存在时打开 TUI 编辑器修改脚本内容 |
| `j <script> [args...]` | 在当前终端执行脚本 |
| `j <script> -w [args...]` | 在**新终端窗口**中执行脚本 |
| `j time countdown <duration>` | 启动倒计时（支持 30s / 5m / 1h） |

> `-w` 或 `--new-window` 标志可让脚本在新终端窗口中执行，用于需要后台运行的场景

### 🔗 脚本环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量，命名规则为 `J_<别名大写>`（`-` 转为 `_`）：

```bash
#!/bin/bash
# 已注册: chrome → /Applications/Google Chrome.app
# 已注册: vscode → /Applications/Visual Studio Code.app
# 已注册: my-tool → /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_VSCODE" ./src
"$J_MY_TOOL" --version
```

> 覆盖 section: `path`、`inner_url`、`outer_url`、`script`
> 新窗口执行（`-w`）同样支持环境变量注入
> ⚠️ 路径含空格时，脚本中必须用双引号包裹变量：`"$J_CHROME"` 而非 `$J_CHROME`

## ⚙️ 系统设置

| 命令 | 说明 |
|------|------|
| `j log mode <verbose/concise>` | 设置日志模式 |
| `j change <section> <field> <val>` | 直接修改配置字段 |
| `j clear` | 清屏 |
| `j version` | 版本信息 |
| `j help` | 帮助信息 |
| `j exit` | 退出（交互模式） |
| `j completion [shell]` | 生成 shell 补全脚本（支持 zsh/bash） |

---

## 🔄 安装 & 更新

### 一键安装（推荐）
```bash
# 安装最新版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# 安装指定版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- v1.0.0
```

### 从 crates.io 安装
```bash
cargo install j-cli
```

### 从 GitHub Release 下载
```bash
# macOS ARM64 (M1/M2/M3/M4)
curl -fsSL https://github.com/LingoJack/j/releases/latest/download/j-darwin-arm64.tar.gz | tar xz
sudo mv j /usr/local/bin/
```

### 更新
```bash
# 一键更新（安装脚本方式）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# 从 crates.io 更新
cargo install j-cli

# 查看当前版本
j version
```

> **注意**：`cargo install` 会自动检测 crates.io 上的最新版本并更新，无需先卸载。

---

## 🗑️ 卸载

```bash
# 使用安装脚本卸载（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# 或通过 cargo 卸载（cargo 安装的用户）
cargo uninstall j-cli

# 或手动删除
sudo rm /usr/local/bin/j  # 一键安装方式
rm ~/.cargo/bin/j          # cargo 安装方式

# （可选）删除数据目录（包含配置、历史、脚本、日报等）
rm -rf ~/.jdata
```

> **注意**：卸载命令只会删除二进制文件，用户数据（`~/.jdata/`）会保留。如需彻底清理，请手动删除数据目录。

---

## 🤖 AI 对话

| 命令 | 说明 |
|------|------|
| `j chat` / `j ai` | 进入 TUI 对话界面（全屏交互） |
| `j chat 你好` / `j ai 你好` | 进入对话并发送首条消息 |

### 配置

首次使用需配置 LLM 模型提供方。在对话界面中按 **Ctrl+E** 打开内置配置界面，可视化管理模型提供方。

配置文件路径: `~/.jdata/agent/data/agent_config.json`（也可手动编辑）

```json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-your-api-key",
      "model": "gpt-4o"
    }
  ],
  "active_index": 0,
  "system_prompt": "你是一个有用的助手。",
  "stream_mode": true,
  "max_history_messages": 20,
  "theme": "dark"
}
```

> 支持配置多个模型提供方，可在对话中通过 `Ctrl+T` 切换

### 配置界面

按 `Ctrl+E` 进入可视化配置界面，可编辑模型提供方和全局设置：

| 按键 | 功能 |
|------|------|
| `↑` / `k` | 向上移动光标 |
| `↓` / `j` | 向下移动光标 |
| `Tab` / `→` | 切换到下一个 Provider |
| `Shift+Tab` / `←` | 切换到上一个 Provider |
| `Enter` | 进入编辑模式（修改当前字段） |
| `a` | 新增 Provider |
| `d` | 删除当前 Provider |
| `s` | 将当前 Provider 设为活跃模型 |
| `Esc` | 保存配置并返回对话 |

> **提示**：`stream_mode` 和 `theme` 字段直接按 `Enter` 切换，无需手动输入

### 主题风格

支持以下主题（在配置界面中选中 `theme` 字段按 `Enter` 循环切换）：

| 主题 | 说明 |
|------|------|
| `dark` | 深色主题（默认） |
| `light` | 浅色主题 |
| `dracula` | Dracula 配色 |
| `gruvbox` | Gruvbox 配色 |
| `monokai` | Monokai 配色 |
| `nord` | Nord 配色 |

### 对话界面快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `↑` / `↓` | 滚动对话记录 |
| `PageUp` / `PageDown` | 快速滚动（10行） |
| `←` / `→` | 移动输入光标 |
| `Home` / `End` | 跳到输入行首/行尾 |
| `Ctrl+T` | 切换模型提供方 |
| `Ctrl+L` | 清空对话历史 |
| `Ctrl+Y` | 复制最后一条 AI 回复 |
| `Ctrl+B` | 进入消息浏览模式 |
| `Ctrl+S` | 切换流式/整体输出 |
| `Ctrl+E` | 打开配置界面（可视化编辑模型配置） |
| `?` | 显示帮助 |
| `Esc` / `Ctrl+C` | 退出对话 |

### 消息浏览模式

按 `Ctrl+B` 进入浏览模式，可选中任意历史消息并复制到剪切板：

| 按键 | 功能 |
|------|------|
| `↑` / `k` | 选中上一条消息 |
| `↓` / `j` | 选中下一条消息 |
| `y` / `Enter` | 复制选中消息到剪切板 |
| `Esc` | 返回对话模式 |

### 功能特性

- **Markdown 渲染**：AI 回复支持标题、加粗、斜体、行内代码、代码块（语法高亮）、列表、表格、引用块
- **代码高亮**：支持 Rust、Python、JavaScript/TypeScript、Go、Java、Bash/Shell、C/C++、SQL、Ruby 等语言
- **流式/整体输出**：默认流式逐字输出，可通过 `Ctrl+S` 切换为等待完整回复后再显示
- **对话持久化**：对话自动保存到 `~/.jdata/agent/data/chat_session.json`，重启后恢复
- **多模型支持**：可配置多个 LLM 提供方（OpenAI、DeepSeek 等），运行时切换

---

## 💡 使用技巧

- 不带参数运行 `j` 进入**交互模式**，支持 Tab 补全和历史建议
- 交互模式下用 `!` 前缀执行 shell 命令（如 `!ls -la`），自动注入别名环境变量
- 交互模式下输入 `!`（不带命令）进入交互式 shell 模式（提示符变为绿色 `shell >`），cd 等状态延续，输入 `exit` 返回 copilot
- 交互模式下参数支持 `$J_XXX` / `${J_XXX}` 环境变量引用（如 `open "$J_VSCODE"`）
- 路径含空格时用引号包裹：`j set app "/Applications/My App.app"`
- URL 会自动识别并归类到 `inner_url`，无需手动指定 section
- `report` 命令内容不会记入历史，保护日报隐私
- CLI 工具（如 rg、fzf）注册后可直接在终端执行并支持管道
- 脚本需要后台运行时，使用 `-w` 标志在新窗口中执行（如 `j deploy -w`）
- 待办备忘录支持 markdown 风格 `[x]` / `[ ]` checkbox，`j todo` 进入全屏 TUI 管理
- 启用 shell Tab 补全：`eval "$(j completion zsh)"` 加入 `.zshrc` 即可在快捷模式下补全命令、别名和文件路径
