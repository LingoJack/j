//! 日报（Report）模块。
//!
//! 模块拆分：
//! - `io` — 文件读写辅助（路径获取、尾部读取、追加/替换）
//! - `write` — 日报写入、TUI 编辑、周数管理、配置同步
//! - `git` — git 同步（push/pull/set-url、仓库管理）
//! - `query` — 查询命令（check/search）

pub mod git;
pub mod io;
pub mod query;
pub mod write;

// Re-export 外部模块实际使用的公共接口
pub use query::{handle_check, handle_search};
pub use write::{handle_report, write_to_report};
