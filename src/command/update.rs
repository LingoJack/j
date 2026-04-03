use crate::constants::{INSTALL_SOURCE, VERSION};
use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use std::io::{self, Write};

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

    // 检查目标目录是否有写入权限
    let has_write_permission = exe_dir
        .metadata()
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false);

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

    if !has_write_permission || !can_actually_write {
        // 没有写入权限，需要使用 sudo
        println!(
            "{}",
            "需要管理员权限来更新 j（安装目录需要 root 权限）".yellow()
        );
        println!();
        println!("请使用以下命令之一更新：");
        println!();
        println!("  {} (推荐)", "sudo j update".cyan());
        println!(
            "  {}",
            "curl -fsSL https://raw.githubusercontent.com/LingoJack/j/main/install.sh | sh".cyan()
        );
        return;
    }

    let result = self_update::backends::github::Update::configure()
        .repo_owner("LingoJack")
        .repo_name("j")
        .bin_name("j")
        .show_download_progress(true)
        .current_version(VERSION)
        .target(target)
        .build();

    match result {
        Ok(updater) => match updater.update() {
            Ok(status) => {
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
                _ => {}
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
