use super::app::{AppMode, FlatEntryKind, Focus, NotebookApp};
use crate::tui::components::{
    CommandItem, CommandPopupConfig, ConfirmDialogConfig, StatusInputParams, cursor_wrapped_lines,
    draw_command_popup as render_command_popup, draw_confirm_dialog, draw_status_input,
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
};

/// notebook 顶层布局结果。
///
/// 这是 UI 与鼠标处理共享的"事实来源"：`draw_ui` 只通过 [`compute_layout`]
/// 拿到这些 Rect 用于渲染；`handler::compute_mouse_layout` 也调用
/// [`compute_layout`]，把同一份 Rect 用于命中检测。
///
/// 任何对顶栏 / 帮助栏高度、左右分栏比例的改动都应当只改 [`compute_layout`]
/// 一处，UI 与鼠标自动同步。
#[derive(Debug, Clone, Copy)]
pub struct NotebookLayout {
    /// 顶部栏（Normal/Adding/Renaming = 标题；其它模式 = 状态/输入栏）
    pub top: Rect,
    /// 主区域（左侧列表 + 右侧编辑器）
    pub main: Rect,
    /// 底部帮助栏（固定 1 行）
    pub footer: Rect,
    /// 左侧列表区。仅 Normal/CommandPopup 模式下渲染，其它模式为 None
    /// （因为顶部输入条会"挤掉"两栏视觉，鼠标也不需要点列表）。
    pub list: Option<Rect>,
    /// 右侧编辑器区。同上。
    pub editor: Option<Rect>,
    /// 列表 / 编辑器 之间分栏线的 X 坐标（用于鼠标拖拽调整比例）
    pub divider_x: Option<u16>,
}

/// 把 frame_area + 当前 app 状态计算成 [`NotebookLayout`]。
///
/// **唯一**的布局源头：`draw_ui` 与鼠标处理都基于它。修改顶栏 / 帮助栏 /
/// 分栏几何只需在这里改一处。
pub fn compute_layout(frame_area: Rect, app: &NotebookApp) -> NotebookLayout {
    // 顶栏高度按 mode 浮动：
    // - Normal/Adding/Renaming：标题栏 = 一行文字 + 一行底分隔线 = 2 行
    // - 其它模式（Mkdir/Mv/Search/RatioInput/CommandPopup/ConfirmDelete）：
    //   走 status_input 组件，组件自带边框，需要 3 行
    let top_height: u16 = if matches!(
        app.mode,
        AppMode::Normal | AppMode::Adding | AppMode::Renaming
    ) {
        2
    } else {
        3
    };

    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_height),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(frame_area);
    let top = v[0];
    let main = v[1];
    let footer = v[2];

    // 仅 Normal / CommandPopup 显示双栏视图；其它模式整个主区域逻辑上还是一个
    // 整体（输入条等弹窗在上方），鼠标也不会去打这两个区域。
    let two_panel = matches!(app.mode, AppMode::Normal | AppMode::CommandPopup);
    let (list, editor, divider_x) = if two_panel {
        let h = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(app.panel_ratio),
                Constraint::Percentage(100 - app.panel_ratio),
            ])
            .split(main);
        (Some(h[0]), Some(h[1]), Some(h[1].x))
    } else {
        (None, None, None)
    };

    NotebookLayout {
        top,
        main,
        footer,
        list,
        editor,
        divider_x,
    }
}

