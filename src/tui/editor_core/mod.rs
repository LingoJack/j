//! 自研 Markdown 编辑器核心模块
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行。

mod history;
mod renderer;
mod search;
mod text_buffer;
mod vim;
mod wrap_engine;

pub use history::{History, Snapshot};
pub use renderer::MarkdownRenderer;
pub use search::{SearchMatch, SearchState};
pub use text_buffer::{Cursor, TextBuffer};
pub use vim::{Input, Key, Mode, Transition, Vim};
pub use wrap_engine::{VisualLine, WrapEngine};

mod editor;
pub use editor::{
    EditorAction, MarkdownEditor as Editor, open_markdown_editor, open_markdown_editor_on_terminal,
    open_markdown_editor_with_content,
};
