//! 编辑器独立主题（解耦 chat::Theme）
//!
//! 定义编辑器渲染所需的所有样式字段，
//! 使 editor_core 模块不依赖 chat 子系统。

use ratatui::style::Color;

// 从主 theme 模块导入 BorderStyle 和 ThemeColor
pub use crate::theme::{BorderStyle, ThemeColor};

/// 编辑器主题
#[derive(Debug, Clone, PartialEq)]
pub struct EditorTheme {
    // ===== 全局背景 =====
    pub bg_primary: Color,
    pub bg_input: Color,
    pub code_bg: Color,

    // ===== 光标 =====
    pub cursor_fg: ThemeColor,
    pub cursor_bg: ThemeColor,

    // ===== 文本 =====
    pub text_normal: Color,
    pub text_dim: Color,
    pub text_bold: Color,

    // ===== Markdown =====
    pub md_h1: Color,
    pub md_h2: Color,
    pub md_h3: Color,
    pub md_h4: Color,
    pub md_heading_sep: Color,
    pub md_link: Color,
    pub md_list_bullet: Color,
    pub md_blockquote_bar: Color,
    pub md_blockquote_bg: Color,
    pub md_blockquote_text: Color,
    pub md_inline_code_fg: Color,
    pub md_inline_code_bg: Color,
    pub md_rule: Color,

    // ===== 代码块 =====
    pub code_border: Color,
    /// 代码块边框样式：圆角或直角
    pub code_border_style: BorderStyle,

    // ===== 表格 =====
    pub table_header: Color,
    pub table_body: Color,

    // ===== 标签 =====
    pub label_ai: Color,

    // ===== Popup =====
    pub popup_border: Color,
    pub popup_title: Color,
    pub popup_bg: Color,
    pub popup_highlight_bg: ThemeColor,
    pub popup_highlight_fg: ThemeColor,

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

// ---------------------------------------------------------------------------
// MdStyle trait 实现：让共享渲染层可以直接使用 EditorTheme
// ---------------------------------------------------------------------------

impl crate::markdown::theme::MdStyle for EditorTheme {
    fn text_normal(&self) -> Color {
        self.text_normal
    }
    fn text_bold(&self) -> Color {
        self.text_bold
    }
    fn text_dim(&self) -> Color {
        self.text_dim
    }

    fn md_h1(&self) -> Color {
        self.md_h1
    }
    fn md_h2(&self) -> Color {
        self.md_h2
    }
    fn md_h3(&self) -> Color {
        self.md_h3
    }
    fn md_h4(&self) -> Color {
        self.md_h4
    }
    fn md_heading_sep(&self) -> Color {
        self.md_heading_sep
    }

    fn md_link(&self) -> Color {
        self.md_link
    }
    fn md_inline_code_fg(&self) -> Color {
        self.md_inline_code_fg
    }

    fn bg_primary(&self) -> Color {
        self.bg_primary
    }

    fn md_blockquote_bar(&self) -> Color {
        self.md_blockquote_bar
    }
    fn md_blockquote_bg(&self) -> Color {
        self.md_blockquote_bg
    }
    fn md_blockquote_text(&self) -> Color {
        self.md_blockquote_text
    }

    fn md_list_bullet(&self) -> Color {
        self.md_list_bullet
    }
    fn md_rule(&self) -> Color {
        self.md_rule
    }

    fn table_header(&self) -> Color {
        self.table_header
    }
    fn table_body(&self) -> Color {
        self.table_body
    }

    fn code_syntax_theme(&self) -> crate::tui::editor_core::EditorTheme {
        self.clone()
    }
}
