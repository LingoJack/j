//! 自研 Markdown 编辑器
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行、Vim 模式等。

mod api;
mod commands;
mod render;
mod selection;

pub use api::{
    MarkdownEditorOpts, open_markdown_editor, open_markdown_editor_on_terminal,
    open_markdown_editor_with_content,
};
pub use selection::RenderedVL;

use super::{
    history::Snapshot,
    renderer::MarkdownRenderer,
    search::SearchState,
    text_buffer::TextBuffer,
    theme::{EditorTheme, HighlightFn},
    vim::{CmdItem, Input, Key, Mode, Transition, Vim, filter_commands, filter_insert_commands},
    wrap_engine::WrapEngine,
};

#[cfg(test)]
use super::theme::BorderStyle;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

/// 编辑器事件轮询间隔（约 60fps）。
const EDITOR_POLL_MS: u64 = 16;

/// 主题画廊项（显示名称 + 主题ID + 主题）
pub type ThemeGalleryItem = (&'static str, &'static str, EditorTheme);

/// 编辑器初始光标策略
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum CursorPolicy {
    /// 光标在文件开头（默认，向后兼容）
    #[default]
    StartOfFile,
    /// 光标在文件末尾
    EndOfFile,
}

/// 视口/滚动状态
struct ViewportState {
    /// 垂直滚动偏移（视觉行级别）
    scroll_offset: usize,
    /// 视口高度
    height: usize,
    /// 视口宽度
    width: usize,
    /// 滚轮滚动锁定：防止 render() 自动将视口拉回到光标位置
    scroll_locked: bool,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            scroll_offset: 0,
            height: 20,
            width: 80,
            scroll_locked: false,
        }
    }
}

/// 主题管理状态
struct ThemeState {
    /// 主题画廊（名称 + 主题列表）
    gallery: Vec<ThemeGalleryItem>,
    /// 当前主题在画廊中的索引
    current_index: usize,
    /// 主题选择弹窗选中项索引
    popup_selected: usize,
    /// 用户在主题画廊中选择的主题ID（退出时返回）
    selected_id: Option<&'static str>,
}

/// 渲染行元数据映射
#[derive(Default)]
struct RenderMeta {
    /// 每个屏幕行对应一个 RenderedVL（每次渲染时更新，用于鼠标点击定位）
    vl_map: Vec<RenderedVL>,
    /// 当前屏幕顶部对应的渲染行**局部下标**——即 `rendered_lines` / `vl_map`
    /// 数组里"屏幕顶行"的索引（render() 末尾写入的 `visible_start_local`）。
    ///
    /// ⚠️ 历史坑：注释一度写"全局渲染行号"，实际值是局部下标。
    /// 局部下标 vs 全局渲染行号差一个 `rendered_offset`，滚动后两者不再相等。
    /// 想拿全局行号，必须 `rendered_offset + map_index + screen_y`。
    /// 详见 `BUGS_TROUBLESHOOTING.md` —— 鼠标选区高亮位置偏移。
    map_index: usize,
    /// 上一次渲染时的实际渲染行总数（用于鼠标滚动计算 max_offset）
    /// 注意：wrap_engine 的 visual_line_count() 是源码行的视觉行计数，
    /// 但表格等元素渲染后会产生更多行。此字段存储实际渲染输出行数，
    /// 确保鼠标滚动能到达表格底部。
    rendered_line_count: usize,
    /// 上一帧渲染输出的全部 Line（**未叠选区高亮**），按局部下标排列。
    /// 全局渲染行号 = `rendered_offset + 局部下标`。用于鼠标拖选复制时
    /// 调 `extract_selection_text` 提取可见正文。每帧 render() 重新写入。
    rendered_lines: Vec<ratatui::text::Line<'static>>,
    /// `rendered_lines[0]` 对应的全局渲染行号偏移（= 当帧的 `visual_offset`）。
    rendered_offset: usize,
}

/// 鼠标拖选状态（**渲染坐标**，独立于 Vim Visual mode 的源码坐标）。
///
/// `(rendered_row, char_in_row)`：
///   - `rendered_row` 是**全局渲染行号**——不是 `rendered_lines` 的局部下标。
///     局部下标 = `rendered_row - rendered_offset`。render 端高亮循环用
///     `gline = visual_offset + idx` 比对 sr/er，必须按全局行号存。
///   - `char_in_row` 是该渲染行内拼接 spans 后的字符偏移（含装饰字符）。
///
/// 选中表格 / 代码块 / 链接时，复制走 `extract_selection_text` 从渲染 spans 提取
/// 可见内容，跳过边框 / padding / 隐藏的 markdown 语法（`[` `]()` 等）。
/// 复制内容 = 屏幕上看到的文字。
#[derive(Clone, Copy, Debug)]
struct MouseSelection {
    anchor: (usize, usize),
    current: (usize, usize),
}

/// 编辑器主结构
pub struct MarkdownEditor {
    // ---- 核心引擎 ----
    /// 文本缓冲区
    buffer: TextBuffer,
    /// 折行引擎
    wrap: WrapEngine,
    /// Vim 引擎
    vim: Vim,
    /// 搜索状态
    search: SearchState,
    /// 渲染器
    renderer: MarkdownRenderer,
    /// 主题
    theme: EditorTheme,

    // ---- 分组状态 ----
    /// 视口/滚动状态
    viewport: ViewportState,
    /// 主题管理状态
    themes: ThemeState,
    /// 渲染行元数据映射
    render_meta: RenderMeta,

    // ---- UI 杂项 ----
    /// 标题
    title: String,
    /// 命令面板选中项索引
    cmd_popup_selected: usize,
    /// 状态消息（短暂显示，下次按键清除）
    status_message: Option<String>,
    /// 进入搜索前的光标位置，用于 Esc 恢复
    cursor_before_search: Option<(usize, usize)>,
    /// Insert 模式命令面板的锚点（触发的 `/` 字符在 buffer 中的逻辑位置）
    /// 用于把 popup 渲染到 `/` 下方，而不是固定在编辑区底部
    insert_panel_anchor: Option<(usize, usize)>,
    /// 鼠标拖拽锚点（左键按下时的逻辑位置）
    mouse_anchor: Option<(usize, usize)>,
    /// 鼠标拖拽锚点（左键按下时的**渲染坐标**），用于驱动 mouse_selection。
    /// 这跟 `mouse_anchor` 是同一次按下事件的不同坐标系视图——前者用于
    /// 编辑（光标/visual_start 等），后者用于跨装饰元素的选区复制。
    mouse_render_anchor: Option<(usize, usize)>,
    /// 鼠标拖选状态（渲染坐标）。`Some` 表示当前有未清空的鼠标选区，
    /// 渲染端会叠加高亮、`y`/`c` 复制时优先用它（走渲染 spans 提取）。
    mouse_selection: Option<MouseSelection>,
    /// 是否绘制外层圆角边框（含状态栏覆盖的底边）。默认 `true`。
    /// 关闭后：
    ///  - 不画顶部 / 左右边框，标题也不渲染（外层 UI 自行画标题）；
    ///  - 内容区可用宽度 = `area.width`，可用高度仅扣除状态栏 / 命令栏；
    ///  - 鼠标坐标与各类悬浮 popup 的锚点不再做边框偏移。
    show_border: bool,
    /// 上次 `rebuild_wrap_cache()` 时光标所在的逻辑行号。
    ///
    /// 折行宽度的计算规则与"光标行 vs 其它行"耦合（光标行按源码字符宽，
    /// 其它行按 Markdown 渲染后宽度）；当光标在不同逻辑行之间移动时，
    /// 这两行的视觉行数都会变 —— 需要重建一次 wrap 缓存来吸收变化。
    ///
    /// 用 `None` 标记"尚未 rebuild 过"，强制下一次 render 走 rebuild 路径。
    last_wrap_cursor_line: Option<usize>,
}

