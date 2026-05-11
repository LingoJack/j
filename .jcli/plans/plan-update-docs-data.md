# 文档数据更新计划

## 分析结果

经过对比 `web/src/data/docs/` 目录下的文档与实际代码定义，发现以下过时/不一致之处：

---

### 1. 斜杠命令文档 (commands.md) - **缺少 6 个命令**

**文档中记录的命令**（6 个）:
```
/copy, /log, /browse, /config, /model, /archive
```

**实际代码定义** (`src/command/chat/input/autocomplete.rs`，共 12 个）:
```
/copy, /log, /browse, /config, /model, /archive,
/clear, /theme, /resume, /dump, /dump-processed, /teammate
```

**需要补充的命令**（6 个）:

| 命令 | 描述 |
|------|------|
| `/clear` | 新建对话 |
| `/theme` | 切换主题 |
| `/resume` | 恢复历史会话 |
| `/dump` | 导出原始 session messages（未经处理管线） |
| `/dump-processed` | 导出经完整处理管线（window → compact → hooks → sanitize）后的最终数据 |
| `/teammate` | Teammate 面板 |

---

### 2. 工具文档 (tools.md) - **多处参数缺失 + 缺少工具**

#### 2.1 Edit 工具参数缺失

**文档中的参数**: `path`, `old_string`, `new_string`

**实际参数** (`j-agent/src/tools/file/edit.rs`):
- `path` (string, 必填)
- `old_string` (string, 必填)
- `new_string` (string, 可选)
- `replace_all` (bool, 可选) - **文档缺失**
- `confirm_token` (string, 可选) - **文档缺失**

#### 2.2 Glob 工具参数缺失

**文档中的参数**: `pattern`, `path`, `excludePattern`, `limit`

**实际参数** (`j-agent/src/tools/file/glob.rs`):
- `pattern`, `path`, `excludePattern`, `limit` - 文档已有
- `offset` (uint, 可选) - **文档缺失**

#### 2.3 Bash 工具参数缺失

**文档中的参数**: `command`, `cwd`, `timeout`, `run_in_background`

**实际参数** (`j-agent/src/tools/shell.rs`):
- `command` (string, 必填)
- `description` (string, 可选) - **文档缺失**
- `cwd` (string, 可选)
- `timeout` (uint, 可选)
- `run_in_background` (bool, 可选)
- `interactive` (bool, 可选) - **文档缺失**

#### 2.4 Agent 工具参数缺失

**文档中的参数**: `prompt`, `description`, `run_in_background`

**实际参数** (`src/command/chat/tools/sub_agent.rs`):
- `prompt` (string, 必填)
- `description` (string, 可选)
- `run_in_background` (bool, 可选)
- `worktree` (bool, 可选) - **文档缺失**
- `inherit_permissions` (bool, 可选) - **文档缺失**

#### 2.5 缺失的工具

以下工具在文档中**完全没有记录**:

| 工具 | 描述 |
|------|------|
| **Session** | 操作交互进程会话的 stdin/stdout/quit |
| **Teammate** | 创建独立运行的队友 Agent |
| **SendMessage** | 向队友 Agent 发送消息 |
| **IgnoreMessage** | 忽略队友 Agent 的消息 |
| **LoadTool** | 加载延迟加载的工具使其可用 |
| **EnterWorktree** | 创建隔离的 git worktree |
| **ExitWorktree** | 退出 git worktree |

---

### 3. 技能文档 - **LoadSkill 技能列表不完整**

tools.md 中 LoadSkill 部分列出的技能（仅 3 个）:
- j-cli, skill-creator, swift-ios-app-gen

实际技能列表（11 个）:
- chat-module-guide, fullstack-team, html-ppt, hyperframes, j-cli, jcli-dev-guide, skill-creator, sql-to-go-struct-and-dao, swift-ios-app-gen, ui-ux-pro-max, webapp-gen

**需要补充 8 个技能**

---

## 执行计划

### 步骤 1: 更新 commands.md（中英文）
- 补充 6 个缺失的斜杠命令（`/clear`, `/theme`, `/resume`, `/dump`, `/dump-processed`, `/teammate`）
- 使用代码中已有的描述文本

### 步骤 2: 更新 tools.md（中英文）
- Edit: 添加 `replace_all` 和 `confirm_token` 参数
- Glob: 添加 `offset` 参数
- Bash: 添加 `description` 和 `interactive` 参数
- Agent: 添加 `worktree` 和 `inherit_permissions` 参数
- 新增工具章节：Session, Teammate, SendMessage, IgnoreMessage, LoadTool, EnterWorktree, ExitWorktree

### 步骤 3: 更新 skills.md（中英文）
- tools.md 中 LoadSkill 部分的技能列表需要补充 8 个缺失技能
- skills.md 本身主要是结构说明，无需修改

## 文件修改清单

| 文件 | 修改类型 |
|------|---------|
| `web/src/data/docs/zh/commands.md` | 补充命令 |
| `web/src/data/docs/en/commands.md` | 补充命令 |
| `web/src/data/docs/zh/tools.md` | 补充参数 + 新增工具 |
| `web/src/data/docs/en/tools.md` | 补充参数 + 新增工具 |

## 注意事项

1. 保持现有文档格式风格（表格 + JSON 示例）
2. 中英文版本同步更新
3. 描述直接取自代码中的定义，确保准确
4. 参数类型使用一致的命名（string/uint/bool/array/object）
