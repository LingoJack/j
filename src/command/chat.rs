pub mod agent;
pub mod agent_md;
pub mod app;
pub mod constants;
pub mod error;
pub mod handler;
pub mod infra;
pub mod input;
pub mod markdown;
pub mod oneshot;
pub mod permission;
pub mod remote;
pub mod render;
pub mod storage;
pub mod teammate;
pub mod tools;
pub mod ui;

pub use oneshot::handle_chat;

// Re-exports needed by super::super:: relative paths within chat submodules
pub use agent::compact;
pub use infra::archive;
pub use infra::hook;
pub use input::autocomplete;
pub use input::input_thread;
pub use render::cache as render_cache;
pub use render::helpers as ui_helpers;
pub use render::theme;
