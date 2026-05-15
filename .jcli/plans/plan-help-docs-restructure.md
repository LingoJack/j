# Help 文档重构计划

## 目标

按命令触发组织文档，每个主题单独目录，每个文件聚焦单一主题。

## 当前问题

1. 所有文档平铺在 `help/` 下，无分组
2. `chat.md` 内容过长（200+行），混合了命令、配置、快捷键、斜杠命令、@补全等多个主题
3. `tools.md` 内容过长（185行），混合了工具列表、Agent模式、权限配置等
4. `daily.md` 混合了日报和待办两个独立功能
5. 文档间关联不明显，用户难以快速定位

## 新目录结构

```
assets/help/
├── quickstart.md              # 快速上手（保留，精简）
├── install.md                 # 安装 & 设置（保留）
├── lock.md                    # 文件加密（保留）
│
├── alias/                     # 别名管理
│   ├── basics.md              # 基础操作：set/rm/rename/mf
│   ├── categories.md          # 分类标记：tag/untag/browser/editor
│   └── list.md                # 列表查找：ls/contain
│
├── ai/                        # AI 对话
│   ├── basics.md              # 基础命令：j chat/j ai/oneshot
│   ├── config.md              # 配置界面：Ctrl+E、各Tab操作
│   ├── commands.md            # 斜杠命令：/copy/log/browse/config...
│   ├── shortcuts.md           # 快捷键：Enter/Shift+Enter/Ctrl+Y...
│   ├── at-reference.md        # @ 引用：@skill:/command:/file:
│   ├── browse.md              # 消息浏览模式：Ctrl+B
│   ├── archive.md             # 归档 & 恢复：/archive、/resume
│   ├── teammate.md            # Teammate 系统：CreateTeammate/SendMessage
│   ├── remote.md              # 远程控制：--remote
│   └── theme.md               # 主题切换：/theme、主题列表
│
├── report/                    # 日报系统
│   ├── basics.md              # 基础命令：j report/j check/j search
│   ├── ctl.md                 # 管理命令：reportctl new/sync/push/pull
│   └── path.md                # 路径配置：默认路径、自定义路径
│
├── todo/                      # 待办备忘
│   ├── basics.md              # 基础命令：j todo/j td add/list
│   ├── tui.md                 # TUI 界面：n/j/k/空格/a/e/d/y/f/J/K
│   └── report-link.md         # 日报联动：完成时写入日报
│
├── md/                        # 笔记本
│   ├── basics.md              # 基础命令：j md/j nb/list/search
│   ├── tui.md                 # TUI 界面：Enter/a/d/r/p/[/]/Tab
│   └── edit.md                # 编辑操作：新建/重命名/移动/删除
│
├── script/                    # 脚本系统
│   ├── basics.md              # 基础命令：j script/j time countdown
│   ├── env-vars.md            # 环境变量注入：J_<别名大写>
│   └── new-window.md          # 新窗口执行：-w 标志
│
├── hook/                      # Hook 系统
│   ├── overview.md            # Hook 概念、三级来源、事件生命周期
│   ├── events.md              # 事件详解：pre_send/post_llm/pre_tool...
│   ├── types.md               # Hook 类型：bash/llm
│   ├── protocol.md            # 协议：HookContext/HookResult JSON
│   └── examples.md            # 配置示例：纠查官/消息审查/危险命令拦截
│
├── tools/                     # 工具 & 权限
│   ├── builtin.md             # 内置工具列表：Bash/Read/Write/Edit/Glob/Grep...
│   ├── agent.md               # Agent 模式：Sub-Agent/Teammate/AgentTeam
│   ├── task.md                # 任务管理：Task 工具
│   ├── permissions.md         # 权限配置：.jcli/permissions.yaml
│   └── confirm.md             # 工具确认快捷键：Y/N/Enter/Esc
│
└── command/                   # 其他命令
    ├── log.md                 # 日志设置：j log mode
    ├── config.md              # 系统配置：j config
    ├── completion.md          # Shell 补全：j completion
    ├── update.md              # 更新命令：j update
    └── tips.md                # 使用技巧：交互模式、!前缀、Tab补全
```

## 文件内容拆分规则

### alias/
从 `alias.md` 拆分：
- `basics.md`: set/rm/rename/mf 命令表格
- `categories.md`: tag/untag 命令 + browser/editor 分类说明
- `list.md`: ls/contain 命令表格 + 打开方式（j <alias>/j <browser> <url>）

