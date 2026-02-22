use crate::config::YamlConfig;
use crate::{error, info};
use chrono::Local;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

// ========== 数据结构 ==========

/// 单条待办事项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    /// 待办内容
    pub content: String,
    /// 是否已完成
    pub done: bool,
    /// 创建时间
    pub created_at: String,
    /// 完成时间（可选）
    pub done_at: Option<String>,
}

/// 待办列表（序列化到 JSON）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TodoList {
    pub items: Vec<TodoItem>,
}

// ========== 文件路径 ==========

/// 获取 todo 数据目录: ~/.jdata/todo/
fn todo_dir() -> PathBuf {
    let dir = YamlConfig::data_dir().join("todo");
    let _ = fs::create_dir_all(&dir);
    dir
}

/// 获取 todo 数据文件路径: ~/.jdata/todo/todo.json
fn todo_file_path() -> PathBuf {
    todo_dir().join("todo.json")
}

// ========== 数据读写 ==========

/// 从文件加载待办列表
fn load_todo_list() -> TodoList {
    let path = todo_file_path();
    if !path.exists() {
        return TodoList::default();
    }
    match fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            error!("❌ 解析 todo.json 失败: {}", e);
            TodoList::default()
        }),
        Err(e) => {
            error!("❌ 读取 todo.json 失败: {}", e);
            TodoList::default()
        }
    }
}

/// 保存待办列表到文件
fn save_todo_list(list: &TodoList) -> bool {
    let path = todo_file_path();
    // 确保目录存在
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(list) {
        Ok(json) => match fs::write(&path, json) {
            Ok(_) => true,
            Err(e) => {
                error!("❌ 保存 todo.json 失败: {}", e);
                false
            }
        },
        Err(e) => {
            error!("❌ 序列化 todo 列表失败: {}", e);
            false
        }
    }
}

// ========== 命令入口 ==========

/// 处理 todo 命令: j todo [content...]
pub fn handle_todo(content: &[String], _config: &YamlConfig) {
    if content.is_empty() {
        // 无参数：进入 TUI 待办管理界面
        run_todo_tui();
        return;
    }

    // 有参数：快速添加待办
    let text = content.join(" ");
    let text = text.trim().trim_matches('"').to_string();

    if text.is_empty() {
        error!("⚠️ 内容为空，无法添加待办");
        return;
    }

    let mut list = load_todo_list();
    list.items.push(TodoItem {
        content: text.clone(),
        done: false,
        created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        done_at: None,
    });

    if save_todo_list(&list) {
        info!("✅ 已添加待办: {}", text);
        // 显示当前待办总数
        let undone = list.items.iter().filter(|i| !i.done).count();
        info!("📋 当前未完成待办: {} 条", undone);
    }
}

// ========== TUI 界面 ==========

/// TUI 应用状态
struct TodoApp {
    /// 待办列表数据
    list: TodoList,
    /// 列表选中状态
    state: ListState,
    /// 当前模式
    mode: AppMode,
    /// 输入缓冲区（添加/编辑模式使用）
    input: String,
    /// 编辑时记录的原始索引
    edit_index: Option<usize>,
    /// 是否有未保存的修改
    dirty: bool,
    /// 状态栏消息
    message: Option<String>,
    /// 过滤模式: 0=全部, 1=未完成, 2=已完成
    filter: usize,
}

#[derive(PartialEq)]
enum AppMode {
    /// 正常浏览模式
    Normal,
    /// 输入添加模式
    Adding,
    /// 编辑模式
    Editing,
    /// 确认删除
    ConfirmDelete,
}

impl TodoApp {
    fn new() -> Self {
        let list = load_todo_list();
        let mut state = ListState::default();
        if !list.items.is_empty() {
            state.select(Some(0));
        }
        Self {
            list,
            state,
            mode: AppMode::Normal,
            input: String::new(),
            edit_index: None,
            dirty: false,
            message: None,
            filter: 0,
        }
    }

