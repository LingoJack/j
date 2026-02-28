use crate::assets::HELP_TEXT;
use crate::command::chat::markdown::markdown_to_lines;
use crate::command::chat::theme::{Theme, ThemeName};
use ratatui::text::Line;

/// Tab 定义：名称 + 匹配的 ## 标题关键词列表
struct TabDef {
    name: &'static str,
    heading_keywords: &'static [&'static str],
}

const TAB_DEFS: &[TabDef] = &[
    TabDef {
        name: "快速上手",
        heading_keywords: &["快速上手"],
    },
    TabDef {
        name: "数据目录",
        heading_keywords: &["数据目录"],
    },
    TabDef {
        name: "别名 & 打开",
        heading_keywords: &["别名管理", "分类标记", "列表", "打开"],
    },
    TabDef {
        name: "日报",
        heading_keywords: &["日报系统"],
    },
    TabDef {
        name: "待办",
        heading_keywords: &["待办备忘录"],
    },
    TabDef {
        name: "脚本 & 计时",
        heading_keywords: &["脚本"],
    },
    TabDef {
        name: "系统 & 语音",
        heading_keywords: &["系统设置", "语音转文字"],
    },
    TabDef {
        name: "AI 对话",
        heading_keywords: &["AI 对话"],
    },
    TabDef {
        name: "安装 & 卸载",
        heading_keywords: &["安装", "卸载"],
    },
    TabDef {
        name: "使用技巧",
        heading_keywords: &["使用技巧"],
    },
];

/// 按 `## ` 标题行将 HELP_TEXT 分割到各 Tab
fn split_help_into_tabs() -> Vec<String> {
    // 先按 ## 标题切分所有 section
    let mut sections: Vec<(String, String)> = Vec::new(); // (标题行文本, 内容)
    let mut current_heading = String::new();
    let mut current_content = String::new();

    for line in HELP_TEXT.lines() {
        if line.starts_with("## ") {
            // 保存上一个 section
            if !current_heading.is_empty() {
                sections.push((current_heading.clone(), current_content.clone()));
            }
            current_heading = line.to_string();
            current_content = String::new();
            current_content.push_str(line);
            current_content.push('\n');
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    if !current_heading.is_empty() {
        sections.push((current_heading, current_content));
    }

    // 将 sections 分配到各 tab
    let mut tab_contents: Vec<String> = vec![String::new(); TAB_DEFS.len()];

    for (heading, content) in &sections {
        let mut matched = false;
        for (tab_idx, tab_def) in TAB_DEFS.iter().enumerate() {
            for kw in tab_def.heading_keywords {
                if heading.contains(kw) {
                    if !tab_contents[tab_idx].is_empty() {
                        tab_contents[tab_idx].push_str("\n---\n\n");
                    }
                    tab_contents[tab_idx].push_str(content);
                    matched = true;
                    break;
                }
            }
            if matched {
                break;
            }
        }
    }

    tab_contents
}

/// 每个 Tab 的缓存数据
struct TabCache {
    lines: Vec<Line<'static>>,
    cached_width: usize,
}

/// HelpApp 状态
pub struct HelpApp {
    pub active_tab: usize,
    pub tab_count: usize,
    tab_names: Vec<&'static str>,
    tab_raw_contents: Vec<String>,
    tab_caches: Vec<Option<TabCache>>,
    tab_scrolls: Vec<usize>,
    /// 当前 Tab 的总渲染行数（用于滚动限制）
    pub total_lines: usize,
    theme: Theme,
}

impl HelpApp {
    pub fn new() -> Self {
        let tab_raw_contents = split_help_into_tabs();
        let count = TAB_DEFS.len();
        let tab_names: Vec<&'static str> = TAB_DEFS.iter().map(|t| t.name).collect();
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
        self.tab_names.get(idx).copied().unwrap_or("?")
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

        let cache = self.tab_caches[idx].as_ref().unwrap();
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
