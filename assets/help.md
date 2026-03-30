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
j check                   # 查看最近 10 行
j check 20                # 查看最近 20 行

# 进入交互模式（带 Tab 补全 + 历史建议）
j
```

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
| `j check [N]` | 查看日报最近 N 行（默认 10） |
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
| `j todo add 买牛奶` | 快速添加一条待办 |
| `j todo list` / `j td list` | 输出待办列表（Markdown 渲染）|
| `j todo list --done` / `j td list -d` | 仅显示已完成的待办 |
| `j todo list --undone` / `j td list -u` | 仅显示未完成的待办 |

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
| `Alt+↑` / `Alt+↓` | 预览区滚动（长待办内容时可用） |
| `?` | 查看完整帮助 |
| `q` | 退出（有未保存修改时需先保存或用 `q!` 强制退出） |

### 完成时写入日报联动

标记完成时自动询问是否写入日报：

| 操作 | 效果 |
|------|------|
| `空格` / `回车` 标记完成 | 底部显示确认提示 |
| `Enter` / `y` / `Y` | 写入日报 + 自动保存 todo |
| 其他任意键 | 标记完成，不写入日报 |

> 数据存储路径: `~/.jdata/report/todo.json`

## 📜 脚本 & ⏳ 倒计时

| 命令 | 说明 |
|------|------|
| `j concat <name> "<content>"` | 创建脚本并注册为别名（保存到 `~/.jdata/scripts/`） |
| `j concat <name>` | 脚本已存在时打开 TUI 编辑器修改脚本内容 |
| `j <script> [args...]` | 在当前终端执行脚本 |
| `j <script> -w [args...]` | 在**新终端窗口**中执行脚本 |
| `j time countdown <duration>` | 启动倒计时（支持 30s / 5m / 1h） |

> `-w` 或 `--new-window` 标志可让脚本在新终端窗口中执行，用于需要后台运行的场景

### 脚本环境变量注入

执行脚本时，所有已注册的别名路径会自动注入为环境变量，命名规则为 `J_<别名大写>`（`-` 转为 `_`）：

```bash
#!/bin/bash
# 已注册: chrome → /Applications/Google Chrome.app
# 已注册: my-tool → /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
```

> 覆盖 section: `path`、`inner_url`、`outer_url`、`script`
> 路径含空格时，脚本中必须用双引号包裹变量：`"$J_CHROME"` 而非 `$J_CHROME`

## ⚙️ 系统设置

| 命令 | 说明 |
|------|------|
| `j log mode <verbose/concise>` | 设置日志模式 |
| `j change <section> <field> <val>` | 直接修改配置字段 |
| `j clear` | 清屏 |
| `j version` | 版本信息 |
| `j help` | 帮助信息 |
| `j exit` | 退出（交互模式，或按 `Ctrl+Q` / `Ctrl+D`） |
| `j completion [shell]` | 生成 shell 补全脚本（支持 zsh/bash） |
| `j update` | 更新到最新版本（自动检测安装来源） |
| `j update --check` | 仅检查是否有新版本 |

---

## 🤖 AI 对话

| 命令 | 说明 |
|------|------|
| `j chat` / `j ai` | 进入 TUI 对话界面（全屏交互） |
| `j chat 你好` / `j ai 你好` | 进入对话并发送首条消息 |
| `j ai --remote` | 启用远程控制模式（手机扫码控制） |
| `j ai --remote --port 8080` | 指定远程控制端口（默认 9390） |
| `j ai -c` | 延续上一个会话 |
| `j ai --session <id>` | 指定会话 ID 继续 |

### 远程控制模式

`--remote` 参数启用远程控制功能，可通过手机扫码在网页端控制终端中的 AI 对话：

```bash
j ai --remote              # 默认端口 9390
j ai --remote --port 8080  # 自定义端口
j ai -c --remote           # 延续会话 + 远程控制
```

启动后会显示二维码，手机扫描后即可在浏览器中操作。

> 注意：`--remote` 模式会强制进入 TUI 界面，忽略消息内容参数

### 配置

首次运行 `j chat` 时，若尚未配置模型提供方，会自动进入内置配置界面完成初始配置。已有配置后，也可随时在对话界面中按 **Ctrl+E** 重新编辑。

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
  "theme": "dark",
  "tools_enabled": true
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

> `stream_mode` 和 `theme` 字段直接按 `Enter` 切换，无需手动输入

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
| `Ctrl+L` | 归档当前对话（保存并清空） |
| `Ctrl+R` | 还原归档对话 |
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
| `A` | 消息内容向上滚动 1 行（细粒度） |
| `D` | 消息内容向下滚动 1 行（细粒度） |
| `y` / `Enter` | 复制选中消息到剪切板 |
| `Esc` | 返回对话模式 |

### 归档对话功能

对话支持归档和还原，方便保存有价值的对话历史：

**归档对话（Ctrl+L）**：
- 按下 `Ctrl+L` 后，当前对话会被保存到归档
- 默认归档名称格式：`archive-YYYY-MM-DD`（如 `archive-2026-02-25`）
- 如果同名归档已存在，自动添加后缀（如 `archive-2026-02-25(1)`）
- 归档后当前会话自动清空

**还原归档（Ctrl+R）**：
- 按下 `Ctrl+R` 进入归档列表
- 使用 `↑` / `↓` 或 `j` / `k` 选择归档
- 按 `Enter` 还原选中的归档
- 按 `d` 删除选中的归档
- 按 `Esc` 取消返回

**归档存储位置**：`~/.j/chat/archives/`

### 功能特性

- **Markdown 渲染**：AI 回复支持标题、加粗、斜体、行内代码、代码块（语法高亮）、列表、表格、引用块
- **代码高亮**：支持 Rust、Python、JavaScript/TypeScript、Go、Java、Bash/Shell、C/C++、SQL、Ruby 等语言
- **流式/整体输出**：默认流式逐字输出，可通过 `Ctrl+S` 切换为等待完整回复后再显示
- **对话持久化**：对话自动保存到 `~/.jdata/agent/data/chat_session.json`，重启后恢复
- **多模型支持**：可配置多个 LLM 提供方（OpenAI、DeepSeek 等），运行时切换
- **工具调用**：支持 Function Calling，AI 可执行 shell 命令和读取文件（危险命令需确认）
- **Context Compact**：三层对话压缩机制，自动管理上下文窗口

## 🛠️ AI 工具 & 权限

### 内置工具

| 工具名 | 功能 | 需确认 |
|--------|------|--------|
| `Bash` | 执行 shell 命令；`run_in_background: true` 时后台执行并返回 task_id | ✅ |
| `Read` | 读取本地文件（支持行号范围） | |
| `Write` | 写入文件（自动创建目录） | ✅ |
| `Edit` | 编辑文件（精确字符串替换） | ✅ |
| `Glob` | 按模式匹配搜索文件名 | |
| `Grep` | 正则搜索文件内容 | |
| `Ask` | 向用户提结构化选择题 | |
| `WebFetch` | 获取网页内容并转为 Markdown/纯文本 | |
| `WebSearch` | 使用 Exa Search API 搜索网络 | |
| `Browser` | 浏览器自动化（CDP + Lite fallback） | |
| `TaskOutput` | 查询后台任务输出（`Bash run_in_background` 产生的任务），支持阻塞等待 | |
| `LoadSkill` | 加载指定技能到上下文 | |
| `Compact` | 触发对话压缩以释放上下文窗口 | |
| `Task` | 管理任务（create/get/list/update）；`action` 字段区分操作 | |
| `RegisterHook` | 注册/管理 session 级 hook | ✅ |

**`web_fetch` 参数**：`url`（必需）、`extract_mode`（markdown/text）、`max_chars`、`authorization`、`headers`

**`web_search` 参数**：`query`（必需）、`count`（默认5）、`country`（默认CN）、`search_lang`、`freshness`（pd/pw/pm/py）

**`browser` action**：`start` `stop` `status` `tabs` `open` `navigate` `screenshot`(CDP) `snapshot` `content` `close` `click`(CDP) `type`(CDP) `press`(CDP) `evaluate`(CDP)

> **Lite 模式**（默认）：基于 HTTP 请求，无需安装 Chrome。**CDP 模式**：需 `--features browser_cdp` 编译，支持截图、点击、输入、JS 执行。

### 工具确认快捷键

| 按键 | 功能 |
|------|------|
| `Y` / `Enter` | 执行工具 |
| `N` / `Esc` | 拒绝执行 |

### .jcli/ 权限配置

在项目根目录创建 `.jcli/` 目录，在其中放置 `permissions.yaml` 文件，可细粒度控制工具的自动执行权限。程序会从当前目录向上查找 `.jcli/` 目录。

```yaml
# .jcli/permissions.yaml
# allow_all: true  # 完全放开

