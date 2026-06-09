//! 编辑器渲染逻辑

use super::MarkdownEditor;
use super::selection::{RenderedVL, inclusive_end_col, visual_line_selection_range};
use crate::tui::components::selection::{normalize_selection, rebuild_spans_with_selection};
use crate::tui::editor_core::vim::{Mode, filter_commands, filter_insert_commands};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

impl MarkdownEditor {
    /// 渲染编辑器
    pub fn render(&mut self, f: &mut Frame<'_>, area: Rect) {
        // 计算可用内容区域
        //
        // 布局：area 自上而下为 top-border / 内容 / bottom-border-row
        // 状态栏直接画在最后一行，覆盖 block 的 bottom border；
        // 当处于命令/搜索/命令面板模式时，命令栏画在倒数第二行，会再吃掉一行内容。
        //
        // 因此：
        //  - 普通模式 content_height = area.height - 2（顶边框 + 状态栏覆盖的底行）
        //  - 命令栏可见时再 - 1
        let has_cmd_bar = matches!(
            self.vim.mode(),
            Mode::Command(_) | Mode::Search(_) | Mode::CommandPanel(_)
        );
        let h_pad = self.border_pad();
        let v_pad = self.top_pad();
        // reserved = 顶部留白（边框或空行）+ 状态栏 1 行 + 命令栏（可选）
        let reserved_rows: u16 = v_pad + 1 + if has_cmd_bar { 1 } else { 0 };
        let content_height = area.height.saturating_sub(reserved_rows) as usize;
        let content_width = area.width.saturating_sub(h_pad * 2) as usize; // 左右边框

        self.viewport.height = content_height;
        self.viewport.width = content_width;
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };
        let wrap_width = content_width.saturating_sub(line_num_width);
        self.wrap.set_width(wrap_width);

        // 折行宽度按"光标行 vs 其它行"区分：光标换行时强制重建一次缓存，
        // 让旧光标行回到"渲染宽"路径、新光标行切到"源码宽"路径。
        self.maybe_mark_wrap_dirty_for_cursor();

        // 重建折行元数据（视觉行计数 + 前缀和）
        if self.wrap.is_dirty() {
            self.rebuild_wrap_cache();
        }

        let (cursor_row, mut cursor_col) = self.buffer.cursor();
        let line_count = self.buffer.line_count();

        // Vim Normal 模式下光标不能在行尾（最后一个字符之后），
        // 需要限制到行内最后一个字符上，否则会渲染一个多余的空光标块
        if *self.vim.mode() == Mode::Normal {
            let line_len = self.buffer.current_line_len();
            if line_len > 0 {
                cursor_col = cursor_col.min(line_len - 1);
            }
        }

        // 确保块级缓存有效（fenced 代码块、表格 - 用于快速判断行所属 block 类型）
        self.renderer
            .ensure_cache_valid(self.buffer.lines(), wrap_width);

        // ---- 阶段 1：基于当前 scroll_offset 计算渲染范围 ----
        // 计算视口范围内需要渲染的逻辑行（O(log n)）
        //
        // 现在 wrap_engine 的 visual_line_count() 已经包含表格/代码块的真实渲染高度
        // （由 rebuild_cache_with_blocks 在表格首行膨胀），所以 visual_to_logical
        // 直接给出准确的源码行号区间，不再需要 expand_render_range_for_tables
        // 这种"为表格手工外扩"的补丁。
        let first_visible_visual = self.viewport.scroll_offset;
        let last_visible_visual = self.viewport.scroll_offset + content_height;
        let (start_logical, _) = self.wrap.visual_to_logical(first_visible_visual);
        let (end_logical, _) = self.wrap.visual_to_logical(last_visible_visual);

        // 扩展范围：往前一点确保表格首行进入渲染窗口（其它续行被 wrap_engine
        // 标为 count=0，不影响 visual 坐标，但渲染必须从首行触发）。
        let mut render_start = start_logical.saturating_sub(2).min(cursor_row);
        let render_end = (end_logical + 3).min(line_count).max(cursor_row + 1);

