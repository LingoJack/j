use crate::config::YamlConfig;
use crate::constants::{section, config_key, REPORT_DATE_FORMAT, REPORT_SIMPLE_DATE_FORMAT, DEFAULT_CHECK_LINES};
use crate::util::fuzzy;
use crate::{error, info, usage};
use chrono::{Local, NaiveDate};
use colored::Colorize;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const DATE_FORMAT: &str = REPORT_DATE_FORMAT;
const SIMPLE_DATE_FORMAT: &str = REPORT_SIMPLE_DATE_FORMAT;

// ========== report 命令 ==========

/// 处理 report 命令: j report <content...> 或 j r-meta new [date] / j r-meta sync [date]
pub fn handle_report(sub: &str, content: &[String], config: &mut YamlConfig) {
    if content.is_empty() {
        usage!("j report <content> | j r-meta new [date] | j r-meta sync [date]");
        return;
    }

    let first = content[0].as_str();

    // 元数据操作
    if sub == "r-meta" {
        match first {
            "new" => {
                let date_str = content.get(1).map(|s| s.as_str());
                handle_week_update(date_str, config);
            }
            "sync" => {
                let date_str = content.get(1).map(|s| s.as_str());
                handle_sync(date_str, config);
            }
            _ => {
                error!("❌ 未知的元数据操作: {}，可选: new, sync", first);
            }
        }
        return;
    }

    // 常规日报写入
    let text = content.join(" ");
    let text = text.trim().trim_matches('"').to_string();

    if text.is_empty() {
        error!("⚠️ 内容为空，无法写入");
        return;
    }

    handle_daily_report(&text, config);
}

/// 写入日报
fn handle_daily_report(content: &str, config: &mut YamlConfig) {
    let report_path = match config.get_property(section::REPORT, config_key::WEEK_REPORT) {
        Some(p) => p.clone(),
        None => {
            error!("❌ 配置文件中未设置 report.week_report 路径");
            return;
        }
    };

    info!("📂 从配置文件中读取到路径：{}", report_path);

    let report_file = Path::new(&report_path);
    if !report_file.exists() {
        error!("❌ 路径不存在：{}", report_path);
        return;
    }

    let work_dir = report_file.parent().unwrap();
    let config_path = work_dir.join("settings.json");

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

    match last_day {
        Some(last_day) => {
            if now > last_day {
                // 进入新的一周
                let next_last_day = now + chrono::Duration::days(6);
                let new_week_title = format!(
                    "# Week{}[{}-{}]\n",
                    week_num,
                    now.format(DATE_FORMAT),
                    next_last_day.format(DATE_FORMAT)
                );
                update_config_files(week_num + 1, &next_last_day, &config_path, config);
                append_to_file(report_file, &new_week_title);
            }
        }
        None => {
            error!("❌ 无法解析 last_day 日期: {}", last_day_str);
            return;
        }
    }

    let today_str = now.format(SIMPLE_DATE_FORMAT);
    let log_entry = format!("- 【{}】 {}\n", today_str, content);
    append_to_file(report_file, &log_entry);
    info!("✅ 成功将内容写入：{}", report_path);
}

/// 处理 r-meta new 命令：开启新的一周
fn handle_week_update(date_str: Option<&str>, config: &mut YamlConfig) {
    let report_path = match config.get_property(section::REPORT, config_key::WEEK_REPORT) {
        Some(p) => p.clone(),
        None => {
            error!("❌ 配置文件中未设置 report.week_report 路径");
            return;
        }
    };

    let report_file = Path::new(&report_path);
    let config_path = report_file.parent().unwrap().join("settings.json");

    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    let last_day_str = date_str
        .map(|s| s.to_string())
        .or_else(|| config.get_property(section::REPORT, config_key::LAST_DAY).cloned())
        .unwrap_or_default();

    match parse_date(&last_day_str) {
        Some(last_day) => {
            let next_last_day = last_day + chrono::Duration::days(7);
            update_config_files(week_num + 1, &next_last_day, &config_path, config);
        }
        None => {
            error!("❌ 更新周数失败，请检查日期字符串是否有误: {}", last_day_str);
        }
    }
}

