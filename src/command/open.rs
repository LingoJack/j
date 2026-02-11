use crate::config::YamlConfig;
use crate::constants::{section, config_key, search_engine, DEFAULT_SEARCH_ENGINE};
use crate::{error, info};
use std::process::Command;

/// 通过别名打开应用/文件/URL
/// args[0] = alias, args[1..] = 额外参数
pub fn handle_open(args: &[String], config: &YamlConfig) {
    if args.is_empty() {
        error!("❌ 请指定要打开的别名");
        return;
    }

    let alias = &args[0];

    // 检查别名是否存在
    if !config.alias_exists(alias) {
        error!("❌ 无法找到别名对应的路径或网址 {{{}}}。请检查配置文件。", alias);
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

    // 默认作为普通路径打开
    open_alias(alias, config);
}

/// 打开浏览器，可能带 URL 参数
fn handle_open_browser(args: &[String], config: &YamlConfig) {
    let alias = &args[0];
    if args.len() == 1 {
        // 直接打开浏览器
        open_alias(alias, config);
    } else {
        // j <browser_alias> <url_alias_or_search_text> [engine]
        let url_alias_or_text = &args[1];

        // 尝试从 inner_url 或 outer_url 获取 URL
        let url = if let Some(u) = config.get_property(section::INNER_URL, url_alias_or_text) {
            u.clone()
        } else if let Some(u) = config.get_property(section::OUTER_URL, url_alias_or_text) {
            // outer_url 需要先启动 VPN
            if let Some(vpn_map) = config.get_section(section::VPN) {
                if let Some(vpn_alias) = vpn_map.keys().next() {
                    open_alias(vpn_alias, config);
                }
            }
            u.clone()
        } else if is_url_like(url_alias_or_text) {
            // 直接是 URL
            url_alias_or_text.clone()
        } else {
            // 搜索引擎搜索
            let engine = if args.len() >= 3 {
                args[2].as_str()
            } else {
                config
                    .get_property(section::SETTING, config_key::SEARCH_ENGINE)
                    .map(|s| s.as_str())
                    .unwrap_or(DEFAULT_SEARCH_ENGINE)
            };
            get_search_url(url_alias_or_text, engine)
        };

        open_with_path(alias, Some(&url), config);
    }
}

/// 运行脚本
fn run_script(args: &[String], config: &YamlConfig) {
    let alias = &args[0];
    if let Some(script_path) = config.get_property(section::SCRIPT, alias) {
        info!("⚙️ 即将执行脚本，路径: {}", script_path);
        let script_args: Vec<&str> = args[1..].iter().map(|s| s.as_str()).collect();

        // 在当前终端直接执行脚本（而非打开新终端窗口）
        let result = if cfg!(target_os = "windows") {
            Command::new("cmd.exe")
                .arg("/c")
                .arg(script_path.as_str())
                .args(&script_args)
                .status()
        } else {
            // macOS / Linux: 使用 sh 直接执行
            Command::new("sh")
                .arg(script_path.as_str())
                .args(&script_args)
                .status()
        };

        match result {
            Ok(status) => {
                if status.success() {
                    info!("✅ 脚本执行完成");
                } else {
                    error!("❌ 脚本执行失败，退出码: {}", status);
                }
            }
            Err(e) => error!("💥 执行脚本失败: {}", e),
        }
    }
}

/// 打开一个别名对应的路径
fn open_alias(alias: &str, config: &YamlConfig) {
    if let Some(path) = config.get_path_by_alias(alias) {
        let path = clean_path(path);
        do_open(&path);
        info!("✅ 启动 {{{}}} : {{{}}}", alias, path);
    } else {
        error!("❌ 未找到别名对应的路径或网址: {}。请检查配置文件。", alias);
    }
}

/// 使用指定应用打开某个文件/URL
fn open_with_path(alias: &str, file_path: Option<&str>, config: &YamlConfig) {
    if let Some(app_path) = config.get_property(section::PATH, alias) {
        let app_path = clean_path(app_path);
        let os = std::env::consts::OS;

        let result = if os == "macos" {
            match file_path {
                Some(fp) => Command::new("open").args(["-a", &app_path, fp]).status(),
                None => Command::new("open").arg(&app_path).status(),
            }
        } else if os == "windows" {
            match file_path {
                Some(fp) => Command::new("cmd")
                    .args(["/c", "start", "", &app_path, fp])
                    .status(),
                None => Command::new("cmd")
                    .args(["/c", "start", "", &app_path])
                    .status(),
            }
        } else {
            error!("💥 当前操作系统不支持此功能: {}", os);
            return;
        };

        match result {
            Ok(_) => {
                let target = file_path.unwrap_or("");
                info!("✅ 启动 {{{}}} {} : {{{}}}", alias, target, app_path);
            }
            Err(e) => error!("💥 启动 {} 失败: {}", alias, e),
        }
    } else {
        error!("❌ 未找到别名对应的路径: {}。", alias);
    }
}

/// 跨平台 open 命令
fn do_open(path: &str) {
    let os = std::env::consts::OS;
    let result = if os == "macos" {
        Command::new("open").arg(path).status()
    } else if os == "windows" {
        Command::new("cmd").args(["/c", "start", "", path]).status()
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
    if path.len() >= 2 {
        if (path.starts_with('\'') && path.ends_with('\''))
            || (path.starts_with('"') && path.ends_with('"'))
        {
            path = path[1..path.len() - 1].to_string();
        }
    }

    // 去除转义空格
    path = path.replace("\\ ", " ");

    // 展开 ~
    if path.starts_with('~') {
        if let Some(home) = dirs::home_dir() {
            if path == "~" {
                path = home.to_string_lossy().to_string();
            } else if path.starts_with("~/") {
                path = format!("{}{}", home.to_string_lossy(), &path[1..]);
            }
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
            info!("未指定搜索引擎，使用默认搜索引擎：{}", DEFAULT_SEARCH_ENGINE);
            search_engine::BING
        }
    };
    pattern.replace("{}", query)
}
