//! Notebook TUI 应用状态与交互。
//!
//! 模块拆分：
//! - `types` — 核心数据类型（NoteItem, FlatEntry, NotebookApp 等）
//! - `io` — 文件 I/O 操作（读写笔记、配置持久化、编辑器调用等）
//! - `flat_entries` — 树形目录扁平化构建
//! - `input` — 各模式按键处理分发

pub mod flat_entries;
pub mod input;
pub mod io;
pub mod types;

// Re-export 外部模块（handler.rs / ui.rs）实际使用的类型
pub use types::{AppMode, FlatEntryKind, Focus, NotebookApp};

// Re-export I/O 函数供 handler.rs 使用
pub use io::{
    cleanup_empty_dirs, edit_note_with_editor, format_time, load_notes, note_file_path,
    notebook_dir,
};

// Re-export input 处理函数供 handler.rs 使用
pub use input::{
    handle_command_popup_mode, handle_confirm_delete, handle_input_mode, handle_normal_mode,
    handle_ratio_input_mode,
};
