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
pub use config::{handle_config_mode, handle_select_model, handle_select_theme};
pub use tool_confirm::{
    handle_agent_perm_confirm_mode, handle_plan_approval_confirm_mode, handle_tool_confirm_mode,
};

// Re-export TUI event loop entry point
pub use tui_loop::run_chat_tui;
