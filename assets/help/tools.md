---
name: 工具 & 权限
order: 7
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

### Agent 执行模式总览

| 类型 | 工具 | 生命周期 | 最大轮次 | 消息通信 | 适用场景 |
|------|------|----------|----------|----------|----------|
| **Sub-Agent** | `Agent` | 任务完成后退出 | 30 | 无（返回结果给主对话） | 单次多步骤任务 |
| **Teammate** | `CreateTeammate` | 持久运行，空闲轮询等待消息 | 200 | `SendMessage` 广播 + `@mention` 唤醒 | 多角色长期协作 |
| **AgentTeam** | `AgentTeam` | 批量创建 Teammate | 200（每个） | 同 Teammate | 并行启动多角色 |

### Sub-Agent（`Agent` 工具）

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

### Teammate（`CreateTeammate` 工具）

持久运行的子 Agent，拥有独立上下文和消息历史，支持多角色协作。

**特点**：
- 创建方式：由 AI 通过 `CreateTeammate` 或 `AgentTeam` 工具创建
- 额外工具：`SendMessage`（广播通信）+ `WorkDone`（标记完成并退出）
- **阻塞限制**：不能使用 `Agent`、`AgentTeam`、`CreateTeammate`（防止递归创建）
- 空闲轮询：无工具调用时进入等待状态，`@mention` 或主 Agent 消息唤醒
- 120 轮连续空闲自动退出（约 2 分钟）
- 通过 `CancellationToken` 支持手动停止

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

### 消息通信（`SendMessage` 工具）

- 广播机制：消息发送给所有 Agent（主 Agent + 所有 Teammate）
- **唤醒语义**：
  - `@Target` 定向唤醒：仅被 `@` 的 Teammate 被唤醒
  - 主 Agent 发送的消息唤醒所有 Teammate
  - 旁听消息：其他 Teammate 之间的对话会进入收件箱但不唤醒，避免无限循环
- 消息格式：`<FromAgent> @Target 消息内容`

### WorkDone 工具

- Teammate 调用后标记工作完成，循环立即退出
- 广播完成消息给所有 Agent

### 全局文件锁

- 多 Agent 并发编辑同一文件时，通过全局文件锁（`acquire_global_file_lock`）自动排队，防止写入冲突

### 任务管理（`Task` 工具）

共享任务系统，所有 Agent 均可操作，持久化到 `.jcli/tasks/`：

| 操作 | 参数 | 说明 |
|------|------|------|
| `action: "create"` | `title`（必需）、`description`、`blockedBy`、`taskDocPaths` | 创建待办任务 |
| `action: "get"` | `taskId` | 获取任务详情 |
| `action: "list"` | `ready: true`（可选，仅显示无阻塞的待办任务） | 列出所有任务 |
| `action: "update"` | `taskId` + `status`/`title`/`description`/`owner`/`addBlockedBy` | 更新任务 |

- 任务状态：`pending` → `in_progress` → `completed`（或 `deleted`）
- `blockedBy`：任务依赖 DAG，前置任务完成后自动清理引用
- `owner`：负责该任务的 Agent 名称
- 任务 ID 自增，持久化为 `.jcli/tasks/task_{id}.json`

### EnterPlanMode / ExitPlanMode

进入计划模式后，只允许使用只读工具（Read、Glob、Grep、WebFetch 等），写操作被阻止。通过 `ExitPlanMode` 提交计划，等待用户审批后方可执行。

> **Lite 模式**（默认）：基于 HTTP 请求，无需安装 Chrome。**CDP 模式**：需 `--features browser_cdp` 编译，支持截图、点击、输入、JS 执行。

### Teammate 使用场景

- 全栈开发（前端 + 后端 + 运维）
- 多领域并行研究
- 代码审查 + 实现同步
- 大型重构（按模块分工）

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