impl MarkdownEditor {
    /// 创建新的编辑器
    pub fn new(
        title: &str,
        content: &str,
        theme: EditorTheme,
        highlight_fn: HighlightFn,
        theme_gallery: Vec<ThemeGalleryItem>,
        cursor_policy: CursorPolicy,
    ) -> Self {
        let mut buffer = TextBuffer::from_content(content);
        let initial_mode = if content.is_empty() {
            Mode::Insert
        } else {
            Mode::Normal
        };

        // 根据策略移动光标
        if cursor_policy == CursorPolicy::EndOfFile {
            buffer.move_cursor_bottom();
        }

        let mut vim = Vim::new(initial_mode.clone());
        vim.push_snapshot(Snapshot::new(buffer.snapshot()), buffer.cursor());

        let mut wrap = WrapEngine::new();
        wrap.rebuild_cache(buffer.lines());

        let renderer = MarkdownRenderer::new(theme.clone(), highlight_fn);

        let viewport_width: usize = 80; // 默认值，会在渲染时更新
        wrap.set_width(viewport_width.saturating_sub(6));

        // 查找当前主题在画廊中的索引
        let theme_index = theme_gallery
            .iter()
            .position(|(_, _, t)| *t == theme)
            .unwrap_or(0);

        Self {
            buffer,
            wrap,
            vim,
            search: SearchState::new(),
            renderer,
            theme,
            viewport: ViewportState::default(),
            themes: ThemeState {
                gallery: theme_gallery,
                current_index: theme_index,
                popup_selected: theme_index,
                selected_id: None,
            },
            render_meta: RenderMeta::default(),
            title: title.to_string(),
            cmd_popup_selected: 0,
            status_message: None,
            cursor_before_search: None,
            insert_panel_anchor: None,
            mouse_anchor: None,
            mouse_render_anchor: None,
            mouse_selection: None,
            show_border: true,
            last_wrap_cursor_line: None,
        }
    }

    /// 设置是否绘制外层边框（含上 / 左 / 右 / 底，状态栏始终画在最后一行）。
    ///
    /// 默认 `true`（与 `j report` / `j scripts` 等老入口行为一致）。
    /// `notebook` 这类自带外层 frame 的场景应在创建后调用 `set_show_border(false)`，
    /// 让编辑区与外侧 panel 视觉无缝衔接。
    pub fn set_show_border(&mut self, show: bool) {
        self.show_border = show;
    }

