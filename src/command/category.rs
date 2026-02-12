use crate::command::{CommandResult, output_result};
use crate::config::YamlConfig;
use crate::constants::{self, section};
use crate::{error, info, usage};

/// 支持标记的分类列表（引用全局常量）
const VALID_CATEGORIES: &[&str] = constants::NOTE_CATEGORIES;

/// 处理 note 命令: j note <alias> <category>
/// 将别名标记为指定分类（browser/editor/vpn/outer_url/script）
pub fn handle_note(alias: &str, category: &str, config: &mut YamlConfig) {
    output_result(&handle_note_with_result(alias, category, config));
}

/// 处理 note 命令（返回结果版本）
pub fn handle_note_with_result(alias: &str, category: &str, config: &mut YamlConfig) -> CommandResult {
    // 校验别名是否存在
    if !config.contains(section::PATH, alias)
        && !config.contains(section::INNER_URL, alias)
        && !config.contains(section::OUTER_URL, alias)
    {
        return CommandResult::error(format!("❌ 别名 {} 不存在", alias));
    }

    // 校验 category 是否合法
    if !VALID_CATEGORIES.contains(&category) {
        return CommandResult::error(format!(
            "j note <alias> <category> (可选: {})",
            VALID_CATEGORIES.join(", ")
        ));
    }

    match category {
        c if c == section::OUTER_URL => {
            // outer_url 特殊处理：从 inner_url 移到 outer_url
            if let Some(url) = config.get_property(section::INNER_URL, alias).cloned() {
                config.set_property(section::OUTER_URL, alias, &url);
                config.remove_property(section::INNER_URL, alias);
                CommandResult::with_output(format!("✅ 将别名 {} 标记为 OUTER_URL 成功", alias))
            } else {
                CommandResult::error(format!("❌ 别名 {} 不在 INNER_URL 中，无法标记为 OUTER_URL", alias))
            }
        }
        _ => {
            // 其他分类：将 path 中的值复制到对应分类
            if let Some(path) = config.get_property(section::PATH, alias).cloned() {
                config.set_property(category, alias, &path);
                CommandResult::with_output(format!(
                    "✅ 将别名 {} 标记为 {} 成功",
                    alias,
                    category.to_uppercase()
                ))
            } else {
                CommandResult::error(format!("❌ 别名 {} 不在 PATH 中，无法标记", alias))
            }
        }
    }
}

/// 处理 denote 命令: j denote <alias> <category>
/// 解除别名的分类标记
pub fn handle_denote(alias: &str, category: &str, config: &mut YamlConfig) {
    output_result(&handle_denote_with_result(alias, category, config));
}

/// 处理 denote 命令（返回结果版本）
pub fn handle_denote_with_result(alias: &str, category: &str, config: &mut YamlConfig) -> CommandResult {
    // 校验 category 是否合法
    if !VALID_CATEGORIES.contains(&category) {
        return CommandResult::error(format!(
            "j denote <alias> <category> (可选: {})",
            VALID_CATEGORIES.join(", ")
        ));
    }

    if !config.contains(category, alias) {
        return CommandResult::error(format!("❌ 别名 {} 不在 {} 分类中", alias, category.to_uppercase()));
    }

    config.remove_property(category, alias);
    CommandResult::with_output(format!(
        "✅ 已将别名 {} 从 {} 中移除",
        alias,
        category.to_uppercase()
    ))
}
