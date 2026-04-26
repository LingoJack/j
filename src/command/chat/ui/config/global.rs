use crate::command::chat::app::ChatApp;
use crate::command::chat::render::helpers::{
    config_field_desc_global, config_field_label_global, config_field_value_global,
};
use crate::command::chat::ui::components::{
    GlobalRowCtx, global_preview_row, global_text_row, global_theme_row, global_toggle_row,
};
use crate::constants::CONFIG_GLOBAL_FIELDS_TAB;
use crate::tui::components::{ItemList, ToggleListItemCtx, toggle_list_item};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

/// Global tab 内容（三列布局: label | value | desc，--- 分隔分组）
pub(super) fn draw_tab_global_lines<'a>(app: &ChatApp) -> ItemList<'a> {
    let t = &app.ui.theme;
    let mut list = ItemList::new(t.bg_primary);

    // compact_exempt_tools 子列表模式
    if app.ui.compact_exempt_sublist {
        // 标题行 + 空行
        list.push_raw(Line::from(vec![
            Span::styled(
                "  豁免压缩工具  ",
                Style::default().fg(t.config_label_selected),
            ),
            Span::raw(" "),
            Span::styled(
                "Enter/空格 切换 | Esc 返回",
                Style::default().fg(t.config_dim),
            ),
        ]));
        list.push_raw(Line::from(""));

        use crate::command::chat::context::compact::BUILTIN_EXEMPT_TOOLS;
        let tool_names = app.tool_registry.tool_names();
        let exempt = &app.state.agent_config.compact.micro_compact_exempt_tools;

        for (i, name) in tool_names.iter().enumerate() {
            let is_builtin = BUILTIN_EXEMPT_TOOLS.contains(name);
            let is_exempt = is_builtin || exempt.iter().any(|t| t == name);
            let selected = i == app.ui.compact_exempt_idx;

            let label = if is_builtin {
                format!("{} (内置)", name)
            } else {
                name.to_string()
            };

            list.push(toggle_list_item(&ToggleListItemCtx {
                name: label,
                enabled: is_exempt,
                selected,
                desc: None,
                tag: None,
                theme: t,
            }));
        }
        return list;
    }

    // 顶部留白
    list.push_raw(Line::from(""));

    // 分组定义: (字段起始索引, 包含字段数)
    let groups: &[(usize, usize)] = &[
        (0, 3),  // system_prompt, agent_md, style
        (3, 2),  // max_history_messages, max_context_tokens
        (5, 2),  // max_tool_rounds, tool_confirm_timeout
        (7, 4),  // theme, auto_restore_session, thinking_style, flat_bubble
        (11, 4), // compact_enabled, compact_token_threshold, compact_keep_recent, compact_exempt_tools
    ];

    for (gi, &(start, count)) in groups.iter().enumerate() {
        // --- 分隔线 + 空行（首组不画）
        if gi > 0 {
            list.push_raw(Line::from(""));
            list.push_raw(Line::from(Span::styled(
                "  ────────────────────────────────────",
                Style::default().fg(t.separator),
            )));
            list.push_raw(Line::from(""));
        }

        for i in start..start + count {
            let field = CONFIG_GLOBAL_FIELDS_TAB.get(i);
            if field.is_none() {
                continue;
            }
            // is_none() 已在上方判断并 continue，此处 field 必为 Some
            let field_name = field.expect("checked is_none() above with continue");

            let is_selected = app.ui.config_field_idx == i;
            let label = config_field_label_global(i);
            let value = if app.ui.config_editing && is_selected {
                app.ui.config_edit_buf.clone()
            } else {
                config_field_value_global(app, i)
            };
            let desc = config_field_desc_global(i);

            let line = if *field_name == "auto_restore_session" {
                let toggle_on = app.state.agent_config.auto_restore_session;
                let ctx = GlobalRowCtx::new(label, desc, is_selected, t);
                global_toggle_row(toggle_on, &ctx)
            } else if *field_name == "flat_bubble" {
                let toggle_on = app.state.agent_config.flat_bubble;
                let ctx = GlobalRowCtx::new(label, desc, is_selected, t);
                global_toggle_row(toggle_on, &ctx)
            } else if *field_name == "compact_enabled" {
                let toggle_on = app.state.agent_config.compact.enabled;
                let ctx = GlobalRowCtx::new(label, desc, is_selected, t);
                global_toggle_row(toggle_on, &ctx)
            } else if *field_name == "theme" {
                let theme_name = app.state.agent_config.theme.display_name();
                let ctx = GlobalRowCtx::new(label, desc, is_selected, t);
                global_theme_row(theme_name, &ctx)
            } else if *field_name == "thinking_style" {
                let style_name = app.state.agent_config.thinking_style.display_name();
                let ctx = GlobalRowCtx::new(label, desc, is_selected, t);
                global_theme_row(style_name, &ctx)
            } else if *field_name == "system_prompt"
                || *field_name == "agent_md"
                || *field_name == "style"
            {
                let mut ctx = GlobalRowCtx::new(label, desc, is_selected, t);
                ctx.hint = "Enter \u{7f16}\u{8f91}".to_string();
                global_preview_row(&value, &ctx)
            } else {
                let ctx = GlobalRowCtx::new(label, desc, is_selected, t);
                global_text_row(
                    &value,
                    app.ui.config_editing,
                    app.ui.config_edit_cursor,
                    &ctx,
                )
            };
            list.push(line);
        }
    }
    list
}