    /// 获取用户选择的主题ID（退出时读取）
    pub fn selected_theme_id(&self) -> Option<&'static str> {
        self.themes.selected_id
    }

    /// 获取编辑器当前全部文本内容
    pub fn content(&self) -> String {
        self.buffer.lines().join("\n")
    }

    /// 将一段文本批量插入到当前光标位置（用于终端 bracketed paste）。
    ///
    /// 直接走 `TextBuffer::insert_str`，会按 `\n` 切行；`\r` 会被过滤以兼容
    /// 跨平台粘贴。批量结束后统一重建折行缓存并压入 undo 快照，避免逐字
    /// 触发渲染节流时漏帧。该方法不改变 vim 模式，调用方按需先切到 Insert。
    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        let normalized: String = text.chars().filter(|c| *c != '\r').collect();
        if normalized.is_empty() {
            return;
        }
        self.buffer.insert_str(&normalized);
        self.rebuild_wrap_cache();
        self.vim
            .push_snapshot(Snapshot::new(self.buffer.snapshot()), self.buffer.cursor());
    }

    /// 判断编辑器是否处于"空闲 Normal 模式"（可安全拦截 Esc）
    ///
    /// 当 vim 处于 Normal 模式且无活跃搜索时返回 true。
    /// 此状态下 Esc 对编辑器无实际作用，外部可安全拦截用于焦点切换。
    pub fn is_idle_normal_mode(&self) -> bool {
        self.vim.mode() == &Mode::Normal && !self.search.is_searching()
    }

    /// 获取光标所在的视觉行
    pub fn cursor_visual_line(&self) -> usize {
        let (row, col) = self.buffer.cursor();
        self.wrap.logical_to_visual(row, col)
    }

    /// 视觉行上移（折行感知）
    pub fn move_cursor_visual_up(&mut self) {
        use crate::util::text::char_width;

        let current_visual = self.cursor_visual_line();
        if current_visual == 0 {
            return;
        }

        // 当前光标所在 logical 是否处于表格块续行（不该停留），先把光标拉回首行
        let (cursor_row, _) = self.buffer.cursor();
        if let Some((tbl_start, _tbl_end)) = self.wrap.table_block_for_line(cursor_row)
            && cursor_row != tbl_start
        {
            self.buffer.set_cursor(tbl_start, 0);
            // 直接以 tbl_start 为基准再触发一次上移
            return self.move_cursor_visual_up();
        }

        let target_visual = current_visual - 1;

        // 目标视觉行如果落在某个表格块的"膨胀区"（视觉行 N..N+height 之间），
        // 跳到表格首行的最后一个有效列；这是穿越表格上行的语义。
        if let Some((tbl_start, _)) = self.wrap.table_block_for_visual_row(target_visual) {
            let line = self.buffer.line(tbl_start).map_or("", |v| v);
            let end_col = line.chars().count();
            self.buffer.set_cursor(tbl_start, end_col);
            return;
        }

        let (current_row, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;

            // 保持视觉列位置：计算当前光标在当前视觉行中的屏幕偏移
            let current_vl = self.wrap.get_visual_line(current_visual);
            let current_start_col = current_vl.map(|vl| vl.start_col).unwrap_or(0);
            let current_line_text = self.buffer.line(current_row).map_or("", |v| v);
            let visual_x: usize = current_line_text
                .chars()
                .skip(current_start_col)
                .take(current_col.saturating_sub(current_start_col))
                .map(char_width)
                .sum();

            // 在目标视觉行中找到最接近该视觉 X 的逻辑列
            let target_line_text = self.buffer.line(logical_line).map_or("", |v| v);
            let new_col = if target_line_text.is_empty() {
                0
            } else {
                let segment: String = target_line_text.chars().skip(start_col).collect();
                Self::screen_col_to_char_offset(&segment, visual_x) + start_col
            };
            let new_col = new_col.min(end_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    /// 视觉行下移（折行感知）
    pub fn move_cursor_visual_down(&mut self) {
        use crate::util::text::char_width;

        let current_visual = self.cursor_visual_line();
        let total_visual = self.wrap.visual_line_count();

        // 当前光标所在 logical 是否处于表格块续行（不该停留），先把光标拉到首行
        let (cursor_row, _) = self.buffer.cursor();
        if let Some((tbl_start, _tbl_end)) = self.wrap.table_block_for_line(cursor_row)
            && cursor_row != tbl_start
        {
            self.buffer.set_cursor(tbl_start, 0);
            return self.move_cursor_visual_down();
        }

        // 当前光标在表格首行：穿越整张表，跳到表格末行 + 1（如果存在）
        if let Some((tbl_start, tbl_end)) = self.wrap.table_block_for_line(cursor_row)
            && cursor_row == tbl_start
        {
            let after = tbl_end + 1;
            if after < self.buffer.line_count() {
                self.buffer.set_cursor(after, 0);
                return;
            }
            // 表格在 EOF：cursor 不动，但视口往下推一格，让用户能看到表格底部。
            // `scroll_locked = true` 防止下一帧光标同步把视口拉回去。
            let max_offset = total_visual.saturating_sub(1);
            if self.viewport.scroll_offset < max_offset {
                self.viewport.scroll_offset += 1;
                self.viewport.scroll_locked = true;
            }
            return;
        }

        if current_visual >= total_visual.saturating_sub(1) {
            return;
        }
        let target_visual = current_visual + 1;

        // 目标视觉行如果落在某个表格块的"膨胀区"，把光标放到该表格末行 + 1
        // （或者 EOF 时停在表格首行，由下一次 j 触发上面的"表格首行"分支）。
        if let Some((tbl_start, tbl_end)) = self.wrap.table_block_for_visual_row(target_visual) {
            let after = tbl_end + 1;
            if after < self.buffer.line_count() {
                self.buffer.set_cursor(after, 0);
            } else {
                self.buffer.set_cursor(tbl_start, 0);
            }
            return;
        }

        let (current_row, current_col) = self.buffer.cursor();

        // 确保目标行的缓存已构建
        let (target_logical, _) = self.wrap.visual_to_logical(target_visual);
        self.wrap
            .build_range(self.buffer.lines(), target_logical, target_logical + 1);

        if let Some(target_vl) = self.wrap.get_visual_line(target_visual) {
            let logical_line = target_vl.logical_line;
            let end_col = target_vl.end_col;
            let start_col = target_vl.start_col;

            // 保持视觉列位置：计算当前光标在当前视觉行中的屏幕偏移
            let current_vl = self.wrap.get_visual_line(current_visual);
            let current_start_col = current_vl.map(|vl| vl.start_col).unwrap_or(0);
            let current_line_text = self.buffer.line(current_row).map_or("", |v| v);
            let visual_x: usize = current_line_text
                .chars()
                .skip(current_start_col)
                .take(current_col.saturating_sub(current_start_col))
                .map(char_width)
                .sum();

            // 在目标视觉行中找到最接近该视觉 X 的逻辑列
            let target_line_text = self.buffer.line(logical_line).map_or("", |v| v);
            let new_col = if target_line_text.is_empty() {
                0
            } else {
                let segment: String = target_line_text.chars().skip(start_col).collect();
                Self::screen_col_to_char_offset(&segment, visual_x) + start_col
            };
            let new_col = new_col.min(end_col);
            self.buffer.set_cursor(logical_line, new_col);
        }
    }

    // ========== 输入处理 ==========

    /// 处理输入
    pub fn handle_input(&mut self, input: &Input) -> EditorAction {
        // 键盘输入解除滚动锁定
        self.viewport.scroll_locked = false;

        // 清除状态消息
        self.status_message = None;

        // 鼠标选区存在时优先拦截：
        //   - y / c：从渲染 spans 提取可见文本复制（表格/代码块/链接都准）
        //   - Esc：清空选区
        //   - 其他按键：清空选区，然后正常往下走（让用户继续编辑）
        if self.mouse_selection.is_some() {
            let is_yank = self.vim.mode() == &Mode::Normal
                && (input.key == Key::Char('y') || input.key == Key::Char('c'))
                && !input.ctrl;
            let is_esc = input.key == Key::Esc;

            if is_yank {
                self.copy_mouse_selection_to_clipboard();
                self.mouse_selection = None;
                return EditorAction::Continue;
            }
            if is_esc {
                self.mouse_selection = None;
                return EditorAction::Continue;
            }
            // 其他按键：选区取消，继续走原路径
            self.mouse_selection = None;
        }

        // 帮助弹窗模式：拦截所有按键
        if self.vim.mode() == &Mode::HelpPopup {
            return self.handle_help_popup(input);
        }

        // 主题选择模式：拦截所有按键
        if self.vim.mode() == &Mode::ThemeSelect {
            return self.handle_theme_select(input);
        }

        // 处理撤销
        if self.vim.mode() == &Mode::Normal && input.key == Key::Char('u') && !input.ctrl {
            self.undo();
            return EditorAction::Continue;
        }

        // 处理重做
        if self.vim.mode() == &Mode::Normal && input.key == Key::Char('r') && input.ctrl {
            self.redo();
            return EditorAction::Continue;
        }

        // 处理搜索跳转
        if self.vim.mode() == &Mode::Normal && self.search.is_searching() {
            if input.key == Key::Char('n') && !input.ctrl {
                self.search_next();
                return EditorAction::Continue;
            }
            if input.key == Key::Char('N') && !input.ctrl {
                self.search_prev();
                return EditorAction::Continue;
            }
            // Enter 跳到下一个匹配（直观一致）
            if input.key == Key::Enter && !input.ctrl {
                self.search_next();
                return EditorAction::Continue;
            }
            // Esc 清除搜索高亮
            if input.key == Key::Esc && !input.ctrl {
                self.search.clear();
                return EditorAction::Continue;
            }
        }

        // 命令面板模式：拦截上下键和回车键
        // 先克隆 filter 以释放 self.vim 的借用，避免后续调用 execute_command 时的借用冲突
        //
        // 同时处理两种面板：
        //  - CommandPanel：Normal 模式触发，命令列表 = COMMANDS
        //  - InsertCommandPanel：Insert 模式触发，命令列表 = INSERT_COMMANDS
        {
            #[derive(Clone, Copy)]
            enum PanelKind {
                Normal,
                Insert,
            }
            let panel_state: Option<(PanelKind, String)> = match self.vim.mode() {
                Mode::CommandPanel(f) => Some((PanelKind::Normal, f.clone())),
                Mode::InsertCommandPanel(f) => Some((PanelKind::Insert, f.clone())),
                _ => None,
            };
            if let Some((kind, filter)) = panel_state {
                let filtered: Vec<&'static CmdItem> = match kind {
                    PanelKind::Normal => filter_commands(&filter),
                    PanelKind::Insert => filter_insert_commands(&filter),
                };
                match input.key {
                    Key::Up => {
                        if !filtered.is_empty() {
                            if self.cmd_popup_selected > 0 {
                                self.cmd_popup_selected -= 1;
                            } else {
                                self.cmd_popup_selected = filtered.len() - 1;
                            }
                        }
                        return EditorAction::Continue;
                    }
                    Key::Down => {
                        if !filtered.is_empty() {
                            if self.cmd_popup_selected < filtered.len() - 1 {
                                self.cmd_popup_selected += 1;
                            } else {
                                self.cmd_popup_selected = 0;
                            }
                        }
                        return EditorAction::Continue;
                    }
                    Key::Enter => {
                        let selected = self
                            .cmd_popup_selected
                            .min(filtered.len().saturating_sub(1));
                        if let Some(cmd) = filtered.get(selected) {
                            match kind {
                                PanelKind::Normal => {
                                    let full_cmd = if cmd.name == "jump" {
                                        filter
                                    } else {
                                        cmd.name.to_string()
                                    };
                                    return self.execute_command(&full_cmd);
                                }
                                PanelKind::Insert => {
                                    return self.execute_insert_command(cmd.name, &filter);
                                }
                            }
                        }
                        // 没有匹配项：恢复到对应的来源模式
                        match kind {
                            PanelKind::Normal => self.vim.set_mode(Mode::Normal),
                            PanelKind::Insert => {
                                self.vim.set_mode(Mode::Insert);
                                self.insert_panel_anchor = None;
                            }
                        }
                        return EditorAction::Continue;
                    }
                    Key::Esc => {
                        // Insert 面板：保留已插入的 / 与 filter 文本，回到 Insert
                        // Normal 面板：保持原有行为，由 vim 状态机处理（fallthrough）
                        if matches!(kind, PanelKind::Insert) {
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            return EditorAction::Continue;
                        }
                    }
                    _ => {} // 其他按键交由后续 handle_mode_input 处理
                }
            }
        }

        // 折行感知的上下移动
        // j/k 只在 Normal 模式拦截，方向键在所有模式拦截
        if self.wrap.is_enabled() {
            let is_normal = self.vim.mode() == &Mode::Normal;
            let is_down = matches!(input.key, Key::Down)
                || (is_normal && matches!(input.key, Key::Char('j')));
            let is_up =
                matches!(input.key, Key::Up) || (is_normal && matches!(input.key, Key::Char('k')));

            if is_down && !input.ctrl {
                self.move_cursor_visual_down();
                return EditorAction::Continue;
            }
            if is_up && !input.ctrl {
                self.move_cursor_visual_up();
                return EditorAction::Continue;
            }
        }

        // Vim 状态机处理
        let old_mode = self.vim.mode().clone();
        let transition = self.vim.handle_input(input, &mut self.buffer);

        match transition {
            Transition::Mode(new_mode) => {
                // 如果从 Insert 模式退出，保存 undo 点
                if old_mode == Mode::Insert && new_mode != Mode::Insert {
                    self.vim
                        .push_snapshot(Snapshot::new(self.buffer.snapshot()), self.buffer.cursor());
                }
                // 从 Search 模式退出时跳转到当前匹配结果
                if matches!(old_mode, Mode::Search(_))
                    && new_mode == Mode::Normal
                    && let Some(m) = self.search.current_match()
                {
                    self.buffer.set_cursor(m.line, m.start);
                }
                if matches!(old_mode, Mode::Search(_)) {
                    self.cursor_before_search = None;
                }
                // 进入 Insert 命令面板时记录触发的 `/` 字符位置（光标已前进 1 位）
                if old_mode == Mode::Insert && matches!(new_mode, Mode::InsertCommandPanel(_)) {
                    let (row, col) = self.buffer.cursor();
                    self.insert_panel_anchor = Some((row, col.saturating_sub(1)));
                }
                self.vim.set_mode(new_mode);
                self.rebuild_wrap_cache();
            }
            Transition::Submit => {
                return EditorAction::Submit(self.buffer.to_string());
            }
            Transition::Save => {
                return EditorAction::Save(self.buffer.to_string());
            }
            Transition::Cancel => {
                return EditorAction::Cancel;
            }
            Transition::SearchAbort => {
                // Esc 取消搜索：恢复光标到搜索前位置，清除搜索高亮
                if let Some(pos) = self.cursor_before_search.take() {
                    self.buffer.set_cursor(pos.0, pos.1);
                }
                self.search.clear();
                self.vim.set_mode(Mode::Normal);
            }
            Transition::Nop => {
                // 处理 Command/Search 模式的字符输入
                self.handle_mode_input(input);
            }
            Transition::NeedRebuild => {
                // Normal 模式下的破坏性操作（dd/x/dw/d$）需要 undo 点
                if old_mode == Mode::Normal {
                    self.vim
                        .push_snapshot(Snapshot::new(self.buffer.snapshot()), self.buffer.cursor());
                }
                self.rebuild_wrap_cache();
            }
            Transition::ToggleWrap(enabled) => {
                self.wrap.set_enabled(enabled);
                self.rebuild_wrap_cache();
            }
            Transition::ExecuteCommand(cmd) => {
                return self.execute_command(&cmd);
            }
            Transition::ClipboardCopy => {
                if let Some(text) = self.vim.get_selection_text(&self.buffer) {
                    self.vim.set_yank_register(&text);
                    let _ = self.copy_to_clipboard(&text);
                }
                self.vim.set_mode(Mode::Normal);
                self.rebuild_wrap_cache();
            }
        }

        EditorAction::Continue
    }

    /// 处理模式特定的输入
    fn handle_mode_input(&mut self, input: &Input) {
        match self.vim.mode() {
            Mode::Command(cmd) => {
                let mut cmd = cmd.clone();
                match &input.key {
                    Key::Char(c) => cmd.push(*c),
                    Key::Backspace => {
                        cmd.pop();
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::Command(cmd));
            }
            Mode::Search(pattern) => {
                let mut pattern = pattern.clone();
                match &input.key {
                    Key::Char(c)
                        // 过滤控制字符（如 Esc 产生的 \x1b）
                        if !c.is_control() => {
                        pattern.push(*c);
                        self.search.search(&pattern, self.buffer.lines());
                    }
                    Key::Backspace => {
                        pattern.pop();
                        self.search.search(&pattern, self.buffer.lines());
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::Search(pattern));
            }
            Mode::CommandPanel(filter) => {
                let mut filter = filter.clone();
                match &input.key {
                    Key::Char(c) => {
                        filter.push(*c);
                        self.cmd_popup_selected = 0;
                    }
                    Key::Backspace => {
                        if !filter.is_empty() {
                            filter.pop();
                            self.cmd_popup_selected = 0;
                        } else {
                            self.vim.set_mode(Mode::Normal);
                            return;
                        }
                    }
                    _ => {} // 忽略其他按键（如功能键、组合键等）
                }
                self.vim.set_mode(Mode::CommandPanel(filter));
            }
            Mode::InsertCommandPanel(filter) => {
                // Insert 模式专用面板：触发 `/` 已经写入 buffer。
                // 这里继续把字符同步插入 buffer + filter，让用户的真实输入和面板状态一致。
                let mut filter = filter.clone();
                match &input.key {
                    Key::Char(c) => {
                        // `/` 字符已经走 vim::handle_insert_mode 的分支被插入；
                        // 这里只处理后续字符。
                        self.buffer.insert_char(*c);
                        filter.push(*c);
                        self.cmd_popup_selected = 0;

                        // 如果新输入后没有任何匹配项，自动关闭面板回到 Insert
                        // （让用户能够正常打 `https://` 之类的真实文本）
                        if filter_insert_commands(&filter).is_empty() {
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            self.rebuild_wrap_cache();
                            return;
                        }
                        self.rebuild_wrap_cache();
                    }
                    Key::Backspace => {
                        if !filter.is_empty() {
                            // 同步从 buffer 删除最后一个 filter 字符
                            self.buffer.backspace();
                            filter.pop();
                            self.cmd_popup_selected = 0;
                            self.rebuild_wrap_cache();
                        } else {
                            // filter 为空 → 删除触发的 `/`，回到 Insert
                            self.buffer.backspace();
                            self.vim.set_mode(Mode::Insert);
                            self.insert_panel_anchor = None;
                            self.rebuild_wrap_cache();
                            return;
                        }
                    }
                    _ => {}
                }
                self.vim.set_mode(Mode::InsertCommandPanel(filter));
            }
            _ => {}
        }
    }

    /// 撤销
    pub fn undo(&mut self) {
        if let Some(snap) = self.vim.undo() {
            self.buffer.replace_lines(snap.lines.clone());
            self.buffer.set_cursor(snap.cursor.0, snap.cursor.1);
            self.rebuild_wrap_cache();
        }
    }

    /// 重做
    pub fn redo(&mut self) {
        if let Some(snap) = self.vim.redo() {
            self.buffer.replace_lines(snap.lines.clone());
            self.buffer.set_cursor(snap.cursor.0, snap.cursor.1);
            self.rebuild_wrap_cache();
        }
    }

    /// 搜索下一个匹配
    pub fn search_next(&mut self) {
        self.search.next_match();
        if let Some(m) = self.search.current_match() {
            self.buffer.set_cursor(m.line, m.start);
        }
    }

    /// 搜索上一个匹配
    pub fn search_prev(&mut self) {
        self.search.prev_match();
        if let Some(m) = self.search.current_match() {
            self.buffer.set_cursor(m.line, m.start);
        }
    }

    /// 重建折行缓存
    fn rebuild_wrap_cache(&mut self) {
        // 先确定 wrap_width，再据此构建块级缓存（parser 需要 width）。
        // `wrap_width` 用上一帧 render() 缓存的 viewport.width 推算（首帧 viewport.width
        // 为默认 80，差几个像素无关紧要——第二帧 set_width 不同会触发 `dirty`，自动重建）。
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };
        let wrap_width = self.viewport.width.saturating_sub(line_num_width).max(10);

        // 块级缓存（fenced 代码块、表格等）按当前 wrap_width 重建
        self.renderer
            .ensure_cache_valid(self.buffer.lines(), wrap_width);
        let cb_ranges = self.renderer.code_block_content_ranges();

        // 计算表格渲染高度，灌进 wrap_engine
        let table_blocks = self
            .renderer
            .compute_table_block_heights(self.buffer.lines(), wrap_width);

        // 计算每行的"渲染后 per-char 显示宽度"数组。
        // 只有 **Insert 模式且 cursor 在该行** 时该行才显示源码源码（参见
        // `renderer.rs:render_visual_line` 的 `is_cursor_line && is_insert_mode`
        // 分支）；Normal 模式下光标行也走渲染路径，所以这种情况不该把光标行
        // 切到"按源码宽"，否则会出现光标在 Normal 模式下来回移动也触发 rebuild
        // 而且光标行折行点跳变的诡异表现。
        let cursor_line = if self.vim.mode() == &Mode::Insert {
            Some(self.buffer.cursor().0)
        } else {
            None
        };
        let visible_widths = self
            .renderer
            .compute_line_visible_widths(self.buffer.lines(), cursor_line);

        self.wrap.rebuild_cache_with_blocks_and_widths(
            self.buffer.lines(),
            &cb_ranges,
            &table_blocks,
            &visible_widths,
        );
        // 同时使渲染器缓存失效（语法高亮等）
        self.renderer.invalidate_cache();
        // 记录本次 rebuild 时的"按源码宽路径的光标行"，用于下次 render 判断
        // 是否需要再 rebuild。Normal 模式下该值为 None（光标行没有特殊路径）。
        self.last_wrap_cursor_line = cursor_line;
    }

    /// 如果"按源码宽路径"的光标行变化了，让 wrap engine 失效。
    ///
    /// 折行宽度只有在 **Insert 模式下光标所在行** 才走源码宽路径（其它都走
    /// 渲染宽）。所以：
    ///   - Insert 模式光标换行 → 需要 rebuild（旧光标行回到渲染宽、新光标行
    ///     切到源码宽）；
    ///   - 模式切换（Insert ↔ Normal）→ 也需要 rebuild（光标那一行宽度规则切换）；
    ///   - Normal 模式下光标在不同逻辑行之间移动 → **不需要** rebuild，所有行
    ///     都按渲染宽算。
    ///
    /// 每帧 render 入口处调用一次。
    pub(crate) fn maybe_mark_wrap_dirty_for_cursor(&mut self) {
        let current = if self.vim.mode() == &Mode::Insert {
            Some(self.buffer.cursor().0)
        } else {
            None
        };
        if self.last_wrap_cursor_line != current {
            self.wrap.mark_dirty();
        }
    }

    /// 计算编辑区可用的内容行数。
    ///
    /// 必须与 `render()` 中的 `content_height` 计算保持一致：
    ///  - 顶部 block border 1 行
    ///  - 底部状态栏 1 行（直接覆盖 block 底边框）
    ///  - 命令栏可见时再多占 1 行
    ///
    /// 之前误写为 `area.height - 3`，导致内容区底部少了一行可显示位置，
    /// 表现为代码块/普通文本最末一行被替换成 `~` 占位符。
    fn viewport_content_height(&self, area: Rect) -> usize {
        let has_cmd_bar = matches!(
            self.vim.mode(),
            Mode::Command(_) | Mode::Search(_) | Mode::CommandPanel(_)
        );
        // 顶部留白（边框或空行）+ 状态栏 1 行 + 命令栏（可选）
        let reserved: u16 = self.top_pad() + 1 + if has_cmd_bar { 1 } else { 0 };
        area.height.saturating_sub(reserved) as usize
    }

    /// 单侧水平边框宽度（左 / 右各占 1 列），关掉边框时为 0。
    fn border_pad(&self) -> u16 {
        if self.show_border { 1 } else { 0 }
    }

    /// 内容区与 area 顶部之间的留白行数。
    ///
    /// - `show_border = true` 时为 1（顶边框占的那一行）；
    /// - `show_border = false` 时仍为 1，作为视觉呼吸空间，避免内容紧贴上方面板。
    fn top_pad(&self) -> u16 {
        1
    }

    // ========== 鼠标操作 ==========

    /// 将屏幕坐标转换为逻辑位置 (logical_row, logical_col)。
    ///
    /// 返回 `None` 表示点击在内容区域之外（边框、状态栏等）。
    fn screen_to_logical(
        &self,
        screen_x: u16,
        screen_y: u16,
        area: Rect,
    ) -> Option<(usize, usize)> {
        // 减去边框偏移，得到内容区域内的坐标
        let h_pad = self.border_pad();
        let v_pad = self.top_pad();
        let content_x = screen_x.saturating_sub(area.x + h_pad) as usize; // 左边框（可能为 0）
        let content_y = screen_y.saturating_sub(area.y + v_pad) as usize; // 顶部留白

        let content_height = self.viewport_content_height(area);
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };

        // 超出内容区域
        if content_y >= content_height {
            return None;
        }

        // 使用渲染行元数据映射，将屏幕行号转换为渲染行索引
        let rendered_row = content_y + self.render_meta.map_index;

        // 如果点击位置超出实际渲染内容（落在空白填充 `~` 行），
        // 则把光标移动到最后一行的末尾（符合常见编辑器行为）
        let vl_meta = match self.render_meta.vl_map.get(rendered_row) {
            Some(meta) => meta,
            None => {
                // 没有内容可点击，定位到最后一个有效渲染行
                let last_meta = self.render_meta.vl_map.last()?;
                let line_text = self.buffer.line(last_meta.logical_line)?;
                let max_col = line_text.chars().count();
                return Some((last_meta.logical_line, max_col));
            }
        };

        let logical_row = vl_meta.logical_line;
        let vl_start_col = vl_meta.start_col;

        // 减去行号区域得到内容列
        let content_col_pre_deco = content_x.saturating_sub(line_num_width);

        // 减去渲染装饰列：代码块内容行渲染为 `│ <code> │`，左侧装饰占 2 列；
        // 不补这个偏移，鼠标点击映射到的源码列就比真实位置后移 2 个字符
        // （症状：选 `vscode` 复制成 `code .`）。
        let deco_left_cols = self.row_left_decoration_cols(logical_row);
        let content_col = content_col_pre_deco.saturating_sub(deco_left_cols);

        // 获取该逻辑行的原始文本
        let line_text = self.buffer.line(logical_row)?;

        // 获取该视觉行实际渲染的文本段（从 start_col 开始的子串）
        let vl_text: String = line_text.chars().skip(vl_start_col).collect();

        // 将屏幕列转换为字符偏移（考虑宽字符）
        let logical_col = Self::screen_col_to_char_offset(&vl_text, content_col) + vl_start_col;

        // 限制到行尾
        let max_col = line_text.chars().count();
        let logical_col = logical_col.min(max_col);

        Some((logical_row, logical_col))
    }

    /// 给定源码逻辑行号，返回其渲染时左侧装饰占用的屏幕列数（除行号外）。
    ///
    /// 当前覆盖：
    /// - 代码块内容行（含围栏行内部）：`│ ` 占 2 列
    /// - 其他行：0
    ///
    /// 用于把鼠标点击的"屏幕列"还原成"源码字符列"。围栏行视觉上是
    /// `╭───<lang>───╮`（顶/底边框），用户基本不会在围栏行点选文字；
    /// 这里保守返回 0，落点是行首（行尾），不会出现偏移导致的复制错误。
    fn row_left_decoration_cols(&self, logical_row: usize) -> usize {
        if self.wrap.is_code_block_line(logical_row) {
            2 // `│ `
        } else {
            0
        }
    }

    /// 把屏幕坐标转换为"渲染坐标"`(rendered_row_global, char_in_row)`：
    /// - `rendered_row_global` 是**全局渲染行号**（= `rendered_offset + 局部下标`）。
    /// - `char_in_row` 是该渲染行内的字符偏移（含装饰字符）。
    ///
    /// 这是鼠标拖选 / 复制走的坐标系——与 chat 的 `screen_to_text_pos` 同源。
    /// 失败时返回 `None`：点击在编辑区外、上一帧未渲染过、对应渲染行不存在。
    ///
    /// ⚠️ 坐标系易错点（详见 `BUGS_TROUBLESHOOTING.md` —— 鼠标选区高亮位置偏移）：
    ///   `render_meta.map_index` 是**局部下标**（不是全局行号），
    ///   `render_meta.rendered_offset` 是当帧 `visual_offset`。
    ///   全局行号 = `rendered_offset + map_index + content_y`。
    ///   render 端高亮循环用 `gline = visual_offset + idx` 比对 selection 的 sr/er，
    ///   所以本函数**必须返回全局行号**——少加 rendered_offset 会让滚动后选区
    ///   被高亮循环整段过滤掉，表现为"高亮位置和鼠标偏移 / 底部拖动看不到高亮"。
    fn screen_to_render_pos(
        &self,
        screen_x: u16,
        screen_y: u16,
        area: Rect,
    ) -> Option<(usize, usize)> {
        use crate::tui::components::selection::spans_to_char_offset;

        // area 内左上角是上边框 + 左边框（关闭边框时左/右为 0，但顶部仍留 1 行呼吸空间）
        let h_pad = self.border_pad();
        let v_pad = self.top_pad();
        let content_y = screen_y.saturating_sub(area.y + v_pad) as usize;
        let content_height = self.viewport_content_height(area);
        if content_y >= content_height {
            return None;
        }

        // map_index 是当前屏幕顶行对应的 `rendered_lines` 局部下标
        // （= visible_start_local，render() 末尾写入）。要还原成"全局渲染行号"，
        // 必须再加上 rendered_offset (= visual_offset)。
        // 之前漏加 rendered_offset，导致滚动后 (visual_offset > 0) 选区的行号
        // 比真实值小 visual_offset：render 端 highlight 循环用 `gline = visual_offset + idx`
        // 比对，整段选区都被 `gline > er` 过滤掉 → 看不到高亮；底部拖动尤其明显。
        let local_idx = self.render_meta.map_index + content_y;
        let line = self.render_meta.rendered_lines.get(local_idx)?;
        let global_row = self.render_meta.rendered_offset + local_idx;

        // 屏幕 X：按显示宽度匹配 spans 拼接后的字符偏移。spans 已经包含
        // 行号 / 边框 / padding 等装饰，所以 char_offset 直接是"渲染行内"
        // 的真实偏移，复制时由 extract_selection_text 自动跳过装饰部分。
        let local_x = screen_x.saturating_sub(area.x + h_pad) as usize;
        let char_offset = spans_to_char_offset(&line.spans, local_x);
        Some((global_row, char_offset))
    }

    /// 拖选 fallback：当鼠标超出 editor area 时，把它"夹"到最近的有效渲染行
    /// 边缘并返回渲染坐标，用于持续更新 `mouse_selection.current`。
    ///
    /// 规则：
    /// - 鼠标在 area 上方 → 选到第一个可见渲染行的行首；
    /// - 鼠标在 area 下方 → 选到最后一个可见渲染行的行末；
    /// - X 越界则 clamp 到 area 水平边界后再走标准换算。
    fn clamped_render_pos_for_drag(&self, mouse: MouseEvent, area: Rect) -> Option<(usize, usize)> {
        if area.width == 0 || area.height == 0 {
            return None;
        }
        let max_col = area.x + area.width - 1;
        let max_row = area.y + area.height - 1;

        let v_pad = self.top_pad();
        let content_height = self.viewport_content_height(area);
        if content_height == 0 {
            return None;
        }

        // 决定 fallback 到顶部还是底部：高于 area 走顶行，否则走底行。
        let to_bottom = mouse.row > max_row || mouse.row >= area.y + v_pad + content_height as u16;
        let visible_row = if to_bottom {
            (content_height - 1) as u16
        } else if mouse.row < area.y + v_pad {
            0
        } else {
            mouse.row.saturating_sub(area.y + v_pad)
        };

        // 见 `screen_to_render_pos` 头部的坐标系警示：返回值必须是**全局行号**，
        // 等于 `rendered_offset + 局部下标`。
        let local_idx = visible_row as usize + self.render_meta.map_index;
        let line = self.render_meta.rendered_lines.get(local_idx)?;
        let global_row = self.render_meta.rendered_offset + local_idx;

        // X 方向：超出底部时直接选到行末（包含整行内容）；否则按 clamp 后的列计算。
        if to_bottom {
            let total_chars: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            return Some((global_row, total_chars));
        }
        let clamped_x = mouse.column.clamp(area.x, max_col);
        let h_pad = self.border_pad();
        let local_x = clamped_x.saturating_sub(area.x + h_pad) as usize;
        use crate::tui::components::selection::spans_to_char_offset;
        Some((global_row, spans_to_char_offset(&line.spans, local_x)))
    }

    /// 将屏幕列号转换为字符偏移（考虑 CJK 等宽字符）。
    fn screen_col_to_char_offset(text: &str, screen_col: usize) -> usize {
        use crate::util::text::char_width;

        let mut acc_width = 0;
        for (i, ch) in text.chars().enumerate() {
            if acc_width >= screen_col {
                return i;
            }
            acc_width += char_width(ch);
        }
        text.chars().count()
    }

    /// 处理鼠标事件。
    pub fn handle_mouse(&mut self, mouse: MouseEvent, area: Rect) {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                // 任何点击都先清空旧的鼠标选区
                self.mouse_selection = None;
                self.mouse_render_anchor = None;

                if let Some((row, col)) = self.screen_to_logical(mouse.column, mouse.row, area) {
                    // 点击有效区域：移动光标，解除滚动锁定让视口跟随
                    self.viewport.scroll_locked = false;
                    self.vim.set_mode(Mode::Normal);
                    self.buffer.set_cursor(row, col);
                    self.mouse_anchor = Some((row, col));
                    // 记录渲染坐标的锚点；只有真正拖动后才会进入 mouse_selection
                    self.mouse_render_anchor =
                        self.screen_to_render_pos(mouse.column, mouse.row, area);
                } else {
                    // 点击空白区域（边框、状态栏等）：取消选区
                    self.viewport.scroll_locked = false;
                    self.vim.set_mode(Mode::Normal);
                    self.mouse_anchor = None;
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                self.viewport.scroll_locked = false;

                // 同步光标到屏幕坐标对应的源码位置（让视口跟随、便于后续编辑）
                if let Some((row, col)) = self.screen_to_logical(mouse.column, mouse.row, area) {
                    self.buffer.set_cursor(row, col);
                }

                // 鼠标选区走渲染坐标——表格 / 代码块 / 链接里拖选 + 复制都准确。
                // 鼠标拖出 area 是常态（向下/向上"甩"），此时 `screen_to_render_pos`
                // 会返回 None；fallback 到最后一个/第一个有效渲染行末尾/首部，
                // 让 mouse_selection.current 持续更新，下半段也会有高亮。
                if let Some(anchor_render) = self.mouse_render_anchor {
                    let current_render = self
                        .screen_to_render_pos(mouse.column, mouse.row, area)
                        .or_else(|| self.clamped_render_pos_for_drag(mouse, area));
                    if let Some(current_render) = current_render {
                        self.mouse_selection = Some(MouseSelection {
                            anchor: anchor_render,
                            current: current_render,
                        });
                    }
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                self.mouse_anchor = None;
                // 单击未拖动：选区起止相同 → 清掉，避免空选区扰民
                if let Some(sel) = self.mouse_selection
                    && sel.anchor == sel.current
                {
                    self.mouse_selection = None;
                }
                self.mouse_render_anchor = None;
            }
            MouseEventKind::ScrollUp => {
                // 鼠标滚轮按 step 视觉行驱动光标，再让 render 自动追到视口；
                // 这样无论文档是否满屏都能给出一致的"光标随滚动"反馈。
                self.viewport.scroll_locked = false;
                for _ in 0..3 {
                    self.move_cursor_visual_up();
                }
            }
            MouseEventKind::ScrollDown => {
                self.viewport.scroll_locked = false;
                for _ in 0..3 {
                    self.move_cursor_visual_down();
                }
            }
            _ => {}
        }
    }

    /// 复制文本到系统剪贴板
    fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        use arboard::Clipboard;
        let mut clipboard = Clipboard::new().map_err(|e| format!("无法访问剪贴板: {e}"))?;
        clipboard
            .set_text(text)
            .map_err(|e| format!("复制到剪贴板失败: {e}"))?;
        Ok(())
    }

    /// 把当前鼠标选区对应的可见文本复制到剪贴板。
    ///
    /// 用渲染 spans（`render_meta.rendered_lines`）提取，自动跳过边框 / padding /
    /// 行号等装饰，链接的 `[`、`]()` 也被跳过——复制下来就是屏幕上看到的字。
    fn copy_mouse_selection_to_clipboard(&mut self) {
        use crate::tui::components::selection::extract_selection_text;

        let Some(sel) = self.mouse_selection else {
            return;
        };
        // 把全局渲染行号换成 rendered_lines 的本地下标
        let offset = self.render_meta.rendered_offset;
        let local_anchor = (sel.anchor.0.saturating_sub(offset), sel.anchor.1);
        let local_current = (sel.current.0.saturating_sub(offset), sel.current.1);

        // 行号 gutter 占用的字符数（与 format_line_number 的 `{:>4}  ` 一致）
        let line_num_width = if self.renderer.is_show_line_numbers() {
            6
        } else {
            0
        };

        let text = extract_selection_text(
            &self.render_meta.rendered_lines,
            local_anchor,
            local_current,
            line_num_width,
        );
        if text.is_empty() {
            return;
        }
        self.vim.set_yank_register(&text);
        let _ = self.copy_to_clipboard(&text);
    }
}

