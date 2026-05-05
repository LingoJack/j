# 修复 Esc 取消和 /clear 时 Teammate 未正确停止的问题

## 问题分析

### 问题 1: Esc 取消请求时 Teammate 继续运行

**根因**:
- `cancel_stream()` 方法（`chat_app.rs:781-783`）只调用 `finish_loading(false, true)` 来取消 Main Agent
- `finish_loading()` 只处理 Main Agent 的取消（`self.main_agent.cancel()`），没有调用 `teammate_manager.stop_all()` 来停止 teammates

**代码位置**:
- `src/command/chat/app/chat_app.rs:781-783` — `cancel_stream()` 方法
- `src/command/chat/app/stream_poll.rs:453-456` — `finish_loading()` 方法

### 问题 2: /clear 后 Teammate 依然存在

**根因**:
- `clear_session()` 方法（`session_mgr.rs:344-366`）调用了 `clear_runtime_state()`
- `clear_runtime_state()` 确实调用了 `mgr.stop_all()` 和 `mgr.cleanup_finished()`
- 但是 `stop_all()` 只是发送取消信号，不会立即移除 teammate
- `cleanup_finished()` 只移除已完成的 teammate（`!h.running() && thread_handle.is_finished()`）
- 问题在于：取消信号发出后，teammate 线程需要时间响应，`cleanup_finished()` 立即执行时线程可能还没结束

**代码位置**:
- `src/command/chat/app/session_mgr.rs:317-341` — `clear_runtime_state()` 方法
- `src/command/chat/teammate/manager.rs:466-471` — `stop_all()` 方法
- `src/command/chat/teammate/manager.rs:474-496` — `cleanup_finished()` 方法

## 解决方案

### 修复 1: Esc 取消时同时停止 Teammates

在 `cancel_stream()` 方法中添加对 teammates 的停止：

```rust
// src/command/chat/app/chat_app.rs
pub fn cancel_stream(&mut self) {
    // 停止所有 teammates
    if let Ok(mut mgr) = self.teammate_manager.lock() {
        mgr.stop_all();
    }
    self.finish_loading(false, true);
}
```

### 修复 2: /clear 时确保 Teammates 被清理

在 `clear_runtime_state()` 中，`stop_all()` 后需要等待线程结束或强制清理：

**方案 A（推荐）**: 在 `stop_all()` 后添加短暂等待 + 再次 `cleanup_finished()`

```rust
// src/command/chat/app/session_mgr.rs
pub fn clear_runtime_state(&mut self) {
    if let Ok(mut mgr) = self.teammate_manager.lock() {
        mgr.stop_all();
        // 给 teammate 线程一点时间响应取消信号
        std::thread::sleep(std::time::Duration::from_millis(50));
        mgr.cleanup_finished();
        // 强制清理所有剩余的 teammate（无论是否完成）
        mgr.clear_all();
        mgr.clear_recovered_teammates();
    }
    // ... 其余代码不变
}
```

**方案 B**: 在 `TeammateManager` 中添加 `clear_all()` 方法，强制移除所有 teammates

```rust
// src/command/chat/teammate/manager.rs
/// 强制清除所有 teammates（不等待线程结束）
pub fn clear_all(&mut self) {
    for (name, mut handle) in self.teammates.drain() {
        handle.cancel();
        if let Some(thread) = handle.thread_handle.take() {
            // 线程会自动 detach，不等待 join
            drop(thread);
        }
        write_info_log("TeammateManager", &format!("force cleared teammate: {}", name));
    }
}
```

## 实施步骤

1. 在 `TeammateManager` 中添加 `clear_all()` 方法
2. 修改 `cancel_stream()` 添加 `teammate_manager.stop_all()` 调用
3. 修改 `clear_runtime_state()` 使用 `clear_all()` 确保彻底清理

## 测试验证

1. 创建一个 teammate，按 Esc，确认 teammate 状态变为 Cancelled
2. 创建一个 teammate，执行 /clear，确认 teammate 被移除
3. 创建多个 teammates，执行 /clear，确认所有 teammates 被移除
