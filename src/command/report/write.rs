//! 日报写入、TUI 编辑、配置同步。

use crate::command::chat::storage::load_agent_config;
use crate::config::YamlConfig;
use crate::constants::{REPORT_SIMPLE_DATE_FORMAT, config_key, section};
use crate::theme::Theme;
use crate::{error, info, usage};
use chrono::{Local, NaiveDate};
use std::fs;
use std::path::Path;

use super::io::{
    DATE_FORMAT, append_to_file, get_report_path, get_report_path_silent, get_settings_json_path,
    parse_date, read_last_n_lines, replace_last_n_lines,
};

const SIMPLE_DATE_FORMAT: &str = REPORT_SIMPLE_DATE_FORMAT;

// ========== 对外公共接口 ==========

/// 处理 report 命令: j report <content...> 或 j reportctl new [date] / j reportctl sync [date]
pub fn handle_report(sub: &str, content: &[String], config: &mut YamlConfig) {
    if content.is_empty() {
        if sub == "reportctl" {
            usage!(
                "j reportctl new [date] | j reportctl sync [date] | j reportctl push | j reportctl pull | j reportctl set-url <url> | j reportctl open"
            );
            return;
        }
        // report 无参数：打开 TUI 多行编辑器（预填历史 + 日期前缀，NORMAL 模式）
        handle_report_tui(config);
        return;
    }

    let first = content[0].as_str();

    // 元数据操作
    if sub == "reportctl" {
        match first {
            f if f == crate::constants::rmeta_action::NEW => {
                let date_str = content.get(1).map(|s| s.as_str());
                handle_week_update(date_str, config);
            }
            f if f == crate::constants::rmeta_action::SYNC => {
                let date_str = content.get(1).map(|s| s.as_str());
                handle_sync(date_str, config);
            }
            f if f == crate::constants::rmeta_action::PUSH => {
                let msg = content.get(1).map(|s| s.as_str());
                super::git::handle_push(msg, config);
            }
            f if f == crate::constants::rmeta_action::PULL => {
                super::git::handle_pull(config);
            }
            f if f == crate::constants::rmeta_action::SET_URL => {
                let url = content.get(1).map(|s| s.as_str());
                super::git::handle_set_url(url, config);
            }
            f if f == crate::constants::rmeta_action::OPEN => {
                handle_open_report(config);
            }
            _ => {
                error!(
                    "未知的元数据操作: {}，可选: {}, {}, {}, {}, {}, {}",
                    first,
                    crate::constants::rmeta_action::NEW,
                    crate::constants::rmeta_action::SYNC,
                    crate::constants::rmeta_action::PUSH,
                    crate::constants::rmeta_action::PULL,
                    crate::constants::rmeta_action::SET_URL,
                    crate::constants::rmeta_action::OPEN
                );
            }
        }
        return;
    }

    // 常规日报写入
    let text = content.join(" ");
    let text = text.trim().trim_matches('"').to_string();

    if text.is_empty() {
        error!("内容为空，无法写入");
        return;
    }

    handle_daily_report(&text, config);
}

/// 将一条内容写入日报（供外部模块调用，如 todo 完成时联动写入）
/// 返回 true 表示写入成功
/// 注意：此函数静默执行，不输出任何 info!/error!，适合在 TUI raw mode 下调用
pub fn write_to_report(content: &str, config: &mut YamlConfig) -> bool {
    let report_path = match get_report_path_silent(config) {
        Some(p) => p,
        None => return false,
    };

    let report_file = Path::new(&report_path);
    let config_path = match get_settings_json_path(&report_path) {
        Some(p) => p,
        None => return false,
    };

    // 静默加载 JSON 配置并同步到 YAML（不打印 info）
    load_config_from_json_silent(&config_path, config);

    let now = Local::now().date_naive();
    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);
    let last_day_str = config
        .get_property(section::REPORT, config_key::LAST_DAY)
        .cloned()
        .unwrap_or_default();
    let last_day = parse_date(&last_day_str);

    ensure_week_boundary(
        week_num,
        last_day,
        now,
        report_file,
        &config_path,
        config,
        true,
    );

    let today_str = now.format(SIMPLE_DATE_FORMAT);
    let log_entry = format!("- 【{}】 {}\n", today_str, content);
    append_to_file(report_file, &log_entry);
    true
}

