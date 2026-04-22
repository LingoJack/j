# SubAgent 重试状态更新异常修复计划

## 问题分析

### 核心问题
`call_llm_non_stream` 内部实现了指数退避重试机制（最多 2 次重试），但**重试期间 SubAgent 状态没有任何更新**，导致：

1. **UI 无法感知重试进度**：用户看不到 subagent 正在重试 API 请求
2. **状态停留在 `Working`**：重试期间状态一直是 `Working`，无法区分"正常工作"和"正在重试"
3. **日志与 UI 不同步**：`call_llm_non_stream` 写了日志 `write_info_log("SubAgentLLM", ...)`，但 UI 状态栏无法展示

### 代码路径分析

**调用链**：
```
run_sub_agent_loop (sub_agent.rs:444)
  → call_llm_non_stream (derived_shared.rs:344)
    → 内部重试循环 (derived_shared.rs:356-386)
```

**问题根源**：
- `run_sub_agent_loop` 持有 `params.snapshot: Option<SubAgentLoopStateRefs>`，可以更新状态
- `call_llm_non_stream` **没有接收状态引用参数**，无法在重试期间更新状态
- 只有当 `call_llm_non_stream` 完全失败返回 `Err` 后，`run_sub_agent_loop` 才更新为 `Error` 状态

**对比主 Agent**：
- 主 agent (`agent_loop.rs`) 在重试时发送 `StreamMsg::Retrying { attempt, max_attempts, delay_ms, error }` 消息
- UI 可以显示重试进度条和错误信息
- SubAgent 缺少类似的反馈机制

## 解决方案

### 方案选择：回调机制

采用**回调函数**方案，保持 `call_llm_non_stream` 的通用性：

```rust
// 定义重试回调类型
type RetryCallback = Box<dyn Fn(u32, u32, u64, &str) + Send + Sync>;

// call_llm_non_stream 新增回调参数
pub fn call_llm_non_stream(
    rt: &tokio::runtime::Runtime,
    client: &async_openai::Client<async_openai::config::OpenAIConfig>,
    provider: &ModelProvider,
    messages: &[ChatMessage],
    tools: &[ChatCompletionTools],
    system_prompt: Option<&str>,
    on_retry: Option<&RetryCallback>,  // 新增
) -> Result<ChatChoice, String>
```

### 实现步骤

#### Step 1: 修改 `SubAgentStatus` 枚举（derived_shared.rs）

新增 `Retrying` 状态，携带重试信息：

```rust
pub enum SubAgentStatus {
    Initializing,
    Working,
    Retrying {           // 新增
        attempt: u32,
        max_attempts: u32,
        delay_ms: u64,
        error: String,
    },
    Completed,
    Cancelled,
    Error(String),
}
```

#### Step 2: 修改 `call_llm_non_stream`（derived_shared.rs）

添加回调参数，在重试前调用：

```rust
pub fn call_llm_non_stream(
    ...
    on_retry: Option<&dyn Fn(u32, u32, u64, &str)>,  // 简化：使用 dyn trait
) -> Result<ChatChoice, String> {
    ...
    loop {
        attempt += 1;
        match rt.block_on(...) {
            Err(e) => {
                if let Some(policy) = derived_retry_policy(&chat_err)
                    && attempt <= policy.max_attempts
                {
                    let delay_ms = backoff_delay_ms(...);
                    // 调用回调通知重试
                    if let Some(cb) = on_retry {
                        cb(attempt, policy.max_attempts, delay_ms, &chat_err.display_message());
                    }
                    std::thread::sleep(...);
                    continue;
                }
                ...
            }
        }
    }
}
```

#### Step 3: 修改 `run_sub_agent_loop`（sub_agent.rs）

传入回调函数，更新状态和推送 UI：

