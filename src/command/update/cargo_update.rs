use crate::constants::VERSION;
use colored::Colorize;

use super::feature_select::select_features;
use super::restart::restart_self;

/// cargo 用户：直接执行 cargo install j-cli 更新
pub fn handle_cargo_update(check_only: bool, interactive: bool) {
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

    // 交互式选择 features
    println!();
    let selected_features = select_features();

    // 构建 cargo install 命令参数
    let mut args = vec!["install".to_string(), "j-cli".to_string()];
    if !selected_features.is_empty() {
        args.push("--features".to_string());
        args.push(selected_features.join(","));
    }

    let cmd_display = format!("cargo {}", args.join(" "));
    println!("{}", "正在通过 cargo 更新 j-cli...".yellow());
    println!("执行: {}", cmd_display.cyan());
    if !selected_features.is_empty() {
        println!("启用 Features: {}", selected_features.join(", ").green());
    }
    println!();

    // 检查 cargo 是否在 PATH 中
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let args_str: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    match std::process::Command::new(&cargo).args(&args_str).spawn() {
        Ok(mut child) => match child.wait() {
            Ok(status) if status.success() => {
                println!();
                // cargo 构建的二进制由链接器自动 ad-hoc 签名，无需再次 codesign
                // 再签反而会触发 "internal error in Code Signing subsystem"
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