    /// 获取当前过滤后的索引列表（映射到 list.items 的真实索引）
    fn filtered_indices(&self) -> Vec<usize> {
        self.list
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| match self.filter {
                1 => !item.done,
                2 => item.done,
                _ => true,
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// 获取当前选中项在原始列表中的真实索引
    fn selected_real_index(&self) -> Option<usize> {
        let indices = self.filtered_indices();
        self.state
            .selected()
            .and_then(|sel| indices.get(sel).copied())
    }

    /// 向下移动
    fn move_down(&mut self) {
        let count = self.filtered_indices().len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i >= count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// 向上移动
    fn move_up(&mut self) {
        let count = self.filtered_indices().len();
        if count == 0 {
            return;
        }
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    /// 切换当前选中项的完成状态
    fn toggle_done(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            let item = &mut self.list.items[real_idx];
            item.done = !item.done;
            if item.done {
                item.done_at = Some(Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
                self.message = Some("✅ 已标记为完成".to_string());
            } else {
                item.done_at = None;
                self.message = Some("⬜ 已标记为未完成".to_string());
            }
            self.dirty = true;
        }
    }

    /// 添加新待办
    fn add_item(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            self.message = Some("⚠️ 内容为空，已取消".to_string());
            self.mode = AppMode::Normal;
            self.input.clear();
            return;
        }
        self.list.items.push(TodoItem {
            content: text,
            done: false,
            created_at: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            done_at: None,
        });
        self.dirty = true;
        self.input.clear();
        self.mode = AppMode::Normal;
        // 选中新添加的项
        let count = self.filtered_indices().len();
        if count > 0 {
            self.state.select(Some(count - 1));
        }
        self.message = Some("✅ 已添加新待办".to_string());
    }

    /// 确认编辑
    fn confirm_edit(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            self.message = Some("⚠️ 内容为空，已取消编辑".to_string());
            self.mode = AppMode::Normal;
            self.input.clear();
            self.edit_index = None;
            return;
        }
        if let Some(idx) = self.edit_index {
            if idx < self.list.items.len() {
                self.list.items[idx].content = text;
                self.dirty = true;
                self.message = Some("✅ 已更新待办内容".to_string());
            }
        }
        self.input.clear();
        self.edit_index = None;
        self.mode = AppMode::Normal;
    }

    /// 删除当前选中项
    fn delete_selected(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            let removed = self.list.items.remove(real_idx);
            self.dirty = true;
            self.message = Some(format!("🗑️ 已删除: {}", removed.content));
            // 调整选中位置
            let count = self.filtered_indices().len();
            if count == 0 {
                self.state.select(None);
            } else if let Some(sel) = self.state.selected() {
                if sel >= count {
                    self.state.select(Some(count - 1));
                }
            }
        }
        self.mode = AppMode::Normal;
    }

    /// 移动选中项向上（调整顺序）
    fn move_item_up(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            if real_idx > 0 {
                self.list.items.swap(real_idx, real_idx - 1);
                self.dirty = true;
                self.move_up();
            }
        }
    }

    /// 移动选中项向下（调整顺序）
    fn move_item_down(&mut self) {
        if let Some(real_idx) = self.selected_real_index() {
            if real_idx < self.list.items.len() - 1 {
                self.list.items.swap(real_idx, real_idx + 1);
                self.dirty = true;
                self.move_down();
            }
        }
    }

