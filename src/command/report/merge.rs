//! 日报合并：将另一个仓库 / 本地路径的日报增量合并并写回主日报文件。
//!
//! 合并策略（增量去重）：
//! - 按周的日期范围（`range_str`）匹配两个文件中的周
//! - 匹配到的周：逐条比较，只追加目标周中不存在的条目（按内容去重，忽略空白差异）
//! - 未匹配的周：作为新周插入，编号从当前 `week_num` 起递增；已有周保留原始编号
//! - 若源日报所有条目均已存在，则不做任何写入
//! - 合并前自动备份原文件为 `.md.bak`，然后将结果直接写回主日报文件
//! - 合并后同步 `settings.json`：`week_num` 仅在新增整周时递增，`last_day` 仅在最后一周变更时更新
//!
//! 多行条目处理：
//! - 带 `【日期】` 前缀的行（如 `- 【2024/12/02】 内容`）开启新条目块
//! - 从该日期行到下一个日期行之前的所有内容（子列表、补充说明、无日期普通行等）都属于该日期的块
//! - 合并时以「条目块」为原子单位，确保多行条目不会被拆散

use crate::config::YamlConfig;
use crate::constants::{REPORT_DATE_FORMAT, REPORT_DEFAULT_FILE, REPORT_SIMPLE_DATE_FORMAT};
use crate::{error, info, usage};
use chrono::NaiveDate;
use chrono::{Datelike, Duration};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use super::io::{get_report_path, get_settings_json_path};
use super::write::{read_settings, update_config_files_silent};

/// 一个周区块：标题行 + 该周的所有条目块。
struct WeekSection {
    /// 日期范围字符串，如 "2024.01.01-2024.01.07"，用作周匹配 key
    range_str: String,
    /// 周起始日期，用于排序
    start_date: Option<NaiveDate>,
    /// 原始标题行，如 "# Week3[2024.01.01-2024.01.07]"
    header_line: String,
    /// 该周的所有条目块，每个块是一个逻辑条目（主行 + 子列表 + 补充说明等续行）
    entries: Vec<Vec<String>>,
}

/// 编译好的周标题正则（惰性初始化）
fn week_header_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // 匹配 "# Week3[2024.01.01-2024.01.07]"，捕获周编号和起止日期
        Regex::new(r"^#\s+Week\s*(\d+)\s*\[\s*([\d.]+)\s*-\s*([\d.]+)\s*\]").unwrap()
    })
}

/// 编译好的日期条目正则（惰性初始化）
fn date_entry_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // 匹配 "- 【2024/12/02】 内容" 或 "【2024/12/02】 内容"
        Regex::new(r"^\s*[-*+]?\s*【\d{4}/\d{1,2}/\d{1,2}】").unwrap()
    })
}

// ========== 对外公共接口 ==========

