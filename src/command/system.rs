use crate::config::YamlConfig;
use crate::{error, info, usage};
use colored::Colorize;

/// 处理 version 命令: j version
pub fn handle_version(config: &YamlConfig) {
    if let Some(version_map) = config.get_section("version") {
        for (key, value) in version_map {
            if key == "email" || key == "author" {
                continue;
            }
            info!("{}: {}", key, value);
        }
    }
    info!("kernel version: 11.0.0");
    info!("os: {}", std::env::consts::OS);
    info!("author: lingojack | LingoJack | 达不溜勾勾");
    info!(
        "email: lingojack@qq.com | 3065225677@qq.com | 3065225677w@gmail.com"
    );
}

/// 处理 help 命令: j help
pub fn handle_help() {
    let help_text = r#"
===========================================================
  work-copilot (j) - 快捷命令行工具 🚀
===========================================================

📦 别名管理:
  j set <alias> <path>          设置别名（路径/URL）
  j rm <alias>                  删除别名
  j rename <alias> <new>        重命名别名
  j mf <alias> <new_path>      修改别名路径

🏷️  分类标记:
  j note <alias> <category>     标记别名分类
  j denote <alias> <category>   解除别名分类
    category: browser, editor, vpn, outer_url, script

📋 列表:
  j ls                          列出常用别名
  j ls all                      列出所有别名
  j ls <section>                列出指定 section

🔍 查找:
  j contain <alias>             在所有分类中查找别名
  j contain <alias> <sections>  在指定分类中查找（逗号分隔）

🚀 打开:
  j <alias>                     打开应用/文件/URL
  j <browser> <url_alias>       用浏览器打开 URL
  j <browser> <text>            用浏览器搜索
  j <editor> <file>             用编辑器打开文件

⚙️  系统设置:
  j log mode <verbose|concise>  设置日志模式
  j change <part> <field> <val> 直接修改配置字段
  j clear                       清屏

ℹ️  系统:
  j version                     版本信息
  j help                        帮助信息
  j exit                        退出（交互模式）

💡 提示:
  - 不带参数运行 `j` 进入交互模式
  - 路径可使用引号包裹处理空格
  - URL 会自动识别并归类到 inner_url
==========================================================="#;
    println!("{}", help_text);
}

/// 处理 exit 命令
pub fn handle_exit() {
    info!("Bye~ See you again 😭");
    std::process::exit(0);
}

/// 处理 log 命令: j log mode <verbose|concise>
pub fn handle_log(key: &str, value: &str, config: &mut YamlConfig) {
    if key == "mode" {
        let mode = if value == "verbose" {
            "verbose"
        } else {
            "concise"
        };
        config.set_property("log", "mode", mode);
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
        None => vec![
            "path",
            "script",
            "browser",
            "editor",
            "vpn",
            "inner_url",
            "outer_url",
        ],
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
/// 直接修改配置文件中的某个字段
pub fn handle_change(part: &str, field: &str, value: &str, config: &mut YamlConfig) {
    if !config.contains(part, field) {
        error!("❌ 在配置文件中未找到该字段：{}.{}", part, field);
        return;
    }

    let old_value = config
        .get_property(part, field)
        .cloned()
        .unwrap_or_default();
    config.set_property(part, field, value);
    info!("✅ 已修改 {}.{} 的值为 {}，旧值为 {}", part, field, value, old_value);
    info!("🚧 此命令可能会导致配置文件属性错乱而使 Copilot 无法正常使用，请确保在您清楚在做什么的情况下使用");
}
