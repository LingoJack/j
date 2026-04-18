use crate::constants::{INSTALL_SOURCE, VERSION};
use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use std::io::{self, Write};

/// 修复 macOS 上替换后的代码签名和隔离属性
/// 未签名的二进制文件在 Apple Silicon 上会被内核 SIGKILL
#[cfg(target_os = "macos")]
fn fix_codesign_and_quarantine(bin_path: &std::path::Path) {
    // 移除隔离属性（com.apple.quarantine）
    let _ = std::process::Command::new("xattr")
        .args(["-cr"])
        .arg(bin_path)
        .status();

    // 使用 ad-hoc 签名重签
    match std::process::Command::new("codesign")
        .args(["--force", "-s", "-"])
        .arg(bin_path)
        .status()
    {
        Ok(s) if s.success() => {
            // codesign 重新签名成功
        }
        _ => {
            println!(
                "{}",
                "  警告: codesign 签名失败，新版本可能无法启动"
                    .to_string()
                    .yellow()
            );
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn fix_codesign_and_quarantine(_bin_path: &std::path::Path) {}

/// 尝试获取 GitHub 认证 token
/// 优先级: GITHUB_TOKEN 环境变量 > gh auth token
fn get_github_auth_token() -> Option<String> {
    // 方法1: 检查 GITHUB_TOKEN 环境变量
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.is_empty()
    {
        return Some(token);
    }

    // 方法2: 尝试使用 gh auth token
    if let Ok(output) = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        && output.status.success()
    {
        let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !token.is_empty() {
            return Some(token);
        }
    }

    None
}

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

    let auth_token = get_github_auth_token();
    if auth_token.is_some() {
        println!("{}", "使用 GitHub 认证...".dimmed());
    }

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
                println!(
                    "  curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh"
                );
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

    // 根据当前架构确定 target 名称（匹配 GitHub Release 资产命名）
    // 资产命名格式: j-darwin-arm64.tar.gz, j-darwin-x64.tar.gz
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    let target = "darwin-arm64";

    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    let target = "darwin-x64";

    #[cfg(not(any(
        all(target_arch = "aarch64", target_os = "macos"),
        all(target_arch = "x86_64", target_os = "macos")
    )))]
    let target = {
        println!("{}", "当前平台暂不支持自动更新，请手动更新".red());
        return;
    };

    // 检查是否已经有 root 权限
    // SAFETY: libc::getuid() 是只读系统调用，无副作用，线程安全
    #[cfg(unix)]
    let is_root = unsafe { libc::getuid() == 0 };

    #[cfg(not(unix))]
    let is_root = false;

    // 如果已经有 root 权限，直接执行更新
    if is_root {
        perform_update_internal(target, interactive);
        return;
    }

    // 检查是否有权限写入目标目录
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("{} {}", "无法获取当前可执行文件路径:".red(), e);
            return;
        }
    };

    let exe_dir = match exe_path.parent() {
        Some(d) => d,
        None => {
            println!("{}", "无法获取可执行文件所在目录".red());
            return;
        }
    };

    // 尝试创建临时文件来验证实际的写入权限
    let can_actually_write = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(exe_dir.join(".j_write_test"))
        .map(|_| {
            let _ = std::fs::remove_file(exe_dir.join(".j_write_test"));
            true
        })
        .unwrap_or(false);

    if can_actually_write {
        // 有写入权限，直接执行更新
        perform_update_internal(target, interactive);
        return;
    }

    // 没有写入权限，需要使用 sudo 重新执行
    println!(
        "{}",
        "需要管理员权限来更新 j（安装目录需要 root 权限）".yellow()
    );
    println!("{}", "正在请求管理员权限...".cyan());

    // 使用 osascript 弹出图形化授权对话框
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

/// 内部更新逻辑（假设已有权限）
fn perform_update_internal(target: &str, interactive: bool) {
    let auth_token = get_github_auth_token();
    if auth_token.is_some() {
        println!("{}", "使用 GitHub 认证...".dimmed());
    }

    let mut binding = self_update::backends::github::Update::configure();
    let mut binding = binding
        .repo_owner("LingoJack")
        .repo_name("j")
        .bin_name("j")
        .show_download_progress(true)
        .current_version(VERSION)
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
                // 尝试同步安装 j-indicator
                install_indicator_from_release(status.version());
                if interactive {
                    restart_self();
                }
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("403") || err_str.contains("rate limit") {
                    println!("{} {}", "更新失败:".red(), e);
                    println!(
                        "{}",
                        "GitHub API 请求被限流，尝试使用 curl 方式更新...".yellow()
                    );
                    perform_update_curl(target, interactive);
                } else {
                    println!("{} {}", "更新失败:".red(), e);
                    println!("请尝试手动更新:");
                    println!(
                        "  curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh"
                    );
                }
            }
        },
        Err(e) => {
            println!("{} {}", "配置更新失败:".red(), e);
        }
    }
}

