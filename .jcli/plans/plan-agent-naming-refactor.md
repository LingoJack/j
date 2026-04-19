# Agent 命名语义修正计划

## 背景

`src/command/chat` 模块中存在三种运行 LLM 的实体（Main Agent、SubAgent、Teammate），
但多处变量名、模块名、类型名与实际语义不贴切，导致代码可读性差、新成员理解成本高。
本计划旨在**仅修改命名**（变量名/类型名/文件名/注释），不改变任何运行时行为。

---

## 核心命名思路：派生 Agent（Derived Agent）

SubAgent 和 Teammate 的共性是：**它们都由 Main Agent 派生（spawn）出来**，因此：
- `ChildAgentShared` -> `DerivedAgentShared`（派生 Agent 共享上下文）
- `HeadlessRetryPolicy` -> `DerivedRetryPolicy`（派生 Agent 重试策略）
- `headless_retry_policy()` -> `derived_retry_policy()`
- "Headless Loop" -> "Derived Agent Loop"

---

## 修改项总览

| # | 当前命名 | 问题 | 修改为 | 影响范围 |
|---|---------|------|--------|---------|
| 1 | `tools/agent.rs` | 与 `agent/` 模块同名歧义 | `tools/sub_agent.rs` | 文件重命名 + mod 声明 |
| 2 | `AgentTool` | 听起来像通用 Agent 工具 | `SubAgentTool` | sub_agent.rs, tools.rs, compact.rs |
| 3 | `ChildAgentShared` | Teammate 不是 Main 的 child | `DerivedAgentShared` | agent_shared.rs + 3 处引用 |
| 4 | `HeadlessRetryPolicy` | headless 含义模糊 | `DerivedRetryPolicy` | agent_shared.rs |
| 5 | `headless_retry_policy()` | 同上 | `derived_retry_policy()` | agent_shared.rs |
| 6 | `"Headless Loop 共享 Helper"` 注释 | 同上 | `"Derived Agent Loop 共享 Helper"` | agent_shared.rs |
| 7 | `"Headless 重试策略"` 注释 | 同上 | `"Derived Agent 重试策略"` | agent_shared.rs |
| 8 | `main_agent_pending` | 语义方向模糊 | `main_agent_inbox` | manager.rs (5 处) |
| 9 | `AgentType` 缺少 `Main` | 默认值矛盾 | 添加 `Main` 变体 | queue.rs, thread_identity.rs |
| 10 | `CURRENT_AGENT_TYPE` 默认 `SubAgent` | 与 Main Agent 线程矛盾 | 默认改为 `Main` | thread_identity.rs |
| 11 | teammate_loop.rs 中 `headless` 注释 | 语义模糊 | 改为 `sub_agent_loop` | teammate_loop.rs |

---

## 详细修改步骤

### 步骤 1：重命名 `tools/agent.rs` -> `tools/sub_agent.rs`

**文件操作**：
- `git mv src/command/chat/tools/agent.rs src/command/chat/tools/sub_agent.rs`

**同步修改**：
- `src/command/chat/tools.rs`：`mod agent;` -> `mod sub_agent;`，`pub use agent::*;` -> `pub use sub_agent::*;`
- `tools.rs` 中 `super::agent::AgentTool::NAME` -> `super::sub_agent::SubAgentTool::NAME`

### 步骤 2：`AgentTool` -> `SubAgentTool`

**涉及文件**：
- `src/command/chat/tools/sub_agent.rs`（原 tools/agent.rs）：结构体名、impl 块、`const NAME`、所有内部引用
- `src/command/chat/tools.rs`：`AGENT` 常量的值路径
- `src/command/chat/agent/compact.rs`：引用 `AgentTool::NAME` 处

### 步骤 3：`ChildAgentShared` -> `DerivedAgentShared`

**涉及文件**：
- `src/command/chat/tools/agent_shared.rs`：struct 定义 + impl 块 + `new()` 方法
- `src/command/chat/tools/sub_agent.rs`：`use` 及构造调用
- `src/command/chat/tools/create_teammate.rs`：`use` 及构造调用
- `src/command/chat/tools/agent_team.rs`：`use` 及构造调用

