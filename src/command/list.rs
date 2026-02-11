use crate::config::YamlConfig;
use crate::constants::DEFAULT_DISPLAY_SECTIONS;
use crate::{md};
use crate::util::log::capitalize_first_letter;

/// 处理 list 命令: j ls [part]
pub fn handle_list(part: Option<&str>, config: &YamlConfig) {
    let mut md_text = String::new();
    match part {
        None => {
            // 默认展示常用 section
            for s in DEFAULT_DISPLAY_SECTIONS {
                build_section_md(s, config, &mut md_text);
            }
        }
        Some("all") => {
            // 展示所有 section
            for section in config.all_section_names() {
                build_section_md(section, config, &mut md_text);
            }
        }
        Some(section) => {
            build_section_md(section, config, &mut md_text);
        }
    }

    if md_text.is_empty() {
        crate::info!("无可展示的内容");
    } else {
        md!("{}", md_text);
    }
}

/// 将某个 section 的内容拼接到 Markdown 文本中（空 section 跳过）
fn build_section_md(section: &str, config: &YamlConfig, md_text: &mut String) {
    if let Some(map) = config.get_section(section) {
        if map.is_empty() {
            return;
        }
        md_text.push_str(&format!("## {}\n", capitalize_first_letter(section)));
        for (key, value) in map {
            md_text.push_str(&format!("- {} → {}\n", key, value));
        }
        md_text.push('\n');
    } else {
        crate::error!("该 section 不存在: {}", section);
    }
}