/// 处理 reportctl merge 命令：合并另一个仓库 / 路径的日报。
///
/// `source` 为远程 git URL 时自动 clone 到临时目录；为本地路径时直接读取。
pub fn handle_merge(source: Option<&str>, config: &mut YamlConfig) {
    let source = match source {
        Some(s) if !s.trim().is_empty() => s.trim(),
        _ => {
            usage!("j reportctl merge <repo_url|local_path>");
            return;
        }
    };

    // 读取主日报文件
    let main_path = match get_report_path(config) {
        Some(p) => p,
        None => {
            error!("无法获取主日报文件路径");
            return;
        }
    };

    let main_content = match fs::read_to_string(&main_path) {
        Ok(c) => c,
        Err(e) => {
            error!("读取主日报文件失败: {}", e);
            return;
        }
    };

    // 获取源日报内容及源 settings.json
    let (source_content, source_settings) = match fetch_source_report(source, config) {
        Some(c) => c,
        None => return,
    };

    // 解析两个日报
    let (main_preamble, mut main_weeks) = parse_report(&main_content);
    let (_src_preamble, source_weeks) = parse_report(&source_content);

    if source_weeks.is_empty() {
        error!("源日报文件未包含任何周数据");
        return;
    }

    // 读取主 settings.json 中的 week_num 和 last_day
    let settings_path = get_settings_json_path(&main_path);
    let (original_week_num, main_last_day) = match &settings_path {
        Some(p) => {
            let s = read_settings(p);
            (s.week_num, s.last_day)
        }
        None => (1, None),
    };

    // 从源 settings.json 中读取 last_day
    let source_last_day = source_settings
        .as_ref()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(content).ok())
        .and_then(|json| {
            json.get("last_day")
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .and_then(|s| parse_dot_date(&s));

    // 合并（增量去重），新周从 original_week_num 开始编号
    let (merged, added, new_weeks_count) =
        merge_weeks(&mut main_weeks, source_weeks, original_week_num);

    if added == 0 {
        info!("源日报的所有条目已存在，无需合并");
        return;
    }

    // 渲染输出
    let output = render_report(&main_preamble, &merged);

    // 备份原日报文件，然后将合并结果直接写回
    let backup_path = build_backup_path(&main_path);
    match fs::copy(&main_path, &backup_path) {
        Ok(_) => {
            info!("已备份原日报到: {}", backup_path.display());
        }
        Err(e) => {
            error!("备份原日报失败: {}，合并已中止", e);
            return;
        }
    }

    match fs::write(&main_path, &output) {
        Ok(_) => {
            info!(
                "合并完成，新增 {} 条条目，已写回主日报文件: {}",
                added, main_path
            );
            info!(
                "共 {} 个周（原文件备份在 {}）",
                merged.len(),
                backup_path.display()
            );

            // 同步 settings.json
            sync_settings_after_merge(
                &settings_path,
                original_week_num,
                new_weeks_count,
                main_last_day,
                source_last_day,
            );
        }
        Err(e) => error!("写入主日报文件失败: {}", e),
    }
}

// ========== 源日报获取 ==========

/// 合并完成后同步 settings.json：
/// - `week_num` = 原始值 + 新增周数（仅当新增周数 > 0 时才变化）
/// - `last_day` = max(主 settings.json 的 last_day, 源 settings.json 的 last_day)，
///   仅当比原始值更晚时才变化
///
/// 只有任一字段实际变化时才写入。
fn sync_settings_after_merge(
    settings_path: &Option<PathBuf>,
    original_week_num: i32,
    new_weeks_count: usize,
    main_last_day: Option<NaiveDate>,
    source_last_day: Option<NaiveDate>,
) {
    let settings_path = match settings_path {
        Some(p) => p.as_path(),
        None => {
            info!("无法获取 settings.json 路径，跳过配置同步");
            return;
        }
    };

    // 取两边 last_day 的较大值
    let new_last_day = match (main_last_day, source_last_day) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };

    // 计算新的 week_num
    let new_week_num = original_week_num + new_weeks_count as i32;

    // 判断是否需要更新
    let week_num_changed = new_weeks_count > 0;
    let last_day_changed = match (new_last_day, main_last_day) {
        (Some(new), Some(orig)) => new > orig,
        (Some(_), None) => true,
        (None, _) => false,
    };

    if !week_num_changed && !last_day_changed {
        return; // 无变化，不写
    }

    let last_day_to_write = match new_last_day {
        Some(day) => day,
        None => {
            info!("缺少 last_day，跳过 settings.json 同步");
            return;
        }
    };

    update_config_files_silent(new_week_num, &last_day_to_write, settings_path);
    info!(
        "已同步配置：week_num = {}{}，last_day = {}",
        new_week_num,
        if week_num_changed {
            format!("（+{}）", new_weeks_count)
        } else {
            String::new()
        },
        last_day_to_write.format(REPORT_DATE_FORMAT)
    );
}

/// 获取源日报内容及源 settings.json 内容：
/// 远程 URL 则 clone 到临时目录，本地路径则直接读取。
/// 返回 `(日报内容, 源 settings.json 内容)`。
fn fetch_source_report(source: &str, config: &YamlConfig) -> Option<(String, Option<String>)> {
    if is_remote_url(source) {
        fetch_remote_report(source, config)
    } else {
        fetch_local_report(source, config)
    }
}

/// 判断是否为远程 git URL
fn is_remote_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://") || s.starts_with("git@")
}