/// 编辑器动作
#[derive(Debug)]
pub enum EditorAction {
    /// 继续编辑
    Continue,
    /// 提交内容（保存并退出）
    Submit(String),
    /// 保存内容但不退出
    Save(String),
    /// 取消编辑
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;
    use ratatui::text::Span;

    fn test_theme() -> EditorTheme {
        EditorTheme {
            bg_primary: Color::Reset,
            bg_input: Color::Reset,
            code_bg: Color::DarkGray,
            cursor_fg: Color::Black,
            cursor_bg: Color::Cyan,
            text_normal: Color::White,
            text_dim: Color::DarkGray,
            text_bold: Color::White,
            md_h1: Color::Cyan,
            md_h2: Color::Green,
            md_h3: Color::Yellow,
            md_h4: Color::Magenta,
            md_heading_sep: Color::DarkGray,
            md_link: Color::Blue,
            md_list_bullet: Color::Yellow,
            md_blockquote_bar: Color::Cyan,
            md_blockquote_bg: Color::DarkGray,
            md_blockquote_text: Color::Gray,
            md_inline_code_fg: Color::Magenta,
            md_inline_code_bg: Color::DarkGray,
            md_rule: Color::DarkGray,
            code_border: Color::DarkGray,
            code_border_style: BorderStyle::default(),
            table_header: Color::White,
            table_body: Color::White,
            code_default: Color::White,
            code_keyword: Color::Magenta,
            code_string: Color::Green,
            code_comment: Color::DarkGray,
            code_number: Color::Yellow,
            code_type: Color::Yellow,
            code_primitive: Color::Cyan,
            code_macro: Color::LightCyan,
            code_lifetime: Color::LightMagenta,
            code_attribute: Color::LightBlue,
            code_shell_var: Color::LightCyan,
            label_ai: Color::Green,
        }
    }

