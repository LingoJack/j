//! j-cli 核心库，供 CLI 和 GUI 共同使用

pub mod assets;
pub mod cli;
pub mod command;
pub mod config;
pub mod constants;
pub mod core;
pub mod tui;
pub mod util;

// CLI 专用模块不在 lib 中暴露：
// - interactive (REPL)

// 重导出核心类型
pub use config::YamlConfig;
