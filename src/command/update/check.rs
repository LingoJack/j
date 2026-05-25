use crate::constants::VERSION;
use colored::Colorize;

use super::github_auth::{get_github_auth_token, print_auth_hint};

/// 检查是否有新版本
pub fn check_for_update() {
    println!("{}", "正在检查更新...".yellow());

    let auth_token = get_github_auth_token();
    print_auth_hint(&auth_token);

    let mut binding = self_update::backends::github::ReleaseList::configure();
    let mut binding = binding.repo_owner("LingoJack").repo_name("j");

    if let Some(ref token) = auth_token {
        binding = binding.auth_token(token);
    }

    match binding.build() {
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
        },
        Err(e) => {
            println!("{} {}", "配置更新源失败:".red(), e);
        }
    }
}
