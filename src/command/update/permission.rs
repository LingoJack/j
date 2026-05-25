use colored::Colorize;

/// 检查是否有写入权限并返回 (exe_path, exe_dir)
/// 返回 None 表示无法获取路径信息
pub(crate) fn get_exe_dir() -> Option<(std::path::PathBuf, std::path::PathBuf)> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_dir = exe_path.parent()?.to_path_buf();
    Some((exe_path, exe_dir))
}

/// 尝试创建临时文件来验证实际的写入权限
pub(crate) fn can_write_to_dir(exe_dir: &std::path::Path) -> bool {
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(exe_dir.join(".j_write_test"))
        .map(|_| {
            let _ = std::fs::remove_file(exe_dir.join(".j_write_test"));
            true
        })
        .unwrap_or(false)
}

/// 检查是否已经有 root 权限
#[cfg(unix)]
pub(crate) fn is_root() -> bool {
    // SAFETY: libc::getuid() 是只读系统调用，无副作用，线程安全
    unsafe { libc::getuid() == 0 }
}

#[cfg(not(unix))]
pub(crate) fn is_root() -> bool {
    false
}

/// macOS: 使用 osascript 弹出图形化授权对话框执行更新
#[cfg(target_os = "macos")]
pub(crate) fn elevate_with_osascript(exe_path: &std::path::Path) {
    println!(
        "{}",
        "需要管理员权限来更新 j（安装目录需要 root 权限）".yellow()
    );
    println!("{}", "正在请求管理员权限...".cyan());

    let exe_str = exe_path.to_string_lossy();
    let script = format!(
        r#"do shell script "{} update" with administrator privileges"#,
        exe_str
    );

    let result = std::process::Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .status();

    match result {
        Ok(status) if status.success() => {
            println!("{}", "更新完成！".green());
        }
        Ok(status) => {
            println!(
                "{} 退出码: {}",
                "更新失败".red(),
                status.code().unwrap_or(-1)
            );
            println!("请尝试手动更新:");
            println!("  {}", "sudo j update".cyan());
        }
        Err(e) => {
            println!("{} {}", "请求权限失败:".red(), e);
            println!("请尝试手动更新:");
            println!("  {}", "sudo j update".cyan());
        }
    }
}

/// Windows: 提示管理员权限
#[cfg(target_os = "windows")]
pub(crate) fn print_windows_elevation_hint() {
    println!("{}", "需要管理员权限来更新 j".yellow());
    println!("请以管理员身份运行: {}", "j update".cyan());
}

/// Linux: 提示 sudo
#[cfg(all(unix, not(target_os = "macos")))]
pub(crate) fn print_linux_elevation_hint() {
    println!(
        "{}",
        "需要管理员权限来更新 j（安装目录需要 root 权限）".yellow()
    );
    println!("请尝试手动更新:");
    println!("  {}", "sudo j update".cyan());
}
