//! 帮助页面组件
//!
//! 提供统一的帮助页面渲染框架，用于 todo / notebook 等模块。

use crate::theme::Theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// 帮助页面快捷键项
#[derive(Clone, Debug)]
pub struct HelpShortcut<'a> {
    /// 快捷键描述（左侧，如 "  n / ↓ / j    "）
    pub key: &'a str,
    /// 功能说明（右侧，如 "向下移动"）
    pub desc: &'a str,
}

impl<'a> HelpShortcut<'a> {
    /// 创建快捷键项
    pub const fn new(key: &'a str, desc: &'a str) -> Self {
        Self { key, desc }
    }
}

/// 帮助页面配置
pub struct HelpPageConfig<'a> {
    /// 标题（如 "快捷键帮助"）
    pub title: &'a str,
    /// 标题图标（如 "📖"，可选）
    pub title_icon: Option<&'a str>,
    /// Block 标题（如 " 帮助 "，可选）
    pub block_title: Option<&'a str>,
    /// 快捷键列表
    pub shortcuts: &'a [HelpShortcut<'a>],
    /// 额外尾部行（可选，如分割线 + 说明文字）
    pub footer_lines: Option<Vec<Line<'a>>>,
    /// 主题样式
    pub theme: &'a Theme,
}

/// 绘制帮助页面
///
/// 在指定区域渲染一个帮助页面，包含标题、快捷键列表和可选的尾部内容。
/// 快捷键使用 theme 的 `help_title` 和 `help_key` 颜色。
pub fn draw_help_page(f: &mut Frame, area: Rect, config: &HelpPageConfig<'_>) {
    let t = config.theme;
    let mut lines: Vec<Line<'_>> = Vec::new();

    // 标题行
    let title_text = if let Some(icon) = config.title_icon {
        format!("  {} {}", icon, config.title)
    } else {
        format!("  {}", config.title)
    };
    lines.push(Line::from(Span::styled(
        title_text,
        Style::default()
            .fg(t.help_title)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // 快捷键行
    for shortcut in config.shortcuts {
        lines.push(Line::from(vec![
            Span::styled(shortcut.key.to_string(), Style::default().fg(t.help_key)),
            Span::raw(shortcut.desc),
        ]));
    }

    // 尾部额外行
    if let Some(footer) = &config.footer_lines {
        lines.extend(footer.iter().cloned());
    }

    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.help_title));
    if let Some(bt) = config.block_title {
        block = block.title(bt);
    }
    let help_widget = Paragraph::new(lines).block(block);
    f.render_widget(help_widget, area);
}
