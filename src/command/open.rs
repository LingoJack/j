use crate::config::YamlConfig;
use crate::constants::{
    DEFAULT_SEARCH_ENGINE, NEW_WINDOW_FLAG, NEW_WINDOW_FLAG_LONG, config_key, search_engine,
    section, shell,
};
use crate::{error, info};
use std::path::Path;
use std::process::Command;

/// 通过别名打开应用/文件/URL
/// args[0] = alias, args[1..] = 额外参数
pub fn handle_open(args: &[String], config: &YamlConfig) {
    if args.is_empty() {
        error!("✖️ 请指定要打开的别名");
        return;
    }

    let alias = &args[0];

    // 检查别名是否存在
    if !config.alias_exists(alias) {
        error!(
            "✖️ 无法找到别名对应的路径或网址 {{{}}}。请检查配置文件。",
            alias
        );
        return;
    }

    // 如果是浏览器
    if config.contains(section::BROWSER, alias) {
        handle_open_browser(args, config);
        return;
    }

    // 如果是编辑器
    if config.contains(section::EDITOR, alias) {
        if args.len() == 2 {
            let file_path = &args[1];
            open_with_path(alias, Some(file_path), config);
        } else {
            open_alias(alias, config);
        }
        return;
    }

    // 如果是 VPN
    if config.contains(section::VPN, alias) {
        open_alias(alias, config);
        return;
    }

    // 如果是自定义脚本
    if config.contains(section::SCRIPT, alias) {
        run_script(args, config);
        return;
    }

    // 默认作为普通路径打开（支持带参数执行 CLI 工具）
    open_alias_with_args(alias, &args[1..], config);
}

/// 打开浏览器，可能带 URL 参数
fn handle_open_browser(args: &[String], config: &YamlConfig) {
    let alias = &args[0];
    if args.len() == 1 {
        // 直接打开浏览器
        open_alias(alias, config);
    } else {
        // 用法: j <browser_alias> <url_alias_or_search_words...> [engine]
        // 将 args[1..] 中的参数按顺序尝试匹配:
        //   1. inner_url / outer_url 别名 → 直接用对应 URL
        //   2. URL 格式 (http/https) → 直接用
        //   3. 全部拼接到搜索引擎查询

        // 先尝试用第一个参数匹配 URL 别名或直接 URL
        let first_arg = &args[1];

        let url = if let Some(u) = config.get_property(section::INNER_URL, first_arg) {
            u.clone()
        } else if let Some(u) = config.get_property(section::OUTER_URL, first_arg) {
            // outer_url 需要先启动 VPN
            if let Some(vpn_map) = config.get_section(section::VPN)
                && let Some(vpn_alias) = vpn_map.keys().next()
            {
                open_alias(vpn_alias, config);
            }
            u.clone()
        } else if is_url_like(first_arg) {
            // 直接是 URL
            first_arg.clone()
        } else {
            // 不匹配别名或 URL → 把 args[1..] 拼接为搜索词
            // 检查最后一个参数是否为搜索引擎名称
            let (query_parts, engine) = resolve_search_query_and_engine(&args[1..], config);
            let query = query_parts.join("+");
            get_search_url(&query, engine)
        };

        open_with_path(alias, Some(&url), config);
    }
}

/// 从参数列表中分离搜索词和搜索引擎
/// 策略：最后一个参数如果是已知的搜索引擎名(google/bing/baidu)，则作为引擎；
/// 否则全部作为搜索词，使用默认引擎。
fn resolve_search_query_and_engine<'a>(
    args: &'a [String],
    config: &YamlConfig,
) -> (Vec<&'a str>, &'a str) {
    if args.len() >= 2 {
        let last = args.last().expect("args non-empty");
        if is_known_engine(last) {
            return (
                args[..args.len() - 1].iter().map(|s| s.as_str()).collect(),
                last.as_str(),
            );
        }
    }

    // 全部作为搜索词，使用默认引擎
    let default_engine = config
        .get_property(section::SETTING, config_key::SEARCH_ENGINE)
        .map(|s| s.as_str())
        .unwrap_or(DEFAULT_SEARCH_ENGINE);
    (args.iter().map(|s| s.as_str()).collect(), default_engine)
}

/// 判断字符串是否为已知的搜索引擎名称
fn is_known_engine(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "google" | "bing" | "baidu")
}

