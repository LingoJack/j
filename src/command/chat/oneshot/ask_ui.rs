//! oneshot Ask 请求处理线程：单选/多选 UI，复用 confirm 模块的边框绘制

use crate::command::chat::app::types::{AskQuestion, AskRequest};
use crate::command::chat::oneshot::confirm::*;
use crate::command::chat::oneshot::display::box_width;
use colored::Colorize;
use crossterm::event::{self, Event, KeyCode};
use crossterm::terminal;
use std::io;
use std::sync::mpsc;

/// 启动 Ask 请求处理线程。
///
/// 从 `ask_rx` 接收 `AskRequest`，为每个问题渲染 `┌──┐` 直角边框选择 UI，
/// 收集答案后通过 `AskRequest::response_tx` 回传 JSON。
pub(crate) fn spawn_ask_handler(ask_rx: mpsc::Receiver<AskRequest>) {
    std::thread::spawn(move || {
        while let Ok(req) = ask_rx.recv() {
            handle_ask_request(&req);
        }
    });
}

/// 处理单个 AskRequest（所有问题）
fn handle_ask_request(req: &AskRequest) {
    let mut answers = serde_json::Map::new();
    for q in &req.questions {
        let bw = box_width();

        let title = if q.header.is_empty() {
            "选择".to_string()
        } else {
            q.header.clone()
        };

        // 问题文本按宽度折行
        let question_lines = wrap_text(&q.question, bw.saturating_sub(6).max(1));

        if q.multi_select {
            let answer = handle_multi_select(q, &title, &question_lines, bw);
            println!("  → {}", answer.green());
            answers.insert(q.header.clone(), serde_json::Value::String(answer));
        } else {
            let answer = handle_single_select(q, &title, &question_lines, bw);
            println!("  → {}", answer.green());
            answers.insert(q.header.clone(), serde_json::Value::String(answer));
        }
    }

    let response =
        serde_json::to_string(&serde_json::json!({ "answers": answers })).unwrap_or_default();
    let _ = req.response_tx.send(response);
}

/// 多选 UI
fn handle_multi_select(
    q: &AskQuestion,
    title: &str,
    question_lines: &[String],
    bw: usize,
) -> String {
    let mut selected = vec![false; q.options.len()];
    let mut cursor_pos: usize = 0;

    // 行数：顶 1 + 问题行数 + 空行 1 + 选项*2 + 空行 1 + 提示 1 + 底 1
    let total_lines = (1 + question_lines.len() + 1 + q.options.len() * 2 + 1 + 1 + 1) as u16;

    let draw_multi = |stdout: &mut io::Stdout,
                      cursor_pos: usize,
                      selected: &[bool],
                      first: bool|
     -> io::Result<()> {
        if !first {
            clear_drawn_lines(stdout, total_lines)?;
        }

        draw_top_border(stdout, bw, title)?;
        for line in question_lines {
            draw_content_line(stdout, bw, line)?;
        }
        draw_empty_line(stdout, bw)?;

        for (i, opt) in q.options.iter().enumerate() {
            if cursor_pos == i {
                draw_multi_selected_option(stdout, bw, selected[i], &opt.label)?;
            } else {
                draw_multi_unselected_option(stdout, bw, selected[i], &opt.label)?;
            }
            draw_option_description(stdout, bw, &opt.description)?;
        }

        draw_empty_line(stdout, bw)?;
        draw_hint_line(stdout, bw, "• ↑↓ 移动  Space 切换  Enter 确认")?;
        draw_bottom_border(stdout, bw)?;
        Ok(())
    };

    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = draw_multi(&mut stdout, cursor_pos, &selected, true);

    loop {
        if let Ok(Event::Key(key)) = event::read() {
            if key.code == KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                let _ = terminal::disable_raw_mode();
                let _ = clear_drawn_lines(&mut stdout, total_lines);
                return String::new();
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if cursor_pos + 1 < q.options.len() {
                        cursor_pos += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    selected[cursor_pos] = !selected[cursor_pos];
                }
                KeyCode::Enter | KeyCode::Esc => break,
                _ => continue,
            }
            let _ = draw_multi(&mut stdout, cursor_pos, &selected, false);
        }
    }

    let _ = terminal::disable_raw_mode();
    {
        let _ = clear_drawn_lines(&mut stdout, total_lines);
    }

    let result: Vec<String> = q
        .options
        .iter()
        .zip(selected.iter())
        .filter(|(_, s)| **s)
        .map(|(o, _)| o.label.clone())
        .collect();
    if result.is_empty() {
        "(无选择)".to_string()
    } else {
        result.join(", ")
    }
}

/// 单选 UI
fn handle_single_select(
    q: &AskQuestion,
    title: &str,
    question_lines: &[String],
    bw: usize,
) -> String {
    let mut cursor_pos: usize = 0;

    let total_lines = (1 + question_lines.len() + 1 + q.options.len() * 2 + 1 + 1 + 1) as u16;

    let draw_single = |stdout: &mut io::Stdout, cursor_pos: usize, first: bool| -> io::Result<()> {
        if !first {
            clear_drawn_lines(stdout, total_lines)?;
        }

        draw_top_border(stdout, bw, title)?;
        for line in question_lines {
            draw_content_line(stdout, bw, line)?;
        }
        draw_empty_line(stdout, bw)?;

        for (i, opt) in q.options.iter().enumerate() {
            if cursor_pos == i {
                draw_selected_option(stdout, bw, &opt.label)?;
            } else {
                draw_unselected_option(stdout, bw, &opt.label)?;
            }
            draw_option_description(stdout, bw, &opt.description)?;
        }

        draw_empty_line(stdout, bw)?;
        draw_hint_line(stdout, bw, "• ↑↓ 移动  Enter 确认")?;
        draw_bottom_border(stdout, bw)?;
        Ok(())
    };

    let _ = terminal::enable_raw_mode();
    let mut stdout = io::stdout();
    let _ = draw_single(&mut stdout, cursor_pos, true);

    loop {
        if let Ok(Event::Key(key)) = event::read() {
            if key.code == KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                let _ = terminal::disable_raw_mode();
                let _ = clear_drawn_lines(&mut stdout, total_lines);
                return String::new();
            }
            match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    cursor_pos = cursor_pos.saturating_sub(1);
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if cursor_pos + 1 < q.options.len() {
                        cursor_pos += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Esc => break,
                _ => continue,
            }
            let _ = draw_single(&mut stdout, cursor_pos, false);
        }
    }

    let _ = terminal::disable_raw_mode();
    {
        let _ = clear_drawn_lines(&mut stdout, total_lines);
    }

    q.options
        .get(cursor_pos)
        .map(|o| o.label.clone())
        .unwrap_or_default()
}

/// 文本按最大字符数折行
fn wrap_text(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() || max_chars == 0 {
        return vec![];
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lines = Vec::with_capacity((chars.len() / max_chars).max(1) + 1);
    let mut start = 0;
    while start < chars.len() {
        let end = (start + max_chars).min(chars.len());
        let line: String = chars[start..end].iter().collect();
        lines.push(line);
        start = end;
    }
    lines
}
