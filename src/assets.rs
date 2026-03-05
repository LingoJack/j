//! 编译时嵌入资源统一管理
//!
//! 所有通过 `include_str!` / `include_bytes!` 嵌入的外部资源
//! 都在此模块集中管理，便于维护和追踪。
//!
//! # 资源清单
//!
//! | 资源名称 | 类型 | 路径 | 用途 |
//! |---------|------|------|------|
//! | `HELP_TEXT` | 文本 | `assets/help.md` | 帮助命令输出 |
//! | `VERSION_TEMPLATE` | 文本 | `assets/version.md` | 版本命令模板 |

// ========== 文本资源 ==========

/// 帮助文档内容
///
/// 用途: `j help` 命令输出
/// 格式: Markdown
pub const HELP_TEXT: &str = include_str!("../assets/help.md");

/// 版本信息模板
///
/// 用途: `j version` 命令输出
/// 占位符: `{version}`, `{os}`, `{extra}`
/// 格式: Markdown 表格
pub const VERSION_TEMPLATE: &str = include_str!("../assets/version.md");
