use crate::config::YamlConfig;
use crate::constants::DEFAULT_DISPLAY_SECTIONS;
use crate::info;
use crate::util::log::{capitalize_first_letter, print_line};
use colored::Colorize;

/// 处理 list 命令: j ls [part]
pub fn handle_list(part: Option<&str>, config: &YamlConfig) {
    print_line();

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

/// 列出某个 section 的所有键值对
fn list_section(section: &str, config: &YamlConfig) {
    if let Some(map) = config.get_section(section) {
        println!("{}", format!("[{}]", capitalize_first_letter(section)).yellow());

        if map.is_empty() {
            info!("Empty");
        } else {
            for (key, value) in map {
                print!("{}", key.green());
                info!(": {}", value);
            }
        }
        print_line();
    } else {
        crate::error!("该 section 不存在: {}", section);
    }
}