        // 如果 render_start 落在某个表格的续行内，把它前推到表格首行
        if let Some((tbl_start, _)) = self.wrap.table_block_for_line(render_start) {
            render_start = render_start.min(tbl_start);
        }

        // 为视口范围构建详细视觉行缓存（只构建未缓存的行）
        self.wrap
            .build_range(self.buffer.lines(), render_start, render_end);

        // 渲染起始的视觉偏移（O(1)）；这是 all_visual_lines 在全局视觉坐标中的起点
        let visual_offset = self.wrap.visual_offset_of(render_start);

        let mut all_visual_lines: Vec<Line<'static>> = Vec::new();
        let mut all_vl_meta: Vec<RenderedVL> = Vec::new();

        for logical_line in render_start..render_end {
            let is_cursor_line = logical_line == cursor_row;
            let cached = self.wrap.get_cached_lines(logical_line);

            for vl in cached {
                let is_insert_mode = *self.vim.mode() == Mode::Insert;
                let rendered = self.renderer.render_visual_line(
                    vl,
                    is_cursor_line,
                    if is_cursor_line {
                        Some(cursor_col)
                    } else {
                        None
                    },
                    &self.search,
                    &self.buffer,
                    wrap_width,
                    is_insert_mode,
                );

                if rendered.is_empty() {
                    // 表格续行：不贡献渲染输出。wrap_engine 已经把整张表的渲染高度
                    // 灌进首行的 visual_count，续行 count=0；所以这里的"空输出"也
                    // 不会让全局视觉坐标错位。
                    continue;
                }

                let n = rendered.len();
                let meta_entry = RenderedVL {
                    logical_line,
                    start_col: vl.start_col,
                    end_col: vl.end_col,
                };
                for _ in 0..n {
                    all_vl_meta.push(meta_entry.clone());
                }
                all_visual_lines.extend(rendered);
            }
        }

        // ---- 阶段 2：基于光标位置同步视口（如果未锁定） ----
        // 现在视觉坐标是统一的：wrap_engine 的视觉行号 = all_visual_lines 在全局
        // 渲染坐标中的索引（除了 Insert 模式下光标在表格里时偶发的 ±1 抖动）。
        if !self.viewport.scroll_locked {
            let cursor_visual_global = self.wrap.logical_to_visual(cursor_row, cursor_col);

            let visible_start_global = self.viewport.scroll_offset;
            let visible_end_global = visible_start_global + content_height;

            if cursor_visual_global < visible_start_global {
                self.viewport.scroll_offset = cursor_visual_global;
            } else if cursor_visual_global >= visible_end_global {
                self.viewport.scroll_offset =
                    cursor_visual_global.saturating_sub(content_height - 1);
            }
        }

        // 视口上界 clamp（避免越界）
        let max_offset = self
            .wrap
            .visual_line_count()
            .saturating_sub(1)
            .max(self.viewport.scroll_offset);
        if self.viewport.scroll_offset > max_offset {
            self.viewport.scroll_offset = max_offset;
        }

        // 保存"未叠选区高亮"的渲染输出，用于鼠标拖选复制时提取可见文本。
        // 必须在叠加 Visual / mouse_selection 高亮之前 clone——这样
        // extract_selection_text 看到的是干净的渲染 spans，is_decorative_span
        // 才能准确识别哪些 span 是边框 / padding。
        self.render_meta.rendered_lines = all_visual_lines.clone();
        self.render_meta.rendered_offset = visual_offset;