// ========== 日报写入 ==========

/// 写入日报
fn handle_daily_report(content: &str, config: &mut YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    info!("日报文件路径：{}", report_path);

    let report_file = Path::new(&report_path);
    let config_path = match get_settings_json_path(&report_path) {
        Some(p) => p,
        None => {
            error!("无法获取配置文件路径");
            return;
        }
    };

    load_config_from_json_and_sync(&config_path, config);

    let now = Local::now().date_naive();
    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);
    let last_day_str = config
        .get_property(section::REPORT, config_key::LAST_DAY)
        .cloned()
        .unwrap_or_default();
    let last_day = parse_date(&last_day_str);

    let initialized = ensure_week_boundary(
        week_num,
        last_day,
        now,
        report_file,
        &config_path,
        config,
        false,
    );
    if initialized {
        info!("已自动初始化第一周");
    }

    let today_str = now.format(SIMPLE_DATE_FORMAT);
    let log_entry = format!("- 【{}】 {}\n", today_str, content);
    append_to_file(report_file, &log_entry);
    info!("成功将内容写入：{}", report_path);
}

// ========== TUI 编辑 ==========

/// TUI 模式日报编辑：预加载历史 + 日期前缀，NORMAL 模式进入
fn handle_report_tui(config: &mut YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    let config_path = match get_settings_json_path(&report_path) {
        Some(p) => p,
        None => {
            error!("无法获取配置文件路径");
            return;
        }
    };
    load_config_from_json_and_sync(&config_path, config);

    // 检查是否需要新开一周
    let now = Local::now().date_naive();
    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);
    let last_day_str = config
        .get_property(section::REPORT, config_key::LAST_DAY)
        .cloned()
        .unwrap_or_default();
    let last_day = parse_date(&last_day_str);

    // 先读取文件最后 3 行作为历史上下文（在任何写入之前读取）
    let context_lines = 3;
    let report_file = Path::new(&report_path);
    let last_lines = read_last_n_lines(report_file, context_lines);

    // 拼接编辑器初始内容
    let mut initial_lines: Vec<String> = last_lines.clone();

    // 检查是否需要新开一周 → 只更新配置，不写入文件；新周标题放入编辑器
    if let Some(last_day) = last_day
        && now > last_day
    {
        let next_last_day = now + chrono::Duration::days(6);
        let new_week_title = format!(
            "# Week{}[{}-{}]",
            week_num,
            now.format(DATE_FORMAT),
            next_last_day.format(DATE_FORMAT)
        );
        update_config_files(week_num + 1, &next_last_day, &config_path, config);
        initial_lines.push(new_week_title);
    }

    // 构造日期前缀行
    let today_str = now.format(SIMPLE_DATE_FORMAT);
    let date_prefix = format!("- 【{}】 ", today_str);
    initial_lines.push(date_prefix);

    // 打开带初始内容的编辑器（NORMAL 模式）
    let theme = Theme::from_name(&load_agent_config().theme);
    match crate::tui::editor_markdown::open_markdown_editor_with_content(
        "编辑日报",
        &initial_lines,
        &theme,
    ) {
        Ok((Some(text), _)) => {
            let original_context_count = last_lines.len();
            replace_last_n_lines(report_file, original_context_count, &text);
            info!("日报已写入：{}", report_path);
        }
        Ok((None, _)) => {
            info!("已取消编辑");
        }
        Err(e) => {
            error!("编辑器启动失败: {}", e);
        }
    }
}

