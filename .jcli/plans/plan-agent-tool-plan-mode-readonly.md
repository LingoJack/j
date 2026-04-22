# Plan: Agent Tool 在 Plan Mode 中可用但仅限 Read-Only 工具

## 目标

让 `Agent` 和 `AgentTeam` 工具在 plan mode 下可用，但子 agent 内部只能使用 read-only 工具。

## 背景

当前 `PLAN_MODE_WHITELIST` 不包含 `Agent` / `AgentTeam`，导致在 plan mode 下无法调用子 agent 进行代码探索。用户希望在 plan mode 中能利用子 agent 做多步研究，同时确保子 agent 不能执行写操作。

## 关键发现

1. **Plan Mode 拦截点**：`definition.rs:233-284` 的 `ToolRegistry::execute()` 方法在 plan mode active 时，通过 `is_allowed_in_plan_mode(name)` 检查白名单。
2. **子 Agent Registry 独立**：`build_child_registry()` 调用 `ToolRegistry::new()` 创建全新的 registry，子 registry 的 `plan_mode_state` 是新建的（`active: false`），不继承父 agent 的 plan mode 状态。
3. **子 Agent 无 Plan Mode 限制**：由于子 registry 的 plan mode 状态独立，子 agent 调用 `registry.execute()` 时不会触发 plan mode 拦截。

## 实现方案

### Step 1: 将 `Agent` 和 `AgentTeam` 加入 Plan Mode 白名单

**文件**: `src/command/chat/tools/plan.rs`

在 `PLAN_MODE_WHITELIST` 中添加 `"Agent"` 和 `"AgentTeam"`：

```rust
pub const PLAN_MODE_WHITELIST: &[&str] = &[
    "Read",
    "Glob",
    "Grep",
    "WebFetch",
    "WebSearch",
    "Ask",
    "Compact",
    "TodoRead",
    "TodoWrite",
    "TaskOutput",
    "Task",
    "EnterPlanMode",
    "ExitPlanMode",
    "EnterWorktree",
    "ExitWorktree",
    "Agent",        // 新增：允许在 plan mode 启动子 agent 做研究
    "AgentTeam",    // 新增：允许在 plan mode 批量创建 teammate
];
```

这一步让主 agent 在 plan mode 下能调用 Agent/AgentTeam 工具。

### Step 2: 子 Agent 进入 Plan Mode（继承只读限制）

**文件**: `src/command/chat/tools/sub_agent.rs`

在构建子 registry 后，将子 registry 的 `plan_mode_state` 设置为 active：

```rust
// 构建子 registry（排除 "Agent" 工具防递归，独立 todos 文件）
let (child_registry, _) = self.shared.build_child_registry(subagent_todos_path);

// 如果父 agent 在 plan mode，子 agent 也进入 plan mode（限制为只读工具）
let parent_plan_active = self.shared.plan_mode_active.load(Ordering::Relaxed);
if parent_plan_active {
    child_registry.plan_mode_state.enter("__sub_agent_plan_mode__").ok();
}
```

但这里有个问题：`DerivedAgentShared` 目前没有 `plan_mode_active` 标记。我们需要：

### Step 3: 在 `DerivedAgentShared` 中传递 Plan Mode 状态

**文件**: `src/command/chat/tools/derived_shared.rs`

在 `DerivedAgentShared` 中添加一个 `plan_mode_active: Arc<AtomicBool>` 字段，用于向子 agent 传递 plan mode 状态：

```rust
pub struct DerivedAgentShared {
    // ... 现有字段 ...
    /// 标记父 agent 是否处于 plan mode（子 agent 据此决定是否限制工具）
    pub plan_mode_active: Arc<AtomicBool>,
}
```

### Step 4: 在 ChatApp 中初始化 `plan_mode_active`

在创建 `DerivedAgentShared` 时，需要让它和主 `ToolRegistry` 的 `plan_mode_state` 保持同步。可以在 `EnterPlanMode` / `ExitPlanMode` 执行时同步设置这个标记。

但更简单的做法是：让 `DerivedAgentShared` 持有对 `plan_mode_state` 的 `Arc` 引用，在创建子 agent 时检查是否 active。

### 简化方案（推荐）

经过重新分析，采用更简洁的实现方式：

1. **Step 1**: 在 `PLAN_MODE_WHITELIST` 中添加 `"Agent"` 和 `"AgentTeam"`
2. **Step 2**: 在 `DerivedAgentShared` 中新增 `plan_mode_active: Arc<AtomicBool>` 字段
3. **Step 3**: 在 `EnterPlanModeTool::execute` 和 `ExitPlanModeTool::execute` 中同步更新 `plan_mode_active`
4. **Step 4**: 在 `SubAgentTool::execute` 和 `CreateTeammateTool::execute`（以及 `AgentTeamTool::execute`）中，检查 `plan_mode_active`，若为 true 则让子 registry 进入 plan mode

## 涉及文件

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/command/chat/tools/plan.rs` | 修改 | `PLAN_MODE_WHITELIST` 添加 Agent、AgentTeam |
| `src/command/chat/tools/derived_shared.rs` | 修改 | `DerivedAgentShared` 新增 `plan_mode_active` 字段 |
| `src/command/chat/tools/sub_agent.rs` | 修改 | 检查 `plan_mode_active`，子 registry 进入 plan mode |
| `src/command/chat/tools/create_teammate.rs` | 修改 | 检查 `plan_mode_active`，子 registry 进入 plan mode |
| `src/command/chat/app/chat_app.rs` | 修改 | 创建 `DerivedAgentShared` 时传入 `plan_mode_active` |

## 验证

1. 进入 plan mode 后调用 Agent 工具：应成功启动子 agent
2. 子 agent 尝试调用 Write/Edit/Bash：应被拒绝（plan mode 限制）
3. 子 agent 调用 Read/Glob/Grep：应正常执行
4. 退出 plan mode 后子 agent 正常运行（不受限）
