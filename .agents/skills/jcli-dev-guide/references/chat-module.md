# Chat 模块详细说明

## 工具系统 (`tools/`)

### 注册的内置工具列表

| 工具名 | 文件 | 说明 |
|--------|------|------|
| `Bash` | `tools/shell.rs` | 执行 shell 命令，需用户确认 |
| `Read` | `tools/file/read.rs` | 读取文件内容 |
| `Write` | `tools/file/write.rs` | 写入文件 |
| `Edit` | `tools/file/edit.rs` | 精确字符串替换 |
| `Glob` | `tools/file/glob.rs` | 文件模式匹配 |
| `Grep` | `tools/grep.rs` | 内容搜索 |
| `WebFetch` | `tools/web_fetch.rs` | HTTP 获取网页 |
| `WebSearch` | `tools/web_search.rs` | 网络搜索 |
| `Browser` | `tools/browser.rs` | 浏览器自动化（CDP/Lite 双模式）|
| `AskUser` | `tools/ask.rs` | 询问用户（阻塞等待输入）|
| `TaskOutput` | `tools/background.rs` | 获取后台任务输出 |
| `Task` | `tools/task/task_tool.rs` | 任务管理（create/list/get/update）|
| `TodoWrite` | `tools/todo/todo_write_tool.rs` | 写入 Todo |
| `TodoRead` | `tools/todo/todo_read_tool.rs` | 读取 Todo |
| `Compact` | `tools/compact.rs` | 手动压缩 context |
| `RegisterHook` | `tools/hook.rs` | 注册 session 级别 Hook |
| `ComputerUse` | `tools/computer_use/` | macOS 屏幕/键鼠控制 |
| `EnterPlanMode` | `tools/plan.rs` | 进入计划模式 |
| `ExitPlanMode` | `tools/plan.rs` | 退出计划模式 |
| `EnterWorktree` | `tools/worktree.rs` | 进入 git worktree 隔离 |
| `ExitWorktree` | `tools/worktree.rs` | 退出 worktree |
| `LoadSkill` | `tools/skill.rs` | 加载 skill（有 skill 时自动注册）|

### 添加新工具的步骤

1. 在 `src/command/chat/tools/` 创建新文件（或加入已有文件）
2. 实现 `Tool` trait
3. 在 `tools/mod.rs` 中 `pub mod` 声明
4. 在 `ToolRegistry::new()` 中 `tools: vec![...]` 添加 `Box::new(MyTool)`
5. 如需用户确认，重写 `requires_confirmation()` 返回 `true`

### ToolResult 结构

```rust
pub struct ToolResult {
    pub output: String,    // 返回给 LLM 的文本
    pub is_error: bool,    // 是否错误
    pub images: Vec<ImageData>, // 多模态图片（可选）
}
```

## Hook 系统 (`hook.rs`)

### 事件类型与触发时机

| 事件 | 触发时机 | 可修改内容 |
|------|----------|-----------|
| `pre_send_message` | 用户消息入队前 | `user_input`、`abort` |
| `post_send_message` | 用户消息入队后 | 仅通知 |
| `pre_llm_request` | LLM API 请求前 | `messages`、`system_prompt`、`inject_messages`、`abort` |
| `post_llm_response` | LLM 回复完成后 | `assistant_output` |
| `pre_tool_execution` | 工具执行前 | `tool_arguments`、`abort` |
| `post_tool_execution` | 工具执行后 | `tool_result` |
| `session_start` | 会话启动时 | 仅通知 |
| `session_end` | 会话退出时 | 仅通知 |

### Hook 层级（优先级从低到高）

1. **User 级**：`~/.jdata/agent/hooks/` — 全局生效
2. **Project 级**：`.jcli/hooks/` — 项目目录生效
3. **Session 级**：`RegisterHook` 工具动态注册 — 当前会话生效

### Hook 格式（YAML）

```yaml
# .jcli/hooks/my_hook.yaml
event: pre_tool_execution
command: "echo '{tool_name} {tool_arguments}' >> /tmp/audit.log"
```

## Permission 系统 (`permission.rs`)

配置文件：`.jcli/permissions.yaml`（从 cwd 向上查找）

```yaml
allow_all: false          # true 则跳过所有工具确认
allow:
  - "Bash(git *)"         # 允许 git 命令不需确认
  - "Read(*)"             # 所有读文件操作不需确认
deny:
  - "Bash(rm -rf *)"      # 拒绝危险删除命令
```

规则匹配格式：`ToolName(argument_pattern)`，支持 glob 通配符。

## Agent 循环 (`agent.rs`)

```
run_agent_loop()
  └─ 循环直到：no tool calls / 达到 max_tool_rounds / cancel
       ├─ build_request_with_tools() → 构建 API 请求
       ├─ call_openai_stream() → 流式接收回复
       ├─ 如有 tool_calls：
       │    ├─ 检查 plan_mode（限制可用工具）
       │    ├─ 触发 PreToolExecution hook
       │    ├─ 发送确认请求到 TUI（requires_confirmation = true 时）
       │    ├─ tool_registry.execute() → 执行工具
       │    └─ 触发 PostToolExecution hook
       └─ compact 检查（micro/auto/Compact tool 三层）
```

## Context 压缩 (`compact.rs`)

三层压缩策略（按触发顺序）：

1. **micro_compact**：超过阈值时，删除最早的工具调用结果（保留摘要）
2. **auto_compact**：token 数接近模型上限时，调用 LLM 自动总结历史
3. **Compact tool**：LLM 主动调用，手动触发压缩

## Storage (`storage.rs`)

- 聊天历史：`~/.jdata/agent/sessions/<session_id>.json`
- Agent 配置：`~/.jdata/agent/data/agent_config.json`
- 配置字段：model、provider（baseUrl、apiKey）、systemPrompt、maxToolRounds 等
