//! 别名管理核心逻辑（GUI 友好，无终端依赖）

use crate::config::YamlConfig;
use crate::constants::section;
use crate::constants::{MODIFY_SECTIONS, REMOVE_CLEANUP_SECTIONS, RENAME_SYNC_SECTIONS};
use url::Url;

/// 添加别名（返回 Result）
pub fn set_alias(alias: &str, path: &str, config: &mut YamlConfig) -> Result<String, String> {
    if crate::constants::cmd::all_keywords().contains(&alias) {
        return Err(format!("别名 `{}` 已经是预设命令，请换一个", alias));
    }

    let path = crate::util::remove_quotes(path);
    let path = path.replace("\\ ", " ");

    if is_url(&path) {
        if config.contains(section::INNER_URL, alias) || config.contains(section::OUTER_URL, alias)
        {
            return Err(format!("别名 {} 已存在，请使用修改功能", alias));
        }
        config.set_property(section::INNER_URL, alias, &path);
        Ok(format!("添加别名 {} -> {} 成功", alias, path))
    } else {
        if config.contains(section::PATH, alias) {
            return Err(format!(
                "别名 {} 的路径 {} 已存在，请使用修改功能",
                alias,
                config
                    .get_property(section::PATH, alias)
                    .map_or("(未知)", |v| v)
            ));
        }
        config.set_property(section::PATH, alias, &path);
        Ok(format!("添加别名 {} -> {} 成功", alias, path))
    }
}

/// 删除别名（返回 Result）
pub fn remove_alias(alias: &str, config: &mut YamlConfig) -> Result<String, String> {
    if config.contains(section::PATH, alias) {
        // 如果是脚本别名，同时删除脚本文件
        if let Some(script_path) = config.get_property(section::SCRIPT, alias) {
            let path = std::path::Path::new(&script_path);
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
        config.remove_property(section::PATH, alias);
        for s in REMOVE_CLEANUP_SECTIONS {
            config.remove_property(s, alias);
        }
        Ok(format!("从 PATH 中移除别名 {} 成功", alias))
    } else if config.contains(section::INNER_URL, alias) {
        config.remove_property(section::INNER_URL, alias);
        Ok(format!("从 INNER_URL 中移除别名 {} 成功", alias))
    } else if config.contains(section::OUTER_URL, alias) {
        config.remove_property(section::OUTER_URL, alias);
        Ok(format!("从 OUTER_URL 中移除别名 {} 成功", alias))
    } else {
        Err(format!("别名 {} 不存在", alias))
    }
}

/// 重命名别名（返回 Result）
pub fn rename_alias(
    alias: &str,
    new_alias: &str,
    config: &mut YamlConfig,
) -> Result<String, String> {
    let mut messages = Vec::new();

    if config.contains(section::PATH, alias) {
        config.rename_property(section::PATH, alias, new_alias);
        for s in RENAME_SYNC_SECTIONS {
            config.rename_property(s, alias, new_alias);
        }
        messages.push(format!("PATH: {} -> {}", alias, new_alias));
    }

    if config.contains(section::INNER_URL, alias) {
        config.rename_property(section::INNER_URL, alias, new_alias);
        messages.push(format!("INNER_URL: {} -> {}", alias, new_alias));
    }

    if config.contains(section::OUTER_URL, alias) {
        config.rename_property(section::OUTER_URL, alias, new_alias);
        messages.push(format!("OUTER_URL: {} -> {}", alias, new_alias));
    }

    if messages.is_empty() {
        Err(format!("别名 {} 不存在", alias))
    } else {
        Ok(format!("重命名成功: {}", messages.join(", ")))
    }
}

/// 修改别名路径（返回 Result）
pub fn modify_alias(
    alias: &str,
    new_path: &str,
    config: &mut YamlConfig,
) -> Result<String, String> {
    let path = crate::util::remove_quotes(new_path);
    let path = path.replace("\\ ", " ");

    let mut modified = false;
    for s in MODIFY_SECTIONS {
        if config.contains(s, alias) {
            config.set_property(s, alias, &path);
            modified = true;
        }
    }

    if modified {
        Ok(format!("修改 {} 的值为 {} 成功", alias, path))
    } else {
        Err(format!("别名 {} 不存在", alias))
    }
}

fn is_url(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }
    Url::parse(input)
        .map(|u| u.scheme() == "http" || u.scheme() == "https")
        .unwrap_or(false)
}