/// 运行脚本
/// 支持 -w / --new-window 标志：在新终端窗口中执行脚本
/// 用法：j <script_alias> [-w] [args...]
fn run_script(args: &[String], config: &YamlConfig) {
    let alias = &args[0];
    if let Some(script_path) = config.get_property(section::SCRIPT, alias) {
        // 展开脚本路径中的 ~
        let script_path = clean_path(script_path);

        // 检测 -w / --new-window 标志，并从参数中过滤掉
        let new_window = args[1..]
            .iter()
            .any(|s| s == NEW_WINDOW_FLAG || s == NEW_WINDOW_FLAG_LONG);
        let script_args: Vec<String> = args[1..]
            .iter()
            .filter(|s| s.as_str() != NEW_WINDOW_FLAG && s.as_str() != NEW_WINDOW_FLAG_LONG)
            .map(|s| clean_path(s))
            .collect();
        let script_arg_refs: Vec<&str> = script_args.iter().map(|s| s.as_str()).collect();

        if new_window {
            info!("⚙️ 即将在新窗口执行脚本，路径: {}", script_path);
            run_script_in_new_window(&script_path, &script_arg_refs, config);
        } else {
            info!("⚙️ 即将执行脚本，路径: {}", script_path);
            run_script_in_current_terminal(&script_path, &script_arg_refs, config);
        }
    }
}

/// 为 Command 注入别名路径环境变量
fn inject_alias_envs(cmd: &mut Command, config: &YamlConfig) {
    for (key, value) in config.collect_alias_envs() {
        cmd.env(&key, &value);
    }
}

/// 在当前终端直接执行脚本
fn run_script_in_current_terminal(script_path: &str, script_args: &[&str], config: &YamlConfig) {
    let result = if cfg!(target_os = "windows") {
        let mut cmd = Command::new("cmd.exe");
        cmd.arg("/c").arg(script_path).args(script_args);
        inject_alias_envs(&mut cmd, config);
        cmd.status()
    } else {
        // macOS / Linux: 使用 sh 直接执行
        let mut cmd = Command::new("sh");
        cmd.arg(script_path).args(script_args);
        inject_alias_envs(&mut cmd, config);
        cmd.status()
    };

    match result {
        Ok(status) => {
            if status.success() {
                info!("☑️ 脚本执行完成");
            } else {
                error!("✖️ 脚本执行失败，退出码: {}", status);
            }
        }
        Err(e) => error!("💥 执行脚本失败: {}", e),
    }
}

/// 在新终端窗口中执行脚本
/// 脚本自身决定是否包含等待按键逻辑（通过 TUI 编辑器创建时可预填模板）
/// 脚本执行完后自动 exit 关闭 shell，使新窗口可被关闭
fn run_script_in_new_window(script_path: &str, script_args: &[&str], config: &YamlConfig) {
    let os = std::env::consts::OS;

    // 构建环境变量导出语句（用于新窗口中注入）
    let env_exports = build_env_export_string(config);

    if os == shell::MACOS_OS {
        // macOS: 使用 osascript 在新 Terminal 窗口中执行
        // 末尾追加 ; exit 让 shell 退出，Terminal.app 会根据偏好设置自动关闭窗口
        let script_cmd = if script_args.is_empty() {
            format!("sh {}", shell_escape(script_path))
        } else {
            let args_str = script_args
                .iter()
                .map(|a| shell_escape(a))
                .collect::<Vec<_>>()
                .join(" ");
            format!("sh {} {}", shell_escape(script_path), args_str)
        };

        let full_cmd = if env_exports.is_empty() {
            format!("{}; exit", script_cmd)
        } else {
            format!("{} {}; exit", env_exports, script_cmd)
        };

        // AppleScript: 在 Terminal.app 中打开新窗口并执行命令
        let apple_script = format!(
            "tell application \"Terminal\"\n\
                activate\n\
                do script \"{}\"\n\
            end tell",
            full_cmd.replace('\\', "\\\\").replace('"', "\\\"")
        );

        let result = Command::new("osascript")
            .arg("-e")
            .arg(&apple_script)
            .status();

        match result {
            Ok(status) => {
                if status.success() {
                    info!("☑️ 已在新终端窗口中启动脚本");
                } else {
                    error!("✖️ 启动新终端窗口失败，退出码: {}", status);
                }
            }
            Err(e) => error!("💥 调用 osascript 失败: {}", e),
        }
    } else if os == shell::WINDOWS_OS {
        // Windows: 使用 start cmd /c 在新窗口执行
        let script_cmd = if script_args.is_empty() {
            script_path.to_string()
        } else {
            format!("{} {}", script_path, script_args.join(" "))
        };

        // Windows 通过 set 命令设置环境变量
        let full_cmd = if env_exports.is_empty() {
            script_cmd
        } else {
            format!("{} && {}", env_exports, script_cmd)
        };

        let result = Command::new("cmd")
            .args(["/c", "start", "cmd", "/c", &full_cmd])
            .status();

        match result {
            Ok(status) => {
                if status.success() {
                    info!("☑️ 已在新终端窗口中启动脚本");
                } else {
                    error!("✖️ 启动新终端窗口失败，退出码: {}", status);
                }
            }
            Err(e) => error!("💥 启动新窗口失败: {}", e),
        }
    } else {
        // Linux: 尝试常见的终端模拟器
        // 末尾追加 ; exit 让 shell 退出，终端模拟器会自动关闭窗口
        let script_cmd = if script_args.is_empty() {
            format!("sh {}", script_path)
        } else {
            format!("sh {} {}", script_path, script_args.join(" "))
        };

        let full_cmd = if env_exports.is_empty() {
            format!("{}; exit", script_cmd)
        } else {
            format!("{} {}; exit", env_exports, script_cmd)
        };

        // 尝试 gnome-terminal → xterm → 降级到当前终端
        let terminals = [
            ("gnome-terminal", vec!["--", "sh", "-c", &full_cmd]),
            ("xterm", vec!["-e", &full_cmd]),
            ("konsole", vec!["-e", &full_cmd]),
        ];

        for (term, term_args) in &terminals {
            if let Ok(status) = Command::new(term).args(term_args).status()
                && status.success()
            {
                info!("☑️ 已在新终端窗口中启动脚本");
                return;
            }
        }

        // 所有终端都失败，降级到当前终端执行
        info!("⚠️ 未找到可用的终端模拟器，降级到当前终端执行");
        run_script_in_current_terminal(script_path, script_args, config);
    }
}