```rust
let retry_callback = |attempt: u32, max_attempts: u32, delay_ms: u64, error: &str| {
    // 1. 更新状态为 Retrying
    if let Some(ref refs) = params.snapshot {
        refs.set_status(SubAgentStatus::Retrying {
            attempt,
            max_attempts,
            delay_ms,
            error: error.to_string(),
        });
    }
    // 2. 推送 UI 消息
    push_ui(ChatMessage::text(
        MessageRole::Assistant,
        format!(
            "<{}> [重试 {}/{}, {}ms 后] {}",
            agent_name, attempt, max_attempts, delay_ms, error
        ),
    ));
};

let choice = match call_llm_non_stream(
    &rt,
    &client,
    &params.provider,
    &messages,
    &params.tools,
    params.system_prompt.as_deref(),
    Some(&retry_callback),  // 传入回调
) { ... };
```

#### Step 4: 更新 UI 显示逻辑

修改以下文件以支持 `SubAgentStatus::Retrying` 的显示：

- `ui/chat.rs:367-391`：状态图标和颜色映射
- `ui/config/teammates.rs:257-277`：状态图标和颜色映射
- `app/session_mgr.rs:142-146`：状态字符串映射

```rust
// 状态图标映射
SubAgentStatus::Retrying { .. } => "↻",  // 重试图标
// 状态颜色映射
SubAgentStatus::Retrying { .. } => t.title_warning,  // 使用警告色
// 状态字符串映射
SubAgentStatus::Retrying { .. } => "retrying",
```

#### Step 5: 处理重试成功后的状态恢复

重试成功后，需要将状态从 `Retrying` 恢复为 `Working`：

```rust
// 在 call_llm_non_stream 返回 Ok 后
if let Some(ref refs) = params.snapshot {
    refs.set_status(SubAgentStatus::Working);
}
```

## 影响范围

### 直接修改文件
1. `src/command/chat/tools/derived_shared.rs`：`SubAgentStatus` 枚举 + `call_llm_non_stream` 函数
2. `src/command/chat/tools/sub_agent.rs`：`run_sub_agent_loop` 调用处
3. `src/command/chat/ui/chat.rs`：UI 状态显示
4. `src/command/chat/ui/config/teammates.rs`：UI 状态显示
5. `src/command/chat/app/session_mgr.rs`：状态字符串映射

### 间接影响
- `call_llm_non_stream` 的其他调用点需要适配（目前只有 `run_sub_agent_loop` 和 `run_teammate_loop`）
- `run_teammate_loop` 也需要类似的回调处理（可选，暂不处理）

## 测试验证

### 测试场景
1. **网络超时重试**：模拟网络超时，观察 UI 是否显示 `Retrying` 状态和重试进度
2. **429 Rate Limit 重试**：触发 API 限流，验证重试延迟显示正确
3. **重试成功恢复**：重试成功后，状态应恢复为 `Working`
4. **重试耗尽报错**：重试次数耗尽后，状态应变为 `Error`

### 验证方法
- 检查日志：`write_info_log` 输出应与 UI 状态同步
- 检查 UI：状态栏应显示重试图标和进度
- 检查 `/dump`：`SubAgentSnapshot` 状态字段应正确反映重试状态

## 风险评估

### 低风险
- 修改范围明确，不涉及核心业务逻辑
- 回调机制保持向后兼容（`Option` 参数，现有调用传 `None` 即可）

### 需注意
- 状态更新频率：避免高频更新导致 UI 性能问题（重试间隔通常 2-15 秒，频率很低）
- 状态转换顺序：确保 `Retrying → Working` 转换在成功时正确执行

## 实施顺序

1. 修改 `SubAgentStatus` 枚举（Step 1）
2. 修改 `call_llm_non_stream` 函数（Step 2）
3. 修改 `run_sub_agent_loop` 调用处（Step 3）
4. 更新 UI 显示逻辑（Step 4）
5. 处理状态恢复（Step 5）
6. 运行 `cargo fmt` 和 `cargo clippy` 检查
7. 测试验证