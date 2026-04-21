# 任务规划：检测 Background Task 存活状态

## 问题分析

### 当前问题
后台任务管理器 (`BackgroundManager`) 只依赖内部状态字段 `status == "running"` 来判断任务是否存活，但不会检测实际进程是否真的在运行。这会导致以下问题：

1. **进程被外部杀死**：如果后台进程被用户手动 kill 或被系统 OOM 杀死，`BgTask.status` 仍为 "running"
2. **执行线程 panic**：如果 shell 执行线程意外崩溃，status 可能无法正确更新
3. **system prompt 误导**：`build_running_summary` 会向 AI 报告已死的任务仍在运行，导致 AI 可能尝试查询不存在任务的结果

### 影响范围
- `src/command/chat/tools/background.rs`: BgTask 结构体、BackgroundManager 管理逻辑
- `src/command/chat/tools/shell.rs`: 后台任务执行、进程 PID 获取
- `src/command/chat/app/chat_app.rs`: 内置 hook background_status 的处理逻辑

## 方案对比

### 方案 A：保存 PID + kill(pid, 0) 检测（主策略）

**原理**：
- 在 `spawn_command` 时为 BgTask 添加 `child_pid: Option<u32>` 字段
- 在 `shell.rs` 的 `execute_background` 中，子进程 spawn 成功后立即调用 `manager.update_child_pid(task_id, child.id())`
- 在 PreLlmRequest hook 中使用 `nix::sys::signal::kill(pid, None)` 检测进程存活（不发送信号，只检查进程是否存在）

**优点**：
1. **精确匹配**：PID 是进程的唯一标识，不会误判
2. **性能高效**：kill(pid, 0) 是内核调用，无需解析文本输出
3. **项目已依赖 nix crate**：版本 0.31，可直接使用

**缺点**：
1. 需要修改 BgTask 结构体和 spawn_command 接口
2. 需要在线程间传递 PID（增加几行代码）

### 方案 B：command + pgrep 匹配（辅助策略）

**原理**：
- 不修改 BgTask 结构体
- 检测存活时调用 `pgrep -f "$command"` 查找匹配进程
- 如果找到匹配进程，认为存活；否则认为已死

**优点**：
1. 代码改动少，不需要修改结构体
2. 直观易懂

**缺点**：
1. **误判风险高**：相同命令启动多个进程时无法区分
2. **性能差**：每次检测都要 fork + exec pgrep
3. **依赖外部工具**：pgrep 不是所有系统都有
4. **command 已截断**：`build_running_summary` 显示的 command 可能被截断

## 最终方案：PID + Command 双重验证

### 为什么需要双重验证？

| 风险场景 | 单一 PID 检测的问题 | 双重验证如何解决 |
|----------|---------------------|-----------------|
| PID 复用 | 原进程死后 PID 被新进程复用，`kill(pid, None)` 返回 true，误判为"存活" | 用 command 匹配验证新进程是否是同一个命令 |
| 进程被替换 | 原进程被 kill 后，同目录下其他同名进程占用了该 PID | command 匹配可发现命令行参数不一致 |

### 检测逻辑

```
对每个 status == "running" 的任务:
  1. 如果 child_pid 存在:
     a. kill(pid, None) 检测进程是否存在
     b. 如果进程不存在 → 确认死亡
     c. 如果进程存在 → 用 command + /proc/<pid>/cmdline 验证是同一个命令
        - 匹配 → 确认存活
        - 不匹配（PID 被复用）→ 确认死亡
  2. 如果 child_pid 不存在（SubAgent 等）:
     a. 用 pgrep + command 辅助验证
```

### 为什么选择 /proc/<pid>/cmdline 而不是 pgrep？

- `/proc/<pid>/cmdline` 直接读取指定 PID 的命令行，**无需搜索所有进程**
- 精确匹配特定 PID 的命令，不会受其他进程干扰
- 性能更高：直接读取文件 vs fork+exec pgrep

### 具体实现步骤

#### Step 1: 扩展 BgTask 结构体

**文件**: `src/command/chat/tools/background.rs`

**修改内容**:
```rust
pub(super) struct BgTask {
    pub task_id: String,
    pub command: String,
    pub status: String, // "running" | "completed" | "error" | "timeout" | "dead"
    pub output_buffer: Arc<Mutex<String>>,
    pub result: Option<String>,
    pub started_at: Instant,
    /// 子进程 PID，用于存活检测（仅 shell 后台任务有值，SubAgent 后台无子进程）
    pub child_pid: Option<u32>,
}
```