### ai/
从 `chat.md` 拆分：
- `basics.md`: j chat/j ai/oneshot/--remote/-c/--session
- `config.md`: Ctrl+E、配置文件结构、各Tab操作（Model/Session/Global/Tools/Skills/Hooks/Commands/Teammates/Archive）
- `commands.md`: 斜杠命令表格（/copy/log/browse/config/model/archive/clear/theme/resume/dump/teammate）
- `shortcuts.md`: 快捷键表格（Enter/Shift+Enter/↑↓/PageUp/PageDown/Ctrl+Y/Ctrl+B/Ctrl+G/Ctrl+M/Ctrl+O/Ctrl+E/F1/Esc/Ctrl+C）
- `at-reference.md`: @ 引用系统表格 + 补全弹窗操作
- `browse.md`: Ctrl+B 消息浏览模式快捷键表格
- `archive.md`: /archive 归档流程 + /resume 恢复流程
- `teammate.md`: Teammate 概念 + 消息可见性表格
- `remote.md`: --remote 参数说明 + 二维码扫码
- `theme.md`: /theme 命令 + 主题列表表格

### report/
从 `daily.md` 拆分：
- `basics.md`: j report/j check/j search 命令表格
- `ctl.md`: reportctl new/sync/push/pull/set-url/open 命令表格
- `path.md`: 默认路径、自定义路径、远程仓库配置

### todo/
从 `daily.md` 拆分：
- `basics.md`: j todo/j td add/list 命令表格
- `tui.md`: TUI 快捷键表格（n/N/空格/a/e/d/y/f/J/K/s//?/q）
- `report-link.md`: 标记完成时写入日报的交互流程

### md/
从 `note.md` 拆分：
- `basics.md`: j md/j nb 基础命令表格
- `tui.md`: TUI 快捷键表格（↓↑jk/Enter/a/d/r/p/[/]/Tab/Esc/?）
- `edit.md`: 命令面板操作（search/rename/delete/mkdir/mv/open/ratio）

### script/
从 `script.md` 拆分：
- `basics.md`: j script/j time countdown 命令表格
- `env-vars.md`: 环境变量注入规则 + macOS/Linux/Windows 示例
- `new-window.md`: -w 标志说明

### hook/
从 `hook.md` 拆分：
- `overview.md`: Hook 概念、三级来源、事件生命周期流程图
- `events.md`: 六类事件表格（会话级/消息发送/LLM请求/工具执行/回复结束/压缩）
- `types.md`: bash/llm 类型说明 + 参数表格
- `protocol.md`: HookContext/HookResult JSON 字段表格 + action 语义
- `examples.md`: 5个配置示例（纠查官/消息审查/时间戳注入/危险命令拦截/带过滤器审查）

### tools/
从 `tools.md` 拆分：
- `builtin.md`: 内置工具表格（Bash/PowerShell/Read/Write/Edit/Glob/Grep/Ask/WebFetch/WebSearch/Browser/ComputerUse/TaskOutput/LoadSkill/Compact/Task/RegisterHook/Agent/AgentTeam/TodoWrite/TodoRead/EnterPlanMode/EnterWorktree）
- `agent.md`: Agent 模式总览表格 + Sub-Agent/Teammate/AgentTeam 详细说明
- `task.md`: Task 工具操作表格 + 任务状态/依赖/持久化说明
- `permissions.md`: .jcli/permissions.yaml 配置示例 + allow/deny 规则
- `confirm.md`: 工具确认快捷键表格

### command/
从 `install.md` 的"系统设置"和"使用技巧"部分提取：
- `log.md`: j log mode
- `config.md`: j config 命令
- `completion.md`: j completion + eval "$(j completion zsh)"
- `update.md`: j update/j update --check
- `tips.md`: 交互模式、!前缀、shell模式、Tab补全、平台差异

## 实施步骤

1. 创建新目录结构
2. 拆分现有文档到新文件
3. 删除旧文档
4. 测试 `j help` 显示效果

## 文件数量估算

| 目录 | 文件数 |
|------|--------|
| 根目录 | 3 |
| alias/ | 3 |
| ai/ | 10 |
| report/ | 3 |
| todo/ | 3 |
| md/ | 3 |
| script/ | 3 |
| hook/ | 5 |
| tools/ | 5 |
| command/ | 5 |
| **总计** | **37** |

## 预期效果

- 用户可以从目录名直接定位主题（如 `ai/commands.md` 查找斜杠命令）
- 每个文件内容精简（通常 30-50 行），易于快速阅读
- 树形目录在 Help TUI 左侧清晰展示层级关系
- 新增主题时只需在对应目录添加文件，无需修改现有结构