/// 从远程仓库 clone 后读取日报文件及 settings.json（clone 到临时目录，读取后自动清理）。
/// 返回 `(日报内容, 源 settings.json 内容)`。
fn fetch_remote_report(url: &str, config: &YamlConfig) -> Option<(String, Option<String>)> {
    let report_file_name = config
        .report_file_path()
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| REPORT_DEFAULT_FILE.to_string());

    let tmp_dir = std::env::temp_dir().join(format!("j_report_merge_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);

    info!("正在从远程仓库克隆: {}", url);

    // 优先 clone main 分支，失败则尝试 master
    let status = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "-b",
            "main",
            url,
            &tmp_dir.to_string_lossy(),
        ])
        .status();

    let success = match status {
        Ok(s) if s.success() => true,
        Ok(_) => {
            let _ = fs::remove_dir_all(&tmp_dir);
            let s2 = Command::new("git")
                .args([
                    "clone",
                    "--depth",
                    "1",
                    "-b",
                    "master",
                    url,
                    &tmp_dir.to_string_lossy(),
                ])
                .status();
            matches!(s2, Ok(s) if s.success())
        }
        Err(e) => {
            error!("执行 git clone 失败: {}", e);
            let _ = fs::remove_dir_all(&tmp_dir);
            return None;
        }
    };

    if !success {
        error!("git clone 失败，请检查仓库地址和网络连接");
        let _ = fs::remove_dir_all(&tmp_dir);
        return None;
    }

    // 读取日报文件（优先同名，回退到默认文件名）
    let report_path = tmp_dir.join(&report_file_name);
    let report_result = fs::read_to_string(&report_path).ok().or_else(|| {
        let fallback = tmp_dir.join(REPORT_DEFAULT_FILE);
        fs::read_to_string(&fallback).ok()
    });

    // 读取源 settings.json
    let settings_result = fs::read_to_string(tmp_dir.join("settings.json")).ok();

    // 清理临时目录
    let _ = fs::remove_dir_all(&tmp_dir);

    match report_result {
        Some(content) => {
            info!("成功读取远程日报");
            Some((content, settings_result))
        }
        None => {
            error!("远程仓库中未找到日报文件: {}", report_file_name);
            None
        }
    }
}

/// 从本地路径读取日报文件及 settings.json。
///
/// 路径为目录时在其下查找同名 `week_report.md`；为文件时直接读取。
/// 返回 `(日报内容, 源 settings.json 内容)`。
fn fetch_local_report(path_str: &str, config: &YamlConfig) -> Option<(String, Option<String>)> {
    let path = Path::new(path_str);
    let report_file_name = config
        .report_file_path()
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| REPORT_DEFAULT_FILE.to_string());

    let (report_path, dir_path) = if path.is_dir() {
        let candidate = path.join(&report_file_name);
        if candidate.exists() {
            (candidate, path.to_path_buf())
        } else {
            let fallback = path.join(REPORT_DEFAULT_FILE);
            if fallback.exists() {
                (fallback, path.to_path_buf())
            } else {
                error!("目录下未找到日报文件: {}", report_file_name);
                return None;
            }
        }
    } else if path.is_file() {
        let dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        (path.to_path_buf(), dir)
    } else {
        error!("路径不存在: {}", path_str);
        return None;
    };

    let report_content = match fs::read_to_string(&report_path) {
        Ok(content) => {
            info!("已读取本地日报: {}", report_path.display());
            content
        }
        Err(e) => {
            error!("读取本地日报文件失败: {}", e);
            return None;
        }
    };

    // 读取同目录下的 settings.json
    let settings_content = fs::read_to_string(dir_path.join("settings.json")).ok();

    Some((report_content, settings_content))
}

// ========== 解析与合并 ==========

/// 解析日报文件内容为 (前导文本, 周区块列表)。
///
/// - `# Week{n}[start-end]` 行开启新周区块
/// - 周标题之前的非空行作为前导文本（preamble）
/// - 周区块内的行按「条目块」分组：顶格行（无缩进）开启新块，缩进行 / 空行归入当前块
///
/// **无周标题的文件**：如果文件只有日期条目（`- 【2026/08/03】 ...`）而没有
/// `# WeekN[...]` 周标题行，会自动按条目日期推算所属周（周一 ~ 周日），
/// 生成合成周标题后正常参与合并。
fn parse_report(content: &str) -> (String, Vec<WeekSection>) {
    let re = week_header_re();
    let mut preamble_lines: Vec<&str> = Vec::new();
    let mut weeks: Vec<WeekSection> = Vec::new();
    let mut current: Option<WeekSection> = None;
    let mut found_first_week = false;
    // 周标题出现之前的日期条目（无周标题文件的场景）
    let mut orphan_entries: Vec<Vec<String>> = Vec::new();

    for line in content.lines() {
        if let Some(caps) = re.captures(line) {
            if let Some(mut w) = current.take() {
                trim_trailing_empty(&mut w.entries);
                weeks.push(w);
            }
            found_first_week = true;
            let range_str = format!("{}-{}", &caps[2], &caps[3]);
            let start_date = parse_dot_date(&caps[2]);
            current = Some(WeekSection {
                range_str,
                start_date,
                header_line: line.to_string(),
                entries: Vec::new(),
            });
        } else if !found_first_week {
            if date_entry_re().is_match(line) {
                // 周标题前的日期条目：收集为 orphan，稍后按日期分组
                orphan_entries.push(vec![line.to_string()]);
            } else if let Some(last) = orphan_entries.last_mut() {
                last.push(line.to_string());
            } else if !line.trim().is_empty() {
                preamble_lines.push(line);
            }
        } else if let Some(w) = current.as_mut() {
            group_entry_line(&mut w.entries, line);
        }
    }

    if let Some(mut w) = current.take() {
        trim_trailing_empty(&mut w.entries);
        weeks.push(w);
    }

    // 将 orphan 条目按日期分组为合成周，插入到 weeks 前面（它们出现在第一个周标题之前）
    if !orphan_entries.is_empty() {
        trim_trailing_empty(&mut orphan_entries);
        let mut orphan_weeks = group_orphan_entries_to_weeks(orphan_entries);
        orphan_weeks.extend(weeks);
        weeks = orphan_weeks;
    }

    let preamble = if preamble_lines.is_empty() {
        String::new()
    } else {
        let mut s = preamble_lines.join("\n");
        s.push('\n');
        s
    };

    (preamble, weeks)
}

