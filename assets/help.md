# work-copilot (j) — 快捷命令行工具

> 一条命令打开一切，高效管理日常工作流

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

# 进入交互模式（带 Tab 补全 + 历史建议）
j
```

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
| `/` | 打开命令面板（toggle/edit/add/delete/copy/filter/move/save/quit/help） |
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
# 已注册: chrome -> /Applications/Google Chrome.app
# 已注册: my-tool -> /usr/local/bin/my-tool

open -a "$J_CHROME" https://example.com
"$J_MY_TOOL" --version
```

> 覆盖 section: `path`、`inner_url`、`outer_url`、`script`
> 路径含空格时，脚本中必须用双引号包裹变量：`"$J_CHROME"` 而非 `$J_CHROME`

## Markdown 笔记本

| 命令 | 说明 |
|------|------|
| `j md` / `j notebook` | 进入 TUI 笔记管理界面（全屏交互） |
| `j nb` | 同上（别名） |
| `j md <title>` | 打开指定笔记（不存在则新建） |
| `j md <file-path>` | 直接编辑任意 Markdown/文本文件（支持 `~/`、相对路径、绝对路径） |
| `j md list` | 列出所有笔记 |
| `j md search <keyword>` | 搜索笔记（标题+内容） |
| `j md delete <title>` | 删除笔记 |
| `j md open` | 在系统文件管理器中打开 notebook 根目录 |
| `j md rename <old> <new>` | 重命名笔记 |
| `j md mkdir <dir>` | 创建子目录 |
| `j md mv <src> <dest>` | 移动笔记到新路径 |

### TUI 界面快捷键

| 按键 | 功能 |
|------|------|
| `↓` / `j` / `n` | 向下移动 |
| `↑` / `k` / `N` | 向上移动 |
| `Enter` / `e` | 编辑选中笔记（打开内置 Markdown 编辑器） |
| `a` | 新建笔记（输入标题后打开编辑器） |
| `d` | 删除笔记（需确认） |
| `r` | 重命名笔记 |
| `p` | 全屏预览模式 |
| `/` | 打开命令面板（搜索/重命名/删除/新建目录/移动/调整比例/帮助） |
| `s` | 刷新笔记列表 |
| `y` | 复制笔记名到剪切板 |
| `o` | 在系统文件管理器中打开笔记目录 |
| `[` / `]` | 缩小/放大左侧面板比例（每次 5%） |
| `Tab` | 展开/折叠目录 |
| `Esc` | 退出（有搜索过滤时先清除过滤） |
| `?` | 查看帮助 |

> 笔记存储路径: `~/.jdata/notebook/`，支持子目录组织
> 支持对 notebook 内笔记和外部文件统一使用内置 Markdown 编辑器
> 命令面板可执行 `search`、`rename`、`delete`、`mkdir`、`mv`、`open`、`ratio`、`help`

## 系统设置

| 命令 | 说明 |
|------|------|
| `j log mode <verbose/concise>` | 设置日志模式 |
| `j config <section> <field> <val>` | 直接修改配置字段 |
| `j clear` | 清屏 |
| `j version` / `j v` | 版本信息 |
| `j help` / `j h` | 打开多标签帮助 TUI |
| `j exit` / `j q` / `j quit` | 退出（交互模式，或按 `Ctrl+Q` / `Ctrl+D`） |
| `j completion [shell]` | 生成 shell 补全脚本（支持 `zsh` / `bash`，默认 `zsh`） |
| `j update` / `j up` | 更新到最新版本（自动检测安装来源） |
| `j update --check` | 仅检查是否有新版本 |

### 帮助界面快捷键

| 按键 | 功能 |
|------|------|
| `←` / `→` / `h` / `l` | 切换帮助 Tab |
| `Tab` / `Shift+Tab` | 切换到下一个 / 上一个 Tab |
| `1`-`0` | 直接跳到指定 Tab |
| `↑` / `↓` / `j` / `k` | 滚动内容 |
| `PageUp` / `PageDown` | 快速滚动 |
| `Home` / `End` | 跳到顶部 / 底部 |
| `q` / `Esc` / `Ctrl+C` | 退出帮助 |

---

## AI 对话

| 命令 | 说明 |
|------|------|
| `j chat` / `j ai` | 进入 TUI 对话界面（全屏交互） |
| `j chat 你好` / `j ai 你好` | 快速发送消息并打印回复（oneshot 模式） |
| `j ai --remote` | 启用远程控制模式（手机扫码控制） |
| `j ai --remote --port 8080` | 指定远程控制端口（默认 9390） |
| `j ai -c` | 延续上一个会话（oneshot 模式） |
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

