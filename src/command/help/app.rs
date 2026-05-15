use crate::assets::{self, HelpEntryKind, HelpExpandedDirs};
use crate::command::chat::storage::{load_agent_config, save_agent_config};
use crate::markdown::markdown_to_lines;
use crate::theme::{Theme, ThemeName};
use ratatui::text::Line;

/// 渲染缓存
struct ContentCache {
    lines: Vec<Line<'static>>,
    cached_width: usize,
}

/// 命令面板选项列表 (key, 中文标签)
pub const CMD_POPUP_ITEMS: &[(&str, &str)] = &[("theme", "切换主题"), ("quit", "退出")];

#[derive(PartialEq, Clone)]
/// Help 应用模式枚举
pub enum AppMode {
    /// 正常浏览模式
    Normal,
    /// 命令面板（/ 弹窗）
    CommandPopup,
    /// 主题选择
    ThemeSelect,
}

/// HelpApp 状态
pub struct HelpApp {
    /// 扁平化条目列表（由 expanded_dirs 驱动）
    flat_entries: Vec<crate::assets::HelpEntry>,
    /// 目录展开状态
    expanded_dirs: HelpExpandedDirs,
    /// 当前选中的条目索引（在 flat_entries 中）
    pub selected: usize,
    /// 内容滚动偏移
    pub content_scroll: usize,
    /// 内容渲染缓存（按条目 path 索引）
    content_cache: Option<ContentCache>,
    /// 当前缓存对应的条目路径
    cached_entry_path: Option<String>,
    /// 当前内容的总渲染行数
    pub total_lines: usize,
    /// 左侧面板宽度（字符数，0 表示自动计算）
    pub left_width: usize,
    /// 主题
    theme: Theme,
    /// 当前主题名称
    pub theme_name: ThemeName,
    /// 当前模式
    pub mode: AppMode,
    /// 命令面板筛选文本
    pub cmd_popup_filter: String,
    /// 命令面板选中索引
    pub cmd_popup_selected: usize,
    /// 主题弹窗选中索引
    pub theme_popup_selected: usize,
    /// 状态栏临时消息
    pub message: Option<String>,
    /// 是否正在拖拽分割线
    pub is_dragging_panel: bool,
}

impl Default for HelpApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpApp {
    /// 创建 HelpApp 实例
    pub fn new() -> Self {
        let expanded_dirs = HelpExpandedDirs::new();
        let flat_entries = assets::load_help_entries(&expanded_dirs);

        let agent_config = load_agent_config();
        let theme_name = agent_config.theme.clone();
        let theme = Theme::from_name(&theme_name);
        let theme_popup_selected = ThemeName::all()
            .iter()
            .position(|t| t == &theme_name)
            .unwrap_or(0);

        let mut app = Self {
            flat_entries,
            expanded_dirs,
            selected: 0,
            content_scroll: 0,
            content_cache: None,
            cached_entry_path: None,
            total_lines: 0,
            left_width: 0, // 0 = 自动计算
            theme,
            theme_name,
            mode: AppMode::Normal,
            cmd_popup_filter: String::new(),
            cmd_popup_selected: 0,
            theme_popup_selected,
            message: None,
            is_dragging_panel: false,
        };

        // 默认选中第一个文件条目
        app.select_first_file();
        app
    }

    /// 获取当前主题引用
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// 获取扁平化条目列表
    pub fn entries(&self) -> &[crate::assets::HelpEntry] {
        &self.flat_entries
    }

    /// 获取当前选中的条目
    pub fn selected_entry(&self) -> Option<&crate::assets::HelpEntry> {
        self.flat_entries.get(self.selected)
    }

    /// 选中第一个文件条目
    fn select_first_file(&mut self) {
        for (i, entry) in self.flat_entries.iter().enumerate() {
            if matches!(entry.kind, HelpEntryKind::File { .. }) {
                self.selected = i;
                return;
            }
        }
    }

    /// 向上移动选中
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// 向下移动选中
    pub fn move_down(&mut self) {
        if self.selected + 1 < self.flat_entries.len() {
            self.selected += 1;
        }
    }

    /// 切换当前目录条目的展开/折叠
    pub fn toggle_expand(&mut self) {
        let Some(entry) = self.flat_entries.get(self.selected) else {
            return;
        };
        let HelpEntryKind::Dir { ref dir_path, .. } = entry.kind else {
            return;
        };
        let dir_path = dir_path.clone();
        self.expanded_dirs.toggle(&dir_path);
        self.rebuild_flat_entries();
    }

    /// 重建扁平化条目列表，并保持选中位置
    fn rebuild_flat_entries(&mut self) {
        // 记录当前选中条目的标识
        let selected_id = self.flat_entries.get(self.selected).map(|e| match &e.kind {
            HelpEntryKind::Dir { dir_path, .. } => format!("dir:{}", dir_path),
            HelpEntryKind::File { path, .. } => format!("file:{}", path),
        });

        self.flat_entries = assets::load_help_entries(&self.expanded_dirs);

        // 尝试恢复选中位置
        if let Some(id) = selected_id {
            let new_idx = self.flat_entries.iter().position(|e| match &e.kind {
                HelpEntryKind::Dir { dir_path, .. } => id == format!("dir:{}", dir_path),
                HelpEntryKind::File { path, .. } => id == format!("file:{}", path),
            });
            if let Some(idx) = new_idx {
                self.selected = idx;
            }
        }
    }