/// 将一行归入条目块列表（按日期分组）。
///
/// - 带 `【YYYY/MM/DD】` 日期前缀的行开启新条目块
/// - 其他行（子列表、补充说明、无日期普通行、空行等）归入当前条目块
fn group_entry_line(entries: &mut Vec<Vec<String>>, line: &str) {
    if date_entry_re().is_match(line) {
        entries.push(vec![line.to_string()]);
    } else if let Some(last) = entries.last_mut() {
        last.push(line.to_string());
    } else if !line.trim().is_empty() {
        // 周区块内首行不是日期条目且非空行：单独成块
        entries.push(vec![line.to_string()]);
    }
    // 空行且无已有条目：忽略（周标题后的分隔空行）
}

/// 去除每个条目块末尾的空行（条目间的分隔空行不属于条目内容）。
fn trim_trailing_empty(entries: &mut Vec<Vec<String>>) {
    for block in entries.iter_mut() {
        while block.last().map_or(false, |l| l.trim().is_empty()) {
            block.pop();
        }
    }
}

/// 解析点分日期 "2024.01.01" -> NaiveDate
fn parse_dot_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, REPORT_DATE_FORMAT).ok()
}

/// 从条目行中提取日期，如 `- 【2026/08/03】 内容` -> `2026-08-03`
fn parse_entry_date(line: &str) -> Option<NaiveDate> {
    let re = date_entry_re();
    let caps = re.captures(line)?;
    // caps[0] 匹配到 `【YYYY/MM/DD】` 的前缀部分（含可能的 `- ` 前导）
    let matched = &caps[0];
    let start = matched.find('【')?;
    let after_open = &matched[start + '【'.len_utf8()..];
    let end = after_open.find('】')?;
    let date_str = &after_open[..end];
    NaiveDate::parse_from_str(date_str, REPORT_SIMPLE_DATE_FORMAT).ok()
}

/// 计算给定日期所在周的周一 ~ 周日范围
fn week_range(date: NaiveDate) -> (NaiveDate, NaiveDate) {
    let monday = date - Duration::days(date.weekday().num_days_from_monday() as i64);
    let sunday = monday + Duration::days(6);
    (monday, sunday)
}

/// 将无周标题的 orphan 条目按日期分组为合成周区块。
///
/// 每条目根据其 `【YYYY/MM/DD】` 日期推算所属周（周一 ~ 周日），
/// 同一周的条目归入同一个 `WeekSection`，并生成合成标题 `# Week0[start-end]`。
fn group_orphan_entries_to_weeks(entries: Vec<Vec<String>>) -> Vec<WeekSection> {
    use std::collections::BTreeMap;

    // 按周起始日期分组
    let mut groups: BTreeMap<NaiveDate, WeekSection> = BTreeMap::new();

    for entry in entries {
        let first_line = entry.first().map(|s| s.as_str()).unwrap_or("");
        let date = match parse_entry_date(first_line) {
            Some(d) => d,
            None => {
                // 无法解析日期的条目归入一个占位周（日期为 None）
                let key = NaiveDate::from_ymd_opt(1900, 1, 1).unwrap();
                let week = groups.entry(key).or_insert_with(|| WeekSection {
                    range_str: String::new(),
                    start_date: None,
                    header_line: String::new(),
                    entries: Vec::new(),
                });
                week.entries.push(entry);
                continue;
            }
        };

        let (monday, sunday) = week_range(date);
        let range_str = format!(
            "{}-{}",
            monday.format(REPORT_DATE_FORMAT),
            sunday.format(REPORT_DATE_FORMAT)
        );

        let week = groups.entry(monday).or_insert_with(|| WeekSection {
            range_str: range_str.clone(),
            start_date: Some(monday),
            header_line: format!(
                "# Week0[{}-{}]",
                monday.format(REPORT_DATE_FORMAT),
                sunday.format(REPORT_DATE_FORMAT)
            ),
            entries: Vec::new(),
        });
        week.entries.push(entry);
    }

    groups.into_values().collect()
}

