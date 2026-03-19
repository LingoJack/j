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
use std::fs;

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
    Assets::get("help.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 版本信息模板
///
/// 用途: `j version` 命令输出
/// 占位符: `{version}`, `{os}`, `{extra}`
/// 格式: Markdown 表格
pub fn version_template() -> Cow<'static, str> {
    Assets::get("version.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认系统提示词模板
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/system_prompt.md`
/// 占位符: `{{.tools}}`, `{{.skills}}`, `{{.style}}`, `{{.memory}}`, `{{.soul}}`
/// 格式: Markdown
pub fn default_system_prompt() -> Cow<'static, str> {
    Assets::get("system_prompt_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认记忆占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/memory.md`
/// 格式: Markdown
pub fn default_memory() -> Cow<'static, str> {
    Assets::get("memory_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 默认灵魂占位文件
///
/// 用途: 首次运行时写入 `~/.jdata/agent/data/soul.md`
/// 格式: Markdown
pub fn default_soul() -> Cow<'static, str> {
    Assets::get("soul_default.md")
        .map(|f| String::from_utf8_lossy(&f.data).into_owned().into())
        .unwrap_or_else(|| Cow::Borrowed(""))
}

/// 安装预设 skills 到用户数据目录
///
/// 遍历编译时嵌入的 `assets/skills/` 目录下的所有文件，
/// 将其写入 `~/.jdata/agent/skills/` 对应路径（仅当 skill 目录不存在时才写入，不覆盖用户修改）。
pub fn install_default_skills(skills_dir: &std::path::Path) -> Result<(), std::io::Error> {
    for filename in Assets::iter() {
        let filename = filename.as_ref();

        // 只处理 skills/ 前缀的文件
        if !filename.starts_with("skills/") {
            continue;
        }

        // 提取相对路径，如 "skills/my-skill/SKILL.md" → "my-skill/SKILL.md"
        let rel_path = &filename["skills/".len()..];
        if rel_path.is_empty() {
            continue;
        }

        // skills 目录原封不动复制到用户数据目录
        let dst_path = skills_dir.join(rel_path);
        if dst_path.exists() {
            continue;
        }

        let asset = Assets::get(filename).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("asset not found: {}", filename),
            )
        })?;

        if let Some(parent) = dst_path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&dst_path, asset.data)?;
    }
    Ok(())
}
