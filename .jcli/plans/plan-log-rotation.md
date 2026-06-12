# 日志轮转（Log Rotation）方案

## 问题

`write_info_log` / `write_error_log` 两个函数（分别在 `src/util/log.rs` 和 `j-agent/src/util/log.rs`）使用 `append(true)` 无限写入文件，日志文件会增长到数 GB。

## 方案：基于文件大小的简单轮转

不引入外部依赖，在每次写入前检查文件大小，超过阈值时执行轮转。

### 轮转策略

- **阈值**：单个日志文件最大 **10 MB**（`const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024`）
- **保留份数**：每个日志文件最多保留 **3 份**历史备份（`.1`, `.2`, `.3`）
- **总上限**：info.log 最多 ~40 MB，error.log 最多 ~40 MB

### 轮转逻辑

写入前检查当前日志文件大小：
1. 如果 `file_size >= MAX_LOG_SIZE`：
   - 删除最旧的备份 `info.log.3`（如果存在）
   - `info.log.2` → `info.log.3`
   - `info.log.1` → `info.log.2`
   - `info.log` → `info.log.1`
2. 正常写入 `info.log`

### 改动文件

#### 1. `src/util/log.rs`（CLI 层）

- 新增常量：`MAX_LOG_SIZE`, `MAX_LOG_BACKUPS`
- 新增私有函数：`rotate_log_if_needed(log_dir: &Path, file_name: &str)`
- 在 `write_info_log` 和 `write_error_log` 的 `create_dir_all` 之后、打开文件之前调用 `rotate_log_if_needed`

#### 2. `j-agent/src/util/log.rs`（j-agent 层）

- 同样的改动，保持两份代码逻辑一致

### 代码示例（两个文件相同逻辑）

```rust
/// 单个日志文件最大大小（10 MB）
const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
/// 最大保留备份数
const MAX_LOG_BACKUPS: u32 = 3;

/// 如果日志文件超过阈值，执行轮转
fn rotate_log_if_needed(log_dir: &Path, file_name: &str) {
    let current = log_dir.join(file_name);
    // 检查当前文件大小
    let need_rotate = match fs::metadata(&current) {
        Ok(meta) => meta.len() >= MAX_LOG_SIZE,
        Err(_) => false, // 文件不存在，无需轮转
    };
    if !need_rotate {
        return;
    }
    // 从旧到新依次重命名: .2→.3, .1→.2, original→.1
    for i in (1..MAX_LOG_BACKUPS).rev() {
        let src = log_dir.join(format!("{}.{}", file_name, i));
        let dst = log_dir.join(format!("{}.{}", file_name, i + 1));
        let _ = fs::rename(&src, &dst);
    }
    // original → .1
    let backup = log_dir.join(format!("{}.1", file_name));
    let _ = fs::rename(&current, &backup);
}
```

然后在 `write_info_log` / `write_error_log` 中，在 `fs::create_dir_all(&log_dir)` 之后加入：

```rust
rotate_log_if_needed(&log_dir, AGENT_LOG_INFO);  // 或 AGENT_LOG_ERROR
```

## 优点

- **零依赖**：纯标准库实现
- **零侵入**：函数签名不变，所有调用点无需修改
- **安全**：轮转失败（rename 出错）静默忽略，不影响日志写入
- **对 TUI 窗口日志查看无影响**：`update_open_log_windows` 用 `tail -f info.log`，轮转后新文件可正常跟踪
