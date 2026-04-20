//! Markdown 编辑器（高级渲染版本）
//!
//! 实现类似 Typora 的编辑体验：
//! - 当前编辑行显示原始 Markdown 源码
//! - 其他行显示渲染后的效果
//! - 支持代码块围栏样式、表格渲染、语法高亮等
//!
//! 本模块是 editor_core 的薄封装，负责 Theme → EditorTheme 转换和高亮函数注入。

use crate::command::chat::storage::{load_agent_config, save_agent_config};
use std::io;

use crate::command::chat::markdown::highlight::highlight_code_line;
use crate::theme::{Theme, ThemeName};

use crate::tui::editor_core::{EditorTheme, HighlightFn, ThemeGalleryItem};
// 直接使用 editor_core 的公共 API
use crate::tui::editor_core::{
    open_markdown_editor as core_open, open_markdown_editor_on_terminal as core_open_on_terminal,
    open_markdown_editor_with_content as core_open_with_content,
};

// ========== Theme 转换 ==========

impl From<&Theme> for EditorTheme {
    fn from(t: &Theme) -> Self {
        EditorTheme {
            bg_primary: t.bg_primary,
            bg_input: t.bg_input,
            code_bg: t.code_bg,
            cursor_fg: t.cursor_fg,
            cursor_bg: t.cursor_bg,
            text_normal: t.text_normal,
            text_dim: t.text_dim,
            text_bold: t.text_bold,
            md_h1: t.md_h1,
            md_h2: t.md_h2,
            md_h3: t.md_h3,
            md_h4: t.md_h4,
            md_link: t.md_link,
            md_list_bullet: t.md_list_bullet,
            md_blockquote_bar: t.md_blockquote_bar,
            md_blockquote_bg: t.md_blockquote_bg,
            md_blockquote_text: t.md_blockquote_text,
            md_inline_code_fg: t.md_inline_code_fg,
            md_inline_code_bg: t.md_inline_code_bg,
            code_default: t.code_default,
            code_keyword: t.code_keyword,
            code_string: t.code_string,
            code_comment: t.code_comment,
            code_number: t.code_number,
            code_type: t.code_type,
            code_primitive: t.code_primitive,
            code_macro: t.code_macro,
            code_lifetime: t.code_lifetime,
            code_attribute: t.code_attribute,
            code_shell_var: t.code_shell_var,
            label_ai: t.label_ai,
        }
    }
}

/// 桥接高亮函数：将 EditorTheme 适配到 highlight_code_line
fn bridge_highlight(
    line: &str,
    lang: &str,
    theme: &EditorTheme,
) -> Vec<ratatui::text::Span<'static>> {
    highlight_code_line(line, lang, theme)
}

/// 构建主题画廊（所有内置主题 → EditorTheme）
fn build_theme_gallery() -> Vec<ThemeGalleryItem> {
    ThemeName::all()
        .iter()
        .map(|name| {
            let theme = Theme::from_name(name);
            let editor_theme = EditorTheme::from(&theme);
            (name.display_name(), name.to_str(), editor_theme)
        })
        .collect()
}

/// 如果用户在编辑器中选择了主题，保存到 agent_config
fn save_theme_if_selected(theme_id: Option<&'static str>) {
    if let Some(id) = theme_id {
        let mut config = load_agent_config();
        config.theme = ThemeName::parse(id);
        save_agent_config(&config);
    }
}

// ========== 公共 API ==========

/// 打开 Markdown 编辑器（在已有终端上）
pub fn open_markdown_editor_on_terminal(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    title: &str,
    content: &str,
    theme: &Theme,
) -> io::Result<(Option<String>, Option<&'static str>)> {
    let editor_theme = EditorTheme::from(theme);
    let result = core_open_on_terminal(
        terminal,
        title,
        content,
        &editor_theme,
        bridge_highlight as HighlightFn,
        build_theme_gallery(),
    )?;
    save_theme_if_selected(result.1);
    Ok(result)
}

/// 打开 Markdown 编辑器（独立终端）
pub fn open_markdown_editor(
    title: &str,
    content: &str,
    theme: &Theme,
) -> io::Result<(Option<String>, Option<&'static str>)> {
    let editor_theme = EditorTheme::from(theme);
    let result = core_open(
        title,
        content,
        &editor_theme,
        bridge_highlight as HighlightFn,
        build_theme_gallery(),
    )?;
    save_theme_if_selected(result.1);
    Ok(result)
}

/// 使用指定内容打开编辑器（预填充行，NORMAL 模式启动）
pub fn open_markdown_editor_with_content(
    title: &str,
    initial_lines: &[String],
    theme: &Theme,
) -> io::Result<(Option<String>, Option<&'static str>)> {
    let editor_theme = EditorTheme::from(theme);
    let result = core_open_with_content(
        title,
        initial_lines,
        &editor_theme,
        bridge_highlight as HighlightFn,
        build_theme_gallery(),
    )?;
    save_theme_if_selected(result.1);
    Ok(result)
}

/// 打开脚本编辑器（读取用户保存的主题偏好）
///
/// 适用于脚本编辑等不需要外部传入主题的场景
/// 会从 agent_config 中加载用户上次的 theme 选择
pub fn open_script_editor(
    title: &str,
    initial_lines: &[String],
) -> io::Result<(Option<String>, Option<&'static str>)> {
    let agent_config = load_agent_config();
    let theme = Theme::from_name(&agent_config.theme);
    let editor_theme = EditorTheme::from(&theme);
    let result = core_open_with_content(
        title,
        initial_lines,
        &editor_theme,
        bridge_highlight as HighlightFn,
        build_theme_gallery(),
    )?;
    save_theme_if_selected(result.1);
    Ok(result)
}
