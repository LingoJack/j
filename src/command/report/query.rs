//! 日报查询：check / search 命令。

use crate::config::YamlConfig;
use crate::constants::DEFAULT_CHECK_LINES;
use crate::util::fuzzy;
use crate::{error, info};
use colored::Colorize;
use std::path::Path;

use super::io::{get_report_path, read_last_n_lines};
use super::write::handle_open_report;

// ========== check 命令 ==========

/// 处理 check 命令: j check [line_count]
pub fn handle_check(line_count: Option<&str>, config: &YamlConfig) {
    // 检查是否是 open 子命令
    if line_count == Some("open") {
        handle_open_report(config);
        return;
    }

    let num = match line_count {
        Some(s) => match s.parse::<usize>() {
            Ok(n) if n > 0 => n,
            _ => {
                error!("无效的行数参数: {}，请输入正整数或 open", s);
                return;
            }
        },
        None => DEFAULT_CHECK_LINES,
    };

    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    info!("正在读取周报文件路径: {}", report_path);

    let path = Path::new(&report_path);
    if !path.is_file() {
        error!("文件不存在或不是有效文件: {}", report_path);
        return;
    }

    let lines = read_last_n_lines(path, num);
    info!("最近的 {} 行内容如下：", lines.len());
    let md_content = lines.join("\n");
    crate::md!("{}", md_content);
}

// ========== search 命令 ==========

/// 处理 search 命令: j search <line_count|all> <target...> [-f|--fuzzy]
pub fn handle_search(line_count: &str, target: &str, is_fuzzy: bool, config: &YamlConfig) {
    let num = parse_line_count(line_count);

    let report_path = match get_report_path(config) {
        Some(p) => p,
        None => return,
    };

    info!("正在读取周报文件路径: {}", report_path);

    let path = Path::new(&report_path);
    if !path.is_file() {
        error!("文件不存在或不是有效文件: {}", report_path);
        return;
    }

    if is_fuzzy {
        info!("启用模糊匹配...");
    }

    let lines = read_last_n_lines(path, num);
    info!("搜索目标关键字: {}", target.green());

    let match_count = search_in_lines(&lines, target, is_fuzzy);

    if match_count == 0 {
        info!("nothing found");
    }
}

/// 解析行数参数，"all" 映射为 usize::MAX
fn parse_line_count(line_count: &str) -> usize {
    if line_count == "all" {
        usize::MAX
    } else {
        line_count
            .parse::<usize>()
            .ok()
            .filter(|&n| n > 0)
            .unwrap_or(DEFAULT_CHECK_LINES)
    }
}

/// 在行列表中搜索匹配项并输出，返回匹配总数
fn search_in_lines(lines: &[String], target: &str, is_fuzzy: bool) -> usize {
    let mut index = 0;
    for line in lines {
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
    index
}