allow:
  - "Bash(cargo build:*)"   # Bash 命令前缀匹配
  - "Bash(cargo test:*)"
  - "Read"                   # 工具级别放行
  - "Glob"
  - "Write(path:/Users/jack/projects/*)"  # 文件路径前缀匹配
  - "WebFetch(domain:docs.rs)"            # URL 域名匹配

deny:
  - "Bash(rm -rf:*)"        # 黑名单（优先于 allow）
  - "Bash(sudo:*)"
```

> `deny` 优先于 `allow`。无 `.jcli/` 目录时保持默认行为（需确认的工具弹确认框）

## 🧩 Skill 技能系统

在 `~/.jdata/agent/skills/` 下创建 skill 目录，AI 通过 `load_skill` 工具按需加载技能。

**创建 Skill**：

```bash
mkdir -p ~/.jdata/agent/skills/my-skill
cat > ~/.jdata/agent/skills/my-skill/SKILL.md << 'EOF'
---
name: my-skill
description: 技能描述
argument-hint: "[参数说明]"
---

指令正文，$ARGUMENTS 会被替换为参数...
EOF
```

**使用方式**：

| 操作 | 说明 |
|------|------|
| 输入 `@` | 弹出技能选择列表（支持过滤） |
| `↑↓` 选择 + `Tab/Enter` | 补全技能名称 |
| `@skill 参数` + 发送 | AI 从 skills 摘要识别后调用 `load_skill` |

> Skill 目录支持 `references/` 子目录存放参考文件，会自动附加到上下文

## 🪝 AI Hook

Hook 允许在关键操作节点注入自定义脚本，支持三级配置：

1. **用户级**：`~/.jdata/agent/hooks.yaml` — 全局生效
2. **项目级**：`.jcli/hooks.yaml` — 项目目录下生效
3. **Session 级**：通过 `register_hook` 工具由 AI 动态注册 — 仅当前会话

**执行顺序**：用户级 → 项目级 → Session 级，链式执行。任何 `abort` 立即中止。

**可用事件**：

| 事件 | 触发时机 |
|------|----------|
| `pre_send_message` | 用户发送消息前 |
| `post_send_message` | 用户发送消息后 |
| `pre_llm_request` | LLM API 请求前 |
| `post_llm_response` | LLM 回复完成后 |
| `pre_tool_execution` | 工具执行前 |
| `post_tool_execution` | 工具执行后 |
| `session_start` | 会话启动时 |
| `session_end` | 会话退出时 |

**配置示例**（`~/.jdata/agent/hooks.yaml`）：
```yaml
pre_send_message:
  - command: "python3 ~/.jdata/agent/hooks/inject_time.py"
    timeout: 5
