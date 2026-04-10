# Plan: Teammate 系统 — 多 Agent 聊天室协作

## 概述

在 j-cli 的 TUI 聊天界面中实现 **Teammate** 机制，让多个 Agent 像在聊天室中一样协作。每个 Teammate 是一个独立的 agent loop 实例（独立 LLM 连接、独立消息上下文），通过广播消息 + @提及 进行通信。

**参考**: Claude Code 的 swarm 系统（`src/utils/swarm/`, `src/utils/teammate*.ts`, `src/tools/SendMessageTool/`）

**核心原则**:
- Teammate 不是 tool 的产物，而是聊天基础设施
- 消息通过广播分发，@ 指定接收者
- 每个 Teammate 有独立的 agent loop 实例级隔离
- 广播消息像用户 append 消息一样被各 agent loop drain 进去
- 文件编辑需要互斥锁，同一时刻只允许一个 Agent 编辑一个文件

---

## 一、架构设计

### 1.1 消息流模型

```
┌─────────── TUI 聊天界面（统一渲染）──────────┐
│ [User] 帮我创建 React + Express Todo 应用     │
│ <Main> 正在组建团队...                        │
│ <Frontend> 我来负责 React 前端                 │
│ <Backend> 我来负责 Express 后端                │
│ <Frontend> @Backend API 端点设计好了吗？       │
│ <Backend> @Frontend 已完成，详见 routes/api.js │
│ <Frontend> @Main 前端部分已完成                │
│ <Main> 全部完成！                              │
└──────────────────────────────────────────────┘
```

### 1.2 实例隔离模型

```
ChatApp
├── Main Agent Loop (主 agent，始终存在)
│   ├── provider + messages + tools (独立上下文)
│   ├── streaming_content (独立)
│   └── pending_user_messages ← 广播消息 + 用户消息
│
├── TeammateManager
│   ├── teammates: HashMap<String, TeammateHandle>
│   ├── broadcast(from, text) → 注入到所有其他 agent 的 pending 队列
│   └── file_locks: HashMap<PathBuf, String>  // path → agent_name
│
├── Teammate "Frontend" (session 级别)
│   ├── 独立的 agent loop thread
│   ├── 独立的 provider + messages + tools
│   ├── 独立的 streaming_content
│   └── pending_user_messages ← 广播消息
│
└── Teammate "Backend" (session 级别)
    ├── 独立的 agent loop thread
    ├── 独立的 provider + messages + tools
    ├── 独立的 streaming_content
    └── pending_user_messages ← 广播消息
```

### 1.3 SendMessage 工具

所有 agent（包括 Main）通过 `SendMessage` 工具发送消息：
- 消息以 `<AgentName>` 尖括号前缀注入到目标 agent 的 `pending_user_messages`
- 以 `user` 角色注入（和用户 append 消息走同一个 drain 机制）
- 广播时注入到**所有其他** agent 的 pending 队列
- @AgentName 时消息仍然广播，但前缀标明是 @谁的

### 1.4 System Prompt 增强

每个 agent 的 system prompt 增加：
- `{{.teammates}}` — 当前团队成员列表及其角色
- `{{.background_tasks}}` — 后台任务状态
- 告知 agent 自己的名字和角色，以及如何使用 SendMessage 工具

### 1.5 Teammate 生命周期

- **创建**: 用户通过斜杠命令 `/team` 或 agent 通过 `CreateTeammate` 工具创建
- **存活期**: session 级别，主 agent 始终存在
- **agent loop 结束**: 如果 teammate 的 agent loop 正常结束（Done），可以被新消息重新唤起
- **销毁**: session 结束时全部清理

---

## 二、实现步骤

### Step 1: TeammateManager — 核心基础设施

**新增文件**: `src/command/chat/teammate.rs`

```rust
/// Teammate 句柄
pub struct TeammateHandle {
    pub name: String,
    pub role: String,
    pub pending_user_messages: Arc<Mutex<Vec<ChatMessage>>>,
    pub streaming_content: Arc<Mutex<String>>,
    pub cancel_token: CancellationToken,
    pub is_running: Arc<AtomicBool>,
    // agent loop thread join handle
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
}

/// Teammate 管理器（挂在 ChatApp 上）
pub struct TeammateManager {
    pub teammates: HashMap<String, TeammateHandle>,
    /// 文件编辑互斥锁: file_path → agent_name
    pub file_locks: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl TeammateManager {
    /// 创建新 teammate 并启动其 agent loop
    pub fn create_teammate(&mut self, name, role, initial_prompt, ...) -> Result<()>

    /// 广播消息到所有其他 agent 的 pending_user_messages
    pub fn broadcast(&self, from: &str, text: &str, at_target: Option<&str>)

    /// 获取团队成员列表（供 system prompt 使用）
    pub fn team_summary(&self) -> String

    /// 尝试获取文件编辑锁
    pub fn acquire_file_lock(&self, path: &Path, agent: &str) -> bool

    /// 释放文件编辑锁
    pub fn release_file_lock(&self, path: &Path, agent: &str)

    /// 停止指定 teammate
    pub fn stop_teammate(&mut self, name: &str)

    /// 停止所有 teammates
    pub fn stop_all(&mut self)

    /// 如果 teammate 的 loop 已结束，用新 prompt 重新启动
    pub fn wake_teammate(&mut self, name: &str, prompt: &str)
}
```

