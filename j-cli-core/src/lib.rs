//! j-cli-core: 核心聊天引擎库
//!
//! 包含 LLM 客户端、Agent、工具系统、权限管理、协议定义等核心逻辑。
//! 不依赖 ratatui/crossterm，可被 CLI 和 GUI（Tauri）共同使用。

pub mod agent;
pub mod agent_md;
pub mod chat_error;
pub mod constants;
pub mod context;
pub mod crypto;
pub mod infra;
pub mod llm;
pub mod message_types;
pub mod permission;
pub mod protocol;
pub mod storage;
pub mod teammate;
pub mod template;
pub mod theme_name;
pub mod tools;
pub mod util;
