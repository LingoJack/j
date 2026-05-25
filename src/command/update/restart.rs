use colored::Colorize;

/// 用 execv 替换当前进程，实现无感知重启到新版本
pub(crate) fn restart_self() {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("{} {}", "无法获取当前可执行文件路径:".red(), e);
            println!("请手动重启 j 以使用新版本。");
            return;
        }
    };

    println!("{}", "正在重启 j 以加载新版本...".cyan());

    // Unix: 使用 execv 替换当前进程（进程号不变）
    #[cfg(unix)]
    {
        let exe_cstr = match std::ffi::CString::new(exe.to_string_lossy().as_bytes()) {
            Ok(s) => s,
            Err(e) => {
                println!("{} {}", "路径包含非法字符:".red(), e);
                println!("请手动重启 j 以使用新版本。");
                return;
            }
        };

        let err = nix::unistd::execv(&exe_cstr, &[&exe_cstr]);
        // execv 成功时不会返回；到这里说明失败了
        println!("{} {:?}", "重启失败:".red(), err);
        println!("请手动重启 j 以使用新版本。");
    }

    // Windows: 启动新进程后退出当前进程
    #[cfg(windows)]
    {
        match std::process::Command::new(&exe).spawn() {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                println!("{} {}", "重启失败:".red(), e);
                println!("请手动重启 j 以使用新版本。");
            }
        }
    }
}
