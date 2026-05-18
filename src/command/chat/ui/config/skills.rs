use crate::command::chat::app::ChatApp;
use crate::tui::components::{ItemList, TOGGLE_OFF, TOGGLE_ON, pointer_span};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

/// 描述行缩进宽度（与指针对齐后的缩进）
const DESC_INDENT: usize = 7;

/// Skills tab 固定头部（已启用计数）
pub(super) fn draw_tab_skills_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let total = app.state.loaded_skills.len();
    let enabled_count = total
        - app
            .state
            .agent_config
            .disabled_skills
            .iter()
            .filter(|d| {
                app.state
                    .loaded_skills
                    .iter()
                    .any(|s| &s.frontmatter.name == *d)
            })
            .count();

    lines.push(Line::from(vec![Span::styled(
        format!("  已启用: {}/{}", enabled_count, total),
        Style::default()
            .fg(t.config_toggle_on)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));
}

/// Skills tab 可滚动列表（每个技能：名称行 + 描述折行）
pub(super) fn draw_tab_skills_list<'a>(app: &ChatApp, max_width: usize) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    for (i, skill) in app.state.loaded_skills.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let name = &skill.frontmatter.name;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_skills
            .iter()
            .any(|d| d == name);

        // 第一行：指针 + 圆点 + 技能名
        let toggle_style = if is_enabled {
            Style::default()
                .fg(t.config_toggle_on)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_toggle_off)
        };
        let toggle_text = if is_enabled { TOGGLE_ON } else { TOGGLE_OFF };
        let name_style = if is_selected {
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.config_label)
        };

        let mut name_spans = vec![
            pointer_span(is_selected, t),
            Span::styled(toggle_text, toggle_style),
            Span::styled(" ", Style::default()),
            Span::styled(name.clone(), name_style),
        ];
        let tag = skill.source.label();
        if !tag.is_empty() {
            name_spans.push(Span::styled(
                format!(" [{tag}]"),
                Style::default().fg(t.config_dim),
            ));
        }
        list.push(Line::from(name_spans));

        // 描述行：自动折行（用 push_raw 避免 field_line_indices 被污染）
        if !skill.frontmatter.description.is_empty() {
            let desc_style = Style::default().fg(t.config_dim);
            // 右侧留 4 字符 padding，避免文字贴到右边框
            const RIGHT_PAD: usize = 4;
            let col_width = max_width.saturating_sub(DESC_INDENT + RIGHT_PAD);
            if col_width == 0 {
                continue;
            }

            let mut chars = skill.frontmatter.description.chars().peekable();
            let indent = " ".repeat(DESC_INDENT);

            while chars.peek().is_some() {
                let mut line_buf = String::with_capacity(col_width);
                while line_buf.chars().count() < col_width && chars.peek().is_some() {
                    let ch = chars.next().expect("peek ensured Some");
                    line_buf.push(ch);
                    if line_buf.chars().count() >= col_width {
                        if chars.peek() == Some(&' ') {
                            chars.next();
                        }
                        break;
                    }
                }
                if !line_buf.is_empty() {
                    list.push_raw(Line::from(vec![
                        Span::styled(indent.clone(), desc_style),
                        Span::styled(line_buf, desc_style),
                    ]));
                }
            }
        }
    }
    list
}
