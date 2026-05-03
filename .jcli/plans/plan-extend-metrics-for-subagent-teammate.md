# 计划：扩展 SessionMetrics 统计 SubAgent 和 Teammate

## 问题分析

当前 `SessionMetrics` 只统计 Main Agent（`agent_loop.rs`）的 LLM 调用和工具调用：
- `total_llm_calls` — Main Agent 的 LLM API 调用次数
- `total_tool_calls` — Main Agent 的工具调用次数
- `total_input_tokens` / `total_output_tokens` — Main Agent 的 token 消耗
- `total_llm_elapsed_ms` / `total_tool_elapsed_ms` — Main Agent 的耗时

**缺失**：
- SubAgent (`run_sub_agent_loop`) 和 Teammate (`run_teammate_loop`) 的调用完全未被统计
- 这两个派生 Agent 使用 `call_llm_non_stream`，其返回值是 `Choice`（不含 usage 信息）

## 核心改动

### 1. 扩展 `call_llm_non_stream` 返回值

**文件**: `src/command/chat/tools/derived_shared.rs`

**改动**: 返回 `ChatResponse` 而非 `Choice`，使调用方可以获取 `usage` 信息

```rust
// 改动前
pub fn call_llm_non_stream(req: &LlmNonStreamRequest) -> Result<Choice, String>

// 改动后
pub fn call_llm_non_stream(req: &LlmNonStreamRequest) -> Result<ChatResponse, String>
```

调用方使用 `.choices.into_iter().next()` 获取第一个 choice，同时可访问 `.usage`。

### 2. 新增共享 Metrics Accumulator

**文件**: `src/command/chat/tools/derived_shared.rs`

在 `DerivedAgentShared` 中新增字段：

```rust
pub struct DerivedAgentShared {
    // ... 现有字段 ...
    
    /// 子 Agent metrics 累加器（SubAgent/Teammate 的 LLM/tool 统计）
    /// Main agent loop 结束时读取并合并到 SessionMetrics
    pub sub_agent_metrics: Arc<Mutex<SubAgentMetrics>>,
}
```

新增 `SubAgentMetrics` 结构体（或直接复用部分 `SessionMetrics` 字段）：

```rust
/// 子 Agent（SubAgent/Teammate）的 metrics 累加
#[derive(Debug, Clone, Default)]
pub struct SubAgentMetrics {
    pub total_llm_calls: u32,
    pub total_tool_calls: u32,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_llm_elapsed_ms: u64,
    pub total_tool_elapsed_ms: u64,
    pub ttft_ms_per_call: Vec<u64>,
}
```

### 3. SubAgent Loop 统计 Metrics

**文件**: `src/command/chat/tools/sub_agent.rs`

在 `run_sub_agent_loop` 中：
- 每次 `call_llm_non_stream` 成功后，累加 `total_llm_calls`、`total_input_tokens`、`total_output_tokens`
- 记录 TTFT（非流式场景为整次调用耗时）
- 每次工具调用时累加 `total_tool_calls`、`total_tool_elapsed_ms`
- Loop 结束时将 metrics 推入 `DerivedAgentShared.sub_agent_metrics`

### 4. Teammate Loop 统计 Metrics

**文件**: `src/command/chat/teammate/teammate_loop.rs`

同 SubAgent 的改动：
- 每次 `call_llm_non_stream` 成功后累加 metrics
- 工具调用时累加
- Loop 结束时推入共享 accumulator

### 5. Main Agent Loop 合并 Metrics

**文件**: `src/command/chat/agent/agent_loop.rs`

Session 结束时，在 `write_session_metrics` 之前：
- 读取 `DerivedAgentShared.sub_agent_metrics`
- 将子 Agent 的指标合并到 `SessionMetrics` 的对应字段

### 6. 更新 SESSION_METRICS.md 文档

**文件**: `SESSION_METRICS.md`

说明 metrics 统计范围已扩展到所有 Agent 类型。

## 详细改动清单

### 文件改动列表

| 文件 | 改动类型 | 改动内容 |
|---|---|---|
| `src/command/chat/tools/derived_shared.rs` | 改 + 新增 | 修改 `call_llm_non_stream` 返回 `ChatResponse`；新增 `SubAgentMetrics`；`DerivedAgentShared` 添加 `sub_agent_metrics` 字段 |
| `src/command/chat/tools/sub_agent.rs` | 改 | `run_sub_agent_loop` 统计 LLM/tool metrics，结束时推入 accumulator |
| `src/command/chat/teammate/teammate_loop.rs` | 改 | `run_teammate_loop` 统计 LLM/tool metrics，结束时推入 accumulator |
| `src/command/chat/tools/teammate.rs` | 改 | `TeammateTool` 构造 `DerivedAgentShared` 时初始化 `sub_agent_metrics` |
| `src/command/chat/tools/mod.rs` | 可能改 | 初始化 `DerivedAgentShared` 的位置可能需调整 |
| `src/command/chat/agent/agent_loop.rs` | 改 | Session 结束时合并 `sub_agent_metrics` 到 `SessionMetrics` |
| `SESSION_METRICS.md` | 改 | 更新文档说明 metrics 统计范围 |

