# Chat 模块回归测试方案

## 一、现状分析

### 模块规模
- **~140 个 `.rs` 文件**，617 个公开接口
- **现有测试**：仅 13 处 `#[cfg(test)]`，覆盖严重不足
- **已有回归测试**：`context/regression_tests.rs`（1 个文件，覆盖 compact/window/policy 跨模块一致性）

### 核心路径识别（需要回归测试保护的关键路径）

| # | 核心路径 | 涉及文件 | 当前测试 | 优先级 |
|---|---------|---------|---------|-------|
| 1 | **错误分类与转换** | `error.rs` | 有基础测试，但缺 From 转换和 retry 联动 | P0 |
| 2 | **重试策略选择** | `agent/retry.rs` | 无 | P0 |
| 3 | **退避延迟计算** | `agent/retry.rs` | 无 | P0 |
| 4 | **工具权限队列** | `permission/queue.rs` | 无 | P0 |
| 5 | **消息压缩（other agent）** | `context/message_compress.rs` | 有基础测试 | P1 |
| 6 | **上下文窗口选择** | `context/window.rs` | 有测试 + 回归测试 | P1 |
| 7 | **micro_compact 压缩** | `context/compact.rs` | 仅回归测试覆盖 | P1 |
| 8 | **Policy 映射** | `context/policy.rs` | 仅回归测试覆盖 | P1 |
| 9 | **工具处理器辅助函数** | `agent/tool_processor.rs` | 无 | P1 |
| 10 | **存储层序列化** | `storage/` | 有基础测试 | P2 |
| 11 | **Markdown 解析** | `markdown/parser.rs` | 无 | P2 |
| 12 | **常量一致性** | `constants.rs` | 无 | P2 |

## 二、测试文件组织方案

**采用分散式 + 集中式混合策略**：

1. **分散式**：在每个子模块内新增 `#[cfg(test)] mod tests`（小文件就地测试）
2. **集中式**：新增 `src/command/chat/regression_tests.rs`，覆盖跨模块联动和核心路径集成

### 新增测试文件清单

```
src/command/chat/
├── regression_tests.rs              # 新增：顶层回归测试集
├── error.rs                         # 扩展：From<LlmError> / From<reqwest> / from_api_error 路径
├── agent/
│   └── retry.rs                     # 新增：retry_policy_for + backoff_delay_ms
├── permission/
│   └── queue.rs                     # 新增：PermissionQueue 完整生命周期
├── agent/
│   └── tool_processor.rs            # 新增：drain_pending / push_both / clear_channels
├── markdown/
│   └── parser.rs                    # 新增：核心 Markdown 解析场景
```

## 三、测试用例设计

### 3.1 `agent/retry.rs` — 重试策略（P0，新增）

> **保护目标**：错误类型到重试策略的映射关系，确保每次改动不会意外改变重试行为

| 用例 | 测试内容 |
|-----|---------|
| `retry_network_timeout_is_fast` | NetworkTimeout → network_transient (base=1s, max=5次) |
| `retry_network_error_is_medium` | NetworkError → network_error (base=2s, max=5次) |
| `retry_stream_deserialize` | StreamDeserialize → deserialize_error (base=2s, max=3次) |
| `retry_503_is_overloaded` | 503 → server_overloaded (base=2s, max=4次) |
| `retry_500_is_server_error` | 500 → server_error (base=3s, max=3次) |
| `retry_502_is_server_error` | 502 → server_error |
| `retry_429_with_retry_after` | RateLimit + retry_after=30 → base=30000ms, max=1次 |
| `retry_429_without_retry_after` | RateLimit + None → rate_limit_blind (base=5s, max=3次) |
| `retry_429_retry_after_capped` | RateLimit + retry_after=200 → capped at 120s |
| `retry_abnormal_finish_network` | AbnormalFinish("network_error") → abnormal_finish |
| `retry_abnormal_finish_timeout` | AbnormalFinish("timeout") → abnormal_finish |
| `retry_abnormal_finish_overloaded` | AbnormalFinish("overloaded") → abnormal_finish |
| `retry_abnormal_finish_other` | AbnormalFinish("other_reason") → None（不重试） |
| `retry_other_overloaded_keywords` | Other("访问量过大") / Other("过载") / Other("overloaded") / Other("too busy") / Other("1305") → fallback_overloaded |
| `retry_non_retryable_errors` | ApiAuth / ApiBadRequest / HookAborted / RuntimeFailed / AgentPanic → None |
| `backoff_first_attempt_equals_base` | attempt=1, base=1000 → 范围 [1000, 1200] |
| `backoff_exponential_growth` | attempt=2 → 约 2*base, attempt=3 → 约 4*base |
| `backoff_capped_at_cap` | attempt=10, base=1000, cap=30000 → 不超过 cap+jitter |
| `backoff_never_zero` | attempt=1, base=1 → 至少 1 |

