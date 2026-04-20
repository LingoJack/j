//! 公共 TUI 组件库
//!
//! 从 `chat/ui/components.rs` 抽取的通用组件，供 chat / help / todo 复用。

pub mod consts;
pub mod cursor;
pub mod hint;
pub mod label;
pub mod list;
pub mod pointer;
pub mod row;
pub mod separator;
pub mod tab_bar;

pub use consts::*;
pub use cursor::cursor_spans;
pub use hint::{help_key_row, hint_spans};
pub use label::{desc_span, label_span};
pub use list::ItemList;
pub use pointer::pointer_span;
pub use row::{selectable_row, text_field_row, toggle_list_item, toggle_row};
pub use separator::{section_header, separator_line};
pub use tab_bar::tab_bar;