    /// 获取右侧内容区的渲染行（带缓存）
    pub fn content_lines(&mut self, content_width: usize) -> &[Line<'static>] {
        // 获取当前选中条目的 path 和 content
        let entry_data = self.selected_entry().map(|e| match &e.kind {
            HelpEntryKind::File { path, content, .. } => (path.clone(), content.clone()),
            HelpEntryKind::Dir {
                dir_path,
                name,
                file_count,
            } => {
                // 目录条目显示简要信息
                let md = format!("# {name}\n\n共 {file_count} 篇文档。\n\n选择左侧文件查看内容。");
                (format!("dir:{}", dir_path), md)
            }
        });

        let Some((entry_path, md_text)) = entry_data else {
            self.total_lines = 1;
            return &[];
        };

        // 检查缓存是否有效
        let need_rebuild = self.cached_entry_path.as_ref() != Some(&entry_path)
            || self
                .content_cache
                .as_ref()
                .is_none_or(|c| c.cached_width != content_width);

        if need_rebuild {
            let lines = if md_text.trim().is_empty() {
                vec![Line::from("  (暂无内容)")]
            } else {
                markdown_to_lines(&md_text, content_width, &self.theme)
            };
            self.content_cache = Some(ContentCache {
                lines,
                cached_width: content_width,
            });
            self.cached_entry_path = Some(entry_path);
        }

        let Some(cache) = self.content_cache.as_ref() else {
            return &[];
        };
        self.total_lines = cache.lines.len();
        &cache.lines
    }

    /// 向下滚动内容
    pub fn scroll_down(&mut self, n: usize) {
        self.content_scroll = self.content_scroll.saturating_add(n);
    }

    /// 向上滚动内容
    pub fn scroll_up(&mut self, n: usize) {
        self.content_scroll = self.content_scroll.saturating_sub(n);
    }

    /// 滚动到顶部
    pub fn scroll_to_top(&mut self) {
        self.content_scroll = 0;
    }

    /// 滚动到底部
    pub fn scroll_to_bottom(&mut self) {
        self.content_scroll = usize::MAX;
    }

    /// 清除渲染缓存
    pub fn invalidate_cache(&mut self) {
        self.content_cache = None;
        self.cached_entry_path = None;
    }

    /// 计算左侧面板的实际像素宽度
    ///
    /// 如果 `left_width` 为 0，根据条目中最宽文本自动计算；
    /// 否则直接使用 `left_width`。结果受 [MIN_LEFT, max_width] 约束。
    pub fn compute_left_panel_width(&self, max_width: usize) -> usize {
        const MIN_LEFT: usize = 16;
        const PADDING: usize = 6; // 边框 + 引导线 + 余量

        if self.left_width > 0 {
            return self.left_width.clamp(MIN_LEFT, max_width);
        }

        // 自动计算：取所有条目的最大显示宽度
        let max_entry_width = self
            .flat_entries
            .iter()
            .map(|e| {
                let guide_w = unicode_width::UnicodeWidthStr::width(e.guide.as_str());
                match &e.kind {
                    HelpEntryKind::Dir {
                        name, file_count, ..
                    } => guide_w + name.len() + 1 + file_count.to_string().len() + 3,
                    HelpEntryKind::File { name, .. } => guide_w + name.len(),
                }
            })
            .max()
            .unwrap_or(MIN_LEFT);

        (max_entry_width + PADDING).clamp(MIN_LEFT, max_width)
    }

    /// 放大左侧面板
    pub fn widen_left(&mut self, frame_width: usize) {
        let current = self.compute_left_panel_width(frame_width);
        self.left_width = (current + 2).min(frame_width);
    }

    /// 缩小左侧面板
    pub fn shrink_left(&mut self, frame_width: usize) {
        let current = self.compute_left_panel_width(frame_width);
        self.left_width = current.saturating_sub(2).max(16);
    }

    /// 通过鼠标拖拽设置面板宽度
    pub fn set_panel_width_from_drag(&mut self, col: u16, main_x: u16, main_width: u16) {
        if main_width == 0 {
            return;
        }
        let relative_x = col.saturating_sub(main_x) as usize;
        self.left_width = relative_x.clamp(16, main_width as usize - 10);
    }

    /// 模糊筛选命令面板选项
    pub fn filtered_cmd_items(&self) -> Vec<(usize, &'static str, &'static str)> {
        let filter = self.cmd_popup_filter.to_lowercase();
        CMD_POPUP_ITEMS
            .iter()
            .enumerate()
            .filter(|(_, (key, label))| {
                filter.is_empty()
                    || key.contains(filter.as_str())
                    || label.contains(filter.as_str())
            })
            .map(|(i, (key, label))| (i, *key, *label))
            .collect()
    }

    /// 进入命令面板模式
    pub fn open_command_popup(&mut self) {
        self.mode = AppMode::CommandPopup;
        self.cmd_popup_filter.clear();
        self.cmd_popup_selected = 0;
    }

    /// 进入主题选择模式
    pub fn open_theme_select(&mut self) {
        self.mode = AppMode::ThemeSelect;
        self.theme_popup_selected = ThemeName::all()
            .iter()
            .position(|t| t == &self.theme_name)
            .unwrap_or(0);
    }

    /// 应用选中的主题并持久化
    pub fn apply_selected_theme(&mut self) {
        let all = ThemeName::all();
        if let Some(name) = all.get(self.theme_popup_selected) {
            self.theme_name = name.clone();
            self.theme = Theme::from_name(name);
            self.invalidate_cache();
            let mut config = load_agent_config();
            config.theme = name.clone();
            save_agent_config(&config);
            self.message = Some(format!("主题: {}", name.display_name()));
        }
    }
}