/// 绘制 TUI 界面
pub fn draw_ui(f: &mut ratatui::Frame, app: &mut NotebookApp) {
    let layout = compute_layout(f.area(), app);

    // ========== 顶部栏（在 Normal 模式下是标题；其他模式用作状态/输入栏） ==========
    if matches!(
        app.mode,
        AppMode::Normal | AppMode::Adding | AppMode::Renaming
    ) {
        // Normal: 显示笔记本概览
        // Adding/Renaming: 输入框直接在列表内联渲染，顶部栏继续显示概览即可
        render_title_bar(f, app, layout.top);
    } else {
        render_status_bar(f, app, layout.top);
    }

    // ========== 主区域 ==========
    {
        // 双栏视图（list / editor）只在两栏模式存在；其它模式整个 main 给底层
        // 渲染（list_area / editor_area 不参与命中），但仍然要把列表 + 编辑区
        // 各自画出来 —— 用 fallback：未指定时用 panel_ratio 现算一下。
        let (list_area, editor_area) = match (layout.list, layout.editor) {
            (Some(l), Some(e)) => (l, e),
            _ => {
                // 单栏模式（Mkdir/Mv/Search/...）也仍渲染左右分栏，仅鼠标处理
                // 不去命中它。这里复算一次，保持视觉一致。
                let h = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(app.panel_ratio),
                        Constraint::Percentage(100 - app.panel_ratio),
                    ])
                    .split(layout.main);
                (h[0], h[1])
            }
        };

        render_list(f, app, list_area);
        render_editor(f, app, editor_area);

        // 命令面板弹窗（浮动在主区域上方）
        if app.mode == AppMode::CommandPopup {
            draw_command_popup(f, app, layout.main);
        }

        // 路径补全弹窗（与命令面板互斥）
        if app.completion_active && app.mode != AppMode::CommandPopup {
            draw_completion_popup(f, app, layout.main);
        }
    }

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => match app.focus {
            Focus::Tree => {
                " / 命令面板 | a 新建 | r 改名 | d 删除 | s 刷新 | o 打开目录 | ↑↓/jk 切换 | Enter 编辑 | q/Esc 退出"
            }
            Focus::Editor => " :w 保存 | :wq 保存退出 | :q 退出编辑 | Esc(Normal) 回目录树",
        },
        AppMode::Adding => " Enter 确认 | Tab 补全 | Esc 取消 | 目录用 / 分隔，如 ideas/note",
        AppMode::Renaming => " Enter 确认 | Tab 补全 | Esc 取消 | ←→ 移动光标",
        AppMode::Search => " Enter 搜索 | Esc 取消 | ←→ 移动光标 | Home/End 行首尾",
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
        AppMode::CommandPopup => " ↑↓/jk 选择 | Enter 确认 | 输入筛选 | Esc 取消",
        AppMode::RatioInput => " Enter 确认 | Esc 取消 | 格式: x:y (如 20:80)",
        AppMode::Mkdir => " Enter 确认 | Tab 补全 | Esc 取消 | 支持 a/b 嵌套",
        AppMode::Mv => " Enter 确认 | Tab 补全 | Esc 取消 | 末尾 / = 放入该目录",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, layout.footer);
}

