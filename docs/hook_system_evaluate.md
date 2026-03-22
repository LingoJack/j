Sun Mar 22 16:57:19 CST 2026

现在我对整个架构有了充分的了解。下面是我的评估：

---

## Hook 系统评估报告

### 一、现有架构概览

```
┌─────────────────────────────────────────────────────┐
│  TUI Loop (tui_loop.rs)                             │
│    ├── ChatApp.update(Action) ── 核心调度            │
│    ├── ChatApp.send_message_internal() ── 消息发送入口│
│    └── ChatApp.poll_stream_actions() ── 事件轮询     │
├─────────────────────────────────────────────────────┤
│  Agent Loop (agent.rs)                              │
│    ├── run_agent_loop() ── 多轮工具调用循环           │
│    ├── process_tool_calls() ── 工具调用处理           │
│    └── drain_pending_user_messages() ── 增量消息注入  │
├─────────────────────────────────────────────────────┤
│  配置层                                              │
│    ├── AgentConfig (storage.rs) ── 用户级 JSON 配置  │
│    ├── .jcli (permission.rs) ── 项目级权限配置        │
│    ├── system_prompt / memory / soul ── Markdown 文件 │
│    └── Skills (skill.rs) ── 技能系统                 │
└─────────────────────────────────────────────────────┘
```

**关键数据流：**
1. 用户输入 → `send_message_internal()` → 构建 `api_messages` + `system_prompt` → `AgentHandle::spawn()` → `run_agent_loop()`
2. Agent Loop 每轮：drain 用户消息 → compact → 构建请求 → 调用 LLM → 处理 tool_calls → 循环
3. 工具调用：Agent → `StreamMsg::ToolCallRequest` → TUI 确认/执行 → `ToolResultMsg` → 回传 Agent

### 二、Hook 插入点分析

根据 Claude Code 的 hook 设计理念和 jcli 的架构，我识别出以下关键 hook 点：

| Hook 事件 | 触发时机 | 可操作的数据 | 对应代码位置 |
|---|---|---|---|
| `PreSendMessage` | 用户发送消息前 | messages, system_prompt, model | `send_message_internal()` 开头 |
| `PostSendMessage` | 用户消息已入队后 | messages (只读) | `send_message_internal()` 中间 |
| `PreLLMRequest` | 每轮 LLM 请求前 | messages, system_prompt, tools, model | `run_agent_loop()` 构建 request 前 |
| `PostLLMResponse` | LLM 返回后 | assistant_text, tool_calls, messages | 流式/非流式收到完整响应后 |
| `PreToolExecution` | 工具执行前 | tool_name, arguments, messages | `process_tool_calls()` 发送 ToolCallRequest 前 |
| `PostToolExecution` | 工具执行后 | tool_name, result, messages | `process_tool_calls()` 收到结果后 |
| `SessionStart` | 会话启动时 | config, system_prompt | `ChatApp::new()` |
| `SessionEnd` | 会话退出时 | messages, session | `tui_loop` 中 break 后 |
| `PreCompact` | context 压缩前 | messages | `compact::auto_compact` 调用前 |

### 三、三级作用域设计

```yaml
# 1. 用户级：~/.jdata/agent/hooks/
#    全局生效，所有项目所有 session
hooks:
  pre_send_message:
    - script: "~/.jdata/agent/hooks/inject_context.sh"

# 2. 项目级：.jcli 文件扩展
#    对当前项目生效（已有 .jcli 权限配置，自然扩展）
hooks:
  pre_llm_request:
    - script: "./scripts/add_project_context.sh"
  pre_tool_execution:
    - script: "./scripts/validate_tool.sh"

# 3. Session 级：运行时动态注册
#    通过 /hook 命令或 tool 动态添加，session 结束即失效
```

### 四、Hook 能力矩阵

| 能力 | 实现方式 | 复杂度 |
|---|---|---|
| **读取 messages** | 序列化 `Vec<ChatMessage>` 为 JSON 传入 stdin | 低 |
| **修改 messages** | Hook 输出 JSON patch / 替换后的 messages | 中 |
| **读取 system_prompt** | 环境变量 / stdin 传入 | 低 |
| **修改 system_prompt** | Hook 输出新 prompt | 低 |
| **切换 model** | Hook 输出 `{"switch_model": "gpt-4o"}` | 中 |
| **注入额外 context** | Hook 输出 `{"inject": "..."}` 追加到 messages | 低 |
| **拦截/取消操作** | Hook exit code 非 0 则中止 | 低 |
| **修改工具参数** | Hook 输出修改后的 arguments JSON | 中 |
| **过滤工具结果** | Hook 输出修改后的 result | 低 |

### 五、实现方案建议

#### 核心数据结构

```rust
// src/command/chat/hook.rs

use serde::{Deserialize, Serialize};

/// Hook 事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HookEvent {
    PreSendMessage,
    PostSendMessage,
    PreLlmRequest,
    PostLlmResponse,
    PreToolExecution,
    PostToolExecution,
    SessionStart,
    SessionEnd,
}

/// 单个 Hook 定义
#[derive(Debug, Clone, Deserialize)]
pub struct HookDef {
    /// 外部脚本路径（互斥选 script 或 inline）
    pub script: Option<String>,
    /// 内联命令（如 "echo hello"）
    pub command: Option<String>,
    /// 超时（秒），默认 10
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

/// Hook 上下文：传递给脚本的 JSON
#[derive(Debug, Serialize)]
pub struct HookContext {
    pub event: HookEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ChatMessage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_arguments: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<String>,
}

/// Hook 执行结果：脚本 stdout 的 JSON
#[derive(Debug, Deserialize, Default)]
pub struct HookResult {
    /// 替换 messages
    pub messages: Option<Vec<ChatMessage>>,
    /// 替换 system_prompt
    pub system_prompt: Option<String>,
    /// 切换模型
    pub switch_model: Option<String>,
    /// 注入额外消息
    pub inject_messages: Option<Vec<ChatMessage>>,
    /// 是否中止当前操作（exit code 非 0 也视为中止）
    #[serde(default)]
    pub abort: bool,
}

/// Hook 管理器
pub struct HookManager {
    /// 用户级 hooks（~/.jdata/agent/hooks.yaml）
    user_hooks: HashMap<HookEvent, Vec<HookDef>>,
    /// 项目级 hooks（.jcli 中的 hooks 字段）
    project_hooks: HashMap<HookEvent, Vec<HookDef>>,
    /// Session 级 hooks（运行时动态注册）
    session_hooks: HashMap<HookEvent, Vec<HookDef>>,
}
```

