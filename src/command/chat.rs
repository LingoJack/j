// === 从 j-cli-core re-export 的模块 ===
// 这些模块的源码在 j-cli-core 中，这里只是 re-export
// j-cli 的 TUI 专属模块（handler, render, ui, input, oneshot, remote）保留在本地

pub mod app;
pub mod handler;
pub mod input;
pub mod oneshot;
pub mod remote;
pub mod render;
pub mod ui;

// Re-exports from j-cli-core
pub use j_cli_core::agent;
pub use j_cli_core::agent_md;
pub use j_cli_core::context;
pub use j_cli_core::infra;
pub use j_cli_core::permission;
pub use j_cli_core::storage;
pub use j_cli_core::teammate;
pub use j_cli_core::tools;

// Re-export types
pub use j_cli_core::chat_error as error;
pub use j_cli_core::constants;
pub use j_cli_core::message_types;
pub use j_cli_core::theme_name;
pub use j_cli_core::template;

#[cfg(test)]
mod regression_tests;

pub use oneshot::ChatArgs;
pub use oneshot::handle_chat;

// Re-exports for crate:: absolute paths from submodules
pub use infra::archive;
pub use input::input_thread;