/// 合并：把 source 的周条目增量合并到 target 中匹配的周（按 range_str 匹配）；
/// 已存在的条目（按内容去重）跳过，只追加新增条目；
/// 未匹配的周作为新周加入，使用 `next_week_num` 起始编号。
/// 保留 target 已有周的原始编号，不重新编号。
/// 最终按起始日期排序。
///
/// 返回 `(合并后的周列表, 新增条目数, 新增周数)`。
fn merge_weeks(
    target: &mut Vec<WeekSection>,
    source: Vec<WeekSection>,
    next_week_num: i32,
) -> (Vec<WeekSection>, usize, usize) {
    let mut new_weeks: Vec<WeekSection> = Vec::new();
    let mut added_count: usize = 0;
    let mut new_weeks_count: usize = 0;

    for src_week in source {
        if let Some(tw) = target
            .iter_mut()
            .find(|w| w.range_str == src_week.range_str)
        {
            // 匹配到：增量追加，跳过已存在的条目
            let existing: Vec<String> = tw.entries.iter().map(|e| normalize_entry(e)).collect();
            for entry in src_week.entries {
                let key = normalize_entry(&entry);
                if existing.contains(&key) {
                    continue;
                }
                tw.entries.push(entry);
                added_count += 1;
            }
        } else {
            // 未匹配：作为新周，分配编号
            added_count += src_week.entries.len();
            new_weeks_count += 1;
            new_weeks.push(src_week);
        }
    }

    // 给新周分配编号（从 next_week_num 开始递增）
    for (i, w) in new_weeks.iter_mut().enumerate() {
        let num = next_week_num + i as i32;
        w.header_line = renumber_header(&w.header_line, num);
    }

    let mut all = std::mem::take(target);
    all.extend(new_weeks);

    // 按起始日期排序（无日期的排最后，保持原顺序）
    all.sort_by(|a, b| match (a.start_date, b.start_date) {
        (Some(da), Some(db)) => da.cmp(&db),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });

    (all, added_count, new_weeks_count)
}

