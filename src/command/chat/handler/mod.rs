mod archive;
mod browse;
mod chat;
mod config;
mod tool_confirm;
mod tui_loop;

// Re-export all handler functions
pub use archive::{handle_archive_confirm_mode, handle_archive_list_mode};
pub use browse::handle_browse_mode;
pub use chat::handle_chat_mode;
pub use config::{
    handle_config_mode, handle_select_model, handle_skill_toggle_mode, handle_tool_toggle_mode,
};
pub use tool_confirm::handle_tool_confirm_mode;

// Re-export TUI event loop entry point
pub use tui_loop::run_chat_tui;

// Re-export config_field_* from super::ui_helpers (for ui/config.rs compatibility)
pub use super::ui_helpers::{config_field_label, config_field_value};

// Re-export autocomplete functions (for ui/chat.rs compatibility)
pub use super::autocomplete::{get_filtered_files, get_filtered_skill_names, get_filtered_skills};
