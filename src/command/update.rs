use crate::constants::{INSTALL_SOURCE, VERSION};
use colored::Colorize;

/// 处理 update 命令
pub fn handle_update(check_only: bool, interactive: bool) {
    match INSTALL_SOURCE {
        "github" => handle_github_update(check_only, interactive),
        "cargo" => handle_cargo_update(check_only, interactive),
        _ => show_unknown_source_hint(interactive),
    }
}

/// 从 GitHub Releases 更新
fn handle_github_update(check_only: bool, interactive: bool) {
    println!("{}", "检测到 GitHub Release 安装方式".green());
    println!("当前版本: {}", VERSION.cyan());

    if check_only {
        check_for_update();
    } else {
        perform_update(interactive);
    }
}

/// 检查是否有新版本
fn check_for_update() {
    println!("{}", "正在检查更新...".yellow());

    match self_update::backends::github::ReleaseList::configure()
        .repo_owner("LingoJack")
        .repo_name("j")
        .build()
    {
        Ok(release_list) => match release_list.fetch() {
            Ok(releases) => {
                if let Some(latest) = releases.first() {
                    let latest_version = latest.version.trim_start_matches('v');
                    println!("最新版本: {}", latest_version.cyan());

                    if latest_version == VERSION {
                        println!("{}", "已是最新版本".green());
                    } else {
                        println!("{}", "发现新版本！运行 'j update' 进行更新".yellow());
                    }
                } else {
                    println!("{}", "未找到发布版本".red());
                }
            }
            Err(e) => {
                println!("{} {}", "检查更新失败:".red(), e);
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新源失败:".red(), e);
        }
    }
}

/// 执行更新
fn perform_update(interactive: bool) {
    println!("{}", "正在更新...".yellow());

    let result = self_update::backends::github::Update::configure()
        .repo_owner("LingoJack")
        .repo_name("j")
        .bin_name("j")
        .show_download_progress(true)
        .current_version(VERSION)
        .build();

    match result {
        Ok(updater) => match updater.update() {
            Ok(status) => {
                println!(
                    "{} {}",
                    "更新成功！".green(),
                    format!("版本: {}", status.version()).cyan()
                );
                if interactive {
                    restart_self();
                }
            }
            Err(e) => {
                println!("{} {}", "更新失败:".red(), e);
                println!("请尝试手动更新:");
                println!(
                    "  curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh"
                );
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新失败:".red(), e);
        }
    }
}

/// cargo 用户：直接执行 cargo install j-cli 更新
fn handle_cargo_update(check_only: bool, interactive: bool) {
    println!("{}", "检测到 cargo 安装方式".green());
    println!("当前版本: {}", VERSION.cyan());

    if check_only {
        // --check 模式：只打印提示，不实际安装
        println!();
        println!("如需更新，运行:");
        println!("  {}", "j update".cyan());
        println!("  或: {}", "cargo install j-cli".cyan());
        return;
    }

    println!("{}", "正在通过 cargo 更新 j-cli...".yellow());
    println!("执行: {}", "cargo install j-cli".cyan());
    println!();

    // 检查 cargo 是否在 PATH 中
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

    match std::process::Command::new(&cargo)
        .args(["install", "j-cli"])
        .spawn()
    {
        Ok(mut child) => match child.wait() {
            Ok(status) if status.success() => {
                println!();
                println!("{}", "更新成功！".green());
                if interactive {
                    restart_self();
                }
            }
            Ok(status) => {
                println!();
                println!(
                    "{} 退出码: {}",
                    "更新失败".red(),
                    status.code().unwrap_or(-1)
                );
            }
            Err(e) => {
                println!("{} {}", "等待 cargo 执行失败:".red(), e);
            }
        },
        Err(e) => {
            println!("{} {}", "启动 cargo 失败:".red(), e);
            println!("请确认 cargo 已安装并在 PATH 中，或手动运行:");
            println!("  {}", "cargo install j-cli --force".cyan());
        }
    }
}

/// 未知安装来源：尝试 cargo，失败则给出手动提示
fn show_unknown_source_hint(interactive: bool) {
    println!("{}", "无法确定安装来源，尝试通过 cargo 更新...".yellow());
    handle_cargo_update(false, interactive);
}

/// 用 execv 替换当前进程，实现无感知重启到新版本
fn restart_self() {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("{} {}", "无法获取当前可执行文件路径:".red(), e);
            println!("请手动重启 j 以使用新版本。");
            return;
        }
    };

    println!("{}", "正在重启 j 以加载新版本...".cyan());

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
