//! 日报合并：将另一个仓库 / 本地路径的日报增量合并并写回主日报文件。
//!
//! 合并策略（增量去重）：
//! - 按周的日期范围（`range_str`）匹配两个文件中的周
//! - 匹配到的周：逐条比较，只追加目标周中不存在的条目（按内容去重，忽略空白差异）
//! - 未匹配的周：作为新周插入，最终按起始日期排序并重新编号
//! - 若源日报所有条目均已存在，则不做任何写入
//! - 合并前自动备份原文件为 `.md.bak`，然后将结果直接写回主日报文件
//!
//! 多行条目处理：
//! - 带 `【日期】` 前缀的行（如 `- 【2024/12/02】 内容`）开启新条目块
//! - 从该日期行到下一个日期行之前的所有内容（子列表、补充说明、无日期普通行等）都属于该日期的块
//! - 合并时以「条目块」为原子单位，确保多行条目不会被拆散

use crate::config::YamlConfig;
use crate::constants::{REPORT_DATE_FORMAT, REPORT_DEFAULT_FILE};
use crate::{error, info, usage};
use chrono::NaiveDate;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use super::io::get_report_path;

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
pub fn handle_merge(source: Option<&str>, config: &YamlConfig) {
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

    // 获取源日报内容
    let source_content = match fetch_source_report(source, config) {
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

    // 合并（增量去重）
    let (merged, added) = merge_weeks(&mut main_weeks, source_weeks);

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
        }
        Err(e) => error!("写入主日报文件失败: {}", e),
    }
}

// ========== 源日报获取 ==========

/// 获取源日报内容：远程 URL 则 clone 到临时目录，本地路径则直接读取。
fn fetch_source_report(source: &str, config: &YamlConfig) -> Option<String> {
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

/// 从远程仓库 clone 后读取日报文件（clone 到临时目录，读取后自动清理）。
fn fetch_remote_report(url: &str, config: &YamlConfig) -> Option<String> {
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
    let result = fs::read_to_string(&report_path).ok().or_else(|| {
        let fallback = tmp_dir.join(REPORT_DEFAULT_FILE);
        fs::read_to_string(&fallback).ok()
    });

    // 清理临时目录
    let _ = fs::remove_dir_all(&tmp_dir);

    match result {
        Some(content) => {
            info!("成功读取远程日报");
            Some(content)
        }
        None => {
            error!("远程仓库中未找到日报文件: {}", report_file_name);
            None
        }
    }
}

/// 从本地路径读取日报文件。
///
/// 路径为目录时在其下查找同名 `week_report.md`；为文件时直接读取。
fn fetch_local_report(path_str: &str, config: &YamlConfig) -> Option<String> {
    let path = Path::new(path_str);
    let report_file_name = config
        .report_file_path()
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| REPORT_DEFAULT_FILE.to_string());

    let report_path = if path.is_dir() {
        let candidate = path.join(&report_file_name);
        if candidate.exists() {
            candidate
        } else {
            let fallback = path.join(REPORT_DEFAULT_FILE);
            if fallback.exists() {
                fallback
            } else {
                error!("目录下未找到日报文件: {}", report_file_name);
                return None;
            }
        }
    } else if path.is_file() {
        path.to_path_buf()
    } else {
        error!("路径不存在: {}", path_str);
        return None;
    };

    match fs::read_to_string(&report_path) {
        Ok(content) => {
            info!("已读取本地日报: {}", report_path.display());
            Some(content)
        }
        Err(e) => {
            error!("读取本地日报文件失败: {}", e);
            None
        }
    }
}

// ========== 解析与合并 ==========

/// 解析日报文件内容为 (前导文本, 周区块列表)。
///
/// - `# Week{n}[start-end]` 行开启新周区块
/// - 周标题之前的非空行作为前导文本（preamble）
/// - 周区块内的行按「条目块」分组：顶格行（无缩进）开启新块，缩进行 / 空行归入当前块
fn parse_report(content: &str) -> (String, Vec<WeekSection>) {
    let re = week_header_re();
    let mut preamble_lines: Vec<&str> = Vec::new();
    let mut weeks: Vec<WeekSection> = Vec::new();
    let mut current: Option<WeekSection> = None;
    let mut found_first_week = false;

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
            if !line.trim().is_empty() {
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
    } else {
        // 周区块内首行不是日期条目：单独成块
        entries.push(vec![line.to_string()]);
    }
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

/// 合并：把 source 的周条目增量合并到 target 中匹配的周（按 range_str 匹配）；
/// 已存在的条目（按内容去重）跳过，只追加新增条目；
/// 未匹配的周作为新周加入。最终按起始日期排序并重新编号。
///
/// 返回 `(合并后的周列表, 新增条目数)`。
fn merge_weeks(
    target: &mut Vec<WeekSection>,
    source: Vec<WeekSection>,
) -> (Vec<WeekSection>, usize) {
    let mut new_weeks: Vec<WeekSection> = Vec::new();
    let mut added_count: usize = 0;

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
            // 未匹配：作为新周
            added_count += src_week.entries.len();
            new_weeks.push(src_week);
        }
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

    // 重新编号 Week1, Week2, ...
    for (i, w) in all.iter_mut().enumerate() {
        w.header_line = renumber_header(&w.header_line, (i + 1) as i32);
    }

    (all, added_count)
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