session_start:
  - command: "echo '{\"inject_messages\": [{\"role\": \"user\", \"content\": \"当前用户: jack\"}]}'"
```

**脚本协议**：stdin 接收 HookContext JSON，stdout 输出 HookResult JSON（可为空）。exit 0 成功，非零视为 abort。默认超时 10 秒。

> 详细的 HookContext/HookResult 字段说明见项目 README

---

## 🔄 安装 & 更新

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

## 🗑️ 卸载

```bash
# 使用安装脚本卸载（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# 或通过 cargo 卸载
cargo uninstall j-cli

# （可选）删除数据目录
rm -rf ~/.jdata
```

> 卸载命令只会删除二进制文件，用户数据（`~/.jdata/`）会保留。

---

## 💡 使用技巧

- 不带参数运行 `j` 进入**交互模式**，支持 Tab 补全和历史建议
- 交互模式下按 `Ctrl+Q` 快速退出（等同于 `exit` 命令或 `Ctrl+D`）
- 交互模式下用 `!` 前缀执行 shell 命令（如 `!ls -la`），自动注入别名环境变量
- 交互模式下输入 `!`（不带命令）进入交互式 shell 模式（提示符变为绿色 `shell >`），cd 等状态延续，输入 `exit` 或按 `Ctrl+D` 返回 copilot
- 路径含空格时用引号包裹：`j set app "/Applications/My App.app"`
- URL 会自动识别并归类到 `inner_url`，无需手动指定 section
- CLI 工具（如 rg、fzf）注册后可直接在终端执行并支持管道
- 脚本需要后台运行时，使用 `-w` 标志在新窗口中执行（如 `j deploy -w`）
- 启用 shell Tab 补全：`eval "$(j completion zsh)"` 加入 `.zshrc`