### 3.2 `error.rs` — 错误分类（P0，扩展）

> **保护目标**：ChatError 的分类逻辑，确保 API 错误正确映射到语义变体

| 用例 | 测试内容 |
|-----|---------|
| `from_http_status_401` | → ApiAuth |
| `from_http_status_403` | → ApiAuth |
| `from_http_status_429` | → ApiRateLimit { retry_after: None } |
| `from_http_status_400` | → ApiBadRequest |
| `from_http_status_500_to_599` | → ApiServerError (500/502/503/504/529 各一) |
| `from_http_status_unknown` | → Other |
| `from_api_error_rate_limit` | code="rate_limit_exceeded" → ApiRateLimit |
| `from_api_error_auth` | code="invalid_api_key" / "authentication_required" → ApiAuth |
| `from_api_error_bad_request` | code="invalid_request_error" → ApiBadRequest |
| `from_api_error_code_1305` | code="1305" → ApiRateLimit |
| `from_api_error_message_heuristics` | 各种 message 关键词 → 对应变体 |
| `from_llm_error_http` | LlmError::Http(reqwest_timeout) → NetworkTimeout |
| `from_llm_error_api_body_parsing` | LlmError::Api + JSON body → 结构化解析 |
| `from_llm_error_deserialize` | → StreamDeserialize |
| `from_llm_error_stream_interrupted` | → StreamInterrupted |
| `from_llm_error_request_build` | → RequestBuild |
| `display_message_all_variants` | 每个变体都有合理的中文显示消息 |
| `sanitize_html_edge_cases` | 嵌套标签、空输入、纯文本、自闭合标签风格 |
| `truncate_boundary_safe` | UTF-8 多字节字符边界截断 |

### 3.3 `permission/queue.rs` — 权限队列（P0，新增）

> **保护目标**：派生 Agent 权限请求的完整生命周期

| 用例 | 测试内容 |
|-----|---------|
| `agent_type_title_format` | Main/Teammate/SubAgent 各自的 title 格式 |
| `pending_perm_resolve_approved` | resolve(true) → wait_for_decision 返回 true |
| `pending_perm_resolve_denied` | resolve(false) → wait_for_decision 返回 false |
| `pending_perm_timeout_returns_false` | 超时 → false（用极短 timeout 测试） |
| `queue_request_and_pop` | push → pop 顺序 FIFO |
| `queue_deny_all` | 多个 pending → deny_all → 全部 false |
| `queue_empty_pop_returns_none` | 空队列 pop → None |
| `pending_perm_agent_type_equality` | Clone + PartialEq 正确性 |

### 3.4 `agent/tool_processor.rs` — 辅助函数（P1，新增）

> **保护目标**：agent loop 的消息管道辅助函数

| 用例 | 测试内容 |
|-----|---------|
| `drain_pending_appends_with_marker` | drain 出来的 User 消息加 `[User appended]` 前缀 |
| `drain_pending_preserves_non_user_role` | 非 User 角色的消息不加前缀 |
| `drain_pending_empty_noop` | 空 pending 不修改 messages |
| `push_both_appends_to_both_channels` | 消息同时出现在 display 和 context |
| `push_both_clones_message` | 两个通道各自持有独立 clone |
| `clear_channels_empties_both` | clear 后两个通道都为空 |

