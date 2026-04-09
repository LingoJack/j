//! 编辑器独立主题（解耦 chat::Theme）
//!
//! 定义编辑器渲染所需的所有样式字段，
//! 使 editor_core 模块不依赖 chat 子系统。

use ratatui::style::Color;

/// 编辑器主题
#[derive(Debug, Clone)]
pub struct EditorTheme {
    // ===== 全局背景 =====
    pub bg_primary: Color,
    pub bg_input: Color,
    pub code_bg: Color,

    // ===== 光标 =====
    pub cursor_fg: Color,
    pub cursor_bg: Color,

    // ===== 文本 =====
    pub text_normal: Color,
    pub text_dim: Color,
    pub text_bold: Color,

    // ===== Markdown =====
    pub md_h1: Color,
    pub md_h2: Color,
    pub md_h3: Color,
    pub md_h4: Color,
    pub md_link: Color,
    pub md_list_bullet: Color,
    pub md_blockquote_bar: Color,
    pub md_blockquote_bg: Color,
    pub md_blockquote_text: Color,
    pub md_inline_code_fg: Color,
    pub md_inline_code_bg: Color,

    // ===== 代码高亮 =====
    pub code_default: Color,
    pub code_keyword: Color,
    pub code_string: Color,
    pub code_comment: Color,
    pub code_number: Color,
    pub code_type: Color,
    pub code_primitive: Color,
    pub code_macro: Color,
    pub code_lifetime: Color,
    pub code_attribute: Color,
    pub code_shell_var: Color,
}

/// 语法高亮函数类型
pub type HighlightFn = fn(&str, &str, &EditorTheme) -> Vec<ratatui::text::Span<'static>>;
