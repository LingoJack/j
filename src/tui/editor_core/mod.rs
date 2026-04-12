//! 自研 Markdown 编辑器核心模块
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行。

mod history;
mod renderer;
mod search;
pub mod text_buffer;
pub mod theme;
mod vim;
mod wrap_engine;

mod editor;
pub use editor::{
    open_markdown_editor, open_markdown_editor_on_terminal, open_markdown_editor_with_content,
};
pub use theme::{EditorTheme, HighlightFn};
