use crate::config::YamlConfig;
use crate::{error, info, usage};
use crate::command::all_command_keywords;
use url::Url;

/// 处理 set 命令: j set <alias> <path...>
pub fn handle_set(alias: &str, path_parts: &[String], config: &mut YamlConfig) {
    if path_parts.is_empty() {
        usage!("j set <alias> <path>");
        return;
    }

    // 检查别名是否与内置命令冲突
    if all_command_keywords().contains(&alias) {
        error!("别名 `{}` 已经是预设命令，请换一个。 😢", alias);
        return;
    }

    // 处理路径中包含空格的情况：将多个参数拼接
    let path = path_parts.join(" ");
    let path = remove_quotes(&path);
    let path = path.replace("\\ ", " ");

    if is_url(&path) {
        add_as_url(alias, &path, config);
    } else {
        add_as_path(alias, &path, config);
    }
}

/// 处理 remove 命令: j rm <alias>
pub fn handle_remove(alias: &str, config: &mut YamlConfig) {
    if config.contains("path", alias) {
        config.remove_property("path", alias);
        // 同时清理关联的 category
        config.remove_property("editor", alias);
        config.remove_property("vpn", alias);
        config.remove_property("browser", alias);
        config.remove_property("script", alias);
        info!("成功从 PATH 中移除别名 {} ✅", alias);
    } else if config.contains("inner_url", alias) {
        config.remove_property("inner_url", alias);
        info!("成功从 INNER_URL 中移除别名 {} ✅", alias);
    } else if config.contains("outer_url", alias) {
        config.remove_property("outer_url", alias);
        info!("成功从 OUTER_URL 中移除别名 {} ✅", alias);
    } else {
        error!("别名 {} 不存在 ❌", alias);
    }
}

/// 处理 rename 命令: j rename <alias> <new_alias>
pub fn handle_rename(alias: &str, new_alias: &str, config: &mut YamlConfig) {
    let mut updated = false;

    // path
    if config.contains("path", alias) {
        let path = config.get_property("path", alias).cloned().unwrap_or_default();
        config.rename_property("path", alias, new_alias);
        // 同时重命名关联的 category
        config.rename_property("browser", alias, new_alias);
        config.rename_property("editor", alias, new_alias);
        config.rename_property("vpn", alias, new_alias);
        config.rename_property("script", alias, new_alias);
        updated = true;
        info!("✅ 重命名 {} -> {} 成功! Path: {} 🎉", alias, new_alias, path);
    }

    // inner_url
    if config.contains("inner_url", alias) {
        let url = config.get_property("inner_url", alias).cloned().unwrap_or_default();
        config.rename_property("inner_url", alias, new_alias);
        updated = true;
        info!("✅ 重命名 {} -> {} 成功! Inner URL: {} 🚀", alias, new_alias, url);
    }

    // outer_url
    if config.contains("outer_url", alias) {
        let url = config.get_property("outer_url", alias).cloned().unwrap_or_default();
        config.rename_property("outer_url", alias, new_alias);
        updated = true;
        info!("✅ 重命名 {} -> {} 成功! Outer URL: {} 🌐", alias, new_alias, url);
    }

    if !updated {
        error!("❌ 别名 {} 不存在!", alias);
    }
}

/// 处理 modify 命令: j mf <alias> <new_path...>
pub fn handle_modify(alias: &str, path_parts: &[String], config: &mut YamlConfig) {
    if path_parts.is_empty() {
        usage!("j mf <alias> <new_path>");
        return;
    }

    let path = path_parts.join(" ");
    let path = remove_quotes(&path);
    let path = path.replace("\\ ", " ");

    let mut has_modified = false;

    // 依次检查各个 section 并更新
    let sections = ["path", "inner_url", "outer_url", "editor", "browser", "vpn"];
    for section in sections {
        if config.contains(section, alias) {
            config.set_property(section, alias, &path);
            has_modified = true;
            info!("修改 {} 在 {} 下的值为 {{{}}} 成功 ✅", alias, section, path);
        }
    }

    if !has_modified {
        error!("别名 {} 不存在，请先使用 set 命令添加。", alias);
    }
}

// ========== 辅助函数 ==========

/// 去除字符串两端的引号（单引号或双引号）
fn remove_quotes(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 {
        if (s.starts_with('\'') && s.ends_with('\''))
            || (s.starts_with('"') && s.ends_with('"'))
        {
            return s[1..s.len() - 1].to_string();
        }
    }
    s.to_string()
}

/// 判断是否为 URL
fn is_url(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }
    Url::parse(input)
        .map(|u| u.scheme() == "http" || u.scheme() == "https")
        .unwrap_or(false)
}

/// 添加为路径别名
fn add_as_path(alias: &str, path: &str, config: &mut YamlConfig) {
    if config.contains("path", alias) {
        error!(
            "别名 {} 的路径 {{{}}} 已存在。 😢 请使用 `mf` 命令修改",
            alias,
            config.get_property("path", alias).unwrap()
        );
    } else {
        config.set_property("path", alias, path);
        info!("✅ 添加别名 {} -> {{{}}} 成功! 🎉", alias, path);
    }
}

/// 添加为 URL 别名
fn add_as_url(alias: &str, url: &str, config: &mut YamlConfig) {
    if config.contains("inner_url", alias) || config.contains("outer_url", alias) {
        error!("别名 {} 已存在。 😢 请使用 `mf` 命令修改", alias);
    } else {
        config.set_property("inner_url", alias, url);
        info!("✅ 添加别名 {} -> {{{}}} 成功! 🚀", alias, url);
    }
}