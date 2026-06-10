use colored::Colorize;

use super::codesign::fix_codesign_and_quarantine;
use super::fallback::perform_update_fallback;
use super::github_auth::{get_github_auth_token, print_auth_hint};
#[cfg(target_os = "macos")]
use super::indicator::install_indicator_from_release;
use super::permission::{can_write_to_dir, get_exe_dir, is_root};
use super::restart::restart_self;

/// 执行更新（权限检测与提升）
pub(crate) fn perform_update(interactive: bool) {
    println!("{}", "正在更新...".yellow());

    // 根据当前架构确定 target 名称（匹配 GitHub Release 资产命名）
    // 资产命名格式: j-darwin-arm64.tar.gz, j-darwin-x64.tar.gz
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    let target = "darwin-arm64";

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    let target = "darwin-x64";

    #[cfg(all(target_arch = "x86_64", target_os = "windows"))]
    let target = "windows-x64";

    #[cfg(all(target_arch = "aarch64", target_os = "windows"))]
    let target = "windows-arm64";

    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    let target = "linux-x64";

    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    let target = "linux-arm64";

    #[cfg(not(any(
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "windows"),
        all(target_arch = "aarch64", target_os = "windows"),
        all(target_arch = "x86_64", target_os = "linux"),
        all(target_arch = "aarch64", target_os = "linux")
    )))]
    {
        println!("{}", "当前平台暂不支持自动更新，请手动更新".red());
        return;
    }

    // 如果已经有 root 权限，直接执行更新
    if is_root() {
        perform_update_internal(target, interactive);
        return;
    }

    // 检查是否有权限写入目标目录
    let Some((_exe_path, exe_dir)) = get_exe_dir() else {
        println!("{}", "无法获取当前可执行文件路径或目录".red());
        return;
    };

    if can_write_to_dir(&exe_dir) {
        // 有写入权限，直接执行更新
        perform_update_internal(target, interactive);
        return;
    }

    // 没有写入权限，需要提升权限
    #[cfg(target_os = "macos")]
    super::permission::elevate_with_osascript(&exe_path);

    #[cfg(target_os = "windows")]
    super::permission::print_windows_elevation_hint();

    #[cfg(all(unix, not(target_os = "macos")))]
    super::permission::print_linux_elevation_hint();
}

/// 内部更新逻辑（假设已有权限）
fn perform_update_internal(target: &str, interactive: bool) {
    let auth_token = get_github_auth_token();
    print_auth_hint(&auth_token);

    let mut binding = self_update::backends::github::Update::configure();
    let mut binding = binding
        .repo_owner("LingoJack")
        .repo_name("jcli")
        .bin_name("j")
        .show_download_progress(true)
        .current_version(crate::constants::VERSION)
        .target(target);

    if let Some(ref token) = auth_token {
        binding = binding.auth_token(token);
    }

    match binding.build() {
        Ok(updater) => match updater.update() {
            Ok(status) => {
                // 修复 macOS 代码签名（替换后的二进制文件需要重签）
                if let Ok(exe_path) = std::env::current_exe() {
                    fix_codesign_and_quarantine(&exe_path);
                }
                println!(
                    "{} {}",
                    "更新成功！".green(),
                    format!("版本: {}", status.version()).cyan()
                );
                // 尝试同步安装 j-indicator（仅 macOS）
                #[cfg(target_os = "macos")]
                install_indicator_from_release(status.version());
                if interactive {
                    restart_self();
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                // 多种错误情况都尝试使用备用方案更新
                let should_fallback = err_str.contains("403")
                    || err_str.contains("rate limit")
                    || err_str.contains("decoding")
                    || err_str.contains("IoError")
                    || err_str.contains("certificate")
                    || err_str.contains("ssl")
                    || err_str.contains("TLS")
                    || err_str.contains("connection")
                    || err_str.contains("ReqwestError")
                    || err_str.contains("sending request");

                if should_fallback {
                    println!("{} {}", "更新失败:".red(), e);
                    println!("{}", "尝试使用备用方式更新...".yellow());
                    perform_update_fallback(target, interactive);
                } else {
                    println!("{} {}", "更新失败:".red(), e);
                    println!("请尝试手动更新:");
                    #[cfg(unix)]
                    println!(
                        "  curl -fsSL https://raw.githubusercontent.com/LingoJack/jcli/main/install.sh | sh"
                    );
                    #[cfg(windows)]
                    println!(
                        "  irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex"
                    );
                }
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新失败:".red(), e);
        }
    }
}
