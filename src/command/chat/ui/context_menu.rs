//! 右键上下文菜单渲染
//!
//! 在消息区域右键点击时显示"复制"菜单。

use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::command::chat::app::ChatApp;

/// 右键菜单宽度（"复制" + 左右各 1 空格 + 边框）
const MENU_WIDTH: u16 = 8;

/// 右键菜单高度（1 行内容 + 边框）
const MENU_HEIGHT: u16 = 3;

/// 绘制右键上下文菜单
///
/// 如果 `app.ui.context_menu` 为 `Some`，在点击位置附近渲染一个小型弹窗，
/// 显示"复制"选项。
pub fn draw_context_menu(f: &mut Frame, app: &ChatApp) {
    let menu = match &app.ui.context_menu {
        Some(m) => m,
        None => return,
    };

    let t = &app.ui.theme;

    // 计算弹窗位置：在点击位置右下方偏移一点，避免遮挡点击点
    let (col, row) = menu.screen_pos;
    let x = col.min(f.area().width.saturating_sub(MENU_WIDTH));
    let y = row.min(f.area().height.saturating_sub(MENU_HEIGHT));

    let menu_area = Rect::new(x, y, MENU_WIDTH, MENU_HEIGHT);

    // 先清除背景区域
    f.render_widget(Clear, menu_area);

    // 创建菜单区块
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.model_sel_border))
        .style(Style::default().bg(t.bg_primary));

    // 菜单内容："复制"
    let content = Paragraph::new(" 复制 ").block(block);

    f.render_widget(content, menu_area);
}

/// 检查给定的屏幕坐标是否在右键菜单区域内
///
/// 用于判断左键点击是否应该触发菜单操作
pub fn is_point_in_menu(app: &ChatApp, col: u16, row: u16) -> bool {
    let menu = match &app.ui.context_menu {
        Some(m) => m,
        None => return false,
    };

    // 使用屏幕坐标直接计算菜单位置
    // 这里不依赖 frame area，因为菜单是在点击位置附近固定偏移
    let x = menu.screen_pos.0;
    let y = menu.screen_pos.1;

    let menu_area = Rect::new(x, y, MENU_WIDTH, MENU_HEIGHT);

    col >= menu_area.x
        && col < menu_area.x + menu_area.width
        && row >= menu_area.y
        && row < menu_area.y + menu_area.height
}
