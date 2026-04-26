//! 公共 TUI 组件库
//!
//! 从 `chat/ui/components.rs` 抽取的通用组件，供 chat / help / todo / notebook 复用。

pub mod command_popup;
pub mod confirm_dialog;
pub mod consts;
pub mod cursor;
pub mod help_page;
pub mod hint;
pub mod label;
pub mod list;
pub mod pointer;
pub mod row;
pub mod separator;
pub mod status_input;
pub mod tab_bar;

pub use command_popup::{CommandItem, CommandPopupConfig, draw_command_popup};
pub use confirm_dialog::{ConfirmDialogConfig, draw_confirm_dialog};
pub use consts::*;
pub use cursor::{cursor_spans, cursor_wrapped_lines};
pub use help_page::{HelpPageConfig, HelpShortcut, draw_help_page};
pub use hint::{help_key_row, hint_spans};
pub use label::{desc_span, label_span, value_style};
pub use list::ItemList;
pub use pointer::pointer_span;
pub use row::{
    TextFieldRowCtx, ToggleListItemCtx, ToggleRowCtx, selectable_row, text_field_row,
    toggle_list_item, toggle_row,
};
pub use separator::{section_header, separator_line};
pub use status_input::{StatusInputParams, draw_status_input};
pub use tab_bar::tab_bar;
