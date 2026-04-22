# SubAgent 重试状态更新异常与后台任务误判 dead 修复计划

## 问题一：重试状态更新异常

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

### 解决方案

采用**回调函数**方案，保持 `call_llm_non_stream` 的通用性：

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
    on_retry: Option<&dyn Fn(u32, u32, u64, &str)>,  // 新增回调参数
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
                    // ★ 调用回调通知重试
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

传入回调函数，更新状态：

```rust
let retry_callback = |attempt: u32, max_attempts: u32, delay_ms: u64, error: &str| {
    if let Some(ref refs) = params.snapshot {
        refs.set_status(SubAgentStatus::Retrying {
            attempt,
            max_attempts,
            delay_ms,
            error: error.to_string(),
        });
    }
};

let choice = match call_llm_non_stream(
    ...,
    Some(&retry_callback),  // 传入回调
) { ... };

// 成功后恢复 Working 状态
if let Some(ref refs) = params.snapshot {
    refs.set_status(SubAgentStatus::Working);
}
```

#### Step 4: 更新 UI 显示逻辑

修改以下文件以支持 `SubAgentStatus::Retrying`：
- `ui/chat.rs`：状态图标映射 `"↻"`，颜色 `t.title_warning`
- `ui/config/teammates.rs`：同上
- `app/session_mgr.rs`：状态字符串 `"retrying"`

---

## 问题二：TaskOutput 输出 dead（后台任务误判）

### 核心问题
SubAgent 后台任务在 `cleanup_dead_tasks` 检测中被**误判为 dead**，导致 `TaskOutput` 返回 `status: "dead"`。

### 问题根源分析

**SubAgent 后台任务注册**（sub_agent.rs:218）：
```rust
let (task_id, output_buffer) = self.shared.background_manager.spawn_command(
    &format!("Agent: {}", description),  // command = "Agent: xxx"
    None,
    0,
);
// ★ 没有 child_pid（因为是线程而非子进程）
```

**存活检测逻辑**（background.rs:244-253）：
```rust
let confirmed_alive = if let Some(pid) = task.child_pid {
    process_exists(pid)  // 有 PID 时用 kill(pid, 0) 检测
} else {
    // ★ 无 PID 时用 pgrep 检测
    is_process_alive_by_command(&task.command)
};
```

**`is_process_alive_by_command` 实现**（background.rs:329-337）：
```rust
fn is_process_alive_by_command(command: &str) -> bool {
    let cmd_name = command.split_whitespace().next().unwrap_or(command);
    // cmd_name = "Agent:"  ← 不是可执行程序名！
    let output = Command::new("pgrep").arg("-x").arg(cmd_name).output();
    match output {
        Ok(o) => o.status.success(),  // pgrep -x "Agent:" 永远失败
        Err(_) => true,
    }
}
```

**问题**：
1. SubAgent 是**线程**而非**子进程**，没有 PID
2. command 字段是 `"Agent: xxx"`，第一个单词 `"Agent:"` 不是可执行程序
3. `pgrep -x "Agent:"` 永远找不到进程 → 返回 false → 误判为 dead

### 解决方案

**方案：为线程类后台任务引入 `is_running` Arc 标记**

#### Step 1: 修改 `BgTask` 结构体（background.rs）

添加线程存活标记：

```rust
pub(super) struct BgTask {
    pub task_id: String,
    pub command: String,
    pub status: String,
    pub output_buffer: Arc<Mutex<String>>,
    pub result: Option<String>,
    pub started_at: Instant,
    pub child_pid: Option<u32>,
    /// ★ 新增：线程类任务的存活标记（SubAgent 等非进程任务）
    pub is_thread_running: Option<Arc<AtomicBool>>,
}
```

#### Step 2: 修改 `spawn_command` 返回值（background.rs）

```rust
pub fn spawn_command(
    &self,
    command: &str,
    _cwd: Option<String>,
    _timeout_secs: u64,
    is_thread_running: Option<Arc<AtomicBool>>,  // ★ 新增参数
) -> (String, Arc<Mutex<String>>) {
    ...
    let bg_task = BgTask {
        ...
        is_thread_running,  // 存储 Arc
    };
    ...
}
```

#### Step 3: 修改 `cleanup_dead_tasks` 检测逻辑（background.rs）

```rust
let confirmed_alive = if let Some(pid) = task.child_pid {
    process_exists(pid)  // 进程类任务：PID 检测
} else if let Some(ref is_running) = task.is_thread_running {
    // ★ 线程类任务：直接检查 AtomicBool
    is_running.load(Ordering::Relaxed)
} else {
    // 兜底：pgrep 检测（仅适用于 shell 命令）
    is_process_alive_by_command(&task.command)
};
```

#### Step 4: 修改 SubAgent 注册调用（sub_agent.rs）

```rust
let (task_id, output_buffer) = self.shared.background_manager.spawn_command(
    &format!("Agent: {}", description),
    None,
    0,
    Some(Arc::clone(&handle.is_running)),  // ★ 传入线程存活标记
);
```

---

## 影响范围

### 问题一修改文件
1. `src/command/chat/tools/derived_shared.rs`：`SubAgentStatus` 枚举 + `call_llm_non_stream` 函数
2. `src/command/chat/tools/sub_agent.rs`：`run_sub_agent_loop` 调用处
3. `src/command/chat/ui/chat.rs`：UI 状态显示
4. `src/command/chat/ui/config/teammates.rs`：UI 状态显示
5. `src/command/chat/app/session_mgr.rs`：状态字符串映射

### 问题二修改文件
1. `src/command/chat/tools/background.rs`：`BgTask` 结构体 + `spawn_command` + `cleanup_dead_tasks`
2. `src/command/chat/tools/sub_agent.rs`：`spawn_command` 调用处

### 间接影响
- `call_llm_non_stream` 的其他调用点（`run_teammate_loop`）需要适配，可传 `None` 保持向后兼容

---

## 测试验证

### 问题一测试
1. 模拟网络超时重试，观察 UI 是否显示 `Retrying` 状态
2. 重试成功后，状态应恢复为 `Working`
3. 重试耗尽后，状态应变为 `Error`

### 问题二测试
1. 启动 SubAgent 后台任务，调用 `cleanup_dead_tasks`
2. 验证 `is_running.load()` 返回 true，不被误判为 dead
3. 任务完成后，`is_running` 应变为 false
4. 调用 `TaskOutput` 应返回正确的 status

---

## 实施顺序

1. **问题二修复**（优先，影响更严重）
   - Step 1: 修改 `BgTask` 结构体
   - Step 2: 修改 `spawn_command` 参数
   - Step 3: 修改 `cleanup_dead_tasks` 检测逻辑
   - Step 4: 修改 SubAgent 注册调用

2. **问题一修复**
   - Step 1: 修改 `SubAgentStatus` 枚举
   - Step 2: 修改 `call_llm_non_stream` 函数
   - Step 3: 修改 `run_sub_agent_loop` 调用处
   - Step 4: 更新 UI 显示逻辑

3. **代码质量检查**
   - 运行 `cargo fmt`
   - 运行 `cargo clippy`
   - 编译验证

---

## 风险评估

### 低风险
- 修改范围明确，不涉及核心业务逻辑
- 回调机制和 `is_thread_running` 字段都是可选的，保持向后兼容

### 需注意
- `spawn_command` 新增参数会破坏现有 API，需同步修改所有调用点
  - 当前调用点：`sub_agent.rs`（SubAgent）、`shell.rs`（Shell 命令）
  - Shell 命令有 PID，可传 `None`