/// 可选 feature 列表
const OPTIONAL_FEATURES: &[(&str, &str)] = &[(
    "browser_cdp",
    "浏览器自动化 (CDP 模式，需本地有 Chrome/Chromium)",
)];

/// 计算菜单总行数
fn menu_total_lines() -> u16 {
    // 标题(1) + 空行(1) + features + 空行(1) + 确认按钮(1) + 空行(1) + 提示(1)
    (1 + 1 + OPTIONAL_FEATURES.len() + 1 + 1 + 1 + 1) as u16
}

/// 交互式 feature 选择界面（类似 Claude Code 风格）
/// 返回用户选中的 features 列表
fn select_features() -> Vec<String> {
    let mut selected = vec![false; OPTIONAL_FEATURES.len()];
    let mut cursor_pos: usize = 0;
    let mut is_first_draw = true;

    // 进入 raw 模式
    if terminal::enable_raw_mode().is_err() {
        return vec![];
    }

    let mut stdout = io::stdout();

    // 绘制初始界面
    let _ = draw_feature_menu(&mut stdout, &selected, cursor_pos, is_first_draw);
    is_first_draw = false;

    loop {
        if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if cursor_pos < OPTIONAL_FEATURES.len() {
                        cursor_pos += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    // 空格切换选中状态（仅在 feature 行上有效）
                    if cursor_pos < OPTIONAL_FEATURES.len() {
                        selected[cursor_pos] = !selected[cursor_pos];
                    }
                }
                KeyCode::Enter => {
                    // 如果光标在 "确认安装" 行上，直接确认
                    if cursor_pos == OPTIONAL_FEATURES.len() {
                        break;
                    }
                    // 在 feature 行上按 Enter 也切换选中
                    if cursor_pos < OPTIONAL_FEATURES.len() {
                        selected[cursor_pos] = !selected[cursor_pos];
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    // 取消：不选择任何 feature，直接跳到确认
                    break;
                }
                _ => {} // 忽略其他按键
            }
            let _ = draw_feature_menu(&mut stdout, &selected, cursor_pos, is_first_draw);
        }
    }

    // 退出 raw 模式
    let _ = terminal::disable_raw_mode();
    // 换行，避免后续输出接在同一行
    println!();

    // 收集选中的 features
    selected
        .iter()
        .enumerate()
        .filter(|(_, s)| **s)
        .map(|(i, _)| OPTIONAL_FEATURES[i].0.to_string())
        .collect()
}

