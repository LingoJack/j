use crate::util::log::write_error_log;
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
