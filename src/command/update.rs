use crate::constants::{INSTALL_SOURCE, VERSION};
use colored::Colorize;

/// 处理 update 命令
pub fn handle_update(check_only: bool) {
    match INSTALL_SOURCE {
        "github" => handle_github_update(check_only),
        "cargo" => show_cargo_update_hint(),
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

/// 提示 cargo 用户使用正确的更新方式
fn show_cargo_update_hint() {
    println!("{}", "检测到你通过 cargo 安装了 j-cli".yellow());
    println!();
    println!("请使用以下命令更新:");
    println!("  {}", "cargo install j-cli".cyan());
    println!();
    println!("或从 GitHub 重新安装（切换为 GitHub 安装方式）:");
    println!(
        "  {}",
        "curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh".cyan()
    );
}

/// 未知安装来源的提示
fn show_unknown_source_hint() {
    println!("{}", "无法确定安装来源".yellow());
    println!();
    println!("请选择以下方式更新:");
    println!();
    println!("1. cargo 方式:");
    println!("   {}", "cargo install j-cli".cyan());
    println!();
    println!("2. GitHub Release 方式:");
    println!(
        "   {}",
        "curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh".cyan()
    );
}