/// 绘制 feature 选择菜单
fn draw_feature_menu(
    stdout: &mut io::Stdout,
    selected: &[bool],
    cursor_pos: usize,
    is_first_draw: bool,
) -> io::Result<()> {
    let total_lines = menu_total_lines();

    if !is_first_draw {
        // 非首次绘制：移回菜单起始位置
        execute!(stdout, cursor::MoveUp(total_lines))?;
    }
    // 从当前光标位置清除到屏幕底部
    execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown))?;

    // 标题
    // raw mode 下 \n 不会自动回到行首，需要使用 \r\n
    write!(
        stdout,
        "  {} {}\r\n",
        "?".cyan().bold(),
        "选择要启用的可选 Features:".bold()
    )?;
    write!(stdout, "\r\n")?;

    // Feature 列表
    for (i, (name, desc)) in OPTIONAL_FEATURES.iter().enumerate() {
        let is_focused = cursor_pos == i;
        let is_selected = selected[i];

        let checkbox = if is_selected {
            "◉".green().bold().to_string()
        } else {
            "○".dimmed().to_string()
        };

        let pointer = if is_focused { "❯" } else { " " };

        if is_focused {
            write!(
                stdout,
                "  {} {} {} {}\r\n",
                pointer.cyan().bold(),
                checkbox,
                name.cyan().bold(),
                format!("({})", desc).dimmed()
            )?;
        } else {
            write!(
                stdout,
                "  {} {} {} {}\r\n",
                pointer,
                checkbox,
                name,
                format!("({})", desc).dimmed()
            )?;
        }
    }

    // 空行
    write!(stdout, "\r\n")?;

    // 确认按钮
    let confirm_focused = cursor_pos == OPTIONAL_FEATURES.len();
    if confirm_focused {
        write!(
            stdout,
            "  {} {}\r\n",
            "❯".cyan().bold(),
            "确认安装".green().bold()
        )?;
    } else {
        write!(stdout, "    {}\r\n", "确认安装".dimmed())?;
    }

    // 操作提示
    write!(stdout, "\r\n")?;
    write!(
        stdout,
        "  {} ↑↓ 移动  {} 切换  {} 确认  {} 跳过\r\n",
        "•".dimmed(),
        "空格".dimmed(),
        "Enter".dimmed(),
        "Esc".dimmed()
    )?;

    stdout.flush()?;
    Ok(())
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

    // 交互式选择 features
    println!();
    let selected_features = select_features();

    // 构建 cargo install 命令参数
    let mut args = vec!["install", "j-cli"];
    let features_str;
    if !selected_features.is_empty() {
        features_str = selected_features.join(",");
        args.push("--features");
        args.push(&features_str);
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

    match std::process::Command::new(&cargo).args(&args).spawn() {
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

/// 未知安装来源：尝试 cargo，失败则给出手动提示
fn show_unknown_source_hint(interactive: bool) {
    println!("{}", "无法确定安装来源，尝试通过 cargo 更新...".yellow());
    handle_cargo_update(false, interactive);
}

/// 使用 curl 方式下载更新（当 self_update 因 API 限流失败时的后备方案）
/// 模仿 install.sh 的逻辑：先获取最新版本号，再下载 tarball
fn perform_update_curl(target: &str, interactive: bool) {
    // 获取最新版本号
    let version = get_latest_version_curl();
    let version_display = version.as_deref().unwrap_or("未知").to_string();
    println!("最新版本: {}", version_display.cyan());

    let version = match version {
        Some(v) => v,
        None => {
            println!("{}", "无法获取最新版本号".red());
            println!("请尝试手动更新:");
            println!(
                "  curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh"
            );
            return;
        }
    };

    // 确定 j 所在目录
    let exe_path = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            println!("{} {}", "无法获取当前可执行文件路径:".red(), e);
            return;
        }
    };
    let exe_dir = match exe_path.parent() {
        Some(d) => d.to_path_buf(),
        None => {
            println!("{}", "无法获取可执行文件所在目录".red());
            return;
        }
    };

    let tag = if version.starts_with('v') {
        version.clone()
    } else {
        format!("v{}", version)
    };

    let asset_name = format!("j-{}", target);
    let url = format!(
        "https://github.com/LingoJack/j/releases/download/{}/{}.tar.gz",
        tag, asset_name
    );

    println!("下载地址: {}", url.dimmed());

    // 创建临时目录
    let tmp_dir = std::env::temp_dir().join("j-update-curl");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let tmp_tar = tmp_dir.join(format!("{}.tar.gz", asset_name));

    // 用 curl 下载
    println!("{}", "正在下载...".yellow());
    let download = std::process::Command::new("curl")
        .args(["-fsSL", "--progress-bar", "-o"])
        .arg(&tmp_tar)
        .arg(&url)
        .status();

    match download {
        Ok(status) if status.success() => {}
        Ok(status) => {
            println!(
                "{} 退出码: {}",
                "下载失败".red(),
                status.code().unwrap_or(-1)
            );
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
        Err(e) => {
            println!("{} {}", "下载失败:".red(), e);
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
    }

    // 解压
    println!("{}", "正在解压...".yellow());
    let extract = std::process::Command::new("tar")
        .args(["-xzf"])
        .arg(&tmp_tar)
        .args(["-C"])
        .arg(&tmp_dir)
        .status();

    match extract {
        Ok(status) if status.success() => {}
        Ok(status) => {
            println!(
                "{} 退出码: {}",
                "解压失败".red(),
                status.code().unwrap_or(-1)
            );
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
        Err(e) => {
            println!("{} {}", "解压失败:".red(), e);
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
    }

    // 替换二进制文件
    let src_bin = tmp_dir.join("j");
    let dst_bin = exe_dir.join("j");

    if !src_bin.exists() {
        println!("{}", "解压后未找到 j 二进制文件".red());
        let _ = std::fs::remove_dir_all(&tmp_dir);
        return;
    }

    match std::fs::copy(&src_bin, &dst_bin) {
        Ok(_) => {
            // 设置可执行权限
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&dst_bin, std::fs::Permissions::from_mode(0o755));
            }
            // 修复 macOS 代码签名
            fix_codesign_and_quarantine(&dst_bin);
            println!(
                "{} {}",
                "更新成功！".green(),
                format!("版本: {}", version_display).cyan()
            );

            // 尝试同步安装 j-indicator
            install_indicator_from_release(&version);

            if interactive {
                restart_self();
            }
        }
        Err(e) => {
            println!("{} {}", "安装失败:".red(), e);
            println!("可能需要管理员权限，请尝试:");
            println!("  {}", "sudo j update".cyan());
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
}

/// 通过 curl 获取最新版本号（模仿 install.sh 的多种回退策略）
fn get_latest_version_curl() -> Option<String> {
    println!("{}", "正在获取最新版本号...".yellow());

    let auth_token = get_github_auth_token();

    // 方法1: 使用 GitHub API（带认证）
    let mut api_cmd = std::process::Command::new("curl");
    api_cmd.args(["-fsSL", "-H", "User-Agent: j-cli-updater"]);
    if let Some(ref token) = auth_token {
        api_cmd.args(["-H", &format!("Authorization: token {}", token)]);
    }
    api_cmd.arg("https://api.github.com/repos/LingoJack/j/releases/latest");

    if let Ok(output) = api_cmd.output()
        && output.status.success()
    {
        let body = String::from_utf8_lossy(&output.stdout);
        let re = regex::Regex::new(r#"v[0-9]+\.[0-9]+\.[0-9]+"#).ok();
        // 从 JSON 中提取 tag_name
        for line in body.lines() {
            if line.contains("\"tag_name\"")
                && let Some(re) = &re
                && let Some(m) = re.find(line)
            {
                return Some(m.as_str().to_string());
            }
        }
    }

    // 方法2: 从 releases 页面解析重定向
    if let Ok(output) = std::process::Command::new("curl")
        .args(["-fsSL", "-o", "/dev/null", "-w", "%{url_effective}"])
        .arg("https://github.com/LingoJack/j/releases/latest")
        .output()
        && output.status.success()
    {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // 重定向 URL 格式: https://github.com/LingoJack/j/releases/tag/v12.8.10
        let re = regex::Regex::new(r#"v[0-9]+\.[0-9]+\.[0-9]+"#).ok();
        if let Some(re) = re
            && let Some(m) = re.find(&url)
        {
            return Some(m.as_str().to_string());
        }
    }

    None
}

/// 从 GitHub Release 下载并安装 j-indicator 到 j 同目录
/// 这是 best-effort 的：失败只打印警告，不影响主更新
fn install_indicator_from_release(version: &str) {
    // 确定 j 所在目录
    let j_dir = match std::env::current_exe() {
        Ok(p) => match p.parent() {
            Some(dir) => dir.to_path_buf(),
            None => return,
        },
        Err(_) => return,
    };

    let tag = if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    };
    let url = format!(
        "https://github.com/LingoJack/j/releases/download/{}/j-darwin-arm64.tar.gz",
        tag
    );

    println!("{}", "正在安装 j-indicator...".yellow());

    // 下载到临时文件
    let tmp_dir = std::env::temp_dir().join("j-update-indicator");
    let _ = std::fs::create_dir_all(&tmp_dir);
    let tmp_tar = tmp_dir.join("j-darwin-arm64.tar.gz");

    // 用 curl 下载（macOS 自带）
    let download = std::process::Command::new("curl")
        .args(["-fsSL", "-o"])
        .arg(&tmp_tar)
        .arg(&url)
        .output();

    match download {
        Ok(output) if output.status.success() => {}
        _ => {
            println!(
                "{}",
                "  j-indicator 下载失败，跳过（不影响 j 主程序）".dimmed()
            );
            let _ = std::fs::remove_dir_all(&tmp_dir);
            return;
        }
    }

    // 从 tarball 中提取 j-indicator
    let extract = std::process::Command::new("tar")
        .args(["-xzf"])
        .arg(&tmp_tar)
        .args(["-C"])
        .arg(&tmp_dir)
        .arg("j-indicator")
        .output();

    match extract {
        Ok(output) if output.status.success() => {
            let src = tmp_dir.join("j-indicator");
            let dst = j_dir.join("j-indicator");
            if src.exists() {
                match std::fs::copy(&src, &dst) {
                    Ok(_) => {
                        // 设置可执行权限
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let _ = std::fs::set_permissions(
                                &dst,
                                std::fs::Permissions::from_mode(0o755),
                            );
                        }
                        // 修复 macOS 代码签名
                        fix_codesign_and_quarantine(&dst);
                        println!("{}", "  j-indicator 已安装".green());
                    }
                    Err(e) => {
                        println!(
                            "{}",
                            format!("  j-indicator 拷贝失败: {}（不影响 j 主程序）", e).dimmed()
                        );
                    }
                }
            }
        }
        _ => {
            println!(
                "{}",
                "  j-indicator 提取失败，跳过（不影响 j 主程序）".dimmed()
            );
        }
    }

    let _ = std::fs::remove_dir_all(&tmp_dir);
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
