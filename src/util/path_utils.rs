/// 展开 ~ 为 HOME 目录路径：`~` → HOME，`~/foo` → HOME/foo，其他不变
pub fn expand_tilde(path: &str) -> String {
    if path == "~" {
        std::env::var("HOME").unwrap_or_else(|_| "~".to_string())
    } else if let Some(rest) = path.strip_prefix("~/") {
        match std::env::var("HOME") {
            Ok(home) => format!("{}/{}", home, rest),
            Err(_) => path.to_string(),
        }
    } else {
        path.to_string()
    }
}

/// 解析路径：展开 ~，若路径为相对路径且当前线程有 worktree CWD，则相对于该 CWD 解析。
pub fn resolve_path(path: &str) -> String {
    let expanded = expand_tilde(path);
    if std::path::Path::new(&expanded).is_absolute() {
        return expanded;
    }
    if let Some(cwd) = crate::command::chat::teammate::thread_cwd() {
        return cwd.join(&expanded).to_string_lossy().to_string();
    }
    expanded
}

/// 获取当前有效的工作目录：先取线程本地 worktree CWD，再 fallback 到进程 CWD
pub fn effective_cwd() -> String {
    if let Some(cwd) = crate::command::chat::teammate::thread_cwd() {
        return cwd.to_string_lossy().to_string();
    }
    std::env::current_dir()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_else(|_| ".".to_string())
}
