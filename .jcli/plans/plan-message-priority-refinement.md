# 消息优先级细化方案

## 问题背景

当前滚动消息窗口（`context/window.rs`）的优先级层级为 5 级：

| 优先级 | ContextTier | 值 | 说明 |
|--------|------------|---|------|
| 最高 | System | 0 | 始终保留 |
| 高 | User | 1 | 兜底保留最新一条 |
| 中高 | KeyTool | 2 | Stage 2 豁免保底 |
| 中 | Assistant | 3 | 纯文字回复 |
| 低 | RegularTool | 4 | 常规执行类工具，预算紧张时优先丢弃 |

**问题**：SubAgent/Teammate 的所有消息（包括 tool call 和 tool result）都被统一归类为 `RegularTool`（优先级 4，最低），没有区分：
- SubAgent **Tool Result**（包含子代理的最终输出结果，对主 agent 决策极为重要）
- SubAgent **Tool Call**（子代理内部执行过程的工具调用，如 Bash/Read/Write，信息密度低）
- SubAgent **Assistant Text**（子代理的文本回复）

这导致：
1. SubAgent 的 Tool Result 可能与普通的 Bash/Read 工具调用同等对待，在上下文窗口紧张时被同等丢弃
2. 无法根据"来源"（主 agent vs 子 agent）做差异化保留

## 方案：将 SubAgent/Teammate 的 ToolGroup 作为 KeyTool 级别

### 核心思路

将含有 `sender_name`（来自 SubAgent/Teammate）的 ToolGroup 整体提升为 **KeyTool** 级别（优先级 2），享受 Stage 2 豁免保底，与 EnterPlanMode/Ask/LoadSkill 等关键工具同等待遇。

这意味着：
- SubAgent 的 **Tool Call + Tool Result** 整体作为一个 ToolGroup，不会被 Stage 3 配额丢弃
- 主 agent 的常规工具（Bash/Read/Write 等）保持 `RegularTool`（优先级 4），预算紧张时优先丢弃
- **无需新增 ContextTier 变体，无需调整配额比例**，改动量最小化

### ContextTier 枚举不变

```rust
pub enum ContextTier {
    System = 0,
    User = 1,
    KeyTool = 2,
    Assistant = 3,
    RegularTool = 4,
}
```

### 设计细节

#### 1. `MessageUnit::ToolGroup` 的 `priority()` 方法增加 sender_name 判断（`context/window.rs`）

当前逻辑：
```rust
MessageUnit::ToolGroup { .. } => ContextTier::RegularTool.priority(),
```

修改为：
```rust
MessageUnit::ToolGroup {
    assistant_message_index,
    ..
} => {
    // SubAgent/Teammate 的 ToolGroup → KeyTool
    if messages[*assistant_message_index].sender_name.is_some() {
        ContextTier::KeyTool.priority()
    } else {
        ContextTier::RegularTool.priority()
    }
}
```

但 `priority()` 方法当前只接收 `&self`，无法访问 `messages` 数组。需要改为接收消息列表参数，或者在 `parse_message_units` 阶段就确定优先级。

**推荐方案**：在 `MessageUnit::ToolGroup` 中增加 `is_sub_agent: bool` 字段，在 `parse_message_units` 阶段根据 `sender_name` 设置。

```rust
MessageUnit::ToolGroup {
    assistant_message_index,
    tool_result_indices,
    is_sub_agent,    // 新增
} => {
    if is_sub_agent {
        ContextTier::KeyTool.priority()
    } else {
        ContextTier::RegularTool.priority()
    }
}
```

#### 2. `has_exempt_tool` 不变

Stage 2 豁免保底已有逻辑检查 `has_exempt_tool`。SubAgent ToolGroup 虽然 `priority()` 返回 KeyTool，但它不一定包含豁免工具名（如 Ask/LoadSkill）。

**调整方案**：在 Stage 2 豁免保底中，除了检查 `has_exempt_tool`，还增加 `is_sub_agent` 判断：

```rust
// Stage 2: 豁免保底 — 最近 N 个 unit 中的 KeyTool（含 KeyTool 工具名 + SubAgent ToolGroup）
if unit.has_exempt_tool(messages, exempt_tools) || unit.is_sub_agent() {
    // 保留
}
```

或者更简洁：让 `has_exempt_tool` 也涵盖 `is_sub_agent` 的判断。

#### 3. `parse_message_units` 调整（`context/window.rs`）

在构建 `ToolGroup` 时检查 `sender_name`：

```rust
if msg.tool_calls.is_some() {
    let assistant_message_index = i;
    let is_sub_agent = msg.sender_name.is_some();  // 新增
    // ...收集 tool_result_indices...
    units.push(MessageUnit::ToolGroup {
        assistant_message_index,
        tool_result_indices,
        is_sub_agent,    // 新增
    });
}
```

#### 4. 配额不变（`constants.rs`）

无需新增配额常量。SubAgent ToolGroup 走 KeyTool 的豁免保底通道，不参与 Stage 3 比例配额分配。

## 涉及文件

| 文件 | 修改内容 |
|------|---------|
| `src/command/chat/context/window.rs` | `MessageUnit::ToolGroup` 新增 `is_sub_agent` 字段；`priority()` 方法使用该字段；`parse_message_units` 设置该字段；Stage 2 豁免保底增加 `is_sub_agent` 判断 |
| `src/command/chat/context/window.rs` (tests) | 补充 SubAgent ToolGroup 的优先级测试用例 |

## 不改动的部分

- **`context/policy.rs`**：`ContextTier` 枚举不变，`KEY_TOOL_NAMES` 不变，`policy_for()` 不变
- **`constants.rs`**：配额常量不变
- **UI 渲染层**：`render/cache.rs` 无需修改，`DisplayType` 枚举不变
- **消息写入层**：`sub_agent.rs` / `teammate_loop.rs` 无需修改，已有 `sender_name` 字段
- **micro_compact**：`compact.rs` 中的 `is_exempt_tool` 不受影响
- **`compact.rs`**：`BUILTIN_EXEMPT_TOOLS` 重导出不变

## 风险评估

- **极低风险**：不改变任何现有枚举值/配额，仅在 ToolGroup 判断中增加 sender_name 维度
- **兼容性**：老 session 中没有 `sender_name` 字段的 ToolGroup 会被当作主 agent ToolGroup（RegularTool），行为不变
- **测试覆盖**：需补充 `is_sub_agent = true` 的 ToolGroup 在 Stage 2 豁免保底中被保留的测试
