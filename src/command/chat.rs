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

// Re-exports from agent/ subdirectory (backward compat)
pub use agent::api;
pub use agent::compact;
pub use agent::config as agent_config;

// Re-exports from teammate/ subdirectory (backward compat)
pub use teammate::teammate_loop;

// Re-exports from permission/ subdirectory (backward compat)
pub use permission::queue as permission_queue;

// Re-exports from infra/ subdirectory (backward compat)
pub use infra::archive;
pub use infra::command;
pub use infra::hook;
pub use infra::sandbox;
pub use infra::skill;

// Re-exports from render/ subdirectory (backward compat)
pub use render::cache as render_cache;
pub use render::helpers as ui_helpers;
pub use render::theme;

// Re-exports from input/ subdirectory (backward compat)
pub use input::autocomplete;
pub use input::input_thread;