- `spawn_command` 中初始化 `child_pid: None`
- 新增 `update_child_pid` 方法，允许执行线程在 spawn 进程后回填 PID

#### Step 2: 修改 ShellTool execute_background

**文件**: `src/command/chat/tools/shell.rs`

**修改内容**:
- 在 `child_cmd.spawn()` 成功后（约第 323 行），获取 `child.id()` 并调用 `manager.update_child_pid`
- 仅增加 2 行代码

```rust
let mut child = match child_cmd.spawn() {
    Ok(c) => {
        let pid = c.id();
        manager.update_child_pid(&tid, pid);  // ★ 新增
        c
    },
    Err(e) => { ... }
};
```

#### Step 3: 实现进程存活检测方法（双重验证）

**文件**: `src/command/chat/tools/background.rs`

**新增方法**:

```rust
/// 第一层检测：通过 PID 检测进程是否存在
#[cfg(unix)]
fn process_exists(pid: u32) -> bool {
    use nix::sys::signal::kill;
    use nix::unistd::Pid;
    use nix::errno::Errno;
    
    match kill(Pid::from_raw(pid as i32), None) {
        Ok(_) => true,
        Err(Errno::ESRCH) => false, // 进程不存在
        Err(_) => true, // 其他错误（如权限不足），保守返回 true
    }
}

#[cfg(not(unix))]
fn process_exists(_pid: u32) -> bool { true }

/// 第二层检测：读取 /proc/<pid>/cmdline 验证命令是否匹配
/// 用于防止 PID 复用导致的误判
#[cfg(unix)]
fn command_matches_pid(pid: u32, expected_command: &str) -> bool {
    use std::fs;
    use std::io::Read;
    
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    let mut file = match fs::File::open(&cmdline_path) {
        Ok(f) => f,
        Err(_) => return false, // 无法读取，可能进程已死
    };
    
    let mut cmdline_bytes = Vec::new();
    if file.read_to_end(&mut cmdline_bytes).is_err() {
        return false;
    }
    
    // /proc/<pid>/cmdline 格式：参数以 \0 分隔，需替换为空格
    let cmdline = String::from_utf8_lossy(&cmdline_bytes)
        .replace('\0', " ")
        .trim_end()
        .to_string();
    
    // 命令匹配逻辑：
    // expected_command 是 "bash -c xxx"，cmdline 是 "/bin/bash -c xxx"
    // 只需检查核心部分是否匹配（去除路径前缀）
    let expected_parts: Vec<&str> = expected_command.split_whitespace().collect();
    let cmdline_parts: Vec<&str> = cmdline.split_whitespace().collect();
    
    if expected_parts.is_empty() || cmdline_parts.is_empty() {
        return false;
    }
    
    // 检查命令名（去除路径）是否匹配
    let expected_cmd = expected_parts[0];
    let actual_cmd = cmdline_parts[0].rsplit('/').next().unwrap_or(cmdline_parts[0]);
    
    if expected_cmd != actual_cmd && !expected_cmd.ends_with(actual_cmd) {
        return false;
    }
    
    // 检查参数是否包含 expected_command 的核心部分
    // 例如 expected_command = "npm run dev" → cmdline 应包含 "npm" 和 "run dev"
    cmdline.contains(expected_command) || 
        expected_parts.iter().skip(1).all(|p| cmdline.contains(p))
}

#[cfg(not(unix))]
fn command_matches_pid(_pid: u32, _expected_command: &str) -> bool { true }

/// 无 PID 时的备选检测：通过 pgrep + command 验证
#[cfg(unix)]
fn is_process_alive_by_command(command: &str) -> bool {
    use std::process::Command;
    let cmd_name = command.split_whitespace().next().unwrap_or(command);
    let output = Command::new("pgrep").arg("-x").arg(cmd_name).output();
    match output {
        Ok(o) => o.status.success(),
        Err(_) => true, // pgrep 不存在或执行失败，保守返回 true
    }
}

#[cfg(not(unix))]
fn is_process_alive_by_command(_command: &str) -> bool { true }
```

#### Step 4: 实现清理已死进程的方法（双重验证逻辑）

**文件**: `src/command/chat/tools/background.rs`

