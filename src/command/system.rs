use crate::config::YamlConfig;
use crate::constants::{self, section, config_key, CONTAIN_SEARCH_SECTIONS};
use crate::{error, info, md, usage};
use colored::Colorize;

/// 处理 version 命令: j version
pub fn handle_version(config: &YamlConfig) {
    let mut md_text = String::new();

    // 收集自定义版本信息
    if let Some(version_map) = config.get_section("version") {
        for (key, value) in version_map {
            if key == "email" || key == "author" {
                continue;
            }
            md_text.push_str(&format!("| {} | {} |\n", key, value));
        }
    }

    md!(r#"## ⚡ work-copilot (j)

|:-:|:-:|
|**kernel**|{}|
|**os**|{}|
|**author**|lingojack \| LingoJack \| 达不溜勾勾|
|**email**|lingojack@qq.com|
{}"#, constants::VERSION, std::env::consts::OS, md_text);
}

/// 处理 help 命令: j help
pub fn handle_help() {
    md!(r#"# work-copilot (j) — 快捷命令行工具 🚀

## 📦 别名管理

|:-|:-|
|`j set <alias> <path>`|设置别名（路径/URL）|
|`j rm <alias>`|删除别名|
|`j rename <alias> <new>`|重命名别名|
|`j mf <alias> <new_path>`|修改别名路径|

## 🏷️ 分类标记

|:-|:-|
|`j note <alias> <category>`|标记别名分类|
|`j denote <alias> <category>`|解除别名分类|

category: *browser*, *editor*, *vpn*, *outer_url*, *script*

## 📋 列表 & 查找

|:-|:-|
|`j ls`|列出常用别名|
|`j ls all`|列出所有别名|
|`j ls <section>`|列出指定 section|
|`j contain <alias>`|在所有分类中查找别名|
|`j contain <alias> <sections>`|在指定分类中查找（逗号分隔）|

## 🚀 打开

|:-|:-|
|`j <alias>`|打开应用/文件/URL|
|`j <browser> <url_alias>`|用浏览器打开 URL|
|`j <browser> <text>`|用浏览器搜索|
|`j <editor> <file>`|用编辑器打开文件|

## 📝 日报系统

|:-|:-|
|`j report <content>`|写入日报|
|`j reportctl new [date]`|开启新的一周（周数+1）|
|`j reportctl sync [date]`|同步周数和日期|
|`j reportctl push [msg]`|推送周报到远程 git 仓库|
|`j reportctl pull`|从远程 git 仓库拉取周报|
|`j reportctl set-url <url>`|设置/查看 git 仓库地址|
|`j check [N]`|查看日报最近 N 行（默认 5）|
|`j search <N\|all> <kw>`|在日报中搜索关键字|
|`j search <N\|all> <kw> -f`|模糊搜索（大小写不敏感）|

## 📜 脚本 & ⏳ 倒计时

|:-|:-|
|`j concat <name> "<content>"`|创建脚本并注册为别名|
|`j time countdown <duration>`|启动倒计时（30s/5m/1h）|

## ⚙️ 系统设置

|:-|:-|
|`j log mode <verbose\|concise>`|设置日志模式|
|`j change <part> <field> <val>`|直接修改配置字段|
|`j clear`|清屏|
|`j version`|版本信息|
|`j help`|帮助信息|
|`j exit`|退出（交互模式）|

## 💡 提示

- 不带参数运行 `j` 进入**交互模式**
- 交互模式下用 `!` 前缀执行 shell 命令
- 路径可使用引号包裹处理空格
- URL 会自动识别并归类到 inner_url
- 日报默认存储在 `~/.jdata/report/week_report.md`
- 配置 git 仓库: `j reportctl set-url <repo_url>`
"#);
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