/// 渲染顶部标题栏（Normal/Adding/Renaming 模式下展示笔记本概览）
fn render_title_bar(f: &mut ratatui::Frame, app: &NotebookApp, area: Rect) {
    let total = app.notes.len();
    let dir_count = app
        .flat_entries
        .iter()
        .filter(|e| matches!(e.kind, FlatEntryKind::Dir { .. }))
        .count();
    let filter_suffix = match &app.search_filter {
        Some(kw) => format!(" [搜索: {}]", kw),
        None => String::new(),
    };
    let mut title = if dir_count > 0 {
        format!(
            " 笔记本{} — {} 篇笔记, {} 个文件夹 ",
            filter_suffix, total, dir_count
        )
    } else {
        format!(" 笔记本{} — 共 {} 篇 ", filter_suffix, total)
    };
    // 当前若有提示消息，附加显示在标题后（含 Normal 下的成功提示与 Renaming 下的错误回显）
    if let Some(msg) = app.message.as_deref()
        && !msg.is_empty()
    {
        title.push_str(&format!("· {} ", msg));
    }
    let title_block = Paragraph::new(Line::from(vec![Span::styled(
        title,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        // 顶部栏：一行文字 + 一条底分隔线，视觉上"贴在"内容圆角框之上。
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(title_block, area);
}

/// 渲染笔记列表（树形结构）
fn render_list(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    let inner_width = area.width.saturating_sub(2) as usize; // 减边框
    let selected = app.state.selected();

    let mut items: Vec<ListItem> = app
        .flat_entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let is_selected = selected == Some(i);

            // 重命名模式：特殊渲染文件条目
            if let FlatEntryKind::File { note_index } = &entry.kind
                && app.mode == AppMode::Renaming
                && app.rename_index == Some(*note_index)
            {
                return build_rename_item(
                    &app.input,
                    app.cursor_pos,
                    inner_width,
                    is_selected,
                    &app.theme,
                );
            }

            // 缩进空格
            let indent_style = Style::default().fg(Color::DarkGray);

            match &entry.kind {
                FlatEntryKind::Dir {
                    name, file_count, ..
                } => {
                    let dir_style = Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD);
                    let count_str = format!(" ({})", file_count);

                    ListItem::new(Line::from(vec![
                        Span::styled(entry.guide.clone(), indent_style),
                        Span::styled(name.clone(), dir_style),
                        Span::styled(count_str, Style::default().fg(Color::DarkGray)),
                    ]))
                }
                FlatEntryKind::File { note_index } => {
                    let note = &app.notes[*note_index];
                    let name_style = Style::default().fg(Color::Reset);
                    let guide_width = unicode_width::UnicodeWidthStr::width(entry.guide.as_str());
                    let name_display_width = inner_width.saturating_sub(guide_width);
                    let display_name = note.display_name();
                    let name_text =
                        if display_name.chars().collect::<Vec<_>>().len() > name_display_width {
                            let mut s: String = display_name
                                .chars()
                                .take(name_display_width.saturating_sub(2))
                                .collect();
                            s.push_str("..");
                            s
                        } else {
                            display_name.to_string()
                        };

                    ListItem::new(Line::from(vec![
                        Span::styled(entry.guide.clone(), indent_style),
                        Span::styled(name_text, name_style),
                    ]))
                }
            }
        })
        .collect();

    // 添加模式：在列表末尾追加输入行
    if app.mode == AppMode::Adding {
        let is_selected = selected == Some(app.flat_entries.len());
        items.push(build_adding_item(
            &app.input,
            app.cursor_pos,
            inner_width,
            is_selected,
            &app.theme,
        ));
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(if app.focus == Focus::Tree {
            Color::Cyan
        } else {
            Color::DarkGray
        }))
        .title(" 笔记列表 ");

    if items.is_empty() {
        let empty_hint = List::new(vec![ListItem::new(Line::from(Span::styled(
            "   (空) 按 a 新建笔记...",
            Style::default().fg(Color::DarkGray),
        )))])
        .block(list_block);
        f.render_widget(empty_hint, area);
    } else {
        let list_widget = List::new(items).block(list_block).highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        );
        f.render_stateful_widget(list_widget, area, &mut app.state);
    }
}

/// 构建新建笔记输入行
fn build_adding_item(
    input: &str,
    cursor_pos: usize,
    width: usize,
    selected: bool,
    theme: &crate::theme::Theme,
) -> ListItem<'static> {
    let pointer = if selected {
        Span::styled(
            " ❯ ",
            Style::default()
                .fg(theme.md_h1)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("   ")
    };

    let content_width = width.saturating_sub(3); // pointer
    let cursor_lines =
        cursor_wrapped_lines(input, cursor_pos, content_width, Some("输入标题…"), theme);

    let mut item_lines: Vec<Line<'static>> = Vec::new();
    for (i, line) in cursor_lines.lines.into_iter().enumerate() {
        let mut spans = if i == 0 {
            vec![pointer.clone()]
        } else {
            vec![Span::raw("   ")]
        };
        spans.extend(line.spans);
        item_lines.push(Line::from(spans));
    }

    ListItem::new(item_lines)
}

/// 构建重命名输入行
fn build_rename_item(
    input: &str,
    cursor_pos: usize,
    width: usize,
    selected: bool,
    theme: &crate::theme::Theme,
) -> ListItem<'static> {
    build_adding_item(input, cursor_pos, width, selected, theme)
}

