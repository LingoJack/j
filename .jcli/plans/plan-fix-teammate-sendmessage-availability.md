# Fix: Teammate 的 SendMessage 工具不可用 & 消息互相不可见

## 问题根因

### 问题 1: Teammate 没有 SendMessage 工具

在 `teammate_tool.rs:203` 中，工具列表是**一次性快照**的：

```rust
let tools = child_registry.to_llm_tools_filtered(&disabled);
```

这发生在 teammate **注册到 TeammateManager 之前**（注册在第 331 行）。

而 `SendMessageTool::is_available()` 检查 `has_active_teammates()`，在快照时刻：
- 当前 teammate 尚未注册 → HashMap 为空 → `has_active_teammates()` 返回 `false`
- SendMessage 被过滤掉
- 之后 teammate loop 每轮 LLM 调用都用这个**不含 SendMessage 的固定工具列表**

### 问题 2: Teammate 之间互相看不到消息

第一个 teammate 根本没有 SendMessage 工具，无法发出消息。即使后续 teammate 可能因为第一个已注册而获得 SendMessage，第一个 teammate 永远无法回应或主动通信。

### 对比 Main Agent

Main Agent 的 agent loop（`agent_loop.rs:150`）是**每轮动态调用** `to_llm_tools_filtered`，所以创建 teammate 后下一轮 Main Agent 就能看到 SendMessage。

## 修复方案

**方案：Teammate loop 也改为每轮动态生成工具列表**（与 Main Agent 对齐）

### 改动 1: `TeammateLoopConfig` 传入 `disabled` 列表而非预生成 `tools`

- 文件：`src/command/chat/tools/teammate_tool.rs`
- 修改 `TeammateLoopConfig`：将 `tools: Vec<ToolDefinition>` 改为 `disabled_tools: Vec<String>`
- 删除预生成 `tools` 的代码（第 200-203 行），改为传入 `disabled`

### 改动 2: `run_teammate_loop` 每轮动态调用 `to_llm_tools_filtered`

- 文件：`src/command/chat/teammate/teammate_loop.rs`
- 在 for 循环每轮开始时，通过 `registry.to_llm_tools_filtered(&disabled_tools)` 动态生成工具列表
- 这样 `is_available()` 在每轮检查时，teammate 已注册，`has_active_teammates()` 返回 true

### 改动 3: 清理不再需要的 `tools` 字段

- 从 `TeammateLoopConfig` 移除 `tools` 字段
- 新增 `disabled_tools` 字段

## 具体代码变更

### teammate_loop.rs

```rust
// TeammateLoopConfig 修改
pub struct TeammateLoopConfig {
    // 删除: pub tools: Vec<ToolDefinition>,
    // 新增:
    pub disabled_tools: Vec<String>,
    pub registry: Arc<ToolRegistry>,
    // ... 其余不变
}
```

```rust
// run_teammate_loop 内 for 循环开头，每轮动态生成 tools
for round in 0..MAX_TEAMMATE_ROUNDS {
    // 每轮开始时动态获取可用工具（与 Main Agent 对齐）
    let tools = registry.to_llm_tools_filtered(&disabled_tools);
    // ... 后续使用 &tools 传给 call_llm_non_stream
}
```

### teammate_tool.rs

```rust
// 删除第 200-203 行:
// let mut disabled = ...;
// disabled.push(...);
// disabled.push(...);
// let tools = child_registry.to_llm_tools_filtered(&disabled);

// 改为：
let mut disabled_tools = self.shared.disabled_tools.as_ref().clone();
disabled_tools.push(Self::NAME.to_string());
disabled_tools.push(SubAgentTool::NAME.to_string());

// TeammateLoopConfig 中:
// 删除: tools,
// 新增: disabled_tools,
```

## 性能影响

`to_llm_tools_filtered` 每轮调用一次，遍历 HashMap 中约 20 个工具，开销可忽略不计（< 1μs）。
