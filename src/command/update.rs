//! j-cli 更新模块
//!
//! 根据 INSTALL_SOURCE 环境变量自动选择更新方式：
//! - `cargo`: cargo install 更新
//! - `github`: GitHub Release 下载更新
//! - 其他: 显示手动更新提示

mod cargo_update;
mod check;
mod codesign;
mod fallback;
mod feature_select;
mod github_auth;
mod github_update;
mod indicator;
mod permission;
mod restart;

use colored::Colorize;

pub use cargo_update::handle_cargo_update;
pub use check::check_for_update;

/// INSTALL_SOURCE 环境变量值
const INSTALL_SOURCE_GITHUB: &str = "github";
const INSTALL_SOURCE_CARGO: &str = "cargo";

/// 根据 INSTALL_SOURCE 分发更新
pub fn handle_update(check_only: bool, interactive: bool) {
    let source = std::env::var("INSTALL_SOURCE").unwrap_or_default();

    match source.as_str() {
        INSTALL_SOURCE_GITHUB => handle_github_update(check_only, interactive),
        INSTALL_SOURCE_CARGO => handle_cargo_update(check_only, interactive),
        _ => show_unknown_source_hint(),
    }
}

/// GitHub Release 更新入口
fn handle_github_update(check_only: bool, interactive: bool) {
    println!("{}", "检测到 GitHub Release 安装方式".green());
    println!("当前版本: {}", crate::constants::VERSION.cyan());

    if check_only {
        check_for_update();
        return;
    }

    github_update::perform_update(interactive);
}

/// 显示未知安装来源的更新提示
fn show_unknown_source_hint() {
    println!("{}", "未检测到安装来源，请手动更新:".yellow());
    #[cfg(unix)]
    println!(
        "  {}",
        "curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh".cyan()
    );
    #[cfg(windows)]
    println!(
        "  {}",
        "irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex".cyan()
    );
    println!();
    println!("或使用 cargo 安装:");
    println!("  {}", "cargo install j-cli".cyan());
}