/// 处理 r-meta sync 命令：同步周数和日期
fn handle_sync(date_str: Option<&str>, config: &mut YamlConfig) {
    let report_path = match config.get_property(section::REPORT, config_key::WEEK_REPORT) {
        Some(p) => p.clone(),
        None => {
            error!("❌ 配置文件中未设置 report.week_report 路径");
            return;
        }
    };

    let report_file = Path::new(&report_path);
    let config_path = report_file.parent().unwrap().join("settings.json");

    load_config_from_json_and_sync(&config_path, config);

    let week_num = config
        .get_property(section::REPORT, config_key::WEEK_NUM)
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(1);

    let last_day_str = date_str
        .map(|s| s.to_string())
        .or_else(|| config.get_property(section::REPORT, config_key::LAST_DAY).cloned())
        .unwrap_or_default();

    match parse_date(&last_day_str) {
        Some(last_day) => {
            update_config_files(week_num, &last_day, &config_path, config);
        }
        None => {
            error!("❌ 更新周数失败，请检查日期字符串是否有误: {}", last_day_str);
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

    // 更新 YAML 配置
    config.set_property(section::REPORT, config_key::WEEK_NUM, &week_num.to_string());
    config.set_property(section::REPORT, config_key::LAST_DAY, &last_day_str);
    info!(
        "✅ 更新YAML配置文件成功：周数 = {}, 周结束日期 = {}",
        week_num, last_day_str
    );

    // 更新 JSON 配置
    if config_path.exists() {
        let json = serde_json::json!({
            "week_num": week_num,
            "last_day": last_day_str
        });
        match fs::write(config_path, json.to_string()) {
            Ok(_) => info!(
                "✅ 更新JSON配置文件成功：周数 = {}, 周结束日期 = {}",
                week_num, last_day_str
            ),
            Err(e) => error!("❌ 更新JSON配置文件时出错: {}", e),
        }
    }
}

/// 从 JSON 配置文件读取并同步到 YAML
fn load_config_from_json_and_sync(config_path: &Path, config: &mut YamlConfig) {
    if !config_path.exists() {
        error!("❌ 日报配置文件不存在：{:?}", config_path);
        return;
    }

    match fs::read_to_string(config_path) {
        Ok(content) => {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let last_day = json
                    .get("last_day")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let week_num = json.get("week_num").and_then(|v| v.as_i64()).unwrap_or(1);

                info!(
                    "✅ 从日报配置文件中读取到：last_day = {}, week_num = {}",
                    last_day, week_num
                );

                if let Some(last_day_date) = parse_date(last_day) {
                    update_config_files(week_num as i32, &last_day_date, config_path, config);
                }
            } else {
                error!("❌ 解析日报配置文件时出错");
            }
        }
        Err(e) => error!("❌ 读取日报配置文件失败: {}", e),
    }
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, DATE_FORMAT).ok()
}

fn append_to_file(path: &Path, content: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    match OpenOptions::new().create(true).append(true).open(path) {
        Ok(mut f) => {
            if let Err(e) = f.write_all(content.as_bytes()) {
                error!("❌ 写入文件失败: {}", e);
            }
        }
        Err(e) => error!("❌ 打开文件失败: {}", e),
    }
}

// ========== check 命令 ==========

/// 处理 check 命令: j check [line_count]
pub fn handle_check(line_count: Option<&str>, config: &YamlConfig) {
    let num = match line_count {
        Some(s) => match s.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => {
                error!("❌ 无效的行数参数: {}，请输入正整数", s);
                return;
            }
        },
        None => DEFAULT_CHECK_LINES,
    };

    let report_path = match config.get_property(section::REPORT, config_key::WEEK_REPORT) {
        Some(p) => p.clone(),
        None => {
            error!("❌ 配置文件中未设置 report.week_report 路径");
            return;
        }
    };

    info!("📂 正在读取周报文件路径: {}", report_path);

    let path = Path::new(&report_path);
    if !path.exists() || !path.is_file() {
        error!("❌ 文件不存在或不是有效文件: {}", report_path);
        return;
    }

    let lines = read_last_n_lines(path, num);
    info!("📄 最近的 {} 行内容如下：", lines.len());
    for line in &lines {
        info!("{}", line);
    }
}