#### 调用协议

```
jcli → hook script:
  - stdin: HookContext JSON
  - env: JCLI_HOOK_EVENT, JCLI_MODEL, JCLI_CWD
  - exit 0: 成功，解析 stdout 为 HookResult
  - exit 非 0: 中止操作

执行顺序: user hooks → project hooks → session hooks（链式，前一个输出作为下一个输入）
```

#### 关键插桩位置

**1. `send_message_internal()` — PreSendMessage**
```rust
// app.rs 约 2129 行
pub fn send_message_internal(&mut self, text: String) {
    // ★ Hook: PreSendMessage
    let ctx = HookContext { event: PreSendMessage, messages: Some(...), ... };
    if let Some(result) = self.hook_manager.execute(PreSendMessage, ctx) {
        if result.abort { return; }
        // 应用 messages/system_prompt/model 变更
    }
    // ... 原有逻辑
}
```

**2. `run_agent_loop()` — PreLlmRequest / PostLlmResponse**
```rust
// agent.rs 约 128 行（构建 request 前）
// ★ Hook: PreLlmRequest — 可修改 messages, system_prompt, model
// agent.rs 约响应处理后
// ★ Hook: PostLlmResponse — 可修改/过滤 assistant 回复
```

**3. `process_tool_calls()` — PreToolExecution / PostToolExecution**
```rust
// agent.rs 约 487 行
// ★ Hook: PreToolExecution（每个 tool_call 前）— 可修改参数或中止
// agent.rs 约 531 行
// ★ Hook: PostToolExecution — 可修改 tool result
```

### 六、工作量评估

| 模块 | 工作内容 | 预估 |
|---|---|---|
| `hook.rs` 核心模块 | HookManager, 配置加载, 脚本执行, JSON 协议 | **2-3 天** |
| `.jcli` 扩展 | 在 JcliConfig 中加入 hooks 字段 | **0.5 天** |
| 用户级 hooks 配置 | `~/.jdata/agent/hooks.yaml` 加载 | **0.5 天** |
| Session hooks | `/hook` 命令 + 运行时注册 | **1 天** |
| agent.rs 插桩 | 6-8 个 hook 点的插入 | **1-2 天** |
| app.rs 插桩 | send_message, session 生命周期 | **1 天** |
| HookManager 线程安全 | Agent loop 在后台线程，需 Arc/async | **1 天** |
| 测试 + 示例 hooks | 单测 + 示例脚本 | **1 天** |
| **合计** | | **~7-9 天** |

### 七、技术难点与风险

1. **线程边界**：`run_agent_loop()` 在后台 tokio runtime 中运行，而 `ChatApp` 在主线程。Hook 如果要修改 model（需要重建 client），需要通过 channel 回传或在 agent loop 内部处理。

2. **性能影响**：每轮 LLM 请求前后执行外部脚本会引入延迟。建议：
   - Hook 执行默认超时 10s
   - 提供 `async: true` 选项用于 fire-and-forget 型 hook
   - 缓存无副作用 hook 的结果

3. **agent.rs 参数过多**：`run_agent_loop` 已有 12 个参数，新增 `HookManager` 会更臃肿。建议重构为 `AgentContext` 结构体。

4. **messages 所有权**：当前 `run_agent_loop` 拥有 `mut messages`，Hook 修改 messages 需要序列化/反序列化，有一定开销。对于高频 hook（如 PreLlmRequest 每轮触发），需要注意。

### 八、建议的实施路径

```
Phase 1: 基础框架（MVP）
  ├── hook.rs: HookManager + HookDef + HookContext + HookResult
  ├── 配置加载: 用户级 + 项目级（.jcli 扩展）
  ├── 2 个核心 hook 点: PreSendMessage + PreToolExecution
  └── 脚本执行 + JSON 协议

Phase 2: 完整 hook 点
  ├── 剩余 hook 点: PreLlmRequest, PostLlmResponse, PostToolExecution, Session*
  ├── agent.rs 重构: 引入 AgentContext 减少参数
  └── Session 级 hooks: /hook 命令

Phase 3: 增强功能
  ├── Hook 链式执行 + 优先级
  ├── async hooks
  ├── 内置 hook 模板/marketplace
  └── Hook 调试工具（/hook-debug）
```

---

**总结**：jcli 现有架构的模块化程度较高（storage/app/agent/tools 清晰分离），配置系统已有用户级（`~/.jdata/agent/`）和项目级（`.jcli`）的分层，Skill 系统也提供了扩展先例。Hook 系统可以很自然地融入现有架构，最大的工作量在于 **agent.rs 的插桩和线程安全处理**。建议从 Phase 1 的 MVP 开始，先支持最高价值的 2-3 个 hook 点，验证协议设计后再全面铺开。