    fn noop_highlight(_: &str, _: &str, _: &EditorTheme) -> Vec<Span<'static>> {
        Vec::new()
    }

    #[test]
    fn scroll_down_moves_cursor_three_visual_lines() {
        let content = (0..20)
            .map(|idx| format!("line {idx}"))
            .collect::<Vec<_>>()
            .join("\n");
        let mut editor = MarkdownEditor::new(
            "test",
            &content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        let area = Rect::new(0, 0, 80, 10);

        editor.buffer.set_cursor(0, 0);
        editor.viewport.scroll_offset = 0;
        editor.viewport.scroll_locked = true;

        editor.handle_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 0,
                row: 0,
                modifiers: crossterm::event::KeyModifiers::empty(),
            },
            area,
        );

        // 滚轮下滚 → 光标按 3 视觉行下移；render() 自动追视口，因此解锁。
        assert_eq!(editor.buffer.cursor().0, 3);
        assert!(!editor.viewport.scroll_locked);
    }

    #[test]
    fn move_cursor_visual_down_skips_table_block_to_line_after() {
        // 文档：3 行普通文字 + 空行 + 4 行表格 + 1 行结尾。
        // GFM 表格要求与上文有空行分隔，否则 pulldown-cmark 会把首行并入段落。
        // 旧 `is_table_row` 用 `|...|` 启发式忽略了这个规则；新 BlockCache 严格按 CommonMark/GFM
        // 解析，所以测试数据补一个空行让表格被正确识别。
        let content = "para1\npara2\npara3\n\n| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |\nepilogue";
        let mut editor = MarkdownEditor::new(
            "test",
            content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        // 触发一次重建，让 wrap_engine 灌入表格高度（构造函数里的 rebuild_cache 不带表格信息）
        editor.viewport.width = 78;
        editor.rebuild_wrap_cache();

        // 光标定位到表格首行（line 4）
        editor.buffer.set_cursor(4, 0);
        assert_eq!(editor.buffer.cursor().0, 4);

        // 按一次 j：从表格首行应跳到表格末行 + 1 = line 8（epilogue）
        editor.move_cursor_visual_down();
        assert_eq!(
            editor.buffer.cursor().0,
            8,
            "光标在表格首行按 j 应跳过整张表到 line 8 (epilogue)"
        );
    }

    #[test]
    fn move_cursor_visual_down_at_eof_table_pushes_viewport() {
        // 文档：仅 1 行普通文字 + 4 行表格（表格在 EOF）
        let content = "header\n| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |";
        let mut editor = MarkdownEditor::new(
            "test",
            content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        editor.viewport.width = 78;
        editor.viewport.height = 6;
        editor.rebuild_wrap_cache();

        // 光标定位到表格首行（line 1）
        editor.buffer.set_cursor(1, 0);
        let initial_offset = editor.viewport.scroll_offset;

        // 在 EOF 表格首行按 j：光标不动（无后继行），但视口 scroll_offset 应推进 1 行，
        // 并且 scroll_locked 设为 true（避免下一帧把视口拉回光标位置）。
        editor.move_cursor_visual_down();
        assert_eq!(
            editor.buffer.cursor().0,
            1,
            "EOF 表格无后继行，光标应留在表格首行"
        );
        assert!(
            editor.viewport.scroll_offset > initial_offset,
            "scroll_offset 应被推进；现在 = {}, 初始 = {}",
            editor.viewport.scroll_offset,
            initial_offset
        );
        assert!(
            editor.viewport.scroll_locked,
            "推视口后必须 scroll_locked=true，否则下一帧光标同步会把它拉回去"
        );
    }

    /// Visual 选区复制测试：character-wise visual mode 两端均闭合，
    /// 屏幕上看到的高亮 + 光标块 = 复制到剪贴板的内容。
    #[test]
    fn visual_selection_copy_includes_cursor_char_single_line() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 模拟：光标在 (0, 0)，按 v 后再按 l 一次：visual_start=(0,0), cursor=(0,1)
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 1);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("he"),
            "v + l 应选中 'he'（cursor 块字符也算选中）"
        );
    }

    #[test]
    fn visual_selection_copy_inclusive_at_line_end() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 光标在 (0, 0)，按 v 后跳到行末 cursor=(0, 4) on 'o'
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 4);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("hello"),
            "v + 跳到 'o' 应选中 'hello'"
        );
    }

    #[test]
    fn visual_selection_copy_past_line_end_no_overshoot() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 光标越过行末（cursor=(0, 5) = line_len），不应在 'o' 后再多 +1 越界
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 5);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("hello"),
            "光标已在 line_len 处不再 +1，避免越界 panic 或多取字符"
        );
    }

    #[test]
    fn visual_selection_copy_multi_line_inclusive_end() {
        let mut editor = MarkdownEditor::new(
            "test",
            "hello\nworld",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 起点 (0,0)，cursor 在第二行第 2 个字符 (1, 2) on 'r'
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(1, 2);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("hello\nwor"),
            "多行选区结尾包含 cursor 字符 'r'"
        );
    }

    #[test]
    fn visual_selection_copy_chinese_inclusive() {
        let mut editor = MarkdownEditor::new(
            "test",
            "你好世界",
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );

        // 起点 (0,0)，cursor 在 '世' (col 2)
        editor.buffer.set_cursor(0, 0);
        editor.vim.set_visual_start((0, 0));
        editor.vim.set_mode(Mode::Visual);
        editor.buffer.set_cursor(0, 2);

        let copied = editor.vim.get_selection_text(&editor.buffer);
        assert_eq!(
            copied.as_deref(),
            Some("你好世"),
            "中文按字符切片，cursor 块所在字也算选中"
        );
    }

    /// 鼠标点击代码块内字符时，screen_col → logical_col 必须扣掉左侧
    /// 装饰列（`│ ` 占 2 列）。否则点 `vscode` 的 'v' 实际落到 'c'（偏移 +2），
    /// 后续选区复制就会变成错位的内容（例如 `code .`）。
    #[test]
    fn screen_to_logical_subtracts_code_block_decoration() {
        // 文档：3 行围栏 + 1 行内容（在代码块内）
        let content = "```bash\nset vscode \"/Applications/Visual Studio Code.app\"\n```";
        let mut editor = MarkdownEditor::new(
            "test",
            content,
            test_theme(),
            noop_highlight,
            Vec::new(),
            CursorPolicy::StartOfFile,
        );
        // 让 wrap_engine 重新加载 cb_ranges，code_block_lines 才能正确填充
        editor.viewport.width = 78;
        editor.rebuild_wrap_cache();

        // 行 1 是代码块内容行 → is_code_block_line == true
        assert!(
            editor.wrap.is_code_block_line(1),
            "行 1 应被识别为代码块内容行；当前 wrap.code_block_lines = {:?}",
            (0..3)
                .map(|i| editor.wrap.is_code_block_line(i))
                .collect::<Vec<_>>()
        );

        // 装饰列 = 2（`│ `）
        assert_eq!(editor.row_left_decoration_cols(1), 2);
        assert_eq!(editor.row_left_decoration_cols(0), 0); // 围栏行不计装饰
    }

    // ====== 鼠标拖选 + 复制（基于渲染 spans，跨表格 / 代码块 / 链接）======
    //
    // 这组测试直接喂给共享的 extract_selection_text 一组手工构造的 Line（spans
    // 已经包含装饰），验证选区复制的内容 = 屏幕上看到的可见文字。
    // 不走完整的 render → mouse → handle_input 链路，避免依赖 ratatui Frame。

    use crate::tui::components::selection::extract_selection_text;
    use ratatui::style::Color as RColor;
    use ratatui::style::Style as RStyle;
    use ratatui::text::Line as RLine;
    use ratatui::text::Span as RSpan;

    fn deco(content: &'static str) -> RSpan<'static> {
        // 装饰 span：要么纯空格，要么 box-drawing 字符——会被 is_decorative_span
        // 识别出来不算可见正文
        RSpan::styled(content.to_string(), RStyle::default())
    }
    fn body(content: &'static str) -> RSpan<'static> {
        // 正文 span：用一个非装饰前景色作为标记，避免被误判
        RSpan::styled(content.to_string(), RStyle::default().fg(RColor::White))
    }

    #[test]
    fn mouse_copy_code_block_skips_left_border_padding() {
        // 渲染行：`  42  │ set vscode "/path"  │`
        // 装饰：行号 6 列（`{:>4}  ` = 4 字符右对齐 + 2 空格）+ `│` 1 列 + ` ` 1 列；
        //       正文从字符偏移 8 开始。
        let line = RLine::from(vec![
            deco("  42  "), // line num: "{:>4}  " 模板对应 6 字符
            deco("│"),      // left bar
            deco(" "),      // pad
            body(r#"set vscode "/Applications/Visual Studio Code.app""#),
            deco(" "),
            deco("│"),
        ]);
        let lines = vec![line];

        // 用户拖选 vscode：正文偏移 4..10（'v' 在 4，'e' 末尾开区间在 10）。
        // 渲染坐标 = 8 + 正文偏移 = [12, 18)。
        let copied = extract_selection_text(&lines, (0, 12), (0, 18), 6);
        assert_eq!(
            copied, "vscode",
            "代码块拖选 vscode 应复制 vscode；之前的 bug 会复制成 'code .' 类的偏移结果"
        );
    }

    #[test]
    fn mouse_copy_table_cell_content() {
        // 渲染行：`  12  │ col1   │ col2   │`
        // 装饰：行号 6 + `│` 1 + ` ` 1 = 8 列在前；正文 col1 从偏移 8 起。
        let line = RLine::from(vec![
            deco("  12  "),
            deco("│"),
            deco(" "),
            body("col1"),
            deco("   "), // cell padding
            deco("│"),
            deco(" "),
            body("col2"),
            deco("   "),
            deco("│"),
        ]);
        let lines = vec![line];

        // 选 col1: 渲染偏移 [8, 12)
        let copied = extract_selection_text(&lines, (0, 8), (0, 12), 6);
        assert_eq!(copied, "col1", "表格里拖选单元格内容应复制单元格内容");

        // 跨 cell 选 col1 + col2:
        // col2 起 = 8 + 4 (col1) + 3 (pad) + 1 (│) + 1 (space) = 17
        // col2 末（4 个字符）= 21
        let copied = extract_selection_text(&lines, (0, 8), (0, 21), 6);
        // extract_content_from_line 把所有非装饰 span 拼起来 = "col1col2"
        // （中间 cell padding 和 `│` 被 is_decorative_span 识别丢弃）
        assert_eq!(
            copied, "col1col2",
            "跨表格 cell 拖选只保留可见正文 span，丢弃边框和 padding"
        );
    }

    #[test]
    fn mouse_copy_link_skips_brackets() {
        // 链接 [Click here](https://example.com) 渲染时只有 "Click here" 可见，
        // `[`、`](url)` 不进入渲染 spans。再叠加普通行号装饰。
        let line = RLine::from(vec![
            deco("   1  "), // line num: "   1  " 6 chars
            body("Click here"),
        ]);
        let lines = vec![line];

        // 拖选整个 "Click here"：行号 6 列 → 字符偏移 [6, 16)
        let copied = extract_selection_text(&lines, (0, 6), (0, 16), 6);
        assert_eq!(
            copied, "Click here",
            "链接拖选应复制可见文本，而不是源码 [Click here](url)"
        );

        // 拖选部分（"here"）：偏移 [12, 16)
        let copied = extract_selection_text(&lines, (0, 12), (0, 16), 6);
        assert_eq!(copied, "here");
    }

    #[test]
    fn mouse_copy_multi_line_joins_with_newline() {
        // 两行连续拖选
        let lines = vec![
            RLine::from(vec![deco("   1  "), body("first line")]),
            RLine::from(vec![deco("   2  "), body("second line")]),
        ];
        // anchor (0, 6) = 'f'，current (1, 17) = 第二行末尾
        // 第二行：行号 6 + "second line" 11 = 17
        let copied = extract_selection_text(&lines, (0, 6), (1, 17), 6);
        assert_eq!(
            copied, "first line\nsecond line",
            "多行拖选用 \\n 拼接，每行按可见正文提取"
        );
    }
}
