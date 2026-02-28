//! j-cli 库模块
//!
//! 导出公开模块供集成测试和外部使用

pub mod assets;
pub mod cli;
pub mod command;
pub mod config;
pub mod constants;
pub mod interactive;
pub mod tui;
pub mod util;

// 注意：main.rs 中的实际执行逻辑不放在 lib 中
// lib.rs 只导出可复用的模块
