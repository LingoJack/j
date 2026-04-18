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
| `Agent` | 启动子 Agent 自主处理多步骤任务 | |
| `AgentTeam` | 批量创建多个 Teammate 并行协作 | |
| `TodoWrite` | 创建/更新结构化待办列表 | |
| `TodoRead` | 读取当前待办列表 | |
| `EnterPlanMode` / `ExitPlanMode` | 进入/退出计划模式 | |
| `EnterWorktree` / `ExitWorktree` | 创建/退出 git worktree | Yes |

**`WebFetch` 参数**：`url`（必需）、`extract_mode`（markdown/text）、`max_chars`、`authorization`、`headers`

**`WebSearch` 参数**：`query`（必需）、`count`（默认5）、`type`（auto/keyword/neural）

**`Browser` action**：`start` `stop` `status` `tabs` `open` `navigate` `screenshot`(CDP) `snapshot` `content` `close` `click`(CDP) `type`(CDP) `press`(CDP) `evaluate`(CDP)

**`ComputerUse` action**：`screenshot` `click` `doubleclick` `rightclick` `type` `key` `key_combo` `scroll` `drag` `ax_tree` `find_element` `focus_app` `cursor_position`

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
