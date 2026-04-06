//! 打开别名核心逻辑（GUI 友好，无终端依赖）

use crate::config::YamlConfig;
use crate::constants::{DEFAULT_SEARCH_ENGINE, config_key, search_engine, section};
use std::path::Path;
use std::process::Command;

/// 通过别名打开应用/文件/URL（静默版本，返回 Result）
pub fn open_alias_silent(
    alias: &str,
    args: &[String],
    config: &YamlConfig,
) -> Result<String, String> {
    if !config.alias_exists(alias) {
        return Err(format!("无法找到别名对应的路径或网址: {}", alias));
    }

    // 浏览器
    if config.contains(section::BROWSER, alias) {
        return open_browser_silent(alias, args, config);
    }

    // 编辑器
    if config.contains(section::EDITOR, alias) {
        if args.len() >= 2 {
            return open_with_path_silent(alias, Some(&args[1]), config);
        } else {
            return do_open_alias_silent(alias, &[], config);
        }
    }

    // VPN
    if config.contains(section::VPN, alias) {
        return do_open_alias_silent(alias, &[], config);
    }

    // 脚本
    if config.contains(section::SCRIPT, alias) {
        return run_script_silent(alias, &args[1..], config);
    }

    // 默认：打开路径
    do_open_alias_silent(alias, &args[1..], config)
}

/// 打开浏览器（静默版本）
fn open_browser_silent(
    alias: &str,
    args: &[String],
    config: &YamlConfig,
) -> Result<String, String> {
    if args.len() <= 1 {
        return do_open_alias_silent(alias, &[], config);
    }

    let url_alias_or_text = &args[1];

    let url = if let Some(u) = config.get_property(section::INNER_URL, url_alias_or_text) {
        u.clone()
    } else if let Some(u) = config.get_property(section::OUTER_URL, url_alias_or_text) {
        // outer_url: 先启动 VPN
        if let Some(vpn_map) = config.get_section(section::VPN)
            && let Some(vpn_alias) = vpn_map.keys().next()
        {
            let _ = do_open_alias_silent(vpn_alias, &[], config);
        }
        u.clone()
    } else if url_alias_or_text.starts_with("http://") || url_alias_or_text.starts_with("https://")
    {
        url_alias_or_text.clone()
    } else {
        // 搜索引擎
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

    open_with_path_silent(alias, Some(&url), config)
}

/// 打开别名对应路径（静默版本）
fn do_open_alias_silent(
    alias: &str,
    extra_args: &[String],
    config: &YamlConfig,
) -> Result<String, String> {
    let path = config
        .get_path_by_alias(alias)
        .ok_or_else(|| format!("未找到别名对应的路径: {}", alias))?;
    let path = clean_path(path);
    let expanded_args: Vec<String> = extra_args.iter().map(|s| clean_path(s)).collect();

    if is_cli_executable(&path) {
        Command::new(&path)
            .args(&expanded_args)
            .status()
            .map_err(|e| format!("执行 {} 失败: {}", alias, e))
            .and_then(|status| {
                if status.success() {
                    Ok(format!("执行 {} 完成", alias))
                } else {
                    Err(format!("执行 {} 失败，退出码: {}", alias, status))
                }
            })
    } else {
        do_system_open(&path, &expanded_args)?;
        Ok(format!("启动 {} : {}", alias, path))
    }
}

/// 使用指定应用打开文件/URL（静默版本）
fn open_with_path_silent(
    alias: &str,
    file_path: Option<&str>,
    config: &YamlConfig,
) -> Result<String, String> {
    let app_path = config
        .get_property(section::PATH, alias)
        .ok_or_else(|| format!("未找到别名对应的路径: {}", alias))?;
    let app_path = clean_path(app_path);
    let file_path_expanded = file_path.map(clean_path);
    let file_path = file_path_expanded.as_deref();

    let os = std::env::consts::OS;
    let result = if os == "macos" {
        match file_path {
            Some(fp) => Command::new("open").args(["-a", &app_path, fp]).status(),
            None => Command::new("open").arg(&app_path).status(),
        }
    } else if os == "windows" {
        match file_path {
            Some(fp) => Command::new("cmd.exe")
                .args(["/c", "start", "", &app_path, fp])
                .status(),
            None => Command::new("cmd.exe")
                .args(["/c", "start", "", &app_path])
                .status(),
        }
    } else {
        return Err(format!("当前操作系统不支持此功能: {}", os));
    };

    result
        .map_err(|e| format!("启动 {} 失败: {}", alias, e))
        .map(|_| {
            let target = file_path.unwrap_or("");
            format!("启动 {} {} : {}", alias, target, app_path)
        })
}

/// 运行脚本（静默版本）
fn run_script_silent(
    alias: &str,
    script_args: &[String],
    config: &YamlConfig,
) -> Result<String, String> {
    let script_path = config
        .get_property(section::SCRIPT, alias)
        .ok_or_else(|| format!("未找到脚本路径: {}", alias))?;
    let script_path = clean_path(script_path);
    let script_args: Vec<String> = script_args.iter().map(|s| clean_path(s)).collect();
    let script_arg_refs: Vec<&str> = script_args.iter().map(|s| s.as_str()).collect();

    let mut cmd = Command::new("sh");
    cmd.arg(&script_path).args(&script_arg_refs);
    // 注入别名环境变量
    for (key, value) in config.collect_alias_envs() {
        cmd.env(&key, &value);
    }

    cmd.status()
        .map_err(|e| format!("执行脚本失败: {}", e))
        .and_then(|status| {
            if status.success() {
                Ok(format!("脚本 {} 执行完成", alias))
            } else {
                Err(format!("脚本执行失败，退出码: {}", status))
            }
        })
}

/// 跨平台系统 open
fn do_system_open(path: &str, extra_args: &[String]) -> Result<(), String> {
    let os = std::env::consts::OS;
    let result = if os == "macos" {
        if extra_args.is_empty() {
            Command::new("open").arg(path).status()
        } else {
            Command::new("open")
                .args(["-a", path])
                .args(extra_args)
                .status()
        }
    } else if os == "windows" {
        Command::new("cmd.exe")
            .args(["/c", "start", "", path])
            .status()
    } else {
        Command::new("xdg-open").arg(path).status()
    };

    result
        .map_err(|e| format!("打开 {} 失败: {}", path, e))
        .map(|_| ())
}

fn is_cli_executable(path: &str) -> bool {
    if path.starts_with("http://") || path.starts_with("https://") {
        return false;
    }
    if path.ends_with(".app") || path.contains(".app/") {
        return false;
    }
    let p = Path::new(path);
    if !p.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = p.metadata() {
            return metadata.permissions().mode() & 0o111 != 0;
        }
    }
    false
}

fn clean_path(path: &str) -> String {
    let mut path = path.trim().to_string();
    if path.len() >= 2
        && ((path.starts_with('\'') && path.ends_with('\''))
            || (path.starts_with('"') && path.ends_with('"')))
    {
        path = path[1..path.len() - 1].to_string();
    }
    path = path.replace("\\ ", " ");
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

fn get_search_url(query: &str, engine: &str) -> String {
    let pattern = match engine.to_lowercase().as_str() {
        "google" => search_engine::GOOGLE,
        "bing" => search_engine::BING,
        "baidu" => search_engine::BAIDU,
        _ => search_engine::BING,
    };
    pattern.replace("{}", query)
}