首次运行 `j chat` 时，若尚未配置模型提供方，会自动进入内置配置界面完成初始配置。已有配置后，也可随时在对话界面中按 **Ctrl+E** 或输入 `/config` 重新编辑。

配置文件路径: `~/.jdata/agent/data/agent_config.json`（也可手动编辑）

```json
{
  "providers": [
    {
      "name": "GPT-4o",
      "api_base": "https://api.openai.com/v1",
      "api_key": "sk-your-api-key",
      "model": "gpt-4o",
      "supports_vision": true
    }
  ],
  "active_index": 0,
  "max_history_messages": 20,
  "max_context_tokens": 100000,
  "theme": "midnight",
  "tools_enabled": true,
  "max_tool_rounds": 10,
  "tool_confirm_timeout": 0,
  "auto_restore_session": false
}
```

> 支持配置多个模型提供方，可在对话中切换

### 配置界面

按 `Ctrl+E` 或输入 `/config` 进入可视化配置界面。当前界面包含 `Model`、`Session`、`Global`、`Tools`、`Skills`、`Hooks`、`Commands`、`Teammates`、`Archive` 九个 Tab，不同 Tab 的按键略有不同。

| 按键 | 功能 |
|------|------|
| `←` / `→` | 切换 Tab |
| `↑` / `↓` / `j` / `k` | 在当前列表中移动 |
| `Enter` | 编辑字段 / 执行当前项动作 |
| `Esc` | 保存配置并返回对话 |

**Model Tab**：
- `Tab` / `Shift+Tab`：切换 Provider
- `a`：新增 Provider
- `d`：删除当前 Provider
- `s`：将当前 Provider 设为活跃模型

**Tools / Skills Tab**：
- `Enter` / `空格`：启用或禁用当前项
- `a`：全部启用
- `d`：全部禁用
- `t`：仅 Tools Tab 可切换总开关

**Session / Archive / Teammates Tab**：
- `Session`：`Enter` 恢复，`d` 删除，`n` 新建
- `Archive`：`Enter` 还原，`d` 删除
- `Teammates`：`Enter` 查看状态，`s` 停止选中 teammate

### 主题风格

支持以下主题（输入 `/theme` 切换或在配置界面中修改）：

| 主题 | 说明 |
|------|------|
| `midnight` | Midnight（默认） |
| `dark` | 深色主题 |
| `light` | 浅色主题 |
| `nord` | Nord 配色 |
| `monokai` | Monokai 配色 |
| `anthropic_light` | Anthropic Light |
| `anthropic_dark` | Anthropic Dark |

### 对话界面快捷键

| 按键 | 功能 |
|------|------|
| `Enter` | 发送消息 |
| `Shift+Enter` / `Alt+Enter` | 插入换行符（多行输入） |
| `↑` / `↓` | 滚动对话记录（多行输入时移动光标） |
| `PageUp` / `PageDown` | 快速滚动 |
| `←` / `→` | 移动输入光标 |
| `Home` / `End` | 跳到输入行首/行尾 |
| `Backspace` / `Delete` | 删除字符 |
| `Ctrl+Y` | 复制最后一条 AI 回复 |
| `Ctrl+B` | 进入消息浏览模式 |
| `Ctrl+G` | 打开日志窗口 |
| `Ctrl+M` | 切换鼠标模式（滚动 / 自由选中） |
| `Ctrl+O` | 切换工具详情展开/折叠 |
| `Ctrl+E` | 打开配置界面 |
| `F1` / `?`（输入框为空时） | 显示帮助 |
| `Esc` | 取消当前流式输出 / 退出对话 |
| `Ctrl+C` | 强制退出对话 |

### 斜杠命令（/ 命令）

在输入框中输入 `/` 即可唤起斜杠命令弹窗，支持模糊过滤：

| 命令 | 说明 |
|------|------|
| `/copy` | 复制最后一条 AI 回复 |
| `/log` | 打开日志窗口 |
| `/browse` | 浏览历史消息 |
| `/config` | 打开配置界面 |
| `/model` | 切换模型 |
| `/archive` | 归档当前对话 |
| `/clear` | 新建对话（清空当前会话） |
| `/theme` | 切换主题 |
| `/resume` | 恢复历史会话 |
| `/dump` | 导出真实传给 AI 的 system prompt 和 messages |
| `/dump-processed` | 导出经处理管线后的最终请求数据 |
| `/teammate` | 打开 Teammate 面板 |