**修改**: `src/command/chat/app/chat_app.rs`
- ChatApp 新增字段: `pub teammate_manager: Arc<Mutex<TeammateManager>>`
- 主 agent 的 pending_user_messages 也注册到 TeammateManager 中（名称 "Main"）

### Step 2: SendMessage 工具

**新增文件**: `src/command/chat/tools/send_message.rs`

```rust
struct SendMessageParams {
    /// 消息内容
    message: String,
    /// 可选: @某个 agent（不填则纯广播）
    to: Option<String>,
}
```

执行逻辑:
1. 获取调用者的 agent name（从工具上下文中获取）
2. 构造消息: `<FromAgent> @ToAgent message`（或 `<FromAgent> message`）
3. 调用 `TeammateManager::broadcast()` 注入到所有其他 agent 的 pending 队列
4. 消息同时写入 shared_agent_messages 以在 TUI 中显示

**修改**: `src/command/chat/tools/mod.rs`
- 新增 `pub mod send_message;`
- ToolRegistry::new 中注册 SendMessageTool

### Step 3: CreateTeammate 工具

**新增文件**: `src/command/chat/tools/create_teammate.rs`

```rust
struct CreateTeammateParams {
    /// Teammate 名称（如 "Frontend", "Backend"）
    name: String,
    /// 角色描述
    role: String,
    /// 初始任务提示
    prompt: String,
}
```

执行逻辑:
1. 通过 TeammateManager 创建新 teammate
2. 启动独立 agent loop thread
3. 返回创建结果

**修改**: `src/command/chat/tools/mod.rs`
- 新增 `pub mod create_teammate;`
- ToolRegistry::new 中注册 CreateTeammateTool

### Step 4: Teammate Agent Loop

**新增文件**: `src/command/chat/teammate_loop.rs`

基于现有 `agent.rs::run_agent_loop` 改造的 teammate 版本:
- 独立的 messages 上下文
- 使用共享的 TeammateManager 进行消息广播
- System prompt 带有 teammate 身份信息和团队成员列表
- agent loop 结束时标记 `is_running = false`（可被 wake 重启）
- 工具调用中 Edit/Write 工具前先 acquire_file_lock，完成后 release

关键差异:
- 不需要 TUI 交互式确认（和 sub-agent 一样，通过 permission 规则自动决定）
- streaming_content 输出带 `<AgentName>` 前缀
- 使用 SendMessage 工具而不是直接写 pending 队列（让消息可见）

### Step 5: 文件编辑互斥锁

**修改**: `src/command/chat/tools/file/edit.rs`
**修改**: `src/command/chat/tools/file/mod.rs`（WriteFileTool）

在 EditFileTool 和 WriteFileTool 的 `execute()` 中:
1. 调用 `teammate_manager.acquire_file_lock(path, agent_name)`
2. 如果锁被其他 agent 持有，返回错误: `"文件 {path} 正被 {other_agent} 编辑，请稍后重试"`
3. 执行完成后 `teammate_manager.release_file_lock(path, agent_name)`

使用 RAII guard 模式确保锁总是被释放。

### Step 6: TUI 渲染增强

**修改**: `src/command/chat/handler/tui_loop.rs`（或相关 UI 文件）

Teammate 的输出通过 shared_agent_messages 显示在 TUI 中:
- 方案: TeammateManager 维护一个共享的 `chatroom_messages: Arc<Mutex<Vec<ChatroomMessage>>>`
- ChatroomMessage: `{ agent_name: String, text: String, timestamp: u64 }`
- TUI poll 时检查新消息，渲染为 `<AgentName> text`

**修改**: `src/command/chat/app/types.rs`
- StreamMsg 新增变体: `TeammateMessage { agent_name: String, text: String }`
- 或者：teammate 的 streaming 输出统一写入主 agent 的 shared_agent_messages