**新增方法 `cleanup_dead_tasks`**:

```rust
/// 清理已死进程：双重验证（PID 存在 + command 匹配）
pub fn cleanup_dead_tasks(&self) {
    let mut tasks = safe_lock(&self.tasks, "BackgroundManager::cleanup_dead_tasks");
    let mut dead_tasks = Vec::new();
    
    for task in tasks.values() {
        if task.status != "running" { continue; }
        
        let confirmed_alive = if let Some(pid) = task.child_pid {
            // 双重验证：PID 存在 + command 匹配
            if !process_exists(pid) {
                false // 进程不存在
            } else {
                // 进程存在，验证 command 是否匹配（防止 PID 复用）
                command_matches_pid(pid, &task.command)
            }
        } else {
            // 无 PID（SubAgent 等），用 pgrep 备选验证
            is_process_alive_by_command(&task.command)
        };
        
        if !confirmed_alive {
            dead_tasks.push((task.task_id.clone(), task.command.clone(), task.child_pid));
        }
    }
    
    // 更新状态并生成通知
    let mut notifs = Vec::new();
    for (task_id, command, pid) in dead_tasks {
        if let Some(task) = tasks.get_mut(&task_id) {
            task.status = "dead".to_string();
            let pid_info = pid.map_or(String::new(), |p| format!(" (PID: {})", p));
            task.result = Some(format!("进程已终止{}：被外部杀死、崩溃或 PID 被复用", pid_info));
        }
        let pid_info = pid.map_or(String::new(), |p| format!(" (PID: {})", p));
        notifs.push(BgNotification {
            task_id, command,
            status: "dead".to_string(),
            result: format!("进程已终止{}：被外部杀死、崩溃或 PID 被复用", pid_info),
        });
    }
    
    if !notifs.is_empty() {
        let mut queue = safe_lock(&self.notifications, "cleanup_dead_tasks_notify");
        queue.extend(notifs);
    }
}
```

#### Step 5: 在 PreLlmRequest hook 中执行存活检测

**文件**: `src/command/chat/app/chat_app.rs`

**修改位置**: `background_status` hook（约第 276 行）

**修改内容**: 在 `build_running_summary` 之前调用 `cleanup_dead_tasks()`

```rust
move |ctx| {
    // ★ 先清理已死进程
    bg_mgr.cleanup_dead_tasks();
    
    // 然后构建运行摘要
    let running_summary = build_running_summary(&bg_mgr);
    let notifications = bg_mgr.drain_notifications();
    ...
}
```

#### Step 6: 添加详细的日志输出

**日志框架**：使用项目自定义宏（`crate::info!`、`crate::error!`）和文件日志（`write_info_log`）

**日志输出点**：

| 位置 | 日志级别 | 日志内容 |
|------|----------|----------|
| `update_child_pid` | info! | `[BgTask] 后台任务 {} 已关联子进程 PID: {}` |
| `cleanup_dead_tasks` 开始 | write_info_log | `[BgTask] 开始存活检测，共 {} 个 running 任务` |
| `process_exists` 返回 false | write_info_log | `[BgTask] 任务 {} (PID: {}) 进程不存在 (ESRCH)` |
| `command_matches_pid` 返回 false | write_info_log | `[BgTask] 任务 {} (PID: {}) PID 复用检测: cmdline 不匹配, expected="{}", actual="{}"` |
| 任务确认为 dead | write_info_log | `[BgTask] 任务 {} (PID: {}, cmd: {}) 已确认为 dead: {}` |
| 无 PID 的任务检测 | write_info_log | `[BgTask] 任务 {} 无 PID，使用 command 匹配检测 (cmd: {})` |
| `cleanup_dead_tasks` 结束 | write_info_log | `[BgTask] 存活检测完成，发现 {} 个 dead 任务` |
| 检测过程出错 | error! | `[BgTask] 存活检测异常: {}` |