### @ 补全系统

在输入框中输入 `@` 唤起补全弹窗，支持多种引用类型：

| 输入 | 补全类型 | 说明 |
|------|----------|------|
| `@` | 混合列表 | 弹出分类入口（skill:、command:、file:）及匹配项 |
| `@skill:` | 技能补全 | 从已安装技能中搜索并补全 |
| `@command:` | 命令补全 | 从内置命令中搜索并补全 |
| `@file:` | 文件补全 | 从当前工作目录搜索文件路径，支持目录导航 |

> 补全弹窗操作：`↑↓` 选择、`Tab`/`Enter` 确认、`Esc` 取消、`Backspace` 回退、`空格` 关闭弹窗

### 消息浏览模式

按 `Ctrl+B` 或输入 `/browse` 进入浏览模式，可选中任意历史消息并复制到剪切板：

| 按键 | 功能 |
|------|------|
| `↑` / `k` | 选中上一条消息 |
| `↓` / `j` | 选中下一条消息 |
| `PageUp` | 当前消息内容向上微调滚动 |
| `PageDown` | 当前消息内容向下微调滚动 |
| `Tab` | 切换角色过滤（全部 / AI / 用户） |
| `y` / `Enter` | 复制选中消息到剪切板 |
| 直接输入字符 | 按关键词过滤消息 |
| `Backspace` | 删除过滤字符 |
| `Esc` | 有过滤时先清除过滤；无过滤时返回对话模式 |

### 归档对话功能

对话支持归档和还原，方便保存有价值的对话历史：

**归档对话（/archive）**：
- 输入 `/archive` 后确认，当前对话会被保存到归档
- 默认归档名称格式：`archive-YYYY-MM-DD`
- 如果同名归档已存在，自动添加后缀（如 `archive-2026-02-25(1)`）
- 归档后当前会话自动清空

**恢复会话（/resume）**：
- 输入 `/resume` 进入会话列表
- 使用 `↑` / `↓` 或 `j` / `k` 选择历史会话
- 按 `Enter` 恢复选中的会话

### Teammate 系统

Teammate 是多个并行运行的子 Agent，每个有独立的 system prompt 和消息历史。可通过 `/teammate` 或配置界面（Ctrl+E → Teammates Tab）查看和管理当前团队状态。

- **创建方式**：通常由 AI 通过 `AgentTeam` / `CreateTeammate` 工具创建
- **通信方式**：Teammate 之间通过 `SendMessage` 和 `@mentions` 协作
- **使用场景**：全栈开发（前端 + 后端 + 运维）、多领域并行研究、多角色协作

### Agent 工具（高级）

AI 对话内置以下高级工具，支持复杂的多步骤任务：

| 工具 | 功能 |
|------|------|
| `Agent` | 启动子 Agent 自主处理多步骤任务（独立上下文） |
| `AgentTeam` | 批量创建多个 Teammate 并行协作 |
| `Task` | 管理任务（create/get/list/update），支持依赖关系 |
| `TodoWrite` | 创建和管理结构化待办列表（跨多轮对话） |
| `TodoRead` | 读取当前待办列表 |
| `EnterPlanMode` / `ExitPlanMode` | 进入/退出计划模式（只读探索后设计实现方案） |
| `EnterWorktree` / `ExitWorktree` | 创建/退出隔离的 git worktree（避免多会话编辑冲突） |
| `Compact` | 触发对话压缩以释放上下文窗口 |
| `LoadSkill` | 加载指定技能到上下文 |
| `RegisterHook` | 注册/管理 session 级 hook |

> Agent/AgentTeam 启动的子 Agent 拥有独立的上下文窗口，避免干扰主对话

### 功能特性

- **Markdown 渲染**：AI 回复支持标题、加粗、斜体、行内代码、代码块（语法高亮）、列表、表格、引用块
- **代码高亮**：支持 Rust、Python、JavaScript/TypeScript、Go、Java、Bash/Shell、C/C++、SQL、Ruby 等语言
- **流式渲染**：TUI 会在生成过程中持续刷新回复内容和工具状态
- **对话持久化**：对话自动保存到 `~/.jdata/agent/data/sessions/`，重启后恢复
- **多模型支持**：可配置多个 LLM 提供方（OpenAI、DeepSeek 等），运行时通过 `/model` 切换
- **工具调用**：支持 Function Calling，AI 可执行 shell 命令和读取文件（危险命令需确认）
- **Context Compact**：三层对话压缩机制（rolling window + micro_compact + auto_compact），自动管理上下文窗口
- **多行输入**：支持 `Shift+Enter` / `Alt+Enter` 插入换行符，多行输入时方向键移动光标
- **@ 引用系统**：支持引用技能（`@skill:`）、命令（`@command:`）、文件（`@file:`）到对话中
- **斜杠命令**：输入 `/` 弹出命令面板，快速执行操作

