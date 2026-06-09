use crate::command::chat::app::{ChatApp, CommandsMode};
use crate::command::chat::infra::command;
use crate::tui::components::{ItemList, TOGGLE_OFF, TOGGLE_ON, pointer_span};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthChar;

/// 描述行缩进宽度
const DESC_INDENT: usize = 7;
/// 右侧留白，避免描述贴到边框
const RIGHT_PAD: usize = 4;

/// Commands tab 固定头部（已启用计数 + 操作提示）
pub(super) fn draw_tab_commands_header<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;

    // 选择来源模式：显示选择界面
    if app.ui.commands_mode == CommandsMode::SelectSource {
        draw_select_source_ui(lines, app);
        return;
    }

    let total = app.state.loaded_commands.len();
    let enabled_count = total
        - app
            .state
            .agent_config
            .disabled_commands
            .iter()
            .filter(|d| {
                app.state
                    .loaded_commands
                    .iter()
                    .any(|c| &c.frontmatter.name == *d)
            })
            .count();

    lines.push(Line::from(vec![Span::styled(
        format!("  已启用: {}/{}", enabled_count, total),
        Style::default()
            .fg(t.config_toggle_on)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    if total == 0 {
        lines.push(Line::from(Span::styled(
            "  (没有自定义命令，按 c 快速创建)",
            Style::default().fg(t.config_dim),
        )));
    }
}

/// 渲染选择保存级别的界面
fn draw_select_source_ui<'a>(lines: &mut Vec<Line<'a>>, app: &ChatApp) {
    let t = &app.ui.theme;
    let has_project_dir = command::project_commands_dir().is_some();

    lines.push(Line::from(Span::styled(
        "  选择命令保存位置：",
        Style::default()
            .fg(t.config_label_selected)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    // 用户级选项
    let user_selected = app.ui.commands_source_idx == 0;
    let user_marker = if user_selected {
        Span::styled(
            "  > ",
            Style::default()
                .fg(t.config_label_selected)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::styled("    ", Style::default())
    };
    let user_label = Span::styled(
        "用户级 (~/.jdata/agent/commands/)",
        Style::default()
            .fg(if user_selected {
                t.config_label_selected
            } else {
                t.text_dim
            })
            .add_modifier(if user_selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            }),
    );
    lines.push(Line::from(vec![user_marker, user_label]));

    // 项目级选项
    if has_project_dir {
        let proj_selected = app.ui.commands_source_idx == 1;
        let proj_marker = if proj_selected {
            Span::styled(
                "  > ",
                Style::default()
                    .fg(t.config_label_selected)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("    ", Style::default())
        };
        let proj_label = Span::styled(
            "项目级 (.jcli/commands/)",
            Style::default()
                .fg(if proj_selected {
                    t.config_label_selected
                } else {
                    t.text_dim
                })
                .add_modifier(if proj_selected {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
        );
        lines.push(Line::from(vec![proj_marker, proj_label]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  j/k 或 ↑/↓ 选择，Enter 确认，Esc 取消",
        Style::default().fg(t.config_dim),
    )));
}

/// Commands tab 可滚动列表（每个命令：名称行 + 描述折行）
pub(super) fn draw_tab_commands_list<'a>(app: &ChatApp, max_width: usize) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    for (i, cmd) in app.state.loaded_commands.iter().enumerate() {
        let is_selected = i == app.ui.config_field_idx;
        let name = &cmd.frontmatter.name;
        let is_enabled = !app
            .state
            .agent_config
            .disabled_commands
            .iter()
            .any(|d| d == name);

        // 第一行：指针 + 圆点 + 命令名
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
        let tag = cmd.source.label();
        if !tag.is_empty() {
            let detail_style = if is_selected {
                Style::default().fg(t.config_label_selected)
            } else {
                Style::default().fg(t.config_dim)
            };
            name_spans.push(Span::styled(format!(" [{tag}]"), detail_style));
        }
        list.push(Line::from(name_spans));

        // 描述行：自动折行（用 push_raw 避免 field_line_indices 被污染）
        if !cmd.frontmatter.description.is_empty() {
            let desc_style = if is_selected {
                Style::default().fg(t.config_label_selected)
            } else {
                Style::default().fg(t.config_dim)
            };
            let col_width = max_width.saturating_sub(DESC_INDENT + RIGHT_PAD);
            if col_width == 0 {
                continue;
            }

            let indent = " ".repeat(DESC_INDENT);
            let mut remaining = cmd.frontmatter.description.chars().peekable();

            while remaining.peek().is_some() {
                let mut line_buf = String::new();
                let mut line_width: usize = 0;

                while let Some(&ch) = remaining.peek() {
                    let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
                    if line_width + cw > col_width {
                        break;
                    }
                    remaining.next();
                    line_buf.push(ch);
                    line_width += cw;

                    if line_width >= col_width {
                        if remaining.peek() == Some(&' ') {
                            remaining.next();
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
