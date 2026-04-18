---
name: AI 工具
order: 6
---

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
| `ComputerUse` | 控制 macOS 桌面（截图、点击、输入、滚动、AX 查询） | Yes |
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

**`WebFetch` 参数**：`url`（必需）、`extract_mode`（markdown/text）、`max_chars`、`authorization`、`headers`

**`WebSearch` 参数**：`query`（必需）、`count`（默认5）、`type`（auto/keyword/neural）

**`Browser` action**：`start` `stop` `status` `tabs` `open` `navigate` `screenshot`(CDP) `snapshot` `content` `close` `click`(CDP) `type`(CDP) `press`(CDP) `evaluate`(CDP)

**`ComputerUse` action**：`screenshot` `click` `doubleclick` `rightclick` `type` `key` `key_combo` `scroll` `drag` `ax_tree` `find_element` `focus_app` `cursor_position`

### Agent 子 Agent

启动独立子 Agent 自主执行复杂多步骤任务，完成后返回结果文本。

**参数**：

| 参数 | 必需 | 说明 |
|------|------|------|
| `prompt` | Yes | 子 Agent 的任务描述 |
| `description` | | 简短描述（3-5 词），用于状态栏显示 |
| `run_in_background` | | `true` 时后台运行，立即返回 task_id，可用 `TaskOutput` 查看结果 |
| `worktree` | | `true` 时创建隔离 git worktree（`.jcli/worktrees/`），避免并行编辑冲突 |
| `inherit_permissions` | | `true` 时继承父 Agent 所有工具权限（跳过确认） |

**特点**：
- 子 Agent 在独立线程中运行，拥有自己的 LLM 循环（最多 30 轮）
- 自动继承父 Agent 的系统提示词和模型配置
- 子 Agent 内部不可再启动 Agent（防止递归）
- 后台模式适合耗时任务，前台模式会阻塞直到完成
- worktree 模式下子 Agent 在独立分支工作，完成后自动清理

### AgentTeam 团队协作

批量创建多个 Teammate 并行协作，Teammate 之间可通过 `SendMessage` 互相通信。

**参数**：

| 参数 | 必需 | 说明 |
|------|------|------|
| `members` | Yes | Teammate 数组，每项包含 `name`、`prompt`、可选 `role` |

**每个 member 字段**：

| 字段 | 必需 | 说明 |
|------|------|------|
| `name` | Yes | Teammate 名称（如 `"frontend"`、`"backend"`） |
| `prompt` | Yes | 初始任务描述 |
| `role` | | 角色描述（如 `"React 开发者"`） |

**特点**：
- 1-10 个成员，每个在独立线程运行（最多 200 轮）
- 自动注册 `SendMessage` 和 `WorkDone` 工具供 Teammate 间通信
- Teammate 闲置约 2 分钟后自动退出
- Teammate 内部不可再创建 Teammate 或启动 Agent（防止递归）
- 每个 member 也可设置 `worktree` 和 `inherit_permissions`（由 AI 动态传入）

> **Agent vs AgentTeam**：Agent 是单个隔离子任务，无通信能力；AgentTeam 是多成员协作，成员间可互相发消息协调。

### EnterPlanMode / ExitPlanMode

进入计划模式后，只允许使用只读工具（Read、Glob、Grep、WebFetch 等），写操作被阻止。通过 `ExitPlanMode` 提交计划，等待用户审批后方可执行。

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