## AI 工具 & 权限

### 内置工具

| 工具名 | 功能 | 需确认 |
|--------|------|--------|
| `Bash` | 执行 shell 命令；`run_in_background: true` 时后台执行并返回 task_id | Yes |
| `Read` | 读取本地文件（支持行号范围，可读取图片） | |
| `Write` | 写入文件（自动创建目录） | Yes |
| `Edit` | 编辑文件（精确字符串替换） | Yes |
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
| `RegisterHook` | 注册/管理 session 级 hook | Yes |
| `Agent` | 启动子 Agent 自主处理多步骤任务 | |
| `AgentTeam` | 批量创建多个 Teammate 并行协作 | |
| `TodoWrite` | 创建/更新结构化待办列表 | |
| `TodoRead` | 读取当前待办列表 | |
| `EnterPlanMode` / `ExitPlanMode` | 进入/退出计划模式 | |
| `EnterWorktree` / `ExitWorktree` | 创建/退出 git worktree | |

**`WebFetch` 参数**：`url`（必需）、`extract_mode`（markdown/text）、`max_chars`、`authorization`、`headers`

**`WebSearch` 参数**：`query`（必需）、`count`（默认5）、`type`（auto/keyword/neural）

**`Browser` action**：`start` `stop` `status` `tabs` `open` `navigate` `screenshot`(CDP) `snapshot` `content` `close` `click`(CDP) `type`(CDP) `press`(CDP) `evaluate`(CDP)

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

## Skill 技能系统

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
| 输入 `@` | 弹出混合选择列表（含 skill: 分类入口） |
| `@skill:` | 直接弹出技能选择列表（支持过滤） |
| `↑↓` 选择 + `Tab/Enter` | 补全技能名称 |
| AI 自动调用 `load_skill` | 从 skills 摘要识别后自动加载 |

> Skill 目录支持 `references/` 子目录存放参考文件，会自动附加到上下文

## AGENT.md 项目指令

AGENT.md 是项目级指令系统，让 AI 在每次对话中自动加载项目约定，确保长上下文中始终遵循项目规范。

**创建方式**：

```bash
# 项目级（提交到 git，团队共享）
cat > AGENT.md << 'EOF'
# 项目约定

- 使用 Rust 2024 edition
- 所有公共 API 必须有文档注释
- 错误处理使用 thiserror 而非 anyhow
- 提交信息格式: type(scope): description
EOF

# 项目级（.jcli 目录下，与权限配置同目录）
cat > .jcli/AGENT.md << 'EOF'
# 额外约定
...
EOF

# 个人级（不提交到 git）
cat > AGENT.local.md << 'EOF'
# 个人偏好
...
EOF
```

**加载优先级**（后者覆盖前者）：

| 优先级 | 文件 | 说明 |
|--------|------|------|
| 最低 | `~/.jdata/agent/AGENT.md` | 用户级，所有项目生效 |
| ↓ | `AGENT.md`（从 git root 到 CWD） | 项目级，提交到 git |
| ↓ | `.jcli/AGENT.md`（从 git root 到 CWD） | 项目级，与权限配置同目录 |
| ↓ | `AGENT.local.md`（从 git root 到 CWD） | 个人级，不提交 git |
| 最高 | `.jcli/AGENT.local.md`（从 git root 到 CWD） | 个人级，不提交 git |

> 每个文件上限 200 行 / 25KB。项目级指令自动带有 OVERRIDE 强制力，优先于默认行为。
> 在对话配置界面（Ctrl+E）中选择 "AGENT.md" 字段可编辑用户级指令。

## AI Hook

Hook 允许在关键操作节点注入自定义脚本，支持三级配置：

1. **用户级**：`~/.jdata/agent/hooks.yaml` — 全局生效
2. **项目级**：`.jcli/hooks.yaml` — 项目目录下生效
3. **Session 级**：通过 `register_hook` 工具由 AI 动态注册 — 仅当前会话

**执行顺序**：用户级 -> 项目级 -> Session 级，链式执行。任何 `abort` 立即中止。

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

---

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
