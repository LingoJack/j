# j-cli (j) — 快捷命令行工具

**`j` 是一个快捷命令行工具，核心功能：**

- **别名管理** — 注册 app 路径 / URL / 脚本，`j <alias>` 快速打开
- **日报系统** — 快速写入、查看、搜索日报，自动周数管理
- **待办备忘录** — 内置 TUI 待办管理，支持 markdown checkbox
- **AI 对话** — 内置 TUI AI 对话，多模型、流式输出、工具调用
- **浏览器自动化** — Lite 模式（纯 HTTP）和 CDP 模式（Chromium）
- **脚本系统** — 创建脚本并注册为别名，支持环境变量注入、新窗口执行
- **Skill 技能** — 可扩展的 AI 技能系统，按需加载
- **Hook 系统** — 三级 Hook（用户/项目/Session），灵活扩展 AI 行为
- **移动端远程控制** — 扫码连接手机，远程操作 AI 对话
- **Agent 模式** — 自主多步推理，自动调用工具完成任务
- **交互模式** — 带 Tab 补全 + 历史建议的 REPL 环境

---

## 快速上手

```bash
# 注册应用别名
j set chrome "/Applications/Google Chrome.app"
j set vscode "/Applications/Visual Studio Code.app"

# 注册 URL 别名（自动识别为 inner_url）
j set github https://github.com

# 标记分类（标记后支持组合打开）
j tag chrome browser
j tag vscode editor

# 一键打开
j chrome                  # 打开 Chrome
j chrome github           # 用 Chrome 打开 github 对应的 URL
j chrome "rust lang"      # 用 Chrome 搜索 "rust lang"
j vscode ./src            # 用 VSCode 打开 src 目录

# 写日报 & 查看
j report "完成功能开发"    # 写入今日日报
j check                   # 查看最近 10 行
j check 20                # 查看最近 20 行

# 待办
j todo add 买牛奶         # 快速添加待办
j todo                    # 进入 TUI 管理

# AI 对话
j chat                    # 进入 TUI 对话界面
j chat 你好               # 快速提问

# 进入交互模式（带 Tab 补全 + 历史建议）
j
```

---

## 数据目录

所有数据统一存储在 `~/.jdata/` 下（可通过环境变量 `J_DATA_PATH` 自定义）：

