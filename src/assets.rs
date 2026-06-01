//! 编译时嵌入资源统一管理
//!
//! 使用 `rust-embed` 实现资源嵌入，支持运行时动态查找和迭代。
//!
//! # 资源清单
//!
//! | 资源名称 | 类型 | 路径 | 用途 |
//! |---------|------|------|------|
//! | `HELP_TABS` | 文本 | `assets/help/*.md` | 帮助文档内容（目录树驱动） |
//! | `VERSION_TEMPLATE` | 文本 | `assets/version.md` | 版本命令模板 |
//! | `DEFAULT_SYSTEM_PROMPT` | 文本 | `assets/system_prompt_default.md` | 默认系统提示词模板 |
//! | `DEFAULT_MEMORY` | 文本 | `assets/memory_default.md` | 默认记忆占位文件 |
//! | `DEFAULT_SOUL` | 文本 | `assets/soul_default.md` | 默认灵魂占位文件 |
//! | `DEFAULT_AGENT_MD` | 文本 | `assets/agent_md_default.md` | 默认 AGENTS.md 模板 |
//! | `TEAMMATE_SYSTEM_PROMPT` | 文本 | `assets/teammate_system_prompt.md` | Teammate system prompt 模板 |
//! | `SUB_AGENT_SYSTEM_PROMPT` | 文本 | `assets/sub_agent_system_prompt.md` | SubAgent system prompt 模板 |

mod help;
mod install;
mod template;

use rust_embed::RustEmbed;

/// 编译时嵌入资源统一管理
///
/// 所有 assets 目录下的文件都会被嵌入到二进制中。
/// 例外：`remote/` 与 `reader/` 这两个前端工作区有自己的 RustEmbed struct
/// （`crate::command::chat::remote::RemoteAssets` /
/// `crate::command::read::ReaderAssets`），这里整棵跳过，避免把它们的
/// 源码 / node_modules / 构建配置一并塞进二进制。
#[derive(Debug, RustEmbed)]
#[folder = "assets/"]
#[exclude = "remote/*"]
#[exclude = "reader/*"]
pub struct Assets;

// ========== 重导出子模块公开 API ==========

pub use help::{HelpEntry, HelpEntryKind, HelpExpandedDirs, help_file_count, load_help_entries};
pub use install::{install_default_commands, install_default_scripts, install_default_skills};
pub use template::{
    default_agent_md, default_memory, default_soul, default_system_prompt, quotes_text, tips_text,
    version_template,
};