        // Visual 模式：对选区范围内的行应用精确字符级高亮
        if *self.vim.mode() == Mode::Visual {
            let (vs_row, vs_col) = self.vim.visual_start();
            let (ve_row, ve_col) = (cursor_row, cursor_col);
            let ((sr, sc), (er, ec)) = normalize_selection((vs_row, vs_col), (ve_row, ve_col));

            // 把 ec 扩展为"包含光标字符"的半开区间右端，让屏幕高亮
            // 与 vim::get_selection_text 复制到剪贴板的内容保持一致。
            let er_line_len = self.buffer.line(er).map(|l| l.chars().count()).unwrap_or(0);
            let ec = inclusive_end_col(er_line_len, ec);

            let sel_fg = self.theme.text_normal;
            let sel_bg = Color::DarkGray;
            let line_num_chars = if self.renderer.is_show_line_numbers() {
                6usize
            } else {
                0usize
            };

            for (idx, meta) in all_vl_meta.iter().enumerate() {
                // 计算该视觉行与选区 [sr,sc)-(er,ec) 的交集字符范围
                let (hl_start, hl_end) = visual_line_selection_range(meta, sr, sc, er, ec);
                if hl_start >= hl_end {
                    continue; // 无交集
                }

                // 转为视觉行内的局部字符偏移（相对于 vl.start_col）
                let local_start = hl_start.saturating_sub(meta.start_col);
                let local_end = hl_end.saturating_sub(meta.start_col);

                if let Some(line) = all_visual_lines.get_mut(idx) {
                    line.spans = rebuild_spans_with_selection(
                        &line.spans,
                        line_num_chars,
                        local_start,
                        local_end,
                        sel_fg,
                        sel_bg,
                    );
                }
            }
        }

        // 鼠标拖选高亮（渲染坐标）。与 Vim Visual 互斥使用——鼠标拖选不进
        // Vim Visual 模式，Vim Visual 由键盘 v 触发；两个 if 块不会同时命中。
        if let Some(sel) = self.mouse_selection {
            let ((sr, sc), (er, ec)) = normalize_selection(sel.anchor, sel.current);
            let sel_fg = self.theme.text_normal;
            let sel_bg = Color::DarkGray;

            // 局部下标 idx 的全局渲染行号 = visual_offset + idx
            for (idx, line) in all_visual_lines.iter_mut().enumerate() {
                let gline = visual_offset + idx;
                if gline < sr || gline > er {
                    continue;
                }
                let render_start = if gline == sr { sc } else { 0 };
                let render_end = if gline == er { ec } else { usize::MAX };

                // rebuild_spans_with_selection 用半开区间 [start, end)；
                // 把整行字符数当作 end 的上限。
                let line_chars: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
                let hl_end = render_end.min(line_chars);
                let hl_start = render_start.min(line_chars);
                if hl_start >= hl_end {
                    continue;
                }
                line.spans =
                    rebuild_spans_with_selection(&line.spans, 0, hl_start, hl_end, sel_fg, sel_bg);
            }
        }

        // ---- 阶段 3：提取可见范围并渲染 ----
        // visible_start_local 是 all_visual_lines 数组的索引（局部坐标）；
        // 全局视觉行 N → 局部 N - visual_offset。
        let visible_start_local = self
            .viewport
            .scroll_offset
            .saturating_sub(visual_offset)
            .min(all_visual_lines.len().saturating_sub(1));
        let visible_end_local = (visible_start_local + content_height).min(all_visual_lines.len());

        // EOF 兜底：当渲染窗口已包含到 EOF（render_end == line_count）但局部窗口
        // 装不满 content_height 时，把视口底锚定到 all_visual_lines 末尾。
        // 这覆盖一种特殊情形：scroll_offset 推到了很大的值（例如 EOF 表格场景下
        // move_cursor_visual_down 主动 +1 推视口），但 visual_offset 落后于 scroll_offset
        // 导致 visible_start_local 过大、可见行不足。
        let (visible_start_local, visible_end_local) = if render_end >= line_count
            && visible_end_local - visible_start_local < content_height
            && all_visual_lines.len() > content_height
        {
            let start = all_visual_lines.len().saturating_sub(content_height);
            (start, all_visual_lines.len())
        } else {
            (visible_start_local, visible_end_local)
        };

        let visible_start = visible_start_local;
        let visible_end = visible_end_local;

        // rendered_vl_map_index 是当前屏幕顶部对应的渲染行索引
        self.render_meta.vl_map = all_vl_meta;
        self.render_meta.map_index = visible_start;
        self.render_meta.rendered_line_count = all_visual_lines.len();

