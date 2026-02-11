use crate::config::YamlConfig;
use crate::constants::section;
use crate::constants::{MODIFY_SECTIONS, REMOVE_CLEANUP_SECTIONS, RENAME_SYNC_SECTIONS};
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
    let path = crate::util::remove_quotes(&path);
    let path = path.replace("\\ ", " ");

    if is_url(&path) {
        add_as_url(alias, &path, config);
    } else {
        add_as_path(alias, &path, config);
    }
}

/// 处理 remove 命令: j rm <alias>
pub fn handle_remove(alias: &str, config: &mut YamlConfig) {
    if config.contains(section::PATH, alias) {
        // 如果是脚本别名，同时删除磁盘上的脚本文件
        if let Some(script_path) = config.get_property(section::SCRIPT, alias) {
            let path = std::path::Path::new(&script_path);
            if path.exists() {
                match std::fs::remove_file(path) {
                    Ok(_) => info!("🗑️ 已删除脚本文件: {}", script_path),
                    Err(e) => error!("⚠️ 删除脚本文件失败: {}", e),
                }
            }
        }
        config.remove_property(section::PATH, alias);
        // 同时清理关联的 category
        for s in REMOVE_CLEANUP_SECTIONS {
            config.remove_property(s, alias);
        }
        info!("成功从 PATH 中移除别名 {} ✅", alias);
    } else if config.contains(section::INNER_URL, alias) {
        config.remove_property(section::INNER_URL, alias);
        info!("成功从 INNER_URL 中移除别名 {} ✅", alias);
    } else if config.contains(section::OUTER_URL, alias) {
        config.remove_property(section::OUTER_URL, alias);
        info!("成功从 OUTER_URL 中移除别名 {} ✅", alias);
    } else {
        error!("别名 {} 不存在 ❌", alias);
    }
}

/// 处理 rename 命令: j rename <alias> <new_alias>
pub fn handle_rename(alias: &str, new_alias: &str, config: &mut YamlConfig) {
    let mut updated = false;

    // path
    if config.contains(section::PATH, alias) {
        let path = config.get_property(section::PATH, alias).cloned().unwrap_or_default();
        config.rename_property(section::PATH, alias, new_alias);
        // 同时重命名关联的 category
        for s in RENAME_SYNC_SECTIONS {
            config.rename_property(s, alias, new_alias);
        }
        updated = true;
        info!("✅ 重命名 {} -> {} 成功! Path: {} 🎉", alias, new_alias, path);
    }

    // inner_url
    if config.contains(section::INNER_URL, alias) {
        let url = config.get_property(section::INNER_URL, alias).cloned().unwrap_or_default();
        config.rename_property(section::INNER_URL, alias, new_alias);
        updated = true;
        info!("✅ 重命名 {} -> {} 成功! Inner URL: {} 🚀", alias, new_alias, url);
    }

    // outer_url
    if config.contains(section::OUTER_URL, alias) {
        let url = config.get_property(section::OUTER_URL, alias).cloned().unwrap_or_default();
        config.rename_property(section::OUTER_URL, alias, new_alias);
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
    let path = crate::util::remove_quotes(&path);
    let path = path.replace("\\ ", " ");

    let mut has_modified = false;

    // 依次检查各个 section 并更新
    for s in MODIFY_SECTIONS {
        if config.contains(s, alias) {
            config.set_property(s, alias, &path);
            has_modified = true;
            info!("修改 {} 在 {} 下的值为 {{{}}} 成功 ✅", alias, s, path);
        }
    }

    if !has_modified {
        error!("别名 {} 不存在，请先使用 set 命令添加。", alias);
    }
}

// ========== 辅助函数 ==========

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
    if config.contains(section::PATH, alias) {
        error!(
            "别名 {} 的路径 {{{}}} 已存在。 😢 请使用 `mf` 命令修改",
            alias,
            config.get_property(section::PATH, alias).unwrap()
        );
    } else {
        config.set_property(section::PATH, alias, path);
        info!("✅ 添加别名 {} -> {{{}}} 成功! 🎉", alias, path);
    }
}

/// 添加为 URL 别名
fn add_as_url(alias: &str, url: &str, config: &mut YamlConfig) {
    if config.contains(section::INNER_URL, alias) || config.contains(section::OUTER_URL, alias) {
        error!("别名 {} 已存在。 😢 请使用 `mf` 命令修改", alias);
    } else {
        config.set_property(section::INNER_URL, alias, url);
        info!("✅ 添加别名 {} -> {{{}}} 成功! 🚀", alias, url);
    }
}