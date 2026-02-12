use crate::command::{CommandResult, output_result};
use crate::config::YamlConfig;
use crate::constants::DEFAULT_DISPLAY_SECTIONS;
use crate::{md, error};
use crate::util::log::capitalize_first_letter;

/// 处理 list 命令: j ls [part]
pub fn handle_list(part: Option<&str>, config: &YamlConfig) {
    output_result(&handle_list_with_result(part, config));
}

/// 处理 list 命令（返回结果版本）
pub fn handle_list_with_result(part: Option<&str>, config: &YamlConfig) -> CommandResult {
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
        CommandResult::with_output("无可展示的内容")
    } else {
        // 不在此处输出，由调用方决定是否输出（支持管道）
        CommandResult::with_output(md_text)
    }
}

/// 将某个 section 的内容拼接到 Markdown 文本中（空 section 跳过）
fn build_section_md(section: &str, config: &YamlConfig, md_text: &mut String) {
    use colored::Colorize;

    if let Some(map) = config.get_section(section) {
        if map.is_empty() {
            return;
        }
        md_text.push_str(&format!("## {}\n", capitalize_first_letter(section)));

        // 计算最大 key 长度用于对齐
        let max_key_len = map.keys().map(|k| k.len()).max().unwrap_or(0);

        for (key, value) in map {
            // key 青色显示，右对齐占位，箭头和 value 对齐
            md_text.push_str(&format!(
                "- {:width$} → {}\n",
                key.cyan(),
                value,
                width = max_key_len
            ));
        }
        md_text.push('\n');
    } else {
        error!("该 section 不存在: {}", section);
    }
}
