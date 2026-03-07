//! 编译时嵌入资源统一管理
//!
//! 使用 `rust-embed` 实现资源嵌入，支持运行时动态查找和迭代。
//!
//! # 资源清单
//!
//! | 资源名称 | 类型 | 路径 | 用途 |
//! |---------|------|------|------|
//! | `HELP_TEXT` | 文本 | `assets/help.md` | 帮助命令输出 |
//! | `VERSION_TEMPLATE` | 文本 | `assets/version.md` | 版本命令模板 |
//! | `DEFAULT_SYSTEM_PROMPT` | 文本 | `assets/system_prompt_default.md` | 默认系统提示词模板 |
//! | `DEFAULT_MEMORY` | 文本 | `assets/memory_default.md` | 默认记忆占位文件 |
//! | `DEFAULT_SOUL` | 文本 | `assets/soul_default.md` | 默认灵魂占位文件 |

use rust_embed::RustEmbed;
use std::borrow::Cow;

/// 编译时嵌入资源统一管理
///
/// 所有 assets 目录下的文件都会被嵌入到二进制中
#[derive(RustEmbed)]
#[folder = "assets/"]
pub struct Assets;

// ========== 便捷访问函数 ==========

/// 帮助文档内容
///
/// 用途: `j help` 命令输出
/// 格式: Markdown
pub fn help_text() -> Cow<'static, str> {
    let bytes = Assets::get("help.md")
        .expect("help.md not found in assets")
        .data;
    String::from_utf8(bytes.into_owned())
        .expect("help.md is not valid UTF-8")
        .into()
}

/// 版本信息模板
///
/// 用途: `j version` 命令输出
/// 占位符: `{version}`, `{os}`, `{extra}`
/// 格式: Markdown 表格
pub fn version_template() -> Cow<'static, str> {
    let bytes = Assets::get("version.md")
        .expect("version.md not found in assets")
        .data;
    String::from_utf8(bytes.into_owned())
        .expect("version.md is not valid UTF-8")
        .into()
}

/// 默认系统提示词模板
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/system_prompt.md`
/// 占位符: `{{.tools}}`, `{{.skills}}`, `{{.style}}`, `{{.memory}}`, `{{.soul}}`
/// 格式: Markdown
pub fn default_system_prompt() -> Cow<'static, str> {
    let bytes = Assets::get("system_prompt_default.md")
        .expect("system_prompt_default.md not found in assets")
        .data;
    String::from_utf8(bytes.into_owned())
        .expect("system_prompt_default.md is not valid UTF-8")
        .into()
}

/// 默认记忆占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/memory.md`
/// 格式: Markdown
pub fn default_memory() -> Cow<'static, str> {
    let bytes = Assets::get("memory_default.md")
        .expect("memory_default.md not found in assets")
        .data;
    String::from_utf8(bytes.into_owned())
        .expect("memory_default.md is not valid UTF-8")
        .into()
}

/// 默认灵魂占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/soul.md`
/// 格式: Markdown
pub fn default_soul() -> Cow<'static, str> {
    let bytes = Assets::get("soul_default.md")
        .expect("soul_default.md not found in assets")
        .data;
    String::from_utf8(bytes.into_owned())
        .expect("soul_default.md is not valid UTF-8")
        .into()
}