/// 用内置 TUI 编辑器打开日报文件全文
pub(super) fn handle_open_report(config: &YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    let path = Path::new(&report_path);
    if !path.is_file() {
        error!("日报文件不存在: {}", report_path);
        return;
    }

    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            error!("读取日报文件失败: {}", e);
            return;
        }
    };

    let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();

    let theme = Theme::from_name(&load_agent_config().theme);
    match crate::tui::editor_markdown::open_markdown_editor_with_content(
        "编辑日报文件",
        &lines,
        &theme,
    ) {
        Ok((Some(text), _)) => {
            let mut result = text;
            if !result.ends_with('\n') {
                result.push('\n');
            }
            if let Err(e) = fs::write(path, &result) {
                error!("写入日报文件失败: {}", e);
                return;
            }
            info!("日报文件已保存：{}", report_path);
        }
        Ok((None, _)) => {
            info!("已取消编辑，文件未修改");
        }
        Err(e) => {
            error!("编辑器启动失败: {}", e);
        }
    }
}

// ========== 周数管理 ==========

/// 处理 reportctl new 命令：开启新的一周
fn handle_week_update(date_str: Option<&str>, config: &mut YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    let config_path = match get_settings_json_path(&report_path) {
        Some(p) => p,
        None => {
            error!("无法获取配置文件路径");
            return;
        }
    };

    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    let last_day_str = date_str
        .map(|s| s.to_string())
        .or_else(|| {
            config
                .get_property(section::REPORT, config_key::LAST_DAY)
                .cloned()
        })
        .unwrap_or_default();

    match parse_date(&last_day_str) {
        Some(last_day) => {
            let next_last_day = last_day + chrono::Duration::days(7);
            update_config_files(week_num + 1, &next_last_day, &config_path, config);
        }
        None => {
            error!("更新周数失败，请检查日期字符串是否有误: {}", last_day_str);
        }
    }
}

/// 处理 reportctl sync 命令：同步周数和日期
fn handle_sync(date_str: Option<&str>, config: &mut YamlConfig) {
    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    let config_path = match get_settings_json_path(&report_path) {
        Some(p) => p,
        None => {
            error!("无法获取配置文件路径");
            return;
        }
    };

    load_config_from_json_and_sync(&config_path, config);

    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    let last_day_str = date_str
        .map(|s| s.to_string())
        .or_else(|| {
            config
                .get_property(section::REPORT, config_key::LAST_DAY)
                .cloned()
        })
        .unwrap_or_default();

    match parse_date(&last_day_str) {
        Some(last_day) => {
            update_config_files(week_num, &last_day, &config_path, config);
        }
        None => {
            error!("更新周数失败，请检查日期字符串是否有误: {}", last_day_str);
        }
    }
}

// ========== 配置同步辅助 ==========

/// 确保周边界正确：如果当前日期超过 last_day 则自动开新周。
/// 返回 true 表示首次初始化（last_day 为 None）。
fn ensure_week_boundary(
    week_num: i32,
    last_day: Option<NaiveDate>,
    now: chrono::NaiveDate,
    report_file: &Path,
    config_path: &Path,
    config: &mut YamlConfig,
    silent: bool,
) -> bool {
    match last_day {
        Some(last_day) => {
            if now > last_day {
                let next_last_day = now + chrono::Duration::days(6);
                let new_week_title = format!(
                    "# Week{}[{}-{}]\n",
                    week_num,
                    now.format(DATE_FORMAT),
                    next_last_day.format(DATE_FORMAT)
                );
                if silent {
                    update_config_files_silent(week_num + 1, &next_last_day, config_path, config);
                } else {
                    update_config_files(week_num + 1, &next_last_day, config_path, config);
                }
                append_to_file(report_file, &new_week_title);
            }
            false
        }
        None => {
            let next_last_day = now + chrono::Duration::days(6);
            let new_week_title = format!(
                "# Week{}[{}-{}]\n",
                week_num,
                now.format(DATE_FORMAT),
                next_last_day.format(DATE_FORMAT)
            );
            if silent {
                update_config_files_silent(week_num + 1, &next_last_day, config_path, config);
            } else {
                update_config_files(week_num + 1, &next_last_day, config_path, config);
            }
            append_to_file(report_file, &new_week_title);
            true
        }
    }
}