/// 将条目块归一化为比较 key：每行 trim 后过滤空行，用 `\n` 拼接。
/// 用于去重比较，忽略首尾空白和条目间的空行差异。
fn normalize_entry(entry: &[String]) -> String {
    entry
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// 重新编号周标题行，输出标准格式 `# Week{n}[start-end]`。
fn renumber_header(header: &str, new_num: i32) -> String {
    let re = week_header_re();
    if let Some(caps) = re.captures(header) {
        format!("# Week{}[{}-{}]", new_num, &caps[2], &caps[3])
    } else {
        header.to_string()
    }
}

/// 渲染合并后的日报文本。
fn render_report(preamble: &str, weeks: &[WeekSection]) -> String {
    let mut out = String::new();

    if !preamble.is_empty() {
        out.push_str(preamble);
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push('\n');
    }

    for (i, w) in weeks.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&w.header_line);
        out.push('\n');
        for block in &w.entries {
            for line in block {
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    out
}

/// 构造备份文件路径：`week_report.md` -> `week_report.md.bak`
fn build_backup_path(main_path: &str) -> PathBuf {
    Path::new(main_path).with_extension("md.bak")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_no_week_header() {
        let content = "\
- 【2026/08/03】 jcli 支持了 report merge
- 【2026/08/04】 发现了宝藏工具
- 【2026/08/05】 修复 bug
  - 子任务 A
  - 子任务 B
";

        let (preamble, weeks) = parse_report(content);

        // 无前导文本
        assert!(
            preamble.is_empty(),
            "preamble should be empty, got: {preamble}"
        );
        // 应识别出 1 个周（08/03 周一 ~ 08/09 周日）
        assert_eq!(weeks.len(), 1, "should have 1 week");
        let w = &weeks[0];
        assert_eq!(w.range_str, "2026.08.03-2026.08.09");
        assert_eq!(w.start_date, NaiveDate::from_ymd_opt(2026, 8, 3));
        // 3 个条目
        assert_eq!(w.entries.len(), 3, "should have 3 entries");
        // 第 3 个条目应包含多行
        assert_eq!(w.entries[2].len(), 3, "3rd entry should have 3 lines");
    }

    #[test]
    fn test_parse_mixed_week_header_and_orphan() {
        let content = "\
- 【2026/08/02】 孤儿条目

# Week1[2026.08.03-2026.08.09]

- 【2026/08/03】 周内条目
";

        let (_preamble, weeks) = parse_report(content);

        // 08/02 是上周日，应归入 07/27-08/02 那周
        assert_eq!(weeks.len(), 2, "should have 2 weeks");
        // 第一个周是 orphan 生成的（07/27-08/02）
        assert_eq!(weeks[0].range_str, "2026.07.27-2026.08.02");
        assert_eq!(weeks[0].entries.len(), 1);
        // 第二个周是文件中的 Week1
        assert_eq!(weeks[1].range_str, "2026.08.03-2026.08.09");
        assert_eq!(weeks[1].entries.len(), 1);
    }

    #[test]
    fn test_merge_incremental_dedup() {
        let target_content = "\
# Week1[2026.08.03-2026.08.09]

- 【2026/08/03】 完成登录功能
- 【2026/08/04】 修复导航栏 bug
";

        let source_content = "\
- 【2026/08/03】 完成登录功能
- 【2026/08/04】 修复导航栏 bug
- 【2026/08/05】 代码评审
";

        let (_, mut target_weeks) = parse_report(target_content);
        let (_, source_weeks) = parse_report(source_content);

        // next_week_num = 2（目标已有 Week1）
        let (merged, added, new_weeks) = merge_weeks(&mut target_weeks, source_weeks, 2);

        assert_eq!(added, 1, "should add 1 new entry");
        assert_eq!(new_weeks, 0, "should add 0 new weeks");
        assert_eq!(merged.len(), 1, "should have 1 week");
        assert_eq!(merged[0].entries.len(), 3, "should have 3 entries total");
    }

    #[test]
    fn test_merge_new_week_gets_correct_number() {
        let target_content = "\
# Week133[2026.08.03-2026.08.09]

- 【2026/08/03】 完成登录功能
";

        // 源日报有一个不同日期范围的周
        let source_content = "\
# Week1[2026.08.10-2026.08.16]

- 【2026/08/10】 新周条目
";

        let (_, mut target_weeks) = parse_report(target_content);
        let (_, source_weeks) = parse_report(source_content);

        // next_week_num = 134（目标已有 Week133）
        let (merged, added, new_weeks) = merge_weeks(&mut target_weeks, source_weeks, 134);

        assert_eq!(added, 1, "should add 1 new entry");
        assert_eq!(new_weeks, 1, "should add 1 new week");
        assert_eq!(merged.len(), 2, "should have 2 weeks");

        // 新周应该编号为 Week134，原周保留 Week133
        let week133 = merged
            .iter()
            .find(|w| w.range_str == "2026.08.03-2026.08.09")
            .unwrap();
        assert!(week133.header_line.contains("Week133"));

        let week134 = merged
            .iter()
            .find(|w| w.range_str == "2026.08.10-2026.08.16")
            .unwrap();
        assert!(week134.header_line.contains("Week134"));
    }

    #[test]
    fn test_merge_no_new_weeks_no_renumber() {
        let target_content = "\
# Week133[2026.08.03-2026.08.09]

- 【2026/08/03】 完成登录功能
";

        let source_content = "\
- 【2026/08/03】 完成登录功能
- 【2026/08/04】 修复 bug
";

        let (_, mut target_weeks) = parse_report(target_content);
        let (_, source_weeks) = parse_report(source_content);

        let (merged, added, new_weeks) = merge_weeks(&mut target_weeks, source_weeks, 134);

        assert_eq!(added, 1, "should add 1 new entry");
        assert_eq!(new_weeks, 0, "should add 0 new weeks");
        // 原周编号保留
        assert!(merged[0].header_line.contains("Week133"));
    }
}