    /// 切换过滤模式
    fn toggle_filter(&mut self) {
        self.filter = (self.filter + 1) % 3;
        let count = self.filtered_indices().len();
        if count > 0 {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
        let label = match self.filter {
            1 => "未完成",
            2 => "已完成",
            _ => "全部",
        };
        self.message = Some(format!("🔍 过滤: {}", label));
    }

    /// 保存数据
    fn save(&mut self) {
        if self.dirty {
            if save_todo_list(&self.list) {
                self.dirty = false;
                self.message = Some("💾 已保存".to_string());
            }
        }
    }
}

/// 启动 TUI 待办管理界面
fn run_todo_tui() {
    match run_todo_tui_internal() {
        Ok(_) => {}
        Err(e) => {
            error!("❌ TUI 启动失败: {}", e);
        }
    }
}

fn run_todo_tui_internal() -> io::Result<()> {
    // 进入终端原始模式
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TodoApp::new();

    loop {
        // 渲染界面
        terminal.draw(|f| draw_ui(f, &mut app))?;

        // 处理输入事件
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match app.mode {
                    AppMode::Normal => {
                        if handle_normal_mode(&mut app, key) {
                            break;
                        }
                    }
                    AppMode::Adding => handle_input_mode(&mut app, key),
                    AppMode::Editing => handle_input_mode(&mut app, key),
                    AppMode::ConfirmDelete => handle_confirm_delete(&mut app, key),
                }
            }
        }
    }

    // 退出前自动保存
    if app.dirty {
        save_todo_list(&app.list);
    }

    // 恢复终端
    terminal::disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}