/// 构建环境变量导出字符串（用于新窗口执行时注入）
/// macOS/Linux 格式: export J_CHROME='/Applications/Google Chrome.app'; export J_VSCODE=...
/// Windows 格式: set "J_CHROME=/Applications/Google Chrome.app" && set "J_VSCODE=..."
fn build_env_export_string(config: &YamlConfig) -> String {
    let envs = config.collect_alias_envs();
    if envs.is_empty() {
        return String::new();
    }

    let os = std::env::consts::OS;
    if os == shell::WINDOWS_OS {
        envs.iter()
            .map(|(k, v)| format!("set \"{}={}\"", k, v))
            .collect::<Vec<_>>()
            .join(" && ")
    } else {
        // 修复：统一对所有值使用单引号包裹，避免特殊字符（&!|等）导致 shell 解析错误
        // 单引号内所有字符都按字面值处理，包括空格、&、!、| 等
        envs.iter()
            .map(|(k, v)| {
                // 对值中的单引号进行转义：' → '\''
                let escaped_value = v.replace('\'', "'\\''");
                format!("export {}='{}';", k, escaped_value)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Shell 参数转义（为包含空格等特殊字符的参数添加引号）
fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('"') || s.contains('\'') || s.contains('\\') {
        // 用单引号包裹，内部单引号转义为 '\'''
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}

/// 打开一个别名对应的路径（不带额外参数）
fn open_alias(alias: &str, config: &YamlConfig) {
    open_alias_with_args(alias, &[], config);
}

/// 打开一个别名对应的路径，支持传递额外参数
/// 自动判断路径类型：
/// - CLI 可执行文件 → 在当前终端用 Command::new() 执行（stdin/stdout 继承，支持管道）
/// - GUI 应用 (.app) / 其他文件 → 系统 open 命令打开
fn open_alias_with_args(alias: &str, extra_args: &[String], config: &YamlConfig) {
    if let Some(path) = config.get_path_by_alias(alias) {
        let path = clean_path(path);
        // 展开参数中的 ~
        let expanded_args: Vec<String> = extra_args.iter().map(|s| clean_path(s)).collect();
        if is_cli_executable(&path) {
            // CLI 工具：在当前终端直接执行，继承 stdin/stdout（管道可用）
            let result = Command::new(&path).args(&expanded_args).status();
            match result {
                Ok(status) => {
                    if !status.success() {
                        error!("✖️ 执行 {{{}}} 失败，退出码: {}", alias, status);
                    }
                }
                Err(e) => error!("💥 执行 {{{}}} 失败: {}", alias, e),
            }
        } else {
            // GUI 应用或普通文件：系统 open 命令打开
            if extra_args.is_empty() {
                do_open(&path);
            } else {
                // GUI 应用带参数打开（如 open -a App file）
                let os = std::env::consts::OS;
                let result = if os == shell::MACOS_OS {
                    Command::new("open")
                        .args(["-a", &path])
                        .args(&expanded_args)
                        .status()
                } else if os == shell::WINDOWS_OS {
                    Command::new(shell::WINDOWS_CMD)
                        .args([shell::WINDOWS_CMD_FLAG, "start", "", &path])
                        .args(&expanded_args)
                        .status()
                } else {
                    Command::new("xdg-open").arg(&path).status()
                };
                if let Err(e) = result {
                    error!("💥 启动 {{{}}} 失败: {}", alias, e);
                    return;
                }
            }
            info!("☑️ 启动 {{{}}} : {{{}}}", alias, path);
        }
    } else {
        error!("✖️ 未找到别名对应的路径或网址: {}。请检查配置文件。", alias);
    }
}

/// 判断一个路径是否为 CLI 可执行文件（非 GUI 应用）
/// 规则：
/// - macOS 的 .app 目录 → 不是 CLI 工具，是 GUI 应用
/// - URL（http/https）→ 不是 CLI 工具
/// - 普通文件且具有可执行权限 → 是 CLI 工具
fn is_cli_executable(path: &str) -> bool {
    // URL 不是可执行文件
    if path.starts_with("http://") || path.starts_with("https://") {
        return false;
    }

    // macOS .app 目录是 GUI 应用
    if path.ends_with(".app") || path.contains(".app/") {
        return false;
    }

    let p = Path::new(path);

    // 文件必须存在且是普通文件（不是目录）
    if !p.is_file() {
        return false;
    }

    // 检查可执行权限（Unix）
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = p.metadata() {
            return metadata.permissions().mode() & 0o111 != 0;
        }
    }

    // Windows 上通过扩展名判断
    #[cfg(windows)]
    {
        if let Some(ext) = p.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            return matches!(ext.as_str(), "exe" | "cmd" | "bat" | "com");
        }
    }

    false
}

/// 使用指定应用打开某个文件/URL
fn open_with_path(alias: &str, file_path: Option<&str>, config: &YamlConfig) {
    if let Some(app_path) = config.get_property(section::PATH, alias) {
        let app_path = clean_path(app_path);
        let os = std::env::consts::OS;
        // 展开文件路径参数中的 ~
        let file_path_expanded = file_path.map(clean_path);
        let file_path = file_path_expanded.as_deref();

        let result = if os == shell::MACOS_OS {
            match file_path {
                Some(fp) => Command::new("open").args(["-a", &app_path, fp]).status(),
                None => Command::new("open").arg(&app_path).status(),
            }
        } else if os == shell::WINDOWS_OS {
            match file_path {
                Some(fp) => Command::new(shell::WINDOWS_CMD)
                    .args([shell::WINDOWS_CMD_FLAG, "start", "", &app_path, fp])
                    .status(),
                None => Command::new(shell::WINDOWS_CMD)
                    .args([shell::WINDOWS_CMD_FLAG, "start", "", &app_path])
                    .status(),
            }
        } else {
            error!("💥 当前操作系统不支持此功能: {}", os);
            return;
        };

        match result {
            Ok(_) => {
                let target = file_path.unwrap_or("");
                info!("☑️ 启动 {{{}}} {} : {{{}}}", alias, target, app_path);
            }
            Err(e) => error!("💥 启动 {} 失败: {}", alias, e),
        }
    } else {
        error!("✖️ 未找到别名对应的路径: {}。", alias);
    }
}

/// 跨平台 open 命令
fn do_open(path: &str) {
    let os = std::env::consts::OS;
    let result = if os == shell::MACOS_OS {
        Command::new("open").arg(path).status()
    } else if os == shell::WINDOWS_OS {
        Command::new(shell::WINDOWS_CMD)
            .args([shell::WINDOWS_CMD_FLAG, "start", "", path])
            .status()
    } else {
        // Linux fallback
        Command::new("xdg-open").arg(path).status()
    };

    if let Err(e) = result {
        crate::error!("💥 打开 {} 失败: {}", path, e);
    }
}

/// 清理路径：去除引号和转义符，展开 ~
fn clean_path(path: &str) -> String {
    let mut path = path.trim().to_string();

    // 去除两端引号
    if path.len() >= 2
        && ((path.starts_with('\'') && path.ends_with('\''))
            || (path.starts_with('"') && path.ends_with('"')))
    {
        path = path[1..path.len() - 1].to_string();
    }

    // 去除转义空格
    path = path.replace("\\ ", " ");

    // 展开 ~
    if path.starts_with('~')
        && let Some(home) = dirs::home_dir()
    {
        if path == "~" {
            path = home.to_string_lossy().to_string();
        } else if path.starts_with("~/") {
            path = format!("{}{}", home.to_string_lossy(), &path[1..]);
        }
    }

    path
}

/// 简单判断是否像 URL
fn is_url_like(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// 根据搜索引擎获取搜索 URL
fn get_search_url(query: &str, engine: &str) -> String {
    let pattern = match engine.to_lowercase().as_str() {
        "google" => search_engine::GOOGLE,
        "bing" => search_engine::BING,
        "baidu" => search_engine::BAIDU,
        _ => {
            info!(
                "未指定搜索引擎，使用默认搜索引擎：{}",
                DEFAULT_SEARCH_ENGINE
            );
            search_engine::BING
        }
    };
    pattern.replace("{}", query)
}