```
~/.jdata/
├── config.yaml          # 主配置文件（别名、分类、设置等）
├── agent/               # AI Agent 相关数据
│   ├── data/            # Agent 数据目录
│   │   ├── agent_config.json   # Agent 配置（模型、API 等）
│   │   ├── chat_history.json   # 对话历史
│   │   ├── archives/           # 归档对话
│   │   ├── system_prompt.md    # 系统提示词
│   │   ├── memory.md           # 记忆文件
│   │   ├── soul.md             # 灵魂文件
│   │   └── style.md            # 回复风格配置
│   ├── logs/            # Agent 日志
│   │   ├── info.log
│   │   └── error.log
│   └── skills/          # 技能目录
├── bin/                 # 内置工具
│   └── md_render        # Markdown 渲染器
├── report/              # 日报目录
│   ├── week_report.md   # 周报文件
│   ├── settings.json    # 日报配置（周数、日期）
│   ├── todo.json        # 待办数据（JSON 格式）
│   └── .git/            # git 仓库（配置远程仓库后生成）
├── scripts/             # j script 创建的脚本
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

## 别名管理

| 命令 | 说明 |
|------|------|
| `j set <alias> <path>` | 设置别名（路径自动归类到 path，URL 归类到 inner_url） |
| `j rm <alias>` | 删除别名（同时清理关联的分类标记） |
| `j rename <alias> <new>` | 重命名别名（同步更新所有分类引用） |
| `j mf <alias> <new_path>` | 修改别名指向的路径 |

## 分类标记

| 命令 | 说明 |
|------|------|
| `j tag <alias> <category>` | 标记别名分类 |
| `j untag <alias> <category>` | 解除别名分类 |

可用分类: `browser`, `editor`, `vpn`, `outer_url`, `script`

> 标记为 browser 后可以用 `j <browser> <url>` 打开链接或搜索
> 标记为 editor 后可以用 `j <editor> <file>` 打开文件

## 列表 & 查找

| 命令 | 说明 |
|------|------|
| `j ls` | 列出常用别名（path/url/browser/editor 等） |
| `j ls all` | 列出所有 section 下的别名 |
| `j ls <section>` | 列出指定 section（如 `j ls path`） |
| `j contain <alias>` | 在所有分类中查找别名 |
| `j contain <alias> <sections>` | 在指定分类中查找（逗号分隔） |

## 打开

| 命令 | 说明 |
|------|------|
| `j <alias>` | 打开应用/文件/URL |
| `j <browser> <url_alias>` | 用浏览器打开 URL |
| `j <browser> <text>` | 用浏览器搜索（默认 Bing，可配置） |
| `j <editor> <file>` | 用编辑器打开文件 |

> **智能识别**：CLI 可执行文件在当前终端执行（支持管道），GUI 应用(.app)用系统打开

---

## 日报系统

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
> 自定义路径: `j config report week_report <path>`
> 配置远程仓库: `j reportctl set-url <repo_url>`

---

## 待办备忘录

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
| `q!` | 强制退出（丢弃未保存的修改） |

### 完成时写入日报联动

标记完成时自动询问是否写入日报：

| 操作 | 效果 |
|------|------|
| `空格` / `回车` 标记完成 | 底部显示确认提示：`写入日报: "内容..."？ (Enter/y 写入, 其他跳过)` |
| `Enter` / `y` / `Y` | 写入日报 + 自动保存 todo |
| 其他任意键 | 标记完成，不写入日报 |

> 写入日报的格式与 `j report` 命令一致：`- 【YYYY/MM/DD】 内容`
> 批量操作：可以连续标记多个待办为完成，每次都会询问是否写入日报

### 预览区功能

- 当选中的待办项内容超出列表显示宽度时，列表下方会自动显示预览区
- 预览区展示完整的待办内容，支持自动换行
- 使用 `Alt+↑` / `Alt+↓` 可在预览区滚动查看长内容
- 切换到其他待办项时，预览区会自动刷新并重置滚动位置

> 数据存储路径: `~/.jdata/report/todo.json`

---

## 脚本 & 倒计时

| 命令 | 说明 |
|------|------|
| `j script <name> "<content>"` | 创建脚本并注册为别名（保存到 `~/.jdata/scripts/`） |
| `j script <name>` | 脚本已存在时打开 TUI 编辑器修改脚本内容 |
| `j <script> [args...]` | 在当前终端执行脚本 |
| `j <script> -w [args...]` | 在**新终端窗口**中执行脚本 |
| `j time countdown <duration>` | 启动倒计时（支持 30s / 5m / 1h） |

> `-w` 或 `--new-window` 标志可让脚本在新终端窗口中执行，用于需要后台运行的场景

### 脚本环境变量注入

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
> 路径含空格时，脚本中必须用双引号包裹变量：`"$J_CHROME"` 而非 `$J_CHROME`

---

## 系统设置

| 命令 | 说明 |
|------|------|
| `j log mode <verbose/concise>` | 设置日志模式 |
| `j config <section> <field> <val>` | 直接修改配置字段 |
| `j clear` | 清屏 |
| `j version` | 版本信息 |
| `j help` | 帮助信息 |
| `j exit` | 退出（交互模式，或按 `Ctrl+Q` / `Ctrl+D`） |
| `j completion [shell]` | 生成 shell 补全脚本（支持 zsh/bash） |
| `j update` | 更新到最新版本（自动检测安装来源） |
| `j update --check` | 仅检查是否有新版本 |

> GitHub Release 安装和 cargo 安装（包括 Kago 用户）均支持自动更新

---

## AI 对话

| 命令 | 说明 |
|------|------|
| `j chat` / `j ai` | 进入 TUI 对话界面（全屏交互） |
| `j chat 你好` / `j ai 你好` | 进入对话并发送首条消息 |

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

> 归档存储位置：`~/.j/chat/archives/`
> 还原归档会先清空当前会话，如果当前有未归档的对话，还原时会提示确认

### 功能特性

- **Markdown 渲染**：AI 回复支持标题、加粗、斜体、行内代码、代码块（语法高亮）、列表、表格、引用块
- **代码高亮**：支持 Rust、Python、JavaScript/TypeScript、Go、Java、Bash/Shell、C/C++、SQL、Ruby 等语言
- **流式/整体输出**：默认流式逐字输出，可通过 `Ctrl+S` 切换为等待完整回复后再显示
- **对话持久化**：对话自动保存到 `~/.jdata/agent/data/chat_session.json`，重启后恢复
- **多模型支持**：可配置多个 LLM 提供方（OpenAI、DeepSeek 等），运行时切换
- **工具调用**：支持 Function Calling，AI 可执行 shell 命令和读取文件（危险命令需确认）
- **Context Compact**：三层对话压缩机制，自动管理上下文窗口

### Context Compact（对话压缩）

长对话会逐渐消耗上下文窗口，Context Compact 通过三层机制自动管理：

**Layer 1: micro_compact（零 API 成本）**

每轮 Agent 循环开始时自动执行。将较早的工具调用结果（`role="tool"`）替换为 `[Previous: used {tool_name}]` 占位符，保留最近 `keep_recent`（默认 10）个不替换。

- 仅压缩内容超过 800 字符的 tool result
- **豁免工具**：`LoadSkill`、`TaskCreate/List/Get/Update`、`TodoWrite`、`TodoRead` 的结果永远不压缩（承载工作流指令或任务状态）

**Layer 2: auto_compact（LLM 摘要）**

micro_compact 之后，若估算 token 数仍超过阈值（默认 204,800），自动触发：

1. 保存完整对话记录到 `~/.jdata/agent/data/transcripts/transcript_{timestamp}.jsonl`
2. 将对话 JSON（截断到 80,000 字符）发送给当前模型，请求生成摘要（非流式，max_tokens=2000）
3. 清空所有消息，替换为两条消息：
   - `user`: `[Conversation compressed. Transcript: {path}]\n\n{摘要内容}`
   - `assistant`: `Understood. I have the context from the summary. Continuing.`
4. 若 LLM 调用失败，仅记录错误日志，继续使用原消息（graceful degradation）

**Layer 3: Compact 工具（AI 主动触发）**

AI 判断对话过长时可主动调用 `Compact` 工具，触发与 Layer 2 相同的摘要流程。

**配置**（在 `agent_config.json` 的 `compact` 字段中）：

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `enabled` | `true` | 是否启用 Context Compact |
| `token_threshold` | `204800` | 触发 auto_compact 的 token 阈值 |
| `keep_recent` | `10` | micro_compact 保留最近几个 tool result 不替换 |

> Token 估算方式：`JSON 序列化总字节数 / 4`（粗略估算，约 4 字符 = 1 token）
> Transcript 文件保留完整对话历史，可用于回溯

---

## 移动端远程控制

`ai --remote` 启用远程控制，终端会显示二维码，手机扫码即可连接：

**功能特性**：
- 手机浏览器访问，无需安装 App
- 实时同步对话内容
- 支持发送消息、接收回复
- 同一局域网内使用，安全可靠

**使用方式**：

| 按键 | 功能 |
|------|------|
| `Ctrl+R` | 启动远程控制服务器，显示二维码 |
| 手机扫码 | 打开 Sprite Remote 网页客户端 |
| 连接成功 | 终端显示"已连接"，可开始远程操作 |

**安全机制**：
- Token 验证：每次启动生成随机 UUID token
- 局域网限制：仅同一网络内可访问
- 连接状态提示：终端实时显示连接/断开状态

---

## Agent 模式

Agent 模式让 AI 具备自主执行能力，可自动调用工具完成复杂任务：

**核心能力**：
- **多步推理** — 自主规划任务步骤，逐步执行直至完成
- **工具调用** — 自动选择并执行 Bash/Read/Write/Edit/Grep 等工具
- **上下文管理** — 自动压缩对话历史，保持长对话连贯
- **后台任务** — 支持后台执行长时间命令，不阻塞对话

**工作流程**：

```
用户: "帮我分析这个项目的代码结构"
  ↓
