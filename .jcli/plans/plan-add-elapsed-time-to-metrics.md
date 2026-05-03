# Plan: 为 SessionMetrics 增加 elapsed_time_ms 字段

## 背景
当前 `SessionMetrics` 已经有 `session_start_ms` 和 `session_end_ms`，但没有一个直接表示"消耗时间"的字段。用户希望有一个直观的耗时统计。

## 方案

### 1. 新增字段
在 `SessionMetrics` (src/command/chat/storage/types.rs) 中新增两个字段：

```rust
/// LLM 调用总耗时（毫秒）—— 仅计算 LLM API 等待时间（含流式读取），不含工具执行时间
#[serde(default, skip_serializing_if = "is_zero_u64")]
pub total_llm_elapsed_ms: u64,
/// 工具执行总耗时（毫秒）—— 仅计算工具调用执行时间
#[serde(default, skip_serializing_if = "is_zero_u64")]
pub total_tool_elapsed_ms: u64,
```

选择拆分为两个独立字段而非单一总耗时，原因：
- `session_end_ms - session_start_ms` 已经可以算出总耗时
- 细分耗时更有价值：用户能看出时间花在 LLM 等待还是工具执行上
- 不重复存储已有信息

### 2. 采集点
- **LLM 耗时**：在 `agent_loop.rs` 中，每轮 `call_start` 到流式结束或 fallback 完成的时间，累加到 `metrics.total_llm_elapsed_ms`
- **工具耗时**：需要在 `process_tool_calls` 中返回耗时，或者用 `Instant` 在 agent_loop 的 tool call 处理前后计时

### 3. 修改文件

1. **src/command/chat/storage/types.rs** — `SessionMetrics` 新增 2 个字段
2. **src/command/chat/agent/agent_loop.rs** — 在 LLM 调用和工具调用前后计时并累加

### 4. 采集逻辑详情

#### LLM 耗时 (total_llm_elapsed_ms)
在 `'api_retry` 循环中，`call_start = Instant::now()` 已存在。
- 流式成功路径：在 `'stream` 循环结束后，累加 `call_start.elapsed().as_millis() as u64`
- fallback 非流式路径：同理，在获取到结果后累加

注意：重试场景下 `call_start` 会随 `retry_attempt` 重置，所以每次重试的耗时都会被计入，这是合理的。

#### 工具耗时 (total_tool_elapsed_ms)
- 在调用 `process_tool_calls` 前记录 `Instant::now()`，调用后累加耗时
- 需要覆盖 3 个调用点（流式有工具、fallback 有工具、以及 fallback 非流式有工具）

### 5. 兼容性
- 新字段使用 `#[serde(default)]` + `skip_serializing_if = "is_zero_u64"`，旧数据反序列化时默认为 0，序列化时为 0 则省略
- 无破坏性变更
