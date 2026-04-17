//! 提示栏和 Toast 弹窗
//!
//! 提取自 chat.rs，处理底部快捷键提示栏和右上角 Toast 通知。

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::command::chat::{
    app::{ChatApp, ConfigTab},
    theme::Theme,
};
use crate::util::text::display_width;

use super::components;

/// 绘制底部提示栏
pub fn draw_hint_bar(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    use crate::command::chat::app::ChatMode;

    let hints = match app.ui.mode {
        ChatMode::Chat if app.state.is_loading => vec![], // 加载中时无提示
        ChatMode::Chat => vec![
            ("@", "引用"),
            ("/", "命令"),
            ("Ctrl+M", "选中模式"),
            ("Ctrl+O", "工具详情"),
            ("?/F1", "帮助"),
        ],
        ChatMode::SelectModel => vec![("↑↓/jk", "移动"), ("Enter", "确认"), ("Esc", "取消")],
        ChatMode::SelectTheme => vec![("↑↓/jk", "选择"), ("Enter", "确认"), ("Esc", "返回")],
        ChatMode::Browse => {
            let mut hints = vec![
                ("↑↓/jk", "跳转"),
                ("Tab", "角色"),
                ("y/Enter", "复制"),
                ("PgUp/Dn", "微调"),
            ];
            if !app.ui.browse_filter.is_empty() || app.ui.browse_role_filter.is_some() {
                hints.push(("Esc", "清除过滤"));
            } else {
                hints.push(("Esc", "返回"));
            }
            hints
        }
        ChatMode::Help => vec![("任意键", "返回")],
        ChatMode::Config => config_hints(app),
        ChatMode::ArchiveConfirm => {
            if app.ui.archive_editing_name {
                vec![("Enter", "确认"), ("Esc", "取消")]
            } else {
                vec![
                    ("Enter", "默认名称归档"),
                    ("n", "自定义名称"),
                    ("Esc", "取消"),
                ]
            }
        }
        ChatMode::ArchiveList => {
            if app.ui.restore_confirm_needed {
                vec![("y/Enter", "确认还原"), ("Esc", "取消")]
            } else {
                vec![
                    ("↑↓/jk", "选择"),
                    ("Enter", "还原"),
                    ("d", "删除"),
                    ("Esc", "返回"),
                ]
            }
        }
        ChatMode::ToolConfirm => vec![("↑↓", "选择"), ("Enter", "确认"), ("Esc", "拒绝")],
        ChatMode::AgentPermConfirm => vec![("Y/Enter", "允许"), ("N/Esc", "拒绝")],
        ChatMode::PlanApprovalConfirm => {
            vec![("Y/Enter", "批准"), ("C", "批准并清空"), ("N/Esc", "拒绝")]
        }
    };

    render_hint_bar(f, area, t, &hints);
}

/// 配置界面的提示
fn config_hints(app: &ChatApp) -> Vec<(&'static str, &'static str)> {
    use ConfigTab::*;
    match app.ui.config_tab {
        Model => vec![
            ("←→", "切换标签"),
            ("↑↓", "切换字段"),
            ("Enter", "编辑"),
            ("Tab", "切换Provider"),
            ("a", "新增"),
            ("d", "删除"),
            ("s", "设为活跃"),
            ("Esc", "保存返回"),
        ],
        Global => vec![
            ("←→", "切换标签"),
            ("↑↓", "切换字段"),
            ("Enter", "编辑/切换"),
            ("Esc", "保存返回"),
        ],
        Tools => vec![
            ("←→", "切换标签"),
            ("↑↓", "选择"),
            ("Enter/空格", "切换"),
            ("t", "总开关"),
            ("a", "全部启用"),
            ("d", "全部禁用"),
            ("Esc", "保存返回"),
        ],
        Skills => vec![
            ("←→", "切换标签"),
            ("↑↓", "选择"),
            ("Enter/空格", "切换"),
            ("a", "全部启用"),
            ("d", "全部禁用"),
            ("Esc", "保存返回"),
        ],
        Hooks | Commands => vec![("←→", "切换标签"), ("Esc", "保存返回")],
        Teammates => vec![
            ("←→", "切换标签"),
            ("↑↓", "选择"),
            ("s", "停止"),
            ("Enter", "详情"),
            ("Esc", "保存返回"),
        ],
        Archive => {
            if app.ui.restore_confirm_needed {
                vec![("y/Enter", "确认还原"), ("Esc", "取消")]
            } else {
                vec![
                    ("←→", "切换标签"),
                    ("↑↓", "选择"),
                    ("Enter", "还原"),
                    ("d", "删除"),
                    ("Esc", "保存返回"),
                ]
            }
        }
        Session => {
            if app.ui.session_restore_confirm {
                vec![("y/Enter", "确认恢复"), ("Esc", "取消")]
            } else {
                vec![
                    ("←→", "切换标签"),
                    ("↑↓", "选择"),
                    ("Enter", "恢复"),
                    ("d", "删除"),
                    ("n", "新建"),
                    ("Esc", "保存返回"),
                ]
            }
        }
    }
}

/// 渲染提示栏
fn render_hint_bar(f: &mut ratatui::Frame, area: Rect, t: &Theme, hints: &[(&str, &str)]) {
    let avail_width = area.width as usize;
    let sep_w = display_width("  │  ");
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::styled(" ", Style::default()));
    let mut used = 1usize;

    for (i, (key, desc)) in hints.iter().enumerate() {
        let item_w = display_width(&format!(" {} ", key)) + display_width(&format!(" {}", desc));
        let need_w = if i == 0 { item_w } else { sep_w + item_w };
        if used + need_w > avail_width {
            break;
        }
        if i > 0 {
            spans.push(Span::styled("  │  ", Style::default().fg(t.hint_separator)));
        }
        spans.extend(components::hint_spans(key, desc, t));
        used += need_w;
    }

    let hint_bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg_primary));
    f.render_widget(hint_bar, area);
}

/// 绘制 Toast 弹窗（右上角浮层）
pub fn draw_toast(f: &mut ratatui::Frame, area: Rect, app: &ChatApp) {
    let t = &app.ui.theme;
    if let Some((ref msg, is_error, _)) = app.ui.toast {
        let text_width = display_width(msg);
        let toast_width = (text_width + 10).min(area.width as usize).max(16) as u16;
        let toast_height: u16 = 3;

        let x = area.width.saturating_sub(toast_width + 1);
        let y: u16 = 1;

        if x + toast_width <= area.width && y + toast_height <= area.height {
            let toast_area = Rect::new(x, y, toast_width, toast_height);

            let clear = Block::default().style(Style::default().bg(t.bg_primary));
            f.render_widget(clear, toast_area);

            let (icon, border_color, text_color) = if is_error {
                ("✖️", t.toast_error_border, t.toast_error_text)
            } else {
                ("☑️", t.toast_success_border, t.toast_success_text)
            };

            let toast_widget = Paragraph::new(Line::from(vec![
                Span::styled(format!(" {} ", icon), Style::default()),
                Span::styled(msg.as_str(), Style::default().fg(text_color)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .style(Style::default().bg(t.bg_primary)),
            );
            f.render_widget(toast_widget, toast_area);
        }
    }
}