Agent: 1. 执行 Bash(ls -la) 查看目录
       2. 执行 Glob(**/*.rs) 找到源文件
       3. 执行 Read 读取关键文件
       4. 汇总分析结果
  ↓
返回完整分析报告
```

**配置项**（在 `agent_config.json` 中）：

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `tools_enabled` | `true` | 启用工具调用 |
| `max_tool_rounds` | `50` | 单次对话最大工具调用轮数 |
| `compact.enabled` | `true` | 启用对话压缩 |
| `compact.token_threshold` | `204800` | 触发压缩的 token 阈值 |
| `compact.keep_recent` | `10` | 保留最近 N 个工具结果 |

**权限控制**：

Agent 执行敏感操作（Bash/Write/Edit）前会请求确认，可通过 `.jcli` 文件配置自动执行权限：

```yaml
permissions:
  allow:
    - "Read"
    - "Glob"
    - "Grep"
    - "Bash(cargo build:*)"
```

---

## AI 工具 & 权限

### 工具调用功能

AI 对话支持工具调用，让 AI 能够执行实际操作。

**启用方式**：在配置界面（`Ctrl+E`）中设置 `tools_enabled` 为 `true`（默认启用）

**内置工具**：

| 工具名 | 功能 | 需确认 |
|--------|------|--------|
| `Bash` | 执行 shell 命令 | Yes |
| `Read` | 读取本地文件（支持行号范围） | |
| `Write` | 写入文件（自动创建目录） | Yes |
| `Edit` | 编辑文件（精确字符串替换） | Yes |
| `Glob` | 按模式匹配搜索文件名 | |
| `Grep` | 正则搜索文件内容 | |
| `Ask` | 向用户提结构化选择题 | |
| `WebFetch` | 获取网页内容并转为 Markdown/纯文本 | |
| `WebSearch` | 使用 Exa Search API 搜索网络 | |
| `Browser` | 浏览器自动化（CDP + Lite fallback） | |
| `BackgroundRun` | 后台执行 shell 命令（不阻塞对话） | Yes |
| `CheckBackground` | 查询后台任务状态和结果 | |
| `LoadSkill` | 加载指定技能到上下文 | |
| `Compact` | 触发对话压缩以释放上下文窗口 | |
| `TaskCreate` | 创建任务 | |
| `TaskList` | 列出所有任务 | |
| `TaskGet` | 获取任务详情 | |
| `TaskUpdate` | 更新任务状态/依赖 | |
| `RegisterHook` | 注册/管理 session 级 hook | Yes |

### `web_fetch` 工具参数

| 参数 | 说明 |
|------|------|
| `url` | 目标 URL（必需） |
| `extract_mode` | 输出格式：`markdown`（默认）或 `text` |
| `max_chars` | 最大返回字符数（默认 50000） |
| `authorization` | Authorization 请求头 |
| `headers` | 自定义请求头 |

### `web_search` 工具参数

| 参数 | 说明 |
|------|------|
| `query` | 搜索关键词（必需） |
| `count` | 搜索结果数量（默认 5，最大 10） |
| `country` | 搜索国家/地区代码（默认 CN） |
| `search_lang` | 搜索语言代码（如 zh-hans、en） |
| `freshness` | 时间范围：`pd`(24h) `pw`(一周) `pm`(一月) `py`(一年) |

### `browser` 工具 action 说明

| action | 说明 | 必需参数 |
|--------|------|----------|
| `start` | 启动浏览器 | 无 |
| `stop` | 停止浏览器 | 无 |
| `status` | 查看浏览器状态 | 无 |
| `tabs` | 列出已打开的标签页 | 无 |
| `open` | 打开 URL 到新标签页 | `url` |
| `navigate` | 导航标签页到新 URL | `url`，`tab_id`（可选） |
| `screenshot` | 截图（需 CDP） | `tab_id`（可选），`full_page`（可选） |
| `snapshot` | 获取页面可交互元素列表 | `tab_id`（可选） |
| `content` | 获取页面文本内容 | `tab_id`（可选） |
| `close` | 关闭标签页 | `tab_id` |
| `click` | 点击元素（需 CDP） | `selector` |
| `type` | 输入文本（需 CDP） | `selector`，`text` |
| `press` | 按键（需 CDP） | `key` |
| `evaluate` | 执行 JavaScript（需 CDP） | `script` |

**浏览器模式说明**：
- **Lite 模式**（默认）：基于 HTTP 请求 + HTML 解析的轻量级浏览器模拟，无需安装 Chrome。支持 tab 管理、页面交互元素识别（snapshot）、链接/表单提取等。
- **CDP 模式**：需用 `cargo build --features browser_cdp` 编译。使用真实 Chrome/Chromium 浏览器，支持截图、点击、输入、JS 执行等完整浏览器控制。
- **Headless 配置**：默认 headless 模式。可通过 `j config setting browser_headless false` 切换为有头模式。

**浏览器工具 FAQ**：

**Q: 怎么编译带 CDP 的版本？**
A: `cargo build --release --features browser_cdp` 或 `cargo install j-cli --features browser_cdp`

**Q: 用户需要额外安装什么？**
A: **CDP 模式**需要本地已安装 Chrome 或 Chromium；**Lite 模式**无任何额外依赖。

**Q: 程序退出时浏览器会关闭吗？**
A: 会。正常退出时（`q`/`Ctrl+C`/进程结束）Chrome 进程自动终止，所有标签页一并关闭。仅 `kill -9` 强杀时可能残留。

**Q: 不加 feature 编译会怎样？**
A: 自动使用 Lite 模式，`screenshot`/`click`/`type`/`evaluate` 等 CDP 专属 action 不可用，但 `content`/`snapshot`/`tabs` 等均可用。

### 工具确认快捷键

| 按键 | 功能 |
|------|------|
| `Y` / `Enter` | 执行工具 |
| `N` / `Esc` | 拒绝执行 |

> `Bash` 工具内置危险命令过滤（如 `rm -rf /`），但仍建议执行前检查命令内容

### .jcli 权限配置

在项目根目录创建 `.jcli` 文件（YAML 格式），可细粒度控制 `j chat` 中工具的自动执行权限。程序会从当前目录向上查找 `.jcli` 文件。

**配置示例**：

```yaml
permissions:
  # 完全放开（跳过所有工具确认）
  # allow_all: true

  allow:
    # Bash 命令前缀匹配（:* 表示任意参数后缀）
    - "Bash(cargo build:*)"
    - "Bash(cargo test:*)"
    - "Bash(cargo fmt:*)"
    - "Bash(git status:*)"
    - "Bash(ls:*)"

    # 工具级别：允许该工具所有调用跳过确认
    - "Read"
    - "Glob"
    - "Grep"

    # 文件写入限制到特定目录
    - "Write(path:/Users/jack/projects/*)"
    - "Edit(path:/Users/jack/projects/*)"

    # WebFetch 限制域名
    - "WebFetch(domain:docs.rs)"

  deny:
    # 黑名单（优先于 allow）
    - "Bash(rm -rf:*)"
    - "Bash(sudo:*)"
```

**匹配规则**：

| 规则格式 | 说明 | 示例 |
|----------|------|------|
| `*` | 匹配所有工具所有调用 | `"*"` |
| `ToolName` | 匹配该工具所有调用 | `"Read"`, `"Grep"` |
| `Bash(cmd:*)` | Bash 命令前缀匹配 | `"Bash(cargo build:*)"` |
| `Write(path:dir/*)` | 文件路径前缀匹配 | `"Write(path:/home/user/*)"` |
| `WebFetch(domain:x)` | URL 域名匹配 | `"WebFetch(domain:docs.rs)"` |

- 无 `.jcli` 文件：保持默认行为（需确认的工具弹确认框）
- `deny` 优先于 `allow`（被 deny 的调用直接拒绝执行）
- `allow_all: true` 或 allow 中包含 `"*"`：所有工具跳过确认

---

## Skill 技能系统

在 `~/.jdata/agent/skills/` 下创建 skill 目录，AI 通过 `load_skill` 工具按需加载技能。

系统提示词中仅包含技能的名称和描述摘要，AI 判断需要时调用 `load_skill` 加载完整指令。

**系统提示词模板占位符**：

| 占位符 | 替换内容 |
|--------|----------|
| `{{.current_dir}}` | 当前工作目录的绝对路径 |
| `{{.skills}}` | 所有技能的 name + description 摘要列表 |
| `{{.skill_dir}}` | 技能目录的绝对路径（`~/.jdata/agent/skills/`） |
| `{{.tools}}` | 所有工具的 name + description 摘要列表 |
| `{{.style}}` | 回复风格配置内容（`Ctrl+E` 中编辑） |
| `{{.memory}}` | 记忆内容（存储用户偏好、重要事项等） |
| `{{.soul}}` | 灵魂/人格设定（定义 AI 的角色和行为风格） |

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
| 启用 tools_enabled | AI 可根据 skills 摘要自主决定是否加载技能 |

> Skill 目录支持 `references/` 子目录存放参考文件，会自动附加到上下文

---

## AI Hook

### Hook 系统

Hook 允许在关键操作节点注入自定义脚本，支持三级配置：

**三级 Hook**：
1. **用户级**：`~/.jdata/agent/hooks.yaml` — 全局生效
2. **项目级**：`.jcli` 文件的 `hooks` 字段 — 项目目录下生效
3. **Session 级**：通过 `register_hook` 工具由 AI 动态注册 — 仅当前会话

**执行顺序**：用户级 → 项目级 → Session 级，链式执行。前者输出影响后者输入，任何 `abort` 立即中止。

**可用事件**：

| 事件 | 触发时机 | 可操作数据 |
|------|----------|------------|
| `pre_send_message` | 用户发送消息前 | user_input, messages |
| `post_send_message` | 用户发送消息后 | user_input, messages |
| `pre_llm_request` | LLM API 请求前 | messages, system_prompt, model |
| `post_llm_response` | LLM 回复完成后 | assistant_output, messages |
| `pre_tool_execution` | 工具执行前 | tool_name, tool_arguments |
| `post_tool_execution` | 工具执行后 | tool_name, tool_result |
| `session_start` | 会话启动时 | messages |
| `session_end` | 会话退出时 | messages |

**用户级配置**（`~/.jdata/agent/hooks.yaml`）：
```yaml
pre_send_message:
  - command: "python3 ~/.jdata/agent/hooks/inject_time.py"
    timeout: 5
pre_llm_request:
  - command: "~/.jdata/agent/hooks/add_context.sh"
session_start:
  - command: "echo '{\"inject_messages\": [{\"role\": \"user\", \"content\": \"当前用户: jack\"}]}'"
```

**项目级配置**（`.jcli` 文件）：
```yaml
permissions:
  allow:
    - "Read"
hooks:
  pre_tool_execution:
    - command: "./scripts/validate_tool.sh"
      timeout: 5
```

**脚本协议**：
- 执行方式：`sh -c "<command>"`，工作目录为用户当前目录
- 环境变量：`JCLI_HOOK_EVENT`（事件名）、`JCLI_CWD`（当前目录）
- stdin：HookContext JSON
- stdout：HookResult JSON（可为空/空 JSON 表示无修改）
- exit 0：成功；非零退出：视为 abort
- 超时：默认 10 秒，超时后 kill 子进程

**HookContext 字段**（stdin JSON，各事件仅填充相关字段）：

| 字段 | 说明 | 可用事件 |
|------|------|----------|
| `event` | 当前触发的事件名（始终存在） | 所有事件 |
| `cwd` | 当前工作目录（始终存在） | 所有事件 |
| `messages` | 当前对话的完整消息列表 | PreSendMessage, PostSendMessage, PreLlmRequest, PostLlmResponse, SessionStart, SessionEnd |
| `user_input` | 本轮用户输入的消息文本 | PreSendMessage, PostSendMessage |
| `system_prompt` | 当前系统提示词 | PreLlmRequest |
| `model` | 当前使用的模型名称 | PreLlmRequest |
| `assistant_output` | 本轮 AI 回复的完整文本 | PostLlmResponse |
| `tool_name` | 当前工具调用的工具名 | PreToolExecution, PostToolExecution |
| `tool_arguments` | 当前工具调用的参数 JSON 字符串 | PreToolExecution |
| `tool_result` | 工具执行的结果内容 | PostToolExecution |

**HookResult 字段**（stdout JSON，脚本只需返回想要修改的字段）：
```json
{
  "user_input": "修改后的用户消息",
  "assistant_output": "修改后的 AI 回复",
  "messages": [],
  "system_prompt": "修改后的系统提示词",
  "tool_arguments": "修改后的工具参数",
  "tool_result": "修改后的工具结果",
  "inject_messages": [{"role": "user", "content": "注入的消息"}],
  "abort": false
}
```

**示例：自动注入时间戳**：
```bash
#!/bin/bash
# ~/.jdata/agent/hooks/inject_time.sh
# 用法：pre_send_message hook
read input
msg=$(echo "$input" | python3 -c "import sys,json; print(json.load(sys.stdin).get('user_input',''))")
echo "{\"user_input\": \"[$(date '+%H:%M')] $msg\"}"
```

**register_hook 工具**：AI 可通过 `register_hook` 工具动态注册 session 级 hook：
```json
{"action": "register", "event": "pre_tool_execution", "command": "./validate.sh", "timeout": 5}
{"action": "list"}
{"action": "remove", "event": "pre_tool_execution", "index": 0}
```

---

## 安装 & 更新

### 一键安装（推荐）
```bash
# 安装最新版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# 安装指定版本
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- v1.0.0
```

### 从 crates.io 安装
```bash
# 标准版（Lite 浏览器模式，无额外依赖）
cargo install j-cli

# 完整版（CDP 浏览器模式，需本地已安装 Chrome/Chromium）
cargo install j-cli --features browser_cdp
```

### 从源码编译
```bash
git clone https://github.com/LingoJack/j.git
cd j && cargo install --path .

# 启用完整浏览器自动化（需本地安装 Chrome/Chromium）
cargo install --path . --features browser_cdp
```

### 更新
```bash
# 内置更新命令（自动检测安装来源）
j update

# 仅检查版本，不更新
j update --check

# 一键更新（安装脚本方式）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh

# 从 crates.io 更新（cargo 安装方式，或 Kago 用户）
cargo install j-cli

# 更新到 CDP 版本（浏览器自动化）
cargo install j-cli --features browser_cdp
```

> `j update` 会自动检测安装来源：GitHub Release 安装会从 GitHub 下载更新；cargo 安装（包括 Kago 用户）会执行 `cargo install j-cli` 更新。

---

## 卸载

```bash
# 使用安装脚本卸载（推荐）
curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh -s -- --uninstall

# 或通过 cargo 卸载（cargo 安装的用户，无论标准版还是 CDP 版）
cargo uninstall j-cli

# 或手动删除
sudo rm /usr/local/bin/j  # 一键安装方式
rm ~/.cargo/bin/j          # cargo 安装方式

# （可选）删除数据目录（包含配置、历史、脚本、日报等）
rm -rf ~/.jdata
```

> 卸载命令只会删除二进制文件，用户数据（`~/.jdata/`）会保留。如需彻底清理，请手动删除数据目录。CDP 版和标准版卸载方式相同。

---

## 使用技巧

- 不带参数运行 `j` 进入**交互模式**，支持 Tab 补全和历史建议
- 交互模式下按 `Ctrl+Q` 快速退出（等同于 `exit` 命令或 `Ctrl+D`）
- 交互模式下用 `!` 前缀执行 shell 命令（如 `!ls -la`），自动注入别名环境变量
- 交互模式下输入 `!`（不带命令）进入交互式 shell 模式（提示符变为绿色 `shell >`），cd 等状态延续，输入 `exit` 或按 `Ctrl+D` 返回 copilot
- 交互模式下参数支持 `$J_XXX` / `${J_XXX}` 环境变量引用（如 `open "$J_VSCODE"`）
- 路径含空格时用引号包裹：`j set app "/Applications/My App.app"`
- URL 会自动识别并归类到 `inner_url`，无需手动指定 section
- `report` 命令内容不会记入历史，保护日报隐私
- CLI 工具（如 rg、fzf）注册后可直接在终端执行并支持管道
- 脚本需要后台运行时，使用 `-w` 标志在新窗口中执行（如 `j deploy -w`）
- 待办备忘录支持 markdown 风格 `[x]` / `[ ]` checkbox，`j todo` 进入全屏 TUI 管理
- 启用 shell Tab 补全：`eval "$(j completion zsh)"` 加入 `.zshrc` 即可在快捷模式下补全命令、别名和文件路径

---

## 技术栈

- **clap** — 命令行参数解析
- **rustyline** — 交互模式 REPL
- **ratatui** — TUI 框架
- **async-openai** — OpenAI API 客户端
- **serde** — 序列化框架

---

## License

MIT
