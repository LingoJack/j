use crate::constants::{INSTALL_SOURCE, VERSION};
use colored::Colorize;

/// 处理 update 命令
pub fn handle_update(check_only: bool) {
    match INSTALL_SOURCE {
        "github" => handle_github_update(check_only),
        "cargo" => handle_cargo_update(check_only),
        _ => show_unknown_source_hint(),
    }
}

/// 从 GitHub Releases 更新
fn handle_github_update(check_only: bool) {
    println!("{}", "检测到 GitHub Release 安装方式".green());
    println!("当前版本: {}", VERSION.cyan());

    if check_only {
        check_for_update();
    } else {
        perform_update();
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
fn perform_update() {
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
fn handle_cargo_update(check_only: bool) {
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
fn show_unknown_source_hint() {
    println!("{}", "无法确定安装来源，尝试通过 cargo 更新...".yellow());
    handle_cargo_update(false);
}