**代码示例**:
```rust
pub fn cleanup_dead_tasks(&self) {
    let mut tasks = safe_lock(&self.tasks, "BackgroundManager::cleanup_dead_tasks");
    let running_count = tasks.values().filter(|t| t.status == "running").count();
    
    if running_count == 0 { return; }
    
    crate::util::log::write_info_log(
        "BgTask::cleanup_dead_tasks",
        &format!("开始存活检测，共 {} 个 running 任务", running_count),
    );
    
    let mut dead_tasks = Vec::new();
    
    for task in tasks.values() {
        if task.status != "running" { continue; }
        
        let confirmed_alive = if let Some(pid) = task.child_pid {
            // 双重验证
            if !process_exists(pid) {
                crate::util::log::write_info_log(
                    "BgTask::cleanup_dead_tasks",
                    &format!("任务 {} (PID: {}) 进程不存在", task.task_id, pid),
                );
                false
            } else {
                let matches = command_matches_pid(pid, &task.command);
                if !matches {
                    crate::util::log::write_info_log(
                        "BgTask::cleanup_dead_tasks",
                        &format!("任务 {} (PID: {}) PID 复用检测: cmdline 不匹配, cmd={}", 
                            task.task_id, pid, task.command),
                    );
                }
                matches
            }
        } else {
            crate::util::log::write_info_log(
                "BgTask::cleanup_dead_tasks",
                &format!("任务 {} 无 PID，使用 command 匹配检测 (cmd: {})", 
                    task.task_id, task.command),
            );
            is_process_alive_by_command(&task.command)
        };
        
        if !confirmed_alive {
            dead_tasks.push((task.task_id.clone(), task.command.clone(), task.child_pid));
        }
    }
    
    // 更新状态...
    for (task_id, command, pid) in &dead_tasks {
        let pid_info = pid.map_or(String::new(), |p| format!("PID: {}", p));
        crate::util::log::write_info_log(
            "BgTask::cleanup_dead_tasks",
            &format!("任务 {} ({} cmd: {}) 已确认为 dead", task_id, pid_info, command),
        );
    }
    
    crate::util::log::write_info_log(
        "BgTask::cleanup_dead_tasks",
        &format!("存活检测完成，发现 {} 个 dead 任务", dead_tasks.len()),
    );
    
    // ... 更新 tasks 和 notifications ...
}
```

## 边界情况处理

### 1. SubAgent 后台任务
SubAgent 后台任务**没有子进程**（它是一个纯 Rust 线程，不涉及 `std::process::Command`），所以 `child_pid` 始终为 `None`。对于这类任务：
- PID 检测不适用（`child_pid` 为 None）
- 通过 `is_process_alive_by_command` 辅助验证（command 格式为 `"Agent: xxx"`）
- **注意**：`is_process_alive_by_command` 对 "Agent: xxx" 类命令可能找不到匹配进程，这是正确的行为——SubAgent 本身没有对应的外部进程
- SubAgent 的存活检测可以依赖线程的 `is_finished()` 状态，但这需要额外改动，当前暂不处理

### 2. 非 Unix 平台（Windows）
Windows 平台没有 `kill(pid, 0)` 等效方法：
- 使用条件编译 `#[cfg(not(unix))]` 返回 `true`（保守策略）
- 后续可用 Windows API `OpenProcess` 增强

### 3. PID 复用问题（已解决）
双重验证（PID 存在 + command 匹配）可以有效防止 PID 复用导致的误判：
- 如果 PID 存在但 `/proc/<pid>/cmdline` 不匹配 → 确认死亡
- 如果 PID 存在且 command 匹配 → 确认存活

### 4. cleanup_dead_tasks 的调用频率
- 每次发送 LLM request 前调用（PreLlmRequest hook）
- 频率适中，不会对性能产生显著影响
- kill(pid, 0) 是 O(1) 的内核调用

## 测试场景

1. **正常完成**：任务正常完成，status 正确更新为 "completed"
2. **超时终止**：任务超时被 kill，status 正确更新为 "timeout"
3. **手动杀死**：用户在外部 `kill -9 <pid>` 后，下次 LLM request 前 status 更新为 "dead"
4. **进程崩溃**：进程异常退出，status 更新为 "dead"
5. **PID 复用**：原进程死后新进程占用同一 PID，command 验证能正确识别为 "dead"
6. **SubAgent 后台**：SubAgent 后台任务不受影响（child_pid 为 None，pgrep 备选验证）

## 预估工作量

- 代码修改：3 个文件（background.rs、shell.rs、chat_app.rs），新增 ~60 行代码
- 测试验证：需要手动测试各种场景
- 总预估时间：30-45 分钟

## 依赖项

- `nix = { version = "0.31", features = ["process", "signal"] }`（需要添加 `signal` feature）
- Unix 平台特定代码使用 `#[cfg(unix)]` 条件编译