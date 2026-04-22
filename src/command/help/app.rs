use crate::assets::{self, HelpTab};
use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::storage::{load_agent_config, save_agent_config};
use crate::theme::{Theme, ThemeName};
use ratatui::text::Line;

/// 每个 Tab 的缓存数据
struct TabCache {
    lines: Vec<Line<'static>>,
    cached_width: usize,
}

/// 命令面板选项列表 (key, 中文标签)
pub const CMD_POPUP_ITEMS: &[(&str, &str)] = &[
    ("theme", "切换主题"),
    ("help", "查看帮助首页"),
    ("quit", "退出"),
];

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
    pub active_tab: usize,
    pub tab_count: usize,
    tab_names: Vec<String>,
    tab_raw_contents: Vec<String>,
    tab_caches: Vec<Option<TabCache>>,
    tab_scrolls: Vec<usize>,
    /// 当前 Tab 的总渲染行数（用于滚动限制）
    pub total_lines: usize,
    theme: Theme,
    /// 当前主题名称（用于标识与保存）
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
}

impl Default for HelpApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpApp {
    /// 创建 HelpApp 实例，加载帮助页面内容和主题配置
    pub fn new() -> Self {
        let tabs: Vec<HelpTab> = assets::load_help_tabs();
        let count = tabs.len();
        let tab_names: Vec<String> = tabs.iter().map(|t| t.name.clone()).collect();
        let tab_raw_contents: Vec<String> = tabs.into_iter().map(|t| t.content).collect();
        let agent_config = load_agent_config();
        let theme_name = agent_config.theme.clone();
        let theme = Theme::from_name(&theme_name);
        let theme_popup_selected = ThemeName::all()
            .iter()
            .position(|t| t == &theme_name)
            .unwrap_or(0);
        Self {
            active_tab: 0,
            tab_count: count,
            tab_names,
            tab_raw_contents,
            tab_caches: (0..count).map(|_| None).collect(),
            tab_scrolls: vec![0; count],
            total_lines: 0,
            theme,
            theme_name,
            mode: AppMode::Normal,
            cmd_popup_filter: String::new(),
            cmd_popup_selected: 0,
            theme_popup_selected,
            message: None,
        }
    }

    /// 获取指定索引的 Tab 名称
    pub fn tab_name(&self, idx: usize) -> &str {
        self.tab_names.get(idx).map(|s| s.as_str()).unwrap_or("?")
    }

    /// 获取当前主题引用
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// 获取当前 Tab 的渲染行（带缓存）
    pub fn current_tab_lines(&mut self, content_width: usize) -> &[Line<'static>] {
        let idx = self.active_tab;

        // 边界检查
        if idx >= self.tab_caches.len() || idx >= self.tab_raw_contents.len() {
            self.total_lines = 1;
            return &[];
        }

        // 检查缓存是否有效
        let need_rebuild = match &self.tab_caches[idx] {
            Some(cache) => cache.cached_width != content_width,
            None => true,
        };

        if need_rebuild {
            let md_text = &self.tab_raw_contents[idx];
            let lines = if md_text.trim().is_empty() {
                vec![Line::from("  (暂无内容)")]
            } else {
                markdown_to_lines(md_text, content_width, &self.theme)
            };
            self.tab_caches[idx] = Some(TabCache {
                lines,
                cached_width: content_width,
            });
        }

        let cache = self.tab_caches[idx]
            .as_ref()
            .expect("缓存应该在 need_rebuild 检查后存在");
        self.total_lines = cache.lines.len();
        &cache.lines
    }

    /// 获取当前 Tab 的滚动偏移量
    pub fn scroll_offset(&self) -> usize {
        self.tab_scrolls.get(self.active_tab).copied().unwrap_or(0)
    }

    /// 切换到下一个 Tab
    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tab_count;
    }

    /// 切换到上一个 Tab
    pub fn prev_tab(&mut self) {
        self.active_tab = (self.active_tab + self.tab_count - 1) % self.tab_count;
    }

    /// 跳转到指定索引的 Tab
    pub fn goto_tab(&mut self, idx: usize) {
        if idx < self.tab_count {
            self.active_tab = idx;
        }
    }

    /// 向下滚动指定行数
    pub fn scroll_down(&mut self, n: usize) {
        if let Some(scroll) = self.tab_scrolls.get_mut(self.active_tab) {
            *scroll = scroll.saturating_add(n);
        }
    }

    /// 向上滚动指定行数
    pub fn scroll_up(&mut self, n: usize) {
        if let Some(scroll) = self.tab_scrolls.get_mut(self.active_tab) {
            *scroll = scroll.saturating_sub(n);
        }
    }

    /// 滚动到顶部
    pub fn scroll_to_top(&mut self) {
        if let Some(scroll) = self.tab_scrolls.get_mut(self.active_tab) {
            *scroll = 0;
        }
    }

    /// 滚动到底部
    pub fn scroll_to_bottom(&mut self) {
        if let Some(scroll) = self.tab_scrolls.get_mut(self.active_tab) {
            // total_lines 会在 draw 时更新，这里设一个很大的值，在 draw 时会被钳制
            *scroll = usize::MAX;
        }
    }

    /// 清除所有 Tab 的渲染缓存
    pub fn invalidate_cache(&mut self) {
        for cache in &mut self.tab_caches {
            *cache = None;
        }
    }

    /// 钳制滚动偏移（在 draw 后调用，确保不超出内容范围）
    pub fn clamp_scroll(&mut self, visible_height: usize) {
        if let Some(scroll) = self.tab_scrolls.get_mut(self.active_tab) {
            let max_scroll = self.total_lines.saturating_sub(visible_height);
            if *scroll > max_scroll {
                *scroll = max_scroll;
            }
        }
    }

    /// 模糊筛选命令面板选项，返回 (原索引, key, 标签)
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
