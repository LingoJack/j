use crate::config::YamlConfig;
use crate::constants::{self, section, config_key, CONTAIN_SEARCH_SECTIONS};
use crate::{error, info, md, usage};
use colored::Colorize;

/// 编译时嵌入的版本信息模板
const VERSION_TEMPLATE: &str = include_str!("../../assets/version.md");

/// 处理 version 命令: j version
pub fn handle_version(config: &YamlConfig) {
    let mut extra = String::new();

    // 收集自定义版本信息
    if let Some(version_map) = config.get_section("version") {
        for (key, value) in version_map {
            if key == "email" || key == "author" {
                continue;
            }
            extra.push_str(&format!("| {} | {} |\n", key, value));
        }
    }

    let text = VERSION_TEMPLATE
        .replace("{version}", constants::VERSION)
        .replace("{os}", std::env::consts::OS)
        .replace("{extra}", &extra);
    md!("{}", text);
}

/// 编译时嵌入的帮助文档
const HELP_TEXT: &str = include_str!("../../assets/help.md");

/// 处理 help 命令: j help
pub fn handle_help() {
    md!("{}", HELP_TEXT);
}

/// 处理 exit 命令
pub fn handle_exit() {
    info!("Bye~ See you again 😭");
    std::process::exit(0);
}

/// 处理 log 命令: j log mode <verbose|concise>
pub fn handle_log(key: &str, value: &str, config: &mut YamlConfig) {
    if key == config_key::MODE {
        let mode = if value == config_key::VERBOSE {
            config_key::VERBOSE
        } else {
            config_key::CONCISE
        };
        config.set_property(section::LOG, config_key::MODE, mode);
        info!("✅ 日志模式已切换为: {}", mode);
    } else {
        usage!("j log mode <verbose|concise>");
    }
}

/// 处理 clear 命令: j clear
pub fn handle_clear() {
    // 使用 ANSI 转义序列清屏
    print!("\x1B[2J\x1B[1;1H");
}

/// 处理 contain 命令: j contain <alias> [containers]
/// 在指定分类中查找别名
pub fn handle_contain(alias: &str, containers: Option<&str>, config: &YamlConfig) {
    let sections: Vec<&str> = match containers {
        Some(c) => c.split(',').collect(),
        None => CONTAIN_SEARCH_SECTIONS.to_vec(),
    };

    let mut found = Vec::new();

    for section in &sections {
        if config.contains(section, alias) {
            if let Some(value) = config.get_property(section, alias) {
                found.push(format!(
                    "{} {}: {}",
                    format!("[{}]", section).green(),
                    alias,
                    value
                ));
            }
        }
    }

    if found.is_empty() {
        info!("nothing found 😢");
    } else {
        info!("找到 {} 条结果 😊", found.len().to_string().green());
        for line in &found {
            info!("{}", line);
        }
    }
}

/// 处理 change 命令: j change <part> <field> <value>
/// 直接修改配置文件中的某个字段（如果字段不存在则新增）
pub fn handle_change(part: &str, field: &str, value: &str, config: &mut YamlConfig) {
    if config.get_section(part).is_none() {
        error!("❌ 在配置文件中未找到该 section：{}", part);
        return;
    }

    let old_value = config.get_property(part, field).cloned();
    config.set_property(part, field, value);

    match old_value {
        Some(old) => {
            info!("✅ 已修改 {}.{} 的值为 {}，旧值为 {}", part, field, value, old);
        }
        None => {
            info!("✅ 已新增 {}.{} = {}", part, field, value);
        }
    }
    info!("🚧 此命令可能会导致配置文件属性错乱而使 Copilot 无法正常使用，请确保在您清楚在做什么的情况下使用");
}