/// 更新配置文件（YAML + JSON）
fn update_config_files(
    week_num: i32,
    last_day: &NaiveDate,
    config_path: &Path,
    config: &mut YamlConfig,
) {
    let last_day_str = last_day.format(DATE_FORMAT).to_string();

    config.set_property(section::REPORT, config_key::WEEK_NUM, &week_num.to_string());
    config.set_property(section::REPORT, config_key::LAST_DAY, &last_day_str);
    info!(
        "更新YAML配置文件成功：周数 = {}, 周结束日期 = {}",
        week_num, last_day_str
    );

    let json = serde_json::json!({
        "week_num": week_num,
        "last_day": last_day_str
    });
    match fs::write(config_path, json.to_string()) {
        Ok(_) => info!(
            "更新JSON配置文件成功：周数 = {}, 周结束日期 = {}",
            week_num, last_day_str
        ),
        Err(e) => error!("更新JSON配置文件时出错: {}", e),
    }
}

/// 静默更新配置文件（YAML + JSON），不输出 info
fn update_config_files_silent(
    week_num: i32,
    last_day: &NaiveDate,
    config_path: &Path,
    config: &mut YamlConfig,
) {
    let last_day_str = last_day.format(DATE_FORMAT).to_string();

    config.set_property(section::REPORT, config_key::WEEK_NUM, &week_num.to_string());
    config.set_property(section::REPORT, config_key::LAST_DAY, &last_day_str);

    let json = serde_json::json!({
        "week_num": week_num,
        "last_day": last_day_str
    });
    let _ = fs::write(config_path, json.to_string());
}

/// 从 JSON 配置文件读取并同步到 YAML
fn load_config_from_json_and_sync(config_path: &Path, config: &mut YamlConfig) {
    if !config_path.exists() {
        let now = Local::now().date_naive();
        let last_day = now + chrono::Duration::days(6);
        info!(
            "日报配置文件不存在，自动初始化：week_num = 1, last_day = {}",
            last_day.format(DATE_FORMAT)
        );
        update_config_files(1, &last_day, config_path, config);
        return;
    }

    match fs::read_to_string(config_path) {
        Ok(content) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let last_day = json.get("last_day").and_then(|v| v.as_str()).unwrap_or("");
                let week_num = json.get("week_num").and_then(|v| v.as_i64()).unwrap_or(1);

                info!(
                    "从日报配置文件中读取到：last_day = {}, week_num = {}",
                    last_day, week_num
                );

                if let Some(last_day_date) = parse_date(last_day) {
                    update_config_files(week_num as i32, &last_day_date, config_path, config);
                }
            } else {
                error!("解析日报配置文件时出错");
            }
        }
        Err(e) => error!("读取日报配置文件失败: {}", e),
    }
}

/// 静默从 JSON 配置文件读取并同步到 YAML
fn load_config_from_json_silent(config_path: &Path, config: &mut YamlConfig) {
    if !config_path.exists() {
        let now = Local::now().date_naive();
        let last_day = now + chrono::Duration::days(6);
        update_config_files_silent(1, &last_day, config_path, config);
        return;
    }

    if let Ok(content) = fs::read_to_string(config_path)
        && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
    {
        let last_day = json.get("last_day").and_then(|v| v.as_str()).unwrap_or("");
        let week_num = json.get("week_num").and_then(|v| v.as_i64()).unwrap_or(1);

        if let Some(last_day_date) = parse_date(last_day) {
            update_config_files_silent(week_num as i32, &last_day_date, config_path, config);
        }
    }
}