### 代码改动细节

#### 1. `derived_shared.rs` — `call_llm_non_stream`

```rust
// 改动前（返回 Choice）
pub fn call_llm_non_stream(req: &LlmNonStreamRequest) -> Result<Choice, String> {
    // ...
    Ok(response.choices.into_iter().next().ok_or_else(|| "...")?)
}

// 改动后（返回 ChatResponse）
pub fn call_llm_non_stream(req: &LlmNonStreamRequest) -> Result<ChatResponse, String> {
    // ...
    Ok(response)
}
```

调用方改动（sub_agent.rs、teammate_loop.rs）：

```rust
// 改动前
let choice = call_llm_non_stream(&req)?;
let assistant_text = choice.message.content.clone().unwrap_or_default();

// 改动后
let response = call_llm_non_stream(&req)?;
let choice = response.choices.into_iter().next().ok_or_else(|| "...")?;
let assistant_text = choice.message.content.clone().unwrap_or_default();
// 同时可获取 usage
if let Some(usage) = response.usage {
    // 累加到 metrics
}
```

#### 2. `derived_shared.rs` — `SubAgentMetrics` 和 `DerivedAgentShared`

```rust
/// 子 Agent（SubAgent/Teammate）的 metrics 累加
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubAgentMetrics {
    pub total_llm_calls: u32,
    pub total_tool_calls: u32,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_llm_elapsed_ms: u64,
    pub total_tool_elapsed_ms: u64,
    pub ttft_ms_per_call: Vec<u64>,
}

// DerivedAgentShared 新增字段
pub sub_agent_metrics: Arc<Mutex<SubAgentMetrics>>,
```

#### 3. `sub_agent.rs` — `run_sub_agent_loop`

关键改动位置：
- 第 571 行附近：`call_llm_non_stream` 调用后获取 usage 并累加
- 第 672–698 行：工具执行时累加 `tool_calls_count` 和 `tool_elapsed_ms`
- Loop 结束前（约第 717 行）：将 `SubAgentMetrics` 推入共享 accumulator

需要传入 `sub_agent_metrics: Arc<Mutex<SubAgentMetrics>>` 到 loop 参数（通过 `SubAgentLoopParams` 或直接传入）。

#### 4. `teammate_loop.rs` — `run_teammate_loop`

关键改动位置：
- 第 308 行附近：`call_llm_non_stream` 调用后获取 usage 并累加
- 第 580 行附近：工具执行时累加 `tool_calls_count` 和 `tool_elapsed_ms`
- Loop 结束前（约第 650 行）：将 metrics 推入共享 accumulator

`TeammateLoopConfig` 需添加 `sub_agent_metrics: Arc<Mutex<SubAgentMetrics>>` 字段。

#### 5. `agent_loop.rs` — 合并 metrics

在 `write_session_metrics` 调用前（约第 1362 行）：

```rust
// 合并子 Agent metrics
if let Ok(sub_metrics) = tool_ctx.shared.sub_agent_metrics.lock() {
    metrics.total_llm_calls += sub_metrics.total_llm_calls;
    metrics.total_tool_calls += sub_metrics.total_tool_calls;
    metrics.total_input_tokens += sub_metrics.total_input_tokens;
    metrics.total_output_tokens += sub_metrics.total_output_tokens;
    metrics.total_llm_elapsed_ms += sub_metrics.total_llm_elapsed_ms;
    metrics.total_tool_elapsed_ms += sub_metrics.total_tool_elapsed_ms;
    metrics.ttft_ms_per_call.extend(&sub_metrics.ttft_ms_per_call);
}
```

## 测试验证

1. 手动测试：启动一个 session，调用 Agent 工具（SubAgent），检查 `metrics.json` 是否包含子 agent 的统计
2. 手动测试：启动 Teammate，检查 metrics 是否正确累加
3. 单元测试：为 `SubAgentMetrics` 的累加逻辑添加测试

## 风险与注意事项

1. **并发安全**：`Arc<Mutex<SubAgentMetrics>>` 可能被多个后台 Agent 同时写入，需确保锁不会造成性能瓶颈或死锁
2. **TTFT 精度**：非流式调用没有"首字延迟"概念，只能记录整次调用耗时（已在文档中说明）
3. **向后兼容**：`call_llm_non_stream` 返回值改动会影响所有调用方，需同步更新