### 步骤 4：`HeadlessRetryPolicy` -> `DerivedRetryPolicy` + 相关注释

**涉及文件**（均在 agent_shared.rs 内部）：
- struct 定义：`struct HeadlessRetryPolicy` -> `struct DerivedRetryPolicy`
- 函数：`fn headless_retry_policy()` -> `fn derived_retry_policy()`
- 所有构造调用：`HeadlessRetryPolicy { ... }` -> `DerivedRetryPolicy { ... }`
- 分隔注释：`"Headless Loop 共享 Helper"` -> `"Derived Agent Loop 共享 Helper"`
- 分隔注释：`"Headless 重试策略"` -> `"Derived Agent 重试策略"`

### 步骤 5：`main_agent_pending` -> `main_agent_inbox`

**涉及文件**（均在 `teammate/manager.rs`）：
- struct 字段声明
- `new()` 函数参数
- `new()` 函数体赋值
- `inject_broadcast_message()` 中使用处
- `new_for_recovery()` 中的空队列创建
- 注释同步更新

### 步骤 6：`AgentType` 添加 `Main` 变体 + 修正默认值

**`permission/queue.rs`**：
```rust
// 修改后
pub enum AgentType {
    /// 主 Agent（拥有 TUI，直接与用户交互）
    Main,
    /// Teammate agent（长驻协作 agent，通过广播通信）
    Teammate,
    /// SubAgent（临时子任务 agent，由 SubAgentTool 创建）
    SubAgent,
}
```

**`title()` 方法**：
```rust
pub fn title(&self) -> String {
    match &self.agent_type {
        AgentType::Main => format!(" 权限请求 [Main] "),
        AgentType::Teammate => format!(" 权限请求 [{}] ", self.name),
        AgentType::SubAgent => format!(" SubAgent 权限请求 [{}] ", self.name),
    }
}
```

> 注：`Main` 变体在当前代码中不会被使用（Main Agent 走 TUI 直接确认，不走 permission queue），
> 但添加它可以消除 thread_identity 中的默认值矛盾，并为未来扩展预留。

**`agent/thread_identity.rs`**：
```rust
// 修改后
static CURRENT_AGENT_TYPE: RefCell<AgentType> = const {
    RefCell::new(AgentType::Main)
};
```

### 步骤 7：修正 teammate_loop.rs 中的 `headless` 注释

```rust
// 修改前（约第 54 行）
/// 与 headless agent loop 的关键区别：

// 修改后
/// 与 sub_agent_loop 的关键区别：
```

---

## 不修改的部分（确认安全）

| 项目 | 原因 |
|------|------|
| `agent/` 模块名 | 虽然有歧义，但该模块是整个 chat 命令的主入口模块，改名影响面过大且 `agent_loop.rs` 内部的 `run_main_agent_loop` 已经有 `main` 前缀区分 |
| `SubAgentTracker` | 名字准确，只追踪 SubAgent，Teammate 有独立的 `TeammateManager`，无需改动 |
| `run_sub_agent_loop` / `run_main_agent_loop` / `run_teammate_loop` | 命名清晰对称，无需改动 |
| `agent_shared.rs` 文件名 | 虽然 `derived_agent_shared.rs` 可能更好，但该文件被 7 个文件引用，文件重命名影响面大，本次只改内部类型名 |

---

## 执行顺序

1. **步骤 1 + 2**（文件重命名 + AgentTool -> SubAgentTool）：捆绑执行，避免中间状态
2. **步骤 3**（ChildAgentShared -> DerivedAgentShared）
3. **步骤 4**（HeadlessRetryPolicy -> DerivedRetryPolicy + 相关注释）
4. **步骤 5**（main_agent_pending -> main_agent_inbox）
5. **步骤 6**（AgentType 添加 Main + 修正默认值）
6. **步骤 7**（注释修正）
7. `cargo check` 验证编译通过
8. `cargo clippy` 验证无告警