/// 渲染右侧编辑器区域
fn render_editor(f: &mut ratatui::Frame, app: &mut NotebookApp, area: Rect) {
    if let Some(ref mut editor) = app.editor {
        editor.render(f, area);
    } else {
        // 无内容时居中显示提示。
        // - 水平：Paragraph 自带 `alignment(Center)`
        // - 垂直：用 Layout 把 area 切成「上空白 / 一行文字 / 下空白」
        let v_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // 上方留白
                Constraint::Length(1), // 提示行
                Constraint::Min(0),    // 下方留白
            ])
            .split(area);
        let content = Paragraph::new(Line::from(Span::styled(
            "选择笔记以编辑内容",
            Style::default().fg(Color::DarkGray),
        )))
        .alignment(Alignment::Center);
        f.render_widget(content, v_chunks[1]);
    }
}

/// 渲染顶部输入/确认栏（用于 Mkdir/Mv/Search/ConfirmDelete/CommandPopup/RatioInput 模式）
fn render_status_bar(f: &mut ratatui::Frame, app: &NotebookApp, area: Rect) {
    match &app.mode {
        AppMode::Mkdir => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "新建目录",
                    label_color: Color::Cyan,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入目录名 (支持 a/b 嵌套)",
                    hint: "Enter 确认 | Tab 补全 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::Mv => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "移动笔记",
                    label_color: Color::Magenta,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "目录路径或新文件名 · 末尾 / 放入目录",
                    hint: "Enter 确认 | Tab 补全 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::Search => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "搜索",
                    label_color: Color::Cyan,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "输入关键词…",
                    hint: "Enter 搜索 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::ConfirmDelete => {
            let msg = if let Some(name) = app.selected_name() {
                format!(" 确认删除\"{}\"? (y/n)", name)
            } else {
                " 没有选中的笔记".to_string()
            };
            draw_confirm_dialog(
                f,
                area,
                &ConfirmDialogConfig {
                    title: " 确认删除 ",
                    message: msg,
                    color: Color::Red,
                },
            );
        }
        AppMode::CommandPopup => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "命令面板",
                    label_color: Color::Magenta,
                    input: &app.cmd_popup_filter,
                    cursor_pos: app.cmd_popup_filter.chars().count(),
                    placeholder: "输入筛选…",
                    hint: "↑↓ 选择 | Enter 确认 | Esc 取消",
                },
                &app.theme,
            );
        }
        AppMode::RatioInput => {
            draw_status_input(
                f,
                area,
                &StatusInputParams {
                    label: "比例",
                    label_color: Color::Yellow,
                    input: &app.input,
                    cursor_pos: app.cursor_pos,
                    placeholder: "20:80",
                    hint: "如 20:80",
                },
                &app.theme,
            );
        }
        // Normal/Adding/Renaming 走顶部标题栏，不会进入此分支
        _ => {}
    }
}

/// 绘制命令面板弹窗（浮动在主区域底部）
fn draw_command_popup(f: &mut ratatui::Frame, app: &mut NotebookApp, main_area: Rect) {
    let items = app.filtered_cmd_items();
    // hotkey 放 key 列（左侧），中文 label 放 label 列（右侧）
    let cmd_items: Vec<CommandItem<'_>> = items
        .iter()
        .map(|(_, _, label, hotkey)| CommandItem::new(hotkey, label))
        .collect();

    let title = if app.cmd_popup_filter.is_empty() {
        " 命令面板 ".to_string()
    } else {
        format!(" 命令面板 [{}] ", app.cmd_popup_filter)
    };

    render_command_popup(
        f,
        main_area,
        &CommandPopupConfig {
            title,
            items: cmd_items,
            selected: app.cmd_popup_selected,
            highlight_fg: Some(Color::Black),
            theme: &app.theme,
        },
    );
}

/// 绘制路径补全弹窗（复用命令面板组件）。
fn draw_completion_popup(f: &mut ratatui::Frame, app: &NotebookApp, main_area: Rect) {
    if app.completion_candidates.is_empty() {
        return;
    }
    let cmd_items: Vec<CommandItem<'_>> = app
        .completion_candidates
        .iter()
        .map(|c| CommandItem::new(c.as_str(), ""))
        .collect();

    let title = format!(" 补全 ({}) ", app.completion_candidates.len());

    render_command_popup(
        f,
        main_area,
        &CommandPopupConfig {
            title,
            items: cmd_items,
            selected: app.completion_selected,
            highlight_fg: Some(Color::Black),
            theme: &app.theme,
        },
    );
}
