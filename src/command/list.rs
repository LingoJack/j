use crate::config::YamlConfig;
use crate::constants::DEFAULT_DISPLAY_SECTIONS;
use crate::{md};
use crate::util::log::capitalize_first_letter;

/// 处理 list 命令: j ls [part]
pub fn handle_list(part: Option<&str>, config: &YamlConfig) {
    match part {
        None => {
            // 默认展示常用 section
            for s in DEFAULT_DISPLAY_SECTIONS {
                list_section(s, config);
            }
        }
        Some("all") => {
            // 展示所有 section
            for section in config.all_section_names() {
                list_section(section, config);
            }
        }
        Some(section) => {
            list_section(section, config);
        }
    }
}

/// 列出某个 section 的所有键值对（Markdown 无序列表渲染）
fn list_section(section: &str, config: &YamlConfig) {
    if let Some(map) = config.get_section(section) {
        if map.is_empty() {
            md!("- **{}**\n\t- *Empty*\n", capitalize_first_letter(section));
        } else {
            let mut md_text = format!("- **{}**\n", capitalize_first_letter(section));
            for (key, value) in map {
                md_text.push_str(&format!("\t- {} → {}\n", key, value));
            }
            md!("{}", md_text);
        }
    } else {
        crate::error!("该 section 不存在: {}", section);
    }
}
