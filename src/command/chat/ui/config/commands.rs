use crate::command::chat::app::{ChatApp, CommandsMode};
use crate::command::chat::infra::command;
use crate::tui::components::{ItemList, ToggleListItemCtx, toggle_list_item};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

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

/// Commands tab 可滚动列表
pub(super) fn draw_tab_commands_list<'a>(app: &ChatApp) -> ItemList<'a> {
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
        list.push(toggle_list_item(&ToggleListItemCtx {
            name: name.to_string(),
            enabled: is_enabled,
            selected: is_selected,
            desc: Some(cmd.frontmatter.description.clone()),
            tag: Some(cmd.source.label().to_string()),
            theme: t,
        }));
    }
    list
}
