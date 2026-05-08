//! oneshot 交互确认 UI：直角边框绘制 + 工具权限确认

use crate::command::chat::oneshot::display::{box_width, extract_bash_command, make_args_preview};
use crate::command::chat::tools::classification::ToolCategory;
use colored::Colorize;
use crossterm::event::{self, Event, KeyCode};
use crossterm::{cursor, execute, terminal};
use std::io::{self, Write};

// ─────────────────────────────────────────────────────────────
//  公共边框绘制工具（┌──┐ 直角边框样式）
// ─────────────────────────────────────────────────────────────

/// 绘制顶边框: `┌─ {title} ─────┐`
pub(crate) fn draw_top_border(stdout: &mut io::Stdout, bw: usize, title: &str) -> io::Result<()> {
    let title_text = format!(" {} ", title);
    let inner_w = bw.saturating_sub(2);
    let dash_fill = inner_w.saturating_sub(title_text.chars().count() + 2);
    let dash_tail = "─".repeat(dash_fill);
    writeln!(
        stdout,
        "  {}{}{}{}{}\r",
        "┌".yellow().bold(),
        "─".yellow(),
        title_text.white().bold(),
        "─".yellow(),
        format!("{}{}", dash_tail, "┐").yellow().bold(),
    )
}

/// 绘制内容行: `│ {content}`
pub(crate) fn draw_content_line(stdout: &mut io::Stdout, content: &str) -> io::Result<()> {
    writeln!(stdout, "  {}  {}\r", "│".yellow(), content.white())
}

/// 绘制空行: `│`
pub(crate) fn draw_empty_line(stdout: &mut io::Stdout) -> io::Result<()> {
    writeln!(stdout, "  {}\r", "│".yellow())
}

/// 绘制提示行: `│ {hint}`
pub(crate) fn draw_hint_line(stdout: &mut io::Stdout, hint: &str) -> io::Result<()> {
    writeln!(stdout, "  {}  {}\r", "│".yellow(), hint.dimmed())
}

/// 绘制底边框: `└────────────┘`
pub(crate) fn draw_bottom_border(stdout: &mut io::Stdout, bw: usize) -> io::Result<()> {
    writeln!(
        stdout,
        "  {}{}{}\r",
        "└".yellow().bold(),
        "─".repeat(bw.saturating_sub(3)).yellow(),
        "┘".yellow().bold(),
    )?;
    stdout.flush()
}

/// 绘制选项行（选中态）: `│  ❯ {label}`
pub(crate) fn draw_selected_option(stdout: &mut io::Stdout, label: &str) -> io::Result<()> {
    writeln!(
        stdout,
        "  {}  {} {}\r",
        "│".yellow(),
        "❯".cyan().bold(),
        label.white().bold()
    )
}

/// 绘制选项行（未选中态）: `│    {label}`
pub(crate) fn draw_unselected_option(stdout: &mut io::Stdout, label: &str) -> io::Result<()> {
    writeln!(stdout, "  {}    {}\r", "│".yellow(), label.dimmed())
}

/// 绘制选项描述行: `│    {description}`
pub(crate) fn draw_option_description(stdout: &mut io::Stdout, desc: &str) -> io::Result<()> {
    writeln!(stdout, "  {}    {}\r", "│".yellow(), desc.dimmed())
}

/// 绘制多选选项行（选中态）
pub(crate) fn draw_multi_selected_option(
    stdout: &mut io::Stdout,
    checked: bool,
    label: &str,
) -> io::Result<()> {
    let check = if checked { "◉" } else { "○" };
    writeln!(
        stdout,
        "  {}  {} {} {}\r",
        "│".yellow(),
        "❯".cyan().bold(),
        check.cyan().bold(),
        label.cyan().bold()
    )
}

/// 绘制多选选项行（未选中态）
pub(crate) fn draw_multi_unselected_option(
    stdout: &mut io::Stdout,
    checked: bool,
    label: &str,
) -> io::Result<()> {
    let check = if checked { "◉" } else { "○" };
    writeln!(
        stdout,
        "  {}   {} {}\r",
        "│".yellow(),
        check.white(),
        label.white()
    )
}

/// 清除之前绘制的行，将光标上移 `lines` 行并清空
pub(crate) fn clear_drawn_lines(stdout: &mut io::Stdout, lines: u16) -> io::Result<()> {
    if lines > 0 {
        let _ = execute!(stdout, cursor::MoveUp(lines));
    }
    let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));
    Ok(())
}

// ─────────────────────────────────────────────────────────────
//  交互式工具确认
// ─────────────────────────────────────────────────────────────

/// 交互式工具确认（crossterm raw mode + `┌──┐` 直角边框）
pub(crate) fn interactive_confirm(
    tool_name: &str,
    arguments: &str,
    options: &[&str],
    initial: usize,
) -> Option<usize> {
    let bw = box_width();
    let category = ToolCategory::from_name(tool_name);
    let icon = category.icon();

    let mut stdout = io::stdout();
    let mut cursor_pos = initial;

    // 参数描述
    let desc = if tool_name == "Bash" || tool_name == "Shell" {
        if let Some(cmd) = extract_bash_command(arguments) {
            format!("$ {}", cmd)
        } else {
            make_args_preview(arguments)
        }
    } else {
        make_args_preview(arguments)
    };

    // 截断描述（按字符边界截断，避免 UTF-8 多字节字符中间切割导致 panic）
    let max_char_width = bw.saturating_sub(8);
    let desc_display = if desc.chars().count() > max_char_width {
        let trunc_len = bw.saturating_sub(11);
        let truncated: String = desc.chars().take(trunc_len).collect();
        format!("{}...", truncated)
    } else {
        desc.clone()
    };

    // 计算实际绘制行数：顶边框 1 + 描述 1 + 空行 1 + 每选项 1 + 空行 1 + 提示 1 + 底边框 1
    let total_lines = (options.len() + 6) as u16;

    let draw = |stdout: &mut io::Stdout, cursor_pos: usize, first: bool| -> io::Result<()> {
        if !first {
            clear_drawn_lines(stdout, total_lines)?;
        }
        let _ = execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown));

        let title = format!("{} {} 需要确认", icon, tool_name);
        draw_top_border(stdout, bw, &title)?;

        // 描述行
        draw_content_line(stdout, &desc_display)?;

        draw_empty_line(stdout)?;

        // 选项列表
        for (i, opt) in options.iter().enumerate() {
            if cursor_pos == i {
                draw_selected_option(stdout, opt)?;
            } else {
                draw_unselected_option(stdout, opt)?;
            }
        }

        draw_empty_line(stdout)?;

        draw_hint_line(stdout, "• ↑↓ 移动  Enter 确认")?;

        draw_bottom_border(stdout, bw)?;

        stdout.flush()?;
        Ok(())
    };

    if terminal::enable_raw_mode().is_err() {
        return None;
    }

    let _ = draw(&mut stdout, cursor_pos, true);

    let result = loop {
        if let Ok(Event::Key(key)) = event::read() {
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if cursor_pos + 1 < options.len() {
                        cursor_pos += 1;
                    }
                }
                KeyCode::Enter => break Some(cursor_pos),
                KeyCode::Esc | KeyCode::Char('q') => break None,
                _ => continue,
            }
            let _ = draw(&mut stdout, cursor_pos, false);
        }
    };

    let _ = terminal::disable_raw_mode();
    {
        let _ = clear_drawn_lines(&mut stdout, total_lines);
    }
    result
}