### Step 7: System Prompt 增强

**修改**: `src/command/chat/app/chat_app.rs` (system_prompt_fn)

增加 `{{.teammates}}` 占位符替换:
```
## Teammates
你当前在一个多 Agent 协作环境中。
你的名字是: {{.agent_name}}
团队成员:
- Main (主协调者)
- Frontend (React 前端开发)
- Backend (Express 后端开发)

使用 SendMessage 工具向其他 agent 发送消息。使用 @AgentName 指定接收者。
```

**修改**: 默认 system prompt 模板增加 `{{.teammates}}` 和 `{{.agent_name}}`

### Step 8: 处理旧的 AgentTeam / Agent 工具

**修改**: `src/command/chat/tools/agent_team.rs`
- 删除或重构: AgentTeamTool 的职责被 CreateTeammate + SendMessage 取代
- 可以保留简化版作为 "快速批量创建多个 teammate" 的便捷工具

**Agent 工具不需要改动**: Agent tool 是 sub-agent（一次性任务），Teammate 是持续存在的协作伙伴，两者共存。

---

## 三、关键文件清单

| 操作 | 文件路径 | 说明 |
|------|----------|------|
| 新增 | `src/command/chat/teammate.rs` | TeammateManager + TeammateHandle |
| 新增 | `src/command/chat/teammate_loop.rs` | Teammate 专用 agent loop |
| 新增 | `src/command/chat/tools/send_message.rs` | SendMessage 工具 |
| 新增 | `src/command/chat/tools/create_teammate.rs` | CreateTeammate 工具 |
| 修改 | `src/command/chat/tools/mod.rs` | 注册新工具模块 |
| 修改 | `src/command/chat/app/chat_app.rs` | ChatApp 增加 teammate_manager 字段 |
| 修改 | `src/command/chat/app/types.rs` | StreamMsg 增加 TeammateMessage |
| 修改 | `src/command/chat/tools/file/edit.rs` | 文件编辑互斥锁 |
| 修改 | `src/command/chat/tools/file/mod.rs` | Write 工具互斥锁 |
| 修改 | `src/command/chat/compact.rs` | 更新 import（如有需要） |
| 修改 | system prompt 模板 | 增加 teammates/agent_name 占位符 |
| 可选删除 | `src/command/chat/tools/agent_team.rs` | 被新系统取代 |

---

## 四、复用已有代码

| 已有机制 | 复用方式 |
|----------|----------|
| `pending_user_messages` + `drain_pending` | 广播消息注入机制完全复用 |
| `AgentHandle::spawn` | Teammate loop 参考此模式启动 |
| `run_agent_loop` | Teammate loop 基于此改造 |
| `ToolRegistry::new` | 每个 Teammate 创建独立的 ToolRegistry |
| `JcliConfig` + permission 系统 | Teammate 复用主 agent 的权限配置 |
| `BackgroundManager` | 共享后台任务管理器 |
| `TaskManager` | 共享任务板 |
| `shared_agent_messages` | TUI 消息渲染机制 |

---

## 五、实施计划

### Phase 1: 基础设施 (核心)
1. [x] TeammateManager 结构 + 全局文件锁机制 (`teammate.rs`)
2. [x] SendMessage 工具 (`tools/send_message.rs`)
3. [x] CreateTeammate 工具 (`tools/create_teammate.rs`)
4. [x] Teammate agent loop (`teammate_loop.rs`，基于 run_agent_loop 改造)

### Phase 2: 集成
5. [x] ChatApp 集成 TeammateManager + 注册新工具
6. [x] TUI 渲染 teammate 消息（通过 shared_agent_messages）
7. [x] System prompt 增强（`{{.teammates}}` 占位符）
8. [x] Edit/Write 工具增加全局文件编辑互斥锁

### Phase 3: 打磨
9. [x] AgentTeamTool 重写为 CreateTeammate 的批量封装
10. [x] Teammate 持续轮询机制（idle 时不退出，长轮询等待新消息，~2分钟超时）
11. [x] 全栈开发 skill 模板 (`assets/skills/fullstack-team/SKILL.md`)

---

## 六、验证

1. `cargo build` — 编译通过
2. `cargo clippy -- -D warnings` — 无 lint
3. `cargo test` — 现有测试通过
4. 手动测试:
   - `j chat` → agent 调用 CreateTeammate 创建 Frontend/Backend
   - 验证消息在 TUI 中带 `<AgentName>` 前缀显示
   - 验证 @提及 消息正确路由
   - 验证两个 agent 不能同时编辑同一文件
   - 验证 session 结束时 teammates 被清理