### 3.5 `context/message_compress.rs` — 消息压缩（P1，扩展）

> **保护目标**：已有测试较充分，补充边界条件

| 用例 | 测试内容 |
|-----|---------|
| `compress_empty_messages` | 空输入 → 空输出 |
| `compress_threshold_zero` | threshold=0 → 原样返回 |
| `compress_self_agent_excluded` | self_agent_name 匹配 → 不压缩 |
| `compress_mixed_content` | 混合广播消息和非广播消息 |

### 3.6 `markdown/parser.rs` — Markdown 解析（P2，新增）

> **保护目标**：核心 Markdown 渲染的输入输出稳定性

| 用例 | 测试内容 |
|-----|---------|
| `renders_plain_text` | 纯文本 → 正常输出 |
| `renders_bold` | `**bold**` → 加粗样式 |
| `renders_code_block` | 代码块 → 代码样式 |
| `renders_inline_code` | `` `code` `` → 行内代码样式 |
| `renders_heading` | `# H1` → 标题样式 |
| `renders_list_items` | `- item` → 列表缩进 |
| `handles_chinese_quotes_bold` | `**"中文"**` 加粗修复 |

### 3.7 `regression_tests.rs` — 顶层回归测试集（P0，新增）

> **保护目标**：跨模块集成不变性 — 在顶层文件中保护从 error → retry → tool_processor 的完整链路

| 用例 | 测试内容 |
|-----|---------|
| `error_to_retry_policy_chain` | ChatError 每种变体 → retry_policy_for → 策略参数一致性 |
| `error_display_message_never_panics` | 所有 ChatError 变体的 display_message 不 panic |
| `all_key_tools_have_tool_name_constants` | KeyTool 在 tool_names 模块中都有对应常量 |
| `compact_window_exemption_consistency` | BUILTIN_EXEMPT_TOOLS 在 compact 和 window 中行为一致 |
| `constants_reasonable_ranges` | 关键常量在合理范围内（如 MAX > MIN, 非零等） |
| `retry_backoff_monotonically_non_decreasing` | 同策略下 attempt 增大时 delay 不减少 |

## 四、实施优先级和文件变更清单

### Phase 1（P0，核心路径，预计 4 个文件变更）

1. **`src/command/chat/agent/retry.rs`** — 新增 `#[cfg(test)] mod tests`（~18 个用例）
2. **`src/command/chat/error.rs`** — 扩展 `#[cfg(test)] mod tests`（~18 个用例）
3. **`src/command/chat/permission/queue.rs`** — 新增 `#[cfg(test)] mod tests`（~8 个用例）
4. **`src/command/chat/regression_tests.rs`** — 新建顶层回归测试文件（~6 个用例）

### Phase 2（P1，辅助路径，预计 2 个文件变更）

5. **`src/command/chat/agent/tool_processor.rs`** — 新增 `#[cfg(test)] mod tests`（~6 个用例）
6. **`src/command/chat/context/message_compress.rs`** — 扩展 `#[cfg(test)] mod tests`（~4 个用例）

### Phase 3（P2，渲染层，预计 1 个文件变更）

7. **`src/command/chat/markdown/parser.rs`** — 新增 `#[cfg(test)] mod tests`（~7 个用例）

## 五、测试编写原则

1. **不依赖外部状态**：所有测试自包含，使用临时目录 / 内存数据结构
2. **不依赖网络**：不发起真实 HTTP 请求
3. **确定性**：对于带随机抖动的 backoff，验证范围而非精确值
4. **命名约定**：`<模块>_<行为描述>_<预期结果>` 格式
5. **断言消息**：每个 assert 都带描述性消息，失败时一目了然
6. **回归保护**：每个用例的注释说明"如果此测试失败，说明什么被破坏了"

## 六、验证方式

完成所有测试后执行：
```bash
cargo test --lib command::chat -- --nocapture
cargo clippy --lib -p j  # 确保 clippy 通过
```
