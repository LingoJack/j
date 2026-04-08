//! Markdown 编辑器（高级渲染版本）
//!
//! 实现类似 Typora 的编辑体验：
//! - 当前编辑行显示原始 Markdown 源码
//! - 其他行显示渲染后的效果
//! - 支持代码块围栏样式、表格渲染、语法高亮等
//!
//! 本模块是 editor_core 的薄封装，所有核心逻辑已迁移到 editor_core 中。

use std::io;

use crate::command::chat::theme::Theme;

// 直接使用 editor_core 的公共 API
use crate::tui::editor_core::{
    open_markdown_editor as core_open, open_markdown_editor_on_terminal as core_open_on_terminal,
    open_markdown_editor_with_content as core_open_with_content,
};

// ========== 公共 API ==========

/// 打开 Markdown 编辑器（在已有终端上）
pub fn open_markdown_editor_on_terminal(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>,
    title: &str,
    content: &str,
    theme: &Theme,
) -> io::Result<Option<String>> {
    core_open_on_terminal(terminal, title, content, theme)
}

/// 打开 Markdown 编辑器（独立终端）
pub fn open_markdown_editor(
    title: &str,
    content: &str,
    theme: &Theme,
) -> io::Result<Option<String>> {
    core_open(title, content, theme)
}

/// 使用指定内容打开编辑器（预填充行，NORMAL 模式启动）
pub fn open_markdown_editor_with_content(
    title: &str,
    initial_lines: &[String],
    theme: &Theme,
) -> io::Result<Option<String>> {
    core_open_with_content(title, initial_lines, theme)
}

/// 打开脚本编辑器（使用 Dark 主题的便捷函数）
///
/// 适用于脚本编辑等不需要外部传入主题的场景
pub fn open_script_editor(title: &str, initial_lines: &[String]) -> io::Result<Option<String>> {
    core_open_with_content(title, initial_lines, &Theme::dark())
}
