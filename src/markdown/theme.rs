//! Markdown 渲染层主题抽象
//!
//! `MdStyle` 抽象 markdown 渲染所需的颜色与代码高亮主题，
//! 让 parser/render 不直接依赖具体的 `chat::Theme` 或 `EditorTheme`。
//!
//! 当前阶段（Step 2）：trait 方法名与字段名一一对应，行为零变化。
//! 后续阶段（Step 3+）：editor 侧将实现自己的 `MdStyle`，并替换 `code_syntax_theme`
//! 为独立的语法高亮主题抽象，以彻底解耦跨模块依赖。

use crate::theme::Theme;
use crate::tui::editor_core::EditorTheme;
use ratatui::style::Color;

/// Markdown 渲染样式抽象。
///
/// 每个 method 对应 markdown 渲染中用到的一个样式槽位。
/// 实现方负责把自己的主题字段映射到这些槽位上。
pub trait MdStyle {
    // ===== 文本基础色 =====
    fn text_normal(&self) -> Color;
    fn text_bold(&self) -> Color;
    fn text_dim(&self) -> Color;

    // ===== Heading =====
    fn md_h1(&self) -> Color;
    fn md_h2(&self) -> Color;
    fn md_h3(&self) -> Color;
    fn md_h4(&self) -> Color;
    fn md_heading_sep(&self) -> Color;

    // ===== Inline =====
    fn md_link(&self) -> Color;
    fn md_inline_code_fg(&self) -> Color;

    // ===== Global background =====
    fn bg_primary(&self) -> Color;

    // ===== Blockquote =====
    fn md_blockquote_bar(&self) -> Color;
    fn md_blockquote_bg(&self) -> Color;
    fn md_blockquote_text(&self) -> Color;

    // ===== List / Rule =====
    fn md_list_bullet(&self) -> Color;
    fn md_rule(&self) -> Color;

    // ===== Code Block =====
    // code_border / code_bg 不再使用——代码块边框和背景已统一使用 text_dim / bg_primary

    // ===== Table =====
    fn table_header(&self) -> Color;
    fn table_body(&self) -> Color;

    // ===== Code Block 语法高亮主题 =====
    /// 返回供 `highlight_code_line` 使用的 `EditorTheme`。
    /// Step 3 起会引入独立的 `SyntaxHighlightTheme` 抽象，
    /// 进一步解耦 markdown 模块对 `editor_core` 的依赖。
    fn code_syntax_theme(&self) -> EditorTheme;
}

/// chat 侧的 `MdStyle` 实现：直接把 `Theme` 的字段映射到 trait method。
impl MdStyle for Theme {
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

    fn code_syntax_theme(&self) -> EditorTheme {
        EditorTheme::from(self)
    }
}
