# Plan: ExitPlanMode Tool Result 返回计划文件内容

## 背景

当前 ExitPlanMode 工具返回的 `ToolResult.output` 只包含简短的确认消息（如 "Plan approved! Exited plan mode..."），而用户希望 tool result 直接包含计划文件的完整内容。

这样做的好处：
1. **micro_compact 豁免机制**：`ExitPlanModeTool` 已在 `BUILTIN_EXEMPT_TOOLS` 豁免列表中，其 tool result 不会被 micro_compact 压缩
2. **增强计划遵循能力**：LLM 在后续对话中能继续看到完整的计划内容，即使发生 micro_compact
3. **上下文连续性**：计划内容作为 tool result 始终存在于上下文中

## 实施步骤

### Step 1: 修改 ExitPlanModeTool::execute (Teammate 路径)

**文件**: `src/command/chat/tools/plan.rs`

在 teammate 路径的审批处理中（约 470-513 行），修改 `ToolResult.output`：

- `PlanDecision::Approve`: output 应包含完整计划内容，而非简单确认消息
- `PlanDecision::ApproveAndClearContext`: output 同样包含完整计划内容（agent_loop 会用这个值注入新消息）
- `PlanDecision::Reject`: 保持原有拒绝消息

**修改前**:
```rust
PlanDecision::Approve => {
    // ...
    ToolResult {
        output: format!(
            "Plan approved! Exited plan mode. You can now proceed with implementation.{}",
            preserved_msg
        ),
        // ...
    }
}
```

**修改后**:
```rust
PlanDecision::Approve => {
    // ...
    ToolResult {
        output: format!(
            "Plan approved! Exited plan mode. You can now proceed with implementation.{}\n\n**Plan Content:**\n\n{}",
            preserved_msg,
            plan_content  // 计划文件完整内容
        ),
        // ...
    }
}
```

### Step 2: 修改 ExitPlanModeTool::execute_via_ask_tx (主 Agent 路径)

**文件**: `src/command/chat/tools/plan.rs`

在主 agent 路径的审批处理中（约 584-638 行），同样修改 `ToolResult.output`：

- "批准并清空上下文": output 包含完整计划内容
- "批准": output 包含完整计划内容
- 驳回: 保持原有拒绝消息

### Step 3: 确保 plan_content 变量可用

检查 `execute` 方法中 `plan_content` 变量的作用域，确保在所有返回路径中都能访问到。

当前代码在 `execute` 开头就读取了 `plan_content`（约 431-451 行），所以这个变量在整个方法中都可用。

## 验证要点

1. 编译通过: `cargo check`
2. 格式化: `cargo fmt`
3. Clippy 检查: `cargo clippy`
4. 验证三种决策路径的 output 都正确包含计划内容

## 影响范围

- 仅影响 `src/command/chat/tools/plan.rs` 文件
- 仅修改 `ToolResult.output` 的内容格式
- 不改变 `plan_decision` 字段，agent_loop 的清空上下文逻辑不受影响
