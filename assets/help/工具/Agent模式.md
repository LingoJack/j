---
name: Agent 执行模式
order: 2
parent: tools
---

## Agent 执行模式总览

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

### Teammate 使用场景

- 全栈开发（前端 + 后端 + 运维）
- 多领域并行研究
- 代码审查 + 实现同步
- 大型重构（按模块分工）
