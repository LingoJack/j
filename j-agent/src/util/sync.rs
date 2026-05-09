use crate::util::log::write_error_log;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

/// 安全获取 Mutex 锁：如果锁被毒化（另一个线程持锁时 panic），
/// 自动恢复并记录错误日志，而不是直接 panic
pub fn safe_lock<'a, T>(mutex: &'a Mutex<T>, context: &str) -> MutexGuard<'a, T> {
    mutex.lock().unwrap_or_else(|poisoned| {
        write_error_log(
            "safe_lock",
            &format!("Mutex poisoned at [{}], recovering", context),
        );
        poisoned.into_inner()
    })
}

/// 基于独立文件的互斥守卫
///
/// 通过创建 `.lock` 文件实现进程间互斥，Drop 时自动删除释放。
/// 使用 `File::create_new()` 原子操作，跨平台无兼容问题。
///
/// # Examples
///
/// ```ignore
/// let lock_path = config_path.with_extension("yaml.lock");
/// let _guard = LockFileGuard::acquire(&lock_path)?;
/// // 临界区：此时独占访问
/// fs::write(&config_path, content)?;
/// // _guard drop 时自动删除 lock 文件
/// ```
pub struct LockFileGuard {
    path: PathBuf,
}

impl LockFileGuard {
    /// 获取文件锁：创建 lock 文件，已存在则等待重试
    ///
    /// - `lock_path`: lock 文件路径（如 `config.yaml.lock`）
    ///
    /// 最多重试 20 次，每次间隔 50ms（总计约 1 秒）。
    pub fn acquire(lock_path: &Path) -> Result<Self, String> {
        const MAX_RETRIES: u32 = 20;
        const RETRY_DELAY_MS: u64 = 50;

        for attempt in 0..MAX_RETRIES {
            // create_new：文件已存在则返回 Err，否则原子创建，跨平台一致
            match std::fs::File::create_new(lock_path) {
                Ok(_) => {
                    return Ok(Self {
                        path: lock_path.to_path_buf(),
                    });
                }
                Err(_) if attempt < MAX_RETRIES - 1 => {
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                }
                Err(e) => {
                    return Err(format!("文件被占用，请稍后重试: {}", e));
                }
            }
        }
        unreachable!()
    }
}

impl Drop for LockFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}