// ========== search 命令 ==========

/// 处理 search 命令: j search <line_count|all> <target> [-f|-fuzzy]
pub fn handle_search(line_count: &str, target: &str, fuzzy_flag: Option<&str>, config: &YamlConfig) {
    let num = if line_count == "all" {
        usize::MAX
    } else {
        match line_count.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => {
                error!("❌ 无效的行数参数: {}，请输入正整数或 all", line_count);
                return;
            }
        }
    };

    let report_path = match config.get_property(section::REPORT, config_key::WEEK_REPORT) {
        Some(p) => p.clone(),
        None => {
            error!("❌ 配置文件中未设置 report.week_report 路径");
            return;
        }
    };

    info!("📂 正在读取周报文件路径: {}", report_path);

    let path = Path::new(&report_path);
    if !path.exists() || !path.is_file() {
        error!("❌ 文件不存在或不是有效文件: {}", report_path);
        return;
    }

    let is_fuzzy = matches!(fuzzy_flag, Some("-f") | Some("-fuzzy"));
    if is_fuzzy {
        info!("启用模糊匹配...");
    }

    let lines = read_last_n_lines(path, num);
    info!("🔍 搜索目标关键字: {}", target.green());

    let mut index = 0;
    for line in &lines {
        let matched = if is_fuzzy {
            fuzzy::fuzzy_match(line, target)
        } else {
            line.contains(target)
        };

        if matched {
            index += 1;
            let highlighted = fuzzy::highlight_matches(line, target, is_fuzzy);
            info!("[{}] {}", index, highlighted);
        }
    }

    if index == 0 {
        info!("nothing found 😢");
    }
}

/// 从文件尾部读取最后 N 行（高效实现，不需要读取整个文件）
fn read_last_n_lines(path: &Path, n: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let buffer_size: usize = 16384; // 16KB

    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            error!("❌ 读取文件时发生错误: {}", e);
            return lines;
        }
    };

    let file_len = match file.metadata() {
        Ok(m) => m.len() as usize,
        Err(_) => return lines,
    };

    if file_len == 0 {
        return lines;
    }

    // 对于较小的文件或者需要读取全部内容的情况，直接全部读取
    if n == usize::MAX || file_len <= buffer_size * 2 {
        let mut content = String::new();
        let _ = file.seek(SeekFrom::Start(0));
        if file.read_to_string(&mut content).is_ok() {
            let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
            if n >= all_lines.len() {
                return all_lines;
            }
            return all_lines[all_lines.len() - n..].to_vec();
        }
        return lines;
    }

    // 从文件尾部逐块读取
    let mut pointer = file_len;
    let mut remainder = Vec::new();

    while pointer > 0 && lines.len() < n {
        let bytes_to_read = pointer.min(buffer_size);
        pointer -= bytes_to_read;

        let _ = file.seek(SeekFrom::Start(pointer as u64));
        let mut buffer = vec![0u8; bytes_to_read];
        if file.read_exact(&mut buffer).is_err() {
            break;
        }

        // 将 remainder（上次剩余的不完整行）追加到这个块的末尾
        buffer.extend(remainder.drain(..));

        // 从后向前按行分割
        let text = String::from_utf8_lossy(&buffer).to_string();
        let mut block_lines: Vec<&str> = text.split('\n').collect();

        // 第一行可能是不完整的（跨块）
        if pointer > 0 {
            remainder = block_lines.remove(0).as_bytes().to_vec();
        }

        for line in block_lines.into_iter().rev() {
            if !line.is_empty() {
                lines.push(line.to_string());
                if lines.len() >= n {
                    break;
                }
            }
        }
    }

    // 处理文件最开头的那行
    if !remainder.is_empty() && lines.len() < n {
        let line = String::from_utf8_lossy(&remainder).to_string();
        if !line.is_empty() {
            lines.push(line);
        }
    }

    lines.reverse();
    lines
}
