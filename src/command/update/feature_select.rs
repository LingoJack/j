use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute, terminal,
};
use std::io::{self, Write};

/// 可选 feature 列表
const OPTIONAL_FEATURES: &[(&str, &str)] = &[(
    "browser_cdp",
    "浏览器自动化 (CDP 模式，需本地有 Chrome/Chromium)",
)];

/// 计算菜单总行数
fn menu_total_lines() -> u16 {
    // 标题(1) + 空行(1) + features + 空行(1) + 确认按钮(1) + 空行(1) + 提示(1)
    (1 + 1 + OPTIONAL_FEATURES.len() + 1 + 1 + 1 + 1) as u16
}

/// 交互式 feature 选择界面（类似 Claude Code 风格）
/// 返回用户选中的 features 列表
pub(crate) fn select_features() -> Vec<String> {
    let mut selected = vec![false; OPTIONAL_FEATURES.len()];
    let mut cursor_pos: usize = 0;
    let mut is_first_draw = true;

    // 进入 raw 模式
    if terminal::enable_raw_mode().is_err() {
        return vec![];
    }

    let mut stdout = io::stdout();

    // 绘制初始界面
    let _ = draw_feature_menu(&mut stdout, &selected, cursor_pos, is_first_draw);
    is_first_draw = false;

    loop {
        if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
            match code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j')
                    if cursor_pos < OPTIONAL_FEATURES.len() =>
                {
                    cursor_pos += 1;
                }
                KeyCode::Char(' ')
                    // 空格切换选中状态（仅在 feature 行上有效）
                    if cursor_pos < OPTIONAL_FEATURES.len() =>
                {
                    selected[cursor_pos] = !selected[cursor_pos];
                }
                KeyCode::Enter => {
                    // 如果光标在 "确认安装" 行上，直接确认
                    if cursor_pos == OPTIONAL_FEATURES.len() {
                        break;
                    }
                    // 在 feature 行上按 Enter 也切换选中
                    if cursor_pos < OPTIONAL_FEATURES.len() {
                        selected[cursor_pos] = !selected[cursor_pos];
                    }
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    // 取消：不选择任何 feature，直接跳到确认
                    break;
                }
                _ => {} // 忽略其他按键
            }
            let _ = draw_feature_menu(&mut stdout, &selected, cursor_pos, is_first_draw);
        }
    }

    // 退出 raw 模式
    let _ = terminal::disable_raw_mode();
    // 换行，避免后续输出接在同一行
    println!();

    // 收集选中的 features
    selected
        .iter()
        .enumerate()
        .filter(|(_, s)| **s)
        .map(|(i, _)| OPTIONAL_FEATURES[i].0.to_string())
        .collect()
}

/// 绘制 feature 选择菜单
fn draw_feature_menu(
    stdout: &mut io::Stdout,
    selected: &[bool],
    cursor_pos: usize,
    is_first_draw: bool,
) -> io::Result<()> {
    let total_lines = menu_total_lines();

    if !is_first_draw {
        // 非首次绘制：移回菜单起始位置
        execute!(stdout, cursor::MoveUp(total_lines))?;
    }
    // 从当前光标位置清除到屏幕底部
    execute!(stdout, terminal::Clear(terminal::ClearType::FromCursorDown))?;

    // 标题
    // raw mode 下 \n 不会自动回到行首，需要使用 \r\n
    write!(
        stdout,
        "  {} {}\r\n",
        "?".cyan().bold(),
        "选择要启用的可选 Features:".bold()
    )?;
    write!(stdout, "\r\n")?;

    // Feature 列表
    for (i, (name, desc)) in OPTIONAL_FEATURES.iter().enumerate() {
        let is_focused = cursor_pos == i;
        let is_selected = selected[i];

        let checkbox = if is_selected {
            "◉".green().bold().to_string()
        } else {
            "○".dimmed().to_string()
        };

        let pointer = if is_focused { "❯" } else { " " };

        if is_focused {
            write!(
                stdout,
                "  {} {} {} {}\r\n",
                pointer.cyan().bold(),
                checkbox,
                name.cyan().bold(),
                format!("({})", desc).dimmed()
            )?;
        } else {
            write!(
                stdout,
                "  {} {} {} {}\r\n",
                pointer,
                checkbox,
                name,
                format!("({})", desc).dimmed()
            )?;
        }
    }

    // 空行
    write!(stdout, "\r\n")?;

    // 确认按钮
    let confirm_focused = cursor_pos == OPTIONAL_FEATURES.len();
    if confirm_focused {
        write!(
            stdout,
            "  {} {}\r\n",
            "❯".cyan().bold(),
            "确认安装".green().bold()
        )?;
    } else {
        write!(stdout, "    {}\r\n", "确认安装".dimmed())?;
    }

    // 操作提示
    write!(stdout, "\r\n")?;
    write!(
        stdout,
        "  {} ↑↓ 移动  {} 切换  {} 确认  {} 跳过\r\n",
        "•".dimmed(),
        "空格".dimmed(),
        "Enter".dimmed(),
        "Esc".dimmed()
    )?;

    stdout.flush()?;
    Ok(())
}