        let mut lines_to_render: Vec<Line<'static>> = if visible_start < all_visual_lines.len() {
            all_visual_lines[visible_start..visible_end].to_vec()
        } else {
            Vec::new()
        };

        // 填充空行
        for _ in lines_to_render.len()..content_height {
            lines_to_render.push(Line::from(Span::styled(
                "~",
                Style::default()
                    .fg(Color::DarkGray)
                    .bg(self.theme.bg_primary),
            )));
        }

        // 渲染主内容
        let border_color = self.vim.mode().border_color();
        let block = if self.show_border {
            Block::default()
                .title(format!(" {} ", self.title))
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .style(Style::default().bg(self.theme.bg_primary))
        } else {
            // 关闭边框时，标题留给外层 UI；保留背景色，并在顶部留 1 行呼吸空间
            Block::default()
                .padding(ratatui::widgets::Padding::new(0, 0, 1, 0))
                .style(Style::default().bg(self.theme.bg_primary))
        };

        let paragraph = Paragraph::new(lines_to_render).block(block);
        f.render_widget(paragraph, area);

        // 渲染状态栏
        let status_bar = self.render_status_bar(area.width as usize);
        let status_area = Rect::new(area.x, area.y + area.height - 1, area.width, 1);
        let status_block = Block::default().style(Style::default().bg(self.theme.bg_primary));
        f.render_widget(Paragraph::new(status_bar).block(status_block), status_area);

        // 渲染命令/搜索栏
        if matches!(
            self.vim.mode(),
            Mode::Command(_) | Mode::Search(_) | Mode::CommandPanel(_)
        ) {
            let cmd_bar = self.render_command_bar();
            let cmd_area = Rect::new(area.x, area.y + area.height - 2, area.width, 1);
            let cmd_block = Block::default().style(Style::default().bg(self.theme.bg_primary));
            f.render_widget(Paragraph::new(cmd_bar).block(cmd_block), cmd_area);
        }

        // 渲染命令面板弹窗
        match self.vim.mode() {
            Mode::CommandPanel(filter) => {
                let filter = filter.clone();
                self.render_command_popup(f, &filter, area, false);
            }
            Mode::InsertCommandPanel(filter) => {
                let filter = filter.clone();
                self.render_command_popup(f, &filter, area, true);
            }
            _ => {}
        }

        // 渲染主题选择弹窗
        if self.vim.mode() == &Mode::ThemeSelect {
            self.render_theme_popup(f, area);
        }

        // 渲染帮助弹窗
        if self.vim.mode() == &Mode::HelpPopup {
            self.render_help_popup(f, area);
        }
    }

    /// 渲染状态栏
    fn render_status_bar(&self, width: usize) -> Line<'static> {
        let mode_str = format!(" {} ", self.vim.mode());
        let (row, col) = self.buffer.cursor();
        let pos_str = format!(" {}:{} ", row + 1, col + 1);
        let wrap_str = if self.wrap.is_enabled() {
            " WRAP "
        } else {
            " NOWRAP "
        };
        let hints: String = if let Some(ref msg) = self.status_message {
            msg.clone()
        } else {
            " Ctrl+S 保存 | Ctrl+Q 取消 | / 命令面板 ".to_string()
        };

        let used_width = mode_str.len() + pos_str.len() + wrap_str.len() + hints.len();
        let separator = " ".repeat(width.saturating_sub(used_width));

        let hints_style = if self.status_message.is_some() {
            Style::default()
                .fg(self.theme.text_bold)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(self.theme.text_dim)
        };

        Line::from(vec![
            Span::styled(
                mode_str,
                Style::default()
                    .fg(Color::Black)
                    .bg(self.vim.mode().border_color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(pos_str, Style::default().fg(self.theme.text_dim)),
            Span::styled(wrap_str, Style::default().fg(self.theme.text_dim)),
            Span::styled(separator, Style::default().fg(self.theme.text_normal)),
            Span::styled(hints, hints_style),
        ])
    }

    /// 渲染命令栏
    fn render_command_bar(&self) -> Line<'static> {
        let cursor_style = Style::default()
            .fg(self.theme.cursor_fg)
            .bg(self.theme.cursor_bg)
            .add_modifier(Modifier::BOLD);
        let text_style = Style::default().fg(self.theme.text_normal);
        let hint_style = Style::default().fg(self.theme.text_dim);
        match self.vim.mode() {
            Mode::Command(cmd) => Line::from(vec![
                Span::styled(":", text_style),
                Span::styled(cmd.clone(), text_style),
                Span::styled(" ", cursor_style),
                Span::styled("  Esc:取消  Enter:执行", hint_style),
            ]),
            Mode::Search(pattern) => {
                let count = self.search.match_count();
                let hint = if pattern.is_empty() {
                    "  Esc:取消  Enter:跳到匹配".to_string()
                } else {
                    format!("  [{}匹配]  Esc:取消  Enter:跳转  n/N:上下条", count)
                };
                Line::from(vec![
                    Span::styled("/", Style::default().fg(Color::Magenta)),
                    Span::styled(pattern.clone(), text_style),
                    Span::styled(" ", cursor_style),
                    Span::styled(hint, hint_style),
                ])
            }
            Mode::CommandPanel(filter) => Line::from(vec![
                Span::styled("/", Style::default().fg(Color::Magenta)),
                Span::styled(filter.clone(), text_style),
                Span::styled(" ", cursor_style),
                Span::styled("  Esc:取消  Enter:执行", hint_style),
            ]),
            _ => Line::default(),
        }
    }

    /// 计算 Insert 命令面板的屏幕坐标（位于触发的 `/` 字符下方一行）
    ///
    /// 逻辑：
    ///  1. 在上一帧的 `render_meta.vl_map` 中，找到包含 `insert_panel_anchor`
    ///     `(row, col)` 的视觉行索引（相对屏幕顶部）。
    ///  2. Y = area.y + 1 (上边框) + 视觉行索引 + 1 (锚点下方一行)
    ///  3. X = area.x + 1 (左边框) + line_num_width + 锚点列在视觉行内的显示宽度
    ///
    /// 若信息不足以定位（首次渲染、anchor 不在视口等），fall back 到默认底部位置。
    fn compute_insert_popup_position(
        &self,
        area: Rect,
        popup_width: u16,
        popup_height: u16,
    ) -> (u16, u16) {
        let fallback = || -> (u16, u16) {
            let x = area.x + 2;
            let y = area
                .bottom()
                .saturating_sub(popup_height + 2)
                .max(area.y + 2);
            (x, y)
        };

        let Some((anchor_row, anchor_col)) = self.insert_panel_anchor else {
            return fallback();
        };

        // 在 vl_map 中找到包含 anchor 的视觉行
        let line_num_width: u16 = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };
        let map_index = self.render_meta.map_index;
        let vl_map = &self.render_meta.vl_map;
        let h_pad = self.border_pad();
        let v_pad = self.top_pad();
        let content_height = self.viewport_content_height(area);

        let mut found_screen_y: Option<u16> = None;
        let mut found_start_col: usize = 0;
        for screen_y in 0..content_height {
            let idx = map_index + screen_y;
            if let Some(meta) = vl_map.get(idx)
                && meta.logical_line == anchor_row
                && anchor_col >= meta.start_col
                && anchor_col < meta.end_col.max(meta.start_col + 1)
            {
                found_screen_y = Some(screen_y as u16);
                found_start_col = meta.start_col;
                break;
            }
        }

        let Some(screen_y) = found_screen_y else {
            return fallback();
        };

        // 计算锚点在视觉行内的显示宽度
        let line_text = match self.buffer.line(anchor_row) {
            Some(s) => s,
            None => return fallback(),
        };
        let prefix_text: String = line_text
            .chars()
            .skip(found_start_col)
            .take(anchor_col.saturating_sub(found_start_col))
            .collect();
        let display_x = unicode_width::UnicodeWidthStr::width(prefix_text.as_str()) as u16;

        // 屏幕坐标：area 内左上角是 area.x/area.y，加边框/留白偏移（关闭边框时水平为 0、垂直仍为 1）
        let mut x = area.x + h_pad + line_num_width + display_x;
        let mut y = area.y + v_pad + screen_y + 1; // 锚点下方一行

        // 边界处理：popup 不能溢出 area
        let max_x = area.x + area.width.saturating_sub(popup_width + h_pad);
        if x > max_x {
            x = max_x;
        }
        if x < area.x + h_pad {
            x = area.x + h_pad;
        }
        // 如果下方装不下 popup_height，就放到锚点上方
        if y + popup_height > area.y + area.height.saturating_sub(1) {
            let above_y = area.y + v_pad + screen_y;
            if above_y >= popup_height {
                y = above_y.saturating_sub(popup_height);
            } else {
                // 上下都不够，截断到顶部
                y = area.y + v_pad;
            }
        }

        (x, y)
    }

    /// 渲染命令面板弹窗
    ///
    /// `is_insert`:
    ///  - false: Normal 模式触发的命令面板（COMMANDS）
    ///  - true:  Insert 模式触发（INSERT_COMMANDS），仅 image / `/`
    fn render_command_popup(
        &mut self,
        f: &mut Frame<'_>,
        filter: &str,
        area: Rect,
        is_insert: bool,
    ) {
        let items = if is_insert {
            filter_insert_commands(filter)
        } else {
            filter_commands(filter)
        };
        if items.is_empty() {
            return;
        }

        let item_count = items.len();
        let popup_height = (item_count as u16 + 2).min(area.height.saturating_sub(4));

        // 宽度计算与渲染保持一致：
        //   pointer(2) + name(对齐到 max_name_w) + GAP(3) + desc(完整显示)
        // 旧实现的 bug：max_label_width 假设 name 后 3 空格间隔，但渲染却用
        // `format!("{:<10}", name)` 把 name 列硬编码为 10 字符——name 短于 10
        // 时弹窗宽度被低估、desc 被截；name 长于 10 时根本没有间隔，紧贴 desc。
        const POINTER_W: usize = 2;
        const GAP: usize = 3;
        let max_name_w = items
            .iter()
            .map(|cmd| unicode_width::UnicodeWidthStr::width(cmd.name))
            .max()
            .unwrap_or(0);
        let max_desc_w = items
            .iter()
            .map(|cmd| unicode_width::UnicodeWidthStr::width(cmd.desc))
            .max()
            .unwrap_or(0);
        let content_w = POINTER_W + max_name_w + GAP + max_desc_w;
        // +2 给左右边框；保底 16，避免空标题/极短列表时弹窗过窄
        let popup_width = ((content_w + 2) as u16)
            .max(16)
            .min(area.width.saturating_sub(4));

        // 位置：
        //  - Normal 命令面板：编辑区底部偏左（保持原行为）
        //  - Insert 命令面板：锚定在触发的 `/` 字符下方一行
        let (x, y) = if is_insert {
            self.compute_insert_popup_position(area, popup_width, popup_height)
        } else {
            let x = area.x + 2;
            let y = area
                .bottom()
                .saturating_sub(popup_height + 2) // 留出状态栏和命令栏
                .max(area.y + 2);
            (x, y)
        };
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        // 标题
        let title_prefix = if is_insert {
            " 插入 "
        } else {
            " 命令面板 "
        };
        let title = if filter.is_empty() {
            title_prefix.to_string()
        } else {
            format!("{}[{}] ", title_prefix, filter)
        };

        // 确保选中项在范围内
        self.cmd_popup_selected = self.cmd_popup_selected.min(item_count.saturating_sub(1));

        // 构建列表项
        let accent = self.theme.popup_border;
        let popup_bg = self.theme.popup_bg;
        let dim_color = self.theme.text_dim;
        let label_ai = self.theme.label_ai;
        let selected_style = self.theme.popup_highlight_fg.apply_fg(Style::default());
        let highlight_style = self
            .theme
            .popup_highlight_fg
            .apply_fg(self.theme.popup_highlight_bg.apply_bg(Style::default()))
            .add_modifier(Modifier::BOLD);
        let gap_str = " ".repeat(GAP);
        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, cmd)| {
                let is_selected = i == self.cmd_popup_selected;
                let name_style = if is_selected {
                    selected_style.add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(label_ai)
                };
                let desc_style = if is_selected {
                    selected_style
                } else {
                    Style::default().fg(dim_color)
                };
                let pointer = if is_selected { "❯ " } else { "  " };
                // name 列动态对齐到 max_name_w（命令名都是 ASCII，char 数等于显示宽度）。
                let name_padded = format!("{:<width$}", cmd.name, width = max_name_w);
                ListItem::new(Line::from(vec![
                    Span::styled(pointer.to_string(), name_style),
                    Span::styled(name_padded, name_style),
                    Span::raw(gap_str.clone()),
                    Span::styled(cmd.desc.to_string(), desc_style),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.cmd_popup_selected));

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(accent))
                    .title(Span::styled(
                        title,
                        Style::default().fg(accent).add_modifier(Modifier::BOLD),
                    ))
                    .style(Style::default().bg(popup_bg)),
            )
            .highlight_style(highlight_style);

        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut list_state);
    }

    /// 渲染主题选择弹窗
    fn render_theme_popup(&mut self, f: &mut Frame<'_>, area: Rect) {
        let item_count = self.themes.gallery.len();
        if item_count == 0 {
            return;
        }

        let popup_height = (item_count as u16 + 2).min(area.height.saturating_sub(4));
        let popup_width = 28u16.min(area.width.saturating_sub(4));

        // 位置：编辑区底部偏左
        let x = area.x + 2;
        let y = area
            .bottom()
            .saturating_sub(popup_height + 2)
            .max(area.y + 2);
        let popup_area = Rect::new(x, y, popup_width, popup_height);

        // 确保选中项在范围内
        self.themes.popup_selected = self.themes.popup_selected.min(item_count.saturating_sub(1));

        // 构建列表项
        let accent = self.theme.popup_border;
        let title_color = self.theme.popup_title;
        let popup_bg = self.theme.popup_bg;
        let text_color = self.theme.text_normal;
        let current_color = self.theme.md_link;
        let selected_style = self.theme.popup_highlight_fg.apply_fg(Style::default());
        let highlight_style = self
            .theme
            .popup_highlight_fg
            .apply_fg(self.theme.popup_highlight_bg.apply_bg(Style::default()))
            .add_modifier(Modifier::BOLD);
        let list_items: Vec<ListItem> = self
            .themes
            .gallery
            .iter()
            .enumerate()
            .map(|(i, (name, _, _))| {
                let is_selected = i == self.themes.popup_selected;
                let is_current = i == self.themes.current_index;
                let pointer = if is_selected { "❯ " } else { "  " };
                let check = if is_current { " ●" } else { "" };
                let name_style = if is_selected {
                    selected_style.add_modifier(Modifier::BOLD)
                } else if is_current {
                    Style::default().fg(current_color)
                } else {
                    Style::default().fg(text_color)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(pointer.to_string(), name_style),
                    Span::styled(format!("{}{}", name, check), name_style),
                ]))
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.themes.popup_selected));

        let list = List::new(list_items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(ratatui::widgets::BorderType::Rounded)
                    .border_style(Style::default().fg(accent))
                    .title(Span::styled(
                        " 选择主题 ",
                        Style::default()
                            .fg(title_color)
                            .add_modifier(Modifier::BOLD),
                    ))
                    .style(Style::default().bg(popup_bg)),
            )
            .highlight_style(highlight_style);

        f.render_widget(Clear, popup_area);
        f.render_stateful_widget(list, popup_area, &mut list_state);
    }

    /// 渲染帮助页面（全屏覆盖编辑区域）
    fn render_help_popup(&mut self, f: &mut Frame<'_>, area: Rect) {
        let accent = self.theme.md_h1;
        let bg = self.theme.bg_primary;
        let text_color = self.theme.text_normal;
        let dim_color = self.theme.text_dim;

        // 辅助：快捷键行
        let key = |k: &str| -> Span<'static> {
            let padded = format!(" {:<10}", k);
            Span::styled(padded, Style::default().fg(accent).bg(bg))
        };
        let desc = |d: &str| -> Span<'static> {
            Span::styled(d.to_string(), Style::default().fg(text_color).bg(bg))
        };
        let section = |s: &str| -> Line<'static> {
            Line::from(Span::styled(
                format!("  ── {} ──", s),
                Style::default().fg(dim_color).bg(bg),
            ))
        };
        let blank = || -> Line<'static> { Line::from(Span::styled(" ", Style::default().bg(bg))) };

        let help_lines: Vec<Line<'static>> = vec![
            Line::from(Span::styled(
                "  Markdown 编辑器帮助指南",
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("  {}", "─".repeat(area.width.saturating_sub(4) as usize)),
                Style::default().fg(dim_color).bg(bg),
            )),
            blank(),
            section("模式切换"),
            Line::from(vec![key("i"), desc("进入 Insert 模式（编辑文本）")]),
            Line::from(vec![key("Esc"), desc("退出到 Normal 模式")]),
            Line::from(vec![key("v"), desc("进入 Visual 模式（选择文本）")]),
            blank(),
            section("光标移动"),
            Line::from(vec![key("h/j/k/l"), desc("左 / 下 / 上 / 右")]),
            Line::from(vec![key("w / b / e"), desc("下一个词 / 上一个词 / 词尾")]),
            Line::from(vec![key("0 / $"), desc("行首 / 行尾")]),
            Line::from(vec![key("gg / G"), desc("文档开头 / 结尾")]),
            Line::from(vec![key("Ctrl-D/U"), desc("下 / 上翻半页")]),
            blank(),
            section("编辑操作"),
            Line::from(vec![key("d"), desc("删除当前行")]),
            Line::from(vec![key("x"), desc("删除当前字符")]),
            Line::from(vec![key("p"), desc("粘贴（yank 寄存器）")]),
            Line::from(vec![key("u"), desc("撤销")]),
            Line::from(vec![key("Ctrl-r"), desc("重做")]),
            blank(),
            section("Visual 选区"),
            Line::from(vec![key("y"), desc("Yank 到内部寄存器")]),
            Line::from(vec![key("c"), desc("复制到系统剪贴板")]),
            blank(),
            section("鼠标操作"),
            Line::from(vec![key("左键点击"), desc("定位光标")]),
            Line::from(vec![key("左键拖拽"), desc("选择文本（进入 Visual）")]),
            Line::from(vec![key("滚轮"), desc("滚动视口")]),
            blank(),
            section("搜索"),
            Line::from(vec![key("/"), desc("开始搜索")]),
            Line::from(vec![key("n / N"), desc("下一个 / 上一个匹配")]),
            blank(),
            section("命令面板 (:)"),
            Line::from(vec![key("wrap"), desc("启用自动折行")]),
            Line::from(vec![key("nowrap"), desc("禁用折行")]),
            Line::from(vec![key("theme"), desc("切换主题")]),
            Line::from(vec![key("help"), desc("显示帮助")]),
            Line::from(vec![key("line-number"), desc("显示行号")]),
            Line::from(vec![key("no-line-number"), desc("隐藏行号")]),
            blank(),
            section("全局快捷键"),
            Line::from(vec![key("Ctrl-s"), desc("保存并退出")]),
            Line::from(vec![key("Ctrl-q"), desc("取消退出")]),
        ];

        // 渲染帮助内容（留出底部状态栏 1 行）
        let content_area = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
        let paragraph = Paragraph::new(help_lines).style(Style::default().bg(bg));

        f.render_widget(Clear, content_area);
        f.render_widget(paragraph, content_area);

        // 底部状态栏：提示按任意键返回
        let status_y = area.y + area.height.saturating_sub(1);
        let status_area = Rect::new(area.x, status_y, area.width, 1);
        let status_line = Line::from(vec![
            Span::styled(
                " HELP ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" 按任意键返回编辑器", Style::default().fg(dim_color).bg(bg)),
        ]);
        f.render_widget(Clear, status_area);
        f.render_widget(
            Paragraph::new(status_line).style(Style::default().bg(bg)),
            status_area,
        );
    }
}
