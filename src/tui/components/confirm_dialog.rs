//! 确认对话框组件
//!
//! 提供统一的确认弹窗渲染，用于 todo / notebook 等模块的删除/写入/放弃确认场景。

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// 确认对话框配置
pub struct ConfirmDialogConfig<'a> {
    /// Block 标题（如 " ⚠️ 确认删除 "）
    pub title: &'a str,
    /// 消息内容
    pub message: String,
    /// 主题颜色（用于文本和边框）
    pub color: ratatui::style::Color,
}

/// 绘制确认对话框
///
/// 在指定区域渲染一个带边框的确认弹窗，包含标题和消息内容。
/// 文本和边框使用相同颜色。
pub fn draw_confirm_dialog(f: &mut Frame, area: Rect, config: &ConfirmDialogConfig<'_>) {
    let style = Style::default().fg(config.color);
    let confirm_widget = Paragraph::new(Line::from(Span::styled(config.message.clone(), style)))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style)
                .title(config.title),
        );
    f.render_widget(confirm_widget, area);
}
