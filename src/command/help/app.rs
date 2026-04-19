use crate::assets::{self, HelpTab};
use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::render::theme::{Theme, ThemeName};
use ratatui::text::Line;

/// 每个 Tab 的缓存数据
struct TabCache {
    lines: Vec<Line<'static>>,
    cached_width: usize,
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
}

impl Default for HelpApp {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpApp {
    pub fn new() -> Self {
        let tabs: Vec<HelpTab> = assets::load_help_tabs();
        let count = tabs.len();
        let tab_names: Vec<String> = tabs.iter().map(|t| t.name.clone()).collect();
        let tab_raw_contents: Vec<String> = tabs.into_iter().map(|t| t.content).collect();
        Self {
            active_tab: 0,
            tab_count: count,
            tab_names,
            tab_raw_contents,
            tab_caches: (0..count).map(|_| None).collect(),
            tab_scrolls: vec![0; count],
            total_lines: 0,
            theme: Theme::from_name(&ThemeName::default()),
        }
    }

    pub fn tab_name(&self, idx: usize) -> &str {
        self.tab_names.get(idx).map(|s| s.as_str()).unwrap_or("?")
    }

    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// 获取当前 Tab 的渲染行（带缓存）
    pub fn current_tab_lines(&mut self, content_width: usize) -> &[Line<'static>] {
        let idx = self.active_tab;

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

    pub fn scroll_offset(&self) -> usize {
        self.tab_scrolls[self.active_tab]
    }

    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tab_count;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = (self.active_tab + self.tab_count - 1) % self.tab_count;
    }

    pub fn goto_tab(&mut self, idx: usize) {
        if idx < self.tab_count {
            self.active_tab = idx;
        }
    }

    pub fn scroll_down(&mut self, n: usize) {
        let idx = self.active_tab;
        self.tab_scrolls[idx] = self.tab_scrolls[idx].saturating_add(n);
    }

    pub fn scroll_up(&mut self, n: usize) {
        let idx = self.active_tab;
        self.tab_scrolls[idx] = self.tab_scrolls[idx].saturating_sub(n);
    }

    pub fn scroll_to_top(&mut self) {
        let idx = self.active_tab;
        self.tab_scrolls[idx] = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        let idx = self.active_tab;
        // total_lines 会在 draw 时更新，这里设一个很大的值，在 draw 时会被钳制
        self.tab_scrolls[idx] = usize::MAX;
    }

    pub fn invalidate_cache(&mut self) {
        for cache in &mut self.tab_caches {
            *cache = None;
        }
    }

    /// 钳制滚动偏移（在 draw 后调用，确保不超出内容范围）
    pub fn clamp_scroll(&mut self, visible_height: usize) {
        let idx = self.active_tab;
        let max_scroll = self.total_lines.saturating_sub(visible_height);
        if self.tab_scrolls[idx] > max_scroll {
            self.tab_scrolls[idx] = max_scroll;
        }
    }
}
