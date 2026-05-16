//! Shell 工具统一入口。
//!
//! 对外只暴露一个 `ShellTool`，平台差异分别放在：
//! - `shell/unix.rs`
//! - `shell/windows.rs`

pub(super) use super::ToolResult;
pub(super) use super::background;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::ShellTool;
#[cfg(windows)]
pub use windows::PowerShellTool as ShellTool;
