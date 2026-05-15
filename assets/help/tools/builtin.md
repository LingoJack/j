---
name: 内置工具
order: 1
parent: tools
---

## 内置工具

### 工具列表

| 工具名 | 功能 | 需确认 |
|--------|------|--------|
| `Bash` | 执行 shell 命令；`run_in_background: true` 时后台执行并返回 task_id（仅 macOS/Linux） | Yes |
| `PowerShell` | 执行 PowerShell 命令；`run_in_background: true` 时后台执行并返回 task_id（仅 Windows） | Yes |
| `Read` | 读取本地文件（支持行号范围，可读取图片） | |
| `Write` | 写入文件（自动创建目录） | Yes |
| `Edit` | 编辑文件（精确字符串替换） | Yes |
| `Glob` | 按模式匹配搜索文件名 | |
| `Grep` | 正则搜索文件内容 | |
| `Ask` | 向用户提结构化选择题 | |
| `WebFetch` | 获取网页内容并转为 Markdown/纯文本 | |
| `WebSearch` | 使用 Exa Search API 搜索网络 | |
| `Browser` | 浏览器自动化（CDP + Lite fallback） | |
| `ComputerUse` | 控制 macOS 桌面（截图、点击、输入、滚动、AX 查询）（仅 macOS） | Yes |
| `TaskOutput` | 查询后台任务输出（`Bash run_in_background` 产生的任务），支持阻塞等待 | |
| `LoadSkill` | 加载指定技能到上下文 | |
| `Compact` | 触发对话压缩以释放上下文窗口 | |
| `Task` | 管理任务（create/get/list/update）；`action` 字段区分操作 | |
| `RegisterHook` | 注册/管理 session 级 hook | Yes |
| `Agent` | 启动子 Agent 自主处理多步骤任务（详见下方） | |
| `AgentTeam` | 批量创建多个 Teammate 并行协作（详见下方） | |
| `TodoWrite` | 创建/更新结构化待办列表 | |
| `TodoRead` | 读取当前待办列表 | |
| `EnterPlanMode` / `ExitPlanMode` | 进入/退出计划模式 | |
| `EnterWorktree` / `ExitWorktree` | 创建/退出 git worktree | Yes |

### WebFetch 参数

- `url`（必需）- 目标网页地址
- `extract_mode` - 输出格式：`markdown` 或 `text`
- `max_chars` - 最大返回字符数
- `authorization` - 授权头
- `headers` - 自定义请求头

### WebSearch 参数

- `query`（必需）- 搜索关键词
- `count` - 返回结果数量（默认 5）
- `type` - 搜索类型：`auto` / `keyword` / `neural`

### Browser action

`start` `stop` `status` `tabs` `open` `navigate` `screenshot`(CDP) `snapshot` `content` `close` `click`(CDP) `type`(CDP) `press`(CDP) `evaluate`(CDP)

> **Lite 模式**（默认）：基于 HTTP 请求，无需安装 Chrome。**CDP 模式**：需 `--features browser_cdp` 编译，支持截图、点击、输入、JS 执行。

### ComputerUse action

`screenshot` `click` `doubleclick` `rightclick` `type` `key` `key_combo` `scroll` `drag` `ax_tree` `find_element` `focus_app` `cursor_position`
