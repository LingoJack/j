//! 帮助页面
//!
//! 提取自 chat.rs，显示快捷键帮助信息。

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::command::chat::app::ChatApp;
use crate::command::chat::storage::agent_config_path;

use crate::tui::components;

/// 绘制帮助界面
pub fn draw_help(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    let sep = components::separator_line(area.width, t);

    let shortcuts: &[(&str, &str)] = &[
        ("Enter", "发送消息"),
        ("Shift/Alt+Enter", "输入框内换行"),
        ("↑ / ↓", "滚动对话记录"),
        ("← / →", "移动输入光标"),
        ("Home / End", "光标跳到行首/行尾"),
        (
            "/",
            "斜杠命令（copy/log/browse/config/model/archive/theme/resume）",
        ),
        ("@", "引用（skill/file/command）"),
        ("Ctrl+O", "展开/折叠工具详情"),
        ("Esc / Ctrl+C", "退出对话"),
        ("? / F1", "显示 / 关闭此帮助"),
    ];

    let mut help_lines = vec![
        Line::from(""),
        components::section_header("📖", "快捷键帮助", t),
        Line::from(""),
        sep.clone(),
        Line::from(""),
    ];
    for (key, desc) in shortcuts {
        help_lines.push(components::help_key_row(key, desc, 15, t));
    }
    help_lines.push(Line::from(""));
    help_lines.push(sep);
    help_lines.push(Line::from(""));
    help_lines.push(components::section_header("📁", "配置文件:", t));
    help_lines.push(Line::from(Span::styled(
        format!("     {}", agent_config_path().display()),
        Style::default().fg(t.help_path),
    )));

    help_lines.insert(
        0,
        Line::from(Span::styled(
            " 帮助 (按任意键返回)",
            Style::default().fg(t.text_dim),
        )),
    );

    let help_widget = Paragraph::new(help_lines).style(Style::default().bg(t.help_bg));
    f.render_widget(help_widget, area);
}
