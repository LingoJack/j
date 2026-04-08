//! 自研 Markdown 编辑器核心模块
//!
//! 完全摆脱 tui-textarea 依赖，支持自动折行。

mod text_buffer;
mod wrap_engine;
mod history;
mod vim;
mod search;
mod renderer;

pub use text_buffer::{TextBuffer, Cursor};
pub use wrap_engine::{WrapEngine, VisualLine};
pub use history::{History, Snapshot};
pub use vim::{Vim, Mode, Transition, Input, Key};
pub use search::{SearchState, SearchMatch};
pub use renderer::MarkdownRenderer;

mod editor;
pub use editor::{MarkdownEditor as Editor, EditorAction, open_markdown_editor, open_markdown_editor_on_terminal, open_markdown_editor_with_content};