/// 绘制 TUI 界面
fn draw_ui(f: &mut ratatui::Frame, app: &mut TodoApp) {
    let size = f.area();

    // 整体布局: 标题栏 + 列表区 + 状态栏 + 帮助栏
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // 标题栏
            Constraint::Min(5),    // 列表区
            Constraint::Length(3), // 状态/输入栏
            Constraint::Length(2), // 帮助栏
        ])
        .split(size);

    // ========== 标题栏 ==========
    let filter_label = match app.filter {
        1 => " [未完成]",
        2 => " [已完成]",
        _ => "",
    };
    let total = app.list.items.len();
    let done = app.list.items.iter().filter(|i| i.done).count();
    let undone = total - done;
    let title = format!(
        " 📋 待办备忘录{} — 共 {} 条 | ✅ {} | ⬜ {} ",
        filter_label, total, done, undone
    );
    let title_block = Paragraph::new(Line::from(vec![Span::styled(
        title,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(title_block, chunks[0]);

    // ========== 列表区 ==========
    let indices = app.filtered_indices();
    let items: Vec<ListItem> = indices
        .iter()
        .map(|&idx| {
            let item = &app.list.items[idx];
            let checkbox = if item.done { "[x]" } else { "[ ]" };
            let style = if item.done {
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::CROSSED_OUT)
            } else {
                Style::default().fg(Color::White)
            };

            let mut spans = vec![
                Span::styled(
                    format!(" {} ", checkbox),
                    if item.done {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Yellow)
                    },
                ),
                Span::styled(&item.content, style),
            ];

            // 显示创建时间（缩短格式）
            if let Some(short_date) = item.created_at.get(..10) {
                spans.push(Span::styled(
                    format!("  ({})", short_date),
                    Style::default().fg(Color::DarkGray),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White))
        .title(" 待办列表 ");

    if items.is_empty() {
        // 空列表提示
        let empty_hint = List::new(vec![ListItem::new(Line::from(Span::styled(
            "   (空) 按 a 添加新待办...",
            Style::default().fg(Color::DarkGray),
        )))])
        .block(list_block);
        f.render_widget(empty_hint, chunks[1]);
    } else {
        let list_widget = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");
        f.render_stateful_widget(list_widget, chunks[1], &mut app.state);
    };

    // ========== 状态/输入栏 ==========
    match &app.mode {
        AppMode::Adding => {
            let input_widget = Paragraph::new(Line::from(vec![
                Span::styled(" 新待办: ", Style::default().fg(Color::Green)),
                Span::raw(&app.input),
                Span::styled("█", Style::default().fg(Color::White)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" 添加模式 (Enter 确认 / Esc 取消) "),
            );
            f.render_widget(input_widget, chunks[2]);
        }
        AppMode::Editing => {
            let input_widget = Paragraph::new(Line::from(vec![
                Span::styled(" 编辑: ", Style::default().fg(Color::Yellow)),
                Span::raw(&app.input),
                Span::styled("█", Style::default().fg(Color::White)),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" 编辑模式 (Enter 确认 / Esc 取消) "),
            );
            f.render_widget(input_widget, chunks[2]);
        }
        AppMode::ConfirmDelete => {
            let msg = if let Some(real_idx) = app.selected_real_index() {
                format!(
                    " 确认删除「{}」？(y 确认 / n 取消)",
                    app.list.items[real_idx].content
                )
            } else {
                " 没有选中的项目".to_string()
            };
            let confirm_widget = Paragraph::new(Line::from(Span::styled(
                msg,
                Style::default().fg(Color::Red),
            )))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Red))
                    .title(" ⚠️ 确认删除 "),
            );
            f.render_widget(confirm_widget, chunks[2]);
        }
        AppMode::Normal => {
            let msg = app.message.as_deref().unwrap_or("按 ? 查看完整帮助");
            let dirty_indicator = if app.dirty { " [未保存]" } else { "" };
            let status_widget = Paragraph::new(Line::from(vec![
                Span::styled(msg, Style::default().fg(Color::Gray)),
                Span::styled(
                    dirty_indicator,
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
            f.render_widget(status_widget, chunks[2]);
        }
    }

    // ========== 帮助栏 ==========
    let help_text = match app.mode {
        AppMode::Normal => {
            " n/↓ 下移 | N/↑ 上移 | 空格/回车 切换完成 | a 添加 | e 编辑 | d 删除 | f 过滤 | s 保存 | q/Esc 退出"
        }
        AppMode::Adding | AppMode::Editing => " Enter 确认 | Esc 取消",
        AppMode::ConfirmDelete => " y 确认删除 | n/Esc 取消",
    };
    let help_widget = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )));
    f.render_widget(help_widget, chunks[3]);
}

/// 正常模式按键处理，返回 true 表示退出
fn handle_normal_mode(app: &mut TodoApp, key: KeyEvent) -> bool {
    // Ctrl+C 强制退出
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return true;
    }

    match key.code {
        // 退出
        KeyCode::Char('q') | KeyCode::Esc => return true,

        // 向下移动
        KeyCode::Char('n') | KeyCode::Down | KeyCode::Char('j') => app.move_down(),

        // 向上移动
        KeyCode::Char('N') | KeyCode::Up | KeyCode::Char('k') => app.move_up(),

        // 切换完成状态
        KeyCode::Char(' ') | KeyCode::Enter => app.toggle_done(),

        // 添加
        KeyCode::Char('a') => {
            app.mode = AppMode::Adding;
            app.input.clear();
            app.message = None;
        }

        // 编辑
        KeyCode::Char('e') => {
            if let Some(real_idx) = app.selected_real_index() {
                app.input = app.list.items[real_idx].content.clone();
                app.edit_index = Some(real_idx);
                app.mode = AppMode::Editing;
                app.message = None;
            }
        }

        // 删除（需确认）
        KeyCode::Char('d') => {
            if app.selected_real_index().is_some() {
                app.mode = AppMode::ConfirmDelete;
            }
        }

        // 过滤切换
        KeyCode::Char('f') => app.toggle_filter(),

        // 保存
        KeyCode::Char('s') => app.save(),

        // 调整顺序: Shift+↑ 上移 / Shift+↓ 下移
        KeyCode::Char('K') => app.move_item_up(),
        KeyCode::Char('J') => app.move_item_down(),

        _ => {}
    }

    false
}

/// 输入模式按键处理（添加/编辑通用）
fn handle_input_mode(app: &mut TodoApp, key: KeyEvent) {
    match key.code {
        KeyCode::Enter => {
            if app.mode == AppMode::Adding {
                app.add_item();
            } else {
                app.confirm_edit();
            }
        }
        KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.input.clear();
            app.edit_index = None;
            app.message = Some("已取消".to_string());
        }
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        _ => {}
    }
}

/// 确认删除按键处理
fn handle_confirm_delete(app: &mut TodoApp, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.delete_selected();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.message = Some("已取消删除".to_string());
        }
        _ => {}
    }
}
