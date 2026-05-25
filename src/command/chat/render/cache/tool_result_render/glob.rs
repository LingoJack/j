//! Glob 工具结果渲染：树形文件列表

use crate::command::chat::constants::NORMAL_RESULT_MAX_LINES;
use crate::command::chat::render::theme::Theme;
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// Glob 结果：树形文件列表
pub(crate) fn render_glob_result(
    content: &str,
    _content_w: usize,
    lines: &mut Vec<Line<'static>>,
    theme: &Theme,
) {
    let mut all_paths: Vec<&str> = content.lines().filter(|l| !l.is_empty()).collect();

    // 跳过第一行统计信息（如 "找到 23 个匹配文件:"）
    if !all_paths.is_empty() && all_paths[0].contains("找到") && all_paths[0].contains("匹配") {
        all_paths.remove(0);
    }

    if all_paths.is_empty() {
        lines.push(Line::from(Span::styled(
            "    (无匹配文件)",
            Style::default().fg(theme.text_dim),
        )));
        return;
    }

    let total = all_paths.len();
    let max_display = NORMAL_RESULT_MAX_LINES.min(30);
    let display_paths = &all_paths[..total.min(max_display)];

    // 收集所有出现的目录（用于区分文件和目录）
    let mut dirs = std::collections::HashSet::new();
    for path in display_paths {
        let mut p = *path;
        while let Some(slash_idx) = p.rfind('/') {
            dirs.insert(&p[..slash_idx]);
            p = &p[..slash_idx];
        }
    }

    // 构建树形结构：提取公共前缀，按层级显示
    let common_prefix = find_common_prefix(display_paths);

    for path in display_paths {
        // 相对于公共前缀显示
        let display_path = if common_prefix.is_empty() {
            *path
        } else if path.starts_with(&common_prefix) {
            &path[common_prefix.len()..]
        } else {
            *path
        };

        let indent = display_path.matches('/').count() * 2 + 4;
        let indent_str = " ".repeat(indent);
        let name = path.rsplit('/').next().unwrap_or(path);

        if dirs.contains(path) {
            // 目录：高亮 + 斜杠后缀
            lines.push(Line::from(vec![
                Span::styled(indent_str, Style::default()),
                Span::styled(
                    format!("{}/", name),
                    Style::default()
                        .fg(theme.config_title)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else {
            // 文件：普通颜色
            lines.push(Line::from(vec![
                Span::styled(indent_str, Style::default()),
                Span::styled(name.to_string(), Style::default().fg(theme.text_normal)),
            ]));
        }
    }

    if total > max_display {
        lines.push(Line::from(Span::styled(
            format!("    ... (共 {} 个文件，显示前 {} 个)", total, max_display),
            Style::default().fg(theme.text_dim),
        )));
    } else if total > 1 {
        lines.push(Line::from(Span::styled(
            format!("    (共 {} 个文件)", total),
            Style::default().fg(theme.text_dim),
        )));
    }
}

/// 找出路径列表的最短公共前缀（不含文件名）
pub(crate) fn find_common_prefix(paths: &[&str]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let first = paths[0];
    let first_dir = first.rfind('/').map(|i| &first[..i + 1]).unwrap_or("");

    for path in paths {
        let mut common_len = 0;
        for (c1, c2) in first_dir.chars().zip(path.chars()) {
            if c1 == c2 {
                common_len += c1.len_utf8();
            } else {
                break;
            }
        }
        if common_len == 0 {
            return String::new();
        }
        // 确保 common_len 处停在 '/' 处
        let candidate = &first_dir[..common_len];
        if !candidate.ends_with('/') {
            if let Some(slash) = candidate.rfind('/') {
                return candidate[..slash + 1].to_string();
            }
            return String::new();
        }
    }
    first_dir.to_string()
}
