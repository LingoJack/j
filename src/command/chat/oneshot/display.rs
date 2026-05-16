//! oneshot 终端显示工具：工具参数预览、工具调用/结果打印、Markdown 重绘

use crate::command::chat::constants::{THINKING_PULSE_MIN_FACTOR, THINKING_PULSE_PERIOD_MS};
use crate::command::chat::render::theme::ToolCategoryColor;
use crate::command::chat::tools::classification::ToolCategory;
use crate::theme::Theme;
use std::io;
use std::sync::{Arc, Mutex};

/// 工具调用参数最大预览长度（与 TUI TOOL_ARG_PREVIEW_MAX_CHARS 对齐）
const TOOL_ARG_PREVIEW_MAX_CHARS: usize = 60;

/// 从工具调用参数 JSON 中提取描述信息
pub(crate) fn extract_tool_desc(tool_name: &str, arguments: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    match tool_name {
        "Shell" => parsed.get("description")?.as_str().map(|s| s.to_string()),
        "Read" | "Write" | "Edit" | "Glob" | "Grep" => parsed
            .get("path")
            .or_else(|| parsed.get("file_path"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "Agent" | "Teammate" => parsed
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        "Ask" => parsed
            .get("header")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

/// 从 Bash 参数中提取命令
pub(crate) fn extract_bash_command(arguments: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(arguments).ok()?;
    parsed
        .get("command")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// 生成截断后的参数预览
pub(crate) fn make_args_preview(arguments: &str) -> String {
    let total_len = arguments.chars().count();
    if total_len <= TOOL_ARG_PREVIEW_MAX_CHARS {
        return arguments.to_string();
    }
    let closing_bracket = arguments.chars().next().and_then(|c| match c {
        '{' => Some('}'),
        '[' => Some(']'),
        _ => None,
    });
    let preview_len = if closing_bracket.is_some() {
        TOOL_ARG_PREVIEW_MAX_CHARS - 4
    } else {
        TOOL_ARG_PREVIEW_MAX_CHARS
    };
    let preview: String = arguments.chars().take(preview_len).collect();
    if let Some(bracket) = closing_bracket {
        format!("{}...{}", preview, bracket)
    } else {
        format!("{}…", preview)
    }
}

/// 获取终端宽度
pub(crate) fn term_width() -> usize {
    crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
}

/// 计算交互框宽度
pub(crate) fn box_width() -> usize {
    term_width().saturating_sub(4).clamp(40, 80)
}

/// 思考动画脉冲颜色（与 TUI thinking_pulse_color 对齐）。
/// 返回插值后的 ratatui::Color::Rgb，调用方再走 apply_fg 做最终色阶降级。
pub(crate) fn thinking_pulse_color() -> ratatui::style::Color {
    use std::time::{SystemTime, UNIX_EPOCH};

    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    let period = THINKING_PULSE_PERIOD_MS as f64;
    let phase = (millis % period as u128) as f64 / period;
    let t = (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5;

    let theme = Theme::terminal();
    let (r, g, b) = if let ratatui::style::Color::Rgb(r, g, b) = theme.label_ai {
        let min_factor = THINKING_PULSE_MIN_FACTOR;
        let factor = min_factor + (1.0 - min_factor) * t;
        (
            (r as f64 * factor).round().min(255.0) as u8,
            (g as f64 * factor).round().min(255.0) as u8,
            (b as f64 * factor).round().min(255.0) as u8,
        )
    } else {
        // 命名色无 RGB 信息时退化到固定 label_ai 默认色
        (120, 220, 160)
    };
    ratatui::style::Color::Rgb(r, g, b)
}

/// 打印工具调用行（与 TUI 折叠模式对齐: `  {icon} {tool_name}  {desc}`）
pub(crate) fn print_tool_call_line(tool_name: &str, arguments: &str) {
    use colored::Colorize;

    let category = ToolCategory::from_name(tool_name);
    let icon = category.icon();
    let theme = Theme::terminal();
    let tool_color = category.color(&theme);

    let desc = if let Some(d) = extract_tool_desc(tool_name, arguments) {
        d
    } else if !arguments.is_empty() {
        make_args_preview(arguments)
    } else {
        String::new()
    };

    let desc_colored = if desc.is_empty() {
        String::new()
    } else {
        format!("  {}", desc.dimmed())
    };

    eprintln!(
        "  {} {} {}",
        icon,
        crate::util::color_adapt::apply_fg(tool_name, tool_color).bold(),
        desc_colored
    );
}

/// 打印工具执行结果行（与 TUI tool_result 对齐）
pub(crate) fn print_tool_result_line(
    tool_name: &str,
    is_error: bool,
    summary: &str,
    elapsed: &str,
) {
    use colored::Colorize;

    let category = ToolCategory::from_name(tool_name);
    let theme = Theme::terminal();
    let tool_color = category.color(&theme);

    let status_icon = if is_error { "✗" } else { "✓" };
    let status_style = if is_error { "red" } else { "green" };

    eprintln!(
        "  🔧 {} {}{} {}",
        crate::util::color_adapt::apply_fg(tool_name, tool_color).bold(),
        status_icon.color(status_style),
        summary.dimmed(),
        elapsed.dimmed(),
    );
}

/// 在流式输出开始前保存光标位置（终端行号），供 `redraw_markdown_from_saved` 回退使用。
pub(crate) fn save_cursor_row() -> Option<u16> {
    crossterm::cursor::position().map(|(_, row)| row).ok()
}

/// 从保存的行号回退，清除旧内容，用 Markdown 重绘。
pub(crate) fn redraw_markdown_from_saved(saved_row: u16, text: &str) {
    use crossterm::{cursor, execute, terminal};
    let mut stdout = io::stdout();
    let _ = execute!(
        stdout,
        cursor::MoveTo(0, saved_row),
        terminal::Clear(terminal::ClearType::FromCursorDown)
    );
    crate::util::md_render::render_md(text);
}

/// 回退 raw 文本，用 markdown 重绘（基于行数回退，仅用于无工具模式）
pub(crate) fn redraw_markdown(raw_lines: usize, cur_col: usize, text: &str) {
    use crossterm::{cursor, execute, terminal};
    let total_raw_lines = if cur_col > 0 {
        raw_lines + 1
    } else {
        raw_lines
    };
    let mut stdout = io::stdout();
    if total_raw_lines > 0 {
        let _ = execute!(stdout, cursor::MoveToColumn(0));
        if total_raw_lines > 1 {
            let _ = execute!(stdout, cursor::MoveUp((total_raw_lines - 1) as u16));
        }
        let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
    }
    crate::util::md_render::render_md(text);
}

/// 流式文本回退 + markdown 重绘（基于保存的行号）
pub(crate) fn redraw_streaming_as_markdown(streaming_content: &Arc<Mutex<String>>, saved_row: u16) {
    let content = streaming_content.lock().unwrap();
    if content.is_empty() {
        return;
    }
    redraw_markdown_from_saved(saved_row, &content